use chrono::{DateTime, Utc};
use serde::Serialize;
use utoipa::ToSchema;

/// Information about a single satellite from TLE
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SatelliteInfo {
    pub name: String,
    pub norad_id: u32,
    pub tle_source: String,
}

/// A predicted satellite pass
#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct Pass {
    pub satellite: String,
    pub norad_id: u32,
    pub aos: DateTime<Utc>,
    pub los: DateTime<Utc>,
    pub tca: DateTime<Utc>,
    pub max_elevation_deg: f64,
    pub aos_azimuth_deg: f64,
    pub los_azimuth_deg: f64,
    pub duration_seconds: i64,
    pub orbit_number: Option<u32>,
}
