//! OAuth routes — all public (no JWT required to start the flow).

use axum::{middleware::from_fn, routing::get, Router};

use crate::config::AppState;
use crate::middleware::logger;
use crate::modules::oauth::controller;

pub fn routes(_state: AppState) -> Router<AppState> {
    Router::new()
        .route("/api/auth/google", get(controller::google_login))
        .route(
            "/api/auth/google/callback",
            get(controller::google_callback),
        )
        .layer(from_fn(logger::request_logger))
}
