// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

pub mod contract;
pub mod join;
pub mod middleware;
pub mod scc_id;
pub mod stage_type;
pub mod status;
pub mod subgraphs;
pub mod typing;

use obzenflow_idkit::Id;

// Domain marker type for stages in the topology
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stage;

// Type alias for stage identifiers using phantom-typed ID
pub type StageId = Id<Stage>;

// Re-export stage type enums and the canonical annotation types.
pub use contract::ContractInfo;
pub use join::JoinMetadataInfo;
pub use middleware::{
    BackoffStrategy, CircuitBreakerInfo, MiddlewareInfo, OpenPolicy, RateLimiterInfo, RetryInfo,
};
pub use scc_id::SccId;
pub use stage_type::{StageRole, StageType};
pub use status::StageStatus;
pub use subgraphs::{StageSubgraphMembership, SubgraphInternalEdge, TopologySubgraphInfo};
pub use typing::{
    EdgeTypingInfo, EdgeTypingLabelSource, EdgeTypingRole, StageTypingInfo, TypeHintInfo,
};
