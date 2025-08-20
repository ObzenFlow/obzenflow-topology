pub mod stage_type;

use idkit::Id;

// Domain marker type for stages in the topology
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Stage;

// Type alias for stage identifiers using phantom-typed ID
pub type StageId = Id<Stage>;

// Re-export stage type enums
pub use stage_type::{StageType, SimpleStageType};