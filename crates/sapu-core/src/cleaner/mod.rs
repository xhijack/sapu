//! Clean-up engine (Tahap 2): evaluates rules into concrete findings and
//! executes a selection through the safety layer or official commands.

mod command;
mod evaluate;
mod execute;

pub use command::{CommandOutcome, run_command};
pub use evaluate::{Candidate, CleanReport, CommandFinding, Finding, evaluate};
pub use execute::{CleanRunReport, RuleOutcome, Selection, execute};
