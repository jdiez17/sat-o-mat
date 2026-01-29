use std::{
    fs, io,
    path::{Path, PathBuf},
};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::scheduler::storage::ScheduleState;

pub struct ArtifactsManager {
    base_dir: PathBuf,
    execution_log: ExecutionLog,
}

impl ArtifactsManager {
    pub fn new(base_dir: PathBuf, schedule_id: &str) -> io::Result<Self> {
        let artifacts_dir = base_dir.join("artifacts").join(schedule_id);
        fs::create_dir_all(&artifacts_dir)?;
        Ok(Self {
            base_dir: artifacts_dir,
            execution_log: ExecutionLog::new(schedule_id.to_string()),
        })
    }

    pub fn add_step_result(&mut self, step_result: StepResult) -> io::Result<()> {
        self.execution_log.step_results.push(step_result);
        self.execution_log.save(&self.execution_log_path())
    }

    pub fn finish_with_state(&mut self, state: ScheduleState) -> io::Result<()> {
        self.execution_log.state = state;
        self.execution_log.completed_at = Some(Utc::now());
        self.execution_log.save(&self.execution_log_path())
    }

    pub fn execution_log(&self) -> &ExecutionLog {
        &self.execution_log
    }

    #[allow(dead_code)]
    pub fn artifacts_dir(&self) -> &PathBuf {
        &self.base_dir
    }

    fn execution_log_path(&self) -> PathBuf {
        self.base_dir.join("execution_log.yaml")
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepResult {
    pub step_index: usize,
    pub command_type: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionLog {
    pub schedule_id: String,
    pub state: ScheduleState,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub step_results: Vec<StepResult>,
}

impl ExecutionLog {
    pub fn new(schedule_id: String) -> Self {
        Self {
            schedule_id,
            state: ScheduleState::Running,
            started_at: Utc::now(),
            completed_at: None,
            step_results: Vec::new(),
        }
    }
    pub fn save(&self, path: &Path) -> io::Result<()> {
        fs::write(
            path,
            serde_yaml::to_string(self)
                .map_err(|e| io::Error::other(format!("Failed to serialize log: {}", e)))?,
        )
    }
}
