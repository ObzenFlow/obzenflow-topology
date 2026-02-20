// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Stage type classification for pipeline coordination
//!
//! This is the canonical StageType definition used throughout the system.
//! It provides detailed classification to support proper pipeline coordination,
//! lifecycle management, and safety validation.

use serde::{Deserialize, Serialize};

/// Stage type classification for pipeline coordination
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageType {
    /// Source that will eventually complete (e.g., file reader, bounded collection)
    FiniteSource,

    /// Source that runs indefinitely (e.g., network listener, message queue consumer)
    InfiniteSource,

    /// Transforms input events to output events
    Transform,

    /// Terminal stage that consumes events (e.g., database writer, API caller)
    Sink,

    /// Stage that maintains internal state across events
    Stateful,

    /// Stage that joins events from multiple upstream sources
    Join,
}

impl StageType {
    /// Check if this is a source that needs explicit start
    pub fn is_source(&self) -> bool {
        matches!(self, StageType::FiniteSource | StageType::InfiniteSource)
    }

    /// Check if this is a finite source
    pub fn is_finite_source(&self) -> bool {
        matches!(self, StageType::FiniteSource)
    }

    /// Check if this is an infinite source
    pub fn is_infinite_source(&self) -> bool {
        matches!(self, StageType::InfiniteSource)
    }

    /// Check if this stage generates events
    pub fn generates_events(&self) -> bool {
        self.is_source()
    }

    /// Check if this stage consumes events
    pub fn consumes_events(&self) -> bool {
        !self.is_source()
    }

    /// Check if this is a terminal stage
    pub fn is_terminal(&self) -> bool {
        matches!(self, StageType::Sink)
    }

    /// Get a human-readable name for the stage type
    pub fn as_str(&self) -> &'static str {
        match self {
            StageType::FiniteSource => "finite_source",
            StageType::InfiniteSource => "infinite_source",
            StageType::Transform => "transform",
            StageType::Sink => "sink",
            StageType::Stateful => "stateful",
            StageType::Join => "join",
        }
    }
}

impl std::fmt::Display for StageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// StageRole captures connection semantics (producer/processor/consumer)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum StageRole {
    /// Generates events only (Sources) - no incoming edges
    Producer,
    /// Consumes and produces (Transform/Stateful/Join) - both in/out edges allowed
    Processor,
    /// Consumes events only (Sinks) - no outgoing forward edges allowed
    Consumer,
}

impl std::fmt::Display for StageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            StageRole::Producer => "producer",
            StageRole::Processor => "processor",
            StageRole::Consumer => "consumer",
        };
        write!(f, "{s}")
    }
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
