use gloo_net::http::Request;
use mysqlview_types::{
    ApiError, BrowseRequest, BrowseResponse, DatabaseSummary, QueryRequest, QueryResponse,
    TableStructure, TableSummary,
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
        "/api/databases/{}/tables/{}/rows",
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

pub async fn run_query(req: &QueryRequest) -> Result<QueryResponse, ApiClientError> {
    let resp = Request::post("/api/query")
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
