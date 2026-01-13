#![allow(dead_code)]
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct UdpOutput {
    pub send: String,
    pub format: String,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct Output {
    pub udp: Option<UdpOutput>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Command {
    Run {
        radio: String,
        bandwidth: String,
        #[serde(default)]
        out: Option<Output>,
        #[serde(default)]
        web_fft: bool,
    },
    Stop,
}
