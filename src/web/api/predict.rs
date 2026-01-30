use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use utoipa::ToSchema;

use crate::predict::{predict_passes, Pass};
use crate::web::api::error::{ApiError, ApiResult};
use crate::web::auth::{require_permission, AppState, AuthenticatedUser};
use crate::web::config::Permission;

#[derive(Debug, Deserialize, ToSchema)]
pub struct PredictQuery {
    #[serde(deserialize_with = "deserialize_datetime")]
    pub start: DateTime<Utc>,
    #[serde(deserialize_with = "deserialize_datetime")]
    pub end: DateTime<Utc>,
    #[serde(default)]
    pub min_elevation: Option<f64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PredictResponse {
    pub passes: Vec<Pass>,
    pub satellite_count: usize,
}

#[utoipa::path(
    get,
    path = "/api/predict",
    tag = "predict",
    params(
        ("start" = String, Query, description = "Start time (RFC3339)"),
        ("end" = String, Query, description = "End time (RFC3339)"),
        ("min_elevation" = Option<f64>, Query, description = "Minimum elevation filter (degrees)")
    ),
    responses(
        (status = 200, description = "Pass predictions", body = PredictResponse),
        (status = 400, description = "Invalid parameters"),
        (status = 401, description = "Unauthorized"),
        (status = 503, description = "No satellites loaded or predictions not configured")
    ),
    security(("api_key" = []))
)]
pub async fn list_predictions(
    State(state): State<AppState>,
    user: AuthenticatedUser,
    Query(query): Query<PredictQuery>,
) -> ApiResult<impl IntoResponse> {
    require_permission(&user, Permission::ListPredictions)?;

    let tle_loader = state
        .tle_loader
        .as_ref()
        .ok_or_else(|| ApiError::Validation("Predictions not configured".into()))?;

    let min_el = query.min_elevation.unwrap_or(
        state
            .config
            .predict
            .as_ref()
            .map(|c| c.default_min_elevation)
            .unwrap_or(0.0),
    );

    // Get ground station from config
    let station = crate::predict::GroundStation::from_coordinates(
        &state.config.station.coordinates,
        Some(state.config.station.altitude_m),
    )
    .ok_or_else(|| ApiError::Validation("Invalid station coordinates".into()))?;

    // Get all satellites from loader
    let loader = tle_loader.read().await;
    let satellites = loader.satellites();

    if satellites.is_empty() {
        return Err(ApiError::Validation("No satellites loaded".into()));
    }

    // Predict passes for all satellites
    let mut all_passes = Vec::new();
    for sat in satellites {
        match predict_passes(
            &station,
            &sat.elements,
            &sat.constants,
            &sat.info.name,
            sat.info.norad_id,
            query.start,
            query.end,
            min_el,
        ) {
            Ok(passes) => all_passes.extend(passes),
            Err(e) => {
                log::warn!("Failed to predict passes for {}: {}", sat.info.name, e);
                // Continue with other satellites
            }
        }
    }

    // Sort by AOS time
    all_passes.sort_by_key(|p| p.aos);

    // Count unique satellites
    let satellites_count: HashSet<_> = all_passes.iter().map(|p| p.norad_id).collect();

    Ok((
        StatusCode::OK,
        Json(PredictResponse {
            passes: all_passes,
            satellite_count: satellites_count.len(),
        }),
    ))
}

fn deserialize_datetime<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    DateTime::parse_from_rfc3339(&s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(serde::de::Error::custom)
}
