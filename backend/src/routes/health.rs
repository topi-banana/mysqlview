use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde_json::json;

use crate::state::AppState;

/// Liveness + readiness probe. Returns `200 ok` when the server can also
/// reach the configured MySQL pool, otherwise `503 degraded` with the
/// connection error so external probes (Docker HEALTHCHECK, load
/// balancers) can distinguish "backend up but DB unreachable" from a
/// full outage.
pub async fn health(State(state): State<AppState>) -> impl IntoResponse {
    match sqlx::query("SELECT 1").execute(&state.pool).await {
        Ok(_) => (StatusCode::OK, Json(json!({ "status": "ok" }))).into_response(),
        Err(err) => {
            tracing::warn!(error = %err, "health check: MySQL ping failed");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "status": "degraded",
                    "error": err.to_string(),
                })),
            )
                .into_response()
        }
    }
}
