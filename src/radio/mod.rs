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

/// Execute a radio command
pub fn execute_command(cmd: &Command) -> Result<(), String> {
    match cmd {
        Command::Run {
            radio,
            bandwidth,
            out: _,
            web_fft: _,
        } => {
            log::warn!(
                "Radio command not yet implemented: {} ({})",
                radio,
                bandwidth
            );
            Ok(())
        }
        Command::Stop => {
            log::warn!("Radio stop command not yet implemented");
            Ok(())
        }
    }
}
