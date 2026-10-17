//! Organize ("Rapikan"): propose groups for the loose files of one folder,
//! move them into subfolders on request, record every move, undo it.
//! Moves never leave the target folder; see the design spec.

mod group;
mod scan;

pub use group::{
    Confidence, Group, GroupSource, Naming, OrganizePlan, apply_naming, kind_folder, propose,
    sanitize_name,
};
pub use scan::{FolderListing, OrganizeEntry, Unit, scan_folder};
