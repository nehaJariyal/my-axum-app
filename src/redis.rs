use redis::aio::ConnectionManager;

pub const USERS_LIST_KEY: &str = "users:list";

pub async fn create_client(redis_url: &str) -> Result<ConnectionManager, redis::RedisError> {
    let client = redis::Client::open(redis_url)?;
    ConnectionManager::new(client).await
}
