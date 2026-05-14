use std::path::Path;

use axum::Router;

/// Attaches a fallback service that serves the frontend assets.
///
/// With the default feature set, the backend serves `frontend_dist` at
/// runtime via `tower_http::services::ServeDir`. With the `embedded-frontend`
/// feature, the assets are baked into the binary by `include_dir!` and
/// `frontend_dist` is ignored.
#[cfg(feature = "embedded-frontend")]
pub fn attach(router: Router, frontend_dist: Option<&Path>) -> Router {
    if frontend_dist.is_some() {
        tracing::info!("ignoring --frontend-dist; this binary has the frontend embedded");
    }
    router.fallback(crate::embedded::serve)
}

#[cfg(not(feature = "embedded-frontend"))]
pub fn attach(router: Router, frontend_dist: Option<&Path>) -> Router {
    use tower_http::services::{ServeDir, ServeFile};

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
