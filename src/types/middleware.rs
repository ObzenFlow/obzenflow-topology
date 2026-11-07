// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Checked, descriptive information about resolved middleware bindings.
//! These records do not resolve configuration or grant execution authority.

mod key;
mod settings;

pub use key::MiddlewareAttachmentKey;
pub use settings::{
    MiddlewareSettingInfo, MiddlewareSettingsInfo, ResolvedSettingInfo, SettingProvenanceInfo,
    SettingSubject, SettingValueInfo,
};

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("invalid middleware information at {field}: {reason}")]
pub struct MiddlewareInfoError {
    pub field: String,
    pub reason: String,
}

impl MiddlewareInfoError {
    pub(super) fn new(field: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            field: field.into(),
            reason: reason.into(),
        }
    }
}

/// Membership order has no execution meaning. Keys are unique within a topology.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct MiddlewareInfo {
    pub attachments: Vec<MiddlewareAttachmentInfo>,
}

impl MiddlewareInfo {
    pub fn try_new(
        attachments: Vec<MiddlewareAttachmentInfo>,
    ) -> Result<Self, MiddlewareInfoError> {
        let info = Self { attachments };
        info.validate()?;
        Ok(info)
    }

    pub fn validate(&self) -> Result<(), MiddlewareInfoError> {
        let mut keys = BTreeSet::new();
        for attachment in &self.attachments {
            attachment.details.validate()?;
            if !keys.insert(attachment.key) {
                return Err(MiddlewareInfoError::new(
                    "attachments",
                    format!("duplicate binding key {}", attachment.key),
                ));
            }
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for MiddlewareInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            attachments: Vec<MiddlewareAttachmentInfo>,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::try_new(wire.attachments).map_err(serde::de::Error::custom)
    }
}

/// One resolved binding. Expanded observers retain site and label for logical grouping.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MiddlewareAttachmentInfo {
    pub key: MiddlewareAttachmentKey,
    pub label: String,
    pub authored_site: MiddlewareAuthoredSite,
    pub operation: MiddlewareOperation,
    pub details: MiddlewareDetailsInfo,
}

impl MiddlewareAttachmentInfo {
    pub fn family(&self) -> MiddlewareFamily {
        self.details.family()
    }
}

/// The family discriminant. Every family carries the same settings record.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "info", rename_all = "snake_case")]
pub enum MiddlewareDetailsInfo {
    RateLimiter(MiddlewareSettingsInfo),
    CircuitBreaker(MiddlewareSettingsInfo),
    Retry(MiddlewareSettingsInfo),
    Observer(MiddlewareSettingsInfo),
    Custom {
        name: String,
        info: MiddlewareSettingsInfo,
    },
}

impl<'de> Deserialize<'de> for MiddlewareDetailsInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(
            tag = "kind",
            content = "info",
            rename_all = "snake_case",
            deny_unknown_fields
        )]
        enum Wire {
            RateLimiter(MiddlewareSettingsInfo),
            CircuitBreaker(MiddlewareSettingsInfo),
            Retry(MiddlewareSettingsInfo),
            Observer(MiddlewareSettingsInfo),
            Custom {
                name: String,
                info: MiddlewareSettingsInfo,
            },
        }
        let info = match Wire::deserialize(deserializer)? {
            Wire::RateLimiter(info) => Self::RateLimiter(info),
            Wire::CircuitBreaker(info) => Self::CircuitBreaker(info),
            Wire::Retry(info) => Self::Retry(info),
            Wire::Observer(info) => Self::Observer(info),
            Wire::Custom { name, info } => Self::Custom { name, info },
        };
        info.validate().map_err(serde::de::Error::custom)?;
        Ok(info)
    }
}

impl MiddlewareDetailsInfo {
    pub fn family(&self) -> MiddlewareFamily {
        match self {
            Self::RateLimiter(_) => MiddlewareFamily::RateLimiter,
            Self::CircuitBreaker(_) => MiddlewareFamily::CircuitBreaker,
            Self::Retry(_) => MiddlewareFamily::Retry,
            Self::Observer(_) => MiddlewareFamily::Observer,
            Self::Custom { name, .. } => MiddlewareFamily::Custom { name: name.clone() },
        }
    }

    pub fn settings(&self) -> &MiddlewareSettingsInfo {
        match self {
            Self::RateLimiter(info)
            | Self::CircuitBreaker(info)
            | Self::Retry(info)
            | Self::Observer(info)
            | Self::Custom { info, .. } => info,
        }
    }

    /// Place already-resolved rows under their family, without resolving precedence.
    pub fn try_from_settings(
        family: MiddlewareFamily,
        settings: MiddlewareSettingsInfo,
    ) -> Result<Self, MiddlewareInfoError> {
        let info = match family {
            MiddlewareFamily::RateLimiter => Self::RateLimiter(settings),
            MiddlewareFamily::CircuitBreaker => Self::CircuitBreaker(settings),
            MiddlewareFamily::Retry => Self::Retry(settings),
            MiddlewareFamily::Observer => Self::Observer(settings),
            MiddlewareFamily::Custom { name } => Self::Custom {
                name,
                info: settings,
            },
        };
        info.validate()?;
        Ok(info)
    }

    fn validate(&self) -> Result<(), MiddlewareInfoError> {
        let (prefix, family) = match self {
            Self::RateLimiter(_) => ("middleware.rate_limiter.", "rate limiter"),
            Self::CircuitBreaker(_) => ("middleware.circuit_breaker.", "circuit breaker"),
            Self::Retry(_) => ("middleware.retry.", "retry"),
            Self::Observer(_) => return Ok(()),
            Self::Custom { name, .. } => return settings::nonempty("custom.name", name),
        };
        let outside = self.settings().settings().keys().find(|key| {
            key.strip_prefix(prefix)
                .is_none_or(|setting| setting.is_empty())
        });
        match outside {
            Some(key) => Err(MiddlewareInfoError::new(
                key,
                format!("must sit under {prefix} for a {family}"),
            )),
            None => Ok(()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MiddlewareFamily {
    RateLimiter,
    CircuitBreaker,
    Retry,
    Observer,
    Custom { name: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum MiddlewareAuthoredSite {
    Implementation,
    Effect { effect_type: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
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
