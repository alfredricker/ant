//! Plain axum routes, for the endpoints that aren't server functions: the
//! container healthcheck (needs a real 503), the sign-in redirects and forms,
//! and avatar uploads and images.
use axum::{Router, routing::get};

use crate::server::state::AppState;

pub mod account;
pub mod auth;
pub mod health;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health::health))
        .nest("/api/auth", auth::router())
        .merge(account::router())
}
