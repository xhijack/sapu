//! Organize ("Rapikan"): propose groups for the loose files of one folder,
//! move them into subfolders on request, record every move, undo it.
//! Moves never leave the target folder; see the design spec.

mod advice;
mod apply;
mod dupes;
mod group;
mod history;
mod plan;
mod scan;

pub use advice::{ItemAdvice, plan_from_advice};
pub use apply::{ApplyReport, UndoReport, apply, undo};
pub use dupes::duplicates_in;
pub use group::{
    Confidence, Group, GroupSource, Naming, OrganizePlan, apply_naming, kind_folder, propose,
    sanitize_name,
};
pub use history::{MoveRecord, OrganizeSession, OrganizeStore};
pub use plan::{Move, MovePlan, Selection, build_move_plan, renamed_file};
pub use scan::{FolderListing, OrganizeEntry, Unit, scan_folder};
