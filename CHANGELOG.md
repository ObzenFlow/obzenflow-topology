# Changelog

Notable changes to `obzenflow-topology`, newest first. This crate is pre-1.0;
breaking API and serialized-format changes are called out below.

## [0.7.0] - 2026-10-14

### Breaking changes

- Added `StageType::Merge` for processor stages that combine streams of the
  same type using a framework strategy. It serializes as `"merge"`. Add a
  `StageType::Merge` arm to exhaustive matches on `StageType`, and upgrade
  topology producers and Studio together.

## [0.6.0]

Middleware attachments now describe validated settings and where each setting
came from.

### Breaking changes

- `MiddlewareDetailsInfo` replaces separate family fields and arbitrary
  configuration JSON. Built-in middleware carries typed resolved settings and
  their provenance; observer and custom middleware carry typed scalar settings.
  Read the family through `MiddlewareAttachmentInfo::family()` and built-in
  settings through named getters.
- Attachment keys use `MiddlewareAttachmentKey`, preserving their existing
  128-bit identity. Serialized keys must use canonical uppercase ULID spelling
  and be unique within a topology.
- Deserialization rejects malformed settings, missing provenance, explicit null
  for optional settings, invalid active modes, and duplicate attachment keys.
  Omitted burst capacity still means automatic capacity; settings for inactive
  modes are retained.
- `Topology::replace_stage_info` returns a `ValidationResult`. Handle validation
  errors; invalid middleware information or duplicate keys leave the topology
  unchanged.
- Upgrade topology producers and Studio together. The previous middleware
  format is no longer accepted.

## [0.5.1] - 2026-07-10

### Breaking changes

- Edges crossing a composite boundary must identify each named port they cross.
  Missing, partial, unknown, or endpoint-mismatched bindings fail validation.
  Upgrade ObzenFlow and Studio together and regenerate 0.5.0 topology documents.

### Added

- `CompositePortRef` and `DirectedEdge::composite_ports` record these bindings.
  An edge between two composites carries a port reference for each composite.

## [0.5.0] - 2026-07-09

Composite subgraphs can describe patterns such as sagas or map-reduce as named
units with boundary ports and classified member stages.

### Breaking changes

- `StageSubgraphMembership`, `TopologySubgraphInfo`, and `SubgraphInternalEdge`
  are now `#[non_exhaustive]`. Replace struct literals with `::new(...)` and
  `.with_*` setters. For example, replace
  `SubgraphInternalEdge { from_stage_id, to_stage_id, role }` with
  `SubgraphInternalEdge::new(from_stage_id, to_stage_id, role)`.

### Added

- `BoundaryPortSpec` and `PortDirection` describe a composite's connection
  points, member stages, and event types.
- `StageSubgraphMembership::with_class()` adds an optional member classification,
  such as a saga's `pivot` or `compensation` stage.
- `TopologySubgraphInfo` gains `boundary_ports` and `schema_version`. Existing
  0.4 manifests still load with defaults for the new fields.

### Changed

- `SubgraphInternalEdge::role` describes an edge lane, such as `data` or `status`.
  Its serialized field name remains `role`.
- Member roles, port names, edge lane names, and subgraph kinds are stable
  identifiers; renaming them is a breaking change.

## [0.4.0] - 2026-05-08

`Topology` becomes the shared `/api/topology` format for ObzenFlow and Studio,
with typed annotations for stages, edges, and subgraphs.

### Breaking changes

- **`DirectedEdge` is no longer `Copy`.** Use `.clone()` where code relied on
  copy semantics.
- **`StageInfo` and `DirectedEdge` are now `#[non_exhaustive]`.** Replace struct
  literals with `::new(...)` and `.with_*` setters:

  ```rust
  let edge = DirectedEdge::new(from, to, EdgeKind::Forward);
  ```

- **Enums serialize as `snake_case`.** Update consumers of `StageType`,
  `StageRole`, and `EdgeKind` JSON. For example, `FiniteSource` becomes
  `finite_source` and `Forward` becomes `forward`:

  ```json
  { "stage_type": "finite_source", "kind": "forward" }
  ```

- **Removed extension containers.** Replace `StageInfo::extensions`,
  `StageExtensions`, and `EdgeExtensions` with typed `StageInfo::middleware`
  and `DirectedEdge::{contracts, typing}` annotations.
- **Other removals:** replace `StageMetadata` with `StageInfo` and
  `Shape::stage_type()` with direct `StageType` classification.
  `DirectedEdge::events_per_sec` is removed; metrics are exported through
  `/metrics`.

### Added

- `Serialize` and `Deserialize` for `Topology`, with traversal and cycle caches
  rebuilt on load.
- Typed annotations for flow names, API versions, subgraphs, stage status,
  roles, cycle membership, middleware, joins, contracts, and event types.
- `populate_derived_stage_annotations()` fills stage roles and cycle membership;
  `derive_edge_typings()` derives edge typing from stage typing;
  `replace_stage_info()` replaces stage information after construction.
- `TypeHintInfo::display_name()` strips Rust path qualifiers for display.

### Changed

- `Topology::flow_name()` prefers an explicit annotation, falling back to a name
  derived from sources.
- `DirectedEdge` equality and hashing use only `(from, to, kind)`, ignoring
  annotations.

## [0.3.1] - 2026-03-01

- Added the Developer Certificate of Origin and required DCO sign-off on pull
  requests, with contribution provenance and SPDX guidance in `CONTRIBUTING.md`.
- Clarified trademark ownership and updated licence copyright years to 2025–2026.

## [0.3.0] - 2026-02-19

- Added `SccId`, `Topology::scc_id(stage)`, and `Topology::scc_members(id)` for
  identifying groups of stages in feedback loops. `scc_id` returns `None` for
  stages outside a cycle.
- Component IDs are derived from the smallest member `StageId` and remain
  stable across constructions of the same topology.
- Added SPDX licence headers.

## [0.2.0] - 2025-12-03

Validation now checks whether connections are legal for each stage's role.

### Breaking changes

- `StageInfo` requires a `stage_type: StageType`, `DirectedEdge` requires a
  `kind: EdgeKind`, and `TopologyBuilder::add_stage_with_id` takes a `StageType`.
- `Topology::new` runs full validation, including connection semantics and
  reachability.

### Added

- `StageRole` (`Producer`, `Processor`, `Consumer`) and `EdgeKind` (`Forward`,
  `Backward`) describe connection semantics.
- `ValidationLevel`, `Topology::new_unvalidated()`,
  `TopologyBuilder::build_unchecked()`, and `Topology::validate_with_level()`
  support structural-only construction and validation on demand.
- Role-based `semantic_source_stages()` and `semantic_sink_stages()`, plus
  `NoSources`, `NoSinks`, `UnreachableStages`, `UnproductiveStages`, and
  `InvalidConnection` errors.

## [0.1.0] - 2025-08-14

Initial standalone release of the graph model shared by ObzenFlow's backend
and frontend, including WASM targets.

- Stage and edge types, ULID stage IDs, and `TopologyBuilder`.
- Connectivity checks and cycle detection, with support for multi-stage
  feedback loops and rejection of self-connections.
- Dual MIT OR Apache-2.0 licensing.
