// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use crate::stages::StageId;
use serde::{Deserialize, Serialize};

/// Edge direction kind - preserves operator semantics (`|>` vs `<|`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    /// Forward data flow (a |> b)
    Forward,
    /// Backward data flow / backpressure (a <| b)
    Backward,
}

impl std::fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeKind::Forward => write!(f, "|>"),
            EdgeKind::Backward => write!(f, "<|"),
        }
    }
}

/// Extension point for edge metadata
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EdgeExtensions {
    /// Contract configuration between stages
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<serde_json::Value>,

    /// UI-specific hints (edge styling, animation parameters)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_hints: Option<serde_json::Value>,
}

/// Directed edge - explicit flow direction between stages
///
/// Note: Extensions are not stored on the edge itself to preserve `Hash` and `Eq`
/// traits for use in deduplication. Use a separate `HashMap<(StageId, StageId), EdgeExtensions>`
/// if edge-level metadata is needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DirectedEdge {
    pub from: StageId,
    pub to: StageId,
    pub kind: EdgeKind,
}

impl DirectedEdge {
    pub fn new(from: StageId, to: StageId, kind: EdgeKind) -> Self {
        Self { from, to, kind }
    }
}

impl std::fmt::Display for DirectedEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.from, self.kind, self.to)
    }
}
