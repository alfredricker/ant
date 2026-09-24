//! ant — HTTP server.
//!
//! Serves the Trunk-built yew bundle from `dist/` and the JSON API under
//! `/api`. In dev it sits behind nginx (see build/dev), so it binds plain HTTP
//! on `APP_PORT` (6353) and trusts the proxy for rate limiting.

use std::{env, time::Duration};

use ant::{api::health::health, state::AppState};
use axum::{Router, routing::get};
use sqlx::postgres::PgPoolOptions;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};

const DEFAULT_PORT: u16 = 6353;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ant=debug,tower_http=debug,info".into()),
        )
        .init();

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

    let state = AppState { db };

    let dist_dir = env::var("ANT_DIST_DIR").unwrap_or_else(|_| "dist".to_string());
    let index_html = format!("{dist_dir}/index.html");

    // Anything not matched by a route is either a static asset or a client-side
    // route, so unknown paths fall back to index.html for the yew router.
    let static_files = ServeDir::new(&dist_dir)
        .append_index_html_on_directories(true)
        .not_found_service(ServeFile::new(&index_html));

    let app = Router::new()
        .route("/api/health", get(health))
        .fallback_service(static_files)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let port: u16 = env::var("APP_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(DEFAULT_PORT);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("ant listening on http://0.0.0.0:{port} (serving {dist_dir})");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutting down");
}