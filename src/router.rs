use axum::{routing::get, Json, Router};

use crate::config::AppState;
use crate::error::ApiResponse;
use crate::modules::{oauth, user};

/// Main application router — merges module routes here.
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/about", get(|| async { "about" }))
        .route("/api/health", get(health))
        .merge(user::routes::routes(state.clone()))
        .merge(oauth::routes::routes(state.clone()))
        .with_state(state)
}

async fn health() -> Json<ApiResponse<&'static str>> {
    Json(ApiResponse::ok("ok"))
}
