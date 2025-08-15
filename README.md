# obzenflow-topology

Flow topology graph structures for ObzenFlow - a pure Rust implementation of directed graph structures for building and validating flow-based pipelines with support for cycles and feedback loops.

## Features

- **Pure Data Structures**: No runtime dependencies, just graph topology
- **Type Safety**: Strongly typed stage identifiers and types
- **Cycle Support**: Allows multi-stage cycles for feedback loops and retry patterns
- **Validation**: Connectivity analysis, cycle detection, and topology validation
- **WASM Compatible**: Works in browser environments via WebAssembly
- **Dual Licensed**: MIT OR Apache-2.0

## Installation

```toml
[dependencies]
obzenflow-topology = "0.1.0"
```

For WASM targets:
```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
obzenflow-topology = "0.1.0"
```

## Usage

### Building a Topology

```rust
use obzenflow_topology::{TopologyBuilder, StageInfo, StageType, StageId};

let mut builder = TopologyBuilder::new();

// Add stages
let source_id = StageId::new();
let transform_id = StageId::new();
let sink_id = StageId::new();

builder.add_stage(StageInfo::new(source_id, "data_source"));
builder.add_stage(StageInfo::new(transform_id, "processor"));
builder.add_stage(StageInfo::new(sink_id, "output"));

// Connect stages
builder.add_edge(source_id, transform_id)?;
builder.add_edge(transform_id, sink_id)?;

// Build and validate
let topology = builder.build()?;
```

### Topology Analysis

```rust
use obzenflow_topology::{validate_acyclic, find_disconnected_stages};

// Check for cycles (note: multi-stage cycles are allowed, but self-cycles are forbidden)
if let Err(e) = validate_acyclic(&topology) {
    println!("Cycle detected: {}", e);
}

// Find disconnected stages
let disconnected = find_disconnected_stages(&topology);
if !disconnected.is_empty() {
    println!("Disconnected stages: {:?}", disconnected);
}
```

### Cycle Support

As of FLOWIP-082, topologies support multi-stage cycles to enable:
- **Feedback loops**: Output from downstream stages flowing back upstream
- **Retry patterns**: Sending failed events back for reprocessing  
- **Iterative processing**: Refining data through multiple passes

Note: Self-cycles (a stage connecting to itself) are explicitly forbidden.

### Stage Types

The crate provides comprehensive stage type classification:

```rust
use obzenflow_topology::StageType;

let stage_type = StageType::Transform;

// Query stage properties
if stage_type.consumes_events() {
    // Stage processes input events
}

if stage_type.generates_events() {
    // Stage produces output events
}
```

## Architecture

The crate is organized into the following modules:

- **types**: Core type definitions (StageId, StageType)
- **topology**: Graph structure and metrics
- **builder**: Fluent API for constructing topologies
- **validation**: Cycle detection and connectivity analysis
- **stages**: Stage information and metadata

## WASM Support

This crate is designed to work in WebAssembly environments, making it suitable for browser-based visualization and tooling:

```rust
// Works in both native and WASM targets
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen)]
pub fn create_topology() -> Topology {
    // Topology creation works identically in WASM
    TopologyBuilder::new().build().unwrap()
}
```

## Testing

Run the test suite:

```bash
cargo test
```

For WASM testing:
```bash
wasm-pack test --headless --firefox
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## License

This project is dual-licensed under either:

- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)

at your option.

## Credits

This crate is part of the ObzenFlow project, providing flow-based programming infrastructure for Rust applications.
