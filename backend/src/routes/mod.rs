use axum::Json;
use axum::Router;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, patch, post};
use mysqlview_types::ApiError;
use tower_http::limit::RequestBodyLimitLayer;

use crate::state::AppState;

mod browse;
mod databases;
mod ddl;
mod edit;
mod export;
mod health;
mod import;
mod query;
mod structure;
mod tables;

pub fn router(state: AppState) -> Router {
    let max_import_bytes = state.max_import_bytes;
    let import_limit = RequestBodyLimitLayer::new(max_import_bytes);
    let translate_limit = middleware::from_fn(move |req: Request, next: Next| async move {
        translate_payload_too_large(req, next, max_import_bytes).await
    });

    Router::new()
        .route("/api/health", get(health::health))
        .route(
            "/api/databases",
            get(databases::list).post(ddl::create_database),
        )
        .route("/api/databases/{db}", delete(ddl::drop_database))
        .route(
            "/api/databases/{db}/export.sql",
            get(export::export_database_sql),
        )
        .route(
            "/api/databases/{db}/import.sql",
            post(import::import_database_sql)
                .route_layer(import_limit)
                .route_layer(translate_limit.clone()),
        )
        .route(
            "/api/databases/{db}/tables",
            get(tables::list).post(ddl::create_table),
        )
        .route(
            "/api/databases/{db}/tables/{table}",
            patch(ddl::alter_table).delete(ddl::drop_table),
        )
        .route(
            "/api/databases/{db}/tables/{table}/structure",
            get(structure::structure),
        )
        .route(
            "/api/databases/{db}/tables/{table}/browse",
            post(browse::browse),
        )
        .route(
            "/api/databases/{db}/tables/{table}/rows",
            post(edit::insert).patch(edit::update).delete(edit::delete),
        )
        .route(
            "/api/databases/{db}/tables/{table}/export.csv",
            get(export::export_table_csv),
        )
        .route(
            "/api/databases/{db}/tables/{table}/export.sql",
            get(export::export_table_sql),
        )
        .route(
            "/api/databases/{db}/tables/{table}/import.csv",
            post(import::import_table_csv)
                .route_layer(import_limit)
                .route_layer(translate_limit),
        )
        .route("/api/query", post(query::query))
        .with_state(state)
}

/// Replace tower-http's plain-text 413 (`length limit exceeded`) with our
/// structured `ApiError` so the frontend renders a useful message and the
/// user knows which knob to turn.
async fn translate_payload_too_large(req: Request, next: Next, max: usize) -> Response {
    let resp = next.run(req).await;
    if resp.status() != StatusCode::PAYLOAD_TOO_LARGE {
        return resp;
    }
    let body = ApiError {
        code: "REQUEST_TOO_LARGE".into(),
        message: format!(
            "Import body exceeds the {} limit configured on the backend.",
            format_bytes(max)
        ),
        hint: Some(
            "Raise --max-import-bytes (CLI) or MYSQLVIEW_MAX_IMPORT_BYTES (env) and restart the server."
                .into(),
        ),
    };
    (StatusCode::PAYLOAD_TOO_LARGE, Json(body)).into_response()
}

fn format_bytes(n: usize) -> String {
    const KIB: usize = 1024;
    const MIB: usize = 1024 * 1024;
    const GIB: usize = 1024 * 1024 * 1024;
    if n >= GIB && n.is_multiple_of(GIB) {
        format!("{} GiB", n / GIB)
    } else if n >= MIB && n.is_multiple_of(MIB) {
        format!("{} MiB", n / MIB)
    } else if n >= KIB && n.is_multiple_of(KIB) {
        format!("{} KiB", n / KIB)
    } else {
        format!("{n} bytes")
    }
}

#[cfg(test)]
mod tests {
    use super::format_bytes;

    #[test]
    fn format_bytes_picks_largest_round_unit() {
        assert_eq!(format_bytes(1024 * 1024 * 1024), "1 GiB");
        assert_eq!(format_bytes(2 * 1024 * 1024 * 1024), "2 GiB");
        assert_eq!(format_bytes(100 * 1024 * 1024), "100 MiB");
        assert_eq!(format_bytes(2 * 1024), "2 KiB");
        assert_eq!(format_bytes(500), "500 bytes");
        // Non-round values fall back to bytes so users see the exact threshold.
        assert_eq!(format_bytes(1500), "1500 bytes");
    }
}
