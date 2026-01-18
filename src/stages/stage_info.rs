use super::StageId;
use crate::types::StageType;
use serde::{Deserialize, Serialize};

/// Extensible stage information - core topology node metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageInfo {
    pub id: StageId,
    /// Human-readable name (for debugging/logging/UI)
    pub name: String,
    /// Semantic stage type used for validation and runtime coordination
    pub stage_type: StageType,

    /// Optional extension point for additional metadata (middleware, UI hints, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<StageExtensions>,
}

impl StageInfo {
    pub fn new(id: StageId, name: impl Into<String>, stage_type: StageType) -> Self {
        Self {
            id,
            name: name.into(),
            stage_type,
            extensions: None,
        }
    }

    /// Create with auto-generated name
    pub fn auto_named(id: StageId, stage_type: StageType) -> Self {
        Self {
            id,
            name: format!("stage_{id}"),
            stage_type,
            extensions: None,
        }
    }
}

/// Future-proofing: extensible metadata container for stages
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StageExtensions {
    /// Middleware configuration (rate limiters, circuit breakers, retry policies)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub middleware: Option<serde_json::Value>,

    /// UI-specific hints (custom icons, colors, grouping)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_hints: Option<serde_json::Value>,
}

/// Legacy metadata type - use StageInfo + StageExtensions instead
#[deprecated(
    since = "0.2.0",
    note = "Use StageInfo with optional StageExtensions instead"
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageMetadata {
    pub id: StageId,
    pub name: String,
    pub stage_type: StageType,
    pub description: Option<String>,
}
