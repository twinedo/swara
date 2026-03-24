// src/routes/auth.rs
// POST /api/auth/register
// POST /api/auth/login

use axum::{Json, extract::State};
use bcrypt::{DEFAULT_COST, hash, verify};
use serde::{Deserialize, Serialize};
use tokio::task;
use utoipa::ToSchema;

use crate::{AppState, auth::create_token, db::users, error::AppError, models::UserPublic};

// ── Request / response shapes ─────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserPublic,
}

// ── POST /api/auth/register ───────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/auth/register",
    request_body = AuthRequest,
    responses(
        (status = 200, description = "Register a user", body = AuthResponse),
        (status = 400, description = "Invalid request", body = crate::error::ErrorResponse),
        (status = 409, description = "Username already taken", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Basic input validation
    if body.username.trim().is_empty() || body.username.len() > 30 {
        return Err(AppError::BadRequest(
            "Username must be between 1 and 30 characters".into(),
        ));
    }
    if body.password.len() < 8 {
        return Err(AppError::BadRequest(
            "Password must be at least 8 characters".into(),
        ));
    }

    // bcrypt is CPU-bound — run it on the blocking thread pool
    let password_hash = task::spawn_blocking({
        let pw = body.password.clone();
        move || hash(pw, DEFAULT_COST)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
    .map_err(|e| AppError::Internal(format!("bcrypt error: {}", e)))?;

    let user = users::create_user(&state.db, &body.username, &password_hash).await?;

    let token = create_token(user.id, user.is_pro, &state.config.jwt_secret)?;

    Ok(Json(AuthResponse {
        token,
        user: UserPublic::from(user),
    }))
}

// ── POST /api/auth/login ──────────────────────────────────────────────────────

#[utoipa::path(
    post,
    path = "/api/auth/login",
    request_body = AuthRequest,
    responses(
        (status = 200, description = "Login a user", body = AuthResponse),
        (status = 401, description = "Unauthorized", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "Auth"
)]
pub async fn login(
    State(state): State<AppState>,
    Json(body): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, AppError> {
    // Fetch user — return Unauthorized (not NotFound) to prevent user enumeration
    let user = users::get_user_by_username(&state.db, &body.username)
        .await?
        .ok_or(AppError::Unauthorized)?;

    // Verify password — also CPU-bound
    let stored_hash = user.password_hash.clone();
    let password_ok = task::spawn_blocking({
        let pw = body.password.clone();
        move || verify(pw, &stored_hash)
    })
    .await
    .map_err(|e| AppError::Internal(e.to_string()))?
    .map_err(|e| AppError::Internal(format!("bcrypt error: {}", e)))?;

    if !password_ok {
        return Err(AppError::Unauthorized); // same error as "user not found"
    }

    let token = create_token(user.id, user.is_pro, &state.config.jwt_secret)?;

    Ok(Json(AuthResponse {
        token,
        user: UserPublic::from(user),
    }))
}
