use axum::{extract::State, Json};

use crate::tracker::{self, RunCommand, TrackerError, TrackerMode, TrackerSample};
use crate::web::api::error::{ApiError, ApiResult, ErrorResponse};
use crate::web::auth::{require_permission, AppState, AuthenticatedUser};
use crate::web::config::Permission;

#[utoipa::path(
    post,
    path = "/api/tracker/run",
    request_body = RunCommand,
    security(
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Tracker started", body = TrackerMode),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 409, description = "Tracker already running", body = ErrorResponse),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "tracker"
)]
pub async fn run(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Json(request): Json<RunCommand>,
) -> ApiResult<Json<TrackerMode>> {
    require_permission(&user, Permission::SubmitSchedule)?;

    let cmd = tracker::Command::Run(request);

    let mut tracker = state.tracker.lock().await;
    tracker.execute_command(&cmd).map_err(map_tracker_error)?;

    Ok(Json(tracker.status().mode))
}

#[utoipa::path(
    post,
    path = "/api/tracker/stop",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Tracker stopped", body = TrackerMode),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "tracker"
)]
pub async fn stop(
    State(state): State<AppState>,
    user: AuthenticatedUser,
) -> ApiResult<Json<TrackerMode>> {
    require_permission(&user, Permission::SubmitSchedule)?;
    let mut tracker = state.tracker.lock().await;
    tracker
        .execute_command(&tracker::Command::Stop)
        .map_err(map_tracker_error)?;
    Ok(Json(tracker.status().mode))
}

#[utoipa::path(
    get,
    path = "/api/tracker/status/mode",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Tracker mode", body = TrackerMode),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "tracker"
)]
pub async fn status_mode(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<TrackerMode>> {
    let tracker = state.tracker.lock().await;
    let mode = tracker.status().mode;
    Ok(Json(mode))
}

#[utoipa::path(
    get,
    path = "/api/tracker/status/sample",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Tracker sample", body = Option<TrackerSample>),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "tracker"
)]
pub async fn status_sample(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<Option<TrackerSample>>> {
    let tracker = state.tracker.lock().await;
    Ok(Json(tracker.status().last_sample))
}

#[utoipa::path(
    get,
    path = "/api/tracker/status/trajectory",
    security(
        ("api_key" = [])
    ),
    responses(
        (status = 200, description = "Tracker trajectory", body = Vec<TrackerSample>),
        (status = 401, description = "Unauthorized", body = ErrorResponse)
    ),
    tag = "tracker"
)]
pub async fn status_trajectory(
    State(state): State<AppState>,
    _user: AuthenticatedUser,
) -> ApiResult<Json<Vec<TrackerSample>>> {
    let tracker = state.tracker.lock().await;
    Ok(Json(tracker.status().trajectory))
}

fn map_tracker_error(err: TrackerError) -> ApiError {
    match err {
        TrackerError::AlreadyRunning => ApiError::Conflict("tracker_running"),
        TrackerError::InvalidTleFormat => ApiError::Validation("invalid_tle_format".into()),
        TrackerError::InvalidTle(e) => ApiError::Validation(e.to_string()),
        TrackerError::Elements(e) => ApiError::Validation(e.to_string()),
        TrackerError::Propagation(msg) => ApiError::Validation(msg),
    }
}
