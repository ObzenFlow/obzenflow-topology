pub mod scc_id;
pub mod stage_type;

use obzenflow_idkit::Id;

// Domain marker type for stages in the topology
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stage;

// Type alias for stage identifiers using phantom-typed ID
pub type StageId = Id<Stage>;

// Re-export stage type enums
pub use scc_id::SccId;
pub use stage_type::{StageRole, StageType};
