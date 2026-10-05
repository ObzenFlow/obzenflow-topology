// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Checked, descriptive information about resolved middleware bindings.
//! These records do not resolve configuration or grant execution authority.

mod builtins;
mod key;
mod settings;

pub use builtins::{CircuitBreakerInfo, CircuitBreakerMode, RateLimiterInfo, RetryInfo, RetryKind};
pub use key::MiddlewareAttachmentKey;
pub use settings::{
    MiddlewareExtensionInfo, ResolvedSettingInfo, SettingProvenanceInfo, SettingSubject,
    SettingValueInfo,
};

use serde::{Deserialize, Deserializer, Serialize};
use std::collections::{BTreeMap, BTreeSet};

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

/// A single discriminant owns both family identity and its information schema.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", content = "info", rename_all = "snake_case")]
pub enum MiddlewareDetailsInfo {
    RateLimiter(RateLimiterInfo),
    CircuitBreaker(Box<CircuitBreakerInfo>),
    Retry(RetryInfo),
    Observer(MiddlewareExtensionInfo),
    Custom {
        name: String,
        info: MiddlewareExtensionInfo,
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
            RateLimiter(RateLimiterInfo),
            CircuitBreaker(Box<CircuitBreakerInfo>),
            Retry(RetryInfo),
            Observer(MiddlewareExtensionInfo),
            Custom {
                name: String,
                info: MiddlewareExtensionInfo,
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

    /// Project already-resolved canonical knob rows, without resolving precedence.
    pub fn try_from_settings(
        family: MiddlewareFamily,
        settings: BTreeMap<String, ResolvedSettingInfo<SettingValueInfo>>,
    ) -> Result<Self, MiddlewareInfoError> {
        let info = match family {
            MiddlewareFamily::RateLimiter => {
                Self::RateLimiter(RateLimiterInfo::from_settings(settings)?)
            }
            MiddlewareFamily::CircuitBreaker => {
                Self::CircuitBreaker(Box::new(CircuitBreakerInfo::from_settings(settings)?))
            }
            MiddlewareFamily::Retry => Self::Retry(RetryInfo::from_settings(settings)?),
            MiddlewareFamily::Observer => {
                Self::Observer(MiddlewareExtensionInfo::try_new(settings)?)
            }
            MiddlewareFamily::Custom { name } => Self::Custom {
                name,
                info: MiddlewareExtensionInfo::try_new(settings)?,
            },
        };
        info.validate()?;
        Ok(info)
    }

    fn validate(&self) -> Result<(), MiddlewareInfoError> {
        match self {
            Self::RateLimiter(info) => info.validate(),
            Self::CircuitBreaker(info) => info.validate(),
            Self::Retry(info) => info.validate(),
            Self::Observer(info) => info.validate(),
            Self::Custom { name, info } => {
                settings::nonempty("custom.name", name)?;
                info.validate()
            }
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
