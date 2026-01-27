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

#[derive(Debug, Clone, Deserialize, PartialEq, Serialize, utoipa::ToSchema)]
pub struct RunCommand {
    #[schema(
        example = "ISS (ZARYA)\n1 25544U 98067A   26012.17690827  .00009276  00000-0  17471-3 0  9998\n2 25544  51.6333 351.7881 0007723   8.9804 351.1321 15.49250518547578"
    )]
    pub tle: String,
    pub end: Option<DateTime<chrono::Utc>>,
    pub rotator: Option<String>,
    pub radio: Option<RadioConfig>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Command {
    RotatorPark {
        rotator: String,
    },
    Run(RunCommand),
    Stop,
}
