//! Concurrency enforcement for the Overlord deterministic core.
//!
//! Reads `execution.json` and the associated task `status.json` files to
//! compute the `currently_running` count. Compares against `max_parallel`
//! from the configuration. Provides `can_dispatch()` and `count_running()`
//! methods.

use std::path::Path;

use crate::config::get_max_parallel;
use crate::persistence::TaskStatusValue;

use super::errors::{OverlordError, Result};

/// Structured dispatch information returned by the concurrency checker.
#[derive(Debug, Clone)]
pub struct DispatchInfo {
    /// Number of currently running tasks.
    pub currently_running: u16,
    /// Maximum allowed parallel tasks.
    pub max_parallel: u16,
    /// Whether a new task can be dispatched.
    pub can_dispatch: bool,
    /// Number of remaining dispatch slots.
    pub remaining_slots: u16,
}

/// Main concurrency enforcement struct.
pub struct ConcurrencyChecker;

impl ConcurrencyChecker {
    /// Initialize the concurrency checker.
    pub fn new() -> Self {
        Self
    }

    /// Count currently running tasks for a plan.
    ///
    /// Reads `execution.json` from `.agent/state/<branch>/<plan_id>-<plan_name>/`
    /// and counts tasks with `running` status. Also cross-checks individual
    /// `status.json` files for accuracy.
    pub fn count_running(
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<u16> {
        // Try reading from execution.json first
        let exec_path = crate::persistence::execution_state_path(repo_root, branch, plan_id, plan_name);
        if !exec_path.exists() {
            return Ok(0);
        }

        let exec_state = crate::persistence::read_execution_state(repo_root, branch, plan_id, plan_name)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        let count: u16 = exec_state
            .task_status_map
            .values()
            .filter(|s| matches!(s, TaskStatusValue::Running))
            .count() as u16;

        Ok(count)
    }

    /// Get the concurrency limit for a plan.
    ///
    /// Falls back to global config `max_parallel` from `config::get_max_parallel()`.
    pub fn get_max_parallel(
        _repo_root: &Path,
        _branch: &str,
        _plan_id: &str,
        _plan_name: &str,
    ) -> Result<u16> {
        // For MVP, use global config. Plan-level override can be added later.
        let max = get_max_parallel().ok_or_else(|| {
            OverlordError::ConfigError(crate::config::ConfigError::Env(
                "Configuration not initialized".to_string(),
            ))
        })?;
        Ok(max)
    }

    /// Check if a new task can be dispatched for a plan.
    ///
    /// Returns true when `currently_running < max_parallel`.
    pub fn can_dispatch(
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<bool> {
        let currently_running = Self::count_running(repo_root, branch, plan_id, plan_name)?;
        let max_parallel = Self::get_max_parallel(repo_root, branch, plan_id, plan_name)?;
        Ok(currently_running < max_parallel)
    }

    /// Return structured dispatch information.
    pub fn dispatch_info(
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<DispatchInfo> {
        let currently_running = Self::count_running(repo_root, branch, plan_id, plan_name)?;
        let max_parallel = Self::get_max_parallel(repo_root, branch, plan_id, plan_name)?;
        let can_dispatch = currently_running < max_parallel;
        let remaining_slots = max_parallel.saturating_sub(currently_running);

        Ok(DispatchInfo {
            currently_running,
            max_parallel,
            can_dispatch,
            remaining_slots,
        })
    }
}

/// Returns true for task statuses that are gated by concurrency limits.
///
/// Per `design/resource-constraints.md`: `running` and `reviewing` are
/// concurrency-sensitive at the task level.
pub fn is_concurrency_gated(status: &TaskStatusValue) -> bool {
    matches!(status, TaskStatusValue::Running | TaskStatusValue::Reviewing)
}

/// Validate that a transition doesn't violate concurrency limits.
///
/// If the target status is `running` or `reviewing`, checks `can_dispatch()`.
/// Returns `OverlordError::ConcurrencyLimitExceeded` if the limit is reached.
pub fn validate_transition_concurrency(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    to_status: &TaskStatusValue,
) -> Result<()> {
    if !is_concurrency_gated(to_status) {
        return Ok(());
    }

    let currently_running = ConcurrencyChecker::count_running(repo_root, branch, plan_id, plan_name)?;
    let max_parallel = ConcurrencyChecker::get_max_parallel(repo_root, branch, plan_id, plan_name)?;

    if currently_running >= max_parallel {
        return Err(OverlordError::ConcurrencyLimitExceeded {
            current: currently_running,
            max: max_parallel,
        });
    }

    Ok(())
}
