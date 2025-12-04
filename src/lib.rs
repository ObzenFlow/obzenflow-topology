//! Flow topology graph structures for ObzenFlow
//!
//! This crate provides graph topology data structures and algorithms for building
//! and validating flow-based pipelines. It's designed to be used both in backend
//! services and frontend applications (including WASM targets).

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
pub use types::{StageId, StageType, StageRole};
pub use topology::{Topology, TopologyMetrics, DirectedEdge, EdgeKind, ValidationLevel};
pub use builder::TopologyBuilder;
pub use validation::{TopologyError, ValidationResult};
pub use stages::{StageInfo, StageMetadata};
