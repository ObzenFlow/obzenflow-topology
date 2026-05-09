// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Flow topology graph structures for ObzenFlow
//!
//! This crate provides graph topology data structures and algorithms for building
//! and validating flow-based pipelines. It's designed to be used both in backend
//! services and frontend applications (including WASM targets).
//!
//! As of 0.4 (FLOWIP-114b), `Topology`, `StageInfo`, and `DirectedEdge` carry
//! optional annotation fields (status, role, cycle membership, join metadata,
//! middleware, contracts, stage typing, edge typing, subgraph membership)
//! alongside the structural graph. Validation, SCC computation, and traversal
//! continue to read only the structural fields.

#![allow(clippy::module_inception)]
#![allow(clippy::result_large_err)]

pub mod builder;
pub mod stages;
pub mod topology;
pub mod types;
pub mod validation;

// Test utilities - internal only, not exposed in public API
#[cfg_attr(not(test), allow(dead_code))] // silence warnings in non-test builds
pub(crate) mod test_ids;

// Re-export for unit tests only
#[cfg(test)]
pub use test_ids::next_stage_id;

// Re-export core types for convenience
pub use builder::TopologyBuilder;
pub use stages::StageInfo;
#[allow(deprecated)]
pub use stages::StageMetadata;
pub use topology::{DirectedEdge, EdgeKind, Topology, TopologyMetrics, ValidationLevel};
pub use types::{
    BackoffStrategy, CircuitBreakerInfo, ContractInfo, EdgeTypingInfo, EdgeTypingLabelSource,
    EdgeTypingRole, JoinMetadataInfo, MiddlewareInfo, OpenPolicy, RateLimiterInfo, RetryInfo,
    SccId, StageId, StageRole, StageStatus, StageSubgraphMembership, StageType, StageTypingInfo,
    SubgraphInternalEdge, TopologySubgraphInfo, TypeHintInfo,
};
pub use validation::{TopologyError, ValidationResult};
