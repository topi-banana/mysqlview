use std::time::Duration;

use anyhow::Context;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::{MySql, Pool};

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<MySql>,
    pub max_rows: u32,
    pub max_import_bytes: usize,
}

impl AppState {
    pub async fn new(
        database_url: &str,
        max_rows: u32,
        max_import_bytes: usize,
    ) -> anyhow::Result<Self> {
        let pool = MySqlPoolOptions::new()
            .max_connections(8)
            .acquire_timeout(Duration::from_secs(10))
            .connect(database_url)
            .await
            .context("failed to connect to MySQL")?;

        sqlx::query("SELECT 1")
            .execute(&pool)
            .await
            .context("MySQL ping failed")?;

        Ok(Self {
            pool,
            max_rows: max_rows.max(1),
            max_import_bytes,
        })
    }
}
