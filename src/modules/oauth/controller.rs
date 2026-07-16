//! OAuth 2.0 login/signup controllers.
//!
//! Endpoints:
//! - `GET /api/auth/google`          -> redirect user to Google consent screen
//! - `GET /api/auth/google/callback` -> handle Google redirect, issue our JWT

use axum::{
    extract::{Query, State},
    response::Redirect,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use serde::Deserialize;

use crate::config::{AppState, OAuthProvider};
use crate::error::{AppError, AppResult};
use crate::middleware::csrf::{build_access_cookie, build_csrf_cookie, new_csrf_token};
use crate::middleware::jwt::create_token;
use crate::modules::oauth::google;
use crate::modules::user::helper;
use crate::wal;
use serde_json::json;

const OAUTH_STATE_COOKIE: &str = "oauth_state";

/// Query params Google sends back to the callback URL.
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
}

/// Step 1 — send the user to Google's consent screen.
pub async fn google_login(
    State(state): State<AppState>,
    jar: CookieJar,
) -> AppResult<(CookieJar, Redirect)> {
    let provider = require_google(&state)?;

    // Anti-CSRF `state` — stored in a short-lived cookie, echoed back by Google.
    let csrf_state = new_csrf_token();
    let redirect_url = google::authorize_url(provider, &csrf_state)?;

    let jar = jar.add(build_state_cookie(&csrf_state));
    Ok((jar, Redirect::to(&redirect_url)))
}

/// Step 2 — Google redirects here with `code` + `state`.
pub async fn google_callback(
    State(state): State<AppState>,
    jar: CookieJar,
    Query(query): Query<CallbackQuery>,
) -> AppResult<(CookieJar, Redirect)> {
    let provider = require_google(&state)?;

    if let Some(err) = query.error {
        return Err(AppError::Unauthorized(format!("google denied access: {err}")));
    }

    // Verify the `state` matches the cookie we set in step 1 (CSRF protection).
    let cookie_state = jar
        .get(OAUTH_STATE_COOKIE)
        .map(|c| c.value().to_string())
        .ok_or_else(|| AppError::BadRequest("missing oauth state cookie".into()))?;
    let query_state = query
        .state
        .ok_or_else(|| AppError::BadRequest("missing oauth state".into()))?;
    if cookie_state != query_state {
        return Err(AppError::Forbidden("oauth state mismatch".into()));
    }

    let code = query
        .code
        .ok_or_else(|| AppError::BadRequest("missing authorization code".into()))?;

    let profile = google::profile_from_code(provider, &code).await?;

    let user = helper::find_or_create_oauth_user(
        &state.db,
        &state.redis,
        state.aeron.as_ref(),
        "google",
        &profile.sub,
        profile.name.as_deref().unwrap_or_default(),
        &profile.email,
        profile.picture.as_deref(),
    )
    .await?;

    wal::log_event(
        &state.db,
        "user.login",
        "user",
        user.id,
        json!({ "email": user.email, "method": "google" }),
    )
    .await?;

    let token = create_token(&state.jwt_secret, user.id, &user.email)?;
    let csrf = new_csrf_token();

    let jar = jar
        .remove(Cookie::from(OAUTH_STATE_COOKIE))
        .add(build_csrf_cookie(&csrf))
        .add(build_access_cookie(&token));

    Ok((jar, Redirect::to(&state.oauth_success_redirect)))
}

fn require_google(state: &AppState) -> AppResult<&OAuthProvider> {
    state
        .google_oauth
        .as_ref()
        .ok_or_else(|| AppError::BadRequest("google oauth is not configured".into()))
}

/// Short-lived, http-only cookie holding the anti-CSRF `state` value.
/// `SameSite::Lax` so the browser still sends it on the top-level redirect back from Google.
fn build_state_cookie(state: &str) -> Cookie<'static> {
    Cookie::build((OAUTH_STATE_COOKIE, state.to_owned()))
        .http_only(true)
        .same_site(SameSite::Lax)
        .path("/")
        .build()
}
