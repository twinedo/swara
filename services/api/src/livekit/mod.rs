// src/livekit/mod.rs
//
// Two responsibilities:
//   1. Token generation  — pure local HMAC signing, no network call.
//   2. Room management   — ensure_room() calls the LiveKit server API to
//                          pre-create a room before we commit "live" state to
//                          Postgres.  If LiveKit is unreachable we fail fast
//                          here, before touching Redis or marking the channel
//                          live — keeping all three stores consistent.

use std::time::Duration;

use livekit_api::{
    access_token::{AccessToken, VideoGrants},
    services::room::{CreateRoomOptions, RoomClient},
};

use crate::{config::Config, error::AppError};

// ── Token TTL ─────────────────────────────────────────────────────────────────

/// Read token TTL from config.  Falls back to 6 hours if the env var is absent
/// (config.rs already panics on missing *required* vars; TTL is optional).
fn token_ttl_secs(config: &Config) -> u64 {
    config.livekit_token_ttl_secs
}

// ── Token generation ──────────────────────────────────────────────────────────

/// Generate a **publisher** token (broadcaster).
/// Can publish audio; cannot subscribe to others.
pub fn generate_publisher_token(
    config: &Config,
    room_name: &str,
    participant_identity: &str,
) -> Result<String, AppError> {
    let token = AccessToken::with_api_key(&config.livekit_api_key, &config.livekit_api_secret)
        .with_identity(participant_identity)
        .with_name(participant_identity)
        .with_ttl(Duration::from_secs(token_ttl_secs(config)))
        .with_grants(VideoGrants {
            room: room_name.to_string(),
            room_join: true,
            can_publish: true,
            can_subscribe: false,
            can_publish_data: false, // radio — no data channel needed
            ..Default::default()
        })
        .to_jwt()
        .map_err(|e| AppError::Internal(format!("LiveKit token signing failed: {e}")))?;

    Ok(token)
}

/// Generate a **subscriber** token (listener).
/// Can subscribe to audio; cannot publish anything.
pub fn generate_subscriber_token(
    config: &Config,
    room_name: &str,
    participant_identity: &str,
) -> Result<String, AppError> {
    let token = AccessToken::with_api_key(&config.livekit_api_key, &config.livekit_api_secret)
        .with_identity(participant_identity)
        .with_name(participant_identity)
        .with_ttl(Duration::from_secs(token_ttl_secs(config)))
        .with_grants(VideoGrants {
            room: room_name.to_string(),
            room_join: true,
            can_publish: false,
            can_subscribe: true,
            can_publish_data: false,
            ..Default::default()
        })
        .to_jwt()
        .map_err(|e| AppError::Internal(format!("LiveKit token signing failed: {e}")))?;

    Ok(token)
}

// ── Room management ───────────────────────────────────────────────────────────

/// Create a LiveKit room if it doesn't already exist.
///
/// Idempotent — the LiveKit server returns the existing room if called with a
/// name that's already active, so this is safe to call on crash-recovery
/// restarts where the room may already be open.
///
/// Called by `broadcast/start` *before* Postgres and Redis are updated, so a
/// LiveKit connectivity failure causes the entire start operation to abort
/// cleanly with no partial state.
///
/// Room options:
/// - `empty_timeout`: 30 s — LiveKit closes the room automatically if the
///   publisher drops without calling `/broadcast/stop`.  The dead-air scanner
///   handles the Postgres/Redis side; this keeps LiveKit tidy.
/// - `max_participants`: 0 — no limit enforced at LiveKit level (app logic
///   handles any future per-channel caps).
pub async fn ensure_room(config: &Config, room_name: &str) -> Result<(), AppError> {
    let client = RoomClient::new(&config.livekit_url)
        .map_err(|e| AppError::Internal(format!("LiveKit client init failed: {e}")))?;

    let options = CreateRoomOptions {
        // Auto-close the LiveKit room 30 s after it goes empty.
        // Complements the dead-air scanner without replacing it.
        empty_timeout: 30,
        // No hard participant cap at LiveKit level.
        max_participants: 0,
        ..Default::default()
    };

    client
        .create_room(room_name, options)
        .await
        .map_err(|e| AppError::Internal(format!("LiveKit room creation failed: {e}")))?;

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config() -> Config {
        Config {
            database_url: "postgres://ignored".into(),
            redis_url: "redis://ignored".into(),
            jwt_secret: "test-jwt-secret".into(),
            livekit_url: "http://localhost:7880".into(),
            livekit_api_key: "devkey".into(),
            livekit_api_secret: "devsecret-that-is-long-enough-for-hmac".into(),
            livekit_token_ttl_secs: 3600,
            port: 3100,
        }
    }

    #[test]
    fn publisher_token_is_valid_jwt() {
        let config = test_config();
        let token = generate_publisher_token(&config, "channel-test-uuid", "user-abc")
            .expect("publisher token should succeed");

        // A LiveKit JWT is three base64url segments separated by dots
        assert_eq!(token.split('.').count(), 3, "expected a three-part JWT");
    }

    #[test]
    fn subscriber_token_is_valid_jwt() {
        let config = test_config();
        let token = generate_subscriber_token(&config, "channel-test-uuid", "user-abc")
            .expect("subscriber token should succeed");

        assert_eq!(token.split('.').count(), 3, "expected a three-part JWT");
    }

    #[test]
    fn publisher_and_subscriber_tokens_differ() {
        let config = test_config();
        let pub_token = generate_publisher_token(&config, "channel-test-uuid", "user-abc").unwrap();
        let sub_token =
            generate_subscriber_token(&config, "channel-test-uuid", "user-abc").unwrap();

        // Same identity + room but different grants — payloads must differ
        assert_ne!(pub_token, sub_token);
    }
}
