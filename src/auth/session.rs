//! Cookie sessions backed by the `sessions` table.

use ant_common::models::user::User;
use axum::{extract::FromRequestParts, http::request::Parts};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use sha2::{Digest, Sha256};

use crate::{api::error::ApiError, db, state::AppState};

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

/// Extractor for handlers that need a signed-in user; answers 401 otherwise.
pub struct CurrentUser(pub User);

impl FromRequestParts<AppState> for CurrentUser {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, state: &AppState) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_headers(&parts.headers);
        let token = jar.get(SESSION_COOKIE).ok_or_else(ApiError::unauthorized)?;

        db::sessions::find_user(&state.db, &hash_token(token.value()))
            .await?
            .map(CurrentUser)
            .ok_or_else(ApiError::unauthorized)
    }
}
