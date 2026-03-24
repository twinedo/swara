// src/routes/broadcast.rs
// POST /api/broadcast/start      — go live
// POST /api/broadcast/stop       — end broadcast
// POST /api/broadcast/heartbeat  — keepalive (call every 15 s from client)

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{
    AppState, db::channels, error::AppError, livekit, models::AuthUser, redis as redis_store,
};

// ── Shared request body ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct BroadcastRequest {
    pub channel_id: Uuid,
}

// ── POST /api/broadcast/start ─────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct StartResponse {
    pub livekit_token: String,
    pub room_name: String,
}

#[utoipa::path(
    post,
    path = "/api/broadcast/start",
    request_body = BroadcastRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Start broadcasting", body = StartResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "Channel not found", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "Broadcast"
)]
pub async fn start(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<BroadcastRequest>,
) -> Result<Json<StartResponse>, AppError> {
    // 1. Verify ownership
    let channel = channels::get_channel_by_id(&state.db, body.channel_id).await?;
    if channel.owner_id != user.id {
        return Err(AppError::Forbidden("You do not own this channel".into()));
    }

    // 2. Stable room name — reuse existing if present (crash-recovery idempotency)
    let room_name = channel
        .livekit_room_name
        .unwrap_or_else(|| format!("channel-{}", channel.id));

    // 3. Create LiveKit room and generate publisher token
    livekit::ensure_room(&state.config, &room_name).await?;
    let token = livekit::generate_publisher_token(&state.config, &room_name, &user.id.to_string())?;

    // 4. Mark channel live in Postgres (writes back room_name for idempotency)
    channels::set_channel_live(&state.db, channel.id, &room_name).await?;

    // 5. Start Redis heartbeat
    redis_store::set_heartbeat(&state.redis, channel.id).await;

    Ok(Json(StartResponse {
        livekit_token: token,
        room_name,
    }))
}

// ── POST /api/broadcast/stop ──────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct StopResponse {
    pub success: bool,
}

#[utoipa::path(
    post,
    path = "/api/broadcast/stop",
    request_body = BroadcastRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Stop broadcasting", body = StopResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "Channel not found", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "Broadcast"
)]
pub async fn stop(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<BroadcastRequest>,
) -> Result<Json<StopResponse>, AppError> {
    // 1. Verify ownership
    let channel = channels::get_channel_by_id(&state.db, body.channel_id).await?;
    if channel.owner_id != user.id {
        return Err(AppError::Forbidden("You do not own this channel".into()));
    }

    // 2. Mark offline in Postgres — this is the authoritative state change
    channels::set_channel_offline(&state.db, channel.id).await?;

    // 3. Clean up Redis — atomic pipeline, non-fatal if Redis is down
    redis_store::clear_broadcast_keys(&state.redis, channel.id).await;

    Ok(Json(StopResponse { success: true }))
}

// ── POST /api/broadcast/heartbeat ─────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct HeartbeatResponse {
    pub ok: bool,
}

#[utoipa::path(
    post,
    path = "/api/broadcast/heartbeat",
    request_body = BroadcastRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Broadcast heartbeat accepted", body = HeartbeatResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 403, description = "Forbidden", body = crate::error::ErrorResponse),
        (status = 404, description = "Channel not found", body = crate::error::ErrorResponse),
        (status = 410, description = "Channel is no longer live", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "Broadcast"
)]
pub async fn heartbeat(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<BroadcastRequest>,
) -> Result<Json<HeartbeatResponse>, AppError> {
    // Light ownership check — read from DB to confirm the channel belongs to caller
    let channel = channels::get_channel_by_id(&state.db, body.channel_id).await?;
    if channel.owner_id != user.id {
        return Err(AppError::Forbidden("You do not own this channel".into()));
    }

    // If the dead-air detector already flipped this channel offline,
    // tell the client so it can surface an "interrupted" state rather
    // than silently heartbeating into the void.
    if channel.status != "live" {
        return Err(AppError::Gone("Channel is no longer live".into()));
    }

    // Refresh Redis TTL.  Non-fatal — if Redis is unavailable the dead-air scanner
    // will mark the channel offline within 30 s anyway.
    redis_store::set_heartbeat(&state.redis, channel.id).await;

    Ok(Json(HeartbeatResponse { ok: true }))
}
