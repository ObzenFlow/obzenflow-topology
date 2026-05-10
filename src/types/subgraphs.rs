// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Logical subgraph annotations (FLOWIP-086z-part-2).
//!
//! Subgraph membership identifies stages that participate in higher-level
//! composite shapes (e.g. `ai_map_reduce:digest`). Each stage carries an
//! optional `StageSubgraphMembership`; the topology carries an overall
//! registry of `TopologySubgraphInfo` entries describing the shape.

use crate::stages::StageId;
use serde::{Deserialize, Serialize};

/// Per-stage logical subgraph membership.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StageSubgraphMembership {
    pub subgraph_id: String,
    pub kind: String,
    pub binding: String,
    pub role: String,
    pub order: u16,
    pub is_entry: bool,
    pub is_exit: bool,
}

/// Registry entry describing one logical subgraph.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
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
}

/// One internal edge within a subgraph; carries the structural endpoints
/// plus an opaque role label.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubgraphInternalEdge {
    pub from_stage_id: StageId,
    pub to_stage_id: StageId,
    pub role: String,
}
