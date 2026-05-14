use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateDatabaseRequest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collation: Option<String>,
    #[serde(default)]
    pub if_not_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropDatabaseRequest {
    #[serde(default)]
    pub if_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct ColumnDefinition {
    pub name: String,
    pub data_type: String,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(default)]
    pub auto_increment: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CreateTableRequest {
    pub name: String,
    pub columns: Vec<ColumnDefinition>,
    #[serde(default)]
    pub primary_key: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub engine: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charset: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collation: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(default)]
    pub if_not_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AlterTableOperation {
    AddColumn {
        column: ColumnDefinition,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        after: Option<String>,
    },
    DropColumn {
        name: String,
    },
    ModifyColumn {
        column: ColumnDefinition,
    },
    RenameColumn {
        from: String,
        to: String,
    },
    RenameTable {
        to: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AlterTableRequest {
    pub operations: Vec<AlterTableOperation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DropTableRequest {
    #[serde(default)]
    pub if_exists: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DdlResponse {
    pub statement: String,
}
