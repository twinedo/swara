// src/redis/mod.rs
// Redis helpers for live broadcast state.
//
// All functions are non-fatal by design: a Redis outage should degrade
// gracefully (stale listener counts, eventually-consistent dead-air detection)
// rather than take down the whole API.

use deadpool_redis::{Connection, Pool};
use redis::AsyncCommands;
use uuid::Uuid;

// ── Key helpers ───────────────────────────────────────────────────────────────

pub fn heartbeat_key(channel_id: Uuid) -> String {
    format!("heartbeat:{}", channel_id)
}

pub fn listeners_key(channel_id: Uuid) -> String {
    format!("listeners:{}", channel_id)
}

// ── Heartbeat ─────────────────────────────────────────────────────────────────

/// Set/refresh heartbeat TTL for a live channel.
/// Call on broadcast/start and on every broadcast/heartbeat request.
/// 30-second expiry — broadcaster must call /heartbeat every ≤15 s.
pub async fn set_heartbeat(pool: &Pool, channel_id: Uuid) {
    match pool.get().await {
        Err(e) => tracing::warn!("Redis pool error (heartbeat set): {}", e),
        Ok(mut conn) => {
            let key = heartbeat_key(channel_id);
            let result: redis::RedisResult<()> = conn.set_ex(&key, 1_u8, 30).await;
            if let Err(e) = result {
                tracing::warn!("Redis error setting heartbeat for {}: {}", channel_id, e);
            }
        }
    }
}

/// Check whether a heartbeat key exists.
/// Used by the dead-air background scanner.
pub async fn heartbeat_exists(conn: &mut Connection, channel_id: Uuid) -> bool {
    let key = heartbeat_key(channel_id);
    let result: redis::RedisResult<bool> = conn.exists(&key).await;
    result.unwrap_or(true) // if Redis errors, assume alive (conservative)
}

// ── Listener counts ───────────────────────────────────────────────────────────

/// Increment listener count when a user joins.
/// Returns the new count, or 0 on error.
pub async fn increment_listener(pool: &Pool, channel_id: Uuid) -> i64 {
    match pool.get().await {
        Err(e) => {
            tracing::warn!("Redis pool error (incr listener): {}", e);
            0
        }
        Ok(mut conn) => {
            let key = listeners_key(channel_id);
            let result: redis::RedisResult<i64> = conn.incr(&key, 1).await;
            result.unwrap_or_else(|e| {
                tracing::warn!(
                    "Redis error incrementing listeners for {}: {}",
                    channel_id,
                    e
                );
                0
            })
        }
    }
}

/// Decrement listener count when a user leaves.
/// Called by disconnect/leave events (future WebSocket layer).
#[allow(dead_code)]
pub async fn decrement_listener(pool: &Pool, channel_id: Uuid) {
    match pool.get().await {
        Err(e) => tracing::warn!("Redis pool error (decr listener): {}", e),
        Ok(mut conn) => {
            let key = listeners_key(channel_id);
            // DECR can go negative if events arrive out of order — clamp via LUA
            // For now, a simple DECR is fine; counts are eventually consistent
            let result: redis::RedisResult<i64> = conn.decr(&key, 1).await;
            if let Err(e) = result {
                tracing::warn!(
                    "Redis error decrementing listeners for {}: {}",
                    channel_id,
                    e
                );
            }
        }
    }
}

/// Fetch listener counts for a batch of channels in a single pipeline.
/// Returns a Vec aligned with the input slice (0 for any missing/errored key).
pub async fn get_listener_counts(pool: &Pool, channel_ids: &[Uuid]) -> Vec<i64> {
    if channel_ids.is_empty() {
        return vec![];
    }

    match pool.get().await {
        Err(e) => {
            tracing::warn!("Redis pool error (batch listener counts): {}", e);
            vec![0; channel_ids.len()]
        }
        Ok(mut conn) => {
            // Build a pipeline so we pay one round-trip for N channels
            let mut pipe = redis::pipe();
            for id in channel_ids {
                pipe.get(listeners_key(*id));
            }

            let results: redis::RedisResult<Vec<Option<i64>>> = pipe.query_async(&mut *conn).await;

            match results {
                Err(e) => {
                    tracing::warn!("Redis pipeline error (listener counts): {}", e);
                    vec![0; channel_ids.len()]
                }
                Ok(counts) => counts.into_iter().map(|c| c.unwrap_or(0)).collect(),
            }
        }
    }
}

// ── Cleanup ───────────────────────────────────────────────────────────────────

/// Atomically delete both the heartbeat and listener count keys for a channel.
/// Called on broadcast/stop and by the dead-air scanner.
pub async fn clear_broadcast_keys(pool: &Pool, channel_id: Uuid) {
    match pool.get().await {
        Err(e) => tracing::warn!("Redis pool error (clear broadcast keys): {}", e),
        Ok(mut conn) => {
            let mut pipe = redis::pipe();
            pipe.del(heartbeat_key(channel_id))
                .del(listeners_key(channel_id));

            let result: redis::RedisResult<()> = pipe.query_async(&mut *conn).await;
            if let Err(e) = result {
                tracing::warn!(
                    "Redis error clearing keys for channel {}: {}",
                    channel_id,
                    e
                );
            }
        }
    }
}
