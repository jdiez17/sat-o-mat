use chrono::{DateTime, Utc};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::scheduler::artifacts::ArtifactsManager;
use crate::scheduler::artifacts::StepResult;
use crate::scheduler::parser;
use crate::scheduler::storage::ScheduleState;
use crate::{
    executor::{Executor, ExecutorError},
    scheduler::Schedule,
    tracker::{Tracker, TrackerError},
};

#[derive(Debug, Error)]
pub enum RunnerError {
    #[error("Executor error: {0}")]
    Executor(#[from] ExecutorError),
    #[error("Tracker error: {0}")]
    Tracker(#[from] TrackerError),
    #[error("Radio error: {0}")]
    Radio(String),
    #[allow(dead_code)]
    #[error("Aborted at step {step}: {reason}")]
    Aborted { step: usize, reason: String },
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

type RunnerResult<T> = Result<T, RunnerError>;

#[allow(dead_code)]
pub struct Runner {
    schedule_id: String,
    schedule: Schedule,
    executor: Executor,
    tracker: Arc<Mutex<Tracker>>,
    artifacts: ArtifactsManager,
}

impl Runner {
    pub fn new(
        schedule_id: String,
        schedule: Schedule,
        executor: Executor,
        tracker: Arc<Mutex<Tracker>>,
        base_dir: PathBuf,
    ) -> RunnerResult<Self> {
        let artifacts = ArtifactsManager::new(base_dir, &schedule_id)?;

        Ok(Self {
            schedule_id,
            schedule,
            executor,
            tracker,
            artifacts,
        })
    }

    pub fn run(mut self) -> RunnerResult<ArtifactsManager> {
        let result = self.run_internal();

        match result {
            Ok(_) => {
                self.artifacts.finish_with_state(ScheduleState::Completed)?;
                Ok(self.artifacts)
            }
            Err(e) => {
                self.artifacts.finish_with_state(ScheduleState::Failed)?;
                Err(e)
            }
        }
    }

    fn run_internal(&mut self) -> RunnerResult<()> {
        log::info!("Starting schedule {}", self.schedule_id);

        for (i, step) in self.schedule.steps.clone().iter().enumerate() {
            if let Some(time_expr) = &step.time {
                let target = time_expr.resolve(self.schedule.start);
                log::info!("Step {} waiting until {}", i, target);
                self.wait_until(target);
            }

            self.execute_step(i, step)?;
        }

        log::info!(
            "Schedule {} finished running successfully",
            self.schedule_id
        );
        Ok(())
    }

    fn wait_until(&self, target: DateTime<Utc>) {
        let now = Utc::now();
        if target > now {
            let duration = (target - now).to_std().unwrap_or(Duration::from_secs(0));
            std::thread::sleep(duration);
        }
    }

    fn execute_step(
        &mut self,
        index: usize,
        step: &crate::scheduler::parser::Step,
    ) -> RunnerResult<()> {
        let started_at = Utc::now();
        log::info!("Executing step {}: {:?}", index, step.command);

        let result: RunnerResult<()> = match &step.command {
            parser::Command::Executor(cmd) => self
                .executor
                .execute_command(cmd, index)
                .map_err(|e| e.into()),
            parser::Command::Tracker(cmd) => {
                let mut tracker = self.tracker.blocking_lock();
                tracker.execute_command(cmd).map_err(|e| e.into())
            }
            parser::Command::Radio(cmd) => {
                crate::radio::execute_command(cmd).map_err(RunnerError::Radio)
            }
        };

        self.artifacts.add_step_result(StepResult {
            step_index: index,
            command_type: format!("{:?}", step.command),
            started_at,
            completed_at: Some(Utc::now()),
            success: result.is_ok(),
            error: result.as_ref().err().map(|e| e.to_string()),
        })?;

        result
    }
}
