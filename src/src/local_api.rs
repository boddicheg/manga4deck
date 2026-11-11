use axum::{
    extract::Request,
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use once_cell::sync::Lazy;

// Created once per process, never persisted or included in API responses/logs.
static TOKEN: Lazy<String> = Lazy::new(|| {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("Failed to generate local API credentials");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
});

pub fn media_prefix() -> String {
    format!("/api/media/{}", *TOKEN)
}

pub fn media_url(path: &str) -> String {
    format!("http://127.0.0.1:11337{}/{}", media_prefix(), path)
}

pub fn is_authorized(headers: &HeaderMap) -> bool {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value == authorization_header())
}

pub(crate) fn authorization_header() -> String {
    format!("Bearer {}", *TOKEN)
}

pub async fn authorize(request: Request, next: Next) -> Response {
    if !is_authorized(request.headers()) {
        return (
            StatusCode::UNAUTHORIZED,
            "Local API authentication required",
        )
            .into_response();
    }
    next.run(request).await
}
