// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use super::{MiddlewareInfoError, ResolvedSettingInfo, SettingValueInfo};
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

type Settings = BTreeMap<String, ResolvedSettingInfo<SettingValueInfo>>;

// The private fields keep validated information immutable. Missing optional rows
// and present null rows have deliberately different wire meanings.
macro_rules! checked_info {
    ($name:ident, $wire:ident, $prefix:literal,
        required { $($required:ident: $required_type:ty),* $(,)? }
        optional { $($optional:ident: $optional_type:ty),* $(,)? }
    ) => {
        #[derive(Clone, Debug, PartialEq, Serialize)]
        pub struct $name {
            $($required: ResolvedSettingInfo<$required_type>,)*
            $(#[serde(skip_serializing_if = "Option::is_none")]
              $optional: Option<ResolvedSettingInfo<$optional_type>>,)*
        }

        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct $wire {
            $($required: ResolvedSettingInfo<$required_type>,)*
            $(#[serde(default, deserialize_with = "super::settings::present")]
              $optional: Option<ResolvedSettingInfo<$optional_type>>,)*
        }

        impl $name {
            $(pub fn $required(&self) -> &ResolvedSettingInfo<$required_type> { &self.$required })*
            $(pub fn $optional(&self) -> Option<&ResolvedSettingInfo<$optional_type>> { self.$optional.as_ref() })*

            pub(super) fn from_settings(mut settings: Settings) -> Result<Self, MiddlewareInfoError> {
                let info = Self {
                    $($required: required(&mut settings, concat!($prefix, stringify!($required)))?,)*
                    $($optional: take(&mut settings, concat!($prefix, stringify!($optional)))?,)*
                };
                if let Some(key) = settings.keys().next() {
                    return Err(MiddlewareInfoError::new(key, "unknown field for this middleware family"));
                }
                info.validate()?;
                Ok(info)
            }

            fn validate_fields(&self) -> Result<(), MiddlewareInfoError> {
                $(validate_row(concat!($prefix, stringify!($required)), &self.$required)?;)*
                $(if let Some(row) = &self.$optional { validate_row(concat!($prefix, stringify!($optional)), row)?; })*
                Ok(())
            }
        }

        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                let wire = $wire::deserialize(deserializer)?;
                let info = Self { $($required: wire.$required,)* $($optional: wire.$optional,)* };
                info.validate().map_err(serde::de::Error::custom)?;
                Ok(info)
            }
        }
    };
}

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

checked_info!(RateLimiterInfo, RateLimiterWire, "middleware.rate_limiter.",
    required { events_per_second: f64, cost_per_attempt: f64 }
    optional { burst_capacity: f64 }
);

impl RateLimiterInfo {
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

checked_info!(CircuitBreakerInfo, CircuitBreakerWire, "middleware.circuit_breaker.",
    required { mode: CircuitBreakerMode, open_for_ms: u64, probes: u32, rate_limited_counts_as_failure: bool }
    optional { consecutive_failures: u32, count_window: u32, minimum_calls: u32, failure_rate_threshold: f64, slow_call_duration_ms: u64, slow_call_rate_threshold: f64 }
);

impl CircuitBreakerInfo {
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

checked_info!(RetryInfo, RetryWire, "middleware.retry.",
    required { kind: RetryKind, max_attempts: u32, max_backoff_ms: u64, attempt_start_window_ms: u64 }
    optional { fixed_delay_ms: u64 }
);

impl RetryInfo {
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
