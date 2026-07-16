//! Logs every incoming request: method, path, status, duration.

use std::time::Instant;

use axum::{extract::Request, middleware::Next, response::Response};

/// Express `app.use(logger)` equivalent — runs for all routes where this layer is attached.
pub async fn request_logger(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let started = Instant::now();

    let response = next.run(request).await;

    println!(
        "[logger] {} {} -> {} ({} ms)",
        method,
        uri,
        response.status(),
        started.elapsed().as_millis()
    );

    response
}
