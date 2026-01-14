use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;
use thiserror::Error;

use crate::scheduler::approval::ApprovalMode;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("YAML parse error: {0}")]
    Yaml(#[from] serde_yaml::Error),
}

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub station: StationConfig,
    pub web: WebConfig,
    pub schedules: SchedulesConfig,
    pub approval: ApprovalConfig,
    pub api_keys: Vec<ApiKey>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebConfig {
    #[serde(default = "default_bind")]
    pub bind: String,
}

fn default_bind() -> String {
    "0.0.0.0:8080".to_string()
}

#[derive(Debug, Clone, Deserialize)]
pub struct SchedulesConfig {
    pub base_folder: PathBuf,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApprovalConfig {
    pub mode: ApprovalMode,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApiKey {
    pub key: String,
    pub name: String,
    pub permissions: HashSet<Permission>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct StationConfig {
    pub name: Option<String>,
    pub coordinates: String,
    #[serde(default)]
    pub altitude_m: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    SubmitSchedule,
    ListSchedules,
    ApproveSchedule,
}

impl Config {
    pub fn from_file(path: &str) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = serde_yaml::from_str(&content)?;
        Ok(config)
    }

    pub fn find_api_key(&self, key: &str) -> Option<&ApiKey> {
        self.api_keys.iter().find(|k| k.key == key)
    }
}
