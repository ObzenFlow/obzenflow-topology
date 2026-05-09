// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Stage runtime status annotation.
//!
//! Coarse runtime state for a stage. The canonical topology carries this
//! as an optional annotation so clients can render basic lifecycle state
//! without an additional endpoint. It is not a substitute for live metrics.

use serde::{Deserialize, Serialize};

/// Coarse runtime lifecycle state for a stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StageStatus {
    Running,
    Completed,
    Failed,
    Pending,
}

impl StageStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            StageStatus::Running => "running",
            StageStatus::Completed => "completed",
            StageStatus::Failed => "failed",
            StageStatus::Pending => "pending",
        }
    }
}

impl std::fmt::Display for StageStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
