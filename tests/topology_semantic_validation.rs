use obzenflow_topology::{
    DirectedEdge, EdgeKind, StageInfo, StageType, Topology, TopologyError,
};

mod common;
use common::next_stage_id;

/// Helper to build a StageInfo with given type
fn mk_stage(name: &str, stage_type: StageType) -> StageInfo {
    let id = next_stage_id();
    StageInfo::new(id, name.to_string(), stage_type)
}

#[test]
fn test_semantic_valid_source_processor_sink() {
    // FiniteSource -> Transform -> Sink should be fully valid
    let source = mk_stage("source", StageType::FiniteSource);
    let proc = mk_stage("proc", StageType::Transform);
    let sink = mk_stage("sink", StageType::Sink);

    let stages = vec![source.clone(), proc.clone(), sink.clone()];
    let edges = vec![
        DirectedEdge::new(source.id, proc.id, EdgeKind::Forward),
        DirectedEdge::new(proc.id, sink.id, EdgeKind::Forward),
    ];

    let topo = Topology::new(stages, edges).expect("valid source->proc->sink topology");

    // Semantic sources/sinks should match roles
    assert_eq!(topo.semantic_source_stages(), vec![source.id]);
    assert_eq!(topo.semantic_sink_stages(), vec![sink.id]);
}

#[test]
fn test_semantic_sink_to_source_forward_rejected() {
    // Sink |> Source should be rejected by semantic validation
    let sink = mk_stage("sink", StageType::Sink);
    let source = mk_stage("source", StageType::FiniteSource);

    let stages = vec![sink.clone(), source.clone()];
    let edges = vec![DirectedEdge::new(
        sink.id,
        source.id,
        EdgeKind::Forward,
    )];

    match Topology::new(stages, edges) {
        Err(TopologyError::InvalidConnection { operator, from_name, to_name, .. }) => {
            assert_eq!(operator, "|>");
            assert_eq!(from_name, "sink");
            assert_eq!(to_name, "source");
        }
        Ok(_) => panic!("Expected InvalidConnection error for sink |> source"),
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[test]
fn test_semantic_no_sources_error() {
    // Transform -> Sink with no Producer should trigger NoSources
    let proc = mk_stage("proc", StageType::Transform);
    let sink = mk_stage("sink", StageType::Sink);

    let stages = vec![proc.clone(), sink.clone()];
    let edges = vec![DirectedEdge::new(
        proc.id,
        sink.id,
        EdgeKind::Forward,
    )];

    match Topology::new(stages, edges) {
        Err(TopologyError::NoSources) => {}
        Ok(_) => panic!("Expected NoSources error"),
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[test]
fn test_semantic_no_sinks_error() {
    // Source -> Transform with no Consumer should trigger NoSinks
    let source = mk_stage("source", StageType::FiniteSource);
    let proc = mk_stage("proc", StageType::Transform);

    let stages = vec![source.clone(), proc.clone()];
    let edges = vec![DirectedEdge::new(
        source.id,
        proc.id,
        EdgeKind::Forward,
    )];

    match Topology::new(stages, edges) {
        Err(TopologyError::NoSinks) => {}
        Ok(_) => panic!("Expected NoSinks error"),
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[test]
fn test_semantic_unreachable_stages_error() {
    // Component 1: Source1 -> Proc1 (valid)
    // Component 2: Proc2 -> Sink2 (disconnected from any Producer)
    let source1 = mk_stage("source1", StageType::FiniteSource);
    let proc1 = mk_stage("proc1", StageType::Transform);
    let proc2 = mk_stage("proc2", StageType::Transform);
    let sink2 = mk_stage("sink2", StageType::Sink);

    let stages = vec![source1.clone(), proc1.clone(), proc2.clone(), sink2.clone()];
    let edges = vec![
        DirectedEdge::new(source1.id, proc1.id, EdgeKind::Forward),
        DirectedEdge::new(proc2.id, sink2.id, EdgeKind::Forward),
    ];

    match Topology::new(stages, edges) {
        Err(TopologyError::UnreachableStages { stages }) => {
            // Both proc2 and sink2 are unreachable from any Producer
            assert!(stages.contains(&proc2.id));
            assert!(stages.contains(&sink2.id));
        }
        Ok(_) => panic!("Expected UnreachableStages error"),
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

#[test]
fn test_semantic_unproductive_stages_error() {
    // Source -> Proc1 (-> Sink), and Source -> Proc2 (no path to any Sink)
    let source = mk_stage("source", StageType::FiniteSource);
    let proc1 = mk_stage("proc1", StageType::Transform);
    let proc2 = mk_stage("proc2", StageType::Transform);
    let sink = mk_stage("sink", StageType::Sink);

    let stages = vec![source.clone(), proc1.clone(), proc2.clone(), sink.clone()];
    let edges = vec![
        DirectedEdge::new(source.id, proc1.id, EdgeKind::Forward),
        DirectedEdge::new(proc1.id, sink.id, EdgeKind::Forward),
        DirectedEdge::new(source.id, proc2.id, EdgeKind::Forward),
    ];

    match Topology::new(stages, edges) {
        Err(TopologyError::UnproductiveStages { stages }) => {
            // proc2 cannot reach any Consumer
            assert!(stages.contains(&proc2.id));
            // Source may also be considered unproductive if it can't reach a sink via that branch,
            // but we only assert proc2 is included.
        }
        Ok(_) => panic!("Expected UnproductiveStages error"),
        Err(e) => panic!("Unexpected error: {}", e),
    }
}

