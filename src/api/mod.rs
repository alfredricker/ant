//! Server functions. Each is an HTTP endpoint on the server and a typed fetch
//! in the browser; during server-side rendering it's a plain function call.
//!
//! These are public endpoints like any other: anything that needs a signed-in
//! user takes `user: CurrentUser` (401 otherwise) in the macro arguments.
//! Extractors and bodies only compile in the server build, so they can use
//! `crate::server` freely.

use dioxus::prelude::*;

use crate::models::{health::Health, user::User};

#[cfg(feature = "server")]
use crate::server::{auth::session::MaybeUser, routes::health, state::AppState};
#[cfg(feature = "server")]
use axum::Extension;

/// Who's signed in, if anyone.
#[get("/api/auth/me", user: MaybeUser)]
pub async fn current_user() -> Result<Option<User>> {
    Ok(user.0)
}

/// API and database health, for the status line on the homepage.
#[get("/api/status", state: Extension<AppState>)]
pub async fn status() -> Result<Health> {
    Ok(health::check(&state.db).await)
}
