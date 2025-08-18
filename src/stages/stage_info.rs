use super::StageId;
use crate::types::StageType;
use serde::{Deserialize, Serialize};

/// Stage information - combines ID with human-readable name
#[derive(Debug, Clone)]
pub struct StageInfo {
    pub id: StageId,
    pub name: String,  // For debugging/logging only - never used for logic!
}

impl StageInfo {
    pub fn new(id: StageId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
        }
    }
    
    /// Create with auto-generated name
    pub fn auto_named(id: StageId) -> Self {
        Self {
            name: format!("stage_{}", id),
            id,
        }
    }
}

/// Extended metadata for stages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMetadata {
    pub id: StageId,
    pub name: String,
    pub stage_type: StageType,
    pub description: Option<String>,
}