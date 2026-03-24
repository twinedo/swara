// src/db/schedules.rs
// Schedule queries for the GET /api/channels/:id/schedule endpoint.

use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;
use crate::models::ScheduleRow;

/// Fetch all schedule entries for a channel, ordered by day then start time.
/// day_of_week NULL (daily) entries sort before specific days.
pub async fn get_schedule_for_channel(
    pool: &PgPool,
    channel_id: Uuid,
) -> Result<Vec<ScheduleRow>, AppError> {
    let rows = sqlx::query_as::<_, ScheduleRow>(
        r#"
        SELECT
            id,
            channel_id,
            show_name,
            host_name,
            start_time,
            day_of_week,
            duration_minutes
        FROM schedules
        WHERE channel_id = $1
        ORDER BY
            day_of_week NULLS FIRST,
            start_time ASC
        "#,
    )
    .bind(channel_id)
    .fetch_all(pool)
    .await
    .map_err(AppError::Database)?;

    Ok(rows)
}
