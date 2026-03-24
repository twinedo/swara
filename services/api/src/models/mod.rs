// src/models/mod.rs
// Shared Rust structs used across db/, routes/, and livekit/.
//
// sqlx query_as! maps column names to field names by exact match.
// serde renames are kept snake_case on the wire to match the API contract.

use chrono::{DateTime, FixedOffset, NaiveTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::postgres::types::PgTimeTz;
use utoipa::ToSchema;
use uuid::Uuid;

// ── User ─────────────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String, // never serialised — no Serialize derive
    pub is_pro: bool,
    pub location_mode: String,
    pub created_at: DateTime<Utc>,
}

/// Safe public representation of a user (no password hash).
#[derive(Debug, Serialize, ToSchema)]
pub struct UserPublic {
    pub id: Uuid,
    pub username: String,
    pub is_pro: bool,
}

impl From<User> for UserPublic {
    fn from(u: User) -> Self {
        Self {
            id: u.id,
            username: u.username,
            is_pro: u.is_pro,
        }
    }
}

// ── Channel ───────────────────────────────────────────────────────────────────

/// Full channel row returned from DB joins.
#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct Channel {
    pub id: Uuid,
    pub owner_id: Uuid,
    pub frequency: Decimal,
    pub name: String,
    pub status: String,
    pub radius_m: i32,
    pub livekit_room_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub owner_username: String,
}

/// Lightweight row returned by the /nearby PostGIS query.
#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
pub struct ChannelRow {
    pub id: Uuid,
    pub frequency: Decimal,
    pub name: String,
    pub status: String,
    pub radius_m: i32,
    pub livekit_room_name: Option<String>,
    pub distance_m: i32,
    pub owner_username: String,
}

/// Input for creating a new channel.
#[derive(Debug, Deserialize)]
pub struct NewChannel {
    pub owner_id: Uuid,
    pub frequency: f32,
    pub name: String,
    pub lat: f64,
    pub lng: f64,
    pub radius_m: i32,
}

// ── Schedule ──────────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, sqlx::FromRow)]
pub struct ScheduleRow {
    pub id: Uuid,
    pub channel_id: Uuid,
    pub show_name: String,
    pub host_name: Option<String>,
    pub start_time: PgTimeTz<NaiveTime, FixedOffset>,
    pub day_of_week: Option<i32>,
    pub duration_minutes: Option<i32>,
}

// ── Auth claims (JWT payload) ─────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: Uuid,  // user id
    pub exp: usize, // expiry (Unix timestamp)
    pub is_pro: bool,
}

/// Extractor type — attached to request by JWT middleware.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AuthUser {
    pub id: Uuid,
    pub is_pro: bool,
}

// ── Singgah ───────────────────────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
pub struct SinggahRequest {
    pub lat: f64,
    pub lng: f64,
}

#[allow(dead_code)]
#[derive(Debug, Serialize)]
pub struct SinggahResponse {
    pub success: bool,
}
