use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::collections::HashSet;
use std::sync::Arc;

use crate::scheduler::Storage;

use super::config::{Config, Permission};

#[derive(Clone)]
pub struct AuthenticatedUser {
    pub name: String,
    pub permissions: HashSet<Permission>,
}

impl AuthenticatedUser {
    pub fn has_permission(&self, permission: Permission) -> bool {
        self.permissions.contains(&permission)
    }
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub storage: Arc<Storage>,
}

pub enum AuthError {
    MissingAuth,
    InvalidFormat,
    InvalidKey,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::MissingAuth => (StatusCode::UNAUTHORIZED, "Missing Authorization header"),
            AuthError::InvalidFormat => (StatusCode::UNAUTHORIZED, "Invalid Authorization format"),
            AuthError::InvalidKey => (StatusCode::UNAUTHORIZED, "Invalid API key"),
        };
        (status, Json(json!({ "error": message }))).into_response()
    }
}

pub struct PermissionError;

impl IntoResponse for PermissionError {
    fn into_response(self) -> Response {
        (
            StatusCode::FORBIDDEN,
            Json(json!({ "error": "Insufficient permissions" })),
        )
            .into_response()
    }
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .ok_or(AuthError::MissingAuth)?
            .to_str()
            .map_err(|_| AuthError::InvalidFormat)?;

        let key = auth_header
            .strip_prefix("Bearer ")
            .ok_or(AuthError::InvalidFormat)?;

        let api_key = state
            .config
            .find_api_key(key)
            .ok_or(AuthError::InvalidKey)?;

        Ok(AuthenticatedUser {
            name: api_key.name.clone(),
            permissions: api_key.permissions.clone(),
        })
    }
}

pub fn require_permission(
    user: &AuthenticatedUser,
    permission: Permission,
) -> Result<(), PermissionError> {
    if user.has_permission(permission) {
        Ok(())
    } else {
        Err(PermissionError)
    }
}
