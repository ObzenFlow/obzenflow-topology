// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use obzenflow_topology::{
    BoundaryPortSpec, CompositePortRef, DirectedEdge, EdgeKind, PortDirection, StageId, StageInfo,
    StageType, Topology, TopologySubgraphInfo, ValidationLevel,
};

fn stage_id(n: u128) -> StageId {
    StageId::from_bytes(n.to_be_bytes())
}

fn fixture(
    annotate_input: bool,
    annotate_output: bool,
) -> (Topology, StageId, StageId, StageId, StageId) {
    let source = stage_id(1);
    let entry = stage_id(2);
    let exit = stage_id(3);
    let sink = stage_id(4);
    let composite = "test:branch";

    let input_refs = annotate_input
        .then(|| vec![CompositePortRef::new(composite, "in")])
        .unwrap_or_default();
    let output_refs = annotate_output
        .then(|| vec![CompositePortRef::new(composite, "out")])
        .unwrap_or_default();

    let topology = Topology::new_unvalidated(
        vec![
            StageInfo::new(source, "source", StageType::FiniteSource),
            StageInfo::new(entry, "entry", StageType::Transform),
            StageInfo::new(exit, "exit", StageType::Transform),
            StageInfo::new(sink, "sink", StageType::Sink),
        ],
        vec![
            DirectedEdge::new(source, entry, EdgeKind::Forward).with_composite_ports(input_refs),
            DirectedEdge::new(entry, exit, EdgeKind::Forward),
            DirectedEdge::new(exit, sink, EdgeKind::Forward).with_composite_ports(output_refs),
        ],
    )
    .expect("structural topology")
    .with_subgraphs(vec![TopologySubgraphInfo::new(
        composite,
        "test",
        "branch",
        "branch",
        vec![entry, exit],
        vec![],
        vec![entry],
        vec![exit],
        true,
    )
    .with_boundary_ports(vec![
        BoundaryPortSpec::new(
            "in",
            PortDirection::Input,
            entry,
            vec!["test.input.v1".into()],
            true,
        ),
        BoundaryPortSpec::new(
            "out",
            PortDirection::Output,
            exit,
            vec!["test.output.v1".into()],
            true,
        ),
    ])]);

    (topology, source, entry, exit, sink)
}

#[test]
fn named_cut_bindings_validate() {
    let (topology, ..) = fixture(true, true);
    topology
        .validate_with_level(ValidationLevel::Structural)
        .expect("every crossing edge has the correct named port");
}

#[test]
fn crossing_edge_without_binding_fails_loud() {
    let (topology, ..) = fixture(true, false);
    let error = topology
        .validate_with_level(ValidationLevel::Structural)
        .expect_err("unbound output cut must fail");
    assert!(error.to_string().contains("has no named port binding"));
}

#[test]
fn internal_edge_cannot_claim_a_boundary_port() {
    let (topology, source, entry, exit, sink) = fixture(true, true);
    let subgraphs = topology.subgraphs().to_vec();
    let malformed = Topology::new_unvalidated(
        vec![
            StageInfo::new(source, "source", StageType::FiniteSource),
            StageInfo::new(entry, "entry", StageType::Transform),
            StageInfo::new(exit, "exit", StageType::Transform),
            StageInfo::new(sink, "sink", StageType::Sink),
        ],
        vec![
            DirectedEdge::new(source, entry, EdgeKind::Forward)
                .with_composite_ports(vec![CompositePortRef::new("test:branch", "in")]),
            DirectedEdge::new(entry, exit, EdgeKind::Forward)
                .with_composite_ports(vec![CompositePortRef::new("test:branch", "out")]),
            DirectedEdge::new(exit, sink, EdgeKind::Forward)
                .with_composite_ports(vec![CompositePortRef::new("test:branch", "out")]),
        ],
    )
    .expect("structural topology")
    .with_subgraphs(subgraphs);

    let error = malformed
        .validate_composite_boundaries()
        .expect_err("internal annotation must fail");
    assert!(error.to_string().contains("does not cross output port"));
}

#[test]
fn overlapping_payload_ownership_fails_loud() {
    let (topology, source, entry, exit, sink) = fixture(true, true);
    let mut subgraph = topology.subgraphs()[0].clone();
    subgraph.boundary_ports.push(BoundaryPortSpec::new(
        "also_out",
        PortDirection::Output,
        exit,
        vec!["test.output.v1".into()],
        false,
    ));
    let malformed = Topology::new_unvalidated(
        vec![
            StageInfo::new(source, "source", StageType::FiniteSource),
            StageInfo::new(entry, "entry", StageType::Transform),
            StageInfo::new(exit, "exit", StageType::Transform),
            StageInfo::new(sink, "sink", StageType::Sink),
        ],
        topology.edges().to_vec(),
    )
    .expect("structural topology")
    .with_subgraphs(vec![subgraph]);

    let error = malformed
        .validate_composite_boundaries()
        .expect_err("one payload cannot have two same-direction owners");
    assert!(error.to_string().contains("owned by both output ports"));
}

#[test]
fn absent_binding_annotation_preserves_old_edge_json() {
    let edge = DirectedEdge::new(stage_id(10), stage_id(11), EdgeKind::Forward);
    assert_eq!(
        serde_json::to_value(edge).expect("edge serializes"),
        serde_json::json!({
            "from": stage_id(10),
            "to": stage_id(11),
            "kind": "forward"
        })
    );
}
