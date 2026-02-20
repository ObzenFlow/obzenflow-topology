# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
