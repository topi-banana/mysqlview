use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::browse::CellValue;

/// Column name → cell value mapping used by the row editing endpoints.
///
/// `BTreeMap` is used so JSON serialisation is deterministic, which keeps tests
/// stable and makes API responses easier to read in logs.
pub type RowValues = BTreeMap<String, CellValue>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertRowRequest {
    pub values: RowValues,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InsertRowResponse {
    pub affected_rows: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_insert_id: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateRowRequest {
    /// Column → value pairs identifying the row to update. Must match a PK or
    /// NOT NULL UNIQUE index exactly.
    pub key: RowValues,
    /// Column → value pairs to set on the matched row.
    pub set: RowValues,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteRowRequest {
    pub key: RowValues,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EditAffectedResponse {
    pub affected_rows: u64,
}
