//! Sapu core engine.
//!
//! All business logic lives here. The GUI and CLI are thin layers that call
//! into this crate and render the structured data it returns.
//!
//! Architecture invariants (see `CLAUDE.md`):
//! - The only code path that removes files is [`safety`].
//! - Core never formats text for display; it emits data and [`progress`] events.

pub mod apps;
pub mod care;
pub mod cleaner;
pub mod dedupe;
pub mod error;
pub mod eta;
pub mod organize;
pub mod platform;
pub mod progress;
pub mod rules;
pub mod safety;
pub mod scanner;
pub mod settings;

pub use error::SapuError;
