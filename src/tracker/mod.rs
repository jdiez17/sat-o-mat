#![allow(dead_code)]
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct RadioConfig {
    pub device: String,
    pub frequencies: Frequencies,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Frequencies {
    pub uplink: String,
    pub downlink: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Command {
    Initialize {
        tle: String,
        rotator: String,
        radio: RadioConfig,
    },
    RotatorPark {
        rotator: String,
    },
}
