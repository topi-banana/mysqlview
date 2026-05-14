use serde::{Deserialize, Serialize};

use crate::browse::CellValue;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueryRequest {
    pub sql: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QueryResponse {
    ResultSet {
        columns: Vec<String>,
        rows: Vec<Vec<CellValue>>,
        duration_ms: u64,
        truncated: bool,
    },
    Affected {
        affected_rows: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        last_insert_id: Option<u64>,
        duration_ms: u64,
        #[serde(default)]
        warnings: Vec<String>,
    },
}
