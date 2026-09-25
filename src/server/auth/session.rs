//! Cookie sessions backed by the `sessions` table.

use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

use crate::{
    models::user::User,
    server::{db, error::ApiError, state::AppState},
};

pub const SESSION_COOKIE: &str = "ant_session";
pub const SESSION_DAYS: i32 = 30;

/// A fresh session token for the cookie, and the hash to store for it.
pub fn new_token() -> (String, Vec<u8>) {
    let token = URL_SAFE_NO_PAD.encode(rand::random::<[u8; 32]>());
    let hash = hash_token(&token);
    (token, hash)
}

pub fn hash_token(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub fn session_cookie(token: String, secure: bool) -> Cookie<'static> {
    Cookie::build((SESSION_COOKIE, token))
        .path("/")
        .http_only(true)
        .secure(secure)
        .same_site(SameSite::Lax)
        .max_age(time::Duration::days(SESSION_DAYS.into()))
        .build()
}

pub fn clear_session_cookie() -> Cookie<'static> {
    Cookie::build(SESSION_COOKIE).path("/").build()
}

/// Extractor for handlers and server functions that need a signed-in user;
/// answers 401 otherwise.
pub struct CurrentUser(pub User);

/// The signed-in user, if any. Never rejects for being signed out.
pub struct MaybeUser(pub Option<User>);

// Generic over the state because server functions extract with Dioxus's own
// state type, not ours; the app state comes from the `Extension` layer that
// `server::router` adds instead.
impl<S: Send + Sync> FromRequestParts<S> for MaybeUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _: &S) -> Result<Self, Self::Rejection> {
        let state = parts.extensions.get::<AppState>().ok_or_else(|| {
            tracing::error!("AppState extension missing; is the route behind server::router?");
            ApiError::internal()
        })?;
        let jar = CookieJar::from_headers(&parts.headers);
        let Some(token) = jar.get(SESSION_COOKIE) else {
            return Ok(MaybeUser(None));
        };

        let user = db::sessions::find_user(&state.db, &hash_token(token.value())).await?;
        Ok(MaybeUser(user))
    }
}

impl<S: Send + Sync> FromRequestParts<S> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        MaybeUser::from_request_parts(parts, state)
            .await?
            .0
            .map(CurrentUser)
            .ok_or_else(ApiError::unauthorized)
    }
}
