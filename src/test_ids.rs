// SPDX-License-Identifier: MIT OR Apache-2.0
// SPDX-FileCopyrightText: 2025-2026 ObzenFlow Contributors
// https://obzenflow.dev

// Test-only utilities for deterministic ID generation
// This module is always compiled but only used in tests to avoid cfg complexity

use crate::types::StageId;
use obzenflow_idkit::Id;
use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Global counter for deterministic test IDs
static COUNTER: Lazy<Mutex<u128>> = Lazy::new(|| Mutex::new(0));

/// Generate a deterministic StageId for tests
pub fn next_stage_id() -> StageId {
    let mut counter = COUNTER.lock().unwrap();
    *counter += 1;

    // Convert u128 counter to 16 bytes for ULID format
    Id::from_bytes((*counter).to_be_bytes())
}
