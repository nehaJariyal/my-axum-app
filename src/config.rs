use std::env;

use redis::aio::ConnectionManager;
use sqlx::PgPool;

use crate::aeron::AeronPublisher;

/// Settings for a single OAuth 2.0 provider (authorization code flow).
#[derive(Clone)]
pub struct OAuthProvider {
    pub client_id: String,
    pub client_secret: String,
    /// Where the provider redirects back to after consent.
    pub redirect_uri: String,
    /// Provider authorization endpoint (where we send the user).
    pub auth_url: String,
    /// Provider token endpoint (where we exchange the code).
    pub token_url: String,
    /// Provider userinfo endpoint (where we read the profile).
    pub userinfo_url: String,
    /// Space-separated OAuth scopes.
    pub scopes: String,
}

/// App configuration loaded from environment variables.
#[derive(Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub host: String,
    pub port: u16,
    /// Secret used to sign and verify JWT tokens.
    pub jwt_secret: String,
    /// TTL for cached user list in seconds.
    pub users_cache_ttl_secs: u64,
    pub aeron_publish_url: String,
    /// Google OAuth config — `None` when not configured via env.
    pub google_oauth: Option<OAuthProvider>,
    /// Where to send the browser after a successful OAuth login.
    pub oauth_success_redirect: String,
}

/// Shared state passed to every route handler and middleware.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: ConnectionManager,
    pub aeron: Option<AeronPublisher>,
    pub jwt_secret: String,
    pub users_cache_ttl_secs: u64,
    pub google_oauth: Option<OAuthProvider>,
    pub oauth_success_redirect: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
            redis_url: env::var("REDIS_URL").expect("REDIS_URL must be set"),
            host: env::var("HOST").unwrap_or_else(|_| "128.0.0.1".into()),
            port: env::var("PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(3000),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "change-this-secret-in-production".into()),
            users_cache_ttl_secs: env::var("USERS_CACHE_TTL_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(300),
            aeron_publish_url: env::var("AERON_PUBLISH_URL")
                .unwrap_or_else(|_| "http://localhost:8070/api/publish".into()),
            google_oauth: Self::google_oauth_from_env(),
            oauth_success_redirect: env::var("OAUTH_SUCCESS_REDIRECT")
                .unwrap_or_else(|_| "/".into()),
        }
    }

    /// Build Google OAuth config only when client id + secret are present.
    fn google_oauth_from_env() -> Option<OAuthProvider> {
        let client_id = env::var("GOOGLE_CLIENT_ID").ok().filter(|v| !v.is_empty())?;
        let client_secret = env::var("GOOGLE_CLIENT_SECRET")
            .ok()
            .filter(|v| !v.is_empty())?;

        Some(OAuthProvider {
            client_id,
            client_secret,
            redirect_uri: env::var("GOOGLE_REDIRECT_URI").unwrap_or_else(|_| {
                "http://localhost:3001/api/auth/google/callback".into()
            }),
            auth_url: "https://accounts.google.com/o/oauth2/v2/auth".into(),
            token_url: "https://oauth2.googleapis.com/token".into(),
            userinfo_url: "https://openidconnect.googleapis.com/v1/userinfo".into(),
            scopes: "openid email profile".into(),
        })
    }
}

impl AppState {
    pub fn new(db: PgPool, redis: ConnectionManager, aeron: Option<AeronPublisher>, config: &Config) -> Self {
        Self {
            db,
            redis,
            aeron,
            jwt_secret: config.jwt_secret.clone(),
            users_cache_ttl_secs: config.users_cache_ttl_secs,
            google_oauth: config.google_oauth.clone(),
            oauth_success_redirect: config.oauth_success_redirect.clone(),
        }
    }
}
