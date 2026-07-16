//! JWT authentication middleware and token helpers.
//!
//! Flow:
//! 1. Client sends `Authorization: Bearer <token>` or `access_token` cookie
//! 2. `jwt_auth` middleware validates token
//! 3. Claims are stored in request extensions
//! 4. Handlers can read `AuthUser` extractor

use axum::{
    extract::{FromRequestParts, Request, State},
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::config::AppState;
use crate::error::{ApiResponse, AppError};
use crate::middleware::csrf::{get_cookie_value, USER_TOKEN_NAME};

/// Data stored inside the JWT payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: i32,
    pub email: String,
    pub exp: usize,
}

/// Extractor used in protected handlers after `jwt_auth` middleware runs.
#[derive(Debug, Clone)]
pub struct AuthUser(pub JwtClaims);

impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<JwtClaims>()
            .cloned()
            .map(AuthUser)
            .ok_or_else(|| {
                AppError::Unauthorized("missing auth context".into()).into_response()
            })
    }
}

/// Create a signed JWT for a logged-in user.
pub fn create_token(secret: &str, user_id: i32, email: &str) -> Result<String, AppError> {
    let exp = (Utc::now() + Duration::hours(24)).timestamp() as usize;
    let claims = JwtClaims {
        sub: user_id,
        email: email.to_owned(),
        exp,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|_| AppError::BadRequest("failed to create token".into()))
}

/// Express-style auth middleware: blocks request if JWT is missing/invalid.
pub async fn jwt_auth(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, Response> {
    let token = extract_auth_token(request.headers())
        .ok_or_else(|| unauthorized("missing auth token"))?;

    let claims = decode_jwt(&state.jwt_secret, &token)
        .map_err(|_| unauthorized("invalid or expired token"))?;

    // Save claims so handlers can use `AuthUser`.
    request.extensions_mut().insert(claims);
    Ok(next.run(request).await)
}

fn extract_auth_token(headers: &axum::http::HeaderMap) -> Option<String> {
    if let Some(header) = headers.get(AUTHORIZATION).and_then(|v| v.to_str().ok()) {
        if let Some(token) = header.strip_prefix("Bearer ") {
            return Some(token.to_owned());
        }
    }

    get_cookie_value(headers, USER_TOKEN_NAME)
}

fn decode_jwt(secret: &str, token: &str) -> Result<JwtClaims, jsonwebtoken::errors::Error> {
    let data = decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(data.claims)
}

fn unauthorized(message: &str) -> Response {
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(ApiResponse::<()> {
            success: false,
            data: None,
            message: Some(message.into()),
        }),
    )
        .into_response()
}
