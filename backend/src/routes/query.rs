use std::time::Instant;

use axum::Json;
use axum::extract::State;
use futures_util::TryStreamExt;
use mysqlview_types::{QueryRequest, QueryResponse};
use sqlx::Executor;

use crate::db::dynamic_row::row_to_cells;
use crate::error::{AppError, Result};
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

fn first_statement(raw: &str) -> Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest("SQL is empty".into()));
    }
    // Naive split on `;` that respects single- and double-quoted strings and
    // backtick-quoted identifiers, plus simple line/block comment handling.
    let mut out = String::with_capacity(trimmed.len());
    let mut chars = trimmed.chars().peekable();
    let mut quote: Option<char> = None;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    while let Some(c) = chars.next() {
        if in_line_comment {
            out.push(c);
            if c == '\n' {
                in_line_comment = false;
            }
            continue;
        }
        if in_block_comment {
            out.push(c);
            if c == '*' && matches!(chars.peek(), Some('/')) {
                out.push(chars.next().unwrap());
                in_block_comment = false;
            }
            continue;
        }
        if let Some(q) = quote {
            out.push(c);
            if c == q {
                quote = None;
            } else if c == '\\'
                && let Some(next) = chars.next()
            {
                out.push(next);
            }
            continue;
        }
        match c {
            '\'' | '"' | '`' => {
                quote = Some(c);
                out.push(c);
            }
            '-' if matches!(chars.peek(), Some('-')) => {
                in_line_comment = true;
                out.push(c);
                out.push(chars.next().unwrap());
            }
            '/' if matches!(chars.peek(), Some('*')) => {
                in_block_comment = true;
                out.push(c);
                out.push(chars.next().unwrap());
            }
            ';' => break,
            _ => out.push(c),
        }
    }
    let stmt = out.trim().to_string();
    if stmt.is_empty() {
        return Err(AppError::BadRequest("SQL is empty".into()));
    }
    Ok(stmt)
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

    #[test]
    fn first_statement_strips_trailing_semicolon() {
        assert_eq!(first_statement("SELECT 1;").unwrap(), "SELECT 1");
        assert_eq!(first_statement("  SELECT 1 ; ").unwrap(), "SELECT 1");
    }

    #[test]
    fn first_statement_takes_only_first() {
        assert_eq!(first_statement("SELECT 1; SELECT 2;").unwrap(), "SELECT 1");
    }

    #[test]
    fn first_statement_respects_quotes() {
        assert_eq!(
            first_statement("SELECT ';' AS x; SELECT 2").unwrap(),
            "SELECT ';' AS x"
        );
        assert_eq!(
            first_statement(r#"SELECT ";" AS x; SELECT 2"#).unwrap(),
            r#"SELECT ";" AS x"#
        );
    }

    #[test]
    fn first_statement_respects_line_comment() {
        assert_eq!(
            first_statement("SELECT 1 -- ; not a separator\nFROM t").unwrap(),
            "SELECT 1 -- ; not a separator\nFROM t"
        );
    }

    #[test]
    fn first_statement_rejects_empty() {
        assert!(first_statement("").is_err());
        assert!(first_statement("   ;  ").is_err());
    }
}
