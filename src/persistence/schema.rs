//! Typed data structures for the persistence layer.
//!
//! This module defines all structs and enums that are read from or written
//! to the `.agent/` directory hierarchy. Each type derives `Serialize` and
//! `Deserialize` for JSON round-tripping, and `Debug` / `Clone` for
//! convenience.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// ── Plan ───────────────────────────────────────────────────────────────────

/// A high-level plan that groups related tasks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plan {
    /// Unique plan identifier, e.g. `"PLAN-001"`.
    pub id: String,
    /// Human-readable plan name, e.g. `"oauth2-flow"`.
    pub name: String,
    /// Current lifecycle status of the plan.
    pub status: PlanStatus,
    /// ISO 8601 date when the plan was created.
    pub created: String,
    /// Git branch associated with this plan.
    pub branch: String,
    /// High-level goal statement for the plan.
    pub goal: String,
    /// Scope description for the plan.
    pub scope: String,
    /// Background / context for the plan.
    #[serde(default)]
    pub background: String,
    /// Ordered list of tasks belonging to this plan.
    #[serde(default)]
    pub tasks: Vec<TaskReference>,
}

/// A lightweight reference to a task within a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskReference {
    /// Task identifier, e.g. `"TASK-001"`.
    pub id: String,
    /// Short task name.
    pub name: String,
    /// Whether the task has been completed.
    pub completed: bool,
}

/// Lifecycle status of a plan.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PlanStatus {
    /// Rough idea, user-created, sparse content.
    Draft,
    /// Waiting for planner agent.
    Queued,
    /// Planner is working on it.
    Planning,
    /// Plan ready, waiting for human approval.
    Reviewing,
    /// Human approved, tasks flow to task backlog.
    Approved,
    /// All tasks done (terminal).
    Complete,
    /// Human rejected (terminal).
    Rejected,
}

// ── Task ────────────────────────────────────────────────────────────────────

/// A single task within a plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    /// Unique task identifier, e.g. `"TASK-001"`.
    pub id: String,
    /// Short task name.
    pub name: String,
    /// ID of the parent plan, e.g. `"PLAN-001"`.
    pub parent_plan: String,
    /// IDs of tasks this task depends on.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// Detailed task description.
    pub description: String,
    /// List of acceptance criteria that must be met.
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    /// List of file paths that this task will modify.
    #[serde(default)]
    pub files_to_modify: Vec<String>,
    /// Background / context for this task.
    #[serde(default)]
    pub background: String,
    /// Additional notes for this task.
    #[serde(default)]
    pub notes: String,
}

// ── Task Status ─────────────────────────────────────────────────────────────

/// Current runtime status of a task (stored in `status.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStatus {
    /// Task identifier.
    pub id: String,
    /// Current status value.
    pub status: TaskStatusValue,
    /// Active agent lease, if any agent is working on this task.
    #[serde(default)]
    pub agent: Option<AgentLease>,
    /// History of status transitions.
    #[serde(default)]
    pub transitions: Vec<StatusTransition>,
    /// ISO 8601 timestamp when the task started running (if applicable).
    #[serde(default)]
    pub started_at: Option<String>,
    /// ISO 8601 timestamp when the task completed (if applicable).
    #[serde(default)]
    pub completed_at: Option<String>,
    /// Number of execution attempts.
    pub attempts: u32,
    /// IDs of tasks this task depends on.
    #[serde(default)]
    pub dependencies: Vec<String>,
    /// IDs of tasks that depend on this task.
    #[serde(default)]
    pub dependent_tasks: Vec<String>,
    /// Last heartbeat timestamp from the executing agent.
    #[serde(default)]
    pub heartbeat_at: Option<String>,
}

/// Possible values for a task's runtime status.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TaskStatusValue {
    /// The task has not yet been prioritised.
    Backlog,
    /// The task is queued for execution.
    Queued,
    /// The task is currently being worked on.
    Running,
    /// The task is under review.
    Reviewing,
    /// The task is waiting for manual review.
    WaitingManualReview,
    /// The task is in the merge queue.
    MergeQueue,
    /// The task has been abandoned.
    Abandoned,
    /// The task has been completed successfully.
    Completed,
}

/// Lease held by an agent working on a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLease {
    /// Role of the agent, e.g. `"builder"`.
    pub role: String,
    /// Process ID of the agent.
    pub pid: u32,
    /// ISO 8601 timestamp when the lease was acquired.
    pub leased_at: String,
}

/// A single status transition event (works for both plan and task transitions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusTransition {
    /// Previous status (as a string, e.g. "queued" or "planning").
    pub from: String,
    /// New status (as a string, e.g. "running" or "approved").
    pub to: String,
    /// ISO 8601 timestamp of the transition.
    pub at: String,
    /// Actor that performed the transition, e.g. "builder" or "overlord".
    pub by: String,
}

// ── Execution State ─────────────────────────────────────────────────────────

/// Top-level execution state for a plan (stored in `execution.json`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionState {
    /// ID of the plan this state belongs to.
    pub plan_id: String,
    /// Git branch associated with the plan.
    pub branch: String,
    /// Ordered list of task IDs.
    #[serde(default)]
    pub tasks: Vec<String>,
    /// Map from task ID to its current status.
    #[serde(default)]
    pub task_status_map: HashMap<String, TaskStatusValue>,
}
