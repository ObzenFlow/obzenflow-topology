// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use obzenflow_topology::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

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

fn setting<T>(value: T) -> ResolvedSettingInfo<T> {
    ResolvedSettingInfo {
        value,
        provenance: provenance(),
    }
}

fn row(value: Value) -> Value {
    json!({"value":value,"provenance":provenance()})
}

fn limiter() -> MiddlewareDetailsInfo {
    MiddlewareDetailsInfo::RateLimiter(
        RateLimiterInfo::try_new(setting(10.0), setting(1.0), None).unwrap(),
    )
}

fn breaker_wire(mode: &str) -> Value {
    json!({"kind":"circuit_breaker","info":{
        "mode":row(json!(mode)),"open_for_ms":row(json!(60000)),"probes":row(json!(1)),
        "rate_limited_counts_as_failure":row(json!(false)),
        "consecutive_failures":row(json!(3)),"count_window":row(json!(10)),
        "minimum_calls":row(json!(5)),"failure_rate_threshold":row(json!(0.6)),
        "slow_call_duration_ms":row(json!(250)),"slow_call_rate_threshold":row(json!(0.5))
    }})
}

fn retry_wire(kind: &str) -> Value {
    json!({"kind":"retry","info":{
        "kind":row(json!(kind)),"max_attempts":row(json!(3)),
        "max_backoff_ms":row(json!(30000)),"attempt_start_window_ms":row(json!(120000)),
        "fixed_delay_ms":row(json!(250))
    }})
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
    let extension = MiddlewareExtensionInfo::try_new(BTreeMap::from([
        (
            "extension.enabled".into(),
            setting(SettingValueInfo::Bool(true)),
        ),
        (
            "extension.count".into(),
            setting(SettingValueInfo::U64(u64::MAX)),
        ),
        (
            "extension.ratio".into(),
            setting(SettingValueInfo::F64(-1.25)),
        ),
        (
            "extension.label".into(),
            setting(SettingValueInfo::Text("opaque".into())),
        ),
        (
            "extension.secret".into(),
            setting(SettingValueInfo::Redacted),
        ),
    ]))
    .unwrap();
    let details = vec![
        limiter(),
        serde_json::from_value(breaker_wire("rate_based")).unwrap(),
        serde_json::from_value(retry_wire("fixed")).unwrap(),
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
    // The operation remains an effect even where the winner was an unqualified broadcast.
    if let MiddlewareDetailsInfo::RateLimiter(info) = &attachments[0].details {
        assert_eq!(
            info.events_per_second().provenance.winner_subject,
            SettingSubject::Unqualified
        );
    }
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
fn omission_is_automatic_but_explicit_null_and_bad_known_fields_are_errors() {
    let valid = serde_json::to_value(limiter()).unwrap();
    assert!(valid["info"].get("burst_capacity").is_none());
    assert!(serde_json::from_value::<MiddlewareDetailsInfo>(valid.clone()).is_ok());
    for (key, value) in [
        ("burst_capacity", Value::Null),
        ("events_per_second", row(json!("10"))),
        ("events_per_second", row(json!(0.0))),
        ("events_per_second", row(json!(-1.0))),
        ("cost_per_attempt", row(json!(0.0))),
        ("burst_capacity", row(json!(0.5))),
        ("events_per_second_typo", row(json!(10.0))),
    ] {
        let mut malformed = valid.clone();
        malformed["info"][key] = value;
        assert!(
            serde_json::from_value::<MiddlewareDetailsInfo>(malformed).is_err(),
            "{key}"
        );
    }
    let mut missing = valid;
    missing["info"]
        .as_object_mut()
        .unwrap()
        .remove("cost_per_attempt");
    assert!(serde_json::from_value::<MiddlewareDetailsInfo>(missing).is_err());
}

#[test]
fn nested_provenance_is_required_checked_and_open_axes_round_trip() {
    let original = serde_json::to_value(limiter()).unwrap();
    for path in ["source", "scope", "winner_subject"] {
        let mut malformed = original.clone();
        malformed["info"]["events_per_second"]["provenance"]
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
        malformed["info"]["events_per_second"]["provenance"] = replacement;
        assert!(serde_json::from_value::<MiddlewareDetailsInfo>(malformed).is_err());
    }
    let mut exact = original;
    exact["info"]["events_per_second"]["provenance"]["winner_subject"] =
        json!({"kind":"effect","effect_type":"payments.authorize"});
    let back: MiddlewareDetailsInfo = serde_json::from_value(exact.clone()).unwrap();
    assert_eq!(serde_json::to_value(back).unwrap(), exact);
}

#[test]
fn active_modes_are_complete_and_inactive_rows_survive() {
    for mode in ["consecutive", "rate_based"] {
        let wire = breaker_wire(mode);
        let back: MiddlewareDetailsInfo = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), wire);
    }
    for kind in ["fixed", "exponential"] {
        let wire = retry_wire(kind);
        let back: MiddlewareDetailsInfo = serde_json::from_value(wire.clone()).unwrap();
        assert_eq!(serde_json::to_value(back).unwrap(), wire);
    }
    for (mut wire, field) in [
        (breaker_wire("consecutive"), "consecutive_failures"),
        (breaker_wire("rate_based"), "count_window"),
        (breaker_wire("rate_based"), "minimum_calls"),
        (breaker_wire("rate_based"), "slow_call_duration_ms"),
        (retry_wire("fixed"), "fixed_delay_ms"),
    ] {
        wire["info"].as_object_mut().unwrap().remove(field);
        assert!(
            serde_json::from_value::<MiddlewareDetailsInfo>(wire).is_err(),
            "{field}"
        );
    }
    let mut inactive_partial = breaker_wire("consecutive");
    inactive_partial["info"]
        .as_object_mut()
        .unwrap()
        .remove("slow_call_duration_ms");
    assert!(serde_json::from_value::<MiddlewareDetailsInfo>(inactive_partial).is_ok());
    let mut exponential = retry_wire("exponential");
    exponential["info"]
        .as_object_mut()
        .unwrap()
        .remove("fixed_delay_ms");
    assert!(serde_json::from_value::<MiddlewareDetailsInfo>(exponential).is_ok());
}

#[test]
fn numeric_ranges_units_and_mode_tokens_are_checked() {
    for (mut wire, key, value) in [
        (
            breaker_wire("rate_based"),
            "failure_rate_threshold",
            json!(1.01),
        ),
        (
            breaker_wire("rate_based"),
            "slow_call_rate_threshold",
            json!(0.0),
        ),
        (breaker_wire("rate_based"), "minimum_calls", json!(11)),
        (
            breaker_wire("rate_based"),
            "probes",
            json!(u64::from(u32::MAX) + 1),
        ),
        (breaker_wire("consecutive"), "mode", json!("unknown")),
        (retry_wire("fixed"), "max_attempts", json!(0)),
        (retry_wire("fixed"), "max_backoff_ms", json!("1s")),
        (retry_wire("fixed"), "attempt_start_window_ms", json!(0)),
        (retry_wire("fixed"), "fixed_delay_ms", json!(0.5)),
        (retry_wire("fixed"), "kind", json!("future")),
    ] {
        wire["info"][key] = row(value);
        assert!(
            serde_json::from_value::<MiddlewareDetailsInfo>(wire).is_err(),
            "{key}"
        );
    }
    for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert!(RateLimiterInfo::try_new(setting(value), setting(1.0), None).is_err());
        assert!(MiddlewareExtensionInfo::try_new(BTreeMap::from([(
            "x".into(),
            setting(SettingValueInfo::F64(value))
        )]))
        .is_err());
    }
}

#[test]
fn canonical_row_projection_rejects_wrong_family_types_and_preserves_rows() {
    let rows = BTreeMap::from([
        (
            "middleware.rate_limiter.events_per_second".into(),
            setting(SettingValueInfo::F64(10.0)),
        ),
        (
            "middleware.rate_limiter.cost_per_attempt".into(),
            setting(SettingValueInfo::F64(1.0)),
        ),
    ]);
    assert_eq!(
        MiddlewareDetailsInfo::try_from_settings(MiddlewareFamily::RateLimiter, rows.clone())
            .unwrap(),
        limiter()
    );
    assert!(
        MiddlewareDetailsInfo::try_from_settings(MiddlewareFamily::Retry, rows.clone()).is_err()
    );
    let mut wrong_type = rows.clone();
    wrong_type
        .get_mut("middleware.rate_limiter.events_per_second")
        .unwrap()
        .value = SettingValueInfo::Text("10".into());
    assert!(
        MiddlewareDetailsInfo::try_from_settings(MiddlewareFamily::RateLimiter, wrong_type)
            .is_err()
    );
    let mut extra = rows;
    extra.insert(
        "middleware.rate_limiter.unknown".into(),
        setting(SettingValueInfo::Bool(true)),
    );
    assert!(
        MiddlewareDetailsInfo::try_from_settings(MiddlewareFamily::RateLimiter, extra).is_err()
    );
    let mut wrong_family = serde_json::to_value(limiter()).unwrap();
    wrong_family["kind"] = json!("retry");
    assert!(serde_json::from_value::<MiddlewareDetailsInfo>(wrong_family).is_err());
}

#[test]
fn extension_shapes_have_no_arbitrary_json_or_unknown_envelope_fields() {
    for value in [
        json!(null),
        json!({"kind":"array","value":[]}),
        json!({"kind":"text","value":{}}),
    ] {
        let wire = json!({"kind":"observer","info":{"settings":{"custom.x":row(value)}}});
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
                    info: MiddlewareExtensionInfo::default(),
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
