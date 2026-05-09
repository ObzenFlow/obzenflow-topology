// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Edge contract annotation.
//!
//! Identifies a structural contract attached to a directed edge (e.g.
//! `TransportContract`, `SourceContract`). The contract config remains an
//! opaque JSON value because contract types are defined outside the
//! topology crate; the canonical model only needs the name and an optional
//! serialised configuration blob for clients to inspect.

use serde::{Deserialize, Serialize};

/// One contract attached to a directed edge.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ContractInfo {
    /// Contract name (e.g. `"TransportContract"`).
    pub name: String,
    /// Optional opaque configuration blob. Contract types are defined
    /// outside the topology crate; this value is the contract's own
    /// `Serialize` output.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub config: Option<serde_json::Value>,
}

impl ContractInfo {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            config: None,
        }
    }

    pub fn with_config(mut self, config: serde_json::Value) -> Self {
        self.config = Some(config);
        self
    }
}
