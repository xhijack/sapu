//! Clean-up engine (Tahap 2): evaluates rules into concrete findings and
//! executes a selection through the safety layer or official commands.

mod command;
mod evaluate;
mod execute;
mod projects;

pub use command::{CommandOutcome, run_command};
pub use evaluate::{Candidate, CleanReport, CommandFinding, Finding, evaluate, evaluate_with};
pub use execute::{CleanRunReport, RuleOutcome, Selection, execute};
pub use projects::{Artefact, Ecosystem, Project, ProjectScanOptions, find_projects};
