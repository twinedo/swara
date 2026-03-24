// src/main.rs
// Entry point — wires together config, DB pool, Redis pool, and the Axum router.
use std::sync::Arc;

use axum::Extension;
use deadpool_redis::{Config as RedisConfig, Runtime};
use sqlx::PgPool;
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use auth::JwtSecret;
use config::Config;

mod auth;
mod config;
mod db;
mod dead_air;
mod docs;
mod error;
mod livekit;
mod models;
mod redis;
mod routes;

// ── App state ─────────────────────────────────────────────────────────────────

/// Shared state injected into every route handler via `State<AppState>`.
#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis: deadpool_redis::Pool,
    pub config: Arc<Config>,
}

// ── Entry point ───────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    // Structured logging
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "swara_api=debug,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load and validate config
    let config = Arc::new(Config::from_env());

    // Database pool + run migrations
    let db = db::init_pool(&config.database_url).await;
    tracing::info!("Database connected and migrations applied");

    // Redis pool
    let redis_cfg = RedisConfig::from_url(&config.redis_url);
    let redis = redis_cfg
        .create_pool(Some(Runtime::Tokio1))
        .expect("Failed to create Redis pool");
    tracing::info!("Redis pool created");

    // App state
    let state = AppState {
        db: db.clone(),
        redis: redis.clone(),
        config: config.clone(),
    };

    // JWT secret extension (used by the AuthUser extractor)
    let jwt_secret = JwtSecret(config.jwt_secret.clone());

    // Axum router
    let app = routes::router()
        .with_state(state)
        // Attach JWT secret so the AuthUser extractor can reach it
        .layer(Extension(jwt_secret))
        // Request tracing
        .layer(TraceLayer::new_for_http())
        // CORS — tighten in production to your actual frontend domain
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        );

    // Spawn dead-air detection background task
    tokio::spawn(dead_air::run(db, redis));
    tracing::info!("Dead-air detection task started (10 s interval)");

    // Bind and serve
    let addr = format!("0.0.0.0:{}", config.port);
    let listener = TcpListener::bind(&addr)
        .await
        .unwrap_or_else(|_| panic!("Failed to bind to {}", addr));

    tracing::info!("Swara API listening on {}", addr);

    axum::serve(listener, app).await.expect("Server error");
}
