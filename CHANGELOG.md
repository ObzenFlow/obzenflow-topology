# Changelog

All notable changes to `obzenflow-topology` are recorded here, newest first.

This crate follows [Semantic Versioning](https://semver.org). It is pre-1.0, so
a minor release (0.4 to 0.5) may contain breaking changes. Each one is called
out below with the code you need to change.

## [0.5.1] - 2026-07-10

### Added

- (FLOWIP-128a B3/B4) `CompositePortRef` and `DirectedEdge::composite_ports`
  persist the selected named port on every physical composite cut edge. A
  composite-to-composite edge carries both independently owned refs.

### Changed

- Composite graph cuts are a coordinated 0.5.1 contract. Every crossing edge
  must carry its complete named-port binding when constructed or deserialized;
  missing, partial, unknown, or endpoint-mismatched bindings fail validation.
  ObzenFlow and Studio must upgrade together and regenerate any 0.5.0 topology
  documents rather than attempting to infer or render an unbound cut.

## [0.5.0] - 2026-07-09

First-class composite subgraphs. A topology can now describe a higher-level
shape, such as a saga or a map-reduce, as a named unit with declared boundary
ports and a role for each member stage. This builds the subgraph annotations
introduced in 0.4 into a substrate that composite patterns can sit on top of.

The subgraph annotation types are now designed for forward compatibility. They
are `#[non_exhaustive]` with constructors and builder-style setters, so future
releases can add fields without breaking your code, and their serialized form
skips fields that are not set, so older tooling keeps reading newer manifests.

### Breaking changes

`StageSubgraphMembership`, `TopologySubgraphInfo`, and `SubgraphInternalEdge`
can no longer be built with a struct literal. Use the `::new(...)` constructor
and `.with_*` setters instead. These types are new in 0.4 and are usually read
rather than constructed, so most code is unaffected.

```rust
// Before (0.4)
let edge = SubgraphInternalEdge { from_stage_id, to_stage_id, role };

// After (0.5)
let edge = SubgraphInternalEdge::new(from_stage_id, to_stage_id, lane);
```

### Added

- **Boundary ports.** New `BoundaryPortSpec` and `PortDirection` types let a
  composite declare its external connection points: the direction, the member
  stage each port belongs to, and the event types it carries. Diagnostics and
  the Studio UI use these to show exactly how a composite wires into the flow.
- **Member classification.** `StageSubgraphMembership` gains an optional `class`
  (set with `.with_class()`) so a composite can label each member in its own
  terms. A saga, for example, marks members as `compensatable`, `retriable`,
  `pivot`, `compensation`, or `driver`.
- **Richer registry entries.** `TopologySubgraphInfo` gains `boundary_ports`
  and a `schema_version` so manifests can evolve over time. Kind-owned
  extension data remains deferred until a composite kind ships as its real
  producer and consumer.
- **Forward-compatible manifests,** covered by new snapshot and cross-version
  tests. New fields serialize only when set and default sensibly when absent, so
  a 0.4 manifest still loads unchanged.

### Changed

- `SubgraphInternalEdge`'s `role` field is a lane label (`data`, `manifest`,
  `status`, `terminal`, or a composite-specific extra), not a member role. It
  keeps the `role` name so existing serialized data is unaffected.

### A note on stability

Member roles, port names, edge lane names, and subgraph kinds are stable
identifiers. Adding fields to these types is safe; renaming one of those strings
is a breaking change for anything that matches on it.

## [0.4.0] - 2026-05-08

`Topology` became the single source of truth for the `/api/topology` wire
format. The `obzenflow` and `obzenflow-ui` crates now serialize and deserialize
`Topology` directly instead of keeping their own private mirror types. This
release also replaces the old untyped "extension" blobs with typed, optional
annotations for stages, edges, subgraphs, contracts, middleware, status, and
typing. Annotations are metadata only and never affect structural validation,
cycle detection, or traversal.

### Breaking changes

- **`DirectedEdge` is no longer `Copy`.** It now carries a `Vec` of contracts,
  which rules out `Copy`. Use `.clone()` where you relied on copy semantics;
  most call sites already pass references.

  ```rust
  let copy = edge.clone();   // was: let copy = edge;
  ```

- **`StageInfo` and `DirectedEdge` are now `#[non_exhaustive]`.** Build them with
  `::new(...)` and `.with_*` setters instead of struct literals.

  ```rust
  let edge = DirectedEdge::new(from, to, EdgeKind::Forward);
  ```

- **Enums serialize as `snake_case`.** `StageType`, `StageRole`, and `EdgeKind`
  now serialize to match their `as_str()` form. Update any JSON consumer that
  matched on Rust variant names; consumers already reading `as_str()` are
  unaffected.

  ```json
  { "stage_type": "finite_source", "kind": "forward" }   // was "FiniteSource" / "Forward"
  ```

- **Removed the extension containers.** `StageInfo::extensions` /
  `StageExtensions` and `EdgeExtensions` are gone. Set `StageInfo::middleware`
  and `DirectedEdge::{contracts, typing}` directly with their typed annotations.
- **Other removals:** `StageMetadata` (deprecated since 0.2; use `StageInfo`),
  `DirectedEdge::events_per_sec` (metrics are exported through `/metrics`), and
  `Shape::stage_type()` (classify with `StageType` directly).

### Added

- **`Topology` is now `Serialize` and `Deserialize`.** Cycle and SCC caches are
  recomputed on load, so serialized payloads stay small and never carry derived
  state.
- **Typed annotations throughout.** Top-level `flow_name`, `api_version`, and a
  `subgraphs` registry; per-stage `status`, `role`, `is_cycle_member`,
  `middleware`, `join_metadata`, `typing`, and `subgraph`; per-edge `contracts`
  and `typing`. All optional, all with fluent `with_*` setters.
- **Helpers that derive annotations from structure:**
  `populate_derived_stage_annotations()` fills each stage's role and cycle
  membership from cached SCC data, `derive_edge_typings()` folds stage typing
  into per-edge typing, and `replace_stage_info()` attaches annotations after
  validation.
- `TypeHintInfo::display_name()` for UI rendering, which strips Rust path
  qualifiers without otherwise rewriting the type name.

### Changed

- `Topology::flow_name()` prefers an explicit `flow_name` annotation when set,
  and falls back to source-derived naming otherwise.
- `DirectedEdge` equality and hashing use only `(from, to, kind)`, so edges with
  the same endpoints and kind still deduplicate even when their annotations
  differ.

## [0.3.1] - 2026-03-01

Governance and provenance housekeeping ahead of the public launch.

- Added `DCO.md` (Developer Certificate of Origin 1.1) and made DCO sign-off a
  required check on pull requests.
- Expanded `CONTRIBUTING.md` with sign-off instructions, provenance guidance for
  employed contributors, and SPDX header rules.
- Clarified trademark ownership and the contribution/trademark boundary in
  `TRADEMARKS.md`, and updated the licence copyright year to 2025-2026.

## [0.3.0] - 2026-02-19

Added a cycle-aware API for strongly connected components (SCCs), for
coordinating pipelines that contain feedback loops.

- `Topology::scc_id(stage)` returns the component a stage belongs to, or `None`
  outside a cycle, and `Topology::scc_members(id)` returns a component's full
  member set.
- SCC identifiers are ULIDs derived deterministically from each component's
  smallest `StageId`, so they are stable across runs and consistent with every
  other identifier in the crate.
- `Topology` now keeps full SCC partition data, `is_in_cycle` is computed from
  it, and `SccId` is re-exported at the crate root.
- Added SPDX licence headers to all source files. Additive only; nothing breaks.

## [0.2.0] - 2025-12-03

Made stage and edge semantics explicit, so a topology can validate that its
connections make sense rather than only that they are structurally connected.

### Breaking changes

- `StageInfo` now requires a `stage_type: StageType`, `DirectedEdge` requires a
  `kind: EdgeKind`, and `TopologyBuilder::add_stage_with_id` takes a `StageType`.
- `Topology::new` now runs full validation (structural, semantic, and
  reachability) rather than structural checks alone.

### Added

- `StageRole` (`Producer`, `Processor`, `Consumer`) and `EdgeKind` (`Forward`,
  `Backward`) to capture connection semantics and preserve operator direction.
- Validation you can dial in: `ValidationLevel` (`Structural`, `Semantic`,
  `Full`), `Topology::new_unvalidated()` and `TopologyBuilder::build_unchecked()`
  for structural-only workflows, and `Topology::validate_with_level()` on demand.
- Role-based `semantic_source_stages()` and `semantic_sink_stages()`, plus
  clearer error variants: `NoSources`, `NoSinks`, `UnreachableStages`,
  `UnproductiveStages`, and `InvalidConnection`.

The model keeps three concerns separate: `StageRole` answers "can this
connection exist?", `StageType` answers "how does this stage behave at runtime?",
and `Shape` answers "how is it drawn?".

## [0.1.0] - 2025-08-14

Initial release as a standalone crate, extracted from ObzenFlow so the topology
model can be shared by the backend and the frontend, including WASM targets.

- Core data structures: `Topology`, `DirectedEdge`, `StageId`, `StageType`,
  `StageInfo`, and a fluent `TopologyBuilder`.
- Validation utilities: connectivity analysis and strongly connected component
  detection.
- Support for directed graphs with cycles, for feedback loops and retry
  patterns. Self-cycles are rejected. Stage IDs are ULIDs for global uniqueness.
- Dual licensed under MIT OR Apache-2.0.
