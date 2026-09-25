use std::sync::Arc;

use axum::extract::FromRef;
use axum_extra::extract::cookie::Key;
use sqlx::PgPool;

use crate::server::auth::google::Google;

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    /// `None` when Google sign-in isn't configured.
    pub google: Option<Arc<Google>>,
    /// Encrypts the short-lived cookie that carries sign-in state across the
    /// round trip to Google. Generated per process: a restart only interrupts
    /// sign-ins in flight. Multiple instances would need a shared key.
    pub cookie_key: Key,
    /// Mark cookies `Secure`; set when PUBLIC_URL is https.
    pub secure_cookies: bool,
}

// Lets PrivateCookieJar find the key in the state.
impl FromRef<AppState> for Key {
    fn from_ref(state: &AppState) -> Self {
        state.cookie_key.clone()
    }
}
