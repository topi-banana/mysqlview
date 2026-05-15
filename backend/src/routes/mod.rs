use axum::Router;
use axum::routing::{delete, get, patch, post};
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
    let import_limit = RequestBodyLimitLayer::new(state.max_import_bytes);

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
            post(import::import_database_sql).route_layer(import_limit),
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
            post(import::import_table_csv).route_layer(import_limit),
        )
        .route("/api/query", post(query::query))
        .with_state(state)
}
