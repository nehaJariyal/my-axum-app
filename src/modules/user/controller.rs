use axum::{extract::State, Json};
use axum_extra::extract::cookie::CookieJar;

use crate::config::AppState;
use crate::error::{ApiResponse, AppResult};
use crate::middleware::csrf::{build_csrf_cookie, new_csrf_token,build_access_cookie};
use crate::middleware::jwt::{create_token, AuthUser};
use crate::middleware::validate::ValidatedJson;
use crate::models::user::{AuthResponse, CreateUser, LoginUser, PreviewUrl, User};
use crate::models::wal::WalEntry;
use crate::modules::user::helper;
use crate::wal;

/// Protected route — needs valid JWT from `jwt_auth` middleware.
pub async fn list_users(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<Json<ApiResponse<Vec<User>>>> {
    println!("[controller] list_users by user_id={}", claims.sub);
    let users = helper::find_all(&state.db, &state.redis, state.users_cache_ttl_secs).await?;
    Ok(Json(ApiResponse::ok(users)))
}

/// Protected route — needs valid JWT.
pub async fn get_user(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
    axum::extract::Path(id): axum::extract::Path<i32>,
) -> AppResult<Json<ApiResponse<User>>> {
    println!("[controller] get_user id={id} requested_by={}", claims.sub);
    let user = helper::find_by_id(&state.db, id).await?;
    Ok(Json(ApiResponse::ok(user)))
}

/// Public route — payload validated by `ValidatedJson`.
pub async fn create_user(
    State(state): State<AppState>,
    jar: CookieJar,
    ValidatedJson(payload): ValidatedJson<CreateUser>,
) -> AppResult<(CookieJar, Json<ApiResponse<AuthResponse>>)> {
    let user = helper::create(&state.db, &state.redis, state.aeron.as_ref(), payload).await?;
    let token = create_token(&state.jwt_secret, user.id, &user.email)?;
    let csrf = new_csrf_token();
    let jar = jar.add(build_csrf_cookie(&csrf));
     

    Ok((
        jar,
        Json(ApiResponse::ok(AuthResponse { token, user })),
    ))
}

/// Protected route — safely preview/fetch external URL (SSRF protected).
pub async fn preview_external_url(
    AuthUser(claims): AuthUser,
    ValidatedJson(payload): ValidatedJson<PreviewUrl>,
) -> AppResult<Json<ApiResponse<crate::guards::ssrf::FetchResult>>> {
    println!("[controller] preview_url by user_id={}", claims.sub);
    let result = helper::preview_url(&payload.url).await?;
    Ok(Json(ApiResponse::ok(result)))
}

/// Public route — returns JWT + sets CSRF cookie for browser clients.
pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    ValidatedJson(payload): ValidatedJson<LoginUser>,
) -> AppResult<(CookieJar, Json<ApiResponse<AuthResponse>>)> {
    let user = helper::login(&state.db, payload).await?;
    let token = create_token(&state.jwt_secret, user.id, &user.email)?;
    let csrf = new_csrf_token();
    let jar = jar.add(build_csrf_cookie(&csrf))
    .add(build_access_cookie(&token));

    Ok((
        jar,
        Json(ApiResponse::ok(AuthResponse { token, user })),
    ))
}

/// Protected route — list recent WAL (Write-Ahead Log) entries.
pub async fn list_wal(
    State(state): State<AppState>,
    AuthUser(claims): AuthUser,
) -> AppResult<Json<ApiResponse<Vec<WalEntry>>>> {
    println!("[controller] list_wal by user_id={}", claims.sub);
    let entries = wal::list_entries(&state.db, 50).await?;
    Ok(Json(ApiResponse::ok(entries)))
}
