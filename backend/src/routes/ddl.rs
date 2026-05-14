use axum::Json;
use axum::extract::{Path, State};
use mysqlview_types::{
    AlterTableRequest, CreateDatabaseRequest, CreateTableRequest, DdlResponse, DropDatabaseRequest,
    DropTableRequest,
};

use crate::db::ddl;
use crate::error::Result;
use crate::state::AppState;

pub async fn create_database(
    State(state): State<AppState>,
    Json(req): Json<CreateDatabaseRequest>,
) -> Result<Json<DdlResponse>> {
    Ok(Json(ddl::create_database(&state.pool, &req).await?))
}

pub async fn drop_database(
    State(state): State<AppState>,
    Path(db): Path<String>,
    Json(req): Json<DropDatabaseRequest>,
) -> Result<Json<DdlResponse>> {
    Ok(Json(ddl::drop_database(&state.pool, &db, &req).await?))
}

pub async fn create_table(
    State(state): State<AppState>,
    Path(db): Path<String>,
    Json(req): Json<CreateTableRequest>,
) -> Result<Json<DdlResponse>> {
    Ok(Json(ddl::create_table(&state.pool, &db, &req).await?))
}

pub async fn alter_table(
    State(state): State<AppState>,
    Path((db, table)): Path<(String, String)>,
    Json(req): Json<AlterTableRequest>,
) -> Result<Json<DdlResponse>> {
    Ok(Json(
        ddl::alter_table(&state.pool, &db, &table, &req).await?,
    ))
}

pub async fn drop_table(
    State(state): State<AppState>,
    Path((db, table)): Path<(String, String)>,
    Json(req): Json<DropTableRequest>,
) -> Result<Json<DdlResponse>> {
    Ok(Json(ddl::drop_table(&state.pool, &db, &table, &req).await?))
}
