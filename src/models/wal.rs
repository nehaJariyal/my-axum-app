use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct WalEntry {
    pub id: i64,
    pub event_type: String,
    pub entity_type: String,
    pub entity_id: Option<i32>,
    pub payload: Value,
    pub status: String,
    pub error_msg: Option<String>,
    pub created_at: DateTime<Utc>,
    pub committed_at: Option<DateTime<Utc>>,
}
