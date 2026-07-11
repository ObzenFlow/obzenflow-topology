// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! FLOWIP-128a D4/D9 manifest compatibility contract.
//!
//! The subgraph family evolves additively: a 0.4-shape document deserializes
//! with defaults, a value with no annotations serializes byte-identical to
//! the 0.4 shape, and unknown future fields are tolerated.

use obzenflow_topology::{
    BoundaryPortSpec, PortDirection, StageId, StageSubgraphMembership, SubgraphInternalEdge,
    TopologySubgraphInfo,
};

fn stage_id(n: u128) -> StageId {
    StageId::from_bytes(n.to_be_bytes())
}

fn membership_04_json() -> serde_json::Value {
    serde_json::json!({
        "subgraph_id": "ai_map_reduce:digest",
        "kind": "ai_map_reduce",
        "binding": "digest",
        "role": "chunk",
        "order": 0,
        "is_entry": true,
        "is_exit": false
    })
}

fn info_04_json(id: StageId) -> serde_json::Value {
    serde_json::json!({
        "subgraph_id": "ai_map_reduce:digest",
        "kind": "ai_map_reduce",
        "binding": "digest",
        "label": "digest",
        "member_stage_ids": [id],
        "internal_edges": [{
            "from_stage_id": id,
            "to_stage_id": id,
            "role": "manifest"
        }],
        "entry_stage_ids": [id],
        "exit_stage_ids": [id],
        "collapsible": true
    })
}

#[test]
fn zero_four_shape_deserializes_with_defaults() {
    let membership: StageSubgraphMembership =
        serde_json::from_value(membership_04_json()).expect("0.4 membership deserializes");
    assert_eq!(membership.role, "chunk");
    assert_eq!(membership.class, None);

    let id = stage_id(1);
    let info: TopologySubgraphInfo =
        serde_json::from_value(info_04_json(id)).expect("0.4 info deserializes");
    assert_eq!(info.schema_version, 1);
    assert!(info.boundary_ports.is_empty());
    assert_eq!(info.parent_subgraph_id, None);
}

#[test]
fn annotation_free_values_serialize_byte_identical_to_zero_four_shape() {
    let membership = StageSubgraphMembership::new(
        "ai_map_reduce:digest",
        "ai_map_reduce",
        "digest",
        "chunk",
        0,
        true,
        false,
    );
    assert_eq!(
        serde_json::to_value(&membership).expect("serializes"),
        membership_04_json()
    );

    let id = stage_id(1);
    let info = TopologySubgraphInfo::new(
        "ai_map_reduce:digest",
        "ai_map_reduce",
        "digest",
        "digest",
        vec![id],
        vec![SubgraphInternalEdge::new(id, id, "manifest")],
        vec![id],
        vec![id],
        true,
    );
    assert_eq!(
        serde_json::to_value(&info).expect("serializes"),
        info_04_json(id)
    );
}

#[test]
fn unknown_future_fields_are_tolerated() {
    let mut value = membership_04_json();
    value["future_field"] = serde_json::json!({"anything": 1});
    let membership: StageSubgraphMembership =
        serde_json::from_value(value).expect("unknown fields tolerated");
    assert_eq!(membership.role, "chunk");

    let id = stage_id(1);
    let mut value = info_04_json(id);
    value["future_registry_field"] = serde_json::json!([1, 2, 3]);
    value["schema_version"] = serde_json::json!(7);
    let info: TopologySubgraphInfo =
        serde_json::from_value(value).expect("unknown fields tolerated");
    assert_eq!(info.schema_version, 7);
}

#[test]
fn annotated_manifest_shape_snapshot() {
    let id = stage_id(3);
    let info = TopologySubgraphInfo::new(
        "saga:checkout",
        "saga",
        "checkout",
        "checkout",
        vec![id],
        vec![SubgraphInternalEdge::new(id, id, "terminal")],
        vec![id],
        vec![id],
        true,
    )
    .with_boundary_ports(vec![
        BoundaryPortSpec::new(
            "in",
            PortDirection::Input,
            id,
            vec!["order.validated.v1".into()],
            true,
        ),
        BoundaryPortSpec::new(
            "declined",
            PortDirection::Output,
            id,
            vec!["payment.declined.v1".into()],
            false,
        ),
    ]);

    let membership = StageSubgraphMembership::new(
        "saga:checkout",
        "saga",
        "checkout",
        "authorize_payment",
        0,
        true,
        false,
    )
    .with_class("compensatable");

    insta::assert_snapshot!(
        "annotated_subgraph_info",
        serde_json::to_string_pretty(&info).expect("info serializes")
    );
    insta::assert_snapshot!(
        "annotated_membership",
        serde_json::to_string_pretty(&membership).expect("membership serializes")
    );
}
