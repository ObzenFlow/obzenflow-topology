use serde::{Deserialize, Serialize};
use std::fmt;

/// Identifies which strongly connected component (SCC) a stage belongs to.
///
/// SCC IDs are deterministic for a given topology but not stable across
/// topology changes. They are used for grouping cycle-member stages within
/// a single materialisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SccId(u32);

impl SccId {
    /// Create a new SCC identifier from a raw index.
    pub fn new(id: u32) -> Self {
        Self(id)
    }

    /// Return the underlying `u32` value.
    pub fn as_u32(self) -> u32 {
        self.0
    }

    /// Convert to `usize` for Vec indexing (crate-internal only).
    pub(crate) fn into_index(self) -> usize {
        self.0 as usize
    }
}

impl fmt::Display for SccId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "scc_{}", self.0)
    }
}
