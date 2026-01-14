use chrono::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize, utoipa::ToSchema)]
pub struct RadioConfig {
    pub device: String,
    pub frequencies: Frequencies,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize, utoipa::ToSchema)]
pub struct Frequencies {
    pub uplink: String,
    pub downlink: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Command {
    RotatorPark {
        rotator: String,
    },
    Run {
        tle: String,
        rotator: Option<String>,
        radio: Option<RadioConfig>,
    },
    RunFixedDuration {
        tle: String,
        start: DateTime<chrono::Utc>,
        end: DateTime<chrono::Utc>,
        rotator: Option<String>,
        radio: Option<RadioConfig>,
    },
    Stop,
}
