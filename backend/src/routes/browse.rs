use axum::Json;
use axum::extract::{Path, State};
use mysqlview_types::{BrowseRequest, BrowseResponse};

use crate::db::browse;
use crate::error::Result;
use crate::state::AppState;
use crate::validate::check_identifier;

pub async fn browse(
    State(state): State<AppState>,
    Path((db, table)): Path<(String, String)>,
    Json(req): Json<BrowseRequest>,
) -> Result<Json<BrowseResponse>> {
    check_identifier(&db, "database")?;
    check_identifier(&table, "table")?;
    let resp = browse::browse(&state.pool, &db, &table, &req, state.max_rows).await?;
    Ok(Json(resp))
}
