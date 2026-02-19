use crate::stages::{StageId, StageInfo};
use crate::topology::DirectedEdge;
use crate::types::StageRole;
use crate::validation::{
    compute_sccs, validate_all_connections, validate_edges_and_structure,
    validate_topology_structure, TopologyError, ValidationResult,
};
use std::collections::{HashMap, HashSet};

/// Complete topology with efficient traversal
///
/// As of FLOWIP-082, topologies support cycles to enable feedback loops,
/// retry patterns, and iterative processing via the `<|` operator
#[derive(Debug, Clone)]
pub struct Topology {
    stages: HashMap<StageId, StageInfo>,
    edges: Vec<DirectedEdge>,

    // Cached adjacency lists for O(1) lookups
    // Using HashSet for O(1) contains() checks
    downstream: HashMap<StageId, HashSet<StageId>>,
    upstream: HashMap<StageId, HashSet<StageId>>,

    // Stages that are part of cycles (computed from SCCs)
    stages_in_cycles: HashSet<StageId>,

    // Strongly connected components (only SCCs with len > 1 are stored)
    scc_members: Vec<HashSet<StageId>>,  // indexed by scc_id
    stage_to_scc: HashMap<StageId, u32>, // stage -> scc_id
}

/// Validation level for topology semantics
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValidationLevel {
    /// Structural only: endpoints, duplicates, self-cycles, disconnected, SCCs
    Structural,
    /// Structural + StageType/StageRole + EdgeKind connection semantics
    Semantic,
    /// Semantic + structural invariants (NoSources, NoSinks, UnreachableStages, UnproductiveStages)
    Full,
}

impl Topology {
    /// Construct topology with structural validation only.
    ///
    /// "Unvalidated" means: not semantically or reachability validated.
    /// Structural invariants (valid endpoints, no duplicates, no self-cycles, no disconnected components)
    /// still hold so core APIs can rely on consistent graph structure.
    pub fn new_unvalidated(
        stages: Vec<StageInfo>,
        edges: Vec<DirectedEdge>,
    ) -> ValidationResult<Self> {
        let stage_map: HashMap<StageId, StageInfo> =
            stages.into_iter().map(|s| (s.id, s)).collect();

        // Run structural validation on edges
        validate_edges_and_structure(&stage_map, &edges)?;

        // Build adjacency lists for efficient traversal
        let mut downstream: HashMap<StageId, HashSet<StageId>> = HashMap::new();
        let mut upstream: HashMap<StageId, HashSet<StageId>> = HashMap::new();

        for edge in &edges {
            downstream.entry(edge.from).or_default().insert(edge.to);
            upstream.entry(edge.to).or_default().insert(edge.from);
        }

        // Compute SCCs for cycle detection (FLOWIP-082g)
        let mut scc_members: Vec<HashSet<StageId>> = compute_sccs(&stage_map, &downstream)
            .into_iter()
            .filter(|scc| scc.len() > 1)
            .collect();

        // Stabilise SCC identifiers by sorting by minimum stage id in each SCC.
        scc_members.sort_by_key(|scc| scc.iter().copied().min().expect("non-empty SCC"));

        let mut stage_to_scc = HashMap::new();
        for (scc_index, scc) in scc_members.iter().enumerate() {
            let scc_id = u32::try_from(scc_index).expect("SCC index exceeds u32::MAX");
            for stage_id in scc {
                stage_to_scc.insert(*stage_id, scc_id);
            }
        }
        let stages_in_cycles = stage_to_scc.keys().copied().collect();

        Ok(Self {
            stages: stage_map,
            edges,
            downstream,
            upstream,
            stages_in_cycles,
            scc_members,
            stage_to_scc,
        })
    }

    /// Construct and fully validate topology (structural + semantic + structural invariants).
    pub fn new(stages: Vec<StageInfo>, edges: Vec<DirectedEdge>) -> Result<Self, TopologyError> {
        let topo = Self::new_unvalidated(stages, edges)?;
        topo.validate_with_level(ValidationLevel::Full)?;
        Ok(topo)
    }

    /// Validate this topology at the requested level.
    pub fn validate_with_level(&self, level: ValidationLevel) -> Result<(), TopologyError> {
        match level {
            ValidationLevel::Structural => {
                // Reuse existing structure for validation
                validate_edges_and_structure(&self.stages, &self.edges)
            }
            ValidationLevel::Semantic => {
                validate_edges_and_structure(&self.stages, &self.edges)?;
                validate_all_connections(&self.stages, &self.edges)
            }
            ValidationLevel::Full => {
                self.validate_with_level(ValidationLevel::Semantic)?;
                validate_topology_structure(&self.stages, &self.downstream)
            }
        }
    }

    /// Convenience method: run full semantic validation.
    pub fn validate_semantics(&self) -> Result<(), TopologyError> {
        self.validate_with_level(ValidationLevel::Full)
    }

    /// Get stages that flow INTO this stage
    pub fn upstream_stages(&self, stage: StageId) -> Vec<StageId> {
        self.upstream
            .get(&stage)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get stages that this stage flows TO
    pub fn downstream_stages(&self, stage: StageId) -> Vec<StageId> {
        self.downstream
            .get(&stage)
            .map(|set| set.iter().copied().collect())
            .unwrap_or_default()
    }

    /// Get human-readable name for debugging
    pub fn stage_name(&self, stage: StageId) -> Option<&str> {
        self.stages.get(&stage).map(|info| info.name.as_str())
    }

    /// Get stage info
    pub fn stage_info(&self, stage: StageId) -> Option<&StageInfo> {
        self.stages.get(&stage)
    }

    /// Get all stages
    pub fn stages(&self) -> impl Iterator<Item = &StageInfo> {
        self.stages.values()
    }

    /// Get all edges
    pub fn edges(&self) -> &[DirectedEdge] {
        &self.edges
    }

    /// Check if topology has any stages
    pub fn is_empty(&self) -> bool {
        self.stages.is_empty()
    }

    /// Get number of stages
    pub fn num_stages(&self) -> usize {
        self.stages.len()
    }

    /// Find source stages (no upstream)
    pub fn source_stages(&self) -> Vec<StageId> {
        self.stages
            .keys()
            .filter(|&id| self.upstream_stages(*id).is_empty())
            .copied()
            .collect()
    }

    /// Find sink stages (no downstream)
    pub fn sink_stages(&self) -> Vec<StageId> {
        self.stages
            .keys()
            .filter(|&id| self.downstream_stages(*id).is_empty())
            .copied()
            .collect()
    }

    /// Semantic source stages (Producer role based on StageType)
    pub fn semantic_source_stages(&self) -> Vec<StageId> {
        self.stages
            .iter()
            .filter(|(_, info)| matches!(info.stage_type.role(), StageRole::Producer))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Semantic sink stages (Consumer role based on StageType)
    pub fn semantic_sink_stages(&self) -> Vec<StageId> {
        self.stages
            .iter()
            .filter(|(_, info)| matches!(info.stage_type.role(), StageRole::Consumer))
            .map(|(id, _)| *id)
            .collect()
    }

    /// Get flow name (derived from source stage if single source)
    pub fn flow_name(&self) -> String {
        let sources = self.source_stages();
        if sources.len() == 1 {
            if let Some(stage_info) = self.stages.get(&sources[0]) {
                return format!("{}_flow", stage_info.name);
            }
        }
        "multi_source_flow".to_string()
    }

    /// Get topology fingerprint - deterministic hash of structure and semantics
    ///
    /// Includes: stage IDs, names, types, edge endpoints, and edge kinds.
    /// Same topology always produces same fingerprint.
    pub fn topology_fingerprint(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();

        // 1) Canonicalize stages (sort by ULID bytes for determinism)
        let mut stages: Vec<_> = self.stages.iter().collect();
        stages.sort_unstable_by_key(|(id, _)| id.to_bytes());

        for (id, info) in stages {
            id.to_bytes().hash(&mut hasher);
            info.name.hash(&mut hasher);
            info.stage_type.as_str().hash(&mut hasher);
        }

        // 2) Canonicalize edges (sort for determinism)
        let mut edges = self.edges.clone();
        edges.sort_unstable_by_key(|e| (e.from.to_bytes(), e.to.to_bytes()));

        for edge in &edges {
            edge.from.to_bytes().hash(&mut hasher);
            edge.to.to_bytes().hash(&mut hasher);
            std::mem::discriminant(&edge.kind).hash(&mut hasher);
        }

        hasher.finish()
    }

    /// Get source stage name (assumes single source)
    pub fn source_stage_name(&self) -> String {
        let sources = self.source_stages();
        if sources.len() == 1 {
            if let Some(stage_info) = self.stages.get(&sources[0]) {
                return stage_info.name.clone();
            }
        }
        "unknown_source".to_string()
    }

    /// Get sink stage name (assumes single sink)
    pub fn sink_stage_name(&self) -> String {
        let sinks = self.sink_stages();
        if sinks.len() == 1 {
            if let Some(stage_info) = self.stages.get(&sinks[0]) {
                return stage_info.name.clone();
            }
        }
        "unknown_sink".to_string()
    }

    /// Get topology metrics for debugging and optimization
    pub fn metrics(&self) -> TopologyMetrics {
        TopologyMetrics {
            num_stages: self.stages.len(),
            num_edges: self.edges.len(),
            num_sources: self.source_stages().len(),
            num_sinks: self.sink_stages().len(),
            max_fan_out: self
                .downstream
                .values()
                .map(|set| set.len())
                .max()
                .unwrap_or(0),
            max_fan_in: self
                .upstream
                .values()
                .map(|set| set.len())
                .max()
                .unwrap_or(0),
            max_depth: self.calculate_max_depth(),
        }
    }

    /// Calculate the maximum depth (longest path) in the topology
    /// For graphs with cycles, this returns the longest acyclic path
    fn calculate_max_depth(&self) -> usize {
        let mut depths: HashMap<StageId, usize> = HashMap::new();
        let sources = self.source_stages();

        // BFS from all sources
        let mut queue = std::collections::VecDeque::new();
        for source in sources {
            queue.push_back((source, 0));
            depths.insert(source, 0);
        }

        let mut max_depth = 0;
        while let Some((stage, depth)) = queue.pop_front() {
            max_depth = max_depth.max(depth);

            for downstream in self.downstream_stages(stage) {
                let new_depth = depth + 1;
                let should_update = depths
                    .get(&downstream)
                    .map(|&d| new_depth > d)
                    .unwrap_or(true);

                if should_update {
                    depths.insert(downstream, new_depth);
                    queue.push_back((downstream, new_depth));
                }
            }
        }

        max_depth
    }

    /// Check if a specific edge exists
    pub fn has_edge(&self, from: StageId, to: StageId) -> bool {
        self.downstream
            .get(&from)
            .map(|set| set.contains(&to))
            .unwrap_or(false)
    }

    /// Check if a stage is part of a cycle (has a path back to itself)
    pub fn is_in_cycle(&self, stage_id: StageId) -> bool {
        // Use cached SCC information (FLOWIP-082g)
        self.stages_in_cycles.contains(&stage_id)
    }

    /// Returns the SCC identifier for this stage, or None if it is not in any SCC.
    pub fn scc_id(&self, stage_id: StageId) -> Option<u32> {
        self.stage_to_scc.get(&stage_id).copied()
    }

    /// Returns the set of stages that belong to the given SCC identifier.
    pub fn scc_members(&self, scc_id: u32) -> Option<&HashSet<StageId>> {
        self.scc_members.get(scc_id as usize)
    }
}

/// Topology metrics for debugging and optimization
#[derive(Debug, Clone)]
pub struct TopologyMetrics {
    pub num_stages: usize,
    pub num_edges: usize,
    pub num_sources: usize,
    pub num_sinks: usize,
    pub max_fan_out: usize,
    pub max_fan_in: usize,
    pub max_depth: usize,
}

#[cfg(test)]
mod tests {
    use crate::builder::TopologyBuilder;

    #[test]
    fn test_simple_pipeline() {
        let mut builder = TopologyBuilder::new();
        let source = builder.add_stage(Some("source".to_string()));
        let transform = builder.add_stage(Some("transform".to_string()));
        let sink = builder.add_stage(Some("sink".to_string()));

        // Use build_unchecked since we're testing structural traversal, not semantic validation
        let topology = builder.build_unchecked().unwrap();

        assert_eq!(topology.num_stages(), 3);
        assert_eq!(topology.upstream_stages(transform), &[source]);
        assert_eq!(topology.downstream_stages(transform), &[sink]);
        assert_eq!(topology.source_stages(), vec![source]);
        assert_eq!(topology.sink_stages(), vec![sink]);
    }

    #[test]
    fn test_fan_out_topology() {
        let mut builder = TopologyBuilder::new();
        let source = builder.add_stage(Some("source".to_string()));

        // Reset to build parallel branches
        builder.reset_current();
        let transform1 = builder.add_stage(Some("transform1".to_string()));
        let transform2 = builder.add_stage(Some("transform2".to_string()));

        // Manually connect source to both transforms
        builder.add_edge(source, transform1);
        builder.add_edge(source, transform2);

        // Use build_unchecked since we're testing structural fan-out, not semantic validation
        let topology = builder.build_unchecked().unwrap();

        assert_eq!(topology.downstream_stages(source).len(), 2);
        assert!(topology.downstream_stages(source).contains(&transform1));
        assert!(topology.downstream_stages(source).contains(&transform2));
    }

    #[test]
    fn test_self_cycle_rejected() {
        use crate::stages::StageInfo;
        use crate::topology::{DirectedEdge, EdgeKind};
        use crate::validation::TopologyError;

        let stage_id = crate::test_ids::next_stage_id();
        let stage = StageInfo::new(stage_id, "processor", crate::types::StageType::Transform);
        let stages = vec![stage.clone()];

        // Create self-cycle edge
        let edges = vec![DirectedEdge::new(stage.id, stage.id, EdgeKind::Forward)];

        // Use new_unvalidated since we're testing structural self-cycle rejection
        let result = super::Topology::new_unvalidated(stages, edges);
        assert!(result.is_err());

        match result {
            Err(TopologyError::SelfCycle { stage: name }) => {
                assert_eq!(name, "processor");
            }
            _ => panic!("Expected SelfCycle error"),
        }
    }

    #[test]
    fn test_multi_stage_cycle_allowed() {
        use crate::stages::StageInfo;
        use crate::topology::{DirectedEdge, EdgeKind};

        let validator_id = crate::test_ids::next_stage_id();
        let fixer_id = crate::test_ids::next_stage_id();
        let validator = StageInfo::new(
            validator_id,
            "validator",
            crate::types::StageType::Transform,
        );
        let fixer = StageInfo::new(fixer_id, "fixer", crate::types::StageType::Transform);
        let stages = vec![validator.clone(), fixer.clone()];

        // Create a cycle between two different stages (allowed structurally)
        let edges = vec![
            DirectedEdge::new(validator.id, fixer.id, EdgeKind::Forward),
            DirectedEdge::new(fixer.id, validator.id, EdgeKind::Backward),
        ];

        // Use new_unvalidated since we're testing structural cycle allowance
        let result = super::Topology::new_unvalidated(stages, edges);
        assert!(result.is_ok());

        let topology = result.unwrap();
        assert!(topology.has_edge(validator.id, fixer.id));
        assert!(topology.has_edge(fixer.id, validator.id));

        // Test SCC-based cycle detection (FLOWIP-082g)
        assert!(topology.is_in_cycle(validator.id));
        assert!(topology.is_in_cycle(fixer.id));
    }

    #[test]
    fn test_scc_cycle_detection() {
        use crate::stages::StageInfo;
        use crate::topology::{DirectedEdge, EdgeKind};

        // Create a more complex topology: A -> B -> C -> D
        //                                       ^         |
        //                                       +---------+
        let a_id = crate::test_ids::next_stage_id();
        let b_id = crate::test_ids::next_stage_id();
        let c_id = crate::test_ids::next_stage_id();
        let d_id = crate::test_ids::next_stage_id();

        let a = StageInfo::new(a_id, "a", crate::types::StageType::Transform);
        let b = StageInfo::new(b_id, "b", crate::types::StageType::Transform);
        let c = StageInfo::new(c_id, "c", crate::types::StageType::Transform);
        let d = StageInfo::new(d_id, "d", crate::types::StageType::Transform);

        let stages = vec![a.clone(), b.clone(), c.clone(), d.clone()];

        let edges = vec![
            DirectedEdge::new(a.id, b.id, EdgeKind::Forward),
            DirectedEdge::new(b.id, c.id, EdgeKind::Forward),
            DirectedEdge::new(c.id, d.id, EdgeKind::Forward),
            DirectedEdge::new(d.id, b.id, EdgeKind::Backward), // Creates cycle B->C->D->B
        ];

        // Use new_unvalidated since we're testing structural SCC detection
        let result = super::Topology::new_unvalidated(stages, edges);
        assert!(result.is_ok());

        let topology = result.unwrap();

        // A is not in a cycle
        assert!(!topology.is_in_cycle(a.id));
        assert_eq!(topology.scc_id(a.id), None);

        // B, C, D are all in the same cycle
        assert!(topology.is_in_cycle(b.id));
        assert!(topology.is_in_cycle(c.id));
        assert!(topology.is_in_cycle(d.id));

        let scc_id = topology.scc_id(b.id).expect("b should have scc_id");
        assert_eq!(topology.scc_id(c.id), Some(scc_id));
        assert_eq!(topology.scc_id(d.id), Some(scc_id));

        let members = topology
            .scc_members(scc_id)
            .expect("scc_members should exist");
        assert_eq!(members.len(), 3);
        assert!(members.contains(&b.id));
        assert!(members.contains(&c.id));
        assert!(members.contains(&d.id));
    }

    #[test]
    fn test_two_disjoint_sccs_get_distinct_ids() {
        use crate::stages::StageInfo;
        use crate::topology::{DirectedEdge, EdgeKind};

        // Topology: src -> A <-> B -> C <-> D -> snk
        //
        // Two disjoint SCCs: {A, B} and {C, D}.
        let src_id = crate::test_ids::next_stage_id();
        let a_id = crate::test_ids::next_stage_id();
        let b_id = crate::test_ids::next_stage_id();
        let c_id = crate::test_ids::next_stage_id();
        let d_id = crate::test_ids::next_stage_id();
        let snk_id = crate::test_ids::next_stage_id();

        let src = StageInfo::new(src_id, "src", crate::types::StageType::FiniteSource);
        let a = StageInfo::new(a_id, "a", crate::types::StageType::Transform);
        let b = StageInfo::new(b_id, "b", crate::types::StageType::Transform);
        let c = StageInfo::new(c_id, "c", crate::types::StageType::Transform);
        let d = StageInfo::new(d_id, "d", crate::types::StageType::Transform);
        let snk = StageInfo::new(snk_id, "snk", crate::types::StageType::Sink);

        let stages = vec![
            src.clone(),
            a.clone(),
            b.clone(),
            c.clone(),
            d.clone(),
            snk.clone(),
        ];

        let edges = vec![
            DirectedEdge::new(src.id, a.id, EdgeKind::Forward),
            DirectedEdge::new(a.id, b.id, EdgeKind::Forward),
            DirectedEdge::new(b.id, a.id, EdgeKind::Backward), // SCC 1: {A, B}
            DirectedEdge::new(b.id, c.id, EdgeKind::Forward),
            DirectedEdge::new(c.id, d.id, EdgeKind::Forward),
            DirectedEdge::new(d.id, c.id, EdgeKind::Backward), // SCC 2: {C, D}
            DirectedEdge::new(d.id, snk.id, EdgeKind::Forward),
        ];

        let topology = super::Topology::new_unvalidated(stages, edges).unwrap();

        // Non-cycle stages return None.
        assert_eq!(topology.scc_id(src.id), None);
        assert_eq!(topology.scc_id(snk.id), None);

        // Both SCCs have valid, distinct identifiers.
        let scc_ab = topology.scc_id(a.id).expect("a should be in an SCC");
        let scc_cd = topology.scc_id(c.id).expect("c should be in an SCC");
        assert_ne!(scc_ab, scc_cd, "disjoint SCCs must have distinct ids");

        // Members within each SCC share the same id.
        assert_eq!(topology.scc_id(b.id), Some(scc_ab));
        assert_eq!(topology.scc_id(d.id), Some(scc_cd));

        // Member sets are correct and do not bleed.
        let members_ab = topology.scc_members(scc_ab).unwrap();
        assert_eq!(members_ab.len(), 2);
        assert!(members_ab.contains(&a.id));
        assert!(members_ab.contains(&b.id));
        assert!(!members_ab.contains(&c.id));

        let members_cd = topology.scc_members(scc_cd).unwrap();
        assert_eq!(members_cd.len(), 2);
        assert!(members_cd.contains(&c.id));
        assert!(members_cd.contains(&d.id));
        assert!(!members_cd.contains(&a.id));
    }

    #[test]
    fn test_minimal_two_stage_scc() {
        use crate::stages::StageInfo;
        use crate::topology::{DirectedEdge, EdgeKind};

        let x_id = crate::test_ids::next_stage_id();
        let y_id = crate::test_ids::next_stage_id();

        let x = StageInfo::new(x_id, "x", crate::types::StageType::Transform);
        let y = StageInfo::new(y_id, "y", crate::types::StageType::Transform);

        let edges = vec![
            DirectedEdge::new(x.id, y.id, EdgeKind::Forward),
            DirectedEdge::new(y.id, x.id, EdgeKind::Backward),
        ];

        let topology = super::Topology::new_unvalidated(vec![x.clone(), y.clone()], edges).unwrap();

        let scc_id = topology.scc_id(x.id).expect("x should be in SCC");
        assert_eq!(topology.scc_id(y.id), Some(scc_id));

        let members = topology.scc_members(scc_id).unwrap();
        assert_eq!(members.len(), 2);
        assert!(members.contains(&x.id));
        assert!(members.contains(&y.id));
    }

    #[test]
    fn test_dag_has_no_sccs() {
        // Pure DAG: src -> a -> b -> snk. No cycles.
        let topology = {
            let mut builder = TopologyBuilder::new();
            let _src = builder.add_stage(Some("src".to_string()));
            let _a = builder.add_stage(Some("a".to_string()));
            let _b = builder.add_stage(Some("b".to_string()));
            let _snk = builder.add_stage(Some("snk".to_string()));
            builder.build_unchecked().unwrap()
        };

        for stage in topology.stages() {
            assert!(
                !topology.is_in_cycle(stage.id),
                "stage {:?} should not be in a cycle",
                stage.name
            );
            assert_eq!(
                topology.scc_id(stage.id),
                None,
                "stage {:?} should have no scc_id",
                stage.name
            );
        }

        // No SCCs exist, so even index 0 should return None.
        assert_eq!(topology.scc_members(0), None);
    }

    #[test]
    fn test_scc_id_is_deterministic_across_constructions() {
        use crate::stages::StageInfo;
        use crate::topology::{DirectedEdge, EdgeKind};

        // Build the same topology twice with the same stage IDs and
        // verify scc_id assignments are identical.
        let a_id = crate::test_ids::next_stage_id();
        let b_id = crate::test_ids::next_stage_id();
        let c_id = crate::test_ids::next_stage_id();

        let build = || {
            let a = StageInfo::new(a_id, "a", crate::types::StageType::Transform);
            let b = StageInfo::new(b_id, "b", crate::types::StageType::Transform);
            let c = StageInfo::new(c_id, "c", crate::types::StageType::Transform);

            let edges = vec![
                DirectedEdge::new(a.id, b.id, EdgeKind::Forward),
                DirectedEdge::new(b.id, c.id, EdgeKind::Forward),
                DirectedEdge::new(c.id, a.id, EdgeKind::Backward),
            ];
            super::Topology::new_unvalidated(vec![a, b, c], edges).unwrap()
        };

        let t1 = build();
        let t2 = build();

        assert_eq!(t1.scc_id(a_id), t2.scc_id(a_id));
        assert_eq!(t1.scc_id(b_id), t2.scc_id(b_id));
        assert_eq!(t1.scc_id(c_id), t2.scc_id(c_id));
    }

    #[test]
    fn test_scc_members_out_of_bounds_returns_none() {
        let topology = {
            let mut builder = TopologyBuilder::new();
            let _src = builder.add_stage(Some("src".to_string()));
            let _snk = builder.add_stage(Some("snk".to_string()));
            builder.build_unchecked().unwrap()
        };

        assert_eq!(topology.scc_members(0), None);
        assert_eq!(topology.scc_members(999), None);
        assert_eq!(topology.scc_members(u32::MAX), None);
    }

    #[test]
    fn test_is_in_cycle_agrees_with_scc_id() {
        use crate::stages::StageInfo;
        use crate::topology::{DirectedEdge, EdgeKind};

        // Topology with both cycle and non-cycle stages:
        // src -> entry -> iter -> snk
        //          ^        |
        //          +--------+
        let src_id = crate::test_ids::next_stage_id();
        let entry_id = crate::test_ids::next_stage_id();
        let iter_id = crate::test_ids::next_stage_id();
        let snk_id = crate::test_ids::next_stage_id();

        let src = StageInfo::new(src_id, "src", crate::types::StageType::FiniteSource);
        let entry = StageInfo::new(entry_id, "entry", crate::types::StageType::Transform);
        let iter = StageInfo::new(iter_id, "iter", crate::types::StageType::Transform);
        let snk = StageInfo::new(snk_id, "snk", crate::types::StageType::Sink);

        let stages = vec![src.clone(), entry.clone(), iter.clone(), snk.clone()];
        let edges = vec![
            DirectedEdge::new(src.id, entry.id, EdgeKind::Forward),
            DirectedEdge::new(entry.id, iter.id, EdgeKind::Forward),
            DirectedEdge::new(iter.id, entry.id, EdgeKind::Backward),
            DirectedEdge::new(entry.id, snk.id, EdgeKind::Forward),
        ];

        let topology = super::Topology::new_unvalidated(stages, edges).unwrap();

        // The invariant: is_in_cycle and scc_id must always agree.
        for stage in topology.stages() {
            assert_eq!(
                topology.is_in_cycle(stage.id),
                topology.scc_id(stage.id).is_some(),
                "is_in_cycle and scc_id disagree for stage {:?}",
                stage.name
            );
        }

        // Confirm the expected classification.
        assert!(!topology.is_in_cycle(src.id));
        assert!(topology.is_in_cycle(entry.id));
        assert!(topology.is_in_cycle(iter.id));
        assert!(!topology.is_in_cycle(snk.id));
    }
}
