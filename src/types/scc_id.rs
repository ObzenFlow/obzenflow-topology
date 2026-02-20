use obzenflow_idkit::Id;

/// Phantom marker type for strongly connected components.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Scc;

/// Identifies which strongly connected component (SCC) a stage belongs to.
///
/// Each SCC's identity is derived from the minimum `StageId` in its member
/// set, making it deterministic for a given topology without requiring
/// sequential index allocation. This is consistent with every other
/// identifier in the system, which uses ULID-based phantom-typed IDs.
///
/// SCC IDs are deterministic for a given topology but not stable across
/// topology changes. They are used for grouping cycle-member stages within
/// a single materialisation.
pub type SccId = Id<Scc>;
