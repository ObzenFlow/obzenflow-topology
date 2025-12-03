mod validation;

pub use validation::{
    TopologyError,
    ValidationResult,
    validate_acyclic,
    find_disconnected_stages,
    compute_sccs,
    validate_edges_and_structure,
    validate_all_connections,
    validate_topology_structure,
};
