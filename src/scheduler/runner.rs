use std::sync::{Arc, Mutex};

use thiserror::Error;

use crate::{executor::Executor, scheduler::Schedule, tracker::Tracker};

#[allow(dead_code)]
pub struct Runner {
    pub schedule: Schedule,
    pub executor: Executor,
    pub tracker: Arc<Mutex<Tracker>>,
}

#[derive(Debug, Error)]
pub enum RunnerError {}

type RunnerResult<T> = Result<T, RunnerError>;

impl Runner {
    /*
        pub fn new() -> Self {
            Self {}
        }
    */

    pub fn run(self) -> RunnerResult<()> {
        for (i, step) in self.schedule.steps.iter().enumerate() {
            println!("step {i}: {step:?}");
        }

        Ok(())
    }
}
