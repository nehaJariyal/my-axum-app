//! Google OAuth 2.0 helpers (authorization code flow).
//!
//! Flow:
//! 1. `authorize_url` — build the URL we redirect the user to.
//! 2. `exchange_code` — swap the `code` from the callback for an access token.
//! 3. `fetch_profile` — read the user's profile with that access token.

use serde::Deserialize;
use url::Url;

use crate::config::OAuthProvider;
use crate::error::{AppError, AppResult};

/// Profile fields we read from Google's userinfo endpoint.
#[derive(Debug, Deserialize)]
pub struct GoogleProfile {
    /// Stable, unique Google account id.
    pub sub: String,
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub picture: Option<String>,
    #[serde(default)]
    pub email_verified: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
}

/// Build the Google consent screen URL for the given `state`.
pub fn authorize_url(provider: &OAuthProvider, state: &str) -> AppResult<String> {
    let mut url = Url::parse(&provider.auth_url)
        .map_err(|_| AppError::BadRequest("invalid google auth url".into()))?;

    url.query_pairs_mut()
        .append_pair("client_id", &provider.client_id)
        .append_pair("redirect_uri", &provider.redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", &provider.scopes)
        .append_pair("state", state)
        .append_pair("access_type", "offline")
        .append_pair("prompt", "select_account");

    Ok(url.into())
}

/// Exchange an authorization `code` for a Google access token.
async fn exchange_code(provider: &OAuthProvider, code: &str) -> AppResult<String> {
    let params = [
        ("code", code),
        ("client_id", provider.client_id.as_str()),
        ("client_secret", provider.client_secret.as_str()),
        ("redirect_uri", provider.redirect_uri.as_str()),
        ("grant_type", "authorization_code"),
    ];

    let response = reqwest::Client::new()
        .post(&provider.token_url)
        .form(&params)
        .send()
        .await
        .map_err(|_| AppError::BadRequest("failed to reach google token endpoint".into()))?;

    if !response.status().is_success() {
        return Err(AppError::Unauthorized(
            "google rejected the authorization code".into(),
        ));
    }

    let token: TokenResponse = response
        .json()
        .await
        .map_err(|_| AppError::BadRequest("invalid token response from google".into()))?;

    Ok(token.access_token)
}

/// Fetch the Google profile for a given access token.
async fn fetch_profile(provider: &OAuthProvider, access_token: &str) -> AppResult<GoogleProfile> {
    let response = reqwest::Client::new()
        .get(&provider.userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|_| AppError::BadRequest("failed to reach google userinfo endpoint".into()))?;

    if !response.status().is_success() {
        return Err(AppError::Unauthorized(
            "failed to load google profile".into(),
        ));
    }

    response
        .json::<GoogleProfile>()
        .await
        .map_err(|_| AppError::BadRequest("invalid profile response from google".into()))
}

/// Full callback step: code -> access token -> verified profile.
pub async fn profile_from_code(provider: &OAuthProvider, code: &str) -> AppResult<GoogleProfile> {
    let access_token = exchange_code(provider, code).await?;
    let profile = fetch_profile(provider, &access_token).await?;

    if profile.email_verified == Some(false) {
        return Err(AppError::Unauthorized(
            "google email is not verified".into(),
        ));
    }

    Ok(profile)
}
