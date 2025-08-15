mod validation;

pub use validation::{TopologyError, ValidationResult, validate_acyclic, find_disconnected_stages, compute_sccs};