# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.4.0] - 2026-05-08

The canonical `Topology` is now the single source of truth for the
`/api/topology` wire contract. `obzenflow_infra` and `obzen-flow-ui` no
longer maintain private response DTOs; both serialise and deserialise
the canonical type directly. (FLOWIP-114b)

### Added

- `Topology`, `StageInfo`, and `DirectedEdge` now implement `Serialize` and `Deserialize`. Cycle and SCC caches are reconstructed on deserialise; output is sorted by stage id for determinism.
- `Topology` top-level annotations: `flow_name`, `api_version`, `subgraphs: Vec<TopologySubgraphInfo>`, with fluent `with_*` setters.
- `StageInfo` annotation fields: `status`, `role`, `is_cycle_member`, `middleware`, `join_metadata`, `typing`, `subgraph`. All `Option<_>`, all with fluent `with_*` setters.
- `DirectedEdge` annotation fields: `contracts`, `typing`. Both `Option<_>`, both with fluent `with_*` setters.
- Annotation types under `obzenflow_topology::types`:
  - **Typing**: `TypeHintInfo`, `StageTypingInfo`, `EdgeTypingInfo`, `EdgeTypingRole`, `EdgeTypingLabelSource`.
  - **Middleware**: `MiddlewareInfo`, `CircuitBreakerInfo`, `RateLimiterInfo`, `RetryInfo`, `OpenPolicy`, `BackoffStrategy`.
  - **Other**: `JoinMetadataInfo`, `ContractInfo`, `StageStatus`, `StageSubgraphMembership`, `TopologySubgraphInfo`, `SubgraphInternalEdge`.
- `Topology::populate_derived_stage_annotations()` sets `role` and `is_cycle_member` on every stage from the cached SCC.
- `Topology::derive_edge_typings()` folds stage typing and join metadata into per-edge `EdgeTypingInfo`.
- `Topology::replace_stage_info()` lets infra and runtime attach annotations after validation.
- `TypeHintInfo::display_name()` returns the path-stripped form for UI rendering. Type names are rendered as written in source code; the helper strips Rust path qualifiers and nothing else.

### Changed

- `StageType`, `StageRole`, and `EdgeKind` serialise via `#[serde(rename_all = "snake_case")]`. The serde form now matches `as_str()` output.
- `Topology::flow_name()` prefers the explicit `flow_name` annotation when set, falling back to source-derived naming.
- `StageInfo` and `DirectedEdge` are now `#[non_exhaustive]`.
- `DirectedEdge` is no longer `Copy`. `PartialEq`, `Eq`, and `Hash` are implemented manually on `(from, to, kind)` so the existing dedup invariant is preserved across annotated edges.

### Removed

- `StageMetadata` struct and re-exports. Deprecated since 0.2.0; use `StageInfo` directly.
- `StageExtensions` struct and the `extensions: Option<StageExtensions>` field on `StageInfo`. The `serde_json::Value` blob is replaced by typed annotation fields (`middleware: Option<MiddlewareInfo>`, etc.).
- `EdgeExtensions` struct. Was a dangling re-export, never wired as a `DirectedEdge` field; replaced by typed `contracts` and `typing` annotations.
- `events_per_sec: Option<f64>` field on `DirectedEdge`. Always `None` from this crate; runtime metrics are exported via `/metrics`.
- `Shape::stage_type()` helper. Unused; classification lives on `StageType`.

### Migration

- Direct struct-literal construction of `StageInfo` or `DirectedEdge` outside this crate must use `StageInfo::new(...)` / `DirectedEdge::new(...)` plus `.with_*` setters, due to `#[non_exhaustive]`.
- Consumers that relied on `DirectedEdge: Copy` must clone instead. References were the dominant pattern; most call sites are unaffected.
- Consumers that serialised `StageType`, `StageRole`, or `EdgeKind` directly via serde now see snake_case strings instead of PascalCase. Implementations that read via `as_str()` are unchanged.
- Consumers of the removed `StageExtensions` and `EdgeExtensions` blobs should populate the typed annotation fields (`MiddlewareInfo`, `ContractInfo`, `EdgeTypingInfo`, `StageTypingInfo`) instead.

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
