//! Write-Ahead Log (WAL) — durability + audit trail for mutating operations.
//!
//! Pattern:
//! 1. Append a `pending` entry to `wal_entries` (inside a transaction)
//! 2. Apply the actual DB write
//! 3. Mark entry `committed` (or `failed` on error)
//! 4. Commit transaction
//!
//! If the server crashes between steps 1 and 3, the entry stays `pending`
//! and can be inspected / replayed later.

use chrono::Utc;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

use crate::error::AppResult;
use crate::models::wal::WalEntry;

/// Append a `pending` WAL entry inside an open transaction.
async fn insert_pending(
    tx: &mut Transaction<'_, Postgres>,
    event_type: &str,
    entity_type: &str,
    payload: Value,
) -> AppResult<i64> {
    let row: (i64,) = sqlx::query_as(
        "INSERT INTO wal_entries (event_type, entity_type, payload, status)
         VALUES ($1, $2, $3, 'pending')
         RETURNING id",
    )
    .bind(event_type)
    .bind(entity_type)
    .bind(payload)
    .fetch_one(&mut **tx)
    .await?;

    Ok(row.0)
}

async fn mark_committed(
    tx: &mut Transaction<'_, Postgres>,
    wal_id: i64,
    entity_id: i32,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE wal_entries
         SET status = 'committed', entity_id = $2, committed_at = $3
         WHERE id = $1",
    )
    .bind(wal_id)
    .bind(entity_id)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;

    Ok(())
}

async fn mark_failed(
    tx: &mut Transaction<'_, Postgres>,
    wal_id: i64,
    error_msg: &str,
) -> AppResult<()> {
    sqlx::query(
        "UPDATE wal_entries
         SET status = 'failed', error_msg = $2, committed_at = $3
         WHERE id = $1",
    )
    .bind(wal_id)
    .bind(error_msg)
    .bind(Utc::now())
    .execute(&mut **tx)
    .await?;

    Ok(())
}

/// Run a write inside a WAL-protected transaction.
///
/// The closure must return `(result, entity_id)` on success.
pub async fn execute_write<T, F, Fut>(
    pool: &PgPool,
    event_type: &str,
    entity_type: &str,
    payload: Value,
    write: F,
) -> AppResult<T>
where
    F: for<'a> FnOnce(&'a mut Transaction<'_, Postgres>) -> Fut,
    Fut: std::future::Future<Output = AppResult<(T, i32)>>,
{
    let mut tx = pool.begin().await?;
    let wal_id = insert_pending(&mut tx, event_type, entity_type, payload).await?;

    match write(&mut tx).await {
        Ok((result, entity_id)) => {
            mark_committed(&mut tx, wal_id, entity_id).await?;
            tx.commit().await?;
            println!("[wal] committed {event_type} entity_id={entity_id} wal_id={wal_id}");
            Ok(result)
        }
        Err(err) => {
            let msg = format!("{err:?}");
            mark_failed(&mut tx, wal_id, &msg).await?;
            tx.commit().await?;
            eprintln!("[wal] failed {event_type} wal_id={wal_id}: {msg}");
            Err(err)
        }
    }
}

/// Log a read-only / auth event directly as `committed` (no transaction needed).
pub async fn log_event(
    pool: &PgPool,
    event_type: &str,
    entity_type: &str,
    entity_id: i32,
    payload: Value,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO wal_entries (event_type, entity_type, entity_id, payload, status, committed_at)
         VALUES ($1, $2, $3, $4, 'committed', $5)",
    )
    .bind(event_type)
    .bind(entity_type)
    .bind(entity_id)
    .bind(payload)
    .bind(Utc::now())
    .execute(pool)
    .await?;

    println!("[wal] logged {event_type} entity_id={entity_id}");
    Ok(())
}

/// List recent WAL entries (newest first).
pub async fn list_entries(pool: &PgPool, limit: i64) -> AppResult<Vec<WalEntry>> {
    let limit = limit.clamp(1, 500);

    let entries = sqlx::query_as::<_, WalEntry>(
        "SELECT id, event_type, entity_type, entity_id, payload, status,
                error_msg, created_at, committed_at
         FROM wal_entries
         ORDER BY id DESC
         LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    Ok(entries)
}

/// Count entries stuck in `pending` (useful for monitoring / replay).
pub async fn count_pending(pool: &PgPool) -> AppResult<i64> {
    let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM wal_entries WHERE status = 'pending'")
        .fetch_one(pool)
        .await?;

    Ok(row.0)
}
