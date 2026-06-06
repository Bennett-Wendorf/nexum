//! Typed status machines for plans and tasks.
//!
//! Validates transitions against the allowed transition maps from
//! `design/work-statuses.md`. Provides `can_transition()` and `transition()`
//! methods for both plan and task status machines.

use std::collections::HashMap;

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
/// Valid transitions per `design/work-statuses.md`:
/// - `draft` → `queued`
/// - `queued` → `planning`
/// - `planning` → `reviewing`
/// - `reviewing` → `approved` OR `rejected`
/// - `approved` → `complete` OR `rejected`
/// - `complete` — terminal, no outgoing transitions
/// - `rejected` — terminal, no outgoing transitions
pub struct PlanStateMachine {
    transitions: HashMap<PlanStatus, Vec<PlanStatus>>,
}

impl PlanStateMachine {
    /// Initialize with the transition map from `design/work-statuses.md`.
    pub fn new() -> Self {
        let mut transitions = HashMap::new();
        // Pre-planning transitions
        transitions.insert(PlanStatus::Draft, vec![PlanStatus::Queued]);
        transitions.insert(PlanStatus::Queued, vec![PlanStatus::Planning]);
        transitions.insert(PlanStatus::Planning, vec![PlanStatus::Reviewing]);
        transitions.insert(
            PlanStatus::Reviewing,
            vec![PlanStatus::Approved, PlanStatus::Rejected],
        );
        // Post-planning transitions
        transitions.insert(
            PlanStatus::Approved,
            vec![PlanStatus::Complete, PlanStatus::Rejected],
        );
        // Terminal states — no outgoing transitions
        // PlanStatus::Complete and PlanStatus::Rejected have no transitions
        Self { transitions }
    }

    /// Check if a transition from `from` to `to` is valid.
    pub fn can_transition(&self, from: &PlanStatus, to: &PlanStatus) -> bool {
        self.transitions
            .get(from)
            .map(|targets| targets.contains(to))
            .unwrap_or(false)
    }

    /// Create a transition record, validating the transition first.
    pub fn transition(
        &self,
        from: &PlanStatus,
        to: &PlanStatus,
        by: &str,
    ) -> Result<StatusTransitionRecord> {
        if !self.can_transition(from, to) {
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

    /// Expose the transition map for testing.
    pub fn all_transitions(&self) -> &HashMap<PlanStatus, Vec<PlanStatus>> {
        &self.transitions
    }
}

// ── Task Status Machine ─────────────────────────────────────────────────────

/// State machine for task lifecycle transitions.
///
/// Valid transitions per `design/work-statuses.md`:
/// - `backlog` → `queued`
/// - `queued` → `running`
/// - `running` → `reviewing`
/// - `reviewing` → `waiting-manual-review` OR `merge-queue`
/// - `waiting-manual-review` → `merge-queue` OR `abandoned`
/// - `merge-queue` → `completed`
/// - `abandoned` — terminal, no outgoing transitions
/// - `completed` — terminal, no outgoing transitions
pub struct TaskStateMachine {
    transitions: HashMap<TaskStatusValue, Vec<TaskStatusValue>>,
}

impl TaskStateMachine {
    /// Initialize with the transition map from `design/work-statuses.md`.
    pub fn new() -> Self {
        let mut transitions = HashMap::new();
        transitions.insert(TaskStatusValue::Backlog, vec![TaskStatusValue::Queued]);
        transitions.insert(TaskStatusValue::Queued, vec![TaskStatusValue::Running]);
        transitions.insert(
            TaskStatusValue::Running,
            vec![TaskStatusValue::Reviewing, TaskStatusValue::Queued],
        );
        transitions.insert(
            TaskStatusValue::Reviewing,
            vec![
                TaskStatusValue::WaitingManualReview,
                TaskStatusValue::MergeQueue,
            ],
        );
        transitions.insert(
            TaskStatusValue::WaitingManualReview,
            vec![TaskStatusValue::MergeQueue, TaskStatusValue::Abandoned],
        );
        transitions.insert(TaskStatusValue::MergeQueue, vec![TaskStatusValue::Completed]);
        // Terminal states — no outgoing transitions
        // TaskStatusValue::Abandoned and TaskStatusValue::Completed have no transitions
        Self { transitions }
    }

    /// Check if a transition from `from` to `to` is valid.
    pub fn can_transition(&self, from: &TaskStatusValue, to: &TaskStatusValue) -> bool {
        self.transitions
            .get(from)
            .map(|targets| targets.contains(to))
            .unwrap_or(false)
    }

    /// Create a transition record, validating the transition first.
    pub fn transition(
        &self,
        from: &TaskStatusValue,
        to: &TaskStatusValue,
        by: &str,
    ) -> Result<StatusTransitionRecord> {
        if !self.can_transition(from, to) {
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

    /// Expose the transition map for testing.
    pub fn all_transitions(&self) -> &HashMap<TaskStatusValue, Vec<TaskStatusValue>> {
        &self.transitions
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

/// Convert a TaskStatusValue to its kebab-case string representation.
pub fn task_status_to_string(status: &TaskStatusValue) -> &'static str {
    match status {
        TaskStatusValue::Backlog => "backlog",
        TaskStatusValue::Queued => "queued",
        TaskStatusValue::Running => "running",
        TaskStatusValue::Reviewing => "reviewing",
        TaskStatusValue::WaitingManualReview => "waiting-manual-review",
        TaskStatusValue::MergeQueue => "merge-queue",
        TaskStatusValue::Abandoned => "abandoned",
        TaskStatusValue::Completed => "completed",
    }
}

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
