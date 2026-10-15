# ObzenFlow Topology

`obzenflow-topology` is the shared graph model for the ObzenFlow runtime and
Studio. Native Rust and WebAssembly applications use it to describe pipelines,
validate their wiring, and exchange the same topology as JSON.

- Model stages, directed connections, and composite subgraphs with named ports.
- Check that connections are valid and every stage lies on a source-to-sink path.
- Query neighbouring stages and identify feedback loops.
- Carry typed metadata for stage status, middleware, contracts, and event types.

Pipeline execution is handled by the ObzenFlow runtime.

## Install

```toml
[dependencies]
obzenflow-topology = "0.7"
```

Serde support is included. The crate works on native targets and
`wasm32-unknown-unknown` without requiring a random number generator. Stage IDs
come from your application or from serialized topology data.

## Quick start

Build and validate a source → transform → sink pipeline, then find the stage
that feeds the sink:

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

let topology = Topology::new(stages, edges).unwrap();
assert_eq!(topology.upstream_stages(sink), vec![transform]);
```

## Validation

`Topology::new` checks graph structure, legal connections between stage roles,
and reachability from sources to sinks. Multi-stage cycles are supported;
self-connections are rejected.

For draft graphs, `Topology::new_unvalidated` skips connection semantics and
reachability checks while still checking graph structure. Use
`validate_with_level(ValidationLevel::Full)` when the graph is ready for full
validation. See the [API documentation](https://docs.rs/obzenflow-topology) for
validation levels, graph queries, and metadata types.

## Project links

- [Crates.io](https://crates.io/crates/obzenflow-topology)
- [Changelog](CHANGELOG.md)
- [Contributing and development setup](https://github.com/ObzenFlow/obzenflow-topology/blob/main/CONTRIBUTING.md)
- [Code of Conduct](https://github.com/ObzenFlow/obzenflow-topology/blob/main/CODE_OF_CONDUCT.md)
- [Security](https://github.com/ObzenFlow/obzenflow-topology/blob/main/SECURITY.md)
- [Trademarks](TRADEMARKS.md)

## License

Dual-licensed under [MIT](LICENSE-MIT) OR [Apache-2.0](LICENSE-APACHE).
