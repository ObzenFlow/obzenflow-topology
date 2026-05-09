// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use super::StageId;
use crate::types::{
    JoinMetadataInfo, MiddlewareInfo, StageRole, StageStatus, StageSubgraphMembership,
    StageTypingInfo, StageType,
};
use serde::{Deserialize, Serialize};

/// Canonical stage record carried by the topology document.
///
/// Structural fields (`id`, `name`, `stage_type`) drive validation, SCC
/// computation, and traversal. The remaining fields are optional
/// annotations that ObzenFlow populates during flow build (FLOWIP-114b);
/// graph algorithms must not depend on them. A `StageInfo` with every
/// annotation set to `None` is still a valid topology stage.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[non_exhaustive]
pub struct StageInfo {
    pub id: StageId,
    /// Human-readable name (for debugging/logging/UI)
    pub name: String,
    /// Semantic stage type used for validation and runtime coordination
    pub stage_type: StageType,

    /// Optional extension point for additional metadata (legacy).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<StageExtensions>,

    /// Coarse runtime lifecycle state (FLOWIP-059).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<StageStatus>,

    /// Connection role derived from `stage_type` at flow build time.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<StageRole>,

    /// Whether the stage participates in a cycle (Tarjan SCC). Cached
    /// during topology construction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub is_cycle_member: Option<bool>,

    /// Structured middleware configuration (FLOWIP-059).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub middleware: Option<MiddlewareInfo>,

    /// Catalog/stream source classification for join stages (FLOWIP-082i).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join_metadata: Option<JoinMetadataInfo>,

    /// Authoring-time stage type contracts (FLOWIP-114b).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub typing: Option<StageTypingInfo>,

    /// Logical subgraph membership (FLOWIP-086z-part-2).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subgraph: Option<StageSubgraphMembership>,
}

impl StageInfo {
    /// Construct with all annotation fields unset.
    pub fn new(id: StageId, name: impl Into<String>, stage_type: StageType) -> Self {
        Self {
            id,
            name: name.into(),
            stage_type,
            extensions: None,
            status: None,
            role: None,
            is_cycle_member: None,
            middleware: None,
            join_metadata: None,
            typing: None,
            subgraph: None,
        }
    }

    /// Create with auto-generated name and all annotations unset.
    pub fn auto_named(id: StageId, stage_type: StageType) -> Self {
        Self::new(id, format!("stage_{id}"), stage_type)
    }

    pub fn with_status(mut self, status: StageStatus) -> Self {
        self.status = Some(status);
        self
    }

    pub fn with_role(mut self, role: StageRole) -> Self {
        self.role = Some(role);
        self
    }

    pub fn with_is_cycle_member(mut self, is_cycle_member: bool) -> Self {
        self.is_cycle_member = Some(is_cycle_member);
        self
    }

    pub fn with_middleware(mut self, middleware: MiddlewareInfo) -> Self {
        self.middleware = Some(middleware);
        self
    }

    pub fn with_join_metadata(mut self, join_metadata: JoinMetadataInfo) -> Self {
        self.join_metadata = Some(join_metadata);
        self
    }

    pub fn with_typing(mut self, typing: StageTypingInfo) -> Self {
        self.typing = Some(typing);
        self
    }

    pub fn with_subgraph(mut self, subgraph: StageSubgraphMembership) -> Self {
        self.subgraph = Some(subgraph);
        self
    }
}

/// Future-proofing: extensible metadata container for stages.
///
/// Predates the typed annotation fields on `StageInfo`. New annotations
/// should be added as typed fields directly; this container is retained
/// for forward-compat with existing payloads.
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
