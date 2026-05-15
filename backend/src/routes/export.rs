//! Streaming CSV / SQL export endpoints.
//!
//! Each handler returns a chunked response (`axum::body::Body::from_stream`)
//! so very large tables don't have to be buffered in memory. The
//! [`async_stream::try_stream!`] macro owns the `PoolConnection` and the
//! sqlx row stream for the lifetime of the body, which sidesteps the borrow
//! checker around `Pool::acquire` → `conn.fetch(...)`.
//!
//! Errors before the first byte (identifier validation, table-existence
//! check) are returned as a normal `AppError` (4xx); errors mid-stream are
//! logged via `tracing::error!` and result in a truncated body — the SQL
//! handlers emit a `-- EXPORT COMPLETE` sentinel as their final line so
//! consumers can detect truncation post-hoc.

use axum::body::{Body, Bytes};
use axum::extract::{Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue};
use axum::response::{IntoResponse, Response};
use futures_util::TryStreamExt;
use sqlx::Executor;

use crate::db::dynamic_row::row_to_cells;
use crate::db::introspection;
use crate::db::io::{build_csv_header, build_csv_row, build_insert_statement};
use crate::error::{AppError, Result};
use crate::state::AppState;
use crate::validate::{check_identifier, quote_identifier};

const SQL_EXPORT_FOOTER: &str = "-- EXPORT COMPLETE\n";

pub async fn export_table_csv(
    State(state): State<AppState>,
    Path((db, table)): Path<(String, String)>,
) -> Result<Response> {
    check_identifier(&db, "database")?;
    check_identifier(&table, "table")?;

    let structure = introspection::describe_table(&state.pool, &db, &table).await?;
    let columns: Vec<String> = structure.columns.iter().map(|c| c.name.clone()).collect();
    let order_cols = primary_key_columns(&structure);
    let sql = build_select_sql(&db, &table, &order_cols);

    let pool = state.pool.clone();
    let columns_for_stream = columns.clone();
    let body = async_stream::try_stream! {
        yield Bytes::from(build_csv_header(&columns_for_stream));
        let mut conn = pool.acquire().await.map_err(AppError::from)?;
        let mut rows = conn.fetch(sql.as_str());
        while let Some(row) = rows.try_next().await.map_err(AppError::from)? {
            let cells = row_to_cells(&row)?;
            yield Bytes::from(build_csv_row(&cells));
        }
    };

    Ok(streaming_response(
        "text/csv; charset=utf-8",
        &format!("{db}__{table}.csv"),
        body,
    ))
}

pub async fn export_table_sql(
    State(state): State<AppState>,
    Path((db, table)): Path<(String, String)>,
) -> Result<Response> {
    check_identifier(&db, "database")?;
    check_identifier(&table, "table")?;

    let structure = introspection::describe_table(&state.pool, &db, &table).await?;
    let columns: Vec<String> = structure.columns.iter().map(|c| c.name.clone()).collect();
    let order_cols = primary_key_columns(&structure);
    let sql = build_select_sql(&db, &table, &order_cols);

    let pool = state.pool.clone();
    let columns_for_stream = columns.clone();
    let db_for_stream = db.clone();
    let table_for_stream = table.clone();
    let body = async_stream::try_stream! {
        yield Bytes::from(format!(
            "-- mysqlview export of `{db_for_stream}`.`{table_for_stream}`\n",
        ));
        let mut conn = pool.acquire().await.map_err(AppError::from)?;
        let mut rows = conn.fetch(sql.as_str());
        while let Some(row) = rows.try_next().await.map_err(AppError::from)? {
            let cells = row_to_cells(&row)?;
            yield Bytes::from(build_insert_statement(
                &db_for_stream,
                &table_for_stream,
                &columns_for_stream,
                &cells,
            ));
        }
        yield Bytes::from_static(SQL_EXPORT_FOOTER.as_bytes());
    };

    Ok(streaming_response(
        "application/sql",
        &format!("{db}__{table}.sql"),
        body,
    ))
}

pub async fn export_database_sql(
    State(state): State<AppState>,
    Path(db): Path<String>,
) -> Result<Response> {
    check_identifier(&db, "database")?;
    if !introspection::database_exists(&state.pool, &db).await? {
        return Err(AppError::NotFound(format!("database `{db}` not found")));
    }

    // Collect the table metadata up front (small) so the stream body can be
    // produced without further await-points outside the macro.
    let tables = introspection::list_tables(&state.pool, &db).await?;

    let pool = state.pool.clone();
    let db_for_stream = db.clone();
    let body = async_stream::try_stream! {
        yield Bytes::from(format!("-- mysqlview dump of `{db_for_stream}`\n"));
        yield Bytes::from_static(b"SET FOREIGN_KEY_CHECKS=0;\n");

        for table in &tables {
            let table_name = &table.name;
            // Per-table conn lives for the duration of one SELECT loop.
            let structure = introspection::describe_table(&pool, &db_for_stream, table_name).await?;
            let columns: Vec<String> = structure.columns.iter().map(|c| c.name.clone()).collect();
            let order_cols = primary_key_columns(&structure);

            yield Bytes::from(format!("\n-- ----- table `{db_for_stream}`.`{table_name}` -----\n"));
            yield Bytes::from(format!(
                "DROP TABLE IF EXISTS {}.{};\n",
                quote_identifier(&db_for_stream),
                quote_identifier(table_name),
            ));
            yield Bytes::from(format!("{};\n", structure.create_statement));

            let select_sql = build_select_sql(&db_for_stream, table_name, &order_cols);
            let mut conn = pool.acquire().await.map_err(AppError::from)?;
            let mut rows = conn.fetch(select_sql.as_str());
            while let Some(row) = rows.try_next().await.map_err(AppError::from)? {
                let cells = row_to_cells(&row)?;
                yield Bytes::from(build_insert_statement(
                    &db_for_stream,
                    table_name,
                    &columns,
                    &cells,
                ));
            }
        }

        yield Bytes::from_static(b"\nSET FOREIGN_KEY_CHECKS=1;\n");
        yield Bytes::from_static(SQL_EXPORT_FOOTER.as_bytes());
    };

    Ok(streaming_response(
        "application/sql",
        &format!("{db}.sql"),
        body,
    ))
}

fn primary_key_columns(structure: &mysqlview_types::TableStructure) -> Vec<String> {
    structure
        .indexes
        .iter()
        .find(|i| i.primary)
        .map(|i| i.columns.clone())
        .unwrap_or_default()
}

fn build_select_sql(db: &str, table: &str, order_cols: &[String]) -> String {
    let qualified = format!("{}.{}", quote_identifier(db), quote_identifier(table));
    let mut sql = format!("SELECT * FROM {qualified}");
    if !order_cols.is_empty() {
        let by = order_cols
            .iter()
            .map(|c| quote_identifier(c))
            .collect::<Vec<_>>()
            .join(", ");
        sql.push_str(&format!(" ORDER BY {by}"));
    }
    sql
}

fn streaming_response<S>(content_type: &str, filename: &str, body: S) -> Response
where
    S: futures_util::Stream<Item = Result<Bytes>> + Send + 'static,
{
    let mapped = body.map_err(|e| {
        // Headers have already been flushed; log so operators see the truncation.
        tracing::error!(error = %e, "export stream terminated with error");
        e
    });
    let mut headers = HeaderMap::new();
    headers.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(content_type).expect("static content-type is valid"),
    );
    headers.insert(
        CONTENT_DISPOSITION,
        HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
            .expect("validated identifier yields ASCII filename"),
    );
    (headers, Body::from_stream(mapped)).into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn select_sql_without_pk_skips_order_by() {
        assert_eq!(
            build_select_sql("demo", "actor", &[]),
            "SELECT * FROM `demo`.`actor`"
        );
    }

    #[test]
    fn select_sql_with_single_pk() {
        assert_eq!(
            build_select_sql("demo", "actor", &["id".into()]),
            "SELECT * FROM `demo`.`actor` ORDER BY `id`"
        );
    }

    #[test]
    fn select_sql_with_composite_pk() {
        assert_eq!(
            build_select_sql("demo", "link", &["a".into(), "b".into()]),
            "SELECT * FROM `demo`.`link` ORDER BY `a`, `b`"
        );
    }

    #[test]
    fn primary_key_columns_picks_primary_index() {
        use mysqlview_types::{IndexInfo, TableStructure};
        let s = TableStructure {
            columns: Vec::new(),
            indexes: vec![
                IndexInfo {
                    name: "name_idx".into(),
                    columns: vec!["name".into()],
                    unique: false,
                    primary: false,
                },
                IndexInfo {
                    name: "PRIMARY".into(),
                    columns: vec!["id".into()],
                    unique: true,
                    primary: true,
                },
            ],
            foreign_keys: Vec::new(),
            create_statement: String::new(),
        };
        assert_eq!(primary_key_columns(&s), vec!["id".to_string()]);
    }

    #[test]
    fn primary_key_columns_falls_back_to_empty() {
        use mysqlview_types::TableStructure;
        let s = TableStructure {
            columns: Vec::new(),
            indexes: Vec::new(),
            foreign_keys: Vec::new(),
            create_statement: String::new(),
        };
        assert!(primary_key_columns(&s).is_empty());
    }
}
