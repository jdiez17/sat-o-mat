use axum::{extract::State, response::IntoResponse};

use crate::web::auth::AppState;

use super::templates::DashboardTemplate;

pub async fn dashboard(State(_state): State<AppState>) -> impl IntoResponse {
    DashboardTemplate {}
}
