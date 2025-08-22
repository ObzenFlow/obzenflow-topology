// Common test utilities for topology tests
use once_cell::sync::Lazy;
use std::sync::Mutex;
use obzenflow_idkit::Id;

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

/// Test helper extension for TopologyBuilder
pub trait TestTopologyBuilder {
    fn add_stage(&mut self, name: Option<String>) -> StageId;
}

impl TestTopologyBuilder for obzenflow_topology::builder::TopologyBuilder {
    fn add_stage(&mut self, name: Option<String>) -> StageId {
        let id = next_stage_id();
        self.add_stage_with_id(id, name)
    }
}