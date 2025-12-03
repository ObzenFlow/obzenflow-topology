//! Stage shapes define connectivity patterns in the topology

use crate::stages::StageId;
use crate::types::StageType;
use core::fmt;

/// Port identifier - composite of stage ID and local port number
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct PortId {
    pub stage: StageId,
    pub port: usize,  // 0, 1, 2... within that stage
}

impl PortId {
    pub fn new(stage: StageId, port: usize) -> Self {
        Self { stage, port }
    }
}

impl fmt::Display for PortId {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}:port{}", self.stage, self.port)
    }
}

/// Shape defines the connectivity of a stage
#[derive(Debug, Clone)]
pub enum Shape {
    /// No inlets, one outlet (generates events)
    Source { out: PortId },
    
    /// One inlet, one outlet (transforms events)
    Flow { in_port: PortId, out: PortId },
    
    /// One inlet, no outlets (consumes events)
    Sink { in_port: PortId },
    
    /// One inlet, multiple outlets (splits stream)
    Broadcast { in_port: PortId, outs: Vec<PortId> },
    
    /// Multiple inlets, one outlet (merges streams)
    Merge { ins: Vec<PortId>, out: PortId },
}

impl Shape {
    /// Get all inlet ports
    pub fn inlets(&self) -> Vec<&PortId> {
        match self {
            Shape::Source { .. } => vec![],
            Shape::Flow { in_port, .. } => vec![in_port],
            Shape::Sink { in_port } => vec![in_port],
            Shape::Broadcast { in_port, .. } => vec![in_port],
            Shape::Merge { ins, .. } => ins.iter().collect(),
        }
    }
    
    /// Get all outlet ports
    pub fn outlets(&self) -> Vec<&PortId> {
        match self {
            Shape::Source { out } => vec![out],
            Shape::Flow { out, .. } => vec![out],
            Shape::Sink { .. } => vec![],
            Shape::Broadcast { outs, .. } => outs.iter().collect(),
            Shape::Merge { out, .. } => vec![out],
        }
    }
    
    /// Create a source shape for the given stage
    pub fn new_source(stage: StageId) -> Self {
        Shape::Source { out: PortId::new(stage, 0) }
    }
    
    /// Create a flow shape for the given stage
    pub fn new_flow(stage: StageId) -> Self {
        Shape::Flow {
            in_port: PortId::new(stage, 0),
            out: PortId::new(stage, 1),
        }
    }
    
    /// Create a sink shape for the given stage
    pub fn new_sink(stage: StageId) -> Self {
        Shape::Sink { in_port: PortId::new(stage, 0) }
    }
    
    /// Simple classification for backwards compatibility
    pub fn stage_type(&self) -> StageType {
        match self {
            Shape::Source { .. } => StageType::InfiniteSource,
            Shape::Sink { .. } => StageType::Sink,
            _ => StageType::Transform,
        }
    }
}
