// Common test utilities for topology tests
use obzenflow_idkit::Id;
use once_cell::sync::Lazy;
use std::sync::Mutex;

pub type StageId = Id<obzenflow_topology::types::Stage>;

/// Global counter for deterministic test IDs
static COUNTER: Lazy<Mutex<u128>> = Lazy::new(|| Mutex::new(0));

/// Generate a deterministic StageId for tests
pub fn next_stage_id() -> StageId {
    let mut counter = COUNTER.lock().unwrap();
    *counter += 1;

    // Convert u128 counter to 16 bytes for ULID format
    StageId::from_bytes((*counter).to_be_bytes())
}
