// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Logical subgraph annotations (FLOWIP-086z-part-2, extended by FLOWIP-128a).
//!
//! Subgraph membership identifies stages that participate in higher-level
//! composite shapes (e.g. `ai_map_reduce:digest`). Each stage carries an
//! optional `StageSubgraphMembership`; the topology carries an overall
//! registry of `TopologySubgraphInfo` entries describing the shape.
//!
//! These types follow the `StageInfo` idiom: `#[non_exhaustive]` with a
//! structural constructor, so additive annotation fields are non-breaking at
//! both the serde layer (skip-when-absent) and the Rust layer (FLOWIP-128a
//! D4/D9). Identity strings (`role`, port names, edge lane strings, `kind`)
//! are durable identifiers; renaming one is a breaking change.

use crate::stages::StageId;
use serde::{Deserialize, Serialize};

fn schema_version_one() -> u32 {
    1
}

// serde's skip_serializing_if predicate takes the field by reference
fn is_schema_version_one(version: &u32) -> bool {
    *version == 1
}

/// Per-stage logical subgraph membership.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StageSubgraphMembership {
    pub subgraph_id: String,
    pub kind: String,
    pub binding: String,
    /// Stable member role id (`[a-z][a-z0-9_]*`, unique per composite).
    pub role: String,
    pub order: u16,
    pub is_entry: bool,
    pub is_exit: bool,
    /// Kind-scoped member classification (FLOWIP-128a D2), e.g. saga's
    /// `compensatable` / `retriable` / `pivot` / `compensation` / `driver`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
}

impl StageSubgraphMembership {
    /// Construct with annotations unset.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subgraph_id: impl Into<String>,
        kind: impl Into<String>,
        binding: impl Into<String>,
        role: impl Into<String>,
        order: u16,
        is_entry: bool,
        is_exit: bool,
    ) -> Self {
        Self {
            subgraph_id: subgraph_id.into(),
            kind: kind.into(),
            binding: binding.into(),
            role: role.into(),
            order,
            is_entry,
            is_exit,
            class: None,
        }
    }

    pub fn with_class(mut self, class: impl Into<String>) -> Self {
        self.class = Some(class.into());
        self
    }
}

/// Registry entry describing one logical subgraph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct TopologySubgraphInfo {
    pub subgraph_id: String,
    pub kind: String,
    pub binding: String,
    pub label: String,
    pub member_stage_ids: Vec<StageId>,
    pub internal_edges: Vec<SubgraphInternalEdge>,
    pub entry_stage_ids: Vec<StageId>,
    pub exit_stage_ids: Vec<StageId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_subgraph_id: Option<String>,
    pub collapsible: bool,
    /// Manifest schema version. Absent means 1; serialized only when > 1.
    #[serde(
        default = "schema_version_one",
        skip_serializing_if = "is_schema_version_one"
    )]
    pub schema_version: u32,
    /// Declared boundary ports (FLOWIP-128a D1).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub boundary_ports: Vec<BoundaryPortSpec>,
}

impl TopologySubgraphInfo {
    /// Construct with annotations unset (schema version 1, no ports, no
    /// kind extension).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        subgraph_id: impl Into<String>,
        kind: impl Into<String>,
        binding: impl Into<String>,
        label: impl Into<String>,
        member_stage_ids: Vec<StageId>,
        internal_edges: Vec<SubgraphInternalEdge>,
        entry_stage_ids: Vec<StageId>,
        exit_stage_ids: Vec<StageId>,
        collapsible: bool,
    ) -> Self {
        Self {
            subgraph_id: subgraph_id.into(),
            kind: kind.into(),
            binding: binding.into(),
            label: label.into(),
            member_stage_ids,
            internal_edges,
            entry_stage_ids,
            exit_stage_ids,
            parent_subgraph_id: None,
            collapsible,
            schema_version: 1,
            boundary_ports: Vec::new(),
        }
    }

    pub fn with_parent_subgraph_id(mut self, parent: impl Into<String>) -> Self {
        self.parent_subgraph_id = Some(parent.into());
        self
    }

    pub fn with_boundary_ports(mut self, ports: Vec<BoundaryPortSpec>) -> Self {
        self.boundary_ports = ports;
        self
    }
}

/// One internal edge within a subgraph; carries the structural endpoints
/// plus a lane label (`data`, `manifest`, `status`, `terminal`, or a
/// kind-declared extra; FLOWIP-128a D3). The field keeps its historical
/// `role` name for serialization compatibility; it is a lane, not a D2
/// member role.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct SubgraphInternalEdge {
    pub from_stage_id: StageId,
    pub to_stage_id: StageId,
    pub role: String,
}

impl SubgraphInternalEdge {
    pub fn new(from_stage_id: StageId, to_stage_id: StageId, lane: impl Into<String>) -> Self {
        Self {
            from_stage_id,
            to_stage_id,
            role: lane.into(),
        }
    }
}

/// Direction of a composite boundary port (FLOWIP-128a D1).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PortDirection {
    Input,
    Output,
}

/// A durable reference from one physical topology edge to the named composite
/// port that edge crosses (FLOWIP-128a B3).
///
/// The port definition remains canonical in [`BoundaryPortSpec`]. Direction,
/// member, and payload types are deliberately not duplicated here. A physical
/// composite-to-composite edge can carry two references, one for the upstream
/// composite's output port and one for the downstream composite's input port.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub struct CompositePortRef {
    pub subgraph_id: String,
    pub port_name: String,
}

impl CompositePortRef {
    pub fn new(subgraph_id: impl Into<String>, port_name: impl Into<String>) -> Self {
        Self {
            subgraph_id: subgraph_id.into(),
            port_name: port_name.into(),
        }
    }
}

/// Declared boundary port on a composite (FLOWIP-128a D1). External edges
/// bind to ports structurally by the downstream's declared input type, with
/// the default port as fallback; the port names the `(member, payload set)`
/// pair for diagnostics, Studio, and the future nominal binding surface.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[non_exhaustive]
pub struct BoundaryPortSpec {
    pub name: String,
    pub direction: PortDirection,
    pub member_stage_id: StageId,
    pub payload_event_types: Vec<String>,
    pub default: bool,
}

impl BoundaryPortSpec {
    pub fn new(
        name: impl Into<String>,
        direction: PortDirection,
        member_stage_id: StageId,
        payload_event_types: Vec<String>,
        default: bool,
    ) -> Self {
        Self {
            name: name.into(),
            direction,
            member_stage_id,
            payload_event_types,
            default,
        }
    }
}
