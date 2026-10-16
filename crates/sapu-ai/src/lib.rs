//! Sapu's optional AI layer.
//!
//! Hard rules (PRD §12.3): this crate has no dependency on `sapu-core`, so
//! it cannot remove files or reach the rules/cleaner engine; it only turns
//! metadata into suggestions. Its verdict type has no "safe" variant, so an
//! AI answer can never be shown green. Only metadata is sent, after
//! anonymisation, and the exact payload can be previewed first.

pub mod anonymize;
pub mod cache;
pub mod client;
pub mod config;
pub mod keys;
pub mod provider;
pub mod tasks;

pub use anonymize::Anonymizer;
pub use client::{AiClient, Preview};
pub use config::{AiConfig, AiMode, CloudKind};
pub use provider::{Provider, ProviderError};
pub use tasks::{Task, Verdict};

#[derive(Debug, thiserror::Error)]
pub enum AiError {
    #[error("AI is turned off")]
    Disabled,
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("the model answered in an unexpected format: {0}")]
    BadAnswer(String),
    #[error("keychain error: {0}")]
    Keychain(String),
    #[error("i/o error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AiError>;
