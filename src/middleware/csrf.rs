//! CSRF protection for cookie-based browser clients.
//!
//! When to use:
//! - Browser sends cookies (session/csrf cookie)
//! - State-changing requests: POST, PUT, PATCH, DELETE
//!
//! When skipped:
//! - GET / HEAD / OPTIONS
//! - No CSRF cookie present (pure Bearer-token API clients like Postman/mobile)

use axum::{
    extract::Request,
    http::{header, Method},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::error::AppError;

pub const CSRF_COOKIE_NAME: &str = "csrf_token";
pub const CSRF_HEADER_NAME: &str = "x-csrf-token";
pub const USER_TOKEN_NAME: &str = "access_token";
/// Build a secure CSRF cookie to return from login/signup.
pub fn build_csrf_cookie(token: &str) -> axum_extra::extract::cookie::Cookie<'static> {
    axum_extra::extract::cookie::Cookie::build((CSRF_COOKIE_NAME, token.to_owned()))
        .http_only(true)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .build()
}
 
pub fn build_access_cookie(token: &str) -> axum_extra::extract::cookie::Cookie<'static> {
    axum_extra::extract::cookie::Cookie::build((USER_TOKEN_NAME, token.to_owned()))
        .http_only(true)
        .secure(false)
        .same_site(axum_extra::extract::cookie::SameSite::Lax)
        .path("/")
        .build()
}
/// Express-style CSRF middleware for protected mutating routes.
pub async fn csrf_protect(request: Request, next: Next) -> Result<Response, Response> {
    // Safe/read-only methods do not need CSRF checks.
    if matches!(
        request.method(),
        &Method::GET | &Method::HEAD | &Method::OPTIONS
    ) {
        return Ok(next.run(request).await);
    }

    let (parts, body) = request.into_parts();
    let cookie_token = get_cookie_value(&parts.headers, CSRF_COOKIE_NAME);

    // API clients using only Bearer JWT can skip CSRF.
    let Some(cookie_token) = cookie_token else {
        let request = Request::from_parts(parts, body);
        return Ok(next.run(request).await);
    };

    let header_token = parts
        .headers
        .get(CSRF_HEADER_NAME)
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();

    if cookie_token != header_token {
        return Err(AppError::Forbidden("invalid csrf token".into()).into_response());
    }

    let request = Request::from_parts(parts, body);
    Ok(next.run(request).await)
}

pub fn new_csrf_token() -> String {
    uuid::Uuid::new_v4().to_string()
}

pub fn get_cookie_value(
    headers: &axum::http::HeaderMap,
    name: &str,
) -> Option<String> {
    let cookie_header = headers.get(header::COOKIE)?.to_str().ok()?;

    for part in cookie_header.split(';') {
        let mut split = part.trim().splitn(2, '=');
        let key = split.next()?;
        if key == name {
            
            let value= split.next().map(str::to_owned);
            println!("{:?}",value);
            return value;
        }
    }

    None
}
