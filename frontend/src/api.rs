use gloo_net::http::Request;
use mysqlview_types::{
    AlterTableRequest, ApiError, BrowseRequest, BrowseResponse, CreateDatabaseRequest,
    CreateTableRequest, CsvImportResponse, DatabaseSummary, DdlResponse, DeleteRowRequest,
    DropDatabaseRequest, DropTableRequest, EditAffectedResponse, InsertRowRequest,
    InsertRowResponse, QueryRequest, QueryResponse, SqlImportResponse, TableStructure,
    TableSummary, UpdateRowRequest,
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

/// Like [`handle`], but for endpoints that return a raw text body (the CSV /
/// SQL export routes). Errors still come back as JSON-encoded `ApiError`.
async fn handle_text(resp: gloo_net::http::Response) -> Result<(String, String), ApiClientError> {
    let status = resp.status();
    let filename = filename_from_disposition(&resp).unwrap_or_default();
    let text = resp
        .text()
        .await
        .map_err(|e| ApiClientError::Network(e.to_string()))?;
    if (200..300).contains(&status) {
        Ok((filename, text))
    } else {
        let body = serde_json::from_str::<ApiError>(&text).unwrap_or_else(|_| ApiError {
            code: "UNKNOWN".into(),
            message: text,
            hint: None,
        });
        Err(ApiClientError::Status { code: status, body })
    }
}

fn filename_from_disposition(resp: &gloo_net::http::Response) -> Option<String> {
    let header = resp.headers().get("content-disposition")?;
    // Look for filename="...".
    let idx = header.find("filename=")?;
    let rest = &header[idx + "filename=".len()..];
    let trimmed = rest.trim_start_matches('"');
    let end = trimmed.find('"').unwrap_or(trimmed.len());
    Some(trimmed[..end].to_owned())
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

pub async fn export_table_csv(db: &str, table: &str) -> Result<(String, String), ApiClientError> {
    let resp = Request::get(&format!(
        "/api/databases/{}/tables/{}/export.csv",
        urlencode(db),
        urlencode(table),
    ))
    .send()
    .await
    .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle_text(resp).await
}

pub async fn export_table_sql(db: &str, table: &str) -> Result<(String, String), ApiClientError> {
    let resp = Request::get(&format!(
        "/api/databases/{}/tables/{}/export.sql",
        urlencode(db),
        urlencode(table),
    ))
    .send()
    .await
    .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle_text(resp).await
}

pub async fn export_database_sql(db: &str) -> Result<(String, String), ApiClientError> {
    let resp = Request::get(&format!("/api/databases/{}/export.sql", urlencode(db)))
        .send()
        .await
        .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle_text(resp).await
}

pub async fn import_table_csv(
    db: &str,
    table: &str,
    body: &str,
) -> Result<CsvImportResponse, ApiClientError> {
    let resp = Request::post(&format!(
        "/api/databases/{}/tables/{}/import.csv",
        urlencode(db),
        urlencode(table),
    ))
    .header("Content-Type", "text/csv; charset=utf-8")
    .body(body.to_owned())
    .map_err(|e| ApiClientError::Network(e.to_string()))?
    .send()
    .await
    .map_err(|e| ApiClientError::Network(e.to_string()))?;
    handle(resp).await
}

pub async fn import_database_sql(
    db: &str,
    body: &str,
) -> Result<SqlImportResponse, ApiClientError> {
    let resp = Request::post(&format!("/api/databases/{}/import.sql", urlencode(db)))
        .header("Content-Type", "application/sql")
        .body(body.to_owned())
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
