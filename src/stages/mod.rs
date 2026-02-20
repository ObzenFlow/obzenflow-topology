// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

mod stage_info;

// Use StageId from types module
pub use crate::types::StageId;
pub use stage_info::StageInfo;
#[allow(deprecated)]
pub use stage_info::StageMetadata;
