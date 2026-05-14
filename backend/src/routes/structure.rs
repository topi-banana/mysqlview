use axum::Json;
use axum::extract::{Path, State};
use mysqlview_types::TableStructure;

use crate::db::introspection;
use crate::error::Result;
use crate::state::AppState;
use crate::validate::check_identifier;

pub async fn structure(
    State(state): State<AppState>,
    Path((db, table)): Path<(String, String)>,
) -> Result<Json<TableStructure>> {
    check_identifier(&db, "database")?;
    check_identifier(&table, "table")?;
    let st = introspection::describe_table(&state.pool, &db, &table).await?;
    Ok(Json(st))
}
