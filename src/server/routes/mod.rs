//! Plain axum routes, for the endpoints that aren't server functions: the
//! container healthcheck (needs a real 503), the sign-in redirects and forms,
//! avatar uploads, and serving uploaded images.
use axum::{
    Router,
    http::{HeaderValue, header},
    response::{IntoResponse, Response},
    routing::get,
};

use crate::server::state::AppState;

pub mod account;
pub mod auth;
pub mod health;
pub mod post_images;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health::health))
        .nest("/api/auth", auth::router())
        .merge(account::router())
        .merge(post_images::router())
}

/// An image we stored (see `server::images`). Its URL never serves other
/// bytes, so it can be cached for good.
fn image_response(content_type: &str, data: Vec<u8>) -> Response {
    // Stored by us, so only ever these two.
    let content_type = if content_type == "image/png" { "image/png" } else { "image/jpeg" };
    let headers = [
        (header::CONTENT_TYPE, HeaderValue::from_static(content_type)),
        (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=31536000, immutable")),
        (header::X_CONTENT_TYPE_OPTIONS, HeaderValue::from_static("nosniff")),
        (header::CONTENT_SECURITY_POLICY, HeaderValue::from_static("default-src 'none'; sandbox")),
    ];
    (headers, data).into_response()
}
