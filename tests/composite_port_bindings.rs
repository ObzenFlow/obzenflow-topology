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

    let input_refs = if annotate_input {
        vec![CompositePortRef::new(composite, "in")]
    } else {
        Vec::new()
    };
    let output_refs = if annotate_output {
        vec![CompositePortRef::new(composite, "out")]
    } else {
        Vec::new()
    };

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

    let wire = serde_json::to_value(&topology).unwrap();
    let error = serde_json::from_value::<Topology>(wire)
        .expect_err("partial 0.5.1 bindings must not masquerade as a legacy graph");
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

#[test]
fn canonical_json_pins_multi_output_and_composite_to_composite_cut_refs() {
    let source = stage_id(20);
    let branch_entry = stage_id(21);
    let completed = stage_id(22);
    let failed = stage_id(23);
    let audit_entry = stage_id(24);
    let audit_exit = stage_id(25);
    let sink = stage_id(26);

    let branch = TopologySubgraphInfo::new(
        "saga:checkout",
        "saga",
        "checkout",
        "checkout",
        vec![branch_entry, completed, failed],
        vec![],
        vec![branch_entry],
        vec![completed, failed],
        true,
    )
    .with_boundary_ports(vec![
        BoundaryPortSpec::new(
            "commands",
            PortDirection::Input,
            branch_entry,
            vec!["checkout.command.v1".into()],
            true,
        ),
        BoundaryPortSpec::new(
            "completed",
            PortDirection::Output,
            completed,
            vec!["checkout.completed.v1".into()],
            true,
        ),
        BoundaryPortSpec::new(
            "failed",
            PortDirection::Output,
            failed,
            vec!["checkout.failed.v1".into()],
            false,
        ),
    ]);
    let audit = TopologySubgraphInfo::new(
        "audit:orders",
        "audit",
        "orders",
        "orders",
        vec![audit_entry, audit_exit],
        vec![],
        vec![audit_entry],
        vec![audit_exit],
        true,
    )
    .with_boundary_ports(vec![
        BoundaryPortSpec::new(
            "in",
            PortDirection::Input,
            audit_entry,
            vec!["checkout.completed.v1".into()],
            true,
        ),
        BoundaryPortSpec::new(
            "out",
            PortDirection::Output,
            audit_exit,
            vec!["audit.recorded.v1".into()],
            true,
        ),
    ]);

    let crossing = DirectedEdge::new(completed, audit_entry, EdgeKind::Forward)
        .with_composite_ports(vec![
            CompositePortRef::new("saga:checkout", "completed"),
            CompositePortRef::new("audit:orders", "in"),
        ]);
    let topology = Topology::new_unvalidated(
        vec![
            StageInfo::new(source, "source", StageType::FiniteSource),
            StageInfo::new(branch_entry, "branch_entry", StageType::Transform),
            StageInfo::new(completed, "completed", StageType::Transform),
            StageInfo::new(failed, "failed", StageType::Transform),
            StageInfo::new(audit_entry, "audit_entry", StageType::Transform),
            StageInfo::new(audit_exit, "audit_exit", StageType::Transform),
            StageInfo::new(sink, "sink", StageType::Sink),
        ],
        vec![
            DirectedEdge::new(source, branch_entry, EdgeKind::Forward)
                .with_composite_ports(vec![CompositePortRef::new("saga:checkout", "commands")]),
            DirectedEdge::new(branch_entry, completed, EdgeKind::Forward),
            DirectedEdge::new(branch_entry, failed, EdgeKind::Forward),
            crossing,
            DirectedEdge::new(failed, sink, EdgeKind::Forward)
                .with_composite_ports(vec![CompositePortRef::new("saga:checkout", "failed")]),
            DirectedEdge::new(audit_entry, audit_exit, EdgeKind::Forward),
            DirectedEdge::new(audit_exit, sink, EdgeKind::Forward)
                .with_composite_ports(vec![CompositePortRef::new("audit:orders", "out")]),
        ],
    )
    .unwrap()
    .with_subgraphs(vec![branch, audit]);
    topology
        .validate_composite_boundaries()
        .expect("named multi-output and composite-to-composite cuts validate");

    let mut wire = serde_json::to_value(&topology).unwrap();
    let crossing = wire["edges"]
        .as_array()
        .unwrap()
        .iter()
        .find(|edge| {
            edge["from"] == serde_json::to_value(completed).unwrap()
                && edge["to"] == serde_json::to_value(audit_entry).unwrap()
        })
        .unwrap();
    assert_eq!(
        crossing["composite_ports"],
        serde_json::json!([
            {"subgraph_id": "saga:checkout", "port_name": "completed"},
            {"subgraph_id": "audit:orders", "port_name": "in"}
        ])
    );
    let payloads: Vec<_> = wire["subgraphs"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|subgraph| subgraph["boundary_ports"].as_array().unwrap())
        .flat_map(|port| port["payload_event_types"].as_array().unwrap())
        .filter_map(serde_json::Value::as_str)
        .collect();
    assert!(payloads.contains(&"checkout.completed.v1"));
    assert!(!payloads.iter().any(|payload| payload.contains(".v1.v1")));

    for edge in wire["edges"].as_array_mut().unwrap() {
        edge.as_object_mut().unwrap().remove("composite_ports");
    }
    let legacy: Topology =
        serde_json::from_value(wire).expect("complete 0.5.0 absence remains deserializable");
    let legacy_crossing = legacy
        .edges()
        .iter()
        .find(|edge| edge.from == completed && edge.to == audit_entry)
        .unwrap();
    assert!(legacy_crossing.composite_ports.is_empty());
    assert!(legacy.validate_composite_boundaries().is_err());
}
