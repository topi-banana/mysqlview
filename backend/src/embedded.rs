//! Static asset handler backed by `include_dir!`.
//!
//! Active only when the `embedded-frontend` feature is enabled. The macro
//! captures the workspace-root `dist/` at compile time as a `&'static Dir`,
//! and the `serve` handler streams files out with a content-type guessed
//! from the extension. Unknown paths fall back to `index.html` so the SPA
//! router on the client can take over.

use axum::body::Body;
use axum::http::{StatusCode, Uri, header};
use axum::response::Response;
use include_dir::{Dir, include_dir};

static FRONTEND_DIST: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../dist");

pub async fn serve(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    if let Some(file) = FRONTEND_DIST.get_file(path) {
        return build_response(path, file.contents());
    }
    // SPA fallback: any unmatched path returns index.html so yew-router's
    // BrowserRouter can resolve the route on the client.
    let index = FRONTEND_DIST
        .get_file("index.html")
        .expect("index.html is embedded — verified by build.rs");
    build_response("index.html", index.contents())
}

fn build_response(path: &str, contents: &'static [u8]) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, content_type_for(path))
        .body(Body::from(contents))
        .expect("response builder")
}

fn content_type_for(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "json" => "application/json; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::content_type_for;

    #[test]
    fn maps_common_extensions() {
        assert_eq!(content_type_for("index.html"), "text/html; charset=utf-8");
        assert_eq!(content_type_for("app.css"), "text/css; charset=utf-8");
        assert_eq!(
            content_type_for("app.js"),
            "application/javascript; charset=utf-8"
        );
        assert_eq!(content_type_for("frontend_bg.wasm"), "application/wasm");
        assert_eq!(
            content_type_for("data.json"),
            "application/json; charset=utf-8"
        );
        assert_eq!(content_type_for("logo.svg"), "image/svg+xml");
        assert_eq!(content_type_for("font.woff2"), "font/woff2");
    }

    #[test]
    fn unknown_extensions_use_octet_stream() {
        assert_eq!(content_type_for("noext"), "application/octet-stream");
        assert_eq!(content_type_for("file.weird"), "application/octet-stream");
    }

    #[test]
    fn extensions_are_case_insensitive() {
        assert_eq!(content_type_for("INDEX.HTML"), "text/html; charset=utf-8");
        assert_eq!(content_type_for("logo.SVG"), "image/svg+xml");
    }
}
