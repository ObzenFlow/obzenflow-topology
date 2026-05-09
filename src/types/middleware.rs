// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Stage middleware annotation (FLOWIP-059).
//!
//! Structured middleware configuration exposed as part of the canonical
//! topology so clients (Studio, dashboards) can render rate limiting,
//! circuit-breaker, and retry posture without hitting a runtime endpoint.
//!
//! These are static configuration snapshots, not live runtime metrics.

use serde::{Deserialize, Serialize};

/// Middleware stack and configuration for one stage.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MiddlewareInfo {
    /// Ordered list of middleware names in the stack (outermost first).
    pub stack: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub circuit_breaker: Option<CircuitBreakerInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate_limiter: Option<RateLimiterInfo>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryInfo>,
}

impl MiddlewareInfo {
    pub fn new(stack: Vec<String>) -> Self {
        Self {
            stack,
            circuit_breaker: None,
            rate_limiter: None,
            retry: None,
        }
    }

    pub fn with_circuit_breaker(mut self, config: CircuitBreakerInfo) -> Self {
        self.circuit_breaker = Some(config);
        self
    }

    pub fn with_rate_limiter(mut self, config: RateLimiterInfo) -> Self {
        self.rate_limiter = Some(config);
        self
    }

    pub fn with_retry(mut self, config: RetryInfo) -> Self {
        self.retry = Some(config);
        self
    }
}

/// Static circuit-breaker configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CircuitBreakerInfo {
    /// Number of failures before opening.
    pub threshold: usize,
    /// Cooldown before half-open, in milliseconds.
    pub cooldown_ms: u64,
    pub open_policy: OpenPolicy,
    pub has_fallback: bool,
}

/// Behaviour while the circuit is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpenPolicy {
    EmitFallback,
    FailFast,
    Skip,
}

/// Static rate-limiter configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RateLimiterInfo {
    pub tokens_per_sec: f64,
    pub burst_capacity: f64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configured_burst_capacity: Option<f64>,
    pub cost_per_event: f64,
    pub limit_rate: f64,
}

/// Static retry policy configuration.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RetryInfo {
    /// Maximum retry attempts; `None` means unbounded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_attempts: Option<usize>,
    pub backoff: BackoffStrategy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_delay_ms: Option<u64>,
}

/// Retry backoff curve.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BackoffStrategy {
    Fixed,
    Exponential,
    None,
}
