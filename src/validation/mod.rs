mod validation;

pub use validation::{
    compute_sccs, find_disconnected_stages, validate_acyclic, validate_all_connections,
    validate_edges_and_structure, validate_topology_structure, TopologyError, ValidationResult,
};
