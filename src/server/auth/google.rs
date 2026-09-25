//! Sign in with Google via OpenID Connect.

use std::{
    env,
    sync::{Arc, RwLock},
    time::{Duration, Instant},
};

use openidconnect::{
    ClientId, ClientSecret, EndpointMaybeSet, EndpointNotSet, EndpointSet, IssuerUrl, RedirectUrl,
    core::{CoreClient, CoreProviderMetadata},
    reqwest,
};

const ISSUER: &str = "https://accounts.google.com";

/// Google rotates the keys it signs ID tokens with, and discovery is where we
/// learn them, so a long-running server re-discovers now and then.
const REFRESH_AFTER: Duration = Duration::from_secs(60 * 60);

/// What `CoreClient::from_provider_metadata` returns: the auth endpoint is
/// always known, token/userinfo depend on the discovery document.
pub type GoogleClient = CoreClient<
    EndpointSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointMaybeSet,
    EndpointMaybeSet,
>;

type Error = Box<dyn std::error::Error + Send + Sync>;

pub struct Google {
    /// Doesn't follow redirects, so discovery and the token exchange only
    /// ever talk to the URLs we asked for (SSRF guard from the crate docs).
    pub http: reqwest::Client,
    client_id: ClientId,
    client_secret: ClientSecret,
    redirect_url: RedirectUrl,
    cached: RwLock<(Instant, GoogleClient)>,
}

impl Google {
    /// `None` (with a log line) when GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET
    /// aren't set or Google can't be reached, so the server still starts and
    /// the Google routes answer 503.
    pub async fn from_env(public_url: &str) -> Option<Arc<Self>> {
        let (Ok(id), Ok(secret)) = (env::var("GOOGLE_CLIENT_ID"), env::var("GOOGLE_CLIENT_SECRET"))
        else {
            tracing::warn!("GOOGLE_CLIENT_ID / GOOGLE_CLIENT_SECRET not set; Google sign-in disabled");
            return None;
        };
        let redirect_url = format!("{public_url}/api/auth/google/callback");

        match Self::new(id, secret, redirect_url).await {
            Ok(google) => Some(Arc::new(google)),
            Err(err) => {
                tracing::error!("Google sign-in disabled, discovery failed: {err}");
                None
            }
        }
    }

    async fn new(id: String, secret: String, redirect_url: String) -> Result<Self, Error> {
        let http = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        let client_id = ClientId::new(id);
        let client_secret = ClientSecret::new(secret);
        let redirect_url = RedirectUrl::new(redirect_url)?;
        let client = discover(&http, &client_id, &client_secret, &redirect_url).await?;

        Ok(Self {
            http,
            client_id,
            client_secret,
            redirect_url,
            cached: RwLock::new((Instant::now(), client)),
        })
    }

    /// The configured client, re-discovered once it's older than an hour. If
    /// Google is briefly unreachable, keeps using the last good one.
    pub async fn client(&self) -> GoogleClient {
        let (fetched_at, client) = self.cached.read().unwrap().clone();
        if fetched_at.elapsed() < REFRESH_AFTER {
            return client;
        }

        match discover(&self.http, &self.client_id, &self.client_secret, &self.redirect_url).await {
            Ok(fresh) => {
                *self.cached.write().unwrap() = (Instant::now(), fresh.clone());
                fresh
            }
            Err(err) => {
                tracing::warn!("Google re-discovery failed, keeping the cached client: {err}");
                client
            }
        }
    }
}

async fn discover(
    http: &reqwest::Client,
    client_id: &ClientId,
    client_secret: &ClientSecret,
    redirect_url: &RedirectUrl,
) -> Result<GoogleClient, Error> {
    let metadata = CoreProviderMetadata::discover_async(IssuerUrl::new(ISSUER.to_string())?, http).await?;
    Ok(CoreClient::from_provider_metadata(
        metadata,
        client_id.clone(),
        Some(client_secret.clone()),
    )
    .set_redirect_uri(redirect_url.clone()))
}
