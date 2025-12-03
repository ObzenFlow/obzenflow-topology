# FLOWIP-TOP-001: Topology Validation and Enforcement – Shared Topology Core

## Summary

This document describes the intended **Phase 1** of topology validation/enforcement work for the shared `obzenflow-topology` crate.

- The published `obzenflow-topology` **0.1.0** release implements the structural behavior described in **Current Implementation** below.
- The unreleased **0.2.0 (main branch)** implements most of the Phase 1 design for StageType/StageRole + EdgeKind semantics and the new validation API. See “Implementation Status vs Design” for details on what is done vs pending.

Goal:
- Enrich the core `Topology` model so it can reject semantically invalid graphs (e.g. `sink |> source`) while still allowing the backflow/cycle patterns introduced in FLOWIP‑082.
- Keep all semantics **local to this crate** and independent of runtime/contract details, so both the backend and the UI can rely on the same source of truth.

Phase 2 (to be implemented in the `flowstate_rs` repo) will add **runtime/DSL‑level validation** that uses these semantics together with contracts and source configuration.

### Cross-Codebase Review

This design is informed by a deep review of three codebases:

1. **`obzenflow-topology`** (this crate) – current structural validation, `StageType`, `Shape`
2. **`flowstate_rs`** (server/runtime) – `obzenflow_runtime_services` FSMs for Transform, Stateful, and Join stages; `obzenflow_core::event::context::StageType`
3. **`obzen-flow-ui`** (visualization) – `NodeType`, `Shape` inference from connections, `VisualTopology`, layout algorithms

Key findings from this review shaped the design decisions in this document, particularly:
- The distinction between **StageRole** (connection semantics) vs **StageType** (runtime behavior) vs **Shape** (visual port layout)
- The need for extensibility to support future middleware/contract visualization in the UI
- The runtime differences between Transform (stateless), Stateful (accumulator), and Join (multi-input correlator) that justify keeping them as distinct `StageType` variants

**Key term:** `StageRole` is a derived classification used purely for connection semantics validation, with three variants: `Producer` (sources), `Processor` (transform-like stages), and `Consumer` (sinks). It is distinct from `StageType` (which determines runtime FSM behavior) and `Shape` (which determines visual port layout).

## Current Implementation (pre‑Phase 1)

### Data Structures

- `StageInfo { id: StageId, name: String }`
  - Carries no `StageType`; only ID and a human‑readable name.
- `DirectedEdge { from: StageId, to: StageId }`
  - No indication whether the edge came from `|>` (forward) or `<|` (backflow).
- `StageMetadata { id: StageId, name: String, stage_type: StageType, description: Option<String> }`
  - Defined in `src/stages/stage_info.rs` and exported from the crate.
  - Used as a serializable metadata container; **not** currently used by `Topology` or `TopologyBuilder` for validation.
- `Topology`
  - `stages: HashMap<StageId, StageInfo>`
  - `edges: Vec<DirectedEdge>`
  - `downstream` / `upstream` adjacency lists
  - `stages_in_cycles` computed via Tarjan’s algorithm (FLOWIP‑082g)

### Validation

- Ensures:
  - All edge endpoints reference known stages.
  - No duplicate edges.
  - Self‑cycles are rejected.
  - Disconnected stages are identified (`DisconnectedStages`).
  - Multi‑stage cycles are allowed; `stages_in_cycles` is populated via `compute_sccs`.
- Does **not** enforce:
  - Stage‑type semantics (e.g. “sinks can’t produce” or “sources can’t consume”).
  - Operator semantics (`|>` vs `<|`).
  - Structural requirements such as “at least one source and one sink” or “all stages are on some path from a source to a sink”.
  - Any logic based on `StageMetadata` or `StageType`; these types exist but are not consulted by `Topology::new`.

As a result, graphs like `sink |> source`, `source1 |> source2`, or “no sinks at all” are currently allowed.

## Phase 1 Design: Stage‑Type‑Aware Validation

*Status: design‑only. The items in this section are **not yet implemented** as of `obzenflow-topology` 0.1.0. Where code snippets appear, they represent the intended target API/behavior rather than the current implementation. See “Implementation Status vs Design” below for a concise checklist.*

### 1. StageType on StageInfo

**File:** `src/stages/stage_info.rs`

Add `StageType` to `StageInfo` so the core graph knows what each stage is:

```rust
use super::StageId;
use crate::types::StageType;

#[derive(Debug, Clone)]
pub struct StageInfo {
    pub id: StageId,
    pub name: String,
    pub stage_type: StageType,
}

impl StageInfo {
    pub fn new(id: StageId, name: impl Into<String>, stage_type: StageType) -> Self {
        Self {
            id,
            name: name.into(),
            stage_type,
        }
    }

    pub fn auto_named(id: StageId, stage_type: StageType) -> Self {
        Self {
            id,
            name: format!("stage_{}", id),
            stage_type,
        }
    }
}
```

`StageType` already exists in `src/types/stage_type.rs`. In the shared `obzenflow_core` definition, `StageType` covers `FiniteSource`, `InfiniteSource`, `Transform`, `Sink`, `Stateful`, and `Join`. **Note:** In this crate, `StageType` currently omits `Join`; version 0.2 will align it with the `obzenflow_core` definition.

For validation purposes, we introduce **`StageRole`** (replacing the previous `SimpleStageType`) to classify stages by their **connection semantics** rather than runtime behavior:

```rust
/// Connection role for topology validation.
/// Distinct from StageType (runtime behavior) and Shape (visual port layout).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum StageRole {
    /// Produces events only (Sources) - no incoming forward edges allowed
    Producer,
    /// Consumes and produces events (Transform, Stateful, Join) - can receive and send
    Processor,
    /// Consumes events only (Sinks) - no outgoing forward edges allowed
    Consumer,
}

impl StageType {
    /// Get the connection role for this stage type.
    /// Used for topology validation; distinct from runtime behavior.
    pub fn role(&self) -> StageRole {
        match self {
            StageType::FiniteSource | StageType::InfiniteSource => StageRole::Producer,
            StageType::Transform | StageType::Stateful | StageType::Join => StageRole::Processor,
            StageType::Sink => StageRole::Consumer,
        }
    }
}
```

**Why `StageRole` instead of `SimpleStageType::Transform`?**

The cross-codebase review revealed that calling Stateful and Join stages "transforms" is semantically misleading:

| Stage Type | Runtime Behavior | FSM States | Handler Pattern |
|------------|------------------|------------|-----------------|
| Transform | Stateless, immediate 1:N | Running → Draining | `process(&self, event)` |
| Stateful | Accumulator, windowed emission | Accumulating → Emitting → Draining | `accumulate(&mut self, state, event)` |
| Join | Multi-input correlator, phased | Hydrating → Enriching → Draining | 2 subscriptions, reference-first |

These are fundamentally different at runtime, but share the same **connection role**: they all consume upstream events and produce downstream events. The term "Processor" accurately describes this role without implying statelessness or single-input semantics.

### 2. TopologyBuilder: Stage-Type Aware

**File:** `src/builder/builder.rs`

Update `TopologyBuilder::add_stage_with_id` to accept `StageType` and construct a full `StageInfo`:

```rust
pub fn add_stage_with_id(
    &mut self,
    id: StageId,
    name: Option<String>,
    stage_type: StageType,
) -> StageId {
    let info = match name {
        Some(n) => StageInfo::new(id, n, stage_type),
        None => StageInfo::auto_named(id, stage_type),
    };

    self.stages.push(info);

    if let Some(from) = self.current_stage {
        self.edges.push(DirectedEdge::new(from, id, EdgeKind::Forward));
    }

    self.current_stage = Some(id);
    id
}
```

The test‑only `add_stage` helper can supply a default `StageType` (e.g. `Transform`) to keep tests simple.

### 3. EdgeKind: Preserve `|>` vs `<|`

**File:** `src/topology/edge.rs`

Add an `EdgeKind` enum and extend `DirectedEdge` with a `kind` field. Both must remain serializable so callers can persist and round‑trip graphs:

```rust
use crate::stages::StageId;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EdgeKind {
    Forward,   // a |> b
    Backward,  // a <| b (backflow)
}

/// Directed edge - explicit flow direction between stages
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DirectedEdge {
    pub from: StageId,
    pub to: StageId,
    pub kind: EdgeKind,
}

impl DirectedEdge {
    pub fn new(from: StageId, to: StageId, kind: EdgeKind) -> Self {
        Self { from, to, kind }
    }
}

impl std::fmt::Display for EdgeKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EdgeKind::Forward => write!(f, "|>"),
            EdgeKind::Backward => write!(f, "<|"),
        }
    }
}
```

The DSL and any other callers will use `EdgeKind::Forward` for `|>` edges and `EdgeKind::Backward` for `<|` edges. Phase 1 does not change external consumers yet; it just defines the shape.

### 4. Connection Semantics (StageRole + EdgeKind)

**File:** `src/validation/validation.rs`

Add a function that validates each edge against StageRole + EdgeKind:

```rust
use crate::stages::StageInfo;
use crate::topology::EdgeKind;
use crate::types::StageRole;

pub fn validate_connection_semantics(
    from: &StageInfo,
    to: &StageInfo,
    kind: EdgeKind,
) -> Result<(), TopologyError> {
    use StageRole::*;

    let from_role = from.stage_type.role();
    let to_role = to.stage_type.role();

    match (from_role, to_role, kind) {
        // Forward connections (|>)
        (Producer, Processor, EdgeKind::Forward) => Ok(()),
        (Producer, Consumer, EdgeKind::Forward) => Ok(()),
        (Processor, Processor, EdgeKind::Forward) => Ok(()),
        (Processor, Consumer, EdgeKind::Forward) => Ok(()),

        // Backward connections (<|) for FLOWIP-082 cycles and retry patterns
        (Processor, Processor, EdgeKind::Backward) => Ok(()),
        (Consumer, Processor, EdgeKind::Backward) => Ok(()),

        // Explicit invalid backward combinations for clarity
        (Processor, Producer, EdgeKind::Backward) => Err(TopologyError::InvalidConnection {
            from: from.id,
            from_name: from.name.clone(),
            from_type: from.stage_type,
            to: to.id,
            to_name: to.name.clone(),
            to_type: to.stage_type,
            operator: "<|".into(),
            reason: "Cannot route data into a producer (source) via <| operator".into(),
        }),
        (Consumer, Consumer, EdgeKind::Backward) => Err(TopologyError::InvalidConnection {
            from: from.id,
            from_name: from.name.clone(),
            from_type: from.stage_type,
            to: to.id,
            to_name: to.name.clone(),
            to_type: to.stage_type,
            operator: "<|".into(),
            reason: "Backflow is only allowed into processor stages (transform/stateful/join)".into(),
        }),
        (Consumer, Producer, EdgeKind::Backward) => Err(TopologyError::InvalidConnection {
            from: from.id,
            from_name: from.name.clone(),
            from_type: from.stage_type,
            to: to.id,
            to_name: to.name.clone(),
            to_type: to.stage_type,
            operator: "<|".into(),
            reason: "Producers cannot consume data; consumers cannot send backflow into producers".into(),
        }),

        // Invalid: producers can't consume via backflow
        (Producer, _, EdgeKind::Backward) => Err(TopologyError::InvalidConnection {
            from: from.id,
            from_name: from.name.clone(),
            from_type: from.stage_type,
            to: to.id,
            to_name: to.name.clone(),
            to_type: to.stage_type,
            operator: "<|".into(),
            reason: "Producers (sources) cannot consume data via <| operator".into(),
        }),

        // Invalid: can't pipe into a producer via forward edges
        (_, Producer, EdgeKind::Forward) => Err(TopologyError::InvalidConnection {
            from: from.id,
            from_name: from.name.clone(),
            from_type: from.stage_type,
            to: to.id,
            to_name: to.name.clone(),
            to_type: to.stage_type,
            operator: "|>".into(),
            reason: "Cannot pipe data into a producer (source)".into(),
        }),

        // Invalid: consumers can't produce forward edges
        (Consumer, _, EdgeKind::Forward) => Err(TopologyError::InvalidConnection {
            from: from.id,
            from_name: from.name.clone(),
            from_type: from.stage_type,
            to: to.id,
            to_name: to.name.clone(),
            to_type: to.stage_type,
            operator: "|>".into(),
            reason: "Consumers (sinks) cannot produce data via |> operator".into(),
        }),

        // All other combinations are invalid
        _ => Err(TopologyError::InvalidConnection {
            from: from.id,
            from_name: from.name.clone(),
            from_type: from.stage_type,
            to: to.id,
            to_name: to.name.clone(),
            to_type: to.stage_type,
            operator: kind.to_string(),
            reason: format!(
                "Invalid connection: {} ({}) {} {} ({})",
                from.stage_type, from_role, kind, to.stage_type, to_role
            ),
        }),
    }
}
```

This makes backflow explicit and constrained:

- **Legal backflow:** `Consumer → Processor`, `Processor → Processor`. Note that `Processor → Processor` backflow covers all three transform-like types (Transform, Stateful, Join); Phase 2 can selectively tighten the rules for joins if needed.
- **Illegal:** Any case where a `Producer` appears as a consumer (right side of an edge) or a `Consumer` appears as a producer (left side of a forward edge), as reflected in the table below. All invalid combinations are explicitly rejected with tailored error messages.
  A later runtime‑level validator (Phase 2) can add additional rules such as "no backflow directly into a join".

For quick reference, the core connection rules by **StageRole**:

| From      | To        | Forward (`|>`) | Backward (`<|`) |
|-----------|-----------|----------------|-----------------|
| Producer  | Producer  | ❌              | ❌               |
| Producer  | Processor | ✅              | ❌               |
| Producer  | Consumer  | ✅              | ❌               |
| Processor | Producer  | ❌              | ❌               |
| Processor | Processor | ✅              | ✅               |
| Processor | Consumer  | ✅              | ❌               |
| Consumer  | Producer  | ❌              | ❌               |
| Consumer  | Processor | ❌              | ✅               |
| Consumer  | Consumer  | ❌              | ❌               |

**StageRole mapping:**
- **Producer** = `StageType::FiniteSource`, `StageType::InfiniteSource`
- **Processor** = `StageType::Transform`, `StageType::Stateful`, `StageType::Join`
- **Consumer** = `StageType::Sink`

This mapping is the only place topology validation cares about `StageType` variants; all per-type behavioral differences (stateless vs accumulator vs multi-input correlator) stay in the runtime layer.

Runtime‑level validation in `flowstate_rs` may impose stricter rules for specific stage types (e.g., disallowing direct backflow into joins) based on handler/join descriptors. Phase 1 validates by role; Phase 2 can add type-specific constraints.

### 5. Structural Topology Constraints

**File:** `src/validation/validation.rs`

Enforce basic structural invariants:

- At least one `StageType::FiniteSource` or `StageType::InfiniteSource` → `TopologyError::NoSources`.
- At least one `StageType::Sink` → `TopologyError::NoSinks`.
- All stages reachable from some source → otherwise `TopologyError::UnreachableStages`.
- All stages can reach some sink → otherwise `TopologyError::UnproductiveStages`.
  Together these imply:
  - Every stage participating in the flow lies on at least one path from a semantic source (`StageType::is_source()`) to a semantic sink (`StageType::is_terminal()`).
  - No isolated components remain once purely disconnected stages are taken into account.

This replaces “purely disconnected” checks with a stricter notion of “not on any source→sink path”.

### 6. Wire Validation into Topology::new

**File:** `src/topology/topology.rs`

After building `stage_map`, `downstream`, and `upstream`:

```rust
// 1. Semantic connection validation
for edge in &edges {
    let from = stage_map.get(&edge.from).expect("validated above");
    let to = stage_map.get(&edge.to).expect("validated above");
    crate::validation::validate_connection_semantics(from, to, edge.kind)?;
}

// 2. Structural constraints
crate::validation::validate_topology_structure(&stage_map, &downstream, &upstream)?;
```

Self‑cycle checks and SCC computation remain as they are; this phase only layers additional semantics on top.

### 7. Error Types

**File:** `src/validation/validation.rs`

Extend `TopologyError` with semantic validation variants:

```rust
#[derive(Debug, thiserror::Error)]
pub enum TopologyError {
    // existing variants...

    #[error(
        "Invalid connection: Cannot connect {from_type} '{from_name}' to \
         {to_type} '{to_name}' via {operator} operator. {reason}"
    )]
    InvalidConnection {
        from: StageId,
        from_name: String,
        from_type: StageType,
        to: StageId,
        to_name: String,
        to_type: StageType,
        operator: String,
        reason: String,
    },

    #[error("Topology must have at least one source stage")]
    NoSources,

    #[error("Topology must have at least one sink stage")]
    NoSinks,

    #[error(
        "Stages unreachable from any source: {}",
        stages.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
    )]
    UnreachableStages {
        stages: Vec<StageId>,
    },

    #[error(
        "Stages that cannot reach any sink: {}",
        stages.iter().map(|s| s.to_string()).collect::<Vec<_>>().join(", ")
    )]
    UnproductiveStages {
        stages: Vec<StageId>,
    },
}
```

### 8. Structural vs Semantic Source/Sink Queries

**File:** `src/topology/topology.rs`

To avoid overloading the notion of “source” and “sink”, Phase 1 distinguishes:

- **Structural** sources/sinks (based purely on graph shape, as in 0.1).
- **Semantic** sources/sinks (based on `StageType`).

Existing methods remain structural:

- `source_stages()` → stages with no upstream edges.
- `sink_stages()` → stages with no downstream edges.

New helpers provide semantic views:

```rust
impl Topology {
    /// Semantic sources: stages whose StageType generates events
    pub fn semantic_source_stages(&self) -> Vec<StageId> {
        self.stages
            .iter()
            .filter(|(_, info)| info.stage_type.is_source())
            .map(|(id, _)| *id)
            .collect()
    }

    /// Semantic sinks: stages whose StageType is terminal
    pub fn semantic_sink_stages(&self) -> Vec<StageId> {
        self.stages
            .iter()
            .filter(|(_, info)| info.stage_type.is_terminal())
            .map(|(id, _)| *id)
            .collect()
    }
}
```

Backflow edges (`EdgeKind::Backward`) do not change semantic roles: a `StageType::Sink` remains a sink even if it participates in backflow cycles.

### 9. Validation API and Levels

**File:** `src/topology/topology.rs`

Phase 1 introduces an explicit validation API so callers (especially UIs) can choose when and how strictly to validate. The levels build on each other:

```rust
pub enum ValidationLevel {
    /// Structural only: endpoints, duplicates, self-cycles, disconnected, SCCs
    Structural,
    /// Adds StageType + EdgeKind connection semantics
    Semantic,
    /// Structural + semantic + source/sink reachability
    Full,
}

impl Topology {
    pub fn validate_with_level(&self, level: ValidationLevel) -> Result<(), TopologyError> {
        match level {
            ValidationLevel::Structural => {
                // Edge endpoint existence, duplicate edges, self-cycles, disconnected stages,
                // and SCC computation (cycles are allowed, but structure must be consistent).
                crate::validation::validate_edges_and_structure(self)
            }
            ValidationLevel::Semantic => {
                // Structural + StageType/EdgeKind connection semantics
                crate::validation::validate_edges_and_structure(self)?;
                crate::validation::validate_all_connections(self)
            }
            ValidationLevel::Full => {
                // Semantic + structural invariants (NoSources, NoSinks, UnreachableStages, UnproductiveStages)
                self.validate_with_level(ValidationLevel::Semantic)?;
                crate::validation::validate_topology_structure(self)
            }
        }
    }

    pub fn validate_semantics(&self) -> Result<(), TopologyError> {
        self.validate_with_level(ValidationLevel::Full)
    }
}
```

Topology construction follows a two‑step pattern. Both constructors enforce **basic structural integrity** (no dangling endpoints, no duplicates, no self‑cycles) so that core APIs remain safe to call; semantic and reachability validation are layered on afterwards:

```rust
impl Topology {
    /// Construction with structural validation only.
    ///
    /// "Unvalidated" means: not semantically or reachability validated.
    /// Structural invariants (valid endpoints, no duplicates, no self-cycles) still hold.
    ///
    /// Implementation note: this constructor runs structural checks internally
    /// (equivalent to ValidationLevel::Structural); callers can rely on basic
    /// graph consistency but must not assume semantic correctness.
    pub fn new_unvalidated(stages: Vec<StageInfo>, edges: Vec<DirectedEdge>) -> Self {
        // build stage_map, downstream, upstream, stages_in_cycles
        // enforce structural invariants (endpoints, duplicates, self-cycles)
    }

    /// Convenience constructor that applies full validation by default.
    pub fn new(stages: Vec<StageInfo>, edges: Vec<DirectedEdge>) -> Result<Self, TopologyError> {
        let topo = Self::new_unvalidated(stages, edges);
        topo.validate_with_level(ValidationLevel::Full)?;
        Ok(topo)
    }
}
```

The builder mirrors this:

```rust
impl TopologyBuilder {
    pub fn build_unchecked(self) -> Result<Topology, TopologyError> {
        Topology::new_unvalidated(self.stages, self.edges)
    }

    pub fn build(self) -> Result<Topology, TopologyError> {
        Topology::new(self.stages, self.edges)
    }
}
```

This allows UI workflows to construct graphs with structural validation only (via `build_unchecked`) and apply semantic validation on demand, while production code continues to call `build()` and get fully validated topologies. Note that `build_unchecked` still enforces structural invariants (valid endpoints, no duplicates, no self-cycles, no disconnected stages) — it only skips semantic validation (StageRole connection rules) and reachability checks (NoSources, NoSinks, UnreachableStages, UnproductiveStages).

### 10. Verification: Tests Exercising Phase 1 Semantics

The 0.2.0 codebase includes a dedicated semantic validation test suite in `tests/topology_semantic_validation.rs` alongside existing structural tests:

- Structural tests (`tests/topology_edge_cases.rs`, `tests/topology_validation_tests.rs`):
  - Use `TopologyBuilder::build_unchecked()` to exercise structural behavior only (empty topologies, cycles allowed, duplicates, disconnected stages, metrics, naming).
  - Confirm that structural invariants behave as designed without StageType/StageRole semantics.
- Semantic tests (`tests/topology_semantic_validation.rs`):
  - Use `Topology::new(...)` (full validation) to assert:
    - Valid topologies such as `FiniteSource -> Transform -> Sink` pass and are correctly classified by `semantic_source_stages()` / `semantic_sink_stages()`.
    - Invalid connections (e.g., `Sink |> Source`) produce `TopologyError::InvalidConnection` with the expected operator, names, and roles.
    - Topologies with no Producers / no Consumers produce `NoSources` / `NoSinks`.
    - Topologies with unreachable stages produce `UnreachableStages`.
    - Topologies with stages that cannot reach any Consumer produce `UnproductiveStages`.

The full `cargo test` suite passes, confirming that both structural and semantic aspects of the Phase 1 design are implemented and covered by tests in 0.2.0.

### 11. Cycle Detection and Fingerprints

**Files:** `src/validation/validation.rs`, `src/topology/topology.rs`

Cycle detection (`compute_sccs`) continues to treat all edges uniformly, regardless of `EdgeKind`. It answers the structural question “is there a directed cycle?” and is not responsible for distinguishing “good” backflow cycles from problematic ones. That distinction is handled by the semantic validators.

The existing `TopologyError::CycleDetected` and `validate_acyclic` helpers remain available as **optional utilities** for callers that need to assert acyclicity (e.g., for specific flows or debugging). They are not used by default in Phase 1 validation levels, since multi‑stage cycles are an accepted topology pattern after FLOWIP‑082.

To keep topology fingerprints aligned with semantics:

- `topology_fingerprint()` in 0.2 includes:
  - `StageId`, `StageInfo.name`, and `StageInfo.stage_type` for each stage.
  - `from`, `to`, and `kind: EdgeKind` for each edge.
- Optionally, a separate `structural_fingerprint()` can be introduced that hashes only IDs/names/edges, matching the 0.1 behavior when callers want to ignore semantics.

### 12. Shape vs StageType vs StageRole: Three Orthogonal Concepts

The cross-codebase review revealed that three concepts are often conflated but are actually **orthogonal**:

| Concept | Purpose | Determined By | Used For |
|---------|---------|---------------|----------|
| **StageRole** | Connection semantics | `StageType::role()` | Topology validation (this doc) |
| **StageType** | Runtime behavior | Stage definition | FSM selection, handler traits |
| **Shape** | Visual port layout | Connection count inference | UI layout algorithms |

**Shape** is already defined in `src/topology/shape.rs` and inferred by the UI based on connection counts:

```rust
// From obzen-flow-ui VisualBuilder::infer_shape()
match (input_count, output_count) {
    (0, _) => Shape::Source { ... },
    (_, 0) => Shape::Sink { ... },
    (1, 1) => Shape::Flow { ... },
    (2.., 1) => Shape::Merge { ... },
    (1, 2..) => Shape::Broadcast { ... },
    _ => Shape::Flow { ... },
}
```

This means a `StageType::Join` has `Merge`-like Shape (2 inputs) but fundamentally different runtime behavior. The UI correctly infers Shape from topology structure; it does not need to know `StageType` for port layout purposes.

**Implications for this design:**
- `StageRole` is for **validation** (can this connection exist?)
- `StageType` is for **runtime** (how does this stage behave?)
- `Shape` is for **visualization** (how many ports, where do they go?)

The topology crate exports all three, allowing consumers to use the appropriate abstraction for their needs.

### 13. Extension Points for Future Metadata

To support future UI features (middleware configuration display, contract visualization), `StageInfo` and `DirectedEdge` should be extensible without breaking changes:

**File:** `src/stages/stage_info.rs`

```rust
use serde::{Serialize, Deserialize};

/// Extensible stage information
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StageInfo {
    pub id: StageId,
    pub name: String,
    pub stage_type: StageType,

    /// Extension point for additional metadata (middleware, UI hints, etc.)
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub extensions: Option<StageExtensions>,
}

/// Future-proofing: extensible metadata container
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct StageExtensions {
    /// Middleware configuration (rate limiters, circuit breakers, retry policies)
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub middleware: Option<serde_json::Value>,

    /// UI-specific hints (custom icons, colors, grouping)
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub ui_hints: Option<serde_json::Value>,
}
```

**File:** `src/topology/edge.rs`

```rust
/// Extension point for edge metadata
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct EdgeExtensions {
    /// Contract configuration between stages
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub contract: Option<serde_json::Value>,

    /// UI-specific hints (edge styling, animation parameters)
    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub ui_hints: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct DirectedEdge {
    pub from: StageId,
    pub to: StageId,
    pub kind: EdgeKind,

    #[cfg_attr(feature = "serde", serde(default, skip_serializing_if = "Option::is_none"))]
    pub extensions: Option<EdgeExtensions>,
}
```

Using `serde_json::Value` keeps the topology crate decoupled from specific middleware/contract types while allowing the UI and runtime to attach rich metadata. Phase 2 may introduce typed extension fields as patterns stabilize.

**Dependency note:** Using `serde_json::Value` implies adding a `serde_json` dependency (the crate currently only uses `serde`). This also ties extensions to JSON even if consumers prefer other formats (e.g., bincode in WASM). Consider making `serde_json` a feature-gated dependency (e.g., `extensions` or `json-extensions` feature) if format flexibility is important.

**Relationship to StageMetadata:** The existing `StageMetadata` struct in `src/stages/stage_info.rs` is a serializable container that includes `stage_type`. In 0.2, `StageInfo` gains `stage_type` directly and becomes the primary carrier. `StageMetadata` may be:
- Deprecated in favor of `StageInfo` + `StageExtensions`, or
- Retained as a public, stable "full metadata" type that wraps `StageInfo` with additional fields.

The recommended approach is to migrate consumers to `StageInfo` and use `StageExtensions` for optional metadata, deprecating `StageMetadata` over time.

## Implementation Status vs Design

This section tracks how far the Phase 1 design has been implemented.

### 0.1.0 (published crate)

The 0.1.0 release implements only the behavior described under **Current Implementation (pre‑Phase 1)**:

- Structural validation only (endpoints, duplicates, self‑cycles, disconnected).
- No `StageType` on `StageInfo`, no `EdgeKind`, no semantic validation, no structural invariants.

### 0.2.0 (main branch, unreleased)

- **StageRole (renamed from SimpleStageType)**
  - Design: Introduce `StageRole` enum with `Producer`, `Processor`, `Consumer` variants. Rename `simple()` method to `role()`.
  - Implementation: **Done.**
    - `StageRole` exists and is used by `StageType::role()` to classify `StageType` into Producer/Processor/Consumer.
    - `SimpleStageType` and `simple()` have been removed in favor of `StageRole` in this crate.

- **StageType on StageInfo**
  - Design: `StageInfo` gains a `stage_type: StageType` field and constructors accept a `StageType`.
  - Implementation: **Done.**
    - `StageInfo` is now `{ id, name, stage_type, extensions: Option<StageExtensions> }`, and all callers construct it with an explicit `StageType`.

- **TopologyBuilder: stage‑type aware**
  - Design: `TopologyBuilder::add_stage_with_id` takes a `StageType` and constructs a full `StageInfo`.
  - Implementation: **Done.**
    - `add_stage_with_id` signature is `(StageId, Option<String>, StageType)`.
    - The convenience `add_stage` used in tests generates an ID and defaults to `StageType::Transform`.

- **EdgeKind on DirectedEdge**
  - Design: introduce `EdgeKind` and add `kind: EdgeKind` to `DirectedEdge` so the graph preserves `|>` vs `<|`.
  - Implementation: **Done.**
    - `EdgeKind` exists with `Forward` and `Backward` variants, serde derives, and `Display`.
    - `DirectedEdge` now has `{ from, to, kind }` with serde derives and a `new(from, to, kind)` constructor.

- **Connection semantics (`validate_connection_semantics`)**
  - Design: per‑edge validation based on `(StageRole, EdgeKind)` plus a `TopologyError::InvalidConnection` variant.
  - Implementation: **Done.**
    - `TopologyError::InvalidConnection` is implemented with full context (names, types, roles, operator, reason).
    - `validate_connection_semantics` enforces the StageRole + EdgeKind matrix described earlier.

- **Structural topology constraints**
  - Design: enforce `NoSources`, `NoSinks`, `UnreachableStages`, and `UnproductiveStages` based on StageType/StageRole.
  - Implementation: **Done.**
    - `TopologyError` includes `NoSources`, `NoSinks`, `UnreachableStages`, `UnproductiveStages`.
    - `validate_topology_structure` enforces:
      - at least one Producer and one Consumer,
      - every stage reachable from some Producer,
      - every stage can reach some Consumer.
    - Note: `validate_topology_structure` takes only `(stages, downstream)` — the `upstream` map is not needed for reachability checks since both "reachable from sources" and "can reach sinks" traverse downstream edges.

- **Topology::new wiring**
  - Design: call `validate_connection_semantics` and structural validators from `Topology::new` after building adjacency lists.
  - Implementation: **Done.**
    - `Topology::new_unvalidated` builds maps/adjacency and runs structural checks.
    - `Topology::new` calls `new_unvalidated` and then `validate_with_level(ValidationLevel::Full)`, which runs semantic + structural invariants.

- **TopologyBuilder::build_unchecked**
  - Design: provide a builder method for structural-only validation to support UI workflows.
  - Implementation: **Done.**
    - `build_unchecked()` returns `Result<Topology, TopologyError>` with structural validation only.
    - `build()` continues to apply full validation (structural + semantic + reachability).

- **Semantic vs structural source/sink queries**
  - Design: keep existing structural `source_stages()` / `sink_stages()`, and add semantic `semantic_source_stages()` / `semantic_sink_stages()` based on `StageRole`.
  - Implementation: **Done.**
    - `Topology` includes both structural and semantic getters; semantic getters use `StageType::role()` to identify Producers/Consumers.

- **Join coverage**
  - Design: align `StageType` in this crate with `obzenflow_core::event::context::StageType`, including a `Join` variant that maps to `StageRole::Processor` (can consume and produce events).
  - Implementation: **Done.**
    - `StageType` now includes `Join` and `StageType::role()` maps Transform/Stateful/Join to `StageRole::Processor`.

- **Extension points**
  - Design: `StageExtensions` and `EdgeExtensions` structs with optional fields for middleware, contracts, and UI hints.
  - Implementation: **Done.**
    - `StageInfo` has optional `StageExtensions` and `DirectedEdge` has optional `EdgeExtensions`, both using `serde_json::Value` for flexible metadata.
    - `StageMetadata` remains for now as a legacy metadata type; new code should prefer `StageInfo` + `StageExtensions`.

## Relation to Phase 2 (Runtime / DSL)

Phase 1 (this document) is **pure topology** and lives entirely in `obzenflow-topology`. It is shared by:

- The backend (`flowstate_rs`) for pipeline construction.
- The UI (`obzen-flow-ui`) for visualization and editing.

Phase 2 (to be implemented in `flowstate_rs`) will:

- Update the DSL to pass `StageType` and `EdgeKind` into `TopologyBuilder`.
- Add a runtime validator that combines:
  - This topology information (StageType + EdgeKind + structure).
  - Contract configuration (090c/090d).
  - Source semantics (081a/081b).
  - Error semantics (082h).
- Enforce additional rules such as “backflow is impossible directly into a join stage” where those depend on handler/join descriptors.

### Join Inputs: Reference vs Stream

At the topology layer, a `StageType::Join` stage is modeled as a `Processor` (via `StageRole`) with multiple upstream edges; the graph intentionally does **not** distinguish between the "reference" and "stream" sides of a join. That distinction:

- lives in the runtime/contract layer (subscription configuration, handler descriptors, join FSMs in `obzenflow_runtime_services`), and
- is enforced by Phase‑2 validation in `flowstate_rs`, not by `obzenflow-topology`.

This keeps the shared topology model simple and reusable (UI, backend) while still allowing join FSMs to express richer per‑input behavior at runtime.

By keeping the Phase 1 design here in `obzenflow-topology`, we ensure that both server and UI can converge on the same, semantically rich graph model once the implementation work is completed, and that invalid states (like `sink |> source`) become unrepresentable at the shared topology layer.

## Summary of Key Design Decisions

Based on the cross-codebase review of `obzenflow-topology`, `flowstate_rs`, and `obzen-flow-ui`:

1. **Rename `SimpleStageType` → `StageRole`** with variants `Producer`, `Processor`, `Consumer`
   - Avoids overloading "Transform" to mean three different things
   - Clear that it's about connection semantics, not runtime behavior

2. **Rename `simple()` → `role()`**
   - Method name reflects the semantic purpose

3. **Keep `StageType` granular** (Transform, Stateful, Join remain distinct)
   - Runtime differences (stateless vs accumulator vs multi-input) are significant
   - UI and runtime may need type-specific behavior

4. **Shape remains independent** (inferred by UI from connection counts)
   - Visual port layout is orthogonal to both role and type
   - UI already correctly infers Shape without knowing StageType

5. **Add extension points** (`StageExtensions`, `EdgeExtensions`)
   - Future middleware/contract visualization without breaking changes
   - Uses `serde_json::Value` for flexibility during design iteration

6. **Three orthogonal concepts documented**:
   - `StageRole` → validation (can this connection exist?)
   - `StageType` → runtime (how does this stage behave?)
   - `Shape` → visualization (how many ports, where do they go?)

### Migration Notes for CHANGELOG

For v0.2.0 CHANGELOG, include:

```markdown
### Breaking Changes

- `SimpleStageType` renamed to `StageRole` with variants `Producer`, `Processor`, `Consumer`
- `StageType::simple()` method renamed to `StageType::role()`
- `StageInfo` now requires `stage_type: StageType` field
- `DirectedEdge` now requires `kind: EdgeKind` field
- `StageMetadata` is deprecated; use `StageInfo` with optional `StageExtensions`

### Migration

- Replace `SimpleStageType::Source` → `StageRole::Producer`
- Replace `SimpleStageType::Transform` → `StageRole::Processor`
- Replace `SimpleStageType::Sink` → `StageRole::Consumer`
- Replace `.simple()` calls → `.role()`
```
