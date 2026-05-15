//! Pure formatters and parsers for CSV / SQL serialisation of `CellValue`s.
//!
//! Conventions (matching the Phase 4 design notes):
//! * CSV uses RFC-4180 quoting. An unquoted empty field represents NULL; a
//!   quoted `""` represents an empty string. Bytes are emitted with a `b64:`
//!   prefix so they're distinguishable from real strings on re-import.
//! * SQL literals follow MySQL syntax: `NULL`, `0`/`1` for booleans, bare
//!   numeric literals, single-quoted strings with embedded `''` escaping,
//!   `0x...` for bytes, single-quoted JSON text (MySQL casts implicitly).

// Some helpers (parse_csv_line, parse_csv_cell, hex_encode) are only used by
// follow-up commits' import endpoints; suppress the dead-code warning until
// then.
#![allow(dead_code)]

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use mysqlview_types::CellValue;

use crate::error::{AppError, Result};
use crate::validate::{escape_sql_string_literal, quote_identifier};

/// Marker prefix used in CSV cells to encode a binary payload as base64. The
/// prefix is chosen to be visually distinct and unlikely to appear in real
/// data; on import we strip it before decoding.
pub const CSV_BYTES_PREFIX: &str = "b64:";

/// Render a single `CellValue` as a CSV cell. Callers concatenate the result
/// with `,` and `\n` to build the final document.
pub fn cell_to_csv_string(cell: &CellValue) -> String {
    match cell {
        // NULL: bare empty field (no quotes).
        CellValue::Null => String::new(),
        CellValue::Bool(b) => if *b { "1" } else { "0" }.to_owned(),
        CellValue::Int(n) => n.to_string(),
        CellValue::Float(f) => f.to_string(),
        // Empty string round-trips through the RFC-4180 escape (it becomes "").
        CellValue::String(s) => csv_quote(s),
        CellValue::Bytes { base64 } => csv_quote(&format!("{CSV_BYTES_PREFIX}{base64}")),
        CellValue::Json(v) => csv_quote(&v.to_string()),
    }
}

/// RFC-4180-style CSV quoting. Always emits surrounding quotes when escaping
/// is required *or* when the field is the empty string — the latter so callers
/// can encode an empty string distinct from NULL (which is bare-empty).
pub fn csv_quote(s: &str) -> String {
    let needs_quoting =
        s.is_empty() || s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r');
    if !needs_quoting {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        if ch == '"' {
            out.push_str("\"\"");
        } else {
            out.push(ch);
        }
    }
    out.push('"');
    out
}

/// Parse a single decoded CSV field into a `CellValue`.
///
/// The `was_quoted` flag tells us whether the CSV parser saw an opening quote
/// for this field — required to disambiguate an empty string `""` from a NULL
/// (bare empty). Callers that only have the raw decoded value (no quoting
/// info) can use [`parse_csv_field`] which takes the raw field string.
pub fn parse_csv_cell(field: &str, was_quoted: bool) -> Result<CellValue> {
    if !was_quoted && field.is_empty() {
        return Ok(CellValue::Null);
    }
    if let Some(rest) = field.strip_prefix(CSV_BYTES_PREFIX) {
        // Validate base64 by attempting a decode; we don't keep the bytes,
        // we re-emit the canonical encoding so writers and readers agree.
        BASE64
            .decode(rest.as_bytes())
            .map_err(|e| AppError::BadRequest(format!("invalid base64 cell: {e}")))?;
        return Ok(CellValue::Bytes {
            base64: rest.to_owned(),
        });
    }
    Ok(CellValue::String(field.to_owned()))
}

/// Render a `CellValue` as a SQL literal suitable for an `INSERT ... VALUES`
/// clause.
pub fn cell_to_sql_literal(cell: &CellValue) -> String {
    match cell {
        CellValue::Null => "NULL".to_owned(),
        CellValue::Bool(b) => if *b { "1" } else { "0" }.to_owned(),
        CellValue::Int(n) => n.to_string(),
        CellValue::Float(f) => {
            // Non-finite f64s are stored as String by dynamic_row, so we only
            // see finite values here. Use the default formatter — Rust's f64
            // formatter produces a MySQL-acceptable literal for finite values.
            f.to_string()
        }
        CellValue::String(s) => format!("'{}'", escape_sql_string_literal(s)),
        CellValue::Bytes { base64 } => match BASE64.decode(base64.as_bytes()) {
            Ok(bytes) => format!("0x{}", hex_encode(&bytes)),
            // Should be unreachable for values produced by the server, but
            // fall back to an explicit NULL rather than emitting an invalid
            // literal on dirty input.
            Err(_) => "NULL".to_owned(),
        },
        CellValue::Json(v) => format!("'{}'", escape_sql_string_literal(&v.to_string())),
    }
}

/// Build the CSV header row (column names) terminated by `\n`.
///
/// The column names come from the server's `information_schema` lookup, which
/// has already validated them; we still apply `csv_quote` so column names
/// containing commas or quotes survive.
pub fn build_csv_header(columns: &[String]) -> String {
    let mut out = String::new();
    for (i, name) in columns.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&csv_quote(name));
    }
    out.push('\n');
    out
}

/// Build one CSV data row from a slice of `CellValue`s, terminated by `\n`.
pub fn build_csv_row(cells: &[CellValue]) -> String {
    let mut out = String::new();
    for (i, cell) in cells.iter().enumerate() {
        if i > 0 {
            out.push(',');
        }
        out.push_str(&cell_to_csv_string(cell));
    }
    out.push('\n');
    out
}

/// Build an `INSERT INTO `db`.`table` (`col`, ...) VALUES (...);` statement
/// for a single row. One INSERT per row keeps the import path's failure mode
/// simple (the failing-row index is unambiguous) and keeps statement size
/// bounded.
pub fn build_insert_statement(
    db: &str,
    table: &str,
    columns: &[String],
    cells: &[CellValue],
) -> String {
    debug_assert_eq!(columns.len(), cells.len());
    let qualified = format!("{}.{}", quote_identifier(db), quote_identifier(table));
    let cols = columns
        .iter()
        .map(|c| quote_identifier(c))
        .collect::<Vec<_>>()
        .join(", ");
    let values = cells
        .iter()
        .map(cell_to_sql_literal)
        .collect::<Vec<_>>()
        .join(", ");
    format!("INSERT INTO {qualified} ({cols}) VALUES ({values});\n")
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

/// Split one decoded CSV line (already split on commas with quotes handled)
/// into `(was_quoted, value)` pairs.
///
/// This is a forgiving, character-by-character parser that accepts the subset
/// of RFC-4180 we ever emit: optional surrounding double quotes, embedded
/// `""` escapes inside quoted fields. Line terminators inside quoted fields
/// are preserved.
pub fn parse_csv_line(line: &str) -> Result<Vec<(bool, String)>> {
    let mut out = Vec::new();
    let mut chars = line.chars().peekable();
    loop {
        let mut field = String::new();
        let mut quoted = false;
        match chars.peek() {
            Some('"') => {
                quoted = true;
                chars.next();
                loop {
                    match chars.next() {
                        Some('"') => {
                            // Either a closing quote, or an escaped "".
                            if matches!(chars.peek(), Some('"')) {
                                chars.next();
                                field.push('"');
                            } else {
                                break;
                            }
                        }
                        Some(c) => field.push(c),
                        None => {
                            return Err(AppError::BadRequest(
                                "unterminated quoted CSV field".into(),
                            ));
                        }
                    }
                }
            }
            _ => {
                // Unquoted field: read until comma or EOL.
                while let Some(&c) = chars.peek() {
                    if c == ',' {
                        break;
                    }
                    chars.next();
                    field.push(c);
                }
            }
        }
        out.push((quoted, field));
        match chars.next() {
            Some(',') => continue,
            Some(other) => {
                return Err(AppError::BadRequest(format!(
                    "unexpected character after CSV field: {other:?}"
                )));
            }
            None => break,
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mysqlview_types::CellValue;
    use serde_json::json;

    #[test]
    fn csv_null_is_bare_empty() {
        assert_eq!(cell_to_csv_string(&CellValue::Null), "");
    }

    #[test]
    fn csv_empty_string_is_quoted_empty() {
        // Round-trip safety: NULL and "" must be distinguishable.
        assert_eq!(
            cell_to_csv_string(&CellValue::String(String::new())),
            "\"\""
        );
    }

    #[test]
    fn csv_bool_and_number() {
        assert_eq!(cell_to_csv_string(&CellValue::Bool(true)), "1");
        assert_eq!(cell_to_csv_string(&CellValue::Bool(false)), "0");
        assert_eq!(cell_to_csv_string(&CellValue::Int(-42)), "-42");
        assert_eq!(cell_to_csv_string(&CellValue::Float(2.5)), "2.5");
    }

    #[test]
    fn csv_quotes_only_when_needed() {
        assert_eq!(csv_quote("plain"), "plain");
        assert_eq!(csv_quote(""), "\"\"");
        assert_eq!(csv_quote("a,b"), "\"a,b\"");
        assert_eq!(csv_quote("she said \"hi\""), "\"she said \"\"hi\"\"\"");
        assert_eq!(csv_quote("multi\nline"), "\"multi\nline\"");
    }

    #[test]
    fn csv_bytes_get_prefix() {
        let v = CellValue::Bytes {
            base64: "AAEC".into(),
        };
        assert_eq!(cell_to_csv_string(&v), "b64:AAEC");
    }

    #[test]
    fn csv_json_is_serialised_string() {
        let v = CellValue::Json(json!({"a": 1}));
        assert_eq!(cell_to_csv_string(&v), "\"{\"\"a\"\":1}\"");
    }

    #[test]
    fn sql_literal_variants() {
        assert_eq!(cell_to_sql_literal(&CellValue::Null), "NULL");
        assert_eq!(cell_to_sql_literal(&CellValue::Bool(true)), "1");
        assert_eq!(cell_to_sql_literal(&CellValue::Bool(false)), "0");
        assert_eq!(cell_to_sql_literal(&CellValue::Int(42)), "42");
        assert_eq!(cell_to_sql_literal(&CellValue::Float(2.5)), "2.5");
        assert_eq!(
            cell_to_sql_literal(&CellValue::String("it's".into())),
            "'it''s'"
        );
        assert_eq!(
            cell_to_sql_literal(&CellValue::Bytes {
                base64: "AAEC".into(),
            }),
            "0x000102"
        );
        assert_eq!(
            cell_to_sql_literal(&CellValue::Json(json!({"a": 1}))),
            "'{\"a\":1}'"
        );
    }

    #[test]
    fn parse_csv_line_handles_quotes_and_commas() {
        let parsed = parse_csv_line("plain,\"\",\"a,b\",\"she said \"\"hi\"\"\"").unwrap();
        assert_eq!(
            parsed,
            vec![
                (false, "plain".into()),
                (true, String::new()),
                (true, "a,b".into()),
                (true, "she said \"hi\"".into()),
            ]
        );
    }

    #[test]
    fn parse_csv_cell_distinguishes_null_and_empty() {
        assert_eq!(parse_csv_cell("", false).unwrap(), CellValue::Null);
        assert_eq!(
            parse_csv_cell("", true).unwrap(),
            CellValue::String(String::new())
        );
    }

    #[test]
    fn parse_csv_cell_decodes_bytes_prefix() {
        let cell = parse_csv_cell("b64:AAEC", true).unwrap();
        assert_eq!(
            cell,
            CellValue::Bytes {
                base64: "AAEC".into()
            }
        );
    }

    #[test]
    fn parse_csv_cell_rejects_bad_base64() {
        assert!(parse_csv_cell("b64:!!!", true).is_err());
    }

    #[test]
    fn parse_csv_line_rejects_unterminated_quote() {
        assert!(parse_csv_line("\"unterminated").is_err());
    }

    #[test]
    fn build_csv_header_quotes_only_when_needed() {
        assert_eq!(
            build_csv_header(&["id".into(), "weird,name".into()]),
            "id,\"weird,name\"\n"
        );
    }

    #[test]
    fn build_csv_row_separates_with_comma() {
        let row = build_csv_row(&[
            CellValue::Int(1),
            CellValue::Null,
            CellValue::String("hi".into()),
            CellValue::String(String::new()),
        ]);
        assert_eq!(row, "1,,hi,\"\"\n");
    }

    #[test]
    fn build_insert_statement_emits_full_row() {
        let sql = build_insert_statement(
            "demo",
            "actor",
            &["id".into(), "name".into()],
            &[CellValue::Int(7), CellValue::String("ada".into())],
        );
        assert_eq!(
            sql,
            "INSERT INTO `demo`.`actor` (`id`, `name`) VALUES (7, 'ada');\n"
        );
    }

    /// Round-trip property: every CellValue produced by the server must
    /// survive cell → CSV → cell unchanged (modulo JSON, which always lands
    /// as String on parse — we'd need the destination column type to know to
    /// re-parse, and that's an import-time concern).
    #[test]
    fn csv_roundtrip_preserves_null_string_and_bytes() {
        let cases = [
            CellValue::Null,
            CellValue::String(String::new()),
            CellValue::String("hello".into()),
            CellValue::String("it's, \"quoted\", and\nmulti-line".into()),
            CellValue::Bytes {
                base64: "AAEC".into(),
            },
        ];
        for cell in cases {
            let encoded = cell_to_csv_string(&cell);
            let parsed = parse_csv_line(&encoded).unwrap();
            assert_eq!(parsed.len(), 1, "encoded={encoded:?}");
            let (was_quoted, field) = parsed.into_iter().next().unwrap();
            let back = parse_csv_cell(&field, was_quoted).unwrap();
            assert_eq!(back, cell, "encoded={encoded:?}");
        }
    }
}
