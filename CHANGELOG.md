# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-05-08

`Topology` is now the canonical `/api/topology` wire-contract type. The
`obzenflow` and `obzenflow-ui` crates serialize and deserialize
`Topology` directly instead of maintaining private DTO mirrors.

This release also replaces loose extension blobs with typed optional
annotations for stages, edges, subgraphs, contracts, middleware, status,
and typing. Annotations do not affect structural validation, SCC
computation, or traversal. (FLOWIP-114b)

### Breaking Changes

- `DirectedEdge` is no longer `Copy`. The new `contracts` field carries
  a `Vec`, which makes `Copy` impossible. Use `.clone()` where ownership
  is required; most call sites pass references already.

  ```rust
  // 0.3
  let copy = edge;
  // 0.4
  let copy = edge.clone();
  ```

- `StageInfo` and `DirectedEdge` are now `#[non_exhaustive]`. Construct
  with `::new(...)` and `.with_*` setters instead of struct literals.

  ```rust
  // 0.3
  let edge = DirectedEdge { from, to, kind: EdgeKind::Forward };
  // 0.4
  let edge = DirectedEdge::new(from, to, EdgeKind::Forward);
  ```

- `StageType`, `StageRole`, and `EdgeKind` now serialize as `snake_case`.
  The serde form matches `as_str()` output. Update JSON consumers that
  match on Rust enum variant names.

  ```json
  // 0.3
  { "stage_type": "FiniteSource", "kind": "Forward" }
  // 0.4
  { "stage_type": "finite_source", "kind": "forward" }
  ```

  Consumers that already read via `as_str()` are unchanged.

- Removed `StageInfo::extensions` and the `StageExtensions` container.
  If you populated `extensions.middleware`, set `StageInfo::middleware`
  directly with a `MiddlewareInfo`. UI hints have no replacement; track
  them in a typed annotation if you need them.
- Removed `EdgeExtensions`. Populate `DirectedEdge::contracts` and
  `DirectedEdge::typing` with typed annotations instead.
- Removed `StageMetadata` (deprecated since 0.2.0). Use `StageInfo`
  directly.
- Removed `DirectedEdge::events_per_sec`. Runtime metrics are exported
  through `/metrics`.
- Removed `Shape::stage_type()`. Use `StageType` classification directly.

### Added

- `Topology` now implements `Serialize` and `Deserialize`. Cycle and
  SCC caches are recomputed on deserialization, so serialized payloads
  do not need to include them.

  ```rust
  let topology: Topology = serde_json::from_str(&payload)?;
  ```

- Top-level `Topology` annotations: `flow_name`, `api_version`, and a
  `subgraphs` registry, with fluent setters.

  ```rust
  let topology = topology
      .with_flow_name("orders")
      .with_api_version("0.5")
      .with_subgraphs(subgraphs);
  ```

- `StageInfo` annotation fields: `status`, `role`, `is_cycle_member`,
  `middleware`, `join_metadata`, `typing`, `subgraph`. All optional,
  all with fluent `with_*` setters.

  ```rust
  let stage = StageInfo::new(id, "promo_enriched", StageType::Join)
      .with_typing(promo_typing)
      .with_join_metadata(join_meta);
  ```

- `DirectedEdge` annotation fields: `contracts`, `typing`. Both optional,
  both with fluent `with_*` setters.

  ```rust
  let edge = DirectedEdge::new(from, to, EdgeKind::Forward)
      .with_typing(edge_typing);
  ```

- `Topology::populate_derived_stage_annotations()` derives each stage's
  `role` and `is_cycle_member` from cached SCC data.
- `Topology::derive_edge_typings()` folds stage typing and join metadata
  into per-edge `EdgeTypingInfo`. Call after stage typing is attached;
  calling it earlier leaves edge typing unset.
- `Topology::replace_stage_info()` for attaching annotations to a stage
  after structural validation.
- Annotation types under `obzenflow_topology::types`:
  - **Typing**: `TypeHintInfo`, `StageTypingInfo`, `EdgeTypingInfo`,
    `EdgeTypingRole`, `EdgeTypingLabelSource`.
  - **Middleware**: `MiddlewareInfo`, `CircuitBreakerInfo`,
    `RateLimiterInfo`, `RetryInfo`, `OpenPolicy`, `BackoffStrategy`.
  - **Other**: `JoinMetadataInfo`, `ContractInfo`, `StageStatus`,
    `StageSubgraphMembership`, `TopologySubgraphInfo`,
    `SubgraphInternalEdge`.
- `TypeHintInfo::display_name()` returns the path-stripped form for UI
  rendering. Type names are not rewritten beyond stripping Rust path
  qualifiers.

  ```rust
  TypeHintInfo::exact("crate::domain::EnrichedOrder").display_name()
  // Some("EnrichedOrder"), not "Enriched Order"
  ```

### Changed

- `Topology::flow_name()` prefers the explicit `flow_name` annotation
  when set, falling back to source-derived naming. Callers that never
  set the annotation are unaffected.
- `DirectedEdge` equality and hashing now use only `(from, to, kind)`.
  Edges with the same endpoints and kind compare equal even if their
  annotations differ, preserving existing edge deduplication behavior.

## [0.3.1] - 2026-03-01

### Changed
- **TRADEMARKS.md**: clarified trademark ownership, added explicit contribution/trademark separation
- **CONTRIBUTING.md**: added DCO sign-off requirements, contribution provenance guidance for employed contributors, and SPDX header documentation
- **CI**: added DCO sign-off verification as a merge gate on pull requests
- **Licence files**: updated copyright year to 2025-2026

### Added
- `DCO.md` (Developer Certificate of Origin v1.1)

## [0.3.0] - 2026-02-19

### Added
- **SCC partition API** for cycle-aware pipeline coordination (FLOWIP-051n, FLOWIP-051p)
  - `SccId` type: phantom-typed ULID identifier for strongly connected components, derived deterministically from the minimum `StageId` in each SCC's member set
  - `Topology::scc_id(stage_id) -> Option<SccId>`: returns the SCC a stage belongs to, or `None` for non-cycle stages
  - `Topology::scc_members(scc_id) -> Option<&HashSet<StageId>>`: returns the full member set for a given SCC
- **SCC identity is ULID-based and deterministic**: no sequential index allocation, consistent with every other identifier in the system
- Re-exported `SccId` at crate root alongside `StageId`
- Comprehensive test coverage for SCC partitioning: disjoint SCCs get distinct identifiers, DAGs have no SCCs, `is_in_cycle` and `scc_id` always agree, deterministic across constructions, out-of-bounds lookups return `None`
- SPDX license headers on all `.rs` files

### Changed
- `Topology` now retains full SCC partition data (previously flattened to a `HashSet<StageId>`)
- `is_in_cycle` reimplemented on top of cached SCC membership for consistency with the new API
- Version bump from 0.2 to 0.3 (additive API, no breaking changes)

## [0.2.0] - 2025-12-03

### Breaking Changes
- `StageInfo` now requires a `stage_type: StageType` field
- `DirectedEdge` now requires a `kind: EdgeKind` field
- `TopologyBuilder::add_stage_with_id` now requires a `StageType` argument
- `Topology::new` now performs full validation (structural + semantic + reachability)

### Added
- **StageRole enum** with `Producer`, `Processor`, `Consumer` variants for connection semantics validation
- **EdgeKind enum** with `Forward` (`|>`) and `Backward` (`<|`) variants to preserve operator semantics
- **StageType::role()** method to map stage types to their connection role
- **ValidationLevel enum** with `Structural`, `Semantic`, `Full` levels
- **Topology::new_unvalidated()** for structural-only validation (UI/intermediate workflows)
- **TopologyBuilder::build_unchecked()** for structural-only validation
- **Topology::validate_with_level()** for on-demand validation at any level
- **Topology::semantic_source_stages()** and **semantic_sink_stages()** based on StageRole
- **StageExtensions** and **EdgeExtensions** for future metadata (middleware, UI hints)
- New error variants: `NoSources`, `NoSinks`, `UnreachableStages`, `UnproductiveStages`, `InvalidConnection`
- Comprehensive semantic validation test suite

### Design (FLOWIP-TOP-001)
- StageRole is for **validation** (can this connection exist?)
- StageType is for **runtime** (how does this stage behave?)
- Shape is for **visualization** (how many ports, where do they go?)
- Connection semantics validation enforces valid `(StageRole, EdgeKind)` combinations
- Structural invariants ensure at least one Producer and Consumer, and all stages on a source→sink path

## [0.1.0] - 2025-08-14

### Added
- Initial release of obzenflow-topology as a standalone crate
- Core topology data structures (Topology, DirectedEdge) 
- Stage types (StageId, StageType, StageInfo, StageMetadata)
- Topology builder with fluent API
- Validation utilities (connectivity analysis, strongly connected components)
- Support for directed graphs with cycles (feedback loops, retry patterns)
- WASM compatibility for browser environments
- Comprehensive test suite

### Design Decisions
- Multi-stage cycles are supported to enable feedback loops and retry patterns
- Self-cycles (stage connecting to itself) are explicitly forbidden
- Uses ULID for stage identifiers to ensure global uniqueness

### Notes
- Extracted from the ObzenFlow project to enable reuse across backend and frontend
- Dual licensed under MIT OR Apache-2.0
