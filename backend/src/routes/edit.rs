use axum::Json;
use axum::extract::{Path, State};
use mysqlview_types::{
    DeleteRowRequest, EditAffectedResponse, InsertRowRequest, InsertRowResponse, UpdateRowRequest,
};

use crate::db::edit;
use crate::error::Result;
use crate::state::AppState;
use crate::validate::check_identifier;

pub async fn insert(
    State(state): State<AppState>,
    Path((db, table)): Path<(String, String)>,
    Json(req): Json<InsertRowRequest>,
) -> Result<Json<InsertRowResponse>> {
    check_identifier(&db, "database")?;
    check_identifier(&table, "table")?;
    Ok(Json(
        edit::insert_row(&state.pool, &db, &table, &req).await?,
    ))
}

pub async fn update(
    State(state): State<AppState>,
    Path((db, table)): Path<(String, String)>,
    Json(req): Json<UpdateRowRequest>,
) -> Result<Json<EditAffectedResponse>> {
    check_identifier(&db, "database")?;
    check_identifier(&table, "table")?;
    Ok(Json(
        edit::update_row(&state.pool, &db, &table, &req).await?,
    ))
}

pub async fn delete(
    State(state): State<AppState>,
    Path((db, table)): Path<(String, String)>,
    Json(req): Json<DeleteRowRequest>,
) -> Result<Json<EditAffectedResponse>> {
    check_identifier(&db, "database")?;
    check_identifier(&table, "table")?;
    Ok(Json(
        edit::delete_row(&state.pool, &db, &table, &req).await?,
    ))
}
