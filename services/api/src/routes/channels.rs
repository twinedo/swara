// src/routes/channels.rs
// GET  /api/channels/nearby   — PostGIS proximity search + Redis listener counts
// POST /api/channels           — Create a new channel (auth required)
// GET  /api/channels/mine      — Channels owned by the authenticated user

use axum::{
    Json,
    extract::{Query, State},
};
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

use crate::{AppState, db::channels, error::AppError, models::AuthUser, redis as redis_store};

// ── GET /api/channels/nearby ──────────────────────────────────────────────────

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct NearbyParams {
    pub lat: f64,
    pub lng: f64,
    /// Radius in metres. Defaults to 15 km if not supplied.
    pub radius: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NearbyChannel {
    pub id: Uuid,
    pub frequency: f64,
    pub name: String,
    pub status: String,
    pub distance_m: i32,
    pub listener_count: i64,
    pub owner: OwnerSummary,
    // populated separately via schedule query if desired — placeholder for now
    // pub current_show: Option<ShowSummary>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OwnerSummary {
    pub username: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NearbyResponse {
    pub channels: Vec<NearbyChannel>,
}

#[utoipa::path(
    get,
    path = "/api/channels/nearby",
    params(NearbyParams),
    responses(
        (status = 200, description = "Nearby channels", body = NearbyResponse),
        (status = 400, description = "Invalid query", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "Channels"
)]
pub async fn nearby(
    State(state): State<AppState>,
    Query(params): Query<NearbyParams>,
) -> Result<Json<NearbyResponse>, AppError> {
    let radius_m = params.radius.unwrap_or(15_000);

    if radius_m <= 0 || radius_m > 200_000 {
        return Err(AppError::BadRequest(
            "radius must be between 1 and 200000 metres".into(),
        ));
    }

    let rows = channels::get_nearby_channels(&state.db, params.lng, params.lat, radius_m).await?;

    // Batch-fetch listener counts from Redis.
    // Non-fatal: if Redis is down, we return 0 for all counts rather than 500.
    let channel_ids: Vec<Uuid> = rows.iter().map(|r| r.id).collect();
    let listener_counts = redis_store::get_listener_counts(&state.redis, &channel_ids).await;

    let channels: Vec<NearbyChannel> = rows
        .into_iter()
        .enumerate()
        .map(|(i, row)| NearbyChannel {
            id: row.id,
            frequency: row.frequency.try_into().unwrap_or(0.0),
            name: row.name,
            status: row.status,
            distance_m: row.distance_m,
            listener_count: listener_counts.get(i).copied().unwrap_or(0),
            owner: OwnerSummary {
                username: row.owner_username,
            },
        })
        .collect();

    Ok(Json(NearbyResponse { channels }))
}

// ── POST /api/channels ────────────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateChannelRequest {
    pub frequency: f32,
    pub name: String,
    pub lat: f64,
    pub lng: f64,
    pub radius_m: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateChannelResponse {
    pub id: Uuid,
}

#[utoipa::path(
    post,
    path = "/api/channels",
    request_body = CreateChannelRequest,
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Create a channel", body = CreateChannelResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 409, description = "Conflict", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "Channels"
)]
pub async fn create(
    State(state): State<AppState>,
    user: AuthUser,
    Json(body): Json<CreateChannelRequest>,
) -> Result<Json<CreateChannelResponse>, AppError> {
    if body.name.trim().is_empty() || body.name.len() > 100 {
        return Err(AppError::BadRequest(
            "Channel name must be between 1 and 100 characters".into(),
        ));
    }
    if !(88.0..=108.0_f32).contains(&body.frequency) {
        return Err(AppError::BadRequest(
            "frequency must be a valid FM band value (88.0–108.0)".into(),
        ));
    }

    let new = crate::models::NewChannel {
        owner_id: user.id,
        frequency: body.frequency,
        name: body.name.trim().to_string(),
        lat: body.lat,
        lng: body.lng,
        radius_m: body.radius_m.unwrap_or(15_000),
    };

    let id = channels::create_channel(&state.db, &new).await?;

    Ok(Json(CreateChannelResponse { id }))
}

// ── GET /api/channels/mine ────────────────────────────────────────────────────

#[derive(Debug, Serialize, ToSchema)]
pub struct MyChannel {
    pub id: Uuid,
    pub frequency: f64,
    pub name: String,
    pub status: String,
    pub radius_m: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MineResponse {
    pub channels: Vec<MyChannel>,
}

#[utoipa::path(
    get,
    path = "/api/channels/mine",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Current user's channels", body = MineResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "Channels"
)]
pub async fn mine(
    State(state): State<AppState>,
    user: AuthUser,
) -> Result<Json<MineResponse>, AppError> {
    let rows = channels::get_channels_by_owner(&state.db, user.id).await?;

    let channels = rows
        .into_iter()
        .map(|c| MyChannel {
            id: c.id,
            frequency: c.frequency.try_into().unwrap_or(0.0),
            name: c.name,
            status: c.status,
            radius_m: c.radius_m,
        })
        .collect();

    Ok(Json(MineResponse { channels }))
}
