//! Declarative clean-up rules (Tahap 2). Rules are TOML files embedded at
//! build time; see `docs/superpowers/specs/2026-10-16-tahap-2-clean-design.md`.

pub(crate) mod expand;
mod load;
mod schema;

pub use expand::Env;
pub use load::{RuleSet, load_embedded, parse_rules};
pub use schema::{Command, Localized, Match, MatchKind, Profile, Risk, Rule, RuleError};
