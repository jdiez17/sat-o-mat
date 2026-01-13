use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;
use utoipa::ToSchema;

use crate::scheduler::Schedule;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleState {
    Active,
    AwaitingApproval,
}

impl ScheduleState {
    pub fn folder_name(&self) -> &'static str {
        match self {
            ScheduleState::Active => "Active",
            ScheduleState::AwaitingApproval => "AwaitingApproval",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScheduleEntry {
    pub id: String,
    pub state: ScheduleState,
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Error)]
pub enum StorageError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Parse error: {0}")]
    Parse(#[from] crate::scheduler::parser::ParseError),
    #[error("Schedule not found: {0}")]
    NotFound(String),
    #[error("Schedule overlap detected")]
    Overlap,
}

pub struct Storage {
    base: PathBuf,
}

impl Storage {
    pub fn new(base: PathBuf) -> Self {
        Storage { base }
    }

    fn state_path(&self, state: ScheduleState) -> PathBuf {
        self.base.join(state.folder_name())
    }

    fn schedule_path(&self, state: ScheduleState, id: &str) -> PathBuf {
        self.state_path(state).join(format!("{}.yaml", id))
    }

    pub fn get_schedules(&self, state: ScheduleState) -> Result<Vec<ScheduleEntry>, StorageError> {
        let path = self.state_path(state);

        if !path.exists() {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        for entry in path.read_dir()? {
            let entry = entry?;
            let entry_path = entry.path();

            if !entry_path.is_file() {
                continue;
            }

            let id = entry_path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(String::from)
                .unwrap_or_default();

            let content = std::fs::read_to_string(&entry_path)?;
            let schedule = Schedule::from_str(&content)?;

            entries.push(ScheduleEntry {
                id,
                state,
                start: schedule.start,
                end: schedule.end,
            });
        }

        entries.sort_by_key(|e| e.start);
        Ok(entries)
    }

    pub fn get_schedule(
        &self,
        state: ScheduleState,
        id: &str,
    ) -> Result<(ScheduleEntry, String), StorageError> {
        let path = self.schedule_path(state, id);

        if !path.exists() {
            return Err(StorageError::NotFound(id.to_string()));
        }

        let content = std::fs::read_to_string(&path)?;
        let schedule = Schedule::from_str(&content)?;

        let entry = ScheduleEntry {
            id: id.to_string(),
            state,
            start: schedule.start,
            end: schedule.end,
        };

        Ok((entry, content))
    }

    pub fn save_schedule(
        &self,
        state: ScheduleState,
        id: &str,
        content: &str,
    ) -> Result<(), StorageError> {
        let folder = self.state_path(state);
        std::fs::create_dir_all(&folder)?;

        let path = self.schedule_path(state, id);
        std::fs::write(path, content)?;
        Ok(())
    }

    pub fn delete_schedule(&self, state: ScheduleState, id: &str) -> Result<(), StorageError> {
        let path = self.schedule_path(state, id);

        if !path.exists() {
            return Err(StorageError::NotFound(id.to_string()));
        }

        std::fs::remove_file(path)?;
        Ok(())
    }

    pub fn move_schedule(
        &self,
        from_state: ScheduleState,
        to_state: ScheduleState,
        id: &str,
    ) -> Result<(), StorageError> {
        let from_path = self.schedule_path(from_state, id);
        let to_folder = self.state_path(to_state);
        let to_path = self.schedule_path(to_state, id);

        if !from_path.exists() {
            return Err(StorageError::NotFound(id.to_string()));
        }

        std::fs::create_dir_all(&to_folder)?;
        std::fs::rename(from_path, to_path)?;
        Ok(())
    }

    pub fn check_overlap(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
        exclude_id: Option<&str>,
    ) -> Result<bool, StorageError> {
        let active = self.get_schedules(ScheduleState::Active)?;

        for entry in active {
            if let Some(excluded) = exclude_id {
                if entry.id == excluded {
                    continue;
                }
            }

            // Check if time ranges overlap
            // Two ranges [a, b] and [c, d] overlap if a < d && c < b
            if start < entry.end && entry.start < end {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub fn generate_id(&self, start: DateTime<Utc>) -> String {
        let uuid = uuid::Uuid::new_v4();
        let timestamp = start.format("%Y%m%dT%H%M%SZ");
        format!("{}_{}", timestamp, uuid)
    }
}
