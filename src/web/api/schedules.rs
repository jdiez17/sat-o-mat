use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use utoipa::ToSchema;

use crate::{
    scheduler::storage::{ScheduleEntry, ScheduleState, StorageError},
    scheduler::utils::yaml_value_to_str,
    scheduler::Schedule,
    web::api::error::{ApiError, ApiResult, ErrorResponse},
    web::auth::{require_permission, AppState, AuthenticatedUser},
};

use crate::web::config::Permission;

#[utoipa::path(
    post,
    path = "/api/schedules",
    tag = "schedules",
    request_body(content = String, content_type = "application/yaml"),
    responses(
        (status = 201, description = "Schedule submitted successfully", body = ScheduleEntry),
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
    let (entry, _approval_result) =
        storage.submit_schedule(&schedule, &body, state.config.approval.mode)?;

    Ok((StatusCode::CREATED, Json(entry)))
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
                    yaml_value_to_str(&value).map(|val| ScheduleVariable { name, value: val })
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
        (status = 200, description = "List of schedules", body = Vec<ScheduleEntry>),
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

    let mut filtered: Vec<ScheduleEntry> = Vec::new();
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
            filtered.push(entry);
        }
    }

    Ok((StatusCode::OK, Json(filtered)))
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ScheduleDetailResponse {
    #[serde(flatten)]
    pub schedule: ScheduleEntry,
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
                                yaml_value_to_str(&value)
                                    .map(|val| ScheduleVariable { name, value: val })
                            })
                            .collect()
                    })
                    .unwrap_or_default();
                return Ok((
                    StatusCode::OK,
                    Json(ScheduleDetailResponse {
                        schedule: entry,
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
        (status = 200, description = "Schedule approved", body = ScheduleEntry),
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

    let entry = storage.approve_schedule(&id)?;

    Ok((StatusCode::OK, Json(entry)))
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
