// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use super::{MiddlewareInfoError, ResolvedSettingInfo, SettingValueInfo};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

type Settings = BTreeMap<String, ResolvedSettingInfo<SettingValueInfo>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CircuitBreakerMode {
    Consecutive,
    RateBased,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RetryKind {
    Fixed,
    Exponential,
}

/// Resolved rate-limiter information. Private fields preserve checked values.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RateLimiterInfo {
    events_per_second: ResolvedSettingInfo<f64>,
    cost_per_attempt: ResolvedSettingInfo<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    burst_capacity: Option<ResolvedSettingInfo<f64>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RateLimiterWire {
    events_per_second: ResolvedSettingInfo<f64>,
    cost_per_attempt: ResolvedSettingInfo<f64>,
    // Omission means automatic capacity; an explicit null is invalid.
    #[serde(default, deserialize_with = "super::settings::present")]
    burst_capacity: Option<ResolvedSettingInfo<f64>>,
}

impl<'de> Deserialize<'de> for RateLimiterInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = RateLimiterWire::deserialize(deserializer)?;
        let info = Self {
            events_per_second: wire.events_per_second,
            cost_per_attempt: wire.cost_per_attempt,
            burst_capacity: wire.burst_capacity,
        };
        info.validate().map_err(serde::de::Error::custom)?;
        Ok(info)
    }
}

impl RateLimiterInfo {
    pub fn events_per_second(&self) -> &ResolvedSettingInfo<f64> {
        &self.events_per_second
    }

    pub fn cost_per_attempt(&self) -> &ResolvedSettingInfo<f64> {
        &self.cost_per_attempt
    }

    pub fn burst_capacity(&self) -> Option<&ResolvedSettingInfo<f64>> {
        self.burst_capacity.as_ref()
    }

    pub(super) fn from_settings(mut settings: Settings) -> Result<Self, MiddlewareInfoError> {
        let info = Self {
            events_per_second: required(
                &mut settings,
                "middleware.rate_limiter.events_per_second",
            )?,
            cost_per_attempt: required(&mut settings, "middleware.rate_limiter.cost_per_attempt")?,
            burst_capacity: take(&mut settings, "middleware.rate_limiter.burst_capacity")?,
        };
        if let Some(key) = settings.keys().next() {
            return Err(MiddlewareInfoError::new(
                key,
                "unknown field for this middleware family",
            ));
        }
        info.validate()?;
        Ok(info)
    }

    fn validate_fields(&self) -> Result<(), MiddlewareInfoError> {
        validate_row(
            "middleware.rate_limiter.events_per_second",
            &self.events_per_second,
        )?;
        validate_row(
            "middleware.rate_limiter.cost_per_attempt",
            &self.cost_per_attempt,
        )?;
        if let Some(row) = &self.burst_capacity {
            validate_row("middleware.rate_limiter.burst_capacity", row)?;
        }
        Ok(())
    }

    pub fn try_new(
        events_per_second: ResolvedSettingInfo<f64>,
        cost_per_attempt: ResolvedSettingInfo<f64>,
        burst_capacity: Option<ResolvedSettingInfo<f64>>,
    ) -> Result<Self, MiddlewareInfoError> {
        let info = Self {
            events_per_second,
            cost_per_attempt,
            burst_capacity,
        };
        info.validate()?;
        Ok(info)
    }

    pub(super) fn validate(&self) -> Result<(), MiddlewareInfoError> {
        self.validate_fields()?;
        if self
            .burst_capacity
            .as_ref()
            .is_some_and(|burst| burst.value < self.cost_per_attempt.value)
        {
            return Err(MiddlewareInfoError::new(
                "middleware.rate_limiter.burst_capacity",
                "must be at least cost_per_attempt",
            ));
        }
        Ok(())
    }
}

/// Resolved circuit-breaker information, including retained inactive settings.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct CircuitBreakerInfo {
    mode: ResolvedSettingInfo<CircuitBreakerMode>,
    open_for_ms: ResolvedSettingInfo<u64>,
    probes: ResolvedSettingInfo<u32>,
    rate_limited_counts_as_failure: ResolvedSettingInfo<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    consecutive_failures: Option<ResolvedSettingInfo<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    count_window: Option<ResolvedSettingInfo<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    minimum_calls: Option<ResolvedSettingInfo<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    failure_rate_threshold: Option<ResolvedSettingInfo<f64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slow_call_duration_ms: Option<ResolvedSettingInfo<u64>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    slow_call_rate_threshold: Option<ResolvedSettingInfo<f64>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CircuitBreakerWire {
    mode: ResolvedSettingInfo<CircuitBreakerMode>,
    open_for_ms: ResolvedSettingInfo<u64>,
    probes: ResolvedSettingInfo<u32>,
    rate_limited_counts_as_failure: ResolvedSettingInfo<bool>,
    #[serde(default, deserialize_with = "super::settings::present")]
    consecutive_failures: Option<ResolvedSettingInfo<u32>>,
    #[serde(default, deserialize_with = "super::settings::present")]
    count_window: Option<ResolvedSettingInfo<u32>>,
    #[serde(default, deserialize_with = "super::settings::present")]
    minimum_calls: Option<ResolvedSettingInfo<u32>>,
    #[serde(default, deserialize_with = "super::settings::present")]
    failure_rate_threshold: Option<ResolvedSettingInfo<f64>>,
    #[serde(default, deserialize_with = "super::settings::present")]
    slow_call_duration_ms: Option<ResolvedSettingInfo<u64>>,
    #[serde(default, deserialize_with = "super::settings::present")]
    slow_call_rate_threshold: Option<ResolvedSettingInfo<f64>>,
}

impl<'de> Deserialize<'de> for CircuitBreakerInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = CircuitBreakerWire::deserialize(deserializer)?;
        let info = Self {
            mode: wire.mode,
            open_for_ms: wire.open_for_ms,
            probes: wire.probes,
            rate_limited_counts_as_failure: wire.rate_limited_counts_as_failure,
            consecutive_failures: wire.consecutive_failures,
            count_window: wire.count_window,
            minimum_calls: wire.minimum_calls,
            failure_rate_threshold: wire.failure_rate_threshold,
            slow_call_duration_ms: wire.slow_call_duration_ms,
            slow_call_rate_threshold: wire.slow_call_rate_threshold,
        };
        info.validate().map_err(serde::de::Error::custom)?;
        Ok(info)
    }
}

impl CircuitBreakerInfo {
    pub fn mode(&self) -> &ResolvedSettingInfo<CircuitBreakerMode> {
        &self.mode
    }

    pub fn open_for_ms(&self) -> &ResolvedSettingInfo<u64> {
        &self.open_for_ms
    }

    pub fn probes(&self) -> &ResolvedSettingInfo<u32> {
        &self.probes
    }

    pub fn rate_limited_counts_as_failure(&self) -> &ResolvedSettingInfo<bool> {
        &self.rate_limited_counts_as_failure
    }

    pub fn consecutive_failures(&self) -> Option<&ResolvedSettingInfo<u32>> {
        self.consecutive_failures.as_ref()
    }

    pub fn count_window(&self) -> Option<&ResolvedSettingInfo<u32>> {
        self.count_window.as_ref()
    }

    pub fn minimum_calls(&self) -> Option<&ResolvedSettingInfo<u32>> {
        self.minimum_calls.as_ref()
    }

    pub fn failure_rate_threshold(&self) -> Option<&ResolvedSettingInfo<f64>> {
        self.failure_rate_threshold.as_ref()
    }

    pub fn slow_call_duration_ms(&self) -> Option<&ResolvedSettingInfo<u64>> {
        self.slow_call_duration_ms.as_ref()
    }

    pub fn slow_call_rate_threshold(&self) -> Option<&ResolvedSettingInfo<f64>> {
        self.slow_call_rate_threshold.as_ref()
    }

    pub(super) fn from_settings(mut settings: Settings) -> Result<Self, MiddlewareInfoError> {
        let info = Self {
            mode: required(&mut settings, "middleware.circuit_breaker.mode")?,
            open_for_ms: required(&mut settings, "middleware.circuit_breaker.open_for_ms")?,
            probes: required(&mut settings, "middleware.circuit_breaker.probes")?,
            rate_limited_counts_as_failure: required(
                &mut settings,
                "middleware.circuit_breaker.rate_limited_counts_as_failure",
            )?,
            consecutive_failures: take(
                &mut settings,
                "middleware.circuit_breaker.consecutive_failures",
            )?,
            count_window: take(&mut settings, "middleware.circuit_breaker.count_window")?,
            minimum_calls: take(&mut settings, "middleware.circuit_breaker.minimum_calls")?,
            failure_rate_threshold: take(
                &mut settings,
                "middleware.circuit_breaker.failure_rate_threshold",
            )?,
            slow_call_duration_ms: take(
                &mut settings,
                "middleware.circuit_breaker.slow_call_duration_ms",
            )?,
            slow_call_rate_threshold: take(
                &mut settings,
                "middleware.circuit_breaker.slow_call_rate_threshold",
            )?,
        };
        if let Some(key) = settings.keys().next() {
            return Err(MiddlewareInfoError::new(
                key,
                "unknown field for this middleware family",
            ));
        }
        info.validate()?;
        Ok(info)
    }

    fn validate_fields(&self) -> Result<(), MiddlewareInfoError> {
        validate_row("middleware.circuit_breaker.mode", &self.mode)?;
        validate_row("middleware.circuit_breaker.open_for_ms", &self.open_for_ms)?;
        validate_row("middleware.circuit_breaker.probes", &self.probes)?;
        validate_row(
            "middleware.circuit_breaker.rate_limited_counts_as_failure",
            &self.rate_limited_counts_as_failure,
        )?;
        if let Some(row) = &self.consecutive_failures {
            validate_row("middleware.circuit_breaker.consecutive_failures", row)?;
        }
        if let Some(row) = &self.count_window {
            validate_row("middleware.circuit_breaker.count_window", row)?;
        }
        if let Some(row) = &self.minimum_calls {
            validate_row("middleware.circuit_breaker.minimum_calls", row)?;
        }
        if let Some(row) = &self.failure_rate_threshold {
            validate_row("middleware.circuit_breaker.failure_rate_threshold", row)?;
        }
        if let Some(row) = &self.slow_call_duration_ms {
            validate_row("middleware.circuit_breaker.slow_call_duration_ms", row)?;
        }
        if let Some(row) = &self.slow_call_rate_threshold {
            validate_row("middleware.circuit_breaker.slow_call_rate_threshold", row)?;
        }
        Ok(())
    }

    pub(super) fn validate(&self) -> Result<(), MiddlewareInfoError> {
        self.validate_fields()?;
        for (key, threshold) in [
            (
                "failure_rate_threshold",
                self.failure_rate_threshold.as_ref(),
            ),
            (
                "slow_call_rate_threshold",
                self.slow_call_rate_threshold.as_ref(),
            ),
        ] {
            if threshold.is_some_and(|value| value.value > 1.0) {
                return Err(MiddlewareInfoError::new(
                    format!("middleware.circuit_breaker.{key}"),
                    "must be in (0, 1]",
                ));
            }
        }
        match self.mode.value {
            CircuitBreakerMode::Consecutive => {
                require_active(
                    &self.consecutive_failures,
                    "middleware.circuit_breaker.consecutive_failures",
                )?;
            }
            CircuitBreakerMode::RateBased => {
                let window = require_active(
                    &self.count_window,
                    "middleware.circuit_breaker.count_window",
                )?;
                let minimum = require_active(
                    &self.minimum_calls,
                    "middleware.circuit_breaker.minimum_calls",
                )?;
                if minimum.value > window.value {
                    return Err(MiddlewareInfoError::new(
                        "middleware.circuit_breaker.minimum_calls",
                        "must not exceed count_window",
                    ));
                }
                if self.slow_call_duration_ms.is_some() != self.slow_call_rate_threshold.is_some() {
                    return Err(MiddlewareInfoError::new(
                        "middleware.circuit_breaker.slow_call_duration_ms",
                        "slow-call duration and threshold must occur together in rate_based mode",
                    ));
                }
                if self.failure_rate_threshold.is_none() && self.slow_call_rate_threshold.is_none()
                {
                    return Err(MiddlewareInfoError::new(
                        "middleware.circuit_breaker.mode",
                        "rate_based mode requires a failure-rate or slow-call trigger",
                    ));
                }
            }
        }
        Ok(())
    }
}

/// Resolved retry information with durations expressed in milliseconds.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RetryInfo {
    kind: ResolvedSettingInfo<RetryKind>,
    max_attempts: ResolvedSettingInfo<u32>,
    max_backoff_ms: ResolvedSettingInfo<u64>,
    attempt_start_window_ms: ResolvedSettingInfo<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixed_delay_ms: Option<ResolvedSettingInfo<u64>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RetryWire {
    kind: ResolvedSettingInfo<RetryKind>,
    max_attempts: ResolvedSettingInfo<u32>,
    max_backoff_ms: ResolvedSettingInfo<u64>,
    attempt_start_window_ms: ResolvedSettingInfo<u64>,
    #[serde(default, deserialize_with = "super::settings::present")]
    fixed_delay_ms: Option<ResolvedSettingInfo<u64>>,
}

impl<'de> Deserialize<'de> for RetryInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = RetryWire::deserialize(deserializer)?;
        let info = Self {
            kind: wire.kind,
            max_attempts: wire.max_attempts,
            max_backoff_ms: wire.max_backoff_ms,
            attempt_start_window_ms: wire.attempt_start_window_ms,
            fixed_delay_ms: wire.fixed_delay_ms,
        };
        info.validate().map_err(serde::de::Error::custom)?;
        Ok(info)
    }
}

impl RetryInfo {
    pub fn kind(&self) -> &ResolvedSettingInfo<RetryKind> {
        &self.kind
    }

    pub fn max_attempts(&self) -> &ResolvedSettingInfo<u32> {
        &self.max_attempts
    }

    pub fn max_backoff_ms(&self) -> &ResolvedSettingInfo<u64> {
        &self.max_backoff_ms
    }

    pub fn attempt_start_window_ms(&self) -> &ResolvedSettingInfo<u64> {
        &self.attempt_start_window_ms
    }

    pub fn fixed_delay_ms(&self) -> Option<&ResolvedSettingInfo<u64>> {
        self.fixed_delay_ms.as_ref()
    }

    pub(super) fn from_settings(mut settings: Settings) -> Result<Self, MiddlewareInfoError> {
        let info = Self {
            kind: required(&mut settings, "middleware.retry.kind")?,
            max_attempts: required(&mut settings, "middleware.retry.max_attempts")?,
            max_backoff_ms: required(&mut settings, "middleware.retry.max_backoff_ms")?,
            attempt_start_window_ms: required(
                &mut settings,
                "middleware.retry.attempt_start_window_ms",
            )?,
            fixed_delay_ms: take(&mut settings, "middleware.retry.fixed_delay_ms")?,
        };
        if let Some(key) = settings.keys().next() {
            return Err(MiddlewareInfoError::new(
                key,
                "unknown field for this middleware family",
            ));
        }
        info.validate()?;
        Ok(info)
    }

    fn validate_fields(&self) -> Result<(), MiddlewareInfoError> {
        validate_row("middleware.retry.kind", &self.kind)?;
        validate_row("middleware.retry.max_attempts", &self.max_attempts)?;
        validate_row("middleware.retry.max_backoff_ms", &self.max_backoff_ms)?;
        validate_row(
            "middleware.retry.attempt_start_window_ms",
            &self.attempt_start_window_ms,
        )?;
        if let Some(row) = &self.fixed_delay_ms {
            validate_row("middleware.retry.fixed_delay_ms", row)?;
        }
        Ok(())
    }

    pub fn try_new(
        kind: ResolvedSettingInfo<RetryKind>,
        max_attempts: ResolvedSettingInfo<u32>,
        max_backoff_ms: ResolvedSettingInfo<u64>,
        attempt_start_window_ms: ResolvedSettingInfo<u64>,
        fixed_delay_ms: Option<ResolvedSettingInfo<u64>>,
    ) -> Result<Self, MiddlewareInfoError> {
        let info = Self {
            kind,
            max_attempts,
            max_backoff_ms,
            attempt_start_window_ms,
            fixed_delay_ms,
        };
        info.validate()?;
        Ok(info)
    }

    pub(super) fn validate(&self) -> Result<(), MiddlewareInfoError> {
        self.validate_fields()?;
        if self.kind.value == RetryKind::Fixed {
            require_active(&self.fixed_delay_ms, "middleware.retry.fixed_delay_ms")?;
        }
        Ok(())
    }
}

fn require_active<'a, T>(
    row: &'a Option<ResolvedSettingInfo<T>>,
    key: &str,
) -> Result<&'a ResolvedSettingInfo<T>, MiddlewareInfoError> {
    row.as_ref()
        .ok_or_else(|| MiddlewareInfoError::new(key, "required by the active mode"))
}

trait SettingType: Sized {
    fn from_value(value: SettingValueInfo) -> Option<Self>;
    fn valid(&self) -> bool;
    fn expectation() -> &'static str;
}

impl SettingType for f64 {
    fn from_value(value: SettingValueInfo) -> Option<Self> {
        if let SettingValueInfo::F64(value) = value {
            Some(value)
        } else {
            None
        }
    }
    fn valid(&self) -> bool {
        self.is_finite() && *self > 0.0
    }
    fn expectation() -> &'static str {
        "a finite number greater than zero"
    }
}

impl SettingType for u64 {
    fn from_value(value: SettingValueInfo) -> Option<Self> {
        if let SettingValueInfo::U64(value) = value {
            Some(value)
        } else {
            None
        }
    }
    fn valid(&self) -> bool {
        *self > 0
    }
    fn expectation() -> &'static str {
        "a positive unsigned millisecond count"
    }
}

impl SettingType for u32 {
    fn from_value(value: SettingValueInfo) -> Option<Self> {
        if let SettingValueInfo::U64(value) = value {
            u32::try_from(value).ok()
        } else {
            None
        }
    }
    fn valid(&self) -> bool {
        *self > 0
    }
    fn expectation() -> &'static str {
        "an integer in 1..=4294967295"
    }
}

impl SettingType for bool {
    fn from_value(value: SettingValueInfo) -> Option<Self> {
        if let SettingValueInfo::Bool(value) = value {
            Some(value)
        } else {
            None
        }
    }
    fn valid(&self) -> bool {
        true
    }
    fn expectation() -> &'static str {
        "a boolean"
    }
}

impl SettingType for CircuitBreakerMode {
    fn from_value(value: SettingValueInfo) -> Option<Self> {
        match value {
            SettingValueInfo::Text(value) if value == "consecutive" => Some(Self::Consecutive),
            SettingValueInfo::Text(value) if value == "rate_based" => Some(Self::RateBased),
            _ => None,
        }
    }
    fn valid(&self) -> bool {
        true
    }
    fn expectation() -> &'static str {
        "consecutive or rate_based"
    }
}

impl SettingType for RetryKind {
    fn from_value(value: SettingValueInfo) -> Option<Self> {
        match value {
            SettingValueInfo::Text(value) if value == "fixed" => Some(Self::Fixed),
            SettingValueInfo::Text(value) if value == "exponential" => Some(Self::Exponential),
            _ => None,
        }
    }
    fn valid(&self) -> bool {
        true
    }
    fn expectation() -> &'static str {
        "fixed or exponential"
    }
}

fn validate_row<T: SettingType>(
    key: &str,
    row: &ResolvedSettingInfo<T>,
) -> Result<(), MiddlewareInfoError> {
    row.provenance.validate()?;
    if !row.value.valid() {
        return Err(MiddlewareInfoError::new(
            key,
            format!("expected {}", T::expectation()),
        ));
    }
    Ok(())
}

fn take<T: SettingType>(
    settings: &mut Settings,
    key: &str,
) -> Result<Option<ResolvedSettingInfo<T>>, MiddlewareInfoError> {
    settings
        .remove(key)
        .map(|row| {
            let value = T::from_value(row.value).ok_or_else(|| {
                MiddlewareInfoError::new(key, format!("expected {}", T::expectation()))
            })?;
            Ok(ResolvedSettingInfo {
                value,
                provenance: row.provenance,
            })
        })
        .transpose()
}

fn required<T: SettingType>(
    settings: &mut Settings,
    key: &str,
) -> Result<ResolvedSettingInfo<T>, MiddlewareInfoError> {
    take(settings, key)?.ok_or_else(|| MiddlewareInfoError::new(key, "required field is missing"))
}
