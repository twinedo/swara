// src/dead_air.rs
// Background task — detects and cleans up channels whose heartbeat has expired.
//
// Spawned once in main.rs alongside the Axum server.
// Runs every 10 seconds; cross-references Postgres (channels with status='live')
// against Redis (heartbeat:{channel_id} TTL keys).

use deadpool_redis::Pool as RedisPool;
use sqlx::PgPool;
use std::time::Duration;

use crate::{db::channels, redis as redis_store};

/// Entry point — call once from main.rs:
///
/// ```rust
/// tokio::spawn(dead_air::run(db_pool.clone(), redis_pool.clone()));
/// ```
pub async fn run(db: PgPool, redis: RedisPool) {
    let mut interval = tokio::time::interval(Duration::from_secs(10));

    loop {
        interval.tick().await;

        if let Err(e) = scan_once(&db, &redis).await {
            // Log and continue — a single scan failure should not stop the task
            tracing::error!("Dead-air scan error: {}", e);
        }
    }
}

async fn scan_once(db: &PgPool, redis: &RedisPool) -> Result<(), crate::error::AppError> {
    // 1. Get all channels currently marked live in Postgres
    let live_ids = channels::get_live_channel_ids(db).await?;

    if live_ids.is_empty() {
        return Ok(());
    }

    // 2. Check heartbeat keys in Redis
    let mut redis_conn = redis.get().await.map_err(|e| {
        crate::error::AppError::Internal(format!("Redis pool error in dead-air scan: {}", e))
    })?;

    let mut dead_count = 0usize;

    for channel_id in live_ids {
        let alive = redis_store::heartbeat_exists(&mut redis_conn, channel_id).await;

        if !alive {
            tracing::info!(
                "Dead-air detected: channel {} has no heartbeat — marking offline",
                channel_id
            );

            // 3. Mark offline in Postgres
            if let Err(e) = channels::set_channel_offline(db, channel_id).await {
                tracing::error!("Failed to mark channel {} offline: {}", channel_id, e);
                continue;
            }

            // 4. Clean up Redis keys (non-fatal — they may already be gone)
            redis_store::clear_broadcast_keys(redis, channel_id).await;

            dead_count += 1;
        }
    }

    if dead_count > 0 {
        tracing::info!("Dead-air scan: marked {} channel(s) offline", dead_count);
    }

    Ok(())
}
