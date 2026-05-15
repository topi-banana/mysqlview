//! CSV / SQL import endpoints (Phase 4).
//!
//! Bodies are accepted as raw text via `axum::body::Bytes`. The router applies
//! a `RequestBodyLimitLayer` to these two routes only (the rest of the API
//! keeps axum's 2 MiB default).
//!
//! Fail-fast policy: on the first row/statement that errors, we stop and
//! return the index plus the underlying message. No transaction wraps the
//! import — DDL implicitly commits in MySQL so atomicity would be a lie. The
//! caller (UI) can drop & recreate the table to roll back if needed.

use std::collections::HashMap;

use axum::Json;
use axum::body::Bytes;
use axum::extract::{Path, State};
use mysqlview_types::{
    CellValue, CsvImportFailure, CsvImportResponse, SqlImportFailure, SqlImportResponse,
};

use crate::db::edit::{bind_cell, build_insert_sql, decode_bytes_if_any};
use crate::db::introspection;
use crate::db::io::{parse_csv_cell, parse_csv_line};
use crate::error::{AppError, Result};
use crate::sql_split::split_statements;
use crate::state::AppState;
use crate::validate::check_identifier;

const STATEMENT_PREVIEW_BYTES: usize = 120;

pub async fn import_table_csv(
    State(state): State<AppState>,
    Path((db, table)): Path<(String, String)>,
    body: Bytes,
) -> Result<Json<CsvImportResponse>> {
    check_identifier(&db, "database")?;
    check_identifier(&table, "table")?;

    let structure = introspection::describe_table(&state.pool, &db, &table).await?;
    let nullable: HashMap<&str, bool> = structure
        .columns
        .iter()
        .map(|c| (c.name.as_str(), c.nullable))
        .collect();
    let column_set: std::collections::HashSet<&str> =
        structure.columns.iter().map(|c| c.name.as_str()).collect();

    let body_text = std::str::from_utf8(&body)
        .map_err(|e| AppError::BadRequest(format!("CSV body is not valid UTF-8: {e}")))?;

    let mut lines = LineIter::new(body_text);

    // Header.
    let header_raw = lines
        .next()
        .ok_or_else(|| AppError::BadRequest("CSV body is empty".into()))?;
    let header = parse_csv_line(header_raw)?;
    let header_cols: Vec<String> = header.into_iter().map(|(_, v)| v).collect();
    if header_cols.is_empty() {
        return Err(AppError::BadRequest("CSV header has no columns".into()));
    }
    for col in &header_cols {
        check_identifier(col, "CSV header column")?;
        if !column_set.contains(col.as_str()) {
            return Err(AppError::BadRequest(format!(
                "CSV header references unknown column: {col}"
            )));
        }
    }

    let insert_sql = build_insert_sql(
        &db,
        &table,
        &header_cols.iter().map(String::as_str).collect::<Vec<_>>(),
    );

    let mut inserted: u64 = 0;
    for (row_idx, raw_row) in lines.enumerate() {
        let fields = parse_csv_line(raw_row).map_err(|e| {
            AppError::BadRequest(format!("row {row_idx}: {}", error_message(&e)))
        })?;
        if fields.len() != header_cols.len() {
            return Ok(Json(CsvImportResponse {
                inserted,
                failed_at: Some(CsvImportFailure {
                    row_index: row_idx as u64,
                    message: format!(
                        "row has {} fields but header declared {}",
                        fields.len(),
                        header_cols.len()
                    ),
                }),
            }));
        }
        let mut cells: Vec<CellValue> = Vec::with_capacity(fields.len());
        for ((quoted, field), col) in fields.into_iter().zip(header_cols.iter()) {
            let cell = match parse_csv_cell(&field, quoted) {
                Ok(c) => c,
                Err(e) => {
                    return Ok(Json(CsvImportResponse {
                        inserted,
                        failed_at: Some(CsvImportFailure {
                            row_index: row_idx as u64,
                            message: format!("column `{col}`: {}", error_message(&e)),
                        }),
                    }));
                }
            };
            // NULL into a NOT NULL column: surface the policy clearly.
            if matches!(cell, CellValue::Null) && !nullable.get(col.as_str()).copied().unwrap_or(true)
            {
                return Ok(Json(CsvImportResponse {
                    inserted,
                    failed_at: Some(CsvImportFailure {
                        row_index: row_idx as u64,
                        message: format!("column `{col}` is NOT NULL but CSV cell is empty"),
                    }),
                }));
            }
            cells.push(cell);
        }

        let mut q = sqlx::query(&insert_sql);
        let mut bytes_buf: Vec<Vec<u8>> = Vec::with_capacity(cells.len());
        for cell in &cells {
            bytes_buf.push(decode_bytes_if_any(cell)?);
        }
        for (cell, bytes) in cells.iter().zip(bytes_buf.iter()) {
            q = bind_cell(q, cell, bytes);
        }
        match q.execute(&state.pool).await {
            Ok(res) => inserted += res.rows_affected(),
            Err(e) => {
                return Ok(Json(CsvImportResponse {
                    inserted,
                    failed_at: Some(CsvImportFailure {
                        row_index: row_idx as u64,
                        message: e.to_string(),
                    }),
                }));
            }
        }
    }

    Ok(Json(CsvImportResponse {
        inserted,
        failed_at: None,
    }))
}

pub async fn import_database_sql(
    State(state): State<AppState>,
    Path(db): Path<String>,
    body: Bytes,
) -> Result<Json<SqlImportResponse>> {
    check_identifier(&db, "database")?;
    if !introspection::database_exists(&state.pool, &db).await? {
        return Err(AppError::NotFound(format!("database `{db}` not found")));
    }
    let script = std::str::from_utf8(&body)
        .map_err(|e| AppError::BadRequest(format!("SQL body is not valid UTF-8: {e}")))?;

    // USE the target schema so unqualified identifiers resolve there.
    let mut conn = state.pool.acquire().await?;
    sqlx::query(&format!(
        "USE {}",
        crate::validate::quote_identifier(&db)
    ))
    .execute(&mut *conn)
    .await?;

    let mut statements_run: u64 = 0;
    let mut total_affected_rows: u64 = 0;
    for (idx, stmt) in split_statements(script).enumerate() {
        let stmt = match stmt {
            Ok(s) => s,
            Err(e) => {
                return Ok(Json(SqlImportResponse {
                    statements_run,
                    total_affected_rows,
                    failed_at: Some(SqlImportFailure {
                        statement_index: idx as u64,
                        statement_preview: String::new(),
                        message: error_message(&e),
                    }),
                }));
            }
        };
        match sqlx::query(&stmt).execute(&mut *conn).await {
            Ok(res) => {
                statements_run += 1;
                total_affected_rows += res.rows_affected();
            }
            Err(e) => {
                return Ok(Json(SqlImportResponse {
                    statements_run,
                    total_affected_rows,
                    failed_at: Some(SqlImportFailure {
                        statement_index: idx as u64,
                        statement_preview: statement_preview(&stmt),
                        message: e.to_string(),
                    }),
                }));
            }
        }
    }

    Ok(Json(SqlImportResponse {
        statements_run,
        total_affected_rows,
        failed_at: None,
    }))
}

/// Iterator yielding logical CSV lines. A line break inside a quoted field is
/// preserved as part of the field; we only break on raw `\n` that isn't
/// inside an open quote. Empty trailing lines are skipped.
struct LineIter<'a> {
    rest: &'a str,
}

impl<'a> LineIter<'a> {
    fn new(s: &'a str) -> Self {
        Self { rest: s }
    }
}

impl<'a> Iterator for LineIter<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.rest.is_empty() {
                return None;
            }
            let mut quote_open = false;
            let mut end = self.rest.len();
            let mut split_at = self.rest.len();
            for (i, c) in self.rest.char_indices() {
                if c == '"' {
                    // RFC-4180 doubled quote inside a quoted field: skip both.
                    if quote_open
                        && self.rest.as_bytes().get(i + 1).copied() == Some(b'"')
                    {
                        // We'll handle the second quote in the next iteration of
                        // the same loop because the for-loop visits both; just
                        // toggle off-then-on which has no net effect. Simpler:
                        // skip explicitly by not toggling.
                        continue;
                    }
                    quote_open = !quote_open;
                }
                if !quote_open && c == '\n' {
                    end = i;
                    split_at = i + 1;
                    break;
                }
            }
            let mut line = &self.rest[..end];
            // Strip a trailing \r so CRLF and LF input both work.
            if let Some(stripped) = line.strip_suffix('\r') {
                line = stripped;
            }
            self.rest = &self.rest[split_at..];
            if line.is_empty() {
                // Skip blank separator lines but allow blank-quoted-field
                // lines (those won't be empty after parse_csv_line).
                if self.rest.is_empty() {
                    return None;
                }
                continue;
            }
            return Some(line);
        }
    }
}

fn statement_preview(stmt: &str) -> String {
    let one_line: String = stmt
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();
    let mut iter = one_line.chars();
    let mut out = String::new();
    let mut bytes_used = 0;
    for ch in iter.by_ref() {
        let ch_len = ch.len_utf8();
        if bytes_used + ch_len > STATEMENT_PREVIEW_BYTES {
            out.push('…');
            break;
        }
        out.push(ch);
        bytes_used += ch_len;
    }
    out
}

fn error_message(e: &AppError) -> String {
    e.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_iter_handles_crlf_and_quoted_newlines() {
        let body = "h1,h2\r\n1,2\r\n\"line\nbreak\",3\r\n";
        let lines: Vec<&str> = LineIter::new(body).collect();
        assert_eq!(lines, vec!["h1,h2", "1,2", "\"line\nbreak\",3"]);
    }

    #[test]
    fn line_iter_skips_blank_lines() {
        let body = "a,b\n\n\n1,2\n";
        let lines: Vec<&str> = LineIter::new(body).collect();
        assert_eq!(lines, vec!["a,b", "1,2"]);
    }

    #[test]
    fn statement_preview_truncates_long_input() {
        let stmt = "x".repeat(200);
        let preview = statement_preview(&stmt);
        // 120 ASCII chars + ellipsis fit into our limit.
        assert!(preview.ends_with('…'));
        assert!(preview.chars().count() <= STATEMENT_PREVIEW_BYTES + 1);
    }

    #[test]
    fn statement_preview_strips_newlines() {
        assert_eq!(
            statement_preview("CREATE TABLE t (\n  id INT\n)"),
            "CREATE TABLE t (   id INT )"
        );
    }
}
