use mysqlview_types::{
    AlterTableOperation, AlterTableRequest, ColumnDefinition, CreateDatabaseRequest,
    CreateTableRequest, DdlResponse, DropDatabaseRequest, DropTableRequest,
};
use sqlx::{MySql, Pool};

use crate::db::introspection;
use crate::error::{AppError, Result};
use crate::validate::{
    check_identifier, check_sql_fragment, escape_sql_string_literal, quote_identifier,
};

pub async fn create_database(
    pool: &Pool<MySql>,
    req: &CreateDatabaseRequest,
) -> Result<DdlResponse> {
    check_identifier(&req.name, "database")?;
    if let Some(cs) = &req.charset {
        check_identifier(cs, "charset")?;
    }
    if let Some(co) = &req.collation {
        check_identifier(co, "collation")?;
    }
    let sql = build_create_database_sql(req);
    sqlx::query(&sql).execute(pool).await?;
    Ok(DdlResponse { statement: sql })
}

pub async fn drop_database(
    pool: &Pool<MySql>,
    db: &str,
    req: &DropDatabaseRequest,
) -> Result<DdlResponse> {
    check_identifier(db, "database")?;
    if !req.if_exists && !introspection::database_exists(pool, db).await? {
        return Err(AppError::NotFound(format!("database `{db}` not found")));
    }
    let sql = build_drop_database_sql(db, req);
    sqlx::query(&sql).execute(pool).await?;
    Ok(DdlResponse { statement: sql })
}

pub async fn create_table(
    pool: &Pool<MySql>,
    db: &str,
    req: &CreateTableRequest,
) -> Result<DdlResponse> {
    check_identifier(db, "database")?;
    if !introspection::database_exists(pool, db).await? {
        return Err(AppError::NotFound(format!("database `{db}` not found")));
    }
    validate_create_table(req)?;
    let sql = build_create_table_sql(db, req);
    sqlx::query(&sql).execute(pool).await?;
    Ok(DdlResponse { statement: sql })
}

pub async fn alter_table(
    pool: &Pool<MySql>,
    db: &str,
    table: &str,
    req: &AlterTableRequest,
) -> Result<DdlResponse> {
    check_identifier(db, "database")?;
    check_identifier(table, "table")?;
    if !introspection::table_exists(pool, db, table).await? {
        return Err(AppError::NotFound(format!(
            "table `{db}`.`{table}` not found"
        )));
    }
    validate_alter_table(req)?;
    let sql = build_alter_table_sql(db, table, req);
    sqlx::query(&sql).execute(pool).await?;
    Ok(DdlResponse { statement: sql })
}

pub async fn drop_table(
    pool: &Pool<MySql>,
    db: &str,
    table: &str,
    req: &DropTableRequest,
) -> Result<DdlResponse> {
    check_identifier(db, "database")?;
    check_identifier(table, "table")?;
    if !req.if_exists && !introspection::table_exists(pool, db, table).await? {
        return Err(AppError::NotFound(format!(
            "table `{db}`.`{table}` not found"
        )));
    }
    let sql = build_drop_table_sql(db, table, req);
    sqlx::query(&sql).execute(pool).await?;
    Ok(DdlResponse { statement: sql })
}

// ---- validators ---------------------------------------------------------

fn validate_create_table(req: &CreateTableRequest) -> Result<()> {
    check_identifier(&req.name, "table")?;
    if req.columns.is_empty() {
        return Err(AppError::BadRequest(
            "CREATE TABLE requires at least one column".into(),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for col in &req.columns {
        validate_column_definition(col)?;
        if !seen.insert(col.name.as_str()) {
            return Err(AppError::BadRequest(format!(
                "duplicate column in CREATE TABLE: {}",
                col.name
            )));
        }
    }
    for pk in &req.primary_key {
        check_identifier(pk, "primary key column")?;
        if !req.columns.iter().any(|c| c.name == *pk) {
            return Err(AppError::BadRequest(format!(
                "PRIMARY KEY references unknown column: {pk}"
            )));
        }
    }
    if let Some(engine) = &req.engine {
        check_identifier(engine, "engine")?;
    }
    if let Some(cs) = &req.charset {
        check_identifier(cs, "charset")?;
    }
    if let Some(co) = &req.collation {
        check_identifier(co, "collation")?;
    }
    Ok(())
}

fn validate_alter_table(req: &AlterTableRequest) -> Result<()> {
    if req.operations.is_empty() {
        return Err(AppError::BadRequest(
            "ALTER TABLE requires at least one operation".into(),
        ));
    }
    for op in &req.operations {
        match op {
            AlterTableOperation::AddColumn { column, after } => {
                validate_column_definition(column)?;
                if let Some(col) = after {
                    check_identifier(col, "AFTER column")?;
                }
            }
            AlterTableOperation::DropColumn { name } => {
                check_identifier(name, "column")?;
            }
            AlterTableOperation::ModifyColumn { column } => {
                validate_column_definition(column)?;
            }
            AlterTableOperation::RenameColumn { from, to } => {
                check_identifier(from, "column")?;
                check_identifier(to, "column")?;
            }
            AlterTableOperation::RenameTable { to } => {
                check_identifier(to, "table")?;
            }
        }
    }
    Ok(())
}

fn validate_column_definition(col: &ColumnDefinition) -> Result<()> {
    check_identifier(&col.name, "column")?;
    check_sql_fragment(&col.data_type, "data type")?;
    if let Some(d) = &col.default {
        check_sql_fragment(d, "default")?;
    }
    Ok(())
}

// ---- pure builders (unit-tested) ----------------------------------------

pub fn build_create_database_sql(req: &CreateDatabaseRequest) -> String {
    let mut sql = String::from("CREATE DATABASE ");
    if req.if_not_exists {
        sql.push_str("IF NOT EXISTS ");
    }
    sql.push_str(&quote_identifier(&req.name));
    if let Some(cs) = &req.charset {
        sql.push_str(" CHARACTER SET ");
        sql.push_str(cs);
    }
    if let Some(co) = &req.collation {
        sql.push_str(" COLLATE ");
        sql.push_str(co);
    }
    sql
}

pub fn build_drop_database_sql(db: &str, req: &DropDatabaseRequest) -> String {
    let mut sql = String::from("DROP DATABASE ");
    if req.if_exists {
        sql.push_str("IF EXISTS ");
    }
    sql.push_str(&quote_identifier(db));
    sql
}

pub fn build_create_table_sql(db: &str, req: &CreateTableRequest) -> String {
    let qualified = format!("{}.{}", quote_identifier(db), quote_identifier(&req.name));
    let mut sql = String::from("CREATE TABLE ");
    if req.if_not_exists {
        sql.push_str("IF NOT EXISTS ");
    }
    sql.push_str(&qualified);
    sql.push_str(" (\n");

    let mut parts: Vec<String> = req
        .columns
        .iter()
        .map(|c| format!("  {}", render_column_definition(c)))
        .collect();
    if !req.primary_key.is_empty() {
        let cols = req
            .primary_key
            .iter()
            .map(|c| quote_identifier(c))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(format!("  PRIMARY KEY ({cols})"));
    }
    sql.push_str(&parts.join(",\n"));
    sql.push_str("\n)");

    if let Some(engine) = &req.engine {
        sql.push_str(" ENGINE=");
        sql.push_str(engine);
    }
    if let Some(cs) = &req.charset {
        sql.push_str(" DEFAULT CHARSET=");
        sql.push_str(cs);
    }
    if let Some(co) = &req.collation {
        sql.push_str(" COLLATE=");
        sql.push_str(co);
    }
    if let Some(c) = &req.comment {
        sql.push_str(" COMMENT='");
        sql.push_str(&escape_sql_string_literal(c));
        sql.push('\'');
    }
    sql
}

pub fn build_alter_table_sql(db: &str, table: &str, req: &AlterTableRequest) -> String {
    let qualified = format!("{}.{}", quote_identifier(db), quote_identifier(table));
    let clauses = req
        .operations
        .iter()
        .map(render_alter_clause)
        .collect::<Vec<_>>()
        .join(", ");
    format!("ALTER TABLE {qualified} {clauses}")
}

pub fn build_drop_table_sql(db: &str, table: &str, req: &DropTableRequest) -> String {
    let mut sql = String::from("DROP TABLE ");
    if req.if_exists {
        sql.push_str("IF EXISTS ");
    }
    sql.push_str(&format!(
        "{}.{}",
        quote_identifier(db),
        quote_identifier(table)
    ));
    sql
}

fn render_column_definition(col: &ColumnDefinition) -> String {
    let mut out = String::new();
    out.push_str(&quote_identifier(&col.name));
    out.push(' ');
    out.push_str(&col.data_type);
    out.push_str(if col.nullable { " NULL" } else { " NOT NULL" });
    if col.auto_increment {
        out.push_str(" AUTO_INCREMENT");
    }
    if let Some(d) = &col.default {
        out.push_str(" DEFAULT ");
        out.push_str(d);
    }
    if let Some(c) = &col.comment {
        out.push_str(" COMMENT '");
        out.push_str(&escape_sql_string_literal(c));
        out.push('\'');
    }
    out
}

fn render_alter_clause(op: &AlterTableOperation) -> String {
    match op {
        AlterTableOperation::AddColumn { column, after } => {
            let mut s = format!("ADD COLUMN {}", render_column_definition(column));
            if let Some(col) = after {
                s.push_str(&format!(" AFTER {}", quote_identifier(col)));
            }
            s
        }
        AlterTableOperation::DropColumn { name } => {
            format!("DROP COLUMN {}", quote_identifier(name))
        }
        AlterTableOperation::ModifyColumn { column } => {
            format!("MODIFY COLUMN {}", render_column_definition(column))
        }
        AlterTableOperation::RenameColumn { from, to } => format!(
            "RENAME COLUMN {} TO {}",
            quote_identifier(from),
            quote_identifier(to)
        ),
        AlterTableOperation::RenameTable { to } => {
            format!("RENAME TO {}", quote_identifier(to))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn col(name: &str, ty: &str) -> ColumnDefinition {
        ColumnDefinition {
            name: name.into(),
            data_type: ty.into(),
            nullable: false,
            default: None,
            auto_increment: false,
            comment: None,
        }
    }

    #[test]
    fn create_database_basic() {
        let sql = build_create_database_sql(&CreateDatabaseRequest {
            name: "demo".into(),
            charset: None,
            collation: None,
            if_not_exists: false,
        });
        assert_eq!(sql, "CREATE DATABASE `demo`");
    }

    #[test]
    fn create_database_with_options() {
        let sql = build_create_database_sql(&CreateDatabaseRequest {
            name: "demo".into(),
            charset: Some("utf8mb4".into()),
            collation: Some("utf8mb4_0900_ai_ci".into()),
            if_not_exists: true,
        });
        assert_eq!(
            sql,
            "CREATE DATABASE IF NOT EXISTS `demo` CHARACTER SET utf8mb4 COLLATE utf8mb4_0900_ai_ci"
        );
    }

    #[test]
    fn drop_database_emits_backticked_name() {
        let sql = build_drop_database_sql(
            "demo",
            &DropDatabaseRequest {
                if_exists: true,
            },
        );
        assert_eq!(sql, "DROP DATABASE IF EXISTS `demo`");
    }

    #[test]
    fn create_table_single_column_pk() {
        let req = CreateTableRequest {
            name: "actor".into(),
            columns: vec![ColumnDefinition {
                name: "id".into(),
                data_type: "INT UNSIGNED".into(),
                nullable: false,
                default: None,
                auto_increment: true,
                comment: None,
            }],
            primary_key: vec!["id".into()],
            engine: Some("InnoDB".into()),
            charset: Some("utf8mb4".into()),
            collation: None,
            comment: None,
            if_not_exists: false,
        };
        let sql = build_create_table_sql("db", &req);
        assert_eq!(
            sql,
            "CREATE TABLE `db`.`actor` (\n  `id` INT UNSIGNED NOT NULL AUTO_INCREMENT,\n  PRIMARY KEY (`id`)\n) ENGINE=InnoDB DEFAULT CHARSET=utf8mb4"
        );
    }

    #[test]
    fn create_table_renders_default_and_comment() {
        let req = CreateTableRequest {
            name: "t".into(),
            columns: vec![ColumnDefinition {
                name: "name".into(),
                data_type: "VARCHAR(64)".into(),
                nullable: true,
                default: Some("'anon'".into()),
                auto_increment: false,
                comment: Some("user's display".into()),
            }],
            primary_key: vec![],
            engine: None,
            charset: None,
            collation: None,
            comment: Some("audit's table".into()),
            if_not_exists: true,
        };
        let sql = build_create_table_sql("db", &req);
        assert_eq!(
            sql,
            "CREATE TABLE IF NOT EXISTS `db`.`t` (\n  `name` VARCHAR(64) NULL DEFAULT 'anon' COMMENT 'user''s display'\n) COMMENT='audit''s table'"
        );
    }

    #[test]
    fn alter_table_add_drop_modify() {
        let req = AlterTableRequest {
            operations: vec![
                AlterTableOperation::AddColumn {
                    column: ColumnDefinition {
                        name: "added_at".into(),
                        data_type: "DATETIME".into(),
                        nullable: false,
                        default: Some("CURRENT_TIMESTAMP".into()),
                        auto_increment: false,
                        comment: None,
                    },
                    after: Some("id".into()),
                },
                AlterTableOperation::DropColumn {
                    name: "legacy".into(),
                },
                AlterTableOperation::ModifyColumn {
                    column: col("email", "VARCHAR(255)"),
                },
                AlterTableOperation::RenameColumn {
                    from: "old_name".into(),
                    to: "new_name".into(),
                },
                AlterTableOperation::RenameTable {
                    to: "actor_v2".into(),
                },
            ],
        };
        let sql = build_alter_table_sql("db", "actor", &req);
        assert_eq!(
            sql,
            "ALTER TABLE `db`.`actor` \
ADD COLUMN `added_at` DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP AFTER `id`, \
DROP COLUMN `legacy`, \
MODIFY COLUMN `email` VARCHAR(255) NOT NULL, \
RENAME COLUMN `old_name` TO `new_name`, \
RENAME TO `actor_v2`"
        );
    }

    #[test]
    fn drop_table_basic() {
        let sql = build_drop_table_sql(
            "db",
            "t",
            &DropTableRequest {
                if_exists: false,
            },
        );
        assert_eq!(sql, "DROP TABLE `db`.`t`");
    }

    #[test]
    fn drop_table_if_exists() {
        let sql = build_drop_table_sql(
            "db",
            "t",
            &DropTableRequest { if_exists: true },
        );
        assert_eq!(sql, "DROP TABLE IF EXISTS `db`.`t`");
    }

    #[test]
    fn validate_create_table_rejects_empty_columns() {
        let req = CreateTableRequest {
            name: "t".into(),
            columns: vec![],
            primary_key: vec![],
            engine: None,
            charset: None,
            collation: None,
            comment: None,
            if_not_exists: false,
        };
        assert!(validate_create_table(&req).is_err());
    }

    #[test]
    fn validate_create_table_rejects_unknown_pk() {
        let req = CreateTableRequest {
            name: "t".into(),
            columns: vec![col("id", "INT")],
            primary_key: vec!["missing".into()],
            engine: None,
            charset: None,
            collation: None,
            comment: None,
            if_not_exists: false,
        };
        assert!(validate_create_table(&req).is_err());
    }

    #[test]
    fn validate_create_table_rejects_duplicate_columns() {
        let req = CreateTableRequest {
            name: "t".into(),
            columns: vec![col("id", "INT"), col("id", "BIGINT")],
            primary_key: vec![],
            engine: None,
            charset: None,
            collation: None,
            comment: None,
            if_not_exists: false,
        };
        assert!(validate_create_table(&req).is_err());
    }

    #[test]
    fn validate_create_table_rejects_injected_type() {
        let req = CreateTableRequest {
            name: "t".into(),
            columns: vec![col("id", "INT; DROP TABLE actor")],
            primary_key: vec![],
            engine: None,
            charset: None,
            collation: None,
            comment: None,
            if_not_exists: false,
        };
        assert!(validate_create_table(&req).is_err());
    }

    #[test]
    fn validate_alter_table_rejects_empty_ops() {
        assert!(
            validate_alter_table(&AlterTableRequest { operations: vec![] }).is_err()
        );
    }
}
