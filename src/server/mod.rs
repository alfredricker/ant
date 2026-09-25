//! Server-only code: never compiled into the wasm bundle.
pub mod auth;
pub mod db;
pub mod error;
pub mod routes;
pub mod state;

use std::{env, time::Duration};

use axum::{Extension, Router};
use axum_extra::extract::cookie::Key;
use sqlx::postgres::PgPoolOptions;
use tower_http::trace::TraceLayer;

use crate::server::{auth::google::Google, state::AppState};

/// Connects to postgres, applies migrations and assembles the app state.
pub async fn state() -> anyhow::Result<AppState> {
    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://ant:ant@localhost:5432/ant".to_string());

    // Lazy: the server comes up even if postgres is still starting, and
    // /api/health reports the connection state instead of the process dying.
    let db = PgPoolOptions::new()
        .max_connections(10)
        .acquire_timeout(Duration::from_secs(3))
        .connect_lazy(&database_url)?;

    // Same SQL in every environment: extensions and schema live in ./migrations
    sqlx::migrate!("./migrations").run(&db).await?;
    tracing::info!("migrations applied");

    // Where browsers reach us (nginx in dev); Google redirects back here.
    let public_url = env::var("PUBLIC_URL").unwrap_or_else(|_| "http://localhost:8035".to_string());

    Ok(AppState {
        db,
        google: Google::from_env(&public_url).await,
        cookie_key: Key::generate(),
        secure_cookies: public_url.starts_with("https://"),
    })
}

/// Wraps the Dioxus router (SSR, assets, server functions) with our own
/// axum routes, and hands every handler and server function the state as an
/// `Extension`.
pub fn router(app: Router, state: AppState) -> Router {
    app.merge(routes::router().with_state(state.clone()))
        .layer(Extension(state))
        .layer(TraceLayer::new_for_http())
}
