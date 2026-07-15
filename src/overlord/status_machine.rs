//! Typed status machines for plans and tasks.
//!
//! Zero-sized types (ZSTs) that validate transitions via match
//! expressions instead of runtime HashMap lookups. Provides `can_transition()`
//! and `transition()` static methods for both plan and task status machines.

use crate::persistence::{PlanStatus, TaskStatusValue};

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
/// Zero-sized type using match expressions for transition validation.
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
    /// Check if a transition from `from` to `to` is valid.
    pub fn can_transition(from: &PlanStatus, to: &PlanStatus) -> bool {
        matches!(
            (from, to),
            (PlanStatus::Draft, PlanStatus::Queued)
                | (PlanStatus::Queued, PlanStatus::Planning)
                | (PlanStatus::Planning, PlanStatus::Reviewing)
                | (
                    PlanStatus::Reviewing,
                    PlanStatus::Approved | PlanStatus::Rejected
                )
                | (
                    PlanStatus::Approved,
                    PlanStatus::Complete | PlanStatus::Rejected
                )
        )
    }

    /// Create a transition record, validating the transition first.
    pub fn transition(
        from: &PlanStatus,
        to: &PlanStatus,
        by: &str,
    ) -> Result<StatusTransitionRecord> {
        if !Self::can_transition(from, to) {
            return Err(OverlordError::InvalidTransition {
                from: from.to_string(),
                to: to.to_string(),
                entity: "plan".to_string(),
            });
        }
        Ok(StatusTransitionRecord {
            from: from.to_string(),
            to: to.to_string(),
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
/// Zero-sized type using match expressions for transition validation.
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
    /// Check if a transition from `from` to `to` is valid.
    pub fn can_transition(from: &TaskStatusValue, to: &TaskStatusValue) -> bool {
        matches!(
            (from, to),
            (TaskStatusValue::Backlog, TaskStatusValue::Queued)
                | (TaskStatusValue::Queued, TaskStatusValue::Running)
                | (
                    TaskStatusValue::Running,
                    TaskStatusValue::Reviewing | TaskStatusValue::Queued
                )
                | (
                    TaskStatusValue::Reviewing,
                    TaskStatusValue::WaitingManualReview | TaskStatusValue::MergeQueue
                )
                | (
                    TaskStatusValue::WaitingManualReview,
                    TaskStatusValue::MergeQueue | TaskStatusValue::Abandoned
                )
                | (TaskStatusValue::MergeQueue, TaskStatusValue::Completed)
        )
    }

    /// Create a transition record, validating the transition first.
    pub fn transition(
        from: &TaskStatusValue,
        to: &TaskStatusValue,
        by: &str,
    ) -> Result<StatusTransitionRecord> {
        if !Self::can_transition(from, to) {
            return Err(OverlordError::InvalidTransition {
                from: from.to_string(),
                to: to.to_string(),
                entity: "task".to_string(),
            });
        }
        Ok(StatusTransitionRecord {
            from: from.to_string(),
            to: to.to_string(),
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
    matches!(status, PlanStatus::Planning | PlanStatus::Reviewing)
}


