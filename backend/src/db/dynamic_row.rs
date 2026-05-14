use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use chrono::{DateTime, NaiveDate, NaiveDateTime, NaiveTime, Utc};
use mysqlview_types::CellValue;
use rust_decimal::Decimal;
use sqlx::mysql::MySqlRow;
use sqlx::{Column, Row, TypeInfo};

use crate::error::{AppError, Result};

/// Maps a MySQL column type name (as reported by sqlx `TypeInfo::name()`) to
/// the high-level `CellKind` used to drive value extraction. This is the
/// purely-functional core of dynamic row decoding and is exhaustively unit
/// tested.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CellKind {
    Bool,
    Int,
    UInt,
    Float,
    Decimal,
    Date,
    DateTime,
    Time,
    Year,
    Text,
    Bytes,
    Json,
}

fn classify(type_name: &str) -> CellKind {
    let t = type_name.trim().to_ascii_uppercase();
    let base = t
        .split(|c: char| c == '(' || c.is_whitespace())
        .next()
        .unwrap_or(&t)
        .trim();
    let unsigned = t.contains("UNSIGNED");

    match base {
        "BOOLEAN" | "BOOL" => CellKind::Bool,
        "TINYINT" | "SMALLINT" | "MEDIUMINT" | "INT" | "INTEGER" | "BIGINT" => {
            if unsigned {
                CellKind::UInt
            } else {
                CellKind::Int
            }
        }
        "FLOAT" | "DOUBLE" | "REAL" => CellKind::Float,
        "DECIMAL" | "NUMERIC" | "DEC" | "FIXED" => CellKind::Decimal,
        "DATE" => CellKind::Date,
        "DATETIME" | "TIMESTAMP" => CellKind::DateTime,
        "TIME" => CellKind::Time,
        "YEAR" => CellKind::Year,
        "CHAR" | "VARCHAR" | "TEXT" | "TINYTEXT" | "MEDIUMTEXT" | "LONGTEXT" | "ENUM" | "SET" => {
            CellKind::Text
        }
        "BINARY" | "VARBINARY" | "BLOB" | "TINYBLOB" | "MEDIUMBLOB" | "LONGBLOB" | "BIT"
        | "GEOMETRY" | "POINT" | "LINESTRING" | "POLYGON" => CellKind::Bytes,
        "JSON" => CellKind::Json,
        _ => CellKind::Text,
    }
}

pub fn row_to_cells(row: &MySqlRow) -> Result<Vec<CellValue>> {
    let mut out = Vec::with_capacity(row.columns().len());
    for (idx, col) in row.columns().iter().enumerate() {
        let type_name = col.type_info().name();
        let kind = classify(type_name);
        let cell = extract(row, idx, type_name, kind)?;
        out.push(cell);
    }
    Ok(out)
}

fn extract(row: &MySqlRow, idx: usize, type_name: &str, kind: CellKind) -> Result<CellValue> {
    // First try to read as `Option<T>`; if NULL, return Null cell.
    let cell = match kind {
        CellKind::Bool => {
            // MySQL has no real BOOLEAN; TINYINT(1) is most common. Read as i64 to be safe.
            match row.try_get::<Option<bool>, _>(idx) {
                Ok(Some(b)) => CellValue::Bool(b),
                Ok(None) => CellValue::Null,
                Err(_) => match row.try_get::<Option<i64>, _>(idx)? {
                    Some(n) => CellValue::Bool(n != 0),
                    None => CellValue::Null,
                },
            }
        }
        CellKind::Int => match row.try_get::<Option<i64>, _>(idx)? {
            Some(n) => CellValue::Int(n),
            None => CellValue::Null,
        },
        CellKind::UInt => match row.try_get::<Option<u64>, _>(idx)? {
            Some(n) => {
                if n <= i64::MAX as u64 {
                    CellValue::Int(n as i64)
                } else {
                    CellValue::String(n.to_string())
                }
            }
            None => CellValue::Null,
        },
        CellKind::Float => match row.try_get::<Option<f64>, _>(idx)? {
            Some(f) => {
                if f.is_finite() {
                    CellValue::Float(f)
                } else {
                    CellValue::String(f.to_string())
                }
            }
            None => CellValue::Null,
        },
        CellKind::Decimal => match row.try_get::<Option<Decimal>, _>(idx) {
            Ok(Some(d)) => CellValue::String(d.to_string()),
            Ok(None) => CellValue::Null,
            Err(_) => fallback_string(row, idx)?,
        },
        CellKind::Date => match row.try_get::<Option<NaiveDate>, _>(idx) {
            Ok(Some(d)) => CellValue::String(d.format("%Y-%m-%d").to_string()),
            Ok(None) => CellValue::Null,
            Err(_) => fallback_string(row, idx)?,
        },
        CellKind::DateTime => {
            // sqlx maps DATETIME -> NaiveDateTime but TIMESTAMP -> DateTime<Utc>.
            // Try both, then fall back to the raw string the server sent.
            match row.try_get::<Option<NaiveDateTime>, _>(idx) {
                Ok(Some(dt)) => CellValue::String(dt.format("%Y-%m-%d %H:%M:%S%.f").to_string()),
                Ok(None) => CellValue::Null,
                Err(_) => match row.try_get::<Option<DateTime<Utc>>, _>(idx) {
                    Ok(Some(dt)) => {
                        CellValue::String(dt.format("%Y-%m-%d %H:%M:%S%.f UTC").to_string())
                    }
                    Ok(None) => CellValue::Null,
                    Err(_) => fallback_string(row, idx)?,
                },
            }
        }
        CellKind::Time => match row.try_get::<Option<NaiveTime>, _>(idx) {
            Ok(Some(t)) => CellValue::String(t.format("%H:%M:%S%.f").to_string()),
            Ok(None) => CellValue::Null,
            Err(_) => fallback_string(row, idx)?,
        },
        CellKind::Year => match row.try_get::<Option<u16>, _>(idx) {
            Ok(Some(y)) => CellValue::String(y.to_string()),
            Ok(None) => CellValue::Null,
            Err(_) => match row.try_get::<Option<i64>, _>(idx)? {
                Some(y) => CellValue::String(y.to_string()),
                None => CellValue::Null,
            },
        },
        CellKind::Text => match row.try_get::<Option<String>, _>(idx) {
            Ok(Some(s)) => CellValue::String(s),
            Ok(None) => CellValue::Null,
            Err(_) => match row.try_get::<Option<Vec<u8>>, _>(idx)? {
                Some(bytes) => match String::from_utf8(bytes) {
                    Ok(s) => CellValue::String(s),
                    Err(e) => CellValue::Bytes {
                        base64: BASE64.encode(e.into_bytes()),
                    },
                },
                None => CellValue::Null,
            },
        },
        CellKind::Bytes => match row.try_get::<Option<Vec<u8>>, _>(idx)? {
            Some(bytes) => bytes_to_cell(type_name, bytes),
            None => CellValue::Null,
        },
        CellKind::Json => match row.try_get::<Option<serde_json::Value>, _>(idx) {
            Ok(Some(v)) => CellValue::Json(v),
            Ok(None) => CellValue::Null,
            Err(_) => match row.try_get::<Option<String>, _>(idx)? {
                Some(s) => match serde_json::from_str::<serde_json::Value>(&s) {
                    Ok(v) => CellValue::Json(v),
                    Err(_) => CellValue::String(s),
                },
                None => CellValue::Null,
            },
        },
    };
    Ok::<_, AppError>(cell)
}

/// Decide whether a `Vec<u8>` read from a "Bytes-kind" column should be shown
/// as a string or as base64-encoded bytes.
///
/// sqlx-mysql reports `CHAR` / `VARCHAR` columns whose character set is
/// `binary` (charset id 63) with `TypeInfo::name() == "BINARY" | "VARBINARY"`,
/// so users who store UUIDs or hashes in `CHAR(36) CHARACTER SET binary` would
/// otherwise see `0x... bytes` in the data grid. For those two type names we
/// attempt a UTF-8 decode; if the entire payload is valid and free of control
/// characters (other than tab/newline/cr), we render it as text. Pure binary
/// types (`BLOB`, `BIT`, `GEOMETRY`, …) always render as bytes.
fn bytes_to_cell(type_name: &str, bytes: Vec<u8>) -> CellValue {
    let allow_text_fallback = matches!(
        type_name.to_ascii_uppercase().as_str(),
        "BINARY" | "VARBINARY"
    );
    if allow_text_fallback
        && let Ok(s) = std::str::from_utf8(&bytes)
        && looks_like_text(s)
    {
        return CellValue::String(s.to_owned());
    }
    CellValue::Bytes {
        base64: BASE64.encode(bytes),
    }
}

fn looks_like_text(s: &str) -> bool {
    s.chars()
        .all(|c| !c.is_control() || matches!(c, '\t' | '\n' | '\r'))
}

/// Read a column as `Option<String>` with a `Vec<u8>` fallback. MySQL drivers
/// sometimes return columns under unexpected SQL types (e.g. TIMESTAMP read as
/// raw bytes); we never want one weird column to fail the whole row.
fn fallback_string(row: &MySqlRow, idx: usize) -> Result<CellValue> {
    if let Ok(Some(s)) = row.try_get::<Option<String>, _>(idx) {
        return Ok(CellValue::String(s));
    }
    if let Ok(opt) = row.try_get::<Option<Vec<u8>>, _>(idx) {
        return Ok(match opt {
            Some(bytes) => match String::from_utf8(bytes) {
                Ok(s) => CellValue::String(s),
                Err(e) => CellValue::Bytes {
                    base64: BASE64.encode(e.into_bytes()),
                },
            },
            None => CellValue::Null,
        });
    }
    Ok(CellValue::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_numeric_types() {
        assert_eq!(classify("TINYINT(1)"), CellKind::Int);
        assert_eq!(classify("INT"), CellKind::Int);
        assert_eq!(classify("BIGINT UNSIGNED"), CellKind::UInt);
        assert_eq!(classify("INT UNSIGNED"), CellKind::UInt);
        assert_eq!(classify("FLOAT"), CellKind::Float);
        assert_eq!(classify("DOUBLE PRECISION"), CellKind::Float);
        assert_eq!(classify("DECIMAL(10,2)"), CellKind::Decimal);
        assert_eq!(classify("NUMERIC"), CellKind::Decimal);
    }

    #[test]
    fn classify_string_types() {
        assert_eq!(classify("CHAR"), CellKind::Text);
        assert_eq!(classify("CHAR(36)"), CellKind::Text);
        assert_eq!(classify("VARCHAR"), CellKind::Text);
        assert_eq!(classify("VARCHAR(255)"), CellKind::Text);
        assert_eq!(classify("TINYTEXT"), CellKind::Text);
        assert_eq!(classify("TEXT"), CellKind::Text);
        assert_eq!(classify("MEDIUMTEXT"), CellKind::Text);
        assert_eq!(classify("LONGTEXT"), CellKind::Text);
        assert_eq!(classify("ENUM('a','b')"), CellKind::Text);
        assert_eq!(classify("SET('x','y')"), CellKind::Text);
    }

    #[test]
    fn classify_binary_types() {
        assert_eq!(classify("BLOB"), CellKind::Bytes);
        assert_eq!(classify("VARBINARY(255)"), CellKind::Bytes);
        assert_eq!(classify("BIT(8)"), CellKind::Bytes);
        assert_eq!(classify("GEOMETRY"), CellKind::Bytes);
    }

    #[test]
    fn classify_temporal_types() {
        assert_eq!(classify("DATE"), CellKind::Date);
        assert_eq!(classify("DATETIME"), CellKind::DateTime);
        assert_eq!(classify("TIMESTAMP"), CellKind::DateTime);
        assert_eq!(classify("TIME"), CellKind::Time);
        assert_eq!(classify("YEAR"), CellKind::Year);
    }

    #[test]
    fn classify_json_type() {
        assert_eq!(classify("JSON"), CellKind::Json);
    }

    #[test]
    fn classify_unknown_falls_back_to_text() {
        assert_eq!(classify("WEIRD_TYPE"), CellKind::Text);
    }

    #[test]
    fn looks_like_text_accepts_printable_utf8() {
        assert!(looks_like_text("hello"));
        assert!(looks_like_text("こんにちは"));
        assert!(looks_like_text("550e8400-e29b-41d4-a716-446655440000"));
        assert!(looks_like_text("line1\nline2\twith tab\r\n"));
    }

    #[test]
    fn looks_like_text_rejects_control_bytes() {
        assert!(!looks_like_text("\u{0001}"));
        assert!(!looks_like_text("ab\u{0000}cd"));
        assert!(!looks_like_text("\x07bell"));
    }

    #[test]
    fn bytes_to_cell_returns_string_for_text_in_varbinary() {
        let bytes = b"550e8400-e29b-41d4-a716-446655440000".to_vec();
        let cell = bytes_to_cell("VARBINARY", bytes);
        match cell {
            CellValue::String(s) => {
                assert_eq!(s, "550e8400-e29b-41d4-a716-446655440000");
            }
            other => panic!("expected String, got {other:?}"),
        }
    }

    #[test]
    fn bytes_to_cell_returns_string_for_text_in_binary() {
        let bytes = "uuid-as-binary".as_bytes().to_vec();
        let cell = bytes_to_cell("BINARY", bytes);
        assert!(matches!(cell, CellValue::String(_)));
    }

    #[test]
    fn bytes_to_cell_keeps_raw_binary_as_bytes() {
        let bytes = vec![0x00, 0x01, 0x02, 0xFF];
        let cell = bytes_to_cell("VARBINARY", bytes);
        assert!(matches!(cell, CellValue::Bytes { .. }));
    }

    #[test]
    fn bytes_to_cell_keeps_blob_as_bytes_even_for_valid_utf8() {
        // BLOB-family types are always rendered as raw bytes even if the payload
        // happens to be valid UTF-8 — preserves user intent for "this is binary".
        let bytes = b"this is text".to_vec();
        let cell = bytes_to_cell("BLOB", bytes);
        assert!(matches!(cell, CellValue::Bytes { .. }));
    }
}
