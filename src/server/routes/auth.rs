//! `/api/auth`: Google sign-in and sign-out. Browser-redirect flows, so plain
//! axum routes; "who's signed in" is the `current_user` server function.

use axum::{
    Router,
    extract::{Query, State},
    http::StatusCode,
    response::Redirect,
    routing::{get, post},
};
use axum_extra::extract::{
    CookieJar, PrivateCookieJar,
    cookie::{Cookie, SameSite},
};
use openidconnect::{
    AuthorizationCode, CsrfToken, Nonce, PkceCodeChallenge, PkceCodeVerifier, Scope, TokenResponse,
    core::CoreAuthenticationFlow,
};
use serde::{Deserialize, Serialize};

use crate::server::{
    auth::session::{self, SESSION_COOKIE},
    db,
    error::ApiError,
    state::AppState,
};

/// Holds the CSRF state, nonce and PKCE verifier between the redirect to
/// Google and the callback. Encrypted, scoped to the Google routes, 10 minutes.
const FLOW_COOKIE: &str = "ant_oidc";
const FLOW_PATH: &str = "/api/auth/google";

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/google/login", get(google_login))
        .route("/google/callback", get(google_callback))
        .route("/logout", post(logout))
}

#[derive(Serialize, Deserialize)]
struct Flow {
    csrf: String,
    nonce: String,
    pkce_verifier: String,
}

fn not_configured() -> ApiError {
    ApiError::new(StatusCode::SERVICE_UNAVAILABLE, "Google sign-in is not configured")
}

fn google_failed() -> ApiError {
    ApiError::new(StatusCode::BAD_GATEWAY, "Google sign-in failed, please try again")
}

async fn google_login(
    State(state): State<AppState>,
    jar: PrivateCookieJar,
) -> Result<(PrivateCookieJar, Redirect), ApiError> {
    let google = state.google.as_ref().ok_or_else(not_configured)?;
    let client = google.client().await;

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    // The `openid` scope is added by the crate; `email` gets us the address.
    let (auth_url, csrf, nonce) = client
        .authorize_url(
            CoreAuthenticationFlow::AuthorizationCode,
            CsrfToken::new_random,
            Nonce::new_random,
        )
        .add_scope(Scope::new("email".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let flow = Flow {
        csrf: csrf.secret().clone(),
        nonce: nonce.secret().clone(),
        pkce_verifier: pkce_verifier.secret().clone(),
    };
    let flow_json = serde_json::to_string(&flow).expect("Flow serializes");
    let cookie = Cookie::build((FLOW_COOKIE, flow_json))
        .path(FLOW_PATH)
        .http_only(true)
        .secure(state.secure_cookies)
        // Lax still sends it on the top-level redirect back from Google.
        .same_site(SameSite::Lax)
        .max_age(time::Duration::minutes(10))
        .build();

    Ok((jar.add(cookie), Redirect::to(auth_url.as_str())))
}

#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    /// Set instead of `code` when the user cancels on Google's page.
    error: Option<String>,
}

async fn google_callback(
    State(state): State<AppState>,
    flow_jar: PrivateCookieJar,
    session_jar: CookieJar,
    Query(params): Query<CallbackParams>,
) -> Result<(PrivateCookieJar, CookieJar, Redirect), ApiError> {
    let google = state.google.as_ref().ok_or_else(not_configured)?;

    let expired = || ApiError::new(StatusCode::BAD_REQUEST, "sign-in expired, please try again");
    let flow: Flow = flow_jar
        .get(FLOW_COOKIE)
        .and_then(|c| serde_json::from_str(c.value()).ok())
        .ok_or_else(expired)?;
    // Single use, whatever happens next.
    let flow_jar = flow_jar.remove(Cookie::build(FLOW_COOKIE).path(FLOW_PATH));

    if params.error.is_some() {
        return Ok((flow_jar, session_jar, Redirect::to("/")));
    }
    // CSRF: the callback must answer the sign-in this browser started.
    if params.state.as_deref() != Some(flow.csrf.as_str()) {
        return Err(expired());
    }
    let code = params.code.ok_or_else(expired)?;

    let client = google.client().await;
    let token = client
        .exchange_code(AuthorizationCode::new(code))
        .map_err(|err| {
            tracing::error!("Google discovery has no token endpoint: {err}");
            google_failed()
        })?
        .set_pkce_verifier(PkceCodeVerifier::new(flow.pkce_verifier))
        .request_async(&google.http)
        .await
        .map_err(|err| {
            tracing::warn!("Google code exchange failed: {err}");
            google_failed()
        })?;

    let id_token = token.id_token().ok_or_else(|| {
        tracing::warn!("Google returned no ID token");
        google_failed()
    })?;
    // Checks signature, issuer, audience, expiry and our nonce.
    let claims = id_token
        .claims(&client.id_token_verifier(), &Nonce::new(flow.nonce))
        .map_err(|err| {
            tracing::warn!("Google ID token rejected: {err}");
            google_failed()
        })?;

    let email = match (claims.email(), claims.email_verified()) {
        (Some(email), Some(true)) => email.as_str().to_owned(),
        _ => {
            return Err(ApiError::new(
                StatusCode::FORBIDDEN,
                "your Google account needs a verified email",
            ));
        }
    };
    let subject = claims.subject().as_str().to_owned();

    let user = db::users::find_or_create_from_identity(&state.db, "google", &subject, &email).await?;
    let (token, token_hash) = session::new_token();
    db::sessions::create(&state.db, &token_hash, user.id, session::SESSION_DAYS).await?;
    tracing::info!(user_id = %user.id, "signed in with Google");

    let session_jar = session_jar.add(session::session_cookie(token, state.secure_cookies));
    Ok((flow_jar, session_jar, Redirect::to("/")))
}

/// Posted by a plain form, so sign-out works before the wasm has loaded;
/// lands back on the homepage.
async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Redirect), ApiError> {
    if let Some(cookie) = jar.get(SESSION_COOKIE) {
        db::sessions::delete(&state.db, &session::hash_token(cookie.value())).await?;
    }
    Ok((jar.remove(session::clear_session_cookie()), Redirect::to("/")))
}
