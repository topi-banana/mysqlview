//! Quote- and comment-aware splitter for MySQL SQL scripts.
//!
//! Used by:
//! * `routes::query` — to extract the *first* statement from a Console
//!   submission so we never run more than one statement at a time there.
//! * `routes::import` — to iterate every statement in an imported `.sql`
//!   file (Phase 4).
//!
//! The parser recognises:
//! * Single (`'`), double (`"`) and backtick (`` ` ``) quoted strings, with
//!   backslash escaping inside them.
//! * `--` and `#` line comments (MySQL accepts both).
//! * `/* ... */` block comments. Conditional `/*! ... */` comments are
//!   preserved verbatim so the embedded SQL is executed on import.
//!
//! `DELIMITER` directives (used by mysqldump for stored programs) are
//! rejected with a clear error rather than silently mis-parsed; Phase 4
//! scope explicitly excludes routine bodies.

use crate::error::{AppError, Result};

/// Iterator yielding each non-empty SQL statement from `raw`. Statements are
/// returned trimmed (no trailing whitespace or `;`). Yields an `Err` if the
/// script contains a `DELIMITER` directive or any other shape this parser
/// can't safely split.
pub fn split_statements(raw: &str) -> impl Iterator<Item = Result<String>> + '_ {
    StatementIter {
        rest: raw.trim_start(),
        done: false,
    }
}

/// Convenience for the Console path: extract only the first statement,
/// returning an error if `raw` is empty or only whitespace/comments.
pub fn first_statement(raw: &str) -> Result<String> {
    match split_statements(raw).next() {
        Some(Ok(s)) => Ok(s),
        Some(Err(e)) => Err(e),
        None => Err(AppError::BadRequest("SQL is empty".into())),
    }
}

struct StatementIter<'a> {
    rest: &'a str,
    done: bool,
}

impl Iterator for StatementIter<'_> {
    type Item = Result<String>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        loop {
            // Skip leading whitespace between statements without consuming it
            // from `out` — that way the trim_start the caller would do is
            // unnecessary.
            self.rest = self.rest.trim_start();
            if self.rest.is_empty() {
                self.done = true;
                return None;
            }
            if starts_with_delimiter_directive(self.rest) {
                self.done = true;
                return Some(Err(AppError::BadRequest(
                    "DELIMITER directives are not supported".into(),
                )));
            }
            match scan_statement(self.rest) {
                Ok((stmt, consumed)) => {
                    self.rest = &self.rest[consumed..];
                    let trimmed = stmt.trim().to_owned();
                    if trimmed.is_empty() {
                        // Empty between two `;`s — skip and try again.
                        continue;
                    }
                    return Some(Ok(trimmed));
                }
                Err(e) => {
                    self.done = true;
                    return Some(Err(e));
                }
            }
        }
    }
}

/// Returns true if `s` starts with a `DELIMITER` keyword token, ignoring
/// surrounding whitespace.
fn starts_with_delimiter_directive(s: &str) -> bool {
    let head = s.trim_start();
    let next_token: String = head
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    next_token.eq_ignore_ascii_case("DELIMITER")
}

/// Scan from the start of `input` up to (and consuming) the next unquoted
/// `;`, or to the end of input. Returns the statement text *without* the
/// trailing `;` and the byte count consumed from `input`.
fn scan_statement(input: &str) -> Result<(String, usize)> {
    let mut out = String::new();
    let mut chars = input.char_indices().peekable();
    let mut quote: Option<char> = None;
    let mut in_line_comment = false;
    let mut in_block_comment = false;
    let mut consumed = input.len();

    while let Some((i, c)) = chars.next() {
        if in_line_comment {
            out.push(c);
            if c == '\n' {
                in_line_comment = false;
            }
            continue;
        }
        if in_block_comment {
            out.push(c);
            if c == '*'
                && let Some(&(_, '/')) = chars.peek()
            {
                let (_, slash) = chars.next().unwrap();
                out.push(slash);
                in_block_comment = false;
            }
            continue;
        }
        if let Some(q) = quote {
            out.push(c);
            if c == q {
                quote = None;
            } else if c == '\\'
                && let Some(&(_, next)) = chars.peek()
            {
                let (_, esc) = chars.next().unwrap();
                out.push(esc);
                // Suppress the extra match: we already consumed `next`.
                let _ = next;
            }
            continue;
        }
        match c {
            '\'' | '"' | '`' => {
                quote = Some(c);
                out.push(c);
            }
            '-' if matches!(chars.peek(), Some(&(_, '-'))) => {
                in_line_comment = true;
                out.push(c);
                out.push(chars.next().unwrap().1);
            }
            '#' => {
                in_line_comment = true;
                out.push(c);
            }
            '/' if matches!(chars.peek(), Some(&(_, '*'))) => {
                in_block_comment = true;
                out.push(c);
                out.push(chars.next().unwrap().1);
            }
            ';' => {
                consumed = i + c.len_utf8();
                return Ok((out, consumed));
            }
            _ => out.push(c),
        }
    }

    if quote.is_some() {
        return Err(AppError::BadRequest(
            "unterminated string in SQL script".into(),
        ));
    }
    if in_block_comment {
        return Err(AppError::BadRequest(
            "unterminated block comment in SQL script".into(),
        ));
    }
    Ok((out, consumed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split_ok(raw: &str) -> Vec<String> {
        split_statements(raw).collect::<Result<Vec<_>>>().unwrap()
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
    fn first_statement_respects_single_quotes() {
        assert_eq!(
            first_statement("SELECT ';' AS x; SELECT 2").unwrap(),
            "SELECT ';' AS x"
        );
    }

    #[test]
    fn first_statement_respects_double_quotes() {
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
    fn first_statement_respects_hash_comment() {
        assert_eq!(
            first_statement("SELECT 1 # ; not a separator\nFROM t").unwrap(),
            "SELECT 1 # ; not a separator\nFROM t"
        );
    }

    #[test]
    fn first_statement_rejects_empty() {
        assert!(first_statement("").is_err());
        assert!(first_statement("   ;  ").is_err());
    }

    #[test]
    fn split_handles_multiple_statements() {
        assert_eq!(
            split_ok("SELECT 1;SELECT 2; SELECT 3"),
            vec!["SELECT 1", "SELECT 2", "SELECT 3"]
        );
    }

    #[test]
    fn split_skips_empty_in_between() {
        assert_eq!(
            split_ok("SELECT 1;;SELECT 2;"),
            vec!["SELECT 1", "SELECT 2"]
        );
    }

    #[test]
    fn split_preserves_conditional_comment_body() {
        // /*! ... */ is a MySQL conditional execution comment; its body
        // must reach the server. Our splitter only needs to not mangle it.
        let stmts = split_ok("/*!40101 SET NAMES utf8 */;\nSELECT 1");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "/*!40101 SET NAMES utf8 */");
        assert_eq!(stmts[1], "SELECT 1");
    }

    #[test]
    fn split_rejects_delimiter_directive() {
        let err = split_statements("DELIMITER //\nSELECT 1//\nDELIMITER ;")
            .next()
            .unwrap()
            .unwrap_err();
        assert!(matches!(err, AppError::BadRequest(_)));
    }

    #[test]
    fn split_rejects_unterminated_string() {
        assert!(split_statements("SELECT 'unterminated").next().unwrap().is_err());
    }

    #[test]
    fn split_handles_escaped_quotes_inside_strings() {
        let stmts = split_ok(r#"INSERT INTO t VALUES ('it\'s; tricky'); SELECT 1"#);
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], r#"INSERT INTO t VALUES ('it\'s; tricky')"#);
        assert_eq!(stmts[1], "SELECT 1");
    }

    #[test]
    fn split_respects_backtick_identifier() {
        let stmts = split_ok("SELECT `weird;col` FROM t; SELECT 1");
        assert_eq!(stmts.len(), 2);
        assert_eq!(stmts[0], "SELECT `weird;col` FROM t");
    }

    #[test]
    fn split_handles_block_comment_with_semicolon() {
        let stmts = split_ok("/* a; b */ SELECT 1; SELECT 2");
        assert_eq!(stmts, vec!["/* a; b */ SELECT 1", "SELECT 2"]);
    }
}
