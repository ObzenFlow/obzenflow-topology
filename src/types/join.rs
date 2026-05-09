// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

//! Join metadata annotation (FLOWIP-082i).
//!
//! For join stages, distinguishes which upstream stages provide the
//! catalog/reference inputs (typically finite, hydrated before stream
//! processing) from those that provide stream inputs (the long-lived
//! main flow).

use crate::stages::StageId;
use serde::{Deserialize, Serialize};

/// Join-specific source classification.
///
/// Only meaningful when attached to a `StageInfo` whose `stage_type` is
/// `Join`. Both lists hold canonical topology `StageId`s; identical
/// formatting to other id fields on the wire.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct JoinMetadataInfo {
    /// Stage IDs whose outputs feed the join's reference (catalog) leg.
    pub catalog_source_ids: Vec<StageId>,
    /// Stage IDs whose outputs feed the join's stream leg.
    pub stream_source_ids: Vec<StageId>,
}

impl JoinMetadataInfo {
    pub fn new(catalog_source_ids: Vec<StageId>, stream_source_ids: Vec<StageId>) -> Self {
        Self {
            catalog_source_ids,
            stream_source_ids,
        }
    }
}
