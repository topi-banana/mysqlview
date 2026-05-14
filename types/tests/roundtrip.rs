use std::collections::BTreeMap;

use mysqlview_types::{
    AlterTableOperation, AlterTableRequest, ApiError, BrowseFilter, BrowseRequest, BrowseResponse,
    CellValue, ColumnDefinition, ColumnInfo, CreateDatabaseRequest, CreateTableRequest,
    DatabaseSummary, DdlResponse, DeleteRowRequest, DropDatabaseRequest, DropTableRequest,
    EditAffectedResponse, ForeignKeyInfo, IndexInfo, InsertRowRequest, InsertRowResponse,
    QueryRequest, QueryResponse, RowValues, SortOrder, TableStructure, TableSummary,
    UpdateRowRequest,
};
use serde_json::json;

fn roundtrip<T: serde::Serialize + serde::de::DeserializeOwned + PartialEq + std::fmt::Debug>(
    value: T,
) {
    let json = serde_json::to_string(&value).expect("serialize");
    let back: T = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(value, back);
}

#[test]
fn database_summary_roundtrip() {
    roundtrip(DatabaseSummary {
        name: "sakila".into(),
        charset: Some("utf8mb4".into()),
        collation: Some("utf8mb4_0900_ai_ci".into()),
    });
}

#[test]
fn table_summary_roundtrip() {
    roundtrip(TableSummary {
        name: "actor".into(),
        engine: Some("InnoDB".into()),
        rows: Some(200),
        data_length: Some(16384),
        comment: None,
    });
}

#[test]
fn table_structure_roundtrip() {
    roundtrip(TableStructure {
        columns: vec![ColumnInfo {
            name: "id".into(),
            data_type: "int unsigned".into(),
            nullable: false,
            default: None,
            key: Some("PRI".into()),
            extra: Some("auto_increment".into()),
            comment: None,
        }],
        indexes: vec![IndexInfo {
            name: "PRIMARY".into(),
            columns: vec!["id".into()],
            unique: true,
            primary: true,
        }],
        foreign_keys: vec![ForeignKeyInfo {
            name: "fk_actor".into(),
            columns: vec!["actor_id".into()],
            ref_table: "actor".into(),
            ref_columns: vec!["id".into()],
            on_delete: Some("CASCADE".into()),
            on_update: None,
        }],
        create_statement: "CREATE TABLE `actor` (...)".into(),
    });
}

#[test]
fn browse_request_roundtrip() {
    roundtrip(BrowseRequest {
        offset: 100,
        limit: 50,
        sort: Some("id".into()),
        order: Some(SortOrder::Desc),
        filters: vec![BrowseFilter {
            column: "name".into(),
            op: "LIKE".into(),
            value: Some("a%".into()),
        }],
    });
}

#[test]
fn cell_value_variants_roundtrip() {
    for v in [
        CellValue::Null,
        CellValue::Bool(true),
        CellValue::Int(-42),
        CellValue::Float(2.5),
        CellValue::String("hello".into()),
        CellValue::Bytes {
            base64: "AAEC".into(),
        },
        CellValue::Json(json!({"a": 1})),
    ] {
        roundtrip(v);
    }
}

#[test]
fn browse_response_roundtrip() {
    roundtrip(BrowseResponse {
        columns: vec!["id".into(), "name".into()],
        rows: vec![vec![CellValue::Int(1), CellValue::String("foo".into())]],
        total: Some(1),
        duration_ms: 12,
    });
}

#[test]
fn query_request_roundtrip() {
    roundtrip(QueryRequest {
        sql: "SELECT 1".into(),
    });
}

#[test]
fn query_response_resultset_roundtrip() {
    roundtrip(QueryResponse::ResultSet {
        columns: vec!["n".into()],
        rows: vec![vec![CellValue::Int(1)]],
        duration_ms: 3,
        truncated: false,
    });
}

#[test]
fn query_response_affected_roundtrip() {
    roundtrip(QueryResponse::Affected {
        affected_rows: 3,
        last_insert_id: Some(42),
        duration_ms: 7,
        warnings: vec!["truncated".into()],
    });
}

#[test]
fn api_error_roundtrip() {
    roundtrip(ApiError::new("DB_ERROR", "deadlock detected").with_hint("retry the transaction"));
}

fn sample_row_values() -> RowValues {
    let mut m: RowValues = BTreeMap::new();
    m.insert("id".into(), CellValue::Int(42));
    m.insert("name".into(), CellValue::String("ada".into()));
    m.insert("note".into(), CellValue::Null);
    m
}

#[test]
fn insert_row_request_roundtrip() {
    roundtrip(InsertRowRequest {
        values: sample_row_values(),
    });
}

#[test]
fn insert_row_response_roundtrip() {
    roundtrip(InsertRowResponse {
        affected_rows: 1,
        last_insert_id: Some(99),
    });
    roundtrip(InsertRowResponse {
        affected_rows: 1,
        last_insert_id: None,
    });
}

#[test]
fn update_row_request_roundtrip() {
    let mut key: RowValues = BTreeMap::new();
    key.insert("id".into(), CellValue::Int(1));
    roundtrip(UpdateRowRequest {
        key,
        set: sample_row_values(),
    });
}

#[test]
fn delete_row_request_roundtrip() {
    let mut key: RowValues = BTreeMap::new();
    key.insert("id".into(), CellValue::Int(1));
    roundtrip(DeleteRowRequest { key });
}

#[test]
fn edit_affected_response_roundtrip() {
    roundtrip(EditAffectedResponse { affected_rows: 3 });
}

#[test]
fn create_database_request_roundtrip() {
    roundtrip(CreateDatabaseRequest {
        name: "demo".into(),
        charset: Some("utf8mb4".into()),
        collation: Some("utf8mb4_0900_ai_ci".into()),
        if_not_exists: true,
    });
    roundtrip(CreateDatabaseRequest {
        name: "minimal".into(),
        charset: None,
        collation: None,
        if_not_exists: false,
    });
}

#[test]
fn drop_database_request_roundtrip() {
    roundtrip(DropDatabaseRequest { if_exists: true });
    roundtrip(DropDatabaseRequest { if_exists: false });
}

#[test]
fn create_table_request_roundtrip() {
    roundtrip(CreateTableRequest {
        name: "actor".into(),
        columns: vec![
            ColumnDefinition {
                name: "id".into(),
                data_type: "INT UNSIGNED".into(),
                nullable: false,
                default: None,
                auto_increment: true,
                comment: None,
            },
            ColumnDefinition {
                name: "name".into(),
                data_type: "VARCHAR(64)".into(),
                nullable: true,
                default: Some("''".into()),
                auto_increment: false,
                comment: Some("display name".into()),
            },
        ],
        primary_key: vec!["id".into()],
        engine: Some("InnoDB".into()),
        charset: Some("utf8mb4".into()),
        collation: None,
        comment: Some("primary table".into()),
        if_not_exists: false,
    });
}

#[test]
fn alter_table_request_roundtrip() {
    roundtrip(AlterTableRequest {
        operations: vec![
            AlterTableOperation::AddColumn {
                column: ColumnDefinition {
                    name: "added_at".into(),
                    data_type: "DATETIME".into(),
                    nullable: false,
                    default: Some("CURRENT_TIMESTAMP".into()),
                    auto_increment: false,
                    comment: None,
                },
                after: Some("id".into()),
            },
            AlterTableOperation::DropColumn { name: "legacy".into() },
            AlterTableOperation::ModifyColumn {
                column: ColumnDefinition {
                    name: "email".into(),
                    data_type: "VARCHAR(255)".into(),
                    nullable: true,
                    default: None,
                    auto_increment: false,
                    comment: None,
                },
            },
            AlterTableOperation::RenameColumn {
                from: "old_name".into(),
                to: "new_name".into(),
            },
            AlterTableOperation::RenameTable { to: "actor_v2".into() },
        ],
    });
}

#[test]
fn drop_table_request_roundtrip() {
    roundtrip(DropTableRequest { if_exists: true });
}

#[test]
fn ddl_response_roundtrip() {
    roundtrip(DdlResponse {
        statement: "CREATE TABLE `t` (`id` INT)".into(),
    });
}
