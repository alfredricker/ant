//! The signed-in user's own account.
//!
//! Anything that checks a password lives under `/api/auth/`, so nginx's
//! tighter auth rate limit covers guessing through it too.

use dioxus::prelude::*;

use crate::models::{
    account::{AccountDetails, Theme},
    user::User,
};

#[cfg(feature = "server")]
use crate::{
    models::auth,
    server::{
        auth::{
            password,
            session::{self, CurrentUser, MaybeUser, SESSION_COOKIE},
        },
        db,
        error::{OrInternal, bad_request},
        state::AppState,
    },
};
#[cfg(feature = "server")]
use axum::Extension;
#[cfg(feature = "server")]
use axum_extra::extract::CookieJar;

/// Everything the account page shows. `None` when signed out.
#[get("/api/account", user: MaybeUser, state: Extension<AppState>)]
pub async fn account_details() -> Result<Option<AccountDetails>, HttpError> {
    let Some(user) = user.0 else {
        return Ok(None);
    };
    let db = &state.db;
    Ok(Some(AccountDetails {
        has_password: db::account::has_password(db, user.id).await.or_internal()?,
        google_linked: db::account::has_identity(db, user.id, "google").await.or_internal()?,
        theme: db::account::theme(db, user.id).await.or_internal()?,
        user,
    }))
}

/// The theme to render with: the user's choice, or the default when signed
/// out.
#[get("/api/account/theme", user: MaybeUser, state: Extension<AppState>)]
pub async fn theme() -> Result<Theme, HttpError> {
    match user.0 {
        Some(user) => db::account::theme(&state.db, user.id).await.or_internal(),
        None => Ok(Theme::default()),
    }
}

#[post("/api/account/theme", user: CurrentUser, state: Extension<AppState>)]
pub async fn set_theme(theme: Theme) -> Result<(), HttpError> {
    db::account::set_theme(&state.db, user.0.id, theme).await.or_internal()
}

/// Pick or change your username. Returns the updated user.
#[post("/api/account/username", user: CurrentUser, state: Extension<AppState>)]
pub async fn set_username(username: String) -> Result<User, HttpError> {
    let username = username.trim();
    User::validate_username(username).map_err(bad_request)?;

    let updated = db::users::set_username(&state.db, user.0.id, username).await.or_internal()?;
    updated.ok_or_else(|| HttpError::new(StatusCode::CONFLICT, "that username is taken"))
}

/// Change your email. Accounts with a password confirm it first, since the
/// email is what signs them in. The new address counts as unverified.
#[post("/api/auth/email", user: CurrentUser, state: Extension<AppState>)]
pub async fn change_email(email: String, current_password: Option<String>) -> Result<User, HttpError> {
    let email = email.trim();
    auth::check_email(email).map_err(|err| bad_request(err.message().into()))?;
    confirm_password(&state, user.0.id, current_password).await?;

    let updated = db::users::set_email(&state.db, user.0.id, email).await.or_internal()?;
    let updated = updated.ok_or_else(|| {
        HttpError::new(StatusCode::CONFLICT, "another account already uses that email")
    })?;
    tracing::info!(user_id = %updated.id, "changed email");
    Ok(updated)
}

/// Change your password, or set one on an account that signs in with Google
/// only. Signs out every other device.
#[post("/api/auth/password", user: CurrentUser, state: Extension<AppState>, jar: CookieJar)]
pub async fn change_password(current_password: Option<String>, new_password: String) -> Result<(), HttpError> {
    auth::check_password(&new_password).map_err(|err| bad_request(err.message().into()))?;
    confirm_password(&state, user.0.id, current_password).await?;

    let hash = password::hash(new_password).await.map_err(|_| internal())?;
    db::passwords::set(&state.db, user.0.id, &hash).await.or_internal()?;
    sign_out_others(&state, user.0.id, &jar).await?;
    tracing::info!(user_id = %user.0.id, "changed password");
    Ok(())
}

/// Ends every session but this one. Returns how many it ended.
#[post("/api/auth/sessions/others", user: CurrentUser, state: Extension<AppState>, jar: CookieJar)]
pub async fn sign_out_other_devices() -> Result<u64, HttpError> {
    sign_out_others(&state, user.0.id, &jar).await
}

#[cfg(feature = "server")]
fn internal() -> HttpError {
    HttpError::new(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
}

/// Ok when the account has no password, or `current` is it.
#[cfg(feature = "server")]
async fn confirm_password(state: &AppState, user_id: uuid::Uuid, current: Option<String>) -> Result<(), HttpError> {
    let Some(hash) = db::passwords::find_by_user(&state.db, user_id).await.or_internal()? else {
        return Ok(());
    };
    let current = current.unwrap_or_default();
    let wrong = || HttpError::new(StatusCode::FORBIDDEN, "your current password is wrong");
    if current.chars().count() > auth::PASSWORD_MAX_CHARS {
        return Err(wrong());
    }
    if password::verify(current, Some(hash)).await.map_err(|_| internal())? {
        Ok(())
    } else {
        tracing::info!(%user_id, "wrong current password on an account change");
        Err(wrong())
    }
}

#[cfg(feature = "server")]
async fn sign_out_others(state: &AppState, user_id: uuid::Uuid, jar: &CookieJar) -> Result<u64, HttpError> {
    // CurrentUser already found this cookie's session, so it's there.
    let token = jar.get(SESSION_COOKIE).map(|c| c.value().to_owned()).unwrap_or_default();
    db::sessions::delete_others(&state.db, user_id, &session::hash_token(&token))
        .await
        .or_internal()
}
