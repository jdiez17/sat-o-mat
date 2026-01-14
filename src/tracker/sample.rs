use chrono::DateTime;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct TrackerSample {
    pub timestamp: DateTime<chrono::Utc>,
    pub azimuth_deg: f64,
    pub elevation_deg: f64,
    pub range_km: f64,
    pub range_rate_km_s: f64,
    pub doppler_uplink_hz: Option<f64>,
    pub doppler_downlink_hz: Option<f64>,
}
