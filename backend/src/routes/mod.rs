use axum::Router;
use axum::routing::{delete, get, patch, post};

use crate::state::AppState;

mod browse;
mod databases;
mod ddl;
mod edit;
mod health;
mod query;
mod structure;
mod tables;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::health))
        .route(
            "/api/databases",
            get(databases::list).post(ddl::create_database),
        )
        .route("/api/databases/{db}", delete(ddl::drop_database))
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
        .route("/api/query", post(query::query))
        .with_state(state)
}
