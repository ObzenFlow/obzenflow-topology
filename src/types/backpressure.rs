// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

use serde::{Deserialize, Deserializer, Serialize};
use std::num::NonZeroU64;

/// Build-resolved transport policy for one directed edge.
///
/// This describes the materialised plan, including implicit cycle tracking.
/// It is independent of traffic, current credit balances, and metrics availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "mode", rename_all = "snake_case")]
pub enum BackpressureInfo {
    Track,
    Enforce {
        window: NonZeroU64,
        stall_timeout_ms: NonZeroU64,
    },
}

impl<'de> Deserialize<'de> for BackpressureInfo {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        #[derive(Deserialize)]
        #[serde(tag = "mode", rename_all = "snake_case", deny_unknown_fields)]
        enum Wire {
            Track {},
            Enforce {
                window: NonZeroU64,
                stall_timeout_ms: NonZeroU64,
            },
        }
        Ok(match Wire::deserialize(deserializer)? {
            Wire::Track {} => Self::Track,
            Wire::Enforce {
                window,
                stall_timeout_ms,
            } => Self::Enforce {
                window,
                stall_timeout_ms,
            },
        })
    }
}
