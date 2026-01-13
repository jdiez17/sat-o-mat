use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

use crate::scheduler::approval::{evaluate_approval, ApprovalResult};
use crate::scheduler::storage::{ScheduleEntry, ScheduleState, StorageError};
use crate::scheduler::Schedule;

use crate::web::auth::{require_permission, AppState, AuthenticatedUser, PermissionError};
use crate::web::config::Permission;

// Unified API error type
pub enum ApiError {
    Permission(PermissionError),
    Validation(String),
    NotFound,
    Conflict(&'static str),
    Storage(StorageError),
}

impl From<PermissionError> for ApiError {
    fn from(e: PermissionError) -> Self {
        ApiError::Permission(e)
    }
}

impl From<StorageError> for ApiError {
    fn from(e: StorageError) -> Self {
        match e {
            StorageError::NotFound(_) => ApiError::NotFound,
            _ => ApiError::Storage(e),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::Permission(e) => e.into_response(),
            ApiError::Validation(msg) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::with_message("validation_failed", &msg)),
            )
                .into_response(),
            ApiError::NotFound => (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse::new("schedule_not_found")),
            )
                .into_response(),
            ApiError::Conflict(reason) => {
                (StatusCode::CONFLICT, Json(ErrorResponse::new(reason))).into_response()
            }
            ApiError::Storage(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse::with_message("storage_error", &e.to_string())),
            )
                .into_response(),
        }
    }
}

type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl ErrorResponse {
    pub fn new(error: &str) -> Self {
        ErrorResponse {
            error: error.to_string(),
            message: None,
        }
    }

    pub fn with_message(error: &str, message: &str) -> Self {
        ErrorResponse {
            error: error.to_string(),
            message: Some(message.to_string()),
        }
    }
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct ScheduleResponse {
    pub id: String,
    pub status: String,
    pub start: String,
    pub end: String,
}

impl From<ScheduleEntry> for ScheduleResponse {
    fn from(entry: ScheduleEntry) -> Self {
        let status = match entry.state {
            ScheduleState::Active => "approved",
            ScheduleState::AwaitingApproval => "pending",
        };
        ScheduleResponse {
            id: entry.id,
            status: status.to_string(),
            start: entry.start.to_rfc3339(),
            end: entry.end.to_rfc3339(),
        }
    }
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SubmitScheduleResponse {
    #[serde(flatten)]
    pub schedule: ScheduleResponse,
    pub approval_status: String,
}

#[utoipa::path(
    post,
    path = "/api/schedules",
    tag = "schedules",
    request_body(content = String, content_type = "application/yaml"),
    responses(
        (status = 201, description = "Schedule submitted successfully", body = SubmitScheduleResponse),
        (status = 400, description = "Validation error", body = ErrorResponse),
        (status = 401, description = "Missing or invalid API key"),
        (status = 403, description = "Insufficient permissions"),
        (status = 409, description = "Schedule overlaps with existing", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn submit_schedule(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    body: String,
) -> ApiResult<impl IntoResponse> {
    require_permission(&user, Permission::SubmitSchedule)?;

    let schedule = Schedule::from_str(&body).map_err(|e| ApiError::Validation(e.to_string()))?;

    let storage = &state.storage;

    if storage.check_overlap(schedule.start, schedule.end, None)? {
        return Err(ApiError::Conflict("schedule_overlap"));
    }

    let approval_result = evaluate_approval(state.config.approval.mode);
    let target_state = if approval_result.is_approved() {
        ScheduleState::Active
    } else {
        ScheduleState::AwaitingApproval
    };

    let id = storage.generate_id(schedule.start);
    storage.save_schedule(target_state, &id, &body)?;

    let entry = ScheduleEntry {
        id,
        state: target_state,
        start: schedule.start,
        end: schedule.end,
    };

    let approval_status = match approval_result {
        ApprovalResult::Approved => "approved",
        ApprovalResult::Pending => "pending",
    };

    Ok((
        StatusCode::CREATED,
        Json(SubmitScheduleResponse {
            schedule: entry.into(),
            approval_status: approval_status.to_string(),
        }),
    ))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScheduleValidationResponse {
    pub valid: bool,
    pub errors: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end: Option<String>,
    pub variables: Vec<ScheduleVariable>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScheduleVariable {
    pub name: String,
    pub value: String,
}

#[utoipa::path(
    post,
    path = "/api/schedules/validate",
    tag = "schedules",
    request_body(content = String, content_type = "application/yaml"),
    responses(
        (status = 200, description = "Validation result", body = ScheduleValidationResponse),
        (status = 401, description = "Missing or invalid API key"),
        (status = 403, description = "Insufficient permissions")
    ),
    security(("api_key" = []))
)]
pub async fn validate_schedule(
    State(_state): State<AppState>,
    user: AuthenticatedUser,
    body: String,
) -> ApiResult<impl IntoResponse> {
    require_permission(&user, Permission::SubmitSchedule)?;

    match Schedule::from_str(&body) {
        Ok(schedule) => Ok(Json(ScheduleValidationResponse {
            valid: true,
            errors: Vec::new(),
            start: Some(schedule.start.to_rfc3339()),
            end: Some(schedule.end.to_rfc3339()),
            variables: schedule
                .variables
                .into_iter()
                .filter(|(name, _)| name != "start" && name != "end")
                .filter_map(|(name, value)| {
                    schedule_value_to_string(&value)
                        .map(|val| ScheduleVariable { name, value: val })
                })
                .collect(),
        })),
        Err(err) => Ok(Json(ScheduleValidationResponse {
            valid: false,
            errors: vec![err.to_string()],
            start: None,
            end: None,
            variables: Vec::new(),
        })),
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListSchedulesQuery {
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default, deserialize_with = "deserialize_option_datetime")]
    pub start: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "deserialize_option_datetime")]
    pub end: Option<DateTime<Utc>>,
}

#[utoipa::path(
    get,
    path = "/api/schedules",
    tag = "schedules",
    params(
        ("state" = Option<String>, Query, description = "Filter by state (active, awaiting_approval)"),
        ("start" = Option<String>, Query, description = "Only include schedules overlapping this start time (RFC3339)"),
        ("end" = Option<String>, Query, description = "Only include schedules overlapping this end time (RFC3339)")
    ),
    responses(
        (status = 200, description = "List of schedules", body = Vec<ScheduleResponse>),
        (status = 401, description = "Missing or invalid API key"),
        (status = 403, description = "Insufficient permissions")
    ),
    security(("api_key" = []))
)]
pub async fn list_schedules(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<ListSchedulesQuery>,
) -> ApiResult<impl IntoResponse> {
    require_permission(&user, Permission::ListSchedules)?;

    let storage = &state.storage;

    let states_to_query: Vec<ScheduleState> = match query.state.as_deref() {
        Some("active") => vec![ScheduleState::Active],
        Some("awaiting_approval") => vec![ScheduleState::AwaitingApproval],
        _ => vec![ScheduleState::Active, ScheduleState::AwaitingApproval],
    };

    let start_filter = query.start;
    let end_filter = query.end;

    let mut filtered: Vec<ScheduleResponse> = Vec::new();
    for state_entry in states_to_query {
        let schedules = storage.get_schedules(state_entry)?;
        for entry in schedules {
            if let Some(ref start) = start_filter {
                if entry.end <= *start {
                    continue;
                }
            }
            if let Some(ref end) = end_filter {
                if entry.start >= *end {
                    continue;
                }
            }
            filtered.push(entry.into());
        }
    }

    Ok((StatusCode::OK, Json(filtered)))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScheduleDetailResponse {
    pub schedule: ScheduleResponse,
    pub content: String,
    pub variables: Vec<ScheduleVariable>,
}

#[utoipa::path(
    get,
    path = "/api/schedules/{id}",
    tag = "schedules",
    params(
        ("id" = String, Path, description = "Schedule ID")
    ),
    responses(
        (status = 200, description = "Schedule details", body = ScheduleDetailResponse),
        (status = 401, description = "Missing or invalid API key"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Schedule not found", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn get_schedule(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    require_permission(&user, Permission::ListSchedules)?;

    let storage = &state.storage;

    for s in [ScheduleState::Active, ScheduleState::AwaitingApproval] {
        match storage.get_schedule(s, &id) {
            Ok((entry, content)) => {
                let variables = Schedule::from_str(&content)
                    .map(|schedule| {
                        schedule
                            .variables
                            .into_iter()
                            .filter(|(name, _)| name != "start" && name != "end")
                            .filter_map(|(name, value)| {
                                schedule_value_to_string(&value)
                                    .map(|val| ScheduleVariable { name, value: val })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return Ok((
                    StatusCode::OK,
                    Json(ScheduleDetailResponse {
                        schedule: entry.into(),
                        content,
                        variables,
                    }),
                ));
            }
            Err(StorageError::NotFound(_)) => continue,
            Err(e) => return Err(e.into()),
        }
    }

    Err(ApiError::NotFound)
}

#[utoipa::path(
    delete,
    path = "/api/schedules/{id}",
    tag = "schedules",
    params(
        ("id" = String, Path, description = "Schedule ID")
    ),
    responses(
        (status = 204, description = "Schedule deleted"),
        (status = 401, description = "Missing or invalid API key"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Schedule not found", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn delete_schedule(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    require_permission(&user, Permission::SubmitSchedule)?;

    let storage = &state.storage;

    for s in [ScheduleState::Active, ScheduleState::AwaitingApproval] {
        match storage.delete_schedule(s, &id) {
            Ok(()) => return Ok(StatusCode::NO_CONTENT),
            Err(StorageError::NotFound(_)) => continue,
            Err(e) => return Err(e.into()),
        }
    }

    Err(ApiError::NotFound)
}

#[utoipa::path(
    post,
    path = "/api/schedules/{id}/approve",
    tag = "schedules",
    params(
        ("id" = String, Path, description = "Schedule ID")
    ),
    responses(
        (status = 200, description = "Schedule approved", body = ScheduleResponse),
        (status = 401, description = "Missing or invalid API key"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Schedule not found", body = ErrorResponse),
        (status = 409, description = "Schedule overlaps with existing", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn approve_schedule(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    require_permission(&user, Permission::ApproveSchedule)?;

    let storage = &state.storage;

    let (entry, _) = storage.get_schedule(ScheduleState::AwaitingApproval, &id)?;

    if storage.check_overlap(entry.start, entry.end, None)? {
        return Err(ApiError::Conflict("schedule_overlap"));
    }

    storage.move_schedule(ScheduleState::AwaitingApproval, ScheduleState::Active, &id)?;

    let mut response = ScheduleResponse::from(entry);
    response.status = "approved".to_string();

    Ok((StatusCode::OK, Json(response)))
}

#[utoipa::path(
    post,
    path = "/api/schedules/{id}/reject",
    tag = "schedules",
    params(
        ("id" = String, Path, description = "Schedule ID")
    ),
    responses(
        (status = 204, description = "Schedule rejected"),
        (status = 401, description = "Missing or invalid API key"),
        (status = 403, description = "Insufficient permissions"),
        (status = 404, description = "Schedule not found", body = ErrorResponse)
    ),
    security(("api_key" = []))
)]
pub async fn reject_schedule(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Path(id): Path<String>,
) -> ApiResult<impl IntoResponse> {
    require_permission(&user, Permission::ApproveSchedule)?;

    let storage = &state.storage;
    storage.delete_schedule(ScheduleState::AwaitingApproval, &id)?;

    Ok(StatusCode::NO_CONTENT)
}

fn deserialize_option_datetime<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    let value = Option::<String>::deserialize(deserializer)?;
    match value {
        Some(raw) => DateTime::parse_from_rfc3339(&raw)
            .map(|dt| Some(dt.with_timezone(&Utc)))
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}

fn schedule_value_to_string(value: &serde_yaml::Value) -> Option<String> {
    match value {
        serde_yaml::Value::Null => Some(String::from("null")),
        serde_yaml::Value::Bool(b) => Some(b.to_string()),
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        serde_yaml::Value::String(s) => Some(s.clone()),
        serde_yaml::Value::Sequence(seq) => serde_yaml::to_string(seq).ok(),
        serde_yaml::Value::Mapping(map) => serde_yaml::to_string(map).ok(),
        serde_yaml::Value::Tagged(tagged) => schedule_value_to_string(&tagged.value),
    }
}
