// src/error.rs
// Centralised error type.  All route handlers return Result<_, AppError>.
// The IntoResponse impl converts to a JSON body with an appropriate HTTP status.

use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;
use serde_json::json;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Unauthorized")]
    Unauthorized,

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Gone: {0}")]
    Gone(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error(transparent)]
    Database(#[from] sqlx::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        // Log internal/database errors at ERROR level; client errors at DEBUG
        match &self {
            AppError::Internal(msg) => tracing::error!("Internal error: {}", msg),
            AppError::Database(e) => tracing::error!("Database error: {}", e),
            _ => tracing::debug!("Client error: {}", self),
        }

        let (status, message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::Forbidden(msg) => (StatusCode::FORBIDDEN, msg),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, msg),
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::Gone(msg) => (StatusCode::GONE, msg),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            AppError::Database(_) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "A database error occurred".to_string(), // never leak DB details to client
            ),
        };

        (status, Json(json!(ErrorResponse { error: message }))).into_response()
    }
}
