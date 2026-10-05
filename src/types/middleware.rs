// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Resolved middleware attachments in the canonical topology.
//! These are build-time declarations and effective configuration, not live metrics.

use serde::{Deserialize, Serialize};

/// A stage's resolved middleware membership. Vector position has no execution meaning.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MiddlewareInfo {
    pub attachments: Vec<MiddlewareAttachmentInfo>,
}

/// One resolved binding, carrying its semantic key and actual operation.
/// Expanded observer bindings retain their logical grouping through site and label.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MiddlewareAttachmentInfo {
    pub key: String,
    pub label: String,
    pub family: MiddlewareFamily,
    pub authored_site: MiddlewareAuthoredSite,
    pub operation: MiddlewareOperation,
    /// Built-ins map canonical config keys to `{ value, source, scope }` rows.
    /// Custom attachments retain their definition's resolved key namespace.
    pub configuration: serde_json::Value,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MiddlewareFamily {
    RateLimiter,
    CircuitBreaker,
    Retry,
    Observer,
    Custom { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MiddlewareAuthoredSite {
    Implementation,
    Effect { effect_type: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MiddlewareOperation {
    SourcePoll,
    Ingress,
    SinkDelivery,
    Effect { effect_type: String },
    Handler,
    Stateful,
    Join,
    Lifecycle,
}
