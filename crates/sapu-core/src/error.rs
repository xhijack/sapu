//! Crate-wide error type.

use std::path::PathBuf;

/// Errors produced by `sapu-core`.
#[derive(Debug, thiserror::Error)]
pub enum SapuError {
    /// The path is inside the protected set and may never be touched.
    #[error("path is protected: {0}")]
    Protected(PathBuf),

    /// Re-validation right before disposal failed; the item was skipped.
    #[error("re-validation failed for {path}: {reason}")]
    Validation { path: PathBuf, reason: String },

    /// Moving to trash or permanent deletion failed.
    #[error("disposal failed for {path}: {source}")]
    Disposal {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    /// Restoring from trash is not supported on this platform.
    #[error("restoring from trash is not supported on this platform")]
    RestoreUnsupported,

    /// Generic I/O error with context.
    #[error("{context}: {source}")]
    Io {
        context: String,
        #[source]
        source: std::io::Error,
    },

    /// Serialization error (history store).
    #[error("serialization error: {0}")]
    Serde(#[from] serde_json::Error),
}

impl SapuError {
    pub(crate) fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}

/// Convenience alias.
pub type Result<T> = std::result::Result<T, SapuError>;
