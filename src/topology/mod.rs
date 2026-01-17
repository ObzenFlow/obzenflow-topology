mod edge;
mod shape;
mod topology;

pub use edge::{DirectedEdge, EdgeExtensions, EdgeKind};
pub use shape::*;
pub use topology::{Topology, TopologyMetrics, ValidationLevel};
