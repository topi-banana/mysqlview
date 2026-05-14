use axum::Router;
use axum::routing::{get, post};

use crate::state::AppState;

mod browse;
mod databases;
mod health;
mod query;
mod structure;
mod tables;

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/api/health", get(health::health))
        .route("/api/databases", get(databases::list))
        .route("/api/databases/{db}/tables", get(tables::list))
        .route(
            "/api/databases/{db}/tables/{table}/structure",
            get(structure::structure),
        )
        .route(
            "/api/databases/{db}/tables/{table}/rows",
            post(browse::browse),
        )
        .route("/api/query", post(query::query))
        .with_state(state)
}
