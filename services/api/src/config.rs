// src/config.rs
// Centralised config — reads all required env vars at startup and panics early
// if anything is missing.  Better to crash before binding the port than to fail
// on the first request that needs a secret.

use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub redis_url: String,
    pub jwt_secret: String,
    pub livekit_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    /// Token validity window in seconds.  Defaults to 21600 (6 hours) if
    /// LIVEKIT_TOKEN_TTL_SECS is not set.  A broadcaster's session is expected
    /// to stay well within 6 hours; listeners re-call /api/listen each session
    /// so they get a fresh token anyway.
    pub livekit_token_ttl_secs: u64,
    pub port: u16,
}

impl Config {
    /// Load and validate all required environment variables.
    /// Panics with a descriptive message on the first missing variable.
    pub fn from_env() -> Self {
        Self {
            database_url: required("DATABASE_URL"),
            redis_url: required("REDIS_URL"),
            jwt_secret: required("JWT_SECRET"),
            livekit_url: required("LIVEKIT_URL"),
            livekit_api_key: required("LIVEKIT_API_KEY"),
            livekit_api_secret: required("LIVEKIT_API_SECRET"),
            livekit_token_ttl_secs: env::var("LIVEKIT_TOKEN_TTL_SECS")
                .unwrap_or_else(|_| "21600".to_string())
                .parse()
                .expect("LIVEKIT_TOKEN_TTL_SECS must be a valid u64 (seconds)"),
            port: env::var("PORT")
                .unwrap_or_else(|_| "3100".to_string())
                .parse()
                .expect("PORT must be a valid u16"),
        }
    }
}

fn required(key: &str) -> String {
    env::var(key).unwrap_or_else(|_| panic!("Required environment variable '{key}' is not set"))
}
