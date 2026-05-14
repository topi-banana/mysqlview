use axum::Json;
use axum::extract::{Path, State};
use mysqlview_types::TableSummary;

use crate::db::introspection;
use crate::error::Result;
use crate::state::AppState;
use crate::validate::check_identifier;

pub async fn list(
    State(state): State<AppState>,
    Path(db): Path<String>,
) -> Result<Json<Vec<TableSummary>>> {
    check_identifier(&db, "database")?;
    let tables = introspection::list_tables(&state.pool, &db).await?;
    Ok(Json(tables))
}
