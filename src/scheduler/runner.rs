use chrono::Utc;
use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::Mutex;

use crate::abort::AbortSignal;
use crate::scheduler::artifacts::{ArtifactsManager, StepResult};
use crate::scheduler::parser;
use crate::scheduler::parser::Step;
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
    abort_rx: mpsc::Receiver<AbortSignal>,
}

impl Runner {
    pub fn new(
        schedule_id: String,
        schedule: Schedule,
        tracker: Arc<Mutex<Tracker>>,
        base_dir: PathBuf,
    ) -> RunnerResult<Self> {
        let artifacts = ArtifactsManager::new(base_dir, &schedule_id)?;

        let (abort_tx, abort_rx) = mpsc::channel();
        let executor = Executor::new(artifacts.artifacts_dir().clone(), abort_tx);

        Ok(Self {
            schedule_id,
            schedule,
            executor,
            tracker,
            artifacts,
            abort_rx,
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
            let time_until_next_step = if let Some(time_expr) = &step.time {
                let time_to_execute = time_expr.resolve(self.schedule.start);
                log::info!("Step {} waiting until {}", i, time_to_execute);

                let now = Utc::now();
                (time_to_execute - now)
                    .to_std()
                    .unwrap_or(Duration::from_secs(0))
            } else {
                Duration::ZERO
            };

            self.wait_and_check_abort(time_until_next_step)?;
            self.execute_step(i, step)?;
        }

        // Give background monitoring threads a moment to detect any failures
        self.wait_and_check_abort(Duration::from_millis(100))?;

        log::info!(
            "Schedule {} finished running successfully",
            self.schedule_id
        );
        Ok(())
    }

    fn execute_step(&mut self, index: usize, step: &Step) -> RunnerResult<()> {
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

        self.artifacts
            .add_step_result(StepResult::new(index, step, started_at, &result))?;

        result
    }

    /// Wait for a duration while checking for abort signals.
    /// Use Duration::ZERO for a non-blocking check.
    fn wait_and_check_abort(&mut self, duration: Duration) -> RunnerResult<()> {
        match self.abort_rx.recv_timeout(duration) {
            Ok(signal) => {
                self.artifacts
                    .update_step_result(signal.step, signal.reason.clone())?;

                Err(RunnerError::Aborted {
                    step: signal.step,
                    reason: signal.reason,
                })
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Ok(()),
            Err(mpsc::RecvTimeoutError::Disconnected) => Ok(()),
        }
    }
}
