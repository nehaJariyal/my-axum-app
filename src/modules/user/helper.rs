use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Utc};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde_json::json;
use sqlx::PgPool;

use crate::error::{AppError, AppResult};
use crate::guards::ssrf::{self, FetchResult};
use crate::models::user::{CreateUser, LoginUser, User};
use crate::redis::USERS_LIST_KEY;
use crate::wal;

#[derive(sqlx::FromRow)]
struct UserWithPassword {
    id: i32,
    name: String,
    email: String,
    avatar_url: Option<String>,
    created_at: DateTime<Utc>,
    password: String,
}

pub async fn find_all(
    pool: &PgPool,
    redis: &ConnectionManager,
    cache_ttl_secs: u64,
) -> AppResult<Vec<User>> {
    let mut conn = redis.clone();

    if let Ok(Some(cached)) = conn.get::<_, Option<String>>(USERS_LIST_KEY).await {
        if let Ok(users) = serde_json::from_str::<Vec<User>>(&cached) {
            println!("[cache] users list hit");
            return Ok(users);
        }
        eprintln!("[cache] users list cached value invalid, refetching from db");
    }

    println!("[cache] users list miss");
    let users = fetch_all_from_db(pool).await?;

    match serde_json::to_string(&users) {
        Ok(json) => match conn.set_ex(USERS_LIST_KEY, json, cache_ttl_secs).await {
            Ok(()) => println!(
                "[cache] users list saved to redis (key={USERS_LIST_KEY}, ttl={cache_ttl_secs}s)"
            ),
            Err(err) => eprintln!("[cache] failed to save users list to redis: {err}"),
        },
        Err(err) => eprintln!("[cache] failed to serialize users for redis: {err}"),
    }

    Ok(users)
}

async fn fetch_all_from_db(pool: &PgPool) -> AppResult<Vec<User>> {
    let users = sqlx::query_as::<_, User>(
        "SELECT id, name, email, avatar_url, created_at FROM users ORDER BY id",
    )
    .fetch_all(pool)
    .await?;

    Ok(users)
}

async fn invalidate_users_cache(redis: &ConnectionManager) {
    let mut conn = redis.clone();
    match conn.del::<_, ()>(USERS_LIST_KEY).await {
        Ok(()) => println!("[cache] users list removed from redis"),
        Err(err) => eprintln!("[cache] failed to remove users list from redis: {err}"),
    }
}

pub async fn find_by_id(pool: &PgPool, id: i32) -> AppResult<User> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, name, email, avatar_url, created_at FROM users WHERE id = $1",
    )
    .bind(id)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("user {id} not found")))?;

    Ok(user)
}

async fn find_by_email(pool: &PgPool, email: &str) -> AppResult<(User, String)> {
    let row = sqlx::query_as::<_, UserWithPassword>(
        "SELECT id, name, email, avatar_url, created_at, password FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Unauthorized("invalid email or password".into()))?;

    let user = User {
        id: row.id,
        name: row.name,
        email: row.email,
        avatar_url: row.avatar_url,
        created_at: row.created_at,
    };

    Ok((user, row.password))
}

pub async fn create(
    pool: &PgPool,
    redis: &ConnectionManager,
    aeron: Option<&crate::aeron::AeronPublisher>,
    input: CreateUser,
) -> AppResult<User> {
    let avatar_url = match &input.avatar_url {
        Some(url) => {
            // SSRF guard: validate URL + safe fetch before saving.
            ssrf::validate_external_url(url)?;
            ssrf::fetch_safe_url(url).await?;
            Some(url.trim().to_string())
        }
        None => None,
    };

    let password_hash = hash_password(&input.password)?;

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (name, email, password, avatar_url)
         VALUES ($1, $2, $3, $4)
         RETURNING id, name, email, avatar_url, created_at",
    )
    .bind(input.name.trim())
    .bind(input.email.trim().to_lowercase())
    .bind(password_hash)
    .bind(avatar_url)
    .fetch_one(pool)
    .await?;

    invalidate_users_cache(redis).await;

    if let Some(publisher) = aeron {
        publisher.publish_signup(&user).await;
    }

    Ok(user)
}

pub async fn login(pool: &PgPool, input: LoginUser) -> AppResult<User> {
    let (user, stored_hash) = find_by_email(pool, &input.email.trim().to_lowercase()).await?;
    verify_password(&input.password, &stored_hash)?;
    Ok(user)
}

/// Find an existing user by email or create one from an OAuth profile.
///
/// Used by the OAuth callback (Google, etc.). If a user with the same email
/// already exists (e.g. they signed up locally), we link/return that account
/// instead of creating a duplicate.
pub async fn find_or_create_oauth_user(
    pool: &PgPool,
    redis: &ConnectionManager,
    aeron: Option<&crate::aeron::AeronPublisher>,
    provider: &str,
    provider_id: &str,
    name: &str,
    email: &str,
    avatar_url: Option<&str>,
) -> AppResult<User> {
    let email = email.trim().to_lowercase();

    // Existing account with this email → make sure provider info is linked.
    if let Some(existing) = find_by_email_optional(pool, &email).await? {
        let user = sqlx::query_as::<_, User>(
            "UPDATE users
             SET provider = $1,
                 provider_id = COALESCE(provider_id, $2),
                 avatar_url = COALESCE(avatar_url, $3)
             WHERE id = $4
             RETURNING id, name, email, avatar_url, created_at",
        )
        .bind(provider)
        .bind(provider_id)
        .bind(avatar_url)
        .bind(existing.id)
        .fetch_one(pool)
        .await?;

        invalidate_users_cache(redis).await;
        return Ok(user);
    }

    // Brand new OAuth user. Password stays empty ('') — they log in via provider.
    let display_name = if name.trim().is_empty() {
        email.split('@').next().unwrap_or("user").to_string()
    } else {
        name.trim().to_string()
    };

    let user = sqlx::query_as::<_, User>(
        "INSERT INTO users (name, email, password, avatar_url, provider, provider_id)
         VALUES ($1, $2, '', $3, $4, $5)
         RETURNING id, name, email, avatar_url, created_at",
    )
    .bind(display_name)
    .bind(&email)
    .bind(avatar_url)
    .bind(provider)
    .bind(provider_id)
    .fetch_one(pool)
    .await?;

    invalidate_users_cache(redis).await;

    if let Some(publisher) = aeron {
        publisher.publish_signup(&user).await;
    }

    Ok(user)
}

async fn find_by_email_optional(pool: &PgPool, email: &str) -> AppResult<Option<User>> {
    let user = sqlx::query_as::<_, User>(
        "SELECT id, name, email, avatar_url, created_at FROM users WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

/// Preview external URL safely — for testing SSRF protection.
pub async fn preview_url(raw_url: &str) -> AppResult<FetchResult> {
    ssrf::fetch_safe_url(raw_url).await
}

fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut rand::thread_rng());
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|_| AppError::BadRequest("failed to hash password".into()))
}

fn verify_password(password: &str, stored_hash: &str) -> AppResult<()> {
    let parsed = PasswordHash::new(stored_hash)
        .map_err(|_| AppError::Unauthorized("invalid email or password".into()))?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed)
        .map_err(|_| AppError::Unauthorized("invalid email or password".into())) 
}
