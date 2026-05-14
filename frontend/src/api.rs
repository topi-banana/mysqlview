use gloo_net::http::Request;
use mysqlview_types::{
    AlterTableRequest, ApiError, BrowseRequest, BrowseResponse, CreateDatabaseRequest,
    CreateTableRequest, DatabaseSummary, DdlResponse, DeleteRowRequest, DropDatabaseRequest,
    DropTableRequest, EditAffectedResponse, InsertRowRequest, InsertRowResponse, QueryRequest,
    QueryResponse, TableStructure, TableSummary, UpdateRowRequest,
};
use serde::de::DeserializeOwned;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiClientError {
    Network(String),
    Status { code: u16, body: ApiError },
    Decode(String),
}

impl ApiClientError {
    pub fn user_message(&self) -> String {
        match self {
            Self::Network(msg) => format!("Network error: {msg}"),
            Self::Status { code, body } => {
                format!("{} (HTTP {})", body.message, code)
            }
            Self::Decode(msg) => format!("Failed to decode response: {msg}"),
        }
    }

    pub fn hint(&self) -> Option<&str> {
        match self {
            Self::Status { body, .. } => body.hint.as_deref(),
            _ => None,
        }
    }
}

async fn handle<T: DeserializeOwned>(resp: gloo_net::http::Response) -> Result<T, ApiClientError> {
    let status = resp.status();
    if (200..300).contains(&status) {
        let text = resp
            .text()
            .await
            .map_err(|e| ApiClientError::Network(e.to_string()))?;
        serde_json::from_str::<T>(&text).map_err(|e| ApiClientError::Decode(format!("{e}: {text}")))
    } else {
        let text = resp
            .text()
            .await
            .map_err(|e| ApiClientError::Network(e.to_string()))?;
        let body = serde_json::from_str::<ApiError>(&text).unwrap_or_else(|_| ApiError {
            code: "UNKNOWN".into(),
            message: text,
            hint: None,
        });
        Err(ApiClientError::Status { code: status, body })
    }
}

pub async fn list_databases() -> Result<Vec<DatabaseSummary>, ApiClientError> {
    let resp = Request::get("/api/databases")
        .send()
        .await
        .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn list_tables(db: &str) -> Result<Vec<TableSummary>, ApiClientError> {
    let resp = Request::get(&format!("/api/databases/{}/tables", urlencode(db)))
        .send()
        .await
        .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn describe_table(db: &str, table: &str) -> Result<TableStructure, ApiClientError> {
    let resp = Request::get(&format!(
        "/api/databases/{}/tables/{}/structure",
        urlencode(db),
        urlencode(table),
    ))
    .send()
    .await
    .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn browse_rows(
    db: &str,
    table: &str,
    request: &BrowseRequest,
) -> Result<BrowseResponse, ApiClientError> {
    let resp = Request::post(&format!(
        "/api/databases/{}/tables/{}/browse",
        urlencode(db),
        urlencode(table),
    ))
    .json(request)
    .map_err(|e| ApiClientError::Network(e.to_string()))?
    .send()
    .await
    .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

fn rows_url(db: &str, table: &str) -> String {
    format!(
        "/api/databases/{}/tables/{}/rows",
        urlencode(db),
        urlencode(table),
    )
}

pub async fn insert_row(
    db: &str,
    table: &str,
    request: &InsertRowRequest,
) -> Result<InsertRowResponse, ApiClientError> {
    let resp = Request::post(&rows_url(db, table))
        .json(request)
        .map_err(|e| ApiClientError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn update_row(
    db: &str,
    table: &str,
    request: &UpdateRowRequest,
) -> Result<EditAffectedResponse, ApiClientError> {
    let resp = Request::patch(&rows_url(db, table))
        .json(request)
        .map_err(|e| ApiClientError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn delete_row(
    db: &str,
    table: &str,
    request: &DeleteRowRequest,
) -> Result<EditAffectedResponse, ApiClientError> {
    let resp = Request::delete(&rows_url(db, table))
        .json(request)
        .map_err(|e| ApiClientError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn run_query(req: &QueryRequest) -> Result<QueryResponse, ApiClientError> {
    let resp = Request::post("/api/query")
        .json(req)
        .map_err(|e| ApiClientError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn create_database(req: &CreateDatabaseRequest) -> Result<DdlResponse, ApiClientError> {
    let resp = Request::post("/api/databases")
        .json(req)
        .map_err(|e| ApiClientError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn drop_database(
    db: &str,
    req: &DropDatabaseRequest,
) -> Result<DdlResponse, ApiClientError> {
    let resp = Request::delete(&format!("/api/databases/{}", urlencode(db)))
        .json(req)
        .map_err(|e| ApiClientError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn create_table(
    db: &str,
    req: &CreateTableRequest,
) -> Result<DdlResponse, ApiClientError> {
    let resp = Request::post(&format!("/api/databases/{}/tables", urlencode(db)))
        .json(req)
        .map_err(|e| ApiClientError::Network(e.to_string()))?
        .send()
        .await
        .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn alter_table(
    db: &str,
    table: &str,
    req: &AlterTableRequest,
) -> Result<DdlResponse, ApiClientError> {
    let resp = Request::patch(&format!(
        "/api/databases/{}/tables/{}",
        urlencode(db),
        urlencode(table),
    ))
    .json(req)
    .map_err(|e| ApiClientError::Network(e.to_string()))?
    .send()
    .await
    .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn drop_table(
    db: &str,
    table: &str,
    req: &DropTableRequest,
) -> Result<DdlResponse, ApiClientError> {
    let resp = Request::delete(&format!(
        "/api/databases/{}/tables/{}",
        urlencode(db),
        urlencode(table),
    ))
    .json(req)
    .map_err(|e| ApiClientError::Network(e.to_string()))?
    .send()
    .await
    .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

fn urlencode(s: &str) -> String {
    js_sys::encode_uri_component(s)
        .as_string()
        .unwrap_or_default()
}
