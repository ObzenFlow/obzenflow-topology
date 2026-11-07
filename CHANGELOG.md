# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.8.0] - 2026-11-07

Built-in middleware now lists its settings the same way custom middleware does, each with its value, where the value came from and the text to show for it. Adding a setting to a middleware no longer needs a new release of this crate. A stage that outputs several event types can now name them all.

### Breaking changes
- The rate limiter, circuit breaker and retry records are removed, along with their modes: `RateLimiterInfo`, `CircuitBreakerInfo`, `RetryInfo`, `CircuitBreakerMode` and `RetryKind`. Read any middleware's settings through `MiddlewareDetailsInfo::settings()`.
- `MiddlewareExtensionInfo` is renamed `MiddlewareSettingsInfo`, and `MiddlewareDetailsInfo::try_from_settings` takes one.
- `TypeHintInfo::Mixed` now holds the names of its types, so match it as `TypeHintInfo::Mixed { .. }`. A topology whose mixed types have no names no longer loads.

### Added
- Each setting says whether it's in use or worked out automatically, and carries the text to show for it. A middleware can also carry a short summary, such as a breaker's threshold.
- `TypeHintInfo::mixed` builds a mixed type from its names, and `TypeHintInfo::display_name()` shows it as a set, such as { OrderAccepted, OrderRejected }.

### Changed
- Loading middleware now only checks that each setting is well formed, so it no longer rejects a value out of range or a missing setting. A built-in middleware's settings must start with its name, such as `middleware.retry.`, and their text can't be empty.
- `Topology::derive_edge_typings` keeps an edge's type if you've already set it, and fills in the rest.

### Upgrading from 0.7

Read a setting by its name instead of through its middleware's record:

```diff
-if let MiddlewareDetailsInfo::RateLimiter(info) = details {
-    println!("{}", info.events_per_second().value);
-}
+let settings = details.settings().settings();
+if let Some(rate) = settings.get("middleware.rate_limiter.events_per_second") {
+    println!("{}", rate.display());
+}
```

Match mixed types with their names:

```diff
-TypeHintInfo::Mixed => "mixed".to_string(),
+TypeHintInfo::Mixed { .. } => "mixed".to_string(),
```

Topologies written by 0.7 don't load in 0.8, so upgrade the code that writes them and the code that reads them together, such as ObzenFlow and Studio.

## [0.7.0] - 2026-10-15

Adds merge stages, which combine several streams of the same event type into one.

### Breaking changes
- `StageType` has a new `Merge` variant. A `match` over every stage type needs a `Merge` arm, and code reading topology JSON must accept "merge" as a stage type.

### Upgrading from 0.6

Add the new arm wherever you match on every stage type:

```diff
     StageType::Join => "Join",
+    StageType::Merge => "Merge",
```

## [0.6.0] - 2026-10-06

A stage now lists every middleware attached to it, so it can show two rate limiters, one for each effect. Each setting also says where its value came from, such as the code or a config file.

### Breaking changes
- `MiddlewareInfo` holds a list of `attachments` instead of one optional field each for the circuit breaker, rate limiter and retry. `MiddlewareAttachmentInfo::family()` tells you which kind an attachment is.
- Each attachment has a key that must be unique within the topology. Make keys with `MiddlewareAttachmentKey` so they're always written the same way.
- Loading refuses malformed middleware settings, including a setting set to null or missing where its value came from.
- `Topology::replace_stage_info` returns a `Result`. When the new details aren't valid, the topology stays as it was.

### Upgrading from 0.5

```diff
-if let Some(limiter) = &middleware.rate_limiter {
-    println!("{}", limiter.tokens_per_sec);
-}
+for attachment in &middleware.attachments {
+    if let MiddlewareDetailsInfo::RateLimiter(limiter) = &attachment.details {
+        println!("{}", limiter.events_per_second().value);
+    }
+}
```

```diff
-topology.replace_stage_info(info);
+topology.replace_stage_info(info)?;
```

Topologies written by 0.5 don't load in 0.6, so upgrade the code that writes them and the code that reads them together.

## [0.5.1] - 2026-07-11

Although a patch release, 0.5.1 breaks topologies saved by 0.5.0 whose edges cross a composite's boundary.

### Breaking changes
- An edge that crosses a composite's boundary must name the port it uses, and the port must exist and match. Otherwise validation fails.

### Added
- `DirectedEdge::composite_ports` lists the composite ports an edge crosses. An edge between two composites lists both.

### Upgrading from 0.5.0

Upgrade ObzenFlow and Studio together, and rebuild any topology saved by 0.5.0.

## [0.5.0] - 2026-07-09

A topology can now describe a composite, such as a saga or a map-reduce job, as one named unit with ports where it connects to the rest of the flow.

### Breaking changes
- Build `StageSubgraphMembership`, `TopologySubgraphInfo` and `SubgraphInternalEdge` with their constructors. Struct literals no longer compile.

### Added
- Boundary ports show where a composite connects to the rest of the flow, through which member stages and with which event types.
- `StageSubgraphMembership::with_class()` marks a member's part in a composite, such as a saga's compensation stage.
- A composite's description carries a version number. Descriptions written by 0.4 still load.

### Changed
- An internal edge's `role` now names its lane, such as data or status. The JSON field is still called `role`.

### Upgrading from 0.4

```diff
-let edge = SubgraphInternalEdge {
-    from_stage_id: map,
-    to_stage_id: reduce,
-    role: "data".into(),
-};
+let edge = SubgraphInternalEdge::new(map, reduce, "data");
```

## 0.4.0 - 2026-05-10

ObzenFlow and Studio now share one topology format. ObzenFlow serves a flow's topology in it, and Studio reads it.

### Breaking changes
- `DirectedEdge` is no longer `Copy`. Call `.clone()` where you relied on copying an edge.
- Build `StageInfo` and `DirectedEdge` with their constructors. Struct literals no longer compile.
- Stage types, roles and edge kinds are written in lowercase with underscores in JSON, such as finite_source and forward.
- The free-form extension fields are removed: `StageInfo::extensions`, `StageExtensions` and `EdgeExtensions`. Use the typed fields instead, such as `StageInfo::middleware`, `DirectedEdge::contracts` and `DirectedEdge::typing`.
- `StageMetadata` is removed. Use `StageInfo`.
- `Shape::stage_type()` is removed. Use `StageType` directly.

### Added
- A whole topology can be saved as JSON and loaded back.
- Stages and edges carry typed details, such as the flow's name, each stage's status and role, its middleware, joins and contracts, and the event types on each edge.
- Helpers work out each stage's role, whether it sits in a feedback loop, and each edge's event type. `Topology::replace_stage_info` updates a stage's details after the topology is built.
- `TypeHintInfo::display_name()` shows an event type without its module path.

### Changed
- A flow name you set yourself now wins over one taken from the source stages.
- Two edges are equal when they join the same stages in the same direction, whatever else they carry.

### Upgrading from 0.3

```diff
-let edge = DirectedEdge { from, to, kind: EdgeKind::Forward };
+let edge = DirectedEdge::new(from, to, EdgeKind::Forward);
```

Topology JSON written by 0.3 spells stage types and edge kinds as FiniteSource and Forward. Rewrite them as finite_source and forward.

## [0.3.1] - 2026-03-01

This release changes no code. It updates the contributing guide, the trademark notice and the licence years, and contributors now sign off each commit under the Developer Certificate of Origin.

## 0.3.0 - 2026-02-20

### Added
- `Topology::scc_id` tells you which feedback loop a stage belongs to, and `Topology::scc_members` lists the stages in that loop. A stage outside any loop has no id, and a loop keeps its id when the same topology is built again.

## 0.2.0 - 2026-01-18

The first release on crates.io. Validation now checks that each connection makes sense for the stages it joins.

### Breaking changes
- `StageInfo::new` takes a `StageType`, and `DirectedEdge::new` takes an `EdgeKind`.
- `Topology::new` also checks each connection against its stages' types, and that every stage sits on a path from a source to a sink.

### Added
- Stage roles and edge kinds tell sources, processing stages and sinks apart, and mark edges that feed back upstream.
- `Topology::new_unvalidated()` and `TopologyBuilder::build_unchecked()` build a draft with only basic checks, and `Topology::validate_with_level()` validates it fully later.
- `Topology::source_stages()` and `Topology::sink_stages()` find a topology's sources and sinks. Validation errors say which connection is wrong, which source or sink is missing, and which stages can't reach a sink.
- `TopologyBuilder::add_stage_with_id` and `TopologyBuilder::add_edge_with_kind` add a stage with its type and an edge with its kind.

### Upgrading from 0.1

```diff
-let stage = StageInfo::new(id, "orders");
-let edge = DirectedEdge::new(from, to);
+let stage = StageInfo::new(id, "orders", StageType::FiniteSource);
+let edge = DirectedEdge::new(from, to, EdgeKind::Forward);
```

## 0.1.0 - 2025-08-14

The first version, extracted from ObzenFlow. It describes a flow's stages and how they connect, for both ObzenFlow and Studio, and it builds for WebAssembly.

### Added
- Stage and edge types, stage ids, and a builder for putting a topology together.
- Checks that every stage is connected, and cycle detection that allows feedback loops through several stages but refuses a stage connected to itself.

[Unreleased]: https://github.com/ObzenFlow/obzenflow-topology/compare/v0.8.0...HEAD
[0.8.0]: https://github.com/ObzenFlow/obzenflow-topology/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/ObzenFlow/obzenflow-topology/releases/tag/v0.7.0
[0.6.0]: https://github.com/ObzenFlow/obzenflow-topology/releases/tag/v0.6.0
[0.5.1]: https://github.com/ObzenFlow/obzenflow-topology/releases/tag/v0.5.1
[0.5.0]: https://github.com/ObzenFlow/obzenflow-topology/releases/tag/v0.5.0
[0.3.1]: https://github.com/ObzenFlow/obzenflow-topology/releases/tag/v0.3.1
