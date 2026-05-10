// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Round-trip serde and structural-vs-annotation isolation tests for the
//! canonical `Topology` (FLOWIP-114b).
//!
//! These tests prove two invariants:
//!
//! 1. The canonical `Topology` round-trips through serde with both an empty
//!    annotation set and a fully populated annotation set. Caches are
//!    rebuilt by `Deserialize`.
//! 2. Graph algorithms (validation, SCC, fingerprint) read only the
//!    structural fields. Two topologies with identical structural graphs
//!    but different annotations produce identical fingerprints and pass
//!    validation identically.

use obzenflow_topology::{
    BackoffStrategy, CircuitBreakerInfo, ContractInfo, DirectedEdge, EdgeTypingInfo,
    EdgeTypingLabelSource, EdgeTypingRole, JoinMetadataInfo, MiddlewareInfo, OpenPolicy,
    RateLimiterInfo, RetryInfo, StageInfo, StageStatus, StageSubgraphMembership, StageType,
    StageTypingInfo, SubgraphInternalEdge, Topology, TopologyBuilder, TopologySubgraphInfo,
    TypeHintInfo,
};

fn build_minimal_topology() -> Topology {
    let mut builder = TopologyBuilder::new();
    let source = builder.add_stage(Some("orders".to_string()));
    builder.reset_current();
    let transform = builder.add_stage(Some("validated".to_string()));
    builder.reset_current();
    let sink = builder.add_stage(Some("printer".to_string()));
    builder.reset_current();
    builder.add_edge(source, transform);
    builder.add_edge(transform, sink);
    builder
        .build_unchecked()
        .expect("topology should build with structural validation")
}

fn build_three_stage_with_join() -> Topology {
    use obzenflow_topology::StageType;
    let mut builder = TopologyBuilder::new();
    let stream = builder.add_stage_with_id(
        obzenflow_topology::StageId::from_bytes(2_001_u128.to_be_bytes()),
        Some("orders".to_string()),
        StageType::FiniteSource,
    );
    builder.reset_current();
    let catalog = builder.add_stage_with_id(
        obzenflow_topology::StageId::from_bytes(2_002_u128.to_be_bytes()),
        Some("promotions".to_string()),
        StageType::FiniteSource,
    );
    builder.reset_current();
    let join = builder.add_stage_with_id(
        obzenflow_topology::StageId::from_bytes(2_003_u128.to_be_bytes()),
        Some("promo_enriched".to_string()),
        StageType::Join,
    );
    builder.reset_current();
    builder.add_edge(stream, join);
    builder.add_edge(catalog, join);
    builder.build_unchecked().expect("topology should build")
}

#[test]
fn topology_round_trips_through_serde_with_no_annotations() {
    let topology = build_minimal_topology();
    let fingerprint_before = topology.topology_fingerprint();

    let json = serde_json::to_string(&topology).expect("serialise");
    let restored: Topology = serde_json::from_str(&json).expect("deserialise");

    assert_eq!(restored.topology_fingerprint(), fingerprint_before);
    assert_eq!(restored.num_stages(), topology.num_stages());
    assert_eq!(restored.edges().len(), topology.edges().len());
    assert!(restored.flow_name_annotation().is_none());
    assert!(restored.api_version().is_none());
    assert!(restored.subgraphs().is_empty());
}

#[test]
fn topology_round_trips_with_full_annotations() {
    let topology = build_three_stage_with_join();
    let stage_ids: Vec<_> = topology.stages().map(|s| s.id).collect();
    let stream_id = stage_ids
        .iter()
        .copied()
        .find(|id| topology.stage_name(*id) == Some("orders"))
        .unwrap();
    let catalog_id = stage_ids
        .iter()
        .copied()
        .find(|id| topology.stage_name(*id) == Some("promotions"))
        .unwrap();
    let join_id = stage_ids
        .iter()
        .copied()
        .find(|id| topology.stage_name(*id) == Some("promo_enriched"))
        .unwrap();

    // Annotate the join stage with a full set of optional fields.
    let mut join_info = topology.stage_info(join_id).cloned().unwrap();
    join_info = join_info
        .with_status(StageStatus::Running)
        .with_role(StageType::Transform.role())
        .with_is_cycle_member(false)
        .with_middleware(
            MiddlewareInfo::new(vec!["rate_limiter".to_string(), "retry".to_string()])
                .with_rate_limiter(RateLimiterInfo {
                    tokens_per_sec: 10.0,
                    burst_capacity: 20.0,
                    configured_burst_capacity: Some(20.0),
                    cost_per_event: 1.0,
                    limit_rate: 10.0,
                })
                .with_circuit_breaker(CircuitBreakerInfo {
                    threshold: 3,
                    cooldown_ms: 1500,
                    open_policy: OpenPolicy::FailFast,
                    has_fallback: false,
                })
                .with_retry(RetryInfo {
                    max_attempts: Some(5),
                    backoff: BackoffStrategy::Exponential,
                    base_delay_ms: Some(100),
                }),
        )
        .with_join_metadata(JoinMetadataInfo::new(vec![catalog_id], vec![stream_id]))
        .with_typing(StageTypingInfo {
            input_type: TypeHintInfo::Unspecified,
            output_type: TypeHintInfo::exact("EnrichedOrderWithPromo"),
            boundary_in_type: TypeHintInfo::Unspecified,
            boundary_out_type: TypeHintInfo::Unspecified,
            reference_type: TypeHintInfo::exact("Promotion"),
            stream_type: TypeHintInfo::exact("EnrichedOrder"),
            is_placeholder: false,
            placeholder_message: None,
        })
        .with_subgraph(StageSubgraphMembership {
            subgraph_id: "ai_map_reduce:digest".to_string(),
            kind: "ai_map_reduce".to_string(),
            binding: "digest".to_string(),
            role: "chunk".to_string(),
            order: 0,
            is_entry: true,
            is_exit: false,
        });

    // Annotate the stream-to-join edge with typing + a contract.
    let mut edges: Vec<DirectedEdge> = topology
        .edges()
        .iter()
        .map(|edge| {
            if edge.from == stream_id && edge.to == join_id {
                edge.clone()
                    .with_typing(EdgeTypingInfo::new(
                        EdgeTypingRole::Stream,
                        EdgeTypingLabelSource::DownstreamStreamType,
                        TypeHintInfo::exact("EnrichedOrder"),
                    ))
                    .with_contracts(vec![ContractInfo::new("TransportContract")])
            } else if edge.from == catalog_id && edge.to == join_id {
                edge.clone().with_typing(EdgeTypingInfo::new(
                    EdgeTypingRole::Reference,
                    EdgeTypingLabelSource::DownstreamReferenceType,
                    TypeHintInfo::exact("Promotion"),
                ))
            } else {
                edge.clone()
            }
        })
        .collect();
    edges.sort_unstable_by_key(|e| (e.from.to_bytes(), e.to.to_bytes()));

    let mut stages: Vec<StageInfo> = topology.stages().cloned().collect();
    if let Some(info) = stages.iter_mut().find(|s| s.id == join_id) {
        *info = join_info.clone();
    }

    let subgraph_registry = vec![TopologySubgraphInfo {
        subgraph_id: "ai_map_reduce:digest".to_string(),
        kind: "ai_map_reduce".to_string(),
        binding: "digest".to_string(),
        label: "digest".to_string(),
        member_stage_ids: vec![join_id],
        internal_edges: vec![SubgraphInternalEdge {
            from_stage_id: join_id,
            to_stage_id: join_id,
            role: "self".to_string(),
        }],
        entry_stage_ids: vec![join_id],
        exit_stage_ids: vec![join_id],
        parent_subgraph_id: None,
        collapsible: true,
    }];

    let annotated = Topology::new_unvalidated(stages, edges)
        .expect("annotated topology builds")
        .with_flow_name("promo_enrichment_flow")
        .with_api_version("0.5")
        .with_subgraphs(subgraph_registry.clone());

    let json = serde_json::to_string(&annotated).expect("serialise");
    let restored: Topology = serde_json::from_str(&json).expect("deserialise");

    // Top-level annotations preserved.
    assert_eq!(
        restored.flow_name_annotation(),
        Some("promo_enrichment_flow")
    );
    assert_eq!(restored.api_version(), Some("0.5"));
    assert_eq!(restored.subgraphs(), subgraph_registry.as_slice());

    // Stage-level annotations preserved on the join stage.
    let join_back = restored.stage_info(join_id).expect("join restored");
    assert_eq!(join_back.status, Some(StageStatus::Running));
    assert!(join_back.is_cycle_member.is_some());
    let typing = join_back.typing.as_ref().expect("join typing");
    assert_eq!(
        typing.output_type,
        TypeHintInfo::exact("EnrichedOrderWithPromo")
    );
    assert_eq!(typing.reference_type, TypeHintInfo::exact("Promotion"));
    let join_meta = join_back.join_metadata.as_ref().expect("join metadata");
    assert_eq!(join_meta.catalog_source_ids, vec![catalog_id]);
    assert_eq!(join_meta.stream_source_ids, vec![stream_id]);
    let middleware = join_back.middleware.as_ref().expect("middleware");
    assert_eq!(middleware.stack, vec!["rate_limiter", "retry"]);
    assert_eq!(
        middleware.circuit_breaker.as_ref().unwrap().open_policy,
        OpenPolicy::FailFast
    );
    let subgraph = join_back.subgraph.as_ref().expect("subgraph membership");
    assert_eq!(subgraph.subgraph_id, "ai_map_reduce:digest");

    // Edge-level annotations preserved.
    let stream_edge = restored
        .edges()
        .iter()
        .find(|e| e.from == stream_id && e.to == join_id)
        .expect("stream edge");
    let stream_typing = stream_edge.typing.as_ref().expect("stream typing");
    assert_eq!(stream_typing.role, EdgeTypingRole::Stream);
    assert_eq!(
        stream_typing.label_source,
        EdgeTypingLabelSource::DownstreamStreamType
    );
    assert_eq!(
        stream_typing.payload_type,
        TypeHintInfo::exact("EnrichedOrder")
    );
    let contracts = stream_edge.contracts.as_ref().expect("contracts");
    assert_eq!(contracts.len(), 1);
    assert_eq!(contracts[0].name, "TransportContract");

    let catalog_edge = restored
        .edges()
        .iter()
        .find(|e| e.from == catalog_id && e.to == join_id)
        .expect("catalog edge");
    let catalog_typing = catalog_edge.typing.as_ref().expect("catalog typing");
    assert_eq!(catalog_typing.role, EdgeTypingRole::Reference);
    assert_eq!(
        catalog_typing.payload_type,
        TypeHintInfo::exact("Promotion")
    );

    // Caches were rebuilt: structural fingerprint of the deserialised
    // topology matches the original (annotated) topology, proving that
    // annotations do not contribute to structural identity and that
    // SCC/adjacency caches are reconstructed faithfully on deserialise.
    assert_eq!(
        restored.topology_fingerprint(),
        annotated.topology_fingerprint()
    );
}

#[test]
fn structural_algorithms_ignore_annotations() {
    let bare = build_minimal_topology();
    let bare_fingerprint = bare.topology_fingerprint();
    let bare_metrics = bare.metrics();

    // Build a sister topology with identical structure but every stage and
    // edge annotated.
    let stages: Vec<StageInfo> = bare
        .stages()
        .cloned()
        .map(|s| {
            s.with_status(StageStatus::Running)
                .with_role(StageType::Transform.role())
                .with_is_cycle_member(false)
                .with_typing(StageTypingInfo {
                    input_type: TypeHintInfo::exact("Foo"),
                    output_type: TypeHintInfo::exact("Bar"),
                    boundary_in_type: TypeHintInfo::Unspecified,
                    boundary_out_type: TypeHintInfo::Unspecified,
                    reference_type: TypeHintInfo::Unspecified,
                    stream_type: TypeHintInfo::Unspecified,
                    is_placeholder: false,
                    placeholder_message: None,
                })
        })
        .collect();
    let edges: Vec<DirectedEdge> = bare
        .edges()
        .iter()
        .cloned()
        .map(|e| {
            e.with_typing(EdgeTypingInfo::new(
                EdgeTypingRole::Input,
                EdgeTypingLabelSource::UpstreamOutputType,
                TypeHintInfo::exact("Bar"),
            ))
        })
        .collect();

    let annotated = Topology::new_unvalidated(stages, edges).expect("annotated");

    // Fingerprint identical: annotations do not contribute.
    assert_eq!(annotated.topology_fingerprint(), bare_fingerprint);

    // Metrics identical: graph shape unchanged.
    let annotated_metrics = annotated.metrics();
    assert_eq!(annotated_metrics.num_stages, bare_metrics.num_stages);
    assert_eq!(annotated_metrics.num_edges, bare_metrics.num_edges);
    assert_eq!(annotated_metrics.num_sources, bare_metrics.num_sources);
    assert_eq!(annotated_metrics.num_sinks, bare_metrics.num_sinks);
    assert_eq!(annotated_metrics.max_fan_in, bare_metrics.max_fan_in);
    assert_eq!(annotated_metrics.max_fan_out, bare_metrics.max_fan_out);
    assert_eq!(annotated_metrics.max_depth, bare_metrics.max_depth);

    // SCC computation: neither has cycles, both report no cycle members.
    for stage in annotated.stages() {
        assert!(!annotated.is_in_cycle(stage.id));
        assert!(annotated.scc_id(stage.id).is_none());
    }
}

#[test]
fn populate_derived_stage_annotations_sets_role_and_cycle_membership() {
    let topology = build_minimal_topology().populate_derived_stage_annotations();
    for stage in topology.stages() {
        assert_eq!(stage.is_cycle_member, Some(false));
        assert_eq!(stage.role, Some(stage.stage_type.role()));
    }
}

#[test]
fn topology_wire_omits_default_annotations() {
    let topology = build_minimal_topology();
    let json = serde_json::to_string(&topology).expect("serialise");
    // Optional top-level annotations are skipped when None.
    assert!(!json.contains("flow_name"));
    assert!(!json.contains("api_version"));
    // Subgraphs always emits as []; assert the empty array is present.
    assert!(json.contains("\"subgraphs\":[]"));
    // Stage annotations are skipped when None.
    assert!(!json.contains("\"status\""));
    assert!(!json.contains("\"role\""));
    assert!(!json.contains("\"typing\""));
}

#[test]
fn derive_edge_typings_classifies_join_legs_and_forwards() {
    let topology = build_three_stage_with_join();
    let stage_ids: Vec<_> = topology.stages().map(|s| s.id).collect();
    let stream_id = stage_ids
        .iter()
        .copied()
        .find(|id| topology.stage_name(*id) == Some("orders"))
        .unwrap();
    let catalog_id = stage_ids
        .iter()
        .copied()
        .find(|id| topology.stage_name(*id) == Some("promotions"))
        .unwrap();
    let join_id = stage_ids
        .iter()
        .copied()
        .find(|id| topology.stage_name(*id) == Some("promo_enriched"))
        .unwrap();

    let mut stages: Vec<StageInfo> = topology.stages().cloned().collect();
    if let Some(info) = stages.iter_mut().find(|s| s.id == join_id) {
        *info = info
            .clone()
            .with_join_metadata(JoinMetadataInfo::new(vec![catalog_id], vec![stream_id]))
            .with_typing(StageTypingInfo {
                input_type: TypeHintInfo::Unspecified,
                output_type: TypeHintInfo::exact("EnrichedOrderWithPromo"),
                boundary_in_type: TypeHintInfo::Unspecified,
                boundary_out_type: TypeHintInfo::Unspecified,
                reference_type: TypeHintInfo::exact("Promotion"),
                stream_type: TypeHintInfo::exact("EnrichedOrder"),
                is_placeholder: false,
                placeholder_message: None,
            });
    }

    let edges: Vec<DirectedEdge> = topology.edges().to_vec();
    let derived = Topology::new_unvalidated(stages, edges)
        .expect("rebuild")
        .derive_edge_typings();

    let catalog_edge = derived
        .edges()
        .iter()
        .find(|e| e.from == catalog_id && e.to == join_id)
        .unwrap();
    let typing = catalog_edge.typing.as_ref().expect("catalog edge typing");
    assert_eq!(typing.role, EdgeTypingRole::Reference);
    assert_eq!(
        typing.label_source,
        EdgeTypingLabelSource::DownstreamReferenceType
    );
    assert_eq!(typing.payload_type, TypeHintInfo::exact("Promotion"));

    let stream_edge = derived
        .edges()
        .iter()
        .find(|e| e.from == stream_id && e.to == join_id)
        .unwrap();
    let typing = stream_edge.typing.as_ref().expect("stream edge typing");
    assert_eq!(typing.role, EdgeTypingRole::Stream);
    assert_eq!(typing.payload_type, TypeHintInfo::exact("EnrichedOrder"));
}

#[test]
fn flow_name_prefers_annotation_over_derived() {
    let topology = build_minimal_topology().with_flow_name("custom_flow");
    assert_eq!(topology.flow_name(), "custom_flow");

    let unannotated = build_minimal_topology();
    // Falls back to "<source>_flow" because there is one source.
    assert!(unannotated.flow_name().ends_with("_flow"));
    assert_ne!(unannotated.flow_name(), "custom_flow");
}
