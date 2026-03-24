// src/routes/schedule.rs
// GET /api/channels/:id/schedule

use axum::{
    Json,
    extract::{Path, State},
};
use chrono::Timelike;
use serde::Serialize;
use utoipa::ToSchema;
use uuid::Uuid;

use crate::{AppState, db::schedules, error::AppError};

#[derive(Debug, Serialize, ToSchema)]
pub struct ScheduleEntry {
    pub show_name: String,
    pub host_name: Option<String>,
    pub start_time: String, // formatted as "HH:MM"
    pub day_of_week: Option<i32>,
    pub duration_minutes: Option<i32>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScheduleResponse {
    pub schedule: Vec<ScheduleEntry>,
}

#[utoipa::path(
    get,
    path = "/api/channels/{id}/schedule",
    params(
        ("id" = Uuid, Path, description = "Channel id")
    ),
    responses(
        (status = 200, description = "Channel schedule", body = ScheduleResponse),
        (status = 404, description = "Channel not found", body = crate::error::ErrorResponse),
        (status = 500, description = "Internal server error", body = crate::error::ErrorResponse)
    ),
    tag = "Schedule"
)]
pub async fn get_schedule(
    State(state): State<AppState>,
    Path(channel_id): Path<Uuid>,
) -> Result<Json<ScheduleResponse>, AppError> {
    let rows = schedules::get_schedule_for_channel(&state.db, channel_id).await?;

    let schedule = rows
        .into_iter()
        .map(|r| ScheduleEntry {
            show_name: r.show_name,
            host_name: r.host_name,
            // Format TIMETZ as "HH:MM" for the API contract
            start_time: format!(
                "{:02}:{:02}",
                r.start_time.time.hour(),
                r.start_time.time.minute()
            ),
            day_of_week: r.day_of_week,
            duration_minutes: r.duration_minutes,
        })
        .collect();

    Ok(Json(ScheduleResponse { schedule }))
}
