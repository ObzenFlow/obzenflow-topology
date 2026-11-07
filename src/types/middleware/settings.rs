// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use super::MiddlewareInfoError;
use serde::ser::SerializeMap;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

/// Scalar setting value. Nested objects/arrays and raw secret values are not a wire escape hatch.
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

/// One setting row with the text its middleware writes for it. An automatic row has no
/// resolved value.
#[derive(Clone, Debug, PartialEq)]
pub enum MiddlewareSettingInfo {
    Active {
        setting: ResolvedSettingInfo<SettingValueInfo>,
        display: String,
    },
    Inactive {
        setting: ResolvedSettingInfo<SettingValueInfo>,
        display: String,
    },
    Automatic {
        display: String,
    },
}

impl MiddlewareSettingInfo {
    pub fn display(&self) -> &str {
        match self {
            Self::Active { display, .. }
            | Self::Inactive { display, .. }
            | Self::Automatic { display } => display,
        }
    }

    pub fn setting(&self) -> Option<&ResolvedSettingInfo<SettingValueInfo>> {
        match self {
            Self::Active { setting, .. } | Self::Inactive { setting, .. } => Some(setting),
            Self::Automatic { .. } => None,
        }
    }

    fn validate(&self, key: &str) -> Result<(), MiddlewareInfoError> {
        if let Some(setting) = self.setting() {
            setting.provenance.validate()?;
            if let SettingValueInfo::F64(value) = setting.value {
                if !value.is_finite() {
                    return Err(MiddlewareInfoError::new(key, "must be finite"));
                }
            }
        }
        nonempty(&format!("{key}.display"), self.display())
    }
}

impl Serialize for MiddlewareSettingInfo {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let state = match self {
            Self::Active { .. } => "active",
            Self::Inactive { .. } => "inactive",
            Self::Automatic { .. } => "automatic",
        };
        let mut map = serializer.serialize_map(None)?;
        map.serialize_entry("state", state)?;
        if let Some(setting) = self.setting() {
            map.serialize_entry("value", &setting.value)?;
            map.serialize_entry("provenance", &setting.provenance)?;
        }
        map.serialize_entry("display", self.display())?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for MiddlewareSettingInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
        enum Wire {
            Active {
                value: SettingValueInfo,
                provenance: SettingProvenanceInfo,
                display: String,
            },
            Inactive {
                value: SettingValueInfo,
                provenance: SettingProvenanceInfo,
                display: String,
            },
            Automatic {
                display: String,
            },
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Active {
                value,
                provenance,
                display,
            } => Self::Active {
                setting: ResolvedSettingInfo { value, provenance },
                display,
            },
            Wire::Inactive {
                value,
                provenance,
                display,
            } => Self::Inactive {
                setting: ResolvedSettingInfo { value, provenance },
                display,
            },
            Wire::Automatic { display } => Self::Automatic { display },
        })
    }
}

/// One attachment's settings keyed by canonical path, with the summary its middleware
/// writes for the canvas.
#[derive(Clone, Debug, Default, PartialEq, Serialize)]
pub struct MiddlewareSettingsInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    summary: Option<String>,
    settings: BTreeMap<String, MiddlewareSettingInfo>,
}

impl MiddlewareSettingsInfo {
    pub fn try_new(
        settings: BTreeMap<String, MiddlewareSettingInfo>,
        summary: Option<String>,
    ) -> Result<Self, MiddlewareInfoError> {
        for (key, setting) in &settings {
            nonempty("settings.key", key)?;
            setting.validate(key)?;
        }
        if let Some(summary) = &summary {
            nonempty("summary", summary)?;
        }
        Ok(Self { summary, settings })
    }

    pub fn settings(&self) -> &BTreeMap<String, MiddlewareSettingInfo> {
        &self.settings
    }

    pub fn summary(&self) -> Option<&str> {
        self.summary.as_deref()
    }
}

impl<'de> Deserialize<'de> for MiddlewareSettingsInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(deny_unknown_fields)]
        struct Wire {
            #[serde(default, deserialize_with = "present")]
            summary: Option<String>,
            settings: BTreeMap<String, MiddlewareSettingInfo>,
        }
        let wire = Wire::deserialize(deserializer)?;
        Self::try_new(wire.settings, wire.summary).map_err(serde::de::Error::custom)
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
fn present<'de, D: Deserializer<'de>, T: Deserialize<'de>>(
    deserializer: D,
) -> Result<Option<T>, D::Error> {
    T::deserialize(deserializer).map(Some)
}
