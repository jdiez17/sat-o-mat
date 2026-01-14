use chrono::{DateTime, Duration, Utc};
use sgp4::{Constants, Elements};

use crate::predict::error::PredictError;
use crate::predict::types::Pass;
use crate::predict::{propagate_sample, FrequencyPlan, GroundStation};

const COARSE_STEP_SECONDS: i64 = 60; // 1 minute for initial scan
const FINE_STEP_SECONDS: i64 = 1; // 1 second for refinement
const HORIZON_ELEVATION: f64 = 0.0;

/// Find all passes for a satellite within a time range
pub fn predict_passes(
    station: &GroundStation,
    elements: &Elements,
    constants: &Constants,
    satellite_name: &str,
    norad_id: u32,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    min_elevation: f64,
) -> Result<Vec<Pass>, PredictError> {
    let mut passes = Vec::new();
    let mut cursor = start;
    let coarse_step = Duration::seconds(COARSE_STEP_SECONDS);

    // Use empty frequency plan (we don't need Doppler for pass prediction)
    let frequencies = FrequencyPlan {
        uplink_hz: None,
        downlink_hz: None,
    };

    let mut prev_visible = false;
    let mut pass_start: Option<DateTime<Utc>> = None;
    let mut max_el = 0.0;
    let mut max_el_time = cursor;
    let mut aos_az = 0.0;

    while cursor <= end {
        let sample = propagate_sample(station, elements, constants, cursor, &frequencies)
            .map_err(|e| PredictError::Propagation(e.to_string()))?;

        let visible = sample.elevation_deg >= HORIZON_ELEVATION;

        if visible && !prev_visible {
            // AOS detected - refine to find exact crossing
            let refined_aos = refine_crossing(
                station,
                elements,
                constants,
                cursor - coarse_step,
                cursor,
                true,
                &frequencies,
            )?;
            pass_start = Some(refined_aos.0);
            aos_az = refined_aos.1;
            max_el = sample.elevation_deg;
            max_el_time = cursor;
        } else if visible && pass_start.is_some() {
            // Track maximum elevation during pass
            if sample.elevation_deg > max_el {
                max_el = sample.elevation_deg;
                max_el_time = cursor;
            }
        } else if !visible && prev_visible && pass_start.is_some() {
            // LOS detected - refine and create pass
            let refined_los = refine_crossing(
                station,
                elements,
                constants,
                cursor - coarse_step,
                cursor,
                false,
                &frequencies,
            )?;

            if max_el >= min_elevation {
                let pass = Pass {
                    satellite: satellite_name.to_string(),
                    norad_id,
                    aos: pass_start.unwrap(),
                    los: refined_los.0,
                    tca: max_el_time,
                    max_elevation_deg: round2(max_el),
                    aos_azimuth_deg: round2(aos_az),
                    los_azimuth_deg: round2(refined_los.1),
                    duration_seconds: (refined_los.0 - pass_start.unwrap()).num_seconds(),
                    orbit_number: None,
                };
                passes.push(pass);
            }
            pass_start = None;
            max_el = 0.0;
        }

        prev_visible = visible;
        cursor += coarse_step;
    }

    // Handle pass in progress at end of window
    if pass_start.is_some() {
        let sample = propagate_sample(station, elements, constants, end, &frequencies)
            .map_err(|e| PredictError::Propagation(e.to_string()))?;

        if max_el >= min_elevation {
            let pass = Pass {
                satellite: satellite_name.to_string(),
                norad_id,
                aos: pass_start.unwrap(),
                los: end,
                tca: max_el_time,
                max_elevation_deg: round2(max_el),
                aos_azimuth_deg: round2(aos_az),
                los_azimuth_deg: round2(sample.azimuth_deg),
                duration_seconds: (end - pass_start.unwrap()).num_seconds(),
                orbit_number: None,
            };
            passes.push(pass);
        }
    }

    Ok(passes)
}

/// Binary search to find exact horizon crossing time
fn refine_crossing(
    station: &GroundStation,
    elements: &Elements,
    constants: &Constants,
    before: DateTime<Utc>,
    after: DateTime<Utc>,
    is_aos: bool, // true = rising, false = setting
    frequencies: &FrequencyPlan,
) -> Result<(DateTime<Utc>, f64), PredictError> {
    let mut low = before;
    let mut high = after;

    while (high - low).num_seconds() > FINE_STEP_SECONDS {
        let mid = low + (high - low) / 2;
        let sample = propagate_sample(station, elements, constants, mid, frequencies)
            .map_err(|e| PredictError::Propagation(e.to_string()))?;

        let above = sample.elevation_deg >= HORIZON_ELEVATION;
        if is_aos {
            if above {
                high = mid;
            } else {
                low = mid;
            }
        } else {
            if above {
                low = mid;
            } else {
                high = mid;
            }
        }
    }

    let final_sample = propagate_sample(station, elements, constants, high, frequencies)
        .map_err(|e| PredictError::Propagation(e.to_string()))?;

    Ok((high, final_sample.azimuth_deg))
}

fn round2(v: f64) -> f64 {
    (v * 100.0).round() / 100.0
}
