use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    pub id: i32,
    pub name: String,
    pub email: String,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Signup payload — validated by `ValidatedJson` middleware.
#[derive(Debug, Deserialize, Validate)]
pub struct CreateUser {
    #[validate(length(min = 2, max = 100, message = "name must be 2-100 characters"))]
    #[validate(custom(function = "validate_not_blank"))]
    pub name: String,

    #[validate(email(message = "invalid email format"))]
    #[validate(length(max = 255, message = "email is too long"))]
    pub email: String,

    #[validate(length(min = 8, message = "password must be at least 8 characters"))]
    pub password: String,

    /// Optional external URL — checked by SSRF guard before save/use.
    #[validate(url(message = "avatar_url must be a valid url"))]
    pub avatar_url: Option<String>,
}

/// Login payload — validated by `ValidatedJson` middleware.
#[derive(Debug, Deserialize, Validate)]
pub struct LoginUser {
    #[validate(email(message = "invalid email format"))]
    pub email: String,

    #[validate(length(min = 8, message = "password must be at least 8 characters"))]
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: User,
}

/// Protected SSRF test/preview payload.
#[derive(Debug, Deserialize, Validate)]
pub struct PreviewUrl {
    #[validate(url(message = "url must be valid"))]
    pub url: String,
}

fn validate_not_blank(value: &str) -> Result<(), validator::ValidationError> {
    if value.trim().is_empty() {
        return Err(validator::ValidationError::new("must not be blank"));
    }
    Ok(())
}
