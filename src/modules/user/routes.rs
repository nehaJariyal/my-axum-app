//! User routes split into public and protected groups.
//!
//! Public  -> signup, login (no JWT required)
//! Protected -> users list/detail (JWT + optional CSRF)

use axum::{
    middleware::{from_fn, from_fn_with_state},
    routing::{get, post},
    Router,
};

use crate::config::AppState;
use crate::middleware::{csrf, jwt, logger};
use crate::modules::user::controller;

pub fn routes(state: AppState) -> Router<AppState> {
    // --- PUBLIC ROUTES ---
    // Anyone can hit these endpoints.
    let public_routes = Router::new()
        .route("/api/signup", post(controller::create_user))
        .route("/api/login", post(controller::login));

    // --- PROTECTED ROUTES ---
    // JWT middleware validates Bearer token.
    // CSRF middleware protects cookie-based browser POST/PUT/DELETE.
    let protected_routes = Router::new()
        .route("/api/users", get(controller::list_users))
        .route("/api/users/{id}", get(controller::get_user))
        .route("/api/wal", get(controller::list_wal))
        .route("/api/url/preview", post(controller::preview_external_url))
        .route_layer(from_fn(csrf::csrf_protect))
        .route_layer(from_fn_with_state(state, jwt::jwt_auth));

    // Logger runs for all user routes in this module.
    public_routes
        .merge(protected_routes)
        .layer(from_fn(logger::request_logger))
}
