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

// Re-export core types for convenience
pub use types::{StageId, StageType, SimpleStageType};
pub use topology::{Topology, TopologyMetrics, DirectedEdge};
pub use builder::TopologyBuilder;
pub use validation::{TopologyError, ValidationResult};
pub use stages::{StageInfo, StageMetadata};
