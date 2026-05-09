// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use crate::stages::StageId;
use crate::types::{ContractInfo, EdgeTypingInfo};
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

/// Edge direction kind - preserves operator semantics (`|>` vs `<|`)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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

/// Legacy extension point for edge metadata.
///
/// Predates the typed annotation fields on `DirectedEdge`. New annotations
/// should be added as typed fields directly; this container is retained
/// for forward-compat with existing payloads.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EdgeExtensions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contract: Option<serde_json::Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ui_hints: Option<serde_json::Value>,
}

/// Directed edge - explicit flow direction between stages.
///
/// Structural fields (`from`, `to`, `kind`) drive validation and traversal.
/// The remaining fields are optional annotations populated during flow
/// build (FLOWIP-114b); validation, deduplication, and graph algorithms
/// must remain agnostic to them.
///
/// Equality and hashing intentionally consider only the structural triple
/// `(from, to, kind)`. Two edges with the same endpoints but different
/// annotations are treated as the same edge for deduplication purposes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct DirectedEdge {
    pub from: StageId,
    pub to: StageId,
    pub kind: EdgeKind,

    /// Legacy throughput field, always `None` from this crate. Runtime
    /// metrics are exported via `/metrics`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub events_per_sec: Option<f64>,

    /// Structural contracts attached to this edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contracts: Option<Vec<ContractInfo>>,

    /// Derived per-edge payload typing projection (FLOWIP-114b).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typing: Option<EdgeTypingInfo>,
}

impl DirectedEdge {
    pub fn new(from: StageId, to: StageId, kind: EdgeKind) -> Self {
        Self {
            from,
            to,
            kind,
            events_per_sec: None,
            contracts: None,
            typing: None,
        }
    }

    pub fn with_events_per_sec(mut self, events_per_sec: f64) -> Self {
        self.events_per_sec = Some(events_per_sec);
        self
    }

    pub fn with_contracts(mut self, contracts: Vec<ContractInfo>) -> Self {
        self.contracts = Some(contracts);
        self
    }

    pub fn with_typing(mut self, typing: EdgeTypingInfo) -> Self {
        self.typing = Some(typing);
        self
    }
}

impl PartialEq for DirectedEdge {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from && self.to == other.to && self.kind == other.kind
    }
}

impl Eq for DirectedEdge {}

impl Hash for DirectedEdge {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.from.hash(state);
        self.to.hash(state);
        self.kind.hash(state);
    }
}

impl std::fmt::Display for DirectedEdge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {} {}", self.from, self.kind, self.to)
    }
}
