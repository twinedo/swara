use axum::{
    extract::FromRequestParts,
    http::{request::Parts, HeaderMap},
    RequestPartsExt,
};
use jsonwebtoken::{decode, DecodingKey, Validation};

use crate::{error::AppError, models::{AuthUser, Claims}};

/// Axum extractor — validates Bearer JWT and injects AuthUser into handlers.
/// Usage: `async fn handler(user: AuthUser, ...) -> ...`
#[axum::async_trait]
impl<S> FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let headers: &HeaderMap = &parts.headers;

        let auth_header = headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(AppError::Unauthorized)?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;

        let secret = std::env::var("JWT_SECRET")
            .map_err(|_| AppError::Internal("JWT_SECRET not set".into()))?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized)?;

        let claims = token_data.claims;
        let id = claims.sub.parse().map_err(|_| AppError::Unauthorized)?;

        Ok(AuthUser {
            id,
            username: claims.username,
            is_pro: claims.is_pro,
        })
    }
}