use utoipa::{
    Modify, OpenApi,
    openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme},
};

use crate::{
    error::ErrorResponse,
    models::UserPublic,
    routes::{
        auth::{self, AuthRequest, AuthResponse},
        broadcast::{self, BroadcastRequest, HeartbeatResponse, StartResponse, StopResponse},
        channels::{
            self, CreateChannelRequest, CreateChannelResponse, MineResponse, NearbyChannel,
            NearbyParams, NearbyResponse, OwnerSummary,
        },
        listen::{self, ListenChannelSummary, ListenRequest, ListenResponse},
        schedule::{self, ScheduleEntry, ScheduleResponse},
    },
};

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "bearer_auth",
                SecurityScheme::Http(
                    HttpBuilder::new()
                        .scheme(HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        auth::register,
        auth::login,
        channels::nearby,
        channels::create,
        channels::mine,
        schedule::get_schedule,
        broadcast::start,
        broadcast::stop,
        broadcast::heartbeat,
        listen::listen
    ),
    components(
        schemas(
            ErrorResponse,
            UserPublic,
            AuthRequest,
            AuthResponse,
            NearbyParams,
            OwnerSummary,
            NearbyChannel,
            NearbyResponse,
            CreateChannelRequest,
            CreateChannelResponse,
            MineResponse,
            BroadcastRequest,
            StartResponse,
            StopResponse,
            HeartbeatResponse,
            ListenRequest,
            ListenChannelSummary,
            ListenResponse,
            ScheduleEntry,
            ScheduleResponse
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Auth", description = "Authentication endpoints"),
        (name = "Channels", description = "Channel discovery and management"),
        (name = "Broadcast", description = "Broadcast lifecycle endpoints"),
        (name = "Listen", description = "Listener connection endpoints"),
        (name = "Schedule", description = "Channel schedule endpoints")
    ),
    info(
        title = "Swara API",
        description = "Audio broadcast backend for auth, channels, broadcast control, schedules, and listener token generation."
    )
)]
pub struct ApiDoc;
