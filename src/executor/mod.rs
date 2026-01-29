#![allow(dead_code)]
use serde::Deserialize;
use std::io;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{process::Child, sync::mpsc};

use crate::abort::AbortSignal;

mod process;

#[derive(Debug, Clone, Deserialize, Default, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum OnFail {
    #[default]
    Abort,
    Continue,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Command {
    RunShell {
        cmd: String,
        #[serde(default)]
        on_fail: OnFail,
    },
    Stop,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutorError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("Command failed with exit code {0}")]
    CommandFailed(i32),
    #[error("Process was killed")]
    Killed,
    #[error("No process is running")]
    NotRunning,
}

pub type ExecutorResult<T> = Result<T, ExecutorError>;

struct TrackedProcess {
    child: Arc<Mutex<Option<Child>>>,
}

pub struct Executor {
    artifacts_dir: PathBuf,
    abort_tx: mpsc::Sender<AbortSignal>,
    processes: Vec<TrackedProcess>,
}

impl Executor {
    pub fn new(artifacts_dir: PathBuf, abort_tx: mpsc::Sender<AbortSignal>) -> Self {
        Self {
            artifacts_dir,
            abort_tx,
            processes: Vec::new(),
        }
    }

    /// Execute an executor command
    pub fn execute_command(&mut self, cmd: &Command, step_index: usize) -> ExecutorResult<()> {
        match cmd {
            Command::RunShell { cmd, on_fail } => {
                let result = self.run_shell(cmd, step_index, on_fail);

                if let Err(e) = &result {
                    log::error!("Step {} failed to start: {}", step_index, e);
                    return result;
                }
            }
            Command::Stop => {
                log::info!("Stopping all executor processes");
                self.stop_all()?;
            }
        }

        Ok(())
    }

    pub fn run_shell(
        &mut self,
        cmd: &str,
        step_index: usize,
        on_fail: &OnFail,
    ) -> ExecutorResult<()> {
        self.processes.push(process::spawn(
            cmd,
            step_index,
            on_fail.clone(),
            self.abort_tx.clone(),
            &self.artifacts_dir,
        )?);

        Ok(())
    }

    pub fn stop_all(&mut self) -> ExecutorResult<()> {
        log::debug!("Stopping all child processes");

        for process in self.processes.iter() {
            let mut child_opt = process.child.lock().unwrap();
            if let Some(mut child) = child_opt.take() {
                let pid = child.id();
                match child.kill() {
                    Ok(_) => log::debug!("Killed process (PID: {:?})", pid),
                    Err(e) => log::warn!("Failed to kill process: {}", e),
                }
            }
        }

        self.processes.clear();
        Ok(())
    }
}

impl Drop for Executor {
    fn drop(&mut self) {
        self.stop_all().unwrap()
    }
}
