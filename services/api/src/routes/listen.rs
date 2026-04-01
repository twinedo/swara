// src/routes/listen.rs
// POST /api/listen
// Generate a LiveKit subscriber token and increment the Redis listener count.

use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{AppState, db::channels, error::AppError, livekit, redis as redis_store};

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListenRequest {
    pub channel_id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListenResponse {
    pub livekit_token: String,
    pub room_name: String,
    pub channel: ListenChannelSummary,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListenChannelSummary {
    pub id: Uuid,
    pub frequency: f64,
    pub name: String,
    pub listener_count: i64,
}

#[utoipa::path(
    post,
    path = "/api/listen",
    request_body = ListenRequest,
    responses(
        (status = 200, description = "Create listener token", body = ListenResponse),
        (status = 400, description = "Channel is not live", body = crate::error::ErrorResponse),
        (status = 404, description = "Channel not found", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "Listen"
)]
pub async fn listen(
    State(state): State<AppState>,
    Json(body): Json<ListenRequest>,
) -> Result<Json<ListenResponse>, AppError> {
    // 1. Load channel — 404 if it doesn't exist
    let channel = channels::get_channel_by_id(&state.db, body.channel_id).await?;

    // 2. Only allow joining a live channel
    if channel.status != "live" {
        return Err(AppError::BadRequest(
            "This channel is not currently live".into(),
        ));
    }

    let room_name = channel
        .livekit_room_name
        .as_deref()
        .unwrap_or_else(|| panic!("Live channel {} has no room name", channel.id));

    // 3. Generate an anonymous subscriber token so listening stays public.
    let listener_identity = format!("listener-{}", Uuid::new_v4());
    let token = livekit::generate_subscriber_token(&state.config, room_name, &listener_identity)?;

    // 4. Increment listener count in Redis — non-fatal
    let listener_count = redis_store::increment_listener(&state.redis, channel.id).await;

    Ok(Json(ListenResponse {
        livekit_token: token,
        room_name: room_name.to_string(),
        channel: ListenChannelSummary {
            id: channel.id,
            frequency: channel.frequency.try_into().unwrap_or(0.0),
            name: channel.name,
            listener_count,
        },
    }))
}
