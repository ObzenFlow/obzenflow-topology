// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use super::MiddlewareInfoError;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResolvedSettingInfo<T> {
    pub value: T,
    pub provenance: SettingProvenanceInfo,
}

/// Winning candidate provenance, distinct from the attachment's application point.
/// Source and scope retain their open wire vocabularies, including future spellings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct SettingProvenanceInfo {
    pub source: String,
    pub scope: String,
    pub winner_subject: SettingSubject,
}

impl SettingProvenanceInfo {
    pub fn try_new(
        source: String,
        scope: String,
        winner_subject: SettingSubject,
    ) -> Result<Self, MiddlewareInfoError> {
        let info = Self {
            source,
            scope,
            winner_subject,
        };
        info.validate()?;
        Ok(info)
    }

    pub(super) fn validate(&self) -> Result<(), MiddlewareInfoError> {
        nonempty("provenance.source", &self.source)?;
        nonempty("provenance.scope", &self.scope)?;
        if let SettingSubject::Effect { effect_type } = &self.winner_subject {
            nonempty("provenance.winner_subject.effect_type", effect_type)?;
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for SettingProvenanceInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            source: String,
            scope: String,
            winner_subject: SettingSubject,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::try_new(wire.source, wire.scope, wire.winner_subject)
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SettingSubject {
    Unqualified,
    Effect { effect_type: String },
}

impl<'de> Deserialize<'de> for SettingSubject {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
        enum Wire {
            Unqualified {},
            Effect { effect_type: String },
        }
        match Wire::deserialize(deserializer)? {
            Wire::Unqualified {} => Ok(Self::Unqualified),
            Wire::Effect { effect_type } => {
                nonempty("provenance.winner_subject.effect_type", &effect_type)
                    .map_err(serde::de::Error::custom)?;
                Ok(Self::Effect { effect_type })
            }
        }
    }
}

/// Scalar extension information. Nested objects/arrays and raw secret values are not a wire escape hatch.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SettingValueInfo {
    Bool(bool),
    U64(u64),
    F64(f64),
    Text(String),
    Redacted,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct MiddlewareExtensionInfo {
    settings: BTreeMap<String, ResolvedSettingInfo<SettingValueInfo>>,
}

impl MiddlewareExtensionInfo {
    pub fn try_new(
        settings: BTreeMap<String, ResolvedSettingInfo<SettingValueInfo>>,
    ) -> Result<Self, MiddlewareInfoError> {
        let info = Self { settings };
        info.validate()?;
        Ok(info)
    }

    pub fn settings(&self) -> &BTreeMap<String, ResolvedSettingInfo<SettingValueInfo>> {
        &self.settings
    }

    pub(super) fn validate(&self) -> Result<(), MiddlewareInfoError> {
        for (key, setting) in &self.settings {
            nonempty("settings.key", key)?;
            setting.provenance.validate()?;
            if let SettingValueInfo::F64(value) = &setting.value {
                if !value.is_finite() {
                    return Err(MiddlewareInfoError::new(key, "must be finite"));
                }
            }
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for MiddlewareExtensionInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            settings: BTreeMap<String, ResolvedSettingInfo<SettingValueInfo>>,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::try_new(wire.settings).map_err(serde::de::Error::custom)
    }
}

pub(super) fn nonempty(field: &str, value: &str) -> Result<(), MiddlewareInfoError> {
    if value.trim().is_empty() {
        Err(MiddlewareInfoError::new(field, "must not be empty"))
    } else {
        Ok(())
    }
}

/// Called only for present optional fields. Missing is allowed; explicit null is not.
pub(super) fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}
