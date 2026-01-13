use axum::{extract::State, response::IntoResponse};

use crate::web::auth::AppState;

use super::templates::{DashboardTemplate, TimelineTemplate};

pub async fn dashboard(State(_state): State<AppState>) -> impl IntoResponse {
    DashboardTemplate {}
}

pub async fn timeline(State(_state): State<AppState>) -> impl IntoResponse {
    TimelineTemplate {}
}
