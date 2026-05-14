use std::time::Instant;

use axum::Json;
use axum::extract::State;
use futures_util::TryStreamExt;
use mysqlview_types::{QueryRequest, QueryResponse};
use sqlx::Executor;

use crate::db::dynamic_row::row_to_cells;
use crate::error::Result;
use crate::sql_split::first_statement;
use crate::state::AppState;

pub async fn query(
    State(state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>> {
    let sql = first_statement(&req.sql)?;
    let kind = QueryKind::detect(&sql);
    let started = Instant::now();

    let response = match kind {
        QueryKind::ResultSet => run_result_set(&state, &sql, started).await?,
        QueryKind::Affected => run_affected(&state, &sql, started).await?,
    };
    Ok(Json(response))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum QueryKind {
    ResultSet,
    Affected,
}

impl QueryKind {
    fn detect(sql: &str) -> Self {
        let head = sql
            .trim_start()
            .split(|c: char| c.is_whitespace() || c == '(')
            .next()
            .unwrap_or("")
            .to_ascii_uppercase();
        match head.as_str() {
            "SELECT" | "SHOW" | "DESC" | "DESCRIBE" | "EXPLAIN" | "WITH" | "ANALYZE" | "TABLE"
            | "VALUES" => Self::ResultSet,
            _ => Self::Affected,
        }
    }
}

async fn run_result_set(state: &AppState, sql: &str, started: Instant) -> Result<QueryResponse> {
    let mut conn = state.pool.acquire().await?;
    let mut stream = conn.fetch(sql);
    let mut rows = Vec::new();
    let mut columns: Vec<String> = Vec::new();
    let mut truncated = false;
    let limit = state.max_rows as usize;

    while let Some(row) = stream.try_next().await? {
        if columns.is_empty() {
            columns = sqlx::Row::columns(&row)
                .iter()
                .map(|c| sqlx::Column::name(c).to_owned())
                .collect();
        }
        if rows.len() >= limit {
            truncated = true;
            break;
        }
        rows.push(row_to_cells(&row)?);
    }

    Ok(QueryResponse::ResultSet {
        columns,
        rows,
        duration_ms: started.elapsed().as_millis() as u64,
        truncated,
    })
}

async fn run_affected(state: &AppState, sql: &str, started: Instant) -> Result<QueryResponse> {
    let result = sqlx::query(sql).execute(&state.pool).await?;
    let affected_rows = result.rows_affected();
    let last_insert_id = if result.last_insert_id() != 0 {
        Some(result.last_insert_id())
    } else {
        None
    };
    Ok(QueryResponse::Affected {
        affected_rows,
        last_insert_id,
        duration_ms: started.elapsed().as_millis() as u64,
        warnings: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detect_result_set_kinds() {
        for sql in [
            "SELECT 1",
            "  select * from t",
            "SHOW DATABASES",
            "DESC actor",
            "DESCRIBE actor",
            "EXPLAIN SELECT 1",
            "WITH x AS (SELECT 1) SELECT * FROM x",
        ] {
            assert_eq!(QueryKind::detect(sql), QueryKind::ResultSet, "{sql}");
        }
    }

    #[test]
    fn detect_affected_kinds() {
        for sql in [
            "INSERT INTO t VALUES (1)",
            "UPDATE t SET x=1",
            "DELETE FROM t",
            "CREATE TABLE t (id INT)",
        ] {
            assert_eq!(QueryKind::detect(sql), QueryKind::Affected, "{sql}");
        }
    }
}
