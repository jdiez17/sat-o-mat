use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    Auto,
    Manual,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalResult {
    Approved,
    Pending,
}

impl ApprovalResult {
    pub fn is_approved(&self) -> bool {
        matches!(self, ApprovalResult::Approved)
    }
}

pub fn evaluate_approval(mode: ApprovalMode) -> ApprovalResult {
    match mode {
        ApprovalMode::Auto => ApprovalResult::Approved,
        ApprovalMode::Manual => ApprovalResult::Pending,
    }
}
