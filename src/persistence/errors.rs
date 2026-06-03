//! Error types for the persistence layer.
//!
//! This module defines [`PersistenceError`] which covers all error cases
//! that can arise from file system I/O, JSON parsing, markdown parsing,
//! and schema validation within the persistence layer.

use std::io;
use std::path::PathBuf;

use thiserror::Error;

/// Errors that can occur during persistence operations.
///
/// Each variant carries contextual information to help diagnose the problem,
/// typically including a file path where the error occurred.
#[derive(Debug, Error)]
pub enum PersistenceError {
    /// A file system I/O error occurred at the given path.
    #[error("I/O error at {0}: {1}")]
    Io(PathBuf, #[source] io::Error),

    /// A bare I/O error without path context.
    #[error("I/O error: {0}")]
    IoBare(#[source] io::Error),

    /// JSON deserialization failed for the file at the given path.
    #[error("JSON parse error at {0}: {1}")]
    JsonParse(PathBuf, #[source] serde_json::Error),

    /// A bare JSON parse error without path context.
    #[error("JSON parse error: {0}")]
    JsonParseBare(#[source] serde_json::Error),

    /// JSON serialization failed for the file at the given path.
    #[error("JSON serialize error at {0}: {1}")]
    JsonSerialize(PathBuf, #[source] serde_json::Error),

    /// Markdown parsing failed for the file at the given path.
    #[error("Markdown parse error at {0}: {1}")]
    MarkdownParse(PathBuf, String),

    /// The expected directory does not exist at the given path.
    #[error("Directory not found: {0}")]
    DirectoryNotFound(PathBuf),

    /// The expected file does not exist at the given path.
    #[error("File not found: {0}")]
    FileNotFound(PathBuf),

    /// Schema validation failed for the given reason.
    #[error("Schema validation: {0}")]
    SchemaValidation(String),

    /// Path resolution failed for the given reason.
    #[error("Path resolution: {0}")]
    PathResolution(String),

    /// An atomic write (rename) failed at the given path.
    #[error("Atomic write error at {0}: {1}")]
    AtomicWrite(PathBuf, #[source] io::Error),

    /// A concurrency conflict: the file was modified by another process.
    #[error("Concurrency conflict at {0}: file was modified by another process")]
    ConcurrencyConflict(PathBuf),
}

/// Convert a bare `io::Error` into a `PersistenceError::IoBare`.
impl From<io::Error> for PersistenceError {
    fn from(err: io::Error) -> Self {
        PersistenceError::IoBare(err)
    }
}

/// Convert a bare `serde_json::Error` into a `PersistenceError::JsonParseBare`.
impl From<serde_json::Error> for PersistenceError {
    fn from(err: serde_json::Error) -> Self {
        PersistenceError::JsonParseBare(err)
    }
}

/// Result type alias for persistence operations.
pub type Result<T> = std::result::Result<T, PersistenceError>;
