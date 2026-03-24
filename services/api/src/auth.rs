// src/auth.rs
// JWT creation and the AuthUser Axum extractor.
//
// Route handlers that need an authenticated user declare:
//   async fn my_handler(user: AuthUser, ...) -> ...
//
// Axum calls AuthUser::from_request_parts() automatically before the handler runs.
// If the token is missing or invalid, it returns 401 before the handler is called.

use axum::{
    RequestPartsExt, async_trait,
    extract::FromRequestParts,
    http::{StatusCode, request::Parts},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

use crate::error::AppError;
use crate::models::{AuthUser, Claims};

/// How long a JWT is valid for.
const TOKEN_EXPIRY_HOURS: i64 = 24 * 7; // 7 days

// ── Token generation ──────────────────────────────────────────────────────────

pub fn create_token(user_id: Uuid, is_pro: bool, secret: &str) -> Result<String, AppError> {
    let expiry = Utc::now()
        .checked_add_signed(Duration::hours(TOKEN_EXPIRY_HOURS))
        .expect("Valid timestamp")
        .timestamp() as usize;

    let claims = Claims {
        sub: user_id,
        exp: expiry,
        is_pro,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|e| AppError::Internal(format!("Failed to create token: {}", e)))
}

// ── Axum extractor ────────────────────────────────────────────────────────────

/// Application state must implement this so the extractor can reach the JWT secret.
/// We store config in Axum's extension layer (`Extension<Arc<Config>>`).
///
/// The extractor is implemented directly against AppState rather than a trait
/// to keep things simple — see main.rs for how the state is wired up.

#[async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, axum::Json<serde_json::Value>);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // Pull Bearer token from Authorization header
        let TypedHeader(Authorization(bearer)) = parts
            .extract::<TypedHeader<Authorization<Bearer>>>()
            .await
            .map_err(|_| unauthorized())?;

        // Retrieve JWT secret from request extensions (inserted by main.rs)
        let secret = parts.extensions.get::<JwtSecret>().ok_or_else(|| {
            tracing::error!("JwtSecret extension missing from request — wired up in main.rs?");
            unauthorized()
        })?;

        // Decode and validate
        let token_data = decode::<Claims>(
            bearer.token(),
            &DecodingKey::from_secret(secret.0.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| unauthorized())?;

        Ok(AuthUser {
            id: token_data.claims.sub,
            is_pro: token_data.claims.is_pro,
        })
    }
}

fn unauthorized() -> (StatusCode, axum::Json<serde_json::Value>) {
    (
        StatusCode::UNAUTHORIZED,
        axum::Json(serde_json::json!({ "error": "Unauthorized" })),
    )
}

/// Newtype wrapper so the JWT secret can be stored in Axum's extension map.
#[derive(Clone)]
pub struct JwtSecret(pub String);
