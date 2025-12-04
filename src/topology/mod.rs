mod edge;
mod topology;
mod shape;

pub use edge::{DirectedEdge, EdgeKind, EdgeExtensions};
pub use shape::*;
pub use topology::{Topology, TopologyMetrics, ValidationLevel};
