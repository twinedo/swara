// src/db/mod.rs
// Database connection pool initialisation and submodule re-exports.

pub mod channels;
pub mod schedules;
pub mod users;

use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::Duration;

/// Initialise the Postgres connection pool.
///
/// Reads `DATABASE_URL` from the environment (already validated in `config.rs`).
/// Runs `sqlx::migrate!()` automatically so the binary applies migrations on startup.
///
/// # Panics
/// Panics at startup if the pool cannot connect or migrations fail. Both are
/// unrecoverable — better to crash fast than to serve requests against a bad schema.
pub async fn init_pool(database_url: &str) -> PgPool {
    let pool = PgPoolOptions::new()
        .max_connections(20)
        .min_connections(2)
        .acquire_timeout(Duration::from_secs(5))
        .idle_timeout(Duration::from_secs(600))
        .connect(database_url)
        .await
        .expect("Failed to connect to Postgres");

    // Run compile-time embedded migrations (sqlx migrate! reads ./migrations at build time)
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");

    // Verify PostGIS is present — fail loudly rather than silently breaking /nearby
    verify_postgis(&pool).await;

    pool
}

/// Confirm PostGIS extension is installed.  Called once at startup.
async fn verify_postgis(pool: &PgPool) {
    let row: (String,) = sqlx::query_as("SELECT PostGIS_Version()")
        .fetch_one(pool)
        .await
        .expect("PostGIS extension is not installed — run migration 001 first");

    tracing::info!("PostGIS version: {}", row.0);
}
