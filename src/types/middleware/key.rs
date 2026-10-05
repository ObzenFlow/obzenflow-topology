// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use super::MiddlewareInfoError;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::{fmt, str::FromStr};

/// Opaque resolved-binding identity, scoped to its owning topology.
/// The wire spelling is canonical ULID text; the bytes have no timestamp meaning.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MiddlewareAttachmentKey([u8; 16]);

impl MiddlewareAttachmentKey {
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }
    pub const fn to_bytes(self) -> [u8; 16] {
        self.0
    }
}

impl fmt::Display for MiddlewareAttachmentKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        obzenflow_idkit::Ulid::from_bytes(self.0).fmt(f)
    }
}

impl FromStr for MiddlewareAttachmentKey {
    type Err = MiddlewareInfoError;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let invalid = || {
            MiddlewareInfoError::new("key", "expected canonical uppercase 26-character ULID text")
        };
        if raw.len() != 26 {
            return Err(invalid());
        }
        let parsed = obzenflow_idkit::Ulid::from_str(raw).map_err(|_| invalid())?;
        if parsed.to_string() != raw {
            return Err(invalid());
        }
        Ok(Self(parsed.to_bytes()))
    }
}

impl Serialize for MiddlewareAttachmentKey {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for MiddlewareAttachmentKey {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}
