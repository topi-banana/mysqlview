use axum::http::{HeaderValue, Method, header};
use tower_http::cors::CorsLayer;

pub fn layer() -> CorsLayer {
    let origins: Vec<HeaderValue> = [
        "http://127.0.0.1:8080",
        "http://localhost:8080",
        "http://127.0.0.1:3000",
        "http://localhost:3000",
    ]
    .into_iter()
    .filter_map(|s| HeaderValue::from_str(s).ok())
    .collect();

    CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT])
}
