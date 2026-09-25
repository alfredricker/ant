//! Plain axum routes, for the endpoints that aren't server functions: the
//! container healthcheck (needs a real 503) and the Google sign-in redirects.
use axum::{Router, routing::get};

use crate::server::state::AppState;

pub mod auth;
pub mod health;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health::health))
        .nest("/api/auth", auth::router())
}
