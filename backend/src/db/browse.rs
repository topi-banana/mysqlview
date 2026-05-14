use std::collections::HashSet;
use std::time::Instant;

use mysqlview_types::{BrowseRequest, BrowseResponse};
use sqlx::Row;
use sqlx::mysql::MySqlArguments;
use sqlx::query::Query;
use sqlx::{MySql, Pool};

use crate::db::dynamic_row::row_to_cells;
use crate::db::introspection;
use crate::error::{AppError, Result};
use crate::validate::{check_identifier, quote_identifier};

const ALLOWED_FILTER_OPS: &[&str] = &[
    "=",
    "!=",
    "<>",
    "<",
    "<=",
    ">",
    ">=",
    "LIKE",
    "NOT LIKE",
    "IS NULL",
    "IS NOT NULL",
];

pub async fn browse(
    pool: &Pool<MySql>,
    db: &str,
    table: &str,
    request: &BrowseRequest,
    max_rows: u32,
) -> Result<BrowseResponse> {
    if !introspection::table_exists(pool, db, table).await? {
        return Err(AppError::NotFound(format!(
            "table `{db}`.`{table}` not found"
        )));
    }

    let columns = introspection::column_names(pool, db, table).await?;
    let column_set: HashSet<&str> = columns.iter().map(String::as_str).collect();

    let qualified = format!("{}.{}", quote_identifier(db), quote_identifier(table));
    let (where_clause, bind_values) = build_where(&request.filters, &column_set)?;
    let order_clause = build_order(request.sort.as_deref(), request.order, &column_set)?;
    let limit = request.limit.clamp(1, max_rows) as u64;
    let offset = request.offset;

    let select_sql = format!(
        "SELECT * FROM {qualified}{where_clause}{order_clause} LIMIT {limit} OFFSET {offset}"
    );
    let count_sql = format!("SELECT COUNT(*) FROM {qualified}{where_clause}");

    let started = Instant::now();
    let mut select_q = sqlx::query(&select_sql);
    select_q = apply_binds(select_q, &bind_values);
    let rows = select_q.fetch_all(pool).await?;
    let mut cells: Vec<Vec<_>> = Vec::with_capacity(rows.len());
    for row in &rows {
        cells.push(row_to_cells(row)?);
    }

    let mut count_q = sqlx::query(&count_sql);
    count_q = apply_binds(count_q, &bind_values);
    let total_row = count_q.fetch_one(pool).await?;
    let total: i64 = total_row.try_get(0).unwrap_or(0);
    let duration_ms = started.elapsed().as_millis() as u64;

    Ok(BrowseResponse {
        columns,
        rows: cells,
        total: Some(total.max(0) as u64),
        duration_ms,
    })
}

fn apply_binds<'q>(
    mut q: Query<'q, MySql, MySqlArguments>,
    binds: &'q [String],
) -> Query<'q, MySql, MySqlArguments> {
    for v in binds {
        q = q.bind(v.as_str());
    }
    q
}

fn build_where(
    filters: &[mysqlview_types::BrowseFilter],
    column_set: &HashSet<&str>,
) -> Result<(String, Vec<String>)> {
    if filters.is_empty() {
        return Ok((String::new(), Vec::new()));
    }
    let mut clauses = Vec::with_capacity(filters.len());
    let mut binds = Vec::new();
    for f in filters {
        check_identifier(&f.column, "column")?;
        if !column_set.contains(f.column.as_str()) {
            return Err(AppError::BadRequest(format!(
                "unknown column in filter: {}",
                f.column
            )));
        }
        let op_upper = f.op.trim().to_ascii_uppercase();
        if !ALLOWED_FILTER_OPS.iter().any(|o| *o == op_upper) {
            return Err(AppError::BadRequest(format!(
                "unsupported filter operator: {}",
                f.op
            )));
        }
        let col_sql = quote_identifier(&f.column);
        match op_upper.as_str() {
            "IS NULL" | "IS NOT NULL" => {
                clauses.push(format!("{col_sql} {op_upper}"));
            }
            _ => {
                let value = f.value.clone().ok_or_else(|| {
                    AppError::BadRequest(format!(
                        "operator {op_upper} requires a value (column {})",
                        f.column
                    ))
                })?;
                clauses.push(format!("{col_sql} {op_upper} ?"));
                binds.push(value);
            }
        }
    }
    Ok((format!(" WHERE {}", clauses.join(" AND ")), binds))
}

fn build_order(
    sort: Option<&str>,
    order: Option<mysqlview_types::SortOrder>,
    column_set: &HashSet<&str>,
) -> Result<String> {
    let Some(col) = sort else {
        return Ok(String::new());
    };
    check_identifier(col, "sort column")?;
    if !column_set.contains(col) {
        return Err(AppError::BadRequest(format!("unknown sort column: {col}")));
    }
    let dir = order.unwrap_or(mysqlview_types::SortOrder::Asc).as_sql();
    Ok(format!(" ORDER BY {} {}", quote_identifier(col), dir))
}
