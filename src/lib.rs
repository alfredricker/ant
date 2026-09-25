//! ant — one crate, two builds (see Cargo.toml):
//!
//! - `models`, `api` and `ui` compile into both. `api` holds the server
//!   functions: on the server they run, in the browser they become fetches.
//! - `server` is server-only: axum routes, auth, sessions and the database.
pub mod api;
pub mod models;
pub mod ui;

#[cfg(feature = "server")]
pub mod server;
