//! Typed status machines for plans and tasks.
//!
//! Zero-sized types (ZSTs) that validate transitions via compile-time match
//! expressions instead of runtime HashMap lookups. Provides `can_transition()`
//! and `transition()` static methods for both plan and task status machines.

use crate::persistence::{PlanStatus, TaskStatusValue, task_status_to_string};

use super::errors::{OverlordError, Result};

/// A transition record created by the status machine.
#[derive(Debug, Clone)]
pub struct StatusTransitionRecord {
    pub from: String,
    pub to: String,
    pub at: String,
    pub by: String,
}

// ── Plan Status Machine ─────────────────────────────────────────────────────

/// State machine for plan lifecycle transitions.
///
/// Zero-sized type using match expressions for compile-time transition validation.
///
/// Valid transitions per `design/work-statuses.md`:
/// - `draft` → `queued`
/// - `queued` → `planning`
/// - `planning` → `reviewing`
/// - `reviewing` → `approved` OR `rejected`
/// - `approved` → `complete` OR `rejected`
/// - `complete` — terminal, no outgoing transitions
/// - `rejected` — terminal, no outgoing transitions
pub struct PlanStateMachine;

impl PlanStateMachine {
    /// Create the state machine (returns a ZST, kept for backward compatibility).
    pub fn new() -> Self {
        PlanStateMachine
    }

    /// Check if a transition from `from` to `to` is valid.
    pub fn can_transition(from: &PlanStatus, to: &PlanStatus) -> bool {
        match (from, to) {
            (PlanStatus::Draft, PlanStatus::Queued) => true,
            (PlanStatus::Queued, PlanStatus::Planning) => true,
            (PlanStatus::Planning, PlanStatus::Reviewing) => true,
            (PlanStatus::Reviewing, PlanStatus::Approved | PlanStatus::Rejected) => true,
            (PlanStatus::Approved, PlanStatus::Complete | PlanStatus::Rejected) => true,
            _ => false,
        }
    }

    /// Create a transition record, validating the transition first.
    pub fn transition(
        from: &PlanStatus,
        to: &PlanStatus,
        by: &str,
    ) -> Result<StatusTransitionRecord> {
        if !Self::can_transition(from, to) {
            return Err(OverlordError::InvalidTransition {
                from: plan_status_to_string(from).to_string(),
                to: plan_status_to_string(to).to_string(),
                entity: "plan".to_string(),
            });
        }
        Ok(StatusTransitionRecord {
            from: plan_status_to_string(from).to_string(),
            to: plan_status_to_string(to).to_string(),
            at: chrono::Utc::now().to_rfc3339(),
            by: by.to_string(),
        })
    }

    /// Returns true for terminal states (`complete` and `rejected`).
    pub fn is_terminal(status: &PlanStatus) -> bool {
        matches!(status, PlanStatus::Complete | PlanStatus::Rejected)
    }
}

// ── Task Status Machine ─────────────────────────────────────────────────────

/// State machine for task lifecycle transitions.
///
/// Zero-sized type using match expressions for compile-time transition validation.
///
/// Valid transitions per `design/work-statuses.md`:
/// - `backlog` → `queued`
/// - `queued` → `running`
/// - `running` → `reviewing` OR `queued` (re-queue)
/// - `reviewing` → `waiting-manual-review` OR `merge-queue`
/// - `waiting-manual-review` → `merge-queue` OR `abandoned`
/// - `merge-queue` → `completed`
/// - `abandoned` — terminal, no outgoing transitions
/// - `completed` — terminal, no outgoing transitions
pub struct TaskStateMachine;

impl TaskStateMachine {
    /// Create the state machine (returns a ZST, kept for backward compatibility).
    pub fn new() -> Self {
        TaskStateMachine
    }

    /// Check if a transition from `from` to `to` is valid.
    pub fn can_transition(from: &TaskStatusValue, to: &TaskStatusValue) -> bool {
        match (from, to) {
            (TaskStatusValue::Backlog, TaskStatusValue::Queued) => true,
            (TaskStatusValue::Queued, TaskStatusValue::Running) => true,
            (TaskStatusValue::Running, TaskStatusValue::Reviewing | TaskStatusValue::Queued) => true,
            (TaskStatusValue::Reviewing, TaskStatusValue::WaitingManualReview | TaskStatusValue::MergeQueue) => true,
            (TaskStatusValue::WaitingManualReview, TaskStatusValue::MergeQueue | TaskStatusValue::Abandoned) => true,
            (TaskStatusValue::MergeQueue, TaskStatusValue::Completed) => true,
            _ => false,
        }
    }

    /// Create a transition record, validating the transition first.
    pub fn transition(
        from: &TaskStatusValue,
        to: &TaskStatusValue,
        by: &str,
    ) -> Result<StatusTransitionRecord> {
        if !Self::can_transition(from, to) {
            return Err(OverlordError::InvalidTransition {
                from: task_status_to_string(from).to_string(),
                to: task_status_to_string(to).to_string(),
                entity: "task".to_string(),
            });
        }
        Ok(StatusTransitionRecord {
            from: task_status_to_string(from).to_string(),
            to: task_status_to_string(to).to_string(),
            at: chrono::Utc::now().to_rfc3339(),
            by: by.to_string(),
        })
    }

    /// Returns true for terminal states (`completed` and `abandoned`).
    pub fn is_terminal(status: &TaskStatusValue) -> bool {
        matches!(
            status,
            TaskStatusValue::Completed | TaskStatusValue::Abandoned
        )
    }
}

// ── Concurrency-Sensitive Status Checks ─────────────────────────────────────

/// Returns true for task statuses that are gated by concurrency limits.
///
/// Per `design/resource-constraints.md`: `running` and `reviewing` are
/// concurrency-sensitive at the task level.
pub fn is_concurrency_sensitive(status: &TaskStatusValue) -> bool {
    matches!(
        status,
        TaskStatusValue::Running | TaskStatusValue::Reviewing
    )
}

/// Returns true for plan statuses that are gated by concurrency limits.
///
/// Per `design/resource-constraints.md`: `planning` and `reviewing` are
/// concurrency-sensitive at the plan level.
pub fn is_plan_concurrency_sensitive(status: &PlanStatus) -> bool {
    matches!(
        status,
        PlanStatus::Planning | PlanStatus::Reviewing
    )
}

// ── Status-to-String Conversion ─────────────────────────────────────────────

/// Convert a PlanStatus to its kebab-case string representation.
pub fn plan_status_to_string(status: &PlanStatus) -> &'static str {
    match status {
        PlanStatus::Draft => "draft",
        PlanStatus::Queued => "queued",
        PlanStatus::Planning => "planning",
        PlanStatus::Reviewing => "reviewing",
        PlanStatus::Approved => "approved",
        PlanStatus::Complete => "complete",
        PlanStatus::Rejected => "rejected",
    }
}
