# ObzenFlow Topology

`obzenflow-topology` is an opinionated, WASM-friendly crate for representing and validating flow/pipeline graphs. It was extracted from ObzenFlow so the native backend and the browser UI can share the exact same graph rules.

* Shared contract: same `Topology` + `TopologyError` on server and UI.
* Validation at three levels: structural, semantic, and reachability.
* Cycle-aware: supports multi-stage feedback loops and retry/backflow edges (self-cycles rejected).
* Fast queries: cached upstream/downstream adjacency lists.
* Deterministic: stable fingerprinting for caching and UI diffing.
* Type-safe IDs: phantom-typed ULID `StageId` via `obzenflow-idkit` (no RNG required by default).

## Why this exists

ObzenFlow is full-stack Rust: a native backend (`obzenflow`) plus a WASM UI (`obzenflow-studio`). Both sides need to answer the same questions about a flow graph:

* “Is this wiring valid?”
* “What feeds into this stage?”
* “Is this stage part of a feedback loop?”
* “Can every stage reach a sink?”

Rather than duplicating logic across languages/targets, this crate compiles into both and becomes the single source of truth.

```
obzenflow (native)  <---- JSON (stages/edges) ---->  obzenflow-studio (wasm)
         \                                         /
          \----------- obzenflow-topology ---------/
```

## What this crate provides

### 1) Validation

`Topology::new(...)` performs full validation up front. For UI/editor workflows you can opt into structural-only construction and validate later.

Validation is split into:

* **Structural**: edge endpoints exist, no duplicates, no self-cycles, single connected component.
* **Semantic**: enforces legal connections based on `StageType` → `StageRole` and `EdgeKind` (`|>` vs `<|`).
* **Reachability**: requires at least one Producer and one Consumer, and every stage is on some producer → consumer path.

### 2) Graph queries

Once built, you get cheap queries:

* `upstream_stages` / `downstream_stages`
* `edges()` and stage metadata lookup
* `is_in_cycle` (SCC-based)
* `metrics()` and `topology_fingerprint()`

### 3) Visualization helpers

The crate includes simple port/shape types (`PortId`, `Shape`) that UIs can use as building blocks when turning graph structure into visuals.

> Non-goal: This crate does not execute pipelines; it’s a value type + validation/query layer.

## Install

Serde support is included by default for round-tripping stage/edge data through JSON:

```toml
[dependencies]
obzenflow-topology = "0.2"
```

The same dependency works for `wasm32-unknown-unknown` (no RNG required).

## Quick start

```rust
use obzenflow_topology::{DirectedEdge, EdgeKind, StageId, StageInfo, StageType, Topology};

let source: StageId = "01ARZ3NDEKTSV4RRFFQ69G5FAV".parse().unwrap();
let transform: StageId = "01ARZ3NDEKTSV4RRFFQ69G5FAW".parse().unwrap();
let sink: StageId = "01ARZ3NDEKTSV4RRFFQ69G5FAX".parse().unwrap();

let stages = vec![
    StageInfo::new(source, "source", StageType::FiniteSource),
    StageInfo::new(transform, "transform", StageType::Transform),
    StageInfo::new(sink, "sink", StageType::Sink),
];

let edges = vec![
    DirectedEdge::new(source, transform, EdgeKind::Forward),
    DirectedEdge::new(transform, sink, EdgeKind::Forward),
];

// Full validation happens here (structural + semantic + reachability).
let topology = Topology::new(stages, edges).unwrap();

// Queries are cheap (adjacency lists are cached).
let upstream_of_sink = topology.upstream_stages(sink);
let in_cycle = topology.is_in_cycle(transform);
let fingerprint = topology.topology_fingerprint();
let metrics = topology.metrics();
```

## Validation levels

If you’re building an interactive editor, you often want to accept “draft” graphs and validate on demand:

```rust
use obzenflow_topology::{Topology, ValidationLevel};

// `stages`/`edges` as in the Quick start example.
let draft = Topology::new_unvalidated(stages, edges).unwrap();

// Validate later (semantic only, or full).
draft.validate_with_level(ValidationLevel::Semantic).unwrap();
```

`ValidationLevel`:

* `Structural`: endpoints, duplicates, self-cycles, disconnected components
* `Semantic`: structural + `(StageRole, EdgeKind)` connection rules
* `Full`: semantic + reachability invariants (sources/sinks, producer→sink paths)

## Cycles & backflow

ObzenFlow allows multi-stage cycles for feedback loops and retry patterns. Cycles are represented explicitly with `EdgeKind::Backward` (`<|`) edges; self-cycles are rejected.

Semantic rules are intentionally restrictive (high level):

* Forward (`|>`): Producer/Processor → Processor/Consumer
* Backward (`<|`): Consumer/Processor → Processor

Use `topology.is_in_cycle(stage)` when you need to render or reason about feedback loops.

## IDs, ULIDs, `obzenflow-idkit`, and RNG

Topology IDs are ULIDs, wrapped in a phantom type for safety:

* `StageId` is `obzenflow_idkit::Id<Stage>` (a phantom-typed `ulid::Ulid`).
* This crate depends on `obzenflow-idkit` without its `gen` feature, so it does not require an RNG.
* In practice, IDs usually come from your domain layer (backend) or from parsing API payloads (UI).

If your application wants to generate IDs, do it in the app crate:

```toml
[dependencies]
obzenflow-idkit = { version = "0.2", features = ["gen", "serde"] }
getrandom = { version = "0.2", features = ["js"] } # browser wasm only
```

## Algorithms & data structures (implementation notes)

This crate is intentionally “boring” and predictable:

* Stores stages in a `HashMap<StageId, StageInfo>` and edges in a `Vec<DirectedEdge>`.
* Builds cached adjacency lists (`HashMap<StageId, HashSet<StageId>>`) for both downstream and upstream traversal.
* Uses Tarjan SCC to compute cycle membership (`is_in_cycle`) in `O(V + E)`.
* Structural validation is a single pass over edges plus a connectivity check (`O(V + E)`).
* Full validation adds reachability checks to ensure every stage is on a producer → consumer path.
* `topology_fingerprint()` sorts IDs/edges by raw ULID bytes to produce a stable `u64` across runs/targets.

## Testing (no RNG)

Keep tests deterministic by synthesizing `StageId`s from a counter:

```rust
use std::sync::atomic::{AtomicU64, Ordering};
use obzenflow_topology::StageId;

static CTR: AtomicU64 = AtomicU64::new(0);

fn next_stage_id() -> StageId {
    let n = CTR.fetch_add(1, Ordering::Relaxed);
    let mut bytes = [0u8; 16];
    bytes[8..].copy_from_slice(&n.to_be_bytes());
    StageId::from_bytes(bytes)
}
```

## Testing

```bash
cargo test
```

## Project links

* Changelog: `CHANGELOG.md`
* Contributing: `CONTRIBUTING.md`

## Project policies

* Code of Conduct: `CODE_OF_CONDUCT.md`
* Security: `SECURITY.md`
* Trademarks: `TRADEMARKS.md`

## License

Dual-licensed under MIT OR Apache-2.0.
