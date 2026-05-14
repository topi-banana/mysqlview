use mysqlview_types::{
    ColumnInfo, DatabaseSummary, ForeignKeyInfo, IndexInfo, TableStructure, TableSummary,
};
use sqlx::mysql::MySqlRow;
use sqlx::{ColumnIndex, MySql, Pool, Row};

use crate::error::{AppError, Result};

const SYSTEM_SCHEMAS: &[&str] = &["mysql", "information_schema", "performance_schema", "sys"];

/// MySQL 8.0+ frequently returns `information_schema` text columns with the
/// `VARBINARY` SQL type even though their contents are UTF-8. sqlx refuses to
/// decode those directly into `String`, so we try `String` first and fall
/// back to `Vec<u8>` + UTF-8 decoding.
fn get_string<I>(row: &MySqlRow, idx: I) -> Result<String>
where
    I: ColumnIndex<MySqlRow> + Copy,
{
    match row.try_get::<String, _>(idx) {
        Ok(s) => Ok(s),
        Err(_) => {
            let bytes: Vec<u8> = row.try_get(idx)?;
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        }
    }
}

fn get_optional_string<I>(row: &MySqlRow, idx: I) -> Result<Option<String>>
where
    I: ColumnIndex<MySqlRow> + Copy,
{
    match row.try_get::<Option<String>, _>(idx) {
        Ok(s) => Ok(s),
        Err(_) => {
            let bytes: Option<Vec<u8>> = row.try_get(idx)?;
            Ok(bytes.map(|b| String::from_utf8_lossy(&b).into_owned()))
        }
    }
}

pub async fn list_databases(pool: &Pool<MySql>) -> Result<Vec<DatabaseSummary>> {
    let rows = sqlx::query(
        r"SELECT SCHEMA_NAME, DEFAULT_CHARACTER_SET_NAME, DEFAULT_COLLATION_NAME
            FROM information_schema.SCHEMATA
            ORDER BY SCHEMA_NAME",
    )
    .fetch_all(pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let name = get_string(&row, 0)?;
        if SYSTEM_SCHEMAS.contains(&name.as_str()) {
            continue;
        }
        out.push(DatabaseSummary {
            name,
            charset: get_optional_string(&row, 1)?,
            collation: get_optional_string(&row, 2)?,
        });
    }
    Ok(out)
}

pub async fn database_exists(pool: &Pool<MySql>, db: &str) -> Result<bool> {
    let row =
        sqlx::query(r"SELECT 1 FROM information_schema.SCHEMATA WHERE SCHEMA_NAME = ? LIMIT 1")
            .bind(db)
            .fetch_optional(pool)
            .await?;
    Ok(row.is_some())
}

pub async fn table_exists(pool: &Pool<MySql>, db: &str, table: &str) -> Result<bool> {
    let row = sqlx::query(
        r"SELECT 1 FROM information_schema.TABLES
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ? LIMIT 1",
    )
    .bind(db)
    .bind(table)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

pub async fn list_tables(pool: &Pool<MySql>, db: &str) -> Result<Vec<TableSummary>> {
    if !database_exists(pool, db).await? {
        return Err(AppError::NotFound(format!("database `{db}` not found")));
    }
    let rows = sqlx::query(
        r"SELECT TABLE_NAME, ENGINE, TABLE_ROWS, DATA_LENGTH, TABLE_COMMENT
            FROM information_schema.TABLES
            WHERE TABLE_SCHEMA = ?
            ORDER BY TABLE_NAME",
    )
    .bind(db)
    .fetch_all(pool)
    .await?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        out.push(TableSummary {
            name: get_string(&row, 0)?,
            engine: get_optional_string(&row, 1)?,
            rows: row.try_get::<Option<u64>, _>(2).ok().flatten(),
            data_length: row.try_get::<Option<u64>, _>(3).ok().flatten(),
            comment: get_optional_string(&row, 4)?.filter(|s| !s.is_empty()),
        });
    }
    Ok(out)
}

pub async fn column_names(pool: &Pool<MySql>, db: &str, table: &str) -> Result<Vec<String>> {
    let rows = sqlx::query(
        r"SELECT COLUMN_NAME FROM information_schema.COLUMNS
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
            ORDER BY ORDINAL_POSITION",
    )
    .bind(db)
    .bind(table)
    .fetch_all(pool)
    .await?;
    rows.iter().map(|r| get_string(r, 0)).collect()
}

pub async fn describe_table(pool: &Pool<MySql>, db: &str, table: &str) -> Result<TableStructure> {
    if !table_exists(pool, db, table).await? {
        return Err(AppError::NotFound(format!(
            "table `{db}`.`{table}` not found"
        )));
    }

    let columns = fetch_columns(pool, db, table).await?;
    let indexes = fetch_indexes(pool, db, table).await?;
    let foreign_keys = fetch_foreign_keys(pool, db, table).await?;
    let create_statement = fetch_create_statement(pool, db, table).await?;

    Ok(TableStructure {
        columns,
        indexes,
        foreign_keys,
        create_statement,
    })
}

async fn fetch_columns(pool: &Pool<MySql>, db: &str, table: &str) -> Result<Vec<ColumnInfo>> {
    let rows = sqlx::query(
        r"SELECT COLUMN_NAME, COLUMN_TYPE, IS_NULLABLE, COLUMN_DEFAULT,
                  COLUMN_KEY, EXTRA, COLUMN_COMMENT
            FROM information_schema.COLUMNS
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
            ORDER BY ORDINAL_POSITION",
    )
    .bind(db)
    .bind(table)
    .fetch_all(pool)
    .await?;

    rows.iter()
        .map(|row| {
            let is_nullable = get_string(row, 2)?;
            Ok(ColumnInfo {
                name: get_string(row, 0)?,
                data_type: get_string(row, 1)?,
                nullable: is_nullable.eq_ignore_ascii_case("YES"),
                default: get_optional_string(row, 3)?,
                key: get_optional_string(row, 4)?.filter(|s| !s.is_empty()),
                extra: get_optional_string(row, 5)?.filter(|s| !s.is_empty()),
                comment: get_optional_string(row, 6)?.filter(|s| !s.is_empty()),
            })
        })
        .collect()
}

async fn fetch_indexes(pool: &Pool<MySql>, db: &str, table: &str) -> Result<Vec<IndexInfo>> {
    let rows = sqlx::query(
        r"SELECT INDEX_NAME, COLUMN_NAME, NON_UNIQUE
            FROM information_schema.STATISTICS
            WHERE TABLE_SCHEMA = ? AND TABLE_NAME = ?
            ORDER BY INDEX_NAME, SEQ_IN_INDEX",
    )
    .bind(db)
    .bind(table)
    .fetch_all(pool)
    .await?;

    let mut grouped: Vec<IndexInfo> = Vec::new();
    for row in rows {
        let name = get_string(&row, 0)?;
        let column = get_optional_string(&row, 1)?;
        let non_unique: i64 = row.try_get(2).unwrap_or(1);

        let existing = grouped.iter_mut().find(|idx| idx.name == name);
        match existing {
            Some(idx) => {
                if let Some(c) = column {
                    idx.columns.push(c);
                }
            }
            None => grouped.push(IndexInfo {
                name: name.clone(),
                columns: column.into_iter().collect(),
                unique: non_unique == 0,
                primary: name == "PRIMARY",
            }),
        }
    }
    Ok(grouped)
}

async fn fetch_foreign_keys(
    pool: &Pool<MySql>,
    db: &str,
    table: &str,
) -> Result<Vec<ForeignKeyInfo>> {
    let rows = sqlx::query(
        r"SELECT kcu.CONSTRAINT_NAME,
                  kcu.COLUMN_NAME,
                  kcu.REFERENCED_TABLE_NAME,
                  kcu.REFERENCED_COLUMN_NAME,
                  rc.DELETE_RULE,
                  rc.UPDATE_RULE
            FROM information_schema.KEY_COLUMN_USAGE kcu
            JOIN information_schema.REFERENTIAL_CONSTRAINTS rc
              ON rc.CONSTRAINT_SCHEMA = kcu.CONSTRAINT_SCHEMA
             AND rc.CONSTRAINT_NAME = kcu.CONSTRAINT_NAME
            WHERE kcu.TABLE_SCHEMA = ?
              AND kcu.TABLE_NAME = ?
              AND kcu.REFERENCED_TABLE_NAME IS NOT NULL
            ORDER BY kcu.CONSTRAINT_NAME, kcu.ORDINAL_POSITION",
    )
    .bind(db)
    .bind(table)
    .fetch_all(pool)
    .await?;

    let mut grouped: Vec<ForeignKeyInfo> = Vec::new();
    for row in rows {
        let name = get_string(&row, 0)?;
        let column = get_string(&row, 1)?;
        let ref_table = get_string(&row, 2)?;
        let ref_column = get_string(&row, 3)?;
        let on_delete = get_optional_string(&row, 4)?;
        let on_update = get_optional_string(&row, 5)?;

        let existing = grouped.iter_mut().find(|fk| fk.name == name);
        match existing {
            Some(fk) => {
                fk.columns.push(column);
                fk.ref_columns.push(ref_column);
            }
            None => grouped.push(ForeignKeyInfo {
                name,
                columns: vec![column],
                ref_table,
                ref_columns: vec![ref_column],
                on_delete,
                on_update,
            }),
        }
    }
    Ok(grouped)
}

async fn fetch_create_statement(pool: &Pool<MySql>, db: &str, table: &str) -> Result<String> {
    use crate::validate::quote_identifier;
    let sql = format!(
        "SHOW CREATE TABLE {}.{}",
        quote_identifier(db),
        quote_identifier(table)
    );
    let row = sqlx::query(&sql).fetch_one(pool).await?;
    // SHOW CREATE TABLE returns: Table, Create Table
    get_string(&row, 1)
}
