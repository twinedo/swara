// src/db/channels.rs
// Channel CRUD and PostGIS spatial queries.
//
use rust_decimal::Decimal;
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{Channel, ChannelRow, NewChannel};

// ── Nearby query ─────────────────────────────────────────────────────────────

/// Fetch channels within `radius_m` metres of the given point.
///
/// PostGIS notes:
///  - `ST_MakePoint($1, $2)` takes (longitude, latitude) — note the order.
///  - `::geography` cast activates geodetic (spheroid) distance in metres.
///  - `ST_DWithin` uses the GIST index on `channels.location` automatically.
///  - Results are ordered nearest-first.
pub async fn get_nearby_channels(
    pool: &PgPool,
    lng: f64,
    lat: f64,
    radius_m: i32,
) -> Result<Vec<ChannelRow>, AppError> {
    let rows = sqlx::query_as::<_, ChannelRow>(
        r#"
        SELECT
            c.id,
            c.frequency,
            c.name,
            c.status,
            c.radius_m,
            c.livekit_room_name,
            CAST(
                ST_Distance(
                    c.location,
                    ST_MakePoint($1, $2)::geography
                ) AS INTEGER
            ) AS distance_m,
            u.username AS owner_username
        FROM channels c
        JOIN users u ON u.id = c.owner_id
        WHERE ST_DWithin(
            c.location,
            ST_MakePoint($1, $2)::geography,
            $3
        )
        ORDER BY distance_m ASC
        "#,
    )
    .bind(lng)
    .bind(lat)
    .bind(radius_m as f64)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(rows)
}

// ── Single channel ────────────────────────────────────────────────────────────

pub async fn get_channel_by_id(pool: &PgPool, channel_id: Uuid) -> Result<Channel, AppError> {
    let row = sqlx::query_as::<_, Channel>(
        r#"
        SELECT
            c.id,
            c.owner_id,
            c.frequency,
            c.name,
            c.status,
            c.radius_m,
            c.livekit_room_name,
            c.created_at,
            u.username AS owner_username
        FROM channels c
        JOIN users u ON u.id = c.owner_id
        WHERE c.id = $1
        "#,
    )
    .bind(channel_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound(format!("Channel {} not found", channel_id)))?;

    Ok(row)
}

// ── Create ────────────────────────────────────────────────────────────────────

pub async fn create_channel(pool: &PgPool, new: &NewChannel) -> Result<Uuid, AppError> {
    let frequency = Decimal::new((new.frequency * 10.0).round() as i64, 1);

    let id = sqlx::query_scalar::<_, Uuid>(
        r#"
        INSERT INTO channels (owner_id, frequency, name, location, radius_m)
        VALUES (
            $1,
            $2,
            $3,
            ST_MakePoint($4, $5)::geography,
            $6
        )
        RETURNING id
        "#,
    )
    .bind(new.owner_id)
    .bind(frequency)
    .bind(&new.name)
    .bind(new.lng)
    .bind(new.lat)
    .bind(new.radius_m)
    .fetch_one(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db_err)
            if db_err.constraint() == Some("channels_frequency_owner_id_key") =>
        {
            AppError::Conflict(format!(
                "You already have a channel at {:.1} MHz",
                new.frequency
            ))
        }
        _ => AppError::Database(e),
    })?;

    Ok(id)
}

// ── Status updates (called from broadcast routes) ─────────────────────────────

pub async fn set_channel_live(
    pool: &PgPool,
    channel_id: Uuid,
    room_name: &str,
) -> Result<(), AppError> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE channels
        SET status = 'live', livekit_room_name = $2, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(channel_id)
    .bind(room_name)
    .execute(pool)
    .await
    .map_err(AppError::Database)?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!(
            "Channel {} not found",
            channel_id
        )));
    }
    Ok(())
}

pub async fn set_channel_offline(pool: &PgPool, channel_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE channels
        SET status = 'offline', livekit_room_name = NULL, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(channel_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(())
}

// ── Owner's channels ──────────────────────────────────────────────────────────

pub async fn get_channels_by_owner(
    pool: &PgPool,
    owner_id: Uuid,
) -> Result<Vec<Channel>, AppError> {
    let rows = sqlx::query_as::<_, Channel>(
        r#"
        SELECT
            c.id,
            c.owner_id,
            c.frequency,
            c.name,
            c.status,
            c.radius_m,
            c.livekit_room_name,
            c.created_at,
            u.username AS owner_username
        FROM channels c
        JOIN users u ON u.id = c.owner_id
        WHERE c.owner_id = $1
        ORDER BY c.created_at ASC
        "#,
    )
    .bind(owner_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(rows)
}

// ── Dead-air scanner ──────────────────────────────────────────────────────────

/// Returns IDs of all channels currently marked 'live' in Postgres.
/// The background task cross-references these against Redis heartbeat keys.
pub async fn get_live_channel_ids(pool: &PgPool) -> Result<Vec<Uuid>, AppError> {
    let ids = sqlx::query_scalar::<_, Uuid>(r#"SELECT id FROM channels WHERE status = 'live'"#)
        .fetch_all(pool)
        .await
        .map_err(AppError::Database)?;

    Ok(ids)
}
