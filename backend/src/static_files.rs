use std::path::Path;

use axum::Router;
use tower_http::services::{ServeDir, ServeFile};

pub fn attach(router: Router, frontend_dist: Option<&Path>) -> Router {
    let Some(dist) = frontend_dist else {
        return router;
    };
    if !dist.exists() {
        tracing::warn!(
            "frontend dist directory {} does not exist; skipping static file mount",
            dist.display()
        );
        return router;
    }
    let index = dist.join("index.html");
    let serve = ServeDir::new(dist).not_found_service(ServeFile::new(index));
    router.fallback_service(serve)
}
