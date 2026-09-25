//! ant — entry point for both builds.
//!
//! Server: axum via `dioxus::serve`, which renders pages, serves the wasm
//! bundle and assets, and mounts the server functions; our own routes and
//! state are added by `server::router`. It binds `IP`:`PORT` (set by `dx`, or
//! the environment in production). In dev it sits behind nginx (see
//! build/dev) and trusts the proxy for rate limiting.
//!
//! Web: hydrates the server-rendered page.

use ant::ui::App;

#[cfg(feature = "server")]
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ant=debug,tower_http=debug,info".into()),
        )
        .init();

    dioxus::serve(|| async move {
        let state = ant::server::state().await?;
        Ok(ant::server::router(dioxus::server::router(App), state))
    });
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}
