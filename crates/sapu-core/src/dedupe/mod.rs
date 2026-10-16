//! Duplicate finder (Tahap 3): size → partial hash → full BLAKE3, and a
//! plan type that can never remove every copy of a group.

mod group;
mod keep;
mod pipeline;
mod plan;

pub use group::{DupFile, DuplicateGroup, FolderPair, folder_pairs};
pub use keep::{KeepStrategy, choose_keep, name_score};
pub use pipeline::{DedupeOptions, DedupeResult, DedupeSummary, STAGES, Stage, find_duplicates};
pub use plan::{
    DedupePlan, DedupeRunReport, GroupOutcome, GroupPlan, PlanError, ReplaceMode, execute,
};
