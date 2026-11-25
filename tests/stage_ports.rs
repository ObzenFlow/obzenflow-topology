// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use obzenflow_topology::{StageId, Topology, TopologyError};
use serde_json::{json, Value};

fn document() -> Value {
    json!({
        "stages": [{
            "id": "00000000000000000000000001", "name": "authorize_payment", "stage_type": "transform",
            "ports": [{ "relation":"asks", "port":"PaymentGatewayPort", "system":"simulated-processor",
                "requests":[{"type":"payment.authorize","guarantee":"exactly_once",
                    "outcome":{"kind":"domain_facts","types":["payment.authorized","payment.declined"]}}] }]
        }, { "id": "00000000000000000000000002", "name": "paid", "stage_type": "sink" }],
        "edges": [{ "from":"00000000000000000000000001", "to":"00000000000000000000000002", "kind":"forward" }],
        "subgraphs": []
    })
}

#[test]
fn ports_payment_roundtrips() {
    let doc = document();
    let topology: Topology = serde_json::from_value(doc.clone()).unwrap();
    let port = &topology
        .stage_info(StageId::from_bytes(1_u128.to_be_bytes()))
        .unwrap()
        .ports[0];
    assert_eq!(port.system_label(), port.system);
    assert_eq!(serde_json::to_value(topology).unwrap(), doc);
}

#[test]
fn connection_descriptions_roundtrip() {
    for address in ["localhost:11434", "[::1]:8080/v1", "db.internal"] {
        let mut doc = document();
        let port = &mut doc["stages"][0]["ports"][0];
        port["display_name"] = json!("Payment gateway");
        port["transport"] = json!("HTTP");
        port["address"] = json!(address);
        port["resource"] = json!("model");
        let topology: Topology = serde_json::from_value(doc.clone()).unwrap();
        assert_eq!(
            topology
                .stage_info(StageId::from_bytes(1_u128.to_be_bytes()))
                .unwrap()
                .ports[0]
                .system_label(),
            "Payment gateway"
        );
        assert_eq!(serde_json::to_value(topology).unwrap(), doc);
    }
}

#[test]
fn addresses_private_components_refused() {
    for address in [
        "https://db.internal",
        "user:pass@db",
        "api/v1?token=x",
        "api#key",
        "[::1]:99999",
        "api\nother",
    ] {
        let mut doc = document();
        doc["stages"][0]["ports"][0]["address"] = json!(address);
        assert!(serde_json::from_value::<Topology>(doc).is_err());
    }
}

#[test]
fn ports_legacy_documents_roundtrip() {
    let mut doc = document();
    doc["stages"][0].as_object_mut().unwrap().remove("ports");
    let topology: Topology = serde_json::from_value(doc.clone()).unwrap();
    assert!(topology.stages().next().unwrap().ports.is_empty());
    assert_eq!(serde_json::to_value(topology).unwrap(), doc);
}

#[test]
fn ports_invalid_replacement_preserves_binding() {
    let mut topology: Topology = serde_json::from_value(document()).unwrap();
    let id = StageId::from_bytes(1_u128.to_be_bytes());
    let mut stage = topology.stage_info(id).unwrap().clone();
    stage.ports[0].system = "invalid name".into();
    assert!(matches!(
        topology.replace_stage_info(stage),
        Err(TopologyError::InvalidStagePorts { .. })
    ));
    assert_eq!(
        topology.stage_info(id).unwrap().ports[0].system,
        "simulated-processor"
    );
}

#[test]
fn requests_duplicate_types_refused() {
    let mut doc = document();
    let mut second = doc["stages"][0]["ports"][0].clone();
    second["port"] = json!("AnotherPort");
    doc["stages"][0]["ports"]
        .as_array_mut()
        .unwrap()
        .push(second);
    assert!(serde_json::from_value::<Topology>(doc).is_err());
}

#[test]
fn delivery_guarantees_roundtrip() {
    for guarantee in ["at_least_once", "at_most_once", "exactly_once"] {
        let mut doc = document();
        doc["stages"][1]["ports"] = json!([{
            "relation": "delivers", "port": "DigestOutputPort", "system": "hn-digest-postgres",
            "requests": [{"type": "hn.digest.publish", "method": "publish", "guarantee": guarantee,
                "outcome": {"kind": "recorded_reply"}}]
        }]);
        let topology: Topology = serde_json::from_value(doc.clone()).unwrap();
        assert_eq!(serde_json::to_value(topology).unwrap(), doc);
    }
}

#[test]
fn delivery_domain_facts_refused() {
    let mut doc = document();
    let mut port = doc["stages"][0]["ports"][0].clone();
    port["relation"] = json!("delivers");
    doc["stages"][1]["ports"] = json!([port]);
    assert!(serde_json::from_value::<Topology>(doc).is_err());
}

#[test]
fn source_effect_requests_refused() {
    for relation in ["reads", "receives"] {
        let mut doc = document();
        doc["stages"][0]["ports"][0]["relation"] = json!(relation);
        assert!(serde_json::from_value::<Topology>(doc).is_err());
    }
}
