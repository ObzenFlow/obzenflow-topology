// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use crate::stages::StageId;
use crate::types::{CompositePortRef, ContractInfo, EdgeTypingInfo};
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

/// Directed edge - explicit flow direction between stages.
///
/// Structural fields (`from`, `to`, `kind`) drive validation and traversal.
/// `contracts` and `typing` are optional product annotations populated during
/// flow build (FLOWIP-114b). `composite_ports` is the reviewed structural
/// exception: it names a validated graph cut consumed by runtime boundary
/// classification (FLOWIP-128a B4). Generic edge identity and traversal still
/// use only the physical triple.
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

    /// Structural contracts attached to this edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contracts: Option<Vec<ContractInfo>>,

    /// Derived per-edge payload typing projection (FLOWIP-114b).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typing: Option<EdgeTypingInfo>,

    /// Named composite ports crossed by this physical edge (FLOWIP-128a B3).
    ///
    /// The binding is excluded from physical-edge equality and hashing so it
    /// cannot duplicate an edge. A vector is required because one
    /// composite-to-composite edge crosses two independently named ports.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub composite_ports: Vec<CompositePortRef>,
}

impl DirectedEdge {
    pub fn new(from: StageId, to: StageId, kind: EdgeKind) -> Self {
        Self {
            from,
            to,
            kind,
            contracts: None,
            typing: None,
            composite_ports: Vec::new(),
        }
    }

    pub fn with_contracts(mut self, contracts: Vec<ContractInfo>) -> Self {
        self.contracts = Some(contracts);
        self
    }

    pub fn with_typing(mut self, typing: EdgeTypingInfo) -> Self {
        self.typing = Some(typing);
        self
    }

    pub fn with_composite_ports(mut self, ports: Vec<CompositePortRef>) -> Self {
        self.composite_ports = ports;
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
