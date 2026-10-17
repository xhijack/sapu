//! Safety layer: the single gate through which files leave the disk.
//!
//! Invariants (PRD §8, CLAUDE.md "Aturan keamanan"):
//! - S1 trash by default, permanent deletion needs an explicit token;
//! - S2 every non-dry-run session is recorded and restorable where the OS allows;
//! - S3 protected paths are absolute and append-only;
//! - S4 plans are dry-run unless explicitly applied;
//! - S5 every item is re-validated right before disposal.
//!
//! Typical use:
//! ```no_run
//! use sapu_core::progress::NoProgress;
//! use sapu_core::safety::{DeletePlan, FileSnapshot, HistoryStore, ProtectedPaths, Safety};
//!
//! # fn main() -> sapu_core::error::Result<()> {
//! let safety = Safety::new(ProtectedPaths::os_defaults(), HistoryStore::open_default()?);
//! let plan = DeletePlan::new("/tmp/scope").with_item(FileSnapshot::capture("/tmp/scope/junk")?);
//! let session = safety.execute(&plan, &mut NoProgress)?; // dry run: nothing removed
//! let session = safety.execute(&plan.apply(), &mut NoProgress)?; // moved to trash
//! # let _ = session;
//! # Ok(())
//! # }
//! ```

mod dispose;
mod execute;
mod history;
mod protected;
mod snapshot;
mod validate;

pub use dispose::{Disposal, DisposalKind, PermanentConfirmation};
pub use execute::{DeletePlan, Safety};
pub use history::{CleanSession, HistoryStore, Outcome, RestoreReport, SessionEntry};
pub use protected::{ProtectedEntry, ProtectedPaths, Protection};
pub use snapshot::FileSnapshot;
pub use validate::{ValidationFailure, revalidate};
