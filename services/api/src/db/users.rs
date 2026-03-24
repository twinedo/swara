// src/db/users.rs
// User lookups, creation, and Pro Singgah location updates.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::User;

// ── Lookups ───────────────────────────────────────────────────────────────────

#[allow(dead_code)]
pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> Result<User, AppError> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, password_hash, is_pro, location_mode, created_at
        FROM users
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)?
    .ok_or_else(|| AppError::NotFound(format!("User {} not found", user_id)))
}

pub async fn get_user_by_username(pool: &PgPool, username: &str) -> Result<Option<User>, AppError> {
    sqlx::query_as::<_, User>(
        r#"
        SELECT id, username, password_hash, is_pro, location_mode, created_at
        FROM users
        WHERE username = $1
        "#,
    )
    .bind(username)
    .fetch_optional(pool)
    .await
    .map_err(AppError::Database)
}

// ── Create ────────────────────────────────────────────────────────────────────

pub async fn create_user(
    pool: &PgPool,
    username: &str,
    password_hash: &str,
) -> Result<User, AppError> {
    sqlx::query_as::<_, User>(
        r#"
        INSERT INTO users (username, password_hash)
        VALUES ($1, $2)
        RETURNING id, username, password_hash, is_pro, location_mode, created_at
        "#,
    )
    .bind(username)
    .bind(password_hash)
    .fetch_one(pool)
    .await
    .map_err(|e| match &e {
        sqlx::Error::Database(db_err) if db_err.constraint() == Some("users_username_key") => {
            AppError::Conflict(format!("Username '{}' is already taken", username))
        }
        _ => AppError::Database(e),
    })
}

// ── Singgah location (Pro only) ───────────────────────────────────────────────

/// Persist a user's manual Singgah location override.
///
/// The route handler must verify `user.is_pro` before calling this — this
/// function does not re-check; it trusts the caller has already gated on Pro.
#[allow(dead_code)]
pub async fn update_singgah_location(
    pool: &PgPool,
    user_id: Uuid,
    lng: f64,
    lat: f64,
) -> Result<(), AppError> {
    let rows_affected = sqlx::query(
        r#"
        UPDATE users
        SET
            selected_location = ST_MakePoint($2, $3)::geography,
            location_mode     = 'manual',
            updated_at        = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .bind(lng)
    .bind(lat)
    .execute(pool)
    .await
    .map_err(AppError::Database)?
    .rows_affected();

    if rows_affected == 0 {
        return Err(AppError::NotFound(format!("User {} not found", user_id)));
    }
    Ok(())
}

/// Clear a Pro user's Singgah override, reverting to GPS mode.
#[allow(dead_code)]
pub async fn clear_singgah_location(pool: &PgPool, user_id: Uuid) -> Result<(), AppError> {
    sqlx::query(
        r#"
        UPDATE users
        SET
            selected_location = NULL,
            location_mode     = 'gps',
            updated_at        = NOW()
        WHERE id = $1
        "#,
    )
    .bind(user_id)
    .execute(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(())
}
