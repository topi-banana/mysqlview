use std::net::SocketAddr;

use anyhow::Context;
use clap::Parser;
use tokio::net::TcpListener;
use tokio::signal;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

mod cli;
mod cors;
mod db;
mod error;
mod routes;
mod state;
mod static_files;
mod validate;

use crate::cli::Cli;
use crate::state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let cli = Cli::parse();
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

    let state = AppState::new(&cli.database_url, cli.max_rows)
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

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl-C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
    info!("shutdown signal received");
}
