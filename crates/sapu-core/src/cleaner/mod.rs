//! Clean-up engine (Tahap 2): evaluates rules into concrete findings and
//! executes a selection through the safety layer or official commands.

mod command;
mod evaluate;
mod execute;
mod projects;

pub use command::{CommandOutcome, run_command};
pub use evaluate::{
    Candidate, CleanReport, CommandFinding, EvalProgress, Finding, UnreadableDir, dir_size_pub,
    evaluate, evaluate_with, evaluate_with_progress,
};
pub use execute::{CleanRunReport, RuleOutcome, RunStep, Selection, execute, execute_with_steps};
pub use projects::{
    Artefact, Ecosystem, Project, ProjectScanOptions, find_projects, is_project_root,
};
