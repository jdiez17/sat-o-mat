#![allow(dead_code)]
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnFail {
    #[default]
    Abort,
    Continue,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Command {
    RunShell {
        cmd: String,
        #[serde(default)]
        on_fail: OnFail,
    },
}
