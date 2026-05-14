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
}
