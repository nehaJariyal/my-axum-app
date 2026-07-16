mod aeron;
mod config;
mod db;
mod error;
mod redis;
mod guards;
mod middleware;
mod models;
mod modules;
mod router;
mod wal;

use config::Config;
use db::{create_pool, run_migrations};

#[tokio::main]
async fn main() {
    dotenvy::dotenv().ok();

    let config = Config::from_env();

    let db = create_pool(&config.database_url)
        .await
        .expect("failed to connect to database");

    run_migrations(&db)
        .await
        .expect("failed to run database migrations");

    let redis = redis::create_client(&config.redis_url)
        .await
        .expect("failed to connect to redis");

    let aeron = Some(aeron::AeronPublisher::new(config.aeron_publish_url.clone()));

    let state = config::AppState::new(db, redis, aeron, &config);
    let app = router::create_router(state);

    let addr = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind server");

    println!("Server running on http://{addr}");
    axum::serve(listener, app).await.unwrap();
}
