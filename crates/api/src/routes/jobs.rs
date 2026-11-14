//! Job status, cancellation and event streaming.

use std::convert::Infallible;

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use futures::Stream;
use futures::StreamExt;
use hands_off_core::JobId;
use hands_off_core::job::{JobEvent, JobRecord};

use crate::error::{ApiError, ErrorBody};
use crate::state::AppState;

fn event_name(event: &JobEvent) -> &'static str {
    match event {
        JobEvent::Progress { .. } => "progress",
        JobEvent::Log { .. } => "log",
        JobEvent::Completed { .. } => "completed",
        JobEvent::Failed { .. } => "failed",
        JobEvent::Cancelled => "cancelled",
    }
}

fn to_sse(event: &JobEvent) -> Event {
    Event::default()
        .event(event_name(event))
        .json_data(event)
        .unwrap_or_else(|_| Event::default().comment("event could not be serialised"))
}

/// Fetch a job's current state.
#[utoipa::path(
    get,
    path = "/api/v1/jobs/{id}",
    tag = "jobs",
    params(("id" = JobId, Path, description = "Job id")),
    responses(
        (status = 200, body = JobRecord),
        (status = 404, description = "Unknown job", body = ErrorBody),
    )
)]
pub async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<JobId>,
) -> Result<Json<JobRecord>, ApiError> {
    Ok(Json(state.jobs.status(id).await?))
}

/// Request cancellation of a job. Cancelling a finished job has no effect.
#[utoipa::path(
    delete,
    path = "/api/v1/jobs/{id}",
    tag = "jobs",
    params(("id" = JobId, Path, description = "Job id")),
    responses(
        (status = 202, description = "Cancellation requested"),
        (status = 404, description = "Unknown job", body = ErrorBody),
    )
)]
pub async fn cancel_job(
    State(state): State<AppState>,
    Path(id): Path<JobId>,
) -> Result<StatusCode, ApiError> {
    state.jobs.cancel(id).await?;
    Ok(StatusCode::ACCEPTED)
}

/// Stream a job's events as server-sent events. Each event is a [`JobEvent`] JSON object;
/// the stream ends after the terminal event.
#[utoipa::path(
    get,
    path = "/api/v1/jobs/{id}/events",
    tag = "jobs",
    params(("id" = JobId, Path, description = "Job id")),
    responses(
        (status = 200, description = "Server-sent events", content_type = "text/event-stream", body = JobEvent),
        (status = 404, description = "Unknown job", body = ErrorBody),
    )
)]
pub async fn job_events(
    State(state): State<AppState>,
    Path(id): Path<JobId>,
) -> Result<Sse<impl Stream<Item = Result<Event, Infallible>>>, ApiError> {
    let events = state.jobs.events(id).await?;
    Ok(Sse::new(events.map(|event| Ok(to_sse(&event)))).keep_alive(KeepAlive::default()))
}
