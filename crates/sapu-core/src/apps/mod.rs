//! Installed applications and their leftovers (Tahap 2 P2, redesign R4).
//! Listing and matching only; removal goes through `safety` as always.

mod arch;
mod leftovers;
mod list;

pub use arch::{AppSupport, Arches, arches_from_header, bundle_arches, support_for};
pub use leftovers::{Leftover, LeftoverKind, leftovers_for, orphan_leftovers};
pub use list::{InstalledApp, is_running, list_apps};
