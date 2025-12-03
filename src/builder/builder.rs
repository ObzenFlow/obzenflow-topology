use crate::stages::{StageId, StageInfo};
use crate::topology::DirectedEdge;
use crate::topology::Topology;
use crate::validation::TopologyError;

/// Builder for constructing pipeline topologies
pub struct TopologyBuilder {
    stages: Vec<StageInfo>,
    edges: Vec<DirectedEdge>,
    current_stage: Option<StageId>,
}

impl TopologyBuilder {
    pub fn new() -> Self {
        Self {
            stages: Vec::new(),
            edges: Vec::new(),
            current_stage: None,
        }
    }

    /// Production API: Add a stage with an explicit StageId
    /// Use this when you have IDs from the application layer
    pub fn add_stage_with_id(
        &mut self,
        id: StageId,
        name: Option<String>,
        stage_type: crate::types::StageType,
    ) -> StageId {
        let info = match name {
            Some(n) => StageInfo::new(id, n, stage_type),
            None => StageInfo::auto_named(id, stage_type),
        };

        self.stages.push(info);

        // If there's a current stage, create an edge from it to this new stage
        if let Some(from) = self.current_stage {
        self.edges.push(DirectedEdge::new(from, id, crate::topology::EdgeKind::Forward));
        }

        self.current_stage = Some(id);
        id
    }

    /// Convenience API: Add a stage with a deterministic generated ID
    /// No RNG required - uses a simple counter for ID generation
    pub fn add_stage(&mut self, name: Option<String>) -> StageId {
        use once_cell::sync::Lazy;
        use std::sync::Mutex;
        
        static COUNTER: Lazy<Mutex<u128>> = Lazy::new(|| Mutex::new(0));
        let mut counter = COUNTER.lock().unwrap();
        *counter += 1;
        
        let id = StageId::from_bytes((*counter).to_be_bytes());
        // Default to a generic transform stage type for test-only APIs
        self.add_stage_with_id(id, name, crate::types::StageType::Transform)
    }


    /// Add an explicit edge between stages
    pub fn add_edge(&mut self, from: StageId, to: StageId) {
        self.edges.push(DirectedEdge::new(from, to, crate::topology::EdgeKind::Forward));
    }

    /// Set the current stage (for chaining)
    pub fn set_current(&mut self, stage: StageId) {
        self.current_stage = Some(stage);
    }

    /// Reset current stage (for building separate chains)
    pub fn reset_current(&mut self) {
        self.current_stage = None;
    }

    /// Build the topology with full validation (structural + semantic + reachability)
    pub fn build(self) -> Result<Topology, TopologyError> {
        Topology::new(self.stages, self.edges)
    }

    /// Build the topology with structural validation only (no semantic checks)
    ///
    /// Use this for UI workflows that need to construct intermediate, invalid graphs
    /// and validate on demand.
    pub fn build_unchecked(self) -> Result<Topology, TopologyError> {
        Topology::new_unvalidated(self.stages, self.edges)
    }
}

impl Default for TopologyBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Tests can use StageId::new() because dev-dependencies have gen feature

    #[test]
    fn test_builder_chaining() {
        let mut builder = TopologyBuilder::new();

        // Build a simple chain using test-only add_stage
        let s1 = builder.add_stage(Some("stage1".to_string()));
        let s2 = builder.add_stage(Some("stage2".to_string()));
        let s3 = builder.add_stage(Some("stage3".to_string()));

        // Use build_unchecked since we're testing structural chaining, not semantic validation
        let topology = builder.build_unchecked().unwrap();

        // Verify chain: s1 -> s2 -> s3
        assert_eq!(topology.downstream_stages(s1), &[s2]);
        assert_eq!(topology.downstream_stages(s2), &[s3]);
        assert_eq!(topology.upstream_stages(s3), &[s2]);
    }

    #[test]
    fn test_builder_fan_in() {
        let mut builder = TopologyBuilder::new();

        // Create two sources using test-only add_stage
        let source1 = builder.add_stage(Some("source1".to_string()));
        builder.reset_current();
        let source2 = builder.add_stage(Some("source2".to_string()));
        builder.reset_current();

        // Create a merger
        let merger = builder.add_stage(Some("merger".to_string()));

        // Connect both sources to merger
        builder.add_edge(source1, merger);
        builder.add_edge(source2, merger);

        // Use build_unchecked since we're testing structural fan-in, not semantic validation
        let topology = builder.build_unchecked().unwrap();

        // Verify fan-in
        let upstream = topology.upstream_stages(merger);
        assert_eq!(upstream.len(), 2);
        assert!(upstream.contains(&source1));
        assert!(upstream.contains(&source2));
    }
}
