# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

## [0.1.0] - 2025-01-14

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
