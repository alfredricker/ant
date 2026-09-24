use axum::{Router, routing::get};

use crate::state::AppState;

pub mod auth;
pub mod error;
pub mod health;

/// Everything under `/api`.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health", get(health::health))
        .nest("/auth", auth::router())
}
