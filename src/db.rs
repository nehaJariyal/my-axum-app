use std::env;
use std::path::PathBuf;

use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(10)
        .connect(database_url)
        .await
}

fn migrations_dir() -> PathBuf {
    env::var("MIGRATIONS_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            env::current_dir()
                .expect("failed to get current directory")
                .join("migrations")
        })
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::migrate::Migrator::new(migrations_dir())
        .await
        .map_err(|e| sqlx::Error::Migrate(Box::new(e)))?
        .run(pool)
        .await
        .map_err(Into::into)
}
