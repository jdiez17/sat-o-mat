use chrono::{DateTime, Duration, Utc};
use sgp4::{Constants, Elements};

use super::parsing::parse_frequency_hz;
use crate::tracker::{
    FrequencyPlan, GroundStation, TrackerError, TrackerSample, EARTH_ROTATION_RAD_S,
    SPEED_OF_LIGHT_KM_S,
};

pub fn build_frequency_plan(uplink: Option<String>, downlink: Option<String>) -> FrequencyPlan {
    FrequencyPlan {
        uplink_hz: uplink.as_deref().and_then(parse_frequency_hz),
        downlink_hz: downlink.as_deref().and_then(parse_frequency_hz),
    }
}

pub fn build_trajectory(
    station: &GroundStation,
    elements: &Elements,
    constants: &Constants,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    frequencies: &FrequencyPlan,
    step: Duration,
) -> Result<Vec<TrackerSample>, TrackerError> {
    let mut cursor = start;
    let mut points = Vec::new();

    while cursor <= end {
        let sample = propagate_sample(station, elements, constants, cursor, frequencies)?;
        points.push(sample);
        cursor += step;
    }

    Ok(points)
}

pub fn propagate_sample(
    station: &GroundStation,
    elements: &Elements,
    constants: &Constants,
    timestamp: DateTime<Utc>,
    frequencies: &FrequencyPlan,
) -> Result<TrackerSample, TrackerError> {
    let minutes = elements
        .datetime_to_minutes_since_epoch(&timestamp.naive_utc())
        .map_err(|e| TrackerError::Propagation(e.to_string()))?;

    let prediction = constants
        .propagate(minutes)
        .map_err(|e| TrackerError::Propagation(e.to_string()))?;

    let sidereal =
        sgp4::iau_epoch_to_sidereal_time(sgp4::julian_years_since_j2000(&timestamp.naive_utc()));

    let sat_ecef = teme_to_ecef_position(prediction.position, sidereal);
    let sat_vel_ecef = teme_to_ecef_velocity(prediction.position, prediction.velocity, sidereal);

    let sta_ecef = station.position_ecef_km();
    let sta_vel = station.velocity_ecef_km_s();

    let dr = [
        sat_ecef[0] - sta_ecef[0],
        sat_ecef[1] - sta_ecef[1],
        sat_ecef[2] - sta_ecef[2],
    ];
    let range_km = (dr[0] * dr[0] + dr[1] * dr[1] + dr[2] * dr[2]).sqrt();

    let enu = ecef_to_enu(dr, station.lat_rad(), station.lon_rad());
    let azimuth = enu.0.atan2(enu.1).to_degrees().rem_euclid(360.0);
    let elevation = if range_km > 0.0 {
        (enu.2 / range_km).asin().to_degrees()
    } else {
        0.0
    };

    let los_unit = if range_km > 0.0 {
        [dr[0] / range_km, dr[1] / range_km, dr[2] / range_km]
    } else {
        [0.0, 0.0, 0.0]
    };
    let rel_vel = [
        sat_vel_ecef[0] - sta_vel[0],
        sat_vel_ecef[1] - sta_vel[1],
        sat_vel_ecef[2] - sta_vel[2],
    ];
    let range_rate_km_s =
        rel_vel[0] * los_unit[0] + rel_vel[1] * los_unit[1] + rel_vel[2] * los_unit[2];

    let doppler_downlink_hz = frequencies
        .downlink_hz
        .map(|f| apply_downlink_doppler(f, range_rate_km_s));
    let doppler_uplink_hz = frequencies
        .uplink_hz
        .map(|f| apply_uplink_doppler(f, range_rate_km_s));

    Ok(TrackerSample {
        timestamp,
        azimuth_deg: round2(azimuth),
        elevation_deg: round2(elevation),
        range_km: round2(range_km),
        range_rate_km_s: round2(range_rate_km_s),
        doppler_uplink_hz,
        doppler_downlink_hz,
    })
}

pub fn apply_downlink_doppler(freq_hz: f64, range_rate_km_s: f64) -> f64 {
    freq_hz * (1.0 - range_rate_km_s / SPEED_OF_LIGHT_KM_S)
}

pub fn apply_uplink_doppler(freq_hz: f64, range_rate_km_s: f64) -> f64 {
    freq_hz * (1.0 + range_rate_km_s / SPEED_OF_LIGHT_KM_S)
}

pub fn teme_to_ecef_position(pos_teme: [f64; 3], gmst: f64) -> [f64; 3] {
    let cos_gmst = gmst.cos();
    let sin_gmst = gmst.sin();
    [
        pos_teme[0] * cos_gmst + pos_teme[1] * sin_gmst,
        -pos_teme[0] * sin_gmst + pos_teme[1] * cos_gmst,
        pos_teme[2],
    ]
}

pub fn teme_to_ecef_velocity(pos_teme: [f64; 3], vel_teme: [f64; 3], gmst: f64) -> [f64; 3] {
    let cos_gmst = gmst.cos();
    let sin_gmst = gmst.sin();
    let pos = teme_to_ecef_position(pos_teme, gmst);
    let rotated = [
        vel_teme[0] * cos_gmst + vel_teme[1] * sin_gmst,
        -vel_teme[0] * sin_gmst + vel_teme[1] * cos_gmst,
        vel_teme[2],
    ];
    let rotation = [
        -EARTH_ROTATION_RAD_S * pos[1],
        EARTH_ROTATION_RAD_S * pos[0],
        0.0,
    ];
    [
        rotated[0] - rotation[0],
        rotated[1] - rotation[1],
        rotated[2] - rotation[2],
    ]
}

pub fn ecef_to_enu(dr: [f64; 3], lat_rad: f64, lon_rad: f64) -> (f64, f64, f64) {
    let sin_lat = lat_rad.sin();
    let cos_lat = lat_rad.cos();
    let sin_lon = lon_rad.sin();
    let cos_lon = lon_rad.cos();

    let east = -sin_lon * dr[0] + cos_lon * dr[1];
    let north = -sin_lat * cos_lon * dr[0] - sin_lat * sin_lon * dr[1] + cos_lat * dr[2];
    let up = cos_lat * cos_lon * dr[0] + cos_lat * sin_lon * dr[1] + sin_lat * dr[2];
    (east, north, up)
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}
