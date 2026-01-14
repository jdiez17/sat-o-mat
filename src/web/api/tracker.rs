use axum::{extract::State, Json};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::tracker::{RadioConfig, TrackerError, TrackerMode, TrackerSample};
use crate::web::api::error::{ApiError, ApiResult, ErrorResponse};
use crate::web::auth::{require_permission, AppState, AuthenticatedUser};
use crate::web::config::Permission;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct TrackerRequest {
    #[schema(
        example = "ISS (ZARYA)\n1 25544U 98067A   26012.17690827  .00009276  00000-0  17471-3 0  9998\n2 25544  51.6333 351.7881 0007723   8.9804 351.1321 15.49250518547578"
    )]
    pub tle: String,
    pub end: Option<DateTime<Utc>>,
    pub radio: Option<RadioConfig>,
}

#[utoipa::path(
    post,
    path = "/api/tracker/run",
    request_body = TrackerRequest,
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
    Json(request): Json<TrackerRequest>,
) -> ApiResult<Json<TrackerMode>> {
    require_permission(&user, Permission::SubmitSchedule)?;
    let mut tracker = state.tracker.lock().await;
    tracker
        .run(request.tle, request.end, request.radio)
        .await
        .map_err(map_tracker_error)?;
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
    tracker.stop().await;
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
    Ok(Json(tracker.status().mode))
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
