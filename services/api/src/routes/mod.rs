// src/routes/mod.rs
// Assemble all routes into a single Axum Router.

pub mod auth;
pub mod broadcast;
pub mod channels;
pub mod listen;
pub mod schedule;

use axum::{
    Router,
    routing::{get, post},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::{AppState, docs::ApiDoc};

pub fn router() -> Router<AppState> {
    Router::new()
        // Auth
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        // Channels
        .route("/api/channels/nearby", get(channels::nearby))
        .route("/api/channels", post(channels::create))
        .route("/api/channels/mine", get(channels::mine))
        .route("/api/channels/:id/schedule", get(schedule::get_schedule))
        // Broadcast
        .route("/api/broadcast/start", post(broadcast::start))
        .route("/api/broadcast/stop", post(broadcast::stop))
        .route("/api/broadcast/heartbeat", post(broadcast::heartbeat))
        // Listen
        .route("/api/listen", post(listen::listen))
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
}
