use std::path::PathBuf;

use clap::Parser;

/// Local-only MySQL WebUI server.
///
/// Connection details come from the `DATABASE_URI` environment variable
/// (or `--database-url`). The server binds to 127.0.0.1 by default and
/// has NO authentication — do not expose it to the public internet.
#[derive(Debug, Clone, Parser)]
#[command(name = "mysqlview", version, about, long_about = None)]
pub struct Cli {
    /// Probe the running server (127.0.0.1:port/api/health) and exit
    /// 0 if healthy. Used by Docker HEALTHCHECK and similar tooling.
    /// Skips the rest of the CLI (no MySQL connection is opened).
    #[arg(long)]
    pub healthcheck: bool,

    /// IP address to bind. Default: 127.0.0.1.
    #[arg(long, default_value = "127.0.0.1", env = "MYSQLVIEW_BIND")]
    pub bind: String,

    /// Port to listen on.
    #[arg(long, default_value_t = 3000, env = "MYSQLVIEW_PORT")]
    pub port: u16,

    /// MySQL connection URI, e.g. `mysql://user:pass@127.0.0.1:3306/dbname`.
    /// Required for serving; ignored when `--healthcheck` is set.
    #[arg(long, env = "DATABASE_URI")]
    pub database_url: Option<String>,

    /// Directory holding the built frontend (`trunk build --release` output).
    /// When provided, the server also serves static files from this path.
    #[arg(long, env = "MYSQLVIEW_FRONTEND_DIST")]
    pub frontend_dist: Option<PathBuf>,

    /// Maximum number of rows returned by any single query.
    #[arg(long, default_value_t = 1000, env = "MYSQLVIEW_MAX_ROWS")]
    pub max_rows: u32,

    /// Maximum size (in bytes) accepted by the CSV / SQL import endpoints.
    /// Default 100 MiB. Other routes keep axum's default 2 MiB ceiling.
    #[arg(
        long,
        default_value_t = 100 * 1024 * 1024,
        env = "MYSQLVIEW_MAX_IMPORT_BYTES"
    )]
    pub max_import_bytes: usize,
}
