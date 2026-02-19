//! Flow topology graph structures for ObzenFlow
//!
//! This crate provides graph topology data structures and algorithms for building
//! and validating flow-based pipelines. It's designed to be used both in backend
//! services and frontend applications (including WASM targets).

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
pub use types::{SccId, StageId, StageRole, StageType};
pub use validation::{TopologyError, ValidationResult};
