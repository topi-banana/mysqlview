use std::collections::{BTreeSet, HashMap};

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use mysqlview_types::{
    CellValue, ColumnInfo, DeleteRowRequest, EditAffectedResponse, IndexInfo, InsertRowRequest,
    InsertRowResponse, TableStructure, UpdateRowRequest,
};
use sqlx::mysql::MySqlArguments;
use sqlx::query::Query;
use sqlx::{MySql, Pool};

use crate::db::introspection;
use crate::error::{AppError, Result};
use crate::validate::{check_identifier, quote_identifier};

pub async fn insert_row(
    pool: &Pool<MySql>,
    db: &str,
    table: &str,
    req: &InsertRowRequest,
) -> Result<InsertRowResponse> {
    let structure = introspection::describe_table(pool, db, table).await?;
    let column_set = column_name_set(&structure.columns);

    if req.values.is_empty() {
        return Err(AppError::BadRequest(
            "at least one column value is required for insert".into(),
        ));
    }
    validate_columns_exist(req.values.keys(), &column_set, "values")?;

    let cols: Vec<&str> = req.values.keys().map(String::as_str).collect();
    let sql = build_insert_sql(db, table, &cols);

    let mut q = sqlx::query(&sql);
    let mut bytes_buf: Vec<Vec<u8>> = Vec::new();
    for col in &cols {
        let v = req.values.get(*col).expect("present");
        bytes_buf.push(decode_bytes_if_any(v)?);
    }
    for (col, bytes) in cols.iter().zip(bytes_buf.iter()) {
        let v = req.values.get(*col).expect("present");
        q = bind_cell(q, v, bytes);
    }

    let result = q.execute(pool).await?;
    Ok(InsertRowResponse {
        affected_rows: result.rows_affected(),
        last_insert_id: if result.last_insert_id() == 0 {
            None
        } else {
            Some(result.last_insert_id())
        },
    })
}

pub async fn update_row(
    pool: &Pool<MySql>,
    db: &str,
    table: &str,
    req: &UpdateRowRequest,
) -> Result<EditAffectedResponse> {
    let structure = introspection::describe_table(pool, db, table).await?;
    let column_set = column_name_set(&structure.columns);

    if req.set.is_empty() {
        return Err(AppError::BadRequest(
            "at least one column to set is required for update".into(),
        ));
    }
    if req.key.is_empty() {
        return Err(AppError::BadRequest(
            "key columns are required for update".into(),
        ));
    }
    validate_columns_exist(req.set.keys(), &column_set, "set")?;
    validate_columns_exist(req.key.keys(), &column_set, "key")?;
    validate_row_key(&structure, req.key.keys())?;

    let set_cols: Vec<&str> = req.set.keys().map(String::as_str).collect();
    let key_cols: Vec<&str> = req.key.keys().map(String::as_str).collect();
    let sql = build_update_sql(db, table, &set_cols, &key_cols);

    let mut q = sqlx::query(&sql);
    let mut bytes_buf: Vec<Vec<u8>> = Vec::new();
    for col in &set_cols {
        bytes_buf.push(decode_bytes_if_any(req.set.get(*col).expect("present"))?);
    }
    for col in &key_cols {
        bytes_buf.push(decode_bytes_if_any(req.key.get(*col).expect("present"))?);
    }
    let mut iter = bytes_buf.iter();
    for col in &set_cols {
        let v = req.set.get(*col).expect("present");
        q = bind_cell(q, v, iter.next().expect("bytes buf"));
    }
    for col in &key_cols {
        let v = req.key.get(*col).expect("present");
        q = bind_cell(q, v, iter.next().expect("bytes buf"));
    }

    let result = q.execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("no row matched the supplied key".into()));
    }
    Ok(EditAffectedResponse {
        affected_rows: result.rows_affected(),
    })
}

pub async fn delete_row(
    pool: &Pool<MySql>,
    db: &str,
    table: &str,
    req: &DeleteRowRequest,
) -> Result<EditAffectedResponse> {
    let structure = introspection::describe_table(pool, db, table).await?;
    let column_set = column_name_set(&structure.columns);

    if req.key.is_empty() {
        return Err(AppError::BadRequest(
            "key columns are required for delete".into(),
        ));
    }
    validate_columns_exist(req.key.keys(), &column_set, "key")?;
    validate_row_key(&structure, req.key.keys())?;

    let key_cols: Vec<&str> = req.key.keys().map(String::as_str).collect();
    let sql = build_delete_sql(db, table, &key_cols);

    let mut q = sqlx::query(&sql);
    let mut bytes_buf: Vec<Vec<u8>> = Vec::new();
    for col in &key_cols {
        bytes_buf.push(decode_bytes_if_any(req.key.get(*col).expect("present"))?);
    }
    for (col, bytes) in key_cols.iter().zip(bytes_buf.iter()) {
        let v = req.key.get(*col).expect("present");
        q = bind_cell(q, v, bytes);
    }

    let result = q.execute(pool).await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("no row matched the supplied key".into()));
    }
    Ok(EditAffectedResponse {
        affected_rows: result.rows_affected(),
    })
}

// ---- pure helpers (unit-tested) -----------------------------------------

fn column_name_set(cols: &[ColumnInfo]) -> HashMap<String, bool> {
    cols.iter().map(|c| (c.name.clone(), c.nullable)).collect()
}

fn validate_columns_exist<'a, I>(
    cols: I,
    column_map: &HashMap<String, bool>,
    field: &str,
) -> Result<()>
where
    I: IntoIterator<Item = &'a String>,
{
    for c in cols {
        check_identifier(c, &format!("{field} column"))?;
        if !column_map.contains_key(c.as_str()) {
            return Err(AppError::BadRequest(format!(
                "unknown column in {field}: {c}"
            )));
        }
    }
    Ok(())
}

/// Validates that `key_cols` exactly matches a primary key or a NOT NULL UNIQUE
/// index in `structure`. Prevents update/delete from running with a partial or
/// non-unique WHERE.
pub fn validate_row_key<'a, I>(structure: &TableStructure, key_cols: I) -> Result<()>
where
    I: IntoIterator<Item = &'a String>,
{
    let provided: BTreeSet<&str> = key_cols.into_iter().map(String::as_str).collect();
    if provided.is_empty() {
        return Err(AppError::BadRequest("key is empty".into()));
    }

    let nullability: HashMap<&str, bool> = structure
        .columns
        .iter()
        .map(|c| (c.name.as_str(), c.nullable))
        .collect();

    for index in &structure.indexes {
        if !is_eligible_key_index(index, &nullability) {
            continue;
        }
        let index_set: BTreeSet<&str> = index.columns.iter().map(String::as_str).collect();
        if index_set == provided {
            return Ok(());
        }
    }

    Err(AppError::BadRequest(
        "key columns do not match any primary or NOT NULL UNIQUE index".into(),
    ))
}

fn is_eligible_key_index(index: &IndexInfo, nullability: &HashMap<&str, bool>) -> bool {
    if index.primary {
        return true;
    }
    if !index.unique {
        return false;
    }
    index
        .columns
        .iter()
        .all(|c| nullability.get(c.as_str()).copied() == Some(false))
}

pub fn build_insert_sql(db: &str, table: &str, columns: &[&str]) -> String {
    debug_assert!(!columns.is_empty());
    let qualified = format!("{}.{}", quote_identifier(db), quote_identifier(table));
    let cols = columns
        .iter()
        .map(|c| quote_identifier(c))
        .collect::<Vec<_>>()
        .join(", ");
    let placeholders = vec!["?"; columns.len()].join(", ");
    format!("INSERT INTO {qualified} ({cols}) VALUES ({placeholders})")
}

pub fn build_update_sql(
    db: &str,
    table: &str,
    set_columns: &[&str],
    key_columns: &[&str],
) -> String {
    debug_assert!(!set_columns.is_empty() && !key_columns.is_empty());
    let qualified = format!("{}.{}", quote_identifier(db), quote_identifier(table));
    let set_clause = set_columns
        .iter()
        .map(|c| format!("{} = ?", quote_identifier(c)))
        .collect::<Vec<_>>()
        .join(", ");
    let where_clause = key_columns
        .iter()
        .map(|c| format!("{} = ?", quote_identifier(c)))
        .collect::<Vec<_>>()
        .join(" AND ");
    format!("UPDATE {qualified} SET {set_clause} WHERE {where_clause}")
}

pub fn build_delete_sql(db: &str, table: &str, key_columns: &[&str]) -> String {
    debug_assert!(!key_columns.is_empty());
    let qualified = format!("{}.{}", quote_identifier(db), quote_identifier(table));
    let where_clause = key_columns
        .iter()
        .map(|c| format!("{} = ?", quote_identifier(c)))
        .collect::<Vec<_>>()
        .join(" AND ");
    format!("DELETE FROM {qualified} WHERE {where_clause}")
}

pub(crate) fn decode_bytes_if_any(v: &CellValue) -> Result<Vec<u8>> {
    match v {
        CellValue::Bytes { base64 } => BASE64
            .decode(base64.as_bytes())
            .map_err(|e| AppError::BadRequest(format!("invalid base64 in bytes cell: {e}"))),
        _ => Ok(Vec::new()),
    }
}

pub(crate) fn bind_cell<'q>(
    q: Query<'q, MySql, MySqlArguments>,
    v: &'q CellValue,
    bytes: &'q [u8],
) -> Query<'q, MySql, MySqlArguments> {
    match v {
        CellValue::Null => q.bind(Option::<&str>::None),
        CellValue::Bool(b) => q.bind(*b),
        CellValue::Int(n) => q.bind(*n),
        CellValue::Float(f) => q.bind(*f),
        CellValue::String(s) => q.bind(s.as_str()),
        CellValue::Bytes { .. } => q.bind(bytes),
        CellValue::Json(j) => q.bind(j),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mysqlview_types::{ColumnInfo, IndexInfo, TableStructure};

    fn col(name: &str, nullable: bool) -> ColumnInfo {
        ColumnInfo {
            name: name.into(),
            data_type: "int".into(),
            nullable,
            default: None,
            key: None,
            extra: None,
            comment: None,
        }
    }

    fn idx(name: &str, cols: &[&str], unique: bool, primary: bool) -> IndexInfo {
        IndexInfo {
            name: name.into(),
            columns: cols.iter().map(|s| s.to_string()).collect(),
            unique,
            primary,
        }
    }

    fn structure(columns: Vec<ColumnInfo>, indexes: Vec<IndexInfo>) -> TableStructure {
        TableStructure {
            columns,
            indexes,
            foreign_keys: Vec::new(),
            create_statement: String::new(),
        }
    }

    #[test]
    fn build_insert_sql_single_column() {
        let sql = build_insert_sql("sakila", "actor", &["first_name"]);
        assert_eq!(
            sql,
            "INSERT INTO `sakila`.`actor` (`first_name`) VALUES (?)"
        );
    }

    #[test]
    fn build_insert_sql_multi_columns() {
        let sql = build_insert_sql("sakila", "actor", &["first_name", "last_name"]);
        assert_eq!(
            sql,
            "INSERT INTO `sakila`.`actor` (`first_name`, `last_name`) VALUES (?, ?)"
        );
    }

    #[test]
    fn build_update_sql_composite_key() {
        let sql = build_update_sql("db", "t", &["a", "b"], &["pk1", "pk2"]);
        assert_eq!(
            sql,
            "UPDATE `db`.`t` SET `a` = ?, `b` = ? WHERE `pk1` = ? AND `pk2` = ?"
        );
    }

    #[test]
    fn build_delete_sql_simple_key() {
        let sql = build_delete_sql("db", "t", &["id"]);
        assert_eq!(sql, "DELETE FROM `db`.`t` WHERE `id` = ?");
    }

    #[test]
    fn validate_row_key_accepts_primary() {
        let s = structure(
            vec![col("id", false), col("name", true)],
            vec![idx("PRIMARY", &["id"], true, true)],
        );
        validate_row_key(&s, &["id".to_string()]).unwrap();
    }

    #[test]
    fn validate_row_key_accepts_not_null_unique() {
        let s = structure(
            vec![col("uid", false), col("data", true)],
            vec![idx("uid_unique", &["uid"], true, false)],
        );
        validate_row_key(&s, &["uid".to_string()]).unwrap();
    }

    #[test]
    fn validate_row_key_rejects_nullable_unique() {
        let s = structure(
            vec![col("uid", true), col("data", true)],
            vec![idx("uid_unique", &["uid"], true, false)],
        );
        assert!(validate_row_key(&s, &["uid".to_string()]).is_err());
    }

    #[test]
    fn validate_row_key_rejects_non_unique() {
        let s = structure(
            vec![col("city", false)],
            vec![idx("idx_city", &["city"], false, false)],
        );
        assert!(validate_row_key(&s, &["city".to_string()]).is_err());
    }

    #[test]
    fn validate_row_key_rejects_partial_key() {
        let s = structure(
            vec![col("a", false), col("b", false), col("c", true)],
            vec![idx("PRIMARY", &["a", "b"], true, true)],
        );
        assert!(validate_row_key(&s, &["a".to_string()]).is_err());
    }

    #[test]
    fn validate_row_key_rejects_superset() {
        let s = structure(
            vec![col("a", false), col("b", false)],
            vec![idx("PRIMARY", &["a"], true, true)],
        );
        assert!(validate_row_key(&s, &["a".to_string(), "b".to_string()]).is_err());
    }

    #[test]
    fn validate_row_key_accepts_composite_pk_any_order() {
        let s = structure(
            vec![col("a", false), col("b", false)],
            vec![idx("PRIMARY", &["a", "b"], true, true)],
        );
        validate_row_key(&s, &["b".to_string(), "a".to_string()]).unwrap();
    }
}
