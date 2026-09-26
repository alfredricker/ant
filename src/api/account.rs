//! The signed-in user's own account.

use dioxus::prelude::*;

use crate::models::user::User;

#[cfg(feature = "server")]
use crate::server::{
    auth::session::CurrentUser,
    db,
    error::{OrInternal, bad_request},
    state::AppState,
};
#[cfg(feature = "server")]
use axum::Extension;

/// Pick or change your username. Returns the updated user.
#[post("/api/account/username", user: CurrentUser, state: Extension<AppState>)]
pub async fn set_username(username: String) -> Result<User, HttpError> {
    let username = username.trim();
    User::validate_username(username).map_err(bad_request)?;

    let updated = db::users::set_username(&state.db, user.0.id, username).await.or_internal()?;
    updated.ok_or_else(|| HttpError::new(StatusCode::CONFLICT, "that username is taken"))
}
