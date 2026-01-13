use std::{collections::HashMap, path::PathBuf};

use strum_macros::Display;
use thiserror::Error;

use crate::scheduler::{parser, Schedule};

#[derive(Display)]
pub enum ScheduleState {
    Active,
    AwaitingApproval,
    Error,
}

#[derive(Error, Debug)]
pub enum GetSchedulesError {
    #[error("IO error accessing path {1}: {0}")]
    Io(String, std::io::Error),
    #[error("Schedule parse error: {0}, file {1}")]
    ScheduleParse(String, parser::ParseError),
}

pub fn get_schedules(
    mut base: PathBuf,
    _state: ScheduleState,
) -> Result<HashMap<String, Schedule>, GetSchedulesError> {
    base.push(_state.to_string());
    let path = base.as_path();

    if !path.exists() {
        return Ok(HashMap::new());
    }

    let entries = path
        .read_dir()
        .map_err(|e| GetSchedulesError::Io(path.display().to_string(), e))?;

    let mut schedules = HashMap::new();

    for entry in entries {
        let entry = entry.map_err(|e| GetSchedulesError::Io(path.display().to_string(), e))?;
        let entry_path = entry.path();

        // Skip non-files
        if !entry_path.is_file() {
            continue;
        }

        // Read and parse schedule
        let content = std::fs::read_to_string(&entry_path)
            .map_err(|e| GetSchedulesError::Io(entry_path.display().to_string(), e))?;

        let schedule = Schedule::from_str(&content)
            .map_err(|e| GetSchedulesError::ScheduleParse(entry_path.display().to_string(), e))?;

        let filename = entry.file_name();
        schedules.insert(filename, schedule);
    }
}
