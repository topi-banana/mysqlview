use axum::Json;
use axum::extract::State;
use mysqlview_types::DatabaseSummary;

use crate::db::introspection;
use crate::error::Result;
use crate::state::AppState;

pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<DatabaseSummary>>> {
    let dbs = introspection::list_databases(&state.pool).await?;
    Ok(Json(dbs))
}
