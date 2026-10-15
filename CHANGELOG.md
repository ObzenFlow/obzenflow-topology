# Changelog

Notable changes to obzenflow-topology, newest first. This crate is pre-1.0.
Compatibility notes cover the Rust API and serialized topology format.

## 0.7.0 - 2026-10-14

### Added

- **Breaking:** Support for merge stages, which combine streams carrying the
  same event type. Exhaustive matches on `StageType` must handle `Merge`, and
  JSON consumers must accept `"merge"`. Upgrade topology producers and Studio
  together.

## 0.6.0

Middleware records now describe validated settings and where each value came
from. Upgrade topology producers and Studio together; the previous middleware
format is no longer accepted.

### Changed

- **Breaking:** Middleware settings use typed records in `MiddlewareDetailsInfo`
  instead of arbitrary configuration JSON. Read the family through
  `MiddlewareAttachmentInfo::family()` and built-in settings through named getters.
  Observer and custom middleware retain typed scalar settings.
- **Breaking:** Attachment keys must use canonical uppercase ULID spelling and
  be unique within a topology. Use `MiddlewareAttachmentKey` in Rust; existing
  128-bit identities are preserved.
- **Breaking:** Loading middleware data rejects malformed settings, missing
  information about where settings came from, invalid active modes, and explicit
  null for optional settings. Omitted burst capacity still means automatic
  capacity, and settings for inactive modes are retained.
- **Breaking:** Updating stage information can fail validation. Handle the result
  from `Topology::replace_stage_info`; invalid middleware information or duplicate
  attachment keys leave the existing topology unchanged.

## 0.5.1 - 2026-07-10

### Added

- Edges record the named composite ports they cross through
  `DirectedEdge::composite_ports`. An edge between two composites records both ports.

### Changed

- **Breaking:** Connections across composite boundaries require complete port
  bindings. Missing, unknown, or mismatched bindings fail validation. Upgrade
  ObzenFlow and Studio together and regenerate 0.5.0 topology documents.

## 0.5.0 - 2026-07-09

Composite subgraphs describe patterns such as sagas or map-reduce as named units.

### Added

- Boundary ports describe where a composite connects to the rest of a pipeline,
  including the member stages and event types at each connection.
- Member stages can carry classifications, such as a saga's pivot or compensation
  stage, through `StageSubgraphMembership::with_class()`.
- Subgraph manifests include a schema version and boundary ports. Existing 0.4
  manifests still load with defaults for the new fields.

### Changed

- **Breaking:** Build subgraph records with constructors and builder methods
  instead of struct literals. This affects `StageSubgraphMembership`,
  `TopologySubgraphInfo`, and `SubgraphInternalEdge`.
- Internal edge roles describe connection lanes, such as data or status. The
  serialized field name remains `role`. Member roles, port names, lane names,
  and subgraph kinds are stable identifiers.

## 0.4.0 - 2026-05-08

ObzenFlow and Studio share a single topology format through `/api/topology`.

### Added

- Complete topologies can be serialized and loaded, with graph caches rebuilt
  on load.
- Typed metadata describes flow names, API versions, subgraphs, stage status,
  roles, cycle membership, middleware, joins, contracts, and event types.
- Helpers derive stage roles, cycle membership, and edge types from the graph,
  and allow stage information to be updated after construction.
- Event type names can be displayed without Rust module paths through
  `TypeHintInfo::display_name()`.

### Changed

- **Breaking:** Edges are no longer implicitly copied. Use `.clone()` where code
  relied on `DirectedEdge` being `Copy`.
- **Breaking:** Build `StageInfo` and `DirectedEdge` with constructors and builder
  methods instead of struct literals.
- **Breaking:** Stage types, roles, and edge kinds use snake_case in JSON. Update
  values such as `FiniteSource` to `finite_source` and `Forward` to `forward`.
- Explicit flow names take precedence over names derived from source stages.
- Edge equality and hashing consider only endpoints and direction, ignoring
  metadata.

### Removed

- **Breaking:** Untyped extension containers (`StageInfo::extensions`,
  `StageExtensions`, and `EdgeExtensions`). Use `StageInfo::middleware` and
  `DirectedEdge::{contracts, typing}` for typed metadata.
- **Breaking:** `StageMetadata`; use `StageInfo`.
- **Breaking:** Per-edge event rates (`DirectedEdge::events_per_sec`); use `/metrics`.
- **Breaking:** `Shape::stage_type()`; classify stages with `StageType` directly.

## 0.3.1 - 2026-03-01

### Added

- Required Developer Certificate of Origin sign-off for pull requests, with
  contribution provenance and licence-header guidance in `CONTRIBUTING.md`.

### Changed

- Clarified trademark ownership and updated licence copyright years to 2025–2026.

## 0.3.0 - 2026-02-19

### Added

- Queries identify groups of stages in feedback loops through
  `Topology::scc_id(stage)` and `Topology::scc_members(id)`. Component IDs remain
  stable across constructions of the same topology; stages outside a cycle
  return `None` from `scc_id`.
- SPDX licence headers.

## 0.2.0 - 2025-12-03

Validation checks whether connections are legal for each stage's role.

### Added

- Stage roles and edge directions distinguish sources, processors, consumers,
  and backflow connections.
- Draft graphs can be built with structural checks and fully validated later.
  Use `Topology::new_unvalidated()` or `TopologyBuilder::build_unchecked()`,
  then `Topology::validate_with_level()` when ready.
- Queries identify sources and sinks by role. Validation errors explain invalid
  connections, missing sources or sinks, and stages that cannot participate in
  a source-to-sink path.

### Changed

- **Breaking:** Stage definitions require a `StageType`, and connections require
  an `EdgeKind`. Also pass a stage type to `TopologyBuilder::add_stage_with_id`.
- **Breaking:** `Topology::new` checks connection semantics and source-to-sink
  reachability in addition to graph structure.

## 0.1.0 - 2025-08-14

Initial standalone release of the graph model shared by ObzenFlow's backend
and frontend, including WASM targets.

### Added

- Stage and edge types, ULID stage IDs, and a topology builder.
- Connectivity checks and cycle detection, with support for multi-stage
  feedback loops and rejection of self-connections.
- Dual MIT OR Apache-2.0 licensing.
