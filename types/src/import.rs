use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CsvImportFailure {
    /// 0-based index of the offending data row (the header line is not counted).
    pub row_index: u64,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CsvImportResponse {
    pub inserted: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<CsvImportFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlImportFailure {
    /// 0-based index of the failing statement after splitting.
    pub statement_index: u64,
    /// First ~120 characters of the failing statement (with whitespace trimmed)
    /// so the UI can show context without hauling around a multi-megabyte body.
    pub statement_preview: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SqlImportResponse {
    pub statements_run: u64,
    pub total_affected_rows: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_at: Option<SqlImportFailure>,
}
