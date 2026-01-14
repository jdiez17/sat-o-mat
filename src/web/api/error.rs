use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use utoipa::ToSchema;

use crate::{scheduler::storage::StorageError, web::auth::PermissionError};

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

pub type ApiResult<T> = Result<T, ApiError>;

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
