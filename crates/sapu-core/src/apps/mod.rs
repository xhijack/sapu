//! Installed applications and their leftovers (Tahap 2 P2, redesign R4).
//! Listing and matching only; removal goes through `safety` as always.

mod leftovers;
mod list;

pub use leftovers::{Leftover, LeftoverKind, leftovers_for, orphan_leftovers};
pub use list::{InstalledApp, is_running, list_apps};
