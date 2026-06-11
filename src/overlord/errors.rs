//! Error types for the Overlord deterministic core.
//!
//! This module defines [`OverlordError`] which covers all error cases
//! that can arise from status transitions, ID generation, concurrency
//! enforcement, dependency resolution, heartbeat monitoring, and scheduler
//! lifecycle management.

use thiserror::Error;

use crate::config::ConfigError;
use crate::persistence::PersistenceError;

/// Errors that can occur during overlord operations.
#[derive(Debug, Error)]
pub enum OverlordError {
    /// Attempted an invalid status transition.
    #[error("Invalid transition for {entity}: {from} -> {to}")]
    InvalidTransition {
        from: String,
        to: String,
        entity: String,
    },

    /// Cannot dispatch — at concurrency limit.
    #[error("Concurrency limit exceeded: {current} running, max {max}")]
    ConcurrencyLimitExceeded { current: u16, max: u16 },

    /// Failed to generate a stable ID.
    #[error("ID generation error: {0}")]
    IdGenerationError(String),

    /// Dependency resolution failure.
    #[error("Dependency error: {0}")]
    DependencyError(String),

    /// Heartbeat detection failure.
    #[error("Heartbeat error: {0}")]
    HeartbeatError(String),

    /// Wrapped persistence error.
    #[error("Persistence error: {0}")]
    PersistenceError(#[from] PersistenceError),

    /// Wrapped config error.
    #[error("Config error: {0}")]
    ConfigError(#[from] ConfigError),

    /// Scheduler lifecycle error.
    #[error("Scheduler error: {0}")]
    SchedulerError(String),
}

/// Result type alias for overlord operations.
pub type Result<T> = std::result::Result<T, OverlordError>;
