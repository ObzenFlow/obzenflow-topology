// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use obzenflow_topology::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const RATE: &str = "middleware.rate_limiter.events_per_second";

#[test]
fn backpressure_plan_round_trips_and_rejects_invalid_settings() {
    for wire in [
        json!({"mode":"track"}),
        json!({"mode":"enforce", "window":64, "stall_timeout_ms":30000}),
    ] {
        let info: BackpressureInfo = serde_json::from_value(wire.clone()).unwrap();
        let mut edge = DirectedEdge::new(
            StageId::from_bytes([1; 16]),
            StageId::from_bytes([2; 16]),
            EdgeKind::Forward,
        );
        edge.backpressure = Some(info);
        let encoded = serde_json::to_value(&edge).unwrap();
        assert_eq!(encoded["backpressure"], wire);
        let decoded: DirectedEdge = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.backpressure, Some(info));
    }
    for invalid in [
        json!({"mode":"track", "window":64}),
        json!({"mode":"enforce", "window":0, "stall_timeout_ms":30000}),
        json!({"mode":"enforce", "window":64, "stall_timeout_ms":0}),
        json!({"mode":"enforce", "window":"64", "stall_timeout_ms":30000}),
        json!({"mode":"enforce", "window":64}),
        json!({"mode":"unknown"}),
    ] {
        assert!(
            serde_json::from_value::<BackpressureInfo>(invalid.clone()).is_err(),
            "{invalid}"
        );
    }
}

fn provenance() -> SettingProvenanceInfo {
    SettingProvenanceInfo::try_new(
        "profile".into(),
        "future_scope:blue".into(),
        SettingSubject::Unqualified,
    )
    .unwrap()
}

fn active(value: SettingValueInfo, display: &str) -> MiddlewareSettingInfo {
    MiddlewareSettingInfo::Active {
        setting: ResolvedSettingInfo {
            value,
            provenance: provenance(),
        },
        display: display.into(),
    }
}

fn row(state: &str, value: Value, display: &str) -> Value {
    json!({"state":state,"value":value,"provenance":provenance(),"display":display})
}

fn limiter_wire() -> Value {
    let rate = row("active", json!({"kind":"f64","value":10.0}), "10 events/s");
    let cost = row("active", json!({"kind":"f64","value":1.0}), "1 tokens");
    json!({"kind":"rate_limiter","info":{"summary":"10","settings":{
        RATE:rate,
        "middleware.rate_limiter.cost_per_attempt":cost,
        "middleware.rate_limiter.burst_capacity":{"state":"automatic","display":"Automatic"}
    }}})
}

fn breaker_wire() -> Value {
    let count = |state: &str, value: u64, display: &str| {
        row(state, json!({"kind":"u64","value":value}), display)
    };
    let mode = row(
        "active",
        json!({"kind":"text","value":"consecutive"}),
        "Consecutive failures",
    );
    json!({"kind":"circuit_breaker","info":{"summary":"3 consecutive failures","settings":{
        "middleware.circuit_breaker.mode":mode,
        "middleware.circuit_breaker.consecutive_failures":count("active", 3, "3"),
        "middleware.circuit_breaker.count_window":count("inactive", 10, "10 calls"),
        "middleware.circuit_breaker.minimum_calls":count("inactive", 11, "11 calls")
    }}})
}

fn retry_wire() -> Value {
    json!({"kind":"retry","info":{"settings":{
        "middleware.retry.kind":row("active", json!({"kind":"text","value":"fixed"}), "Fixed"),
        "middleware.retry.on_refusal":row("active", json!({"kind":"text","value":"wait"}), "Wait")
    }}})
}

fn limiter() -> MiddlewareDetailsInfo {
    serde_json::from_value(limiter_wire()).unwrap()
}

fn attachment(id: u128, details: MiddlewareDetailsInfo) -> MiddlewareAttachmentInfo {
    MiddlewareAttachmentInfo {
        key: MiddlewareAttachmentKey::from_bytes(id.to_be_bytes()),
        label: "rate_limiter".into(), // deliberately collides with observer/custom labels
        authored_site: MiddlewareAuthoredSite::Effect {
            effect_type: "payments.authorize".into(),
        },
        operation: MiddlewareOperation::Effect {
            effect_type: "payments.authorize".into(),
        },
        details,
    }
}

#[test]
fn every_family_preserves_all_binding_information_and_provenance() {
    for wire in [limiter_wire(), breaker_wire(), retry_wire()] {
        let details: MiddlewareDetailsInfo = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(details).unwrap(), wire);
    }
    let extension = MiddlewareSettingsInfo::try_new(
        BTreeMap::from([
            (
                "extension.enabled".into(),
                active(SettingValueInfo::Bool(true), "true"),
            ),
            (
                "extension.count".into(),
                active(SettingValueInfo::U64(u64::MAX), "18446744073709551615"),
            ),
            (
                "extension.ratio".into(),
                active(SettingValueInfo::F64(-1.25), "-1.25"),
            ),
            (
                "extension.label".into(),
                active(SettingValueInfo::Text("opaque".into()), "opaque"),
            ),
            (
                "extension.secret".into(),
                active(SettingValueInfo::Redacted, "Redacted"),
            ),
        ]),
        None,
    )
    .unwrap();
    let details = vec![
        limiter(),
        serde_json::from_value(breaker_wire()).unwrap(),
        serde_json::from_value(retry_wire()).unwrap(),
        MiddlewareDetailsInfo::Observer(extension.clone()),
        MiddlewareDetailsInfo::Custom {
            name: "custom-family".into(),
            info: extension,
        },
    ];
    let mut attachments: Vec<_> = details
        .into_iter()
        .enumerate()
        .map(|(i, d)| attachment(i as u128, d))
        .collect();
    attachments[3].authored_site = MiddlewareAuthoredSite::Implementation;
    attachments[3].operation = MiddlewareOperation::SinkDelivery;
    let original = MiddlewareInfo::try_new(attachments).unwrap();
    let wire = serde_json::to_string(&original).unwrap();
    let back: MiddlewareInfo = serde_json::from_str(&wire).unwrap();
    assert_eq!(back, original);
    assert_eq!(back.attachments[3].family(), MiddlewareFamily::Observer);
    assert_eq!(
        back.attachments[4].family(),
        MiddlewareFamily::Custom {
            name: "custom-family".into()
        }
    );
    assert!(!wire.contains("***redacted***")); // redaction has a typed marker, not a magic text value
}

#[test]
fn nested_provenance_is_required_checked_and_open_axes_round_trip() {
    let original = serde_json::to_value(limiter()).unwrap();
    for path in ["source", "scope", "winner_subject"] {
        let mut malformed = original.clone();
        malformed["info"]["settings"][RATE]["provenance"]
            .as_object_mut()
            .unwrap()
            .remove(path);
        assert!(serde_json::from_value::<MiddlewareDetailsInfo>(malformed).is_err());
    }
    for replacement in [
        json!({"source":"","scope":"global","winner_subject":{"kind":"unqualified"}}),
        json!({"source":"dsl","scope":" ","winner_subject":{"kind":"unqualified"}}),
        json!({"source":"dsl","scope":"global","winner_subject":{"kind":"effect","effect_type":""}}),
        json!({"source":"dsl","scope":"global","winner_subject":{"kind":"unqualified","oops":1}}),
        json!({"source":"dsl","scope":"global","winner_subject":{"kind":"unqualified"},"oops":1}),
    ] {
        let mut malformed = original.clone();
        malformed["info"]["settings"][RATE]["provenance"] = replacement;
        assert!(serde_json::from_value::<MiddlewareDetailsInfo>(malformed).is_err());
    }
    let mut exact = original;
    exact["info"]["settings"][RATE]["provenance"]["winner_subject"] =
        json!({"kind":"effect","effect_type":"payments.authorize"});
    let back: MiddlewareDetailsInfo = serde_json::from_value(exact.clone()).unwrap();
    assert_eq!(serde_json::to_value(back).unwrap(), exact);
}

#[test]
fn malformed_rows_refused() {
    let count = json!({"kind":"u64","value":5});
    let breaker =
        |key: &str, row: Value| json!({"kind":"circuit_breaker","info":{"settings":{key:row}}});
    let probes = "middleware.circuit_breaker.probes";
    for wire in [
        json!({"kind":"observer","info":{"settings":{" ":row("active", count.clone(), "5")}}}),
        breaker(
            "middleware.retry.max_attempts",
            row("active", count.clone(), "5"),
        ),
        breaker(
            "middleware.circuit_breaker.",
            row("active", count.clone(), "5"),
        ),
        breaker(
            probes,
            json!({"state":"active","provenance":provenance(),"display":"5"}),
        ),
        breaker(
            probes,
            json!({"state":"inactive","value":count,"display":"5"}),
        ),
        breaker(probes, row("automatic", count.clone(), "Automatic")),
        breaker(probes, row("active", count.clone(), " ")),
        breaker(probes, json!({"state":"automatic","display":null})),
        breaker(
            probes,
            json!({
                "state":"active","value":count,"provenance":provenance(),
                "display":"5","unit":"calls"
            }),
        ),
        breaker(probes, json!({"state":"disabled","display":"5"})),
        json!({"kind":"circuit_breaker","info":{"summary":"","settings":{}}}),
        json!({"kind":"circuit_breaker","info":{"summary":null,"settings":{}}}),
    ] {
        assert!(
            serde_json::from_value::<MiddlewareDetailsInfo>(wire.clone()).is_err(),
            "{wire}"
        );
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        let rows = BTreeMap::from([("x".into(), active(SettingValueInfo::F64(value), "x"))]);
        assert!(MiddlewareSettingsInfo::try_new(rows, None).is_err());
    }
}

#[test]
fn extension_shapes_have_no_arbitrary_json_or_unknown_envelope_fields() {
    for value in [
        json!(null),
        json!({"kind":"array","value":[]}),
        json!({"kind":"text","value":{}}),
    ] {
        let wire =
            json!({"kind":"observer","info":{"settings":{"custom.x":row("active", value, "x")}}});
        assert!(serde_json::from_value::<MiddlewareDetailsInfo>(wire).is_err());
    }
    for wire in [
        json!({"kind":"observer","info":{"settings":{},"arbitrary":1}}),
        json!({"kind":"future","info":{"settings":{}}}),
        json!({"kind":"custom","info":{"name":"","info":{"settings":{}}}}),
        json!({"kind":"observer","info":{"settings":{}},"family":"retry"}),
    ] {
        assert!(serde_json::from_value::<MiddlewareDetailsInfo>(wire).is_err());
    }
    assert!(serde_json::from_value::<MiddlewareDetailsInfo>(
        json!({"kind":"observer","info":{"settings":{}}})
    )
    .is_ok());
}

#[test]
fn keys_preserve_full_bytes_and_only_accept_canonical_wire_spelling() {
    for bytes in [
        [0; 16],
        [255; 16],
        0x0123456789abcdef0011223344556677_u128.to_be_bytes(),
    ] {
        let key = MiddlewareAttachmentKey::from_bytes(bytes);
        let text = key.to_string();
        assert_eq!(text.len(), 26);
        assert_eq!(
            text.parse::<MiddlewareAttachmentKey>().unwrap().to_bytes(),
            bytes
        );
        assert_eq!(
            serde_json::from_value::<MiddlewareAttachmentKey>(json!(text)).unwrap(),
            key
        );
    }
    for text in [
        "",
        "0",
        "8ZZZZZZZZZZZZZZZZZZZZZZZZZ",
        "7zzzzzzzzzzzzzzzzzzzzzzzzz",
        "0000000000000000000000000I",
        "0000000000000000000000000L",
        "0000000000000000000000000O",
        "0000000000000000000000000U",
        " 00000000000000000000000000",
        "00000000000000000000000000\n",
        "stage:effect:rate_limiter",
    ] {
        assert!(text.parse::<MiddlewareAttachmentKey>().is_err(), "{text:?}");
        assert!(
            serde_json::from_value::<MiddlewareAttachmentKey>(json!(text)).is_err(),
            "{text:?}"
        );
    }
}

#[test]
fn duplicate_binding_keys_reject_before_topology_indexes_and_remain_flow_scoped() {
    let member = attachment(1, limiter());
    assert!(MiddlewareInfo::try_new(vec![member.clone(), member.clone()]).is_err());
    let duplicate = json!({"attachments":[member.clone(),member.clone()]});
    assert!(serde_json::from_value::<MiddlewareInfo>(duplicate).is_err());
    let stage = |id| {
        StageInfo::new(
            StageId::from_bytes(u128::to_be_bytes(id)),
            format!("s{id}"),
            StageType::FiniteSource,
        )
        .with_middleware(MiddlewareInfo::try_new(vec![member.clone()]).unwrap())
    };
    let build = |mut sources: Vec<StageInfo>| {
        let sink = StageInfo::new(
            StageId::from_bytes(99_u128.to_be_bytes()),
            "sink",
            StageType::Sink,
        );
        let edges = sources
            .iter()
            .map(|source| DirectedEdge::new(source.id, sink.id, EdgeKind::Forward))
            .collect();
        sources.push(sink);
        Topology::new_unvalidated(sources, edges).map_err(|error| error.to_string())
    };
    assert!(build(vec![stage(1)]).is_ok());
    assert!(build(vec![stage(2)]).is_ok());
    assert!(build(vec![stage(1), stage(2)])
        .unwrap_err()
        .to_string()
        .contains("duplicate binding key"));
}

#[test]
fn stage_replacement_rejects_invalid_information_atomically() {
    let source_id = StageId::from_bytes(1_u128.to_be_bytes());
    let sink_id = StageId::from_bytes(2_u128.to_be_bytes());
    let source = StageInfo::new(source_id, "source", StageType::FiniteSource)
        .with_middleware(MiddlewareInfo::try_new(vec![attachment(1, limiter())]).unwrap());
    let sink = StageInfo::new(sink_id, "sink", StageType::Sink)
        .with_middleware(MiddlewareInfo::try_new(vec![attachment(2, limiter())]).unwrap());
    let mut topology = Topology::new_unvalidated(
        vec![source.clone(), sink],
        vec![DirectedEdge::new(source_id, sink_id, EdgeKind::Forward)],
    )
    .unwrap();
    let before = serde_json::to_value(&topology).unwrap();
    let invalid_memberships = [
        // Public attachment vectors must not bypass the checked boundary.
        MiddlewareInfo {
            attachments: vec![attachment(3, limiter()), attachment(3, limiter())],
        },
        // A valid membership can still collide with a different stage.
        MiddlewareInfo::try_new(vec![attachment(2, limiter())]).unwrap(),
        MiddlewareInfo {
            attachments: vec![attachment(
                3,
                MiddlewareDetailsInfo::Custom {
                    name: "".into(),
                    info: MiddlewareSettingsInfo::default(),
                },
            )],
        },
    ];
    for middleware in invalid_memberships {
        let replacement = source.clone().with_middleware(middleware);
        assert!(matches!(
            topology.replace_stage_info(replacement),
            Err(TopologyError::InvalidMiddlewareInfo { .. })
        ));
        assert_eq!(serde_json::to_value(&topology).unwrap(), before);
    }

    // Reusing this stage's own key is legal; the previous record is returned.
    let replacement = source.clone().with_status(StageStatus::Running);
    let previous = topology.replace_stage_info(replacement).unwrap().unwrap();
    assert_eq!(
        serde_json::to_value(previous).unwrap(),
        serde_json::to_value(source).unwrap()
    );
    assert_eq!(
        topology.stage_info(source_id).unwrap().status,
        Some(StageStatus::Running)
    );
    topology
        .validate_with_level(ValidationLevel::Structural)
        .unwrap();
}
