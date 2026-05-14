use crate::error::AppError;

/// Validates that a MySQL identifier (database/table/column name) consists only
/// of safe characters. Existence in `information_schema` is checked separately.
pub fn check_identifier(name: &str, kind: &str) -> Result<(), AppError> {
    if name.is_empty() || name.len() > 64 {
        return Err(AppError::BadRequest(format!(
            "{kind} name must be 1..=64 characters"
        )));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$')
    {
        return Err(AppError::BadRequest(format!(
            "{kind} name contains invalid characters: {name}"
        )));
    }
    Ok(())
}

/// Quote a validated identifier with backticks for safe interpolation into SQL.
/// The identifier must have already passed [`check_identifier`].
pub fn quote_identifier(name: &str) -> String {
    debug_assert!(check_identifier(name, "ident").is_ok());
    format!("`{}`", name.replace('`', "``"))
}

/// Validates a free-form SQL fragment used for column types (`INT UNSIGNED`,
/// `VARCHAR(64)`, `DECIMAL(10,2)`, `ENUM('a','b')`, …) or DEFAULT/comment
/// expressions. Allowed characters cover MySQL type syntax while rejecting the
/// shapes that enable SQL injection (statement terminators, backticks,
/// comments, NUL/newlines). Quoted strings inside the fragment must be
/// balanced.
pub fn check_sql_fragment(s: &str, kind: &str) -> Result<(), AppError> {
    const MAX_LEN: usize = 256;

    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err(AppError::BadRequest(format!("{kind} must not be empty")));
    }
    if s.len() > MAX_LEN {
        return Err(AppError::BadRequest(format!(
            "{kind} must be ≤ {MAX_LEN} characters"
        )));
    }

    for forbidden in [";", "`", "--", "/*", "*/", "\\"] {
        if s.contains(forbidden) {
            return Err(AppError::BadRequest(format!(
                "{kind} contains forbidden sequence {forbidden:?}"
            )));
        }
    }

    let mut in_quote = false;
    let mut prev_was_quote = false;
    for ch in s.chars() {
        if ch == '\n' || ch == '\r' || ch == '\0' {
            return Err(AppError::BadRequest(format!(
                "{kind} contains a control character"
            )));
        }
        if ch == '\'' {
            // Allow escaped single quotes ('') inside string literals.
            if in_quote && !prev_was_quote {
                in_quote = false;
                prev_was_quote = true;
                continue;
            }
            in_quote = !in_quote;
            prev_was_quote = false;
            continue;
        }
        if !in_quote && !is_safe_unquoted_char(ch) {
            return Err(AppError::BadRequest(format!(
                "{kind} contains invalid character: {ch:?}"
            )));
        }
        prev_was_quote = false;
    }

    if in_quote {
        return Err(AppError::BadRequest(format!(
            "{kind} has an unterminated string literal"
        )));
    }

    Ok(())
}

fn is_safe_unquoted_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric()
        || matches!(
            ch,
            ' ' | '_' | '$' | '(' | ')' | ',' | '.' | '+' | '-' | '='
        )
}

/// Escapes a free-form string so it can be wrapped in single quotes and
/// interpolated as a SQL string literal (used for COMMENT clauses, etc.).
pub fn escape_sql_string_literal(s: &str) -> String {
    s.replace('\\', "\\\\").replace('\'', "''")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_valid_identifiers() {
        for name in ["sakila", "actor", "_internal", "$auto", "t1", "ABC_123"] {
            check_identifier(name, "test").expect(name);
        }
    }

    #[test]
    fn rejects_invalid_identifiers() {
        for name in [
            "",
            "with space",
            "drop;",
            "x`y",
            "tab\t",
            "naïve",
            "name\"x",
            &"a".repeat(65),
        ] {
            assert!(
                check_identifier(name, "test").is_err(),
                "should reject: {name:?}"
            );
        }
    }

    #[test]
    fn quote_wraps_with_backticks() {
        assert_eq!(quote_identifier("foo"), "`foo`");
        assert_eq!(quote_identifier("_id"), "`_id`");
    }

    #[test]
    fn sql_fragment_accepts_common_types() {
        for frag in [
            "INT",
            "INT UNSIGNED",
            "BIGINT UNSIGNED",
            "VARCHAR(64)",
            "DECIMAL(10, 2)",
            "ENUM('a','b','c''d')",
            "TIMESTAMP",
            "JSON",
            "CURRENT_TIMESTAMP",
            "NULL",
            "0",
            "''",
            "'hello world'",
        ] {
            check_sql_fragment(frag, "type").expect(frag);
        }
    }

    #[test]
    fn sql_fragment_rejects_dangerous_input() {
        for frag in [
            "",
            "   ",
            "INT; DROP TABLE x",
            "VARCHAR(64) `evil`",
            "INT -- comment",
            "INT /* block */",
            "INT\nUNSIGNED",
            "weird\\char",
            "ENUM('unterminated",
            "name\"quoted\"",
            &"a".repeat(257),
        ] {
            assert!(
                check_sql_fragment(frag, "type").is_err(),
                "should reject: {frag:?}"
            );
        }
    }

    #[test]
    fn escape_sql_string_doubles_quotes() {
        assert_eq!(escape_sql_string_literal("plain"), "plain");
        assert_eq!(escape_sql_string_literal("it's"), "it''s");
        assert_eq!(escape_sql_string_literal("a\\b"), "a\\\\b");
    }
}
