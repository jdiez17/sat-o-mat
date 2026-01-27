mod error;
mod ground_station;
mod parsing;
mod sample;
mod tracker;
mod types;

pub(crate) const SPEED_OF_LIGHT_KM_S: f64 = 299_792.458;
pub const EARTH_ROTATION_RAD_S: f64 = 7.292_115e-5;

pub use error::TrackerError;
pub use ground_station::GroundStation;
pub use sample::TrackerSample;
pub use tracker::{FrequencyPlan, Tracker, TrackerMode};
pub use types::{Command, RunCommand};
