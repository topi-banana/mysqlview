use std::net::SocketAddr;

use anyhow::Context;
use clap::Parser;
use tokio::net::TcpListener;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod cli;
mod cors;
mod db;
#[cfg(feature = "embedded-frontend")]
mod embedded;
mod error;
mod healthcheck;
mod routes;
mod state;
mod static_files;
mod validate;

use crate::cli::Cli;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if cli.healthcheck {
        std::process::exit(if healthcheck::run(cli.port) { 0 } else { 1 });
    }

    init_tracing();

    let addr: SocketAddr = format!("{}:{}", cli.bind, cli.port)
        .parse()
        .with_context(|| format!("invalid bind address {}:{}", cli.bind, cli.port))?;

    if !is_loopback(&addr) {
        warn!(
            "mysqlview is binding to {} which is NOT a loopback address. \
             This tool has NO authentication and is intended for local development only. \
             Do NOT expose it to the internet or shared networks.",
            addr
        );
    }

    let database_url = cli
        .database_url
        .as_deref()
        .context("DATABASE_URI environment variable or --database-url is required")?;

    let state = AppState::new(database_url, cli.max_rows)
        .await
        .context("failed to initialize MySQL connection pool")?;

    let router = routes::router(state.clone())
        .layer(cors::layer())
        .layer(tower_http::trace::TraceLayer::new_for_http());

    let router = static_files::attach(router, cli.frontend_dist.as_deref());

    let listener = TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;

    info!("mysqlview listening on http://{}", addr);
    axum::serve(listener, router.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server error")?;

    Ok(())
}

fn init_tracing() {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn,tower_http=info"));
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .compact()
        .init();
}

fn is_loopback(addr: &SocketAddr) -> bool {
    addr.ip().is_loopback()
}

/// Wait until the process is asked to shut down.
///
/// When the binary runs as PID 1 inside a scratch container the Linux
/// kernel drops every signal that has no userspace handler installed, so
/// Ctrl-C and `docker stop` would otherwise hang. The explicit
/// `signal()` calls below register handlers for both SIGINT and SIGTERM
/// so either path terminates the process cleanly.
#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM handler");

    tokio::select! {
        _ = sigint.recv() => info!("SIGINT received, shutting down"),
        _ = sigterm.recv() => info!("SIGTERM received, shutting down"),
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    signal::ctrl_c().await.expect("install Ctrl-C handler");
    info!("Ctrl-C received, shutting down");
}
