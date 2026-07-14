//! Request and response DTOs for the Nexum REST API.
//!
//! This module defines all the data transfer objects used by the REST API
//! endpoints, including plan, task, execution state, and config types.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use serde::{Deserialize, Serialize};

use crate::builder::orchestrator::WorkflowOrchestrator;
use crate::config;

// ---------------------------------------------------------------------------
// Plan DTOs
// ---------------------------------------------------------------------------

/// Response payload for a single plan.
///
/// Returned by `GET /plans/{id}` and included in plan list responses.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlanResponse {
    /// Unique identifier of the plan.
    pub id: String,
    /// Human-readable name of the plan.
    pub name: String,
    /// Current status (e.g. "draft", "active", "completed", "cancelled").
    pub status: String,
    /// ISO-8601 timestamp when the plan was created.
    pub created: String,
    /// Git branch associated with this plan.
    pub branch: String,
    /// High-level goal the plan aims to achieve.
    pub goal: String,
    /// Description of the scope / boundaries of the work.
    pub scope: String,
    /// Background context explaining why this plan exists.
    pub background: String,
    /// Ordered list of tasks belonging to this plan.
    pub tasks: Vec<TaskReferenceResponse>,
}

/// Lightweight reference to a task inside a plan response.
///
/// Used to avoid embedding full task details in plan responses.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskReferenceResponse {
    /// Unique identifier of the task.
    pub id: String,
    /// Human-readable name of the task.
    pub name: String,
    /// Whether the task has been completed.
    pub completed: bool,
}

/// Request body for creating a new plan.
#[derive(Serialize, Deserialize, Debug)]
pub struct CreatePlanRequest {
    /// Name of the plan.
    pub name: String,
    /// Git branch to associate with the plan.
    pub branch: String,
    /// High-level goal the plan aims to achieve.
    pub goal: String,
    /// Optional description of the scope / boundaries.
    pub scope: Option<String>,
    /// Optional background context.
    pub background: Option<String>,
}

/// Request body for updating an existing plan.
///
/// Only the fields that are `Some` will be updated.
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdatePlanRequest {
    /// New name for the plan.
    pub name: Option<String>,
    /// New goal statement.
    pub goal: Option<String>,
    /// New scope description.
    pub scope: Option<String>,
    /// New background context.
    pub background: Option<String>,
}

/// Request body for transitioning a plan's status.
#[derive(Serialize, Deserialize, Debug)]
pub struct TransitionPlanStatusRequest {
    /// Target status (e.g. "active", "completed", "cancelled").
    pub status: String,
    /// Optional identifier of the actor performing the transition.
    pub by: Option<String>,
}

// ---------------------------------------------------------------------------
// Task DTOs
// ---------------------------------------------------------------------------

/// Full response payload for a single task.
///
/// Returned by `GET /tasks/{id}`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskResponse {
    /// Unique identifier of the task.
    pub id: String,
    /// Human-readable name of the task.
    pub name: String,
    /// ID of the parent plan this task belongs to.
    pub parent_plan: String,
    /// IDs of tasks that must complete before this one.
    pub dependencies: Vec<String>,
    /// Detailed description of what the task entails.
    pub description: String,
    /// List of acceptance criteria that define completion.
    pub acceptance_criteria: Vec<String>,
    /// List of file paths this task is expected to modify.
    pub files_to_modify: Vec<String>,
    /// Background context specific to this task.
    pub background: String,
    /// Free-form notes.
    pub notes: String,
    /// Current status with agent lease and transition history.
    pub status: TaskStatusResponse,
}

/// Detailed status information for a task.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TaskStatusResponse {
    /// ID of the task this status belongs to.
    pub id: String,
    /// Current status label (e.g. "pending", "in_progress", "completed").
    pub status: String,
    /// Agent currently executing this task, if any.
    pub agent: Option<AgentLeaseResponse>,
    /// History of status transitions.
    pub transitions: Vec<StatusTransitionResponse>,
    /// ISO-8601 timestamp when execution started.
    pub started_at: Option<String>,
    /// ISO-8601 timestamp when execution completed.
    pub completed_at: Option<String>,
    /// Number of execution attempts.
    pub attempts: u32,
    /// IDs of tasks this task depends on.
    pub dependencies: Vec<String>,
    /// IDs of tasks that depend on this one.
    pub dependent_tasks: Vec<String>,
    /// ISO-8601 timestamp of the last heartbeat.
    pub heartbeat_at: Option<String>,
}

/// Information about an agent currently leased to a task.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentLeaseResponse {
    /// Role of the agent (e.g. "coder", "reviewer").
    pub role: String,
    /// Process ID of the agent.
    pub pid: u32,
    /// ISO-8601 timestamp when the lease was acquired.
    pub leased_at: String,
}

/// A single status transition event in task history.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct StatusTransitionResponse {
    /// Previous status.
    pub from: String,
    /// New status.
    pub to: String,
    /// ISO-8601 timestamp of the transition.
    pub at: String,
    /// Identifier of the actor who performed the transition.
    pub by: String,
}

/// Request body for creating a new task within a plan.
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateTaskRequest {
    /// Name of the task.
    pub name: String,
    /// ID of the parent plan.
    pub parent_plan: String,
    /// IDs of tasks that must complete before this one.
    pub dependencies: Vec<String>,
    /// Detailed description of the task.
    pub description: String,
    /// Acceptance criteria defining completion.
    pub acceptance_criteria: Vec<String>,
    /// File paths this task will modify.
    pub files_to_modify: Vec<String>,
    /// Optional background context.
    pub background: Option<String>,
    /// Optional free-form notes.
    pub notes: Option<String>,
}

/// Request body for updating an existing task.
///
/// Only the fields that are `Some` will be updated.
#[derive(Serialize, Deserialize, Debug)]
pub struct UpdateTaskRequest {
    /// New name for the task.
    pub name: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// New acceptance criteria.
    pub acceptance_criteria: Option<Vec<String>>,
    /// New list of files to modify.
    pub files_to_modify: Option<Vec<String>>,
    /// New background context.
    pub background: Option<String>,
    /// New notes.
    pub notes: Option<String>,
}

/// Request body for transitioning a task's status.
#[derive(Serialize, Deserialize, Debug)]
pub struct TransitionTaskStatusRequest {
    /// Target status (e.g. "in_progress", "completed", "failed").
    pub status: String,
    /// Optional identifier of the actor performing the transition.
    pub by: Option<String>,
}

/// Request body for an agent to claim (lease) a task.
#[derive(Serialize, Deserialize, Debug)]
pub struct ClaimTaskRequest {
    /// Role of the claiming agent.
    pub agent_role: String,
    /// Process ID of the claiming agent.
    pub agent_pid: u32,
}

// ---------------------------------------------------------------------------
// Execution state DTOs
// ---------------------------------------------------------------------------

/// Snapshot of the current execution state for a plan.
///
/// Returned by `GET /plans/{id}/execution-state`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ExecutionStateResponse {
    /// ID of the plan.
    pub plan_id: String,
    /// Git branch being used for execution.
    pub branch: String,
    /// IDs of all tasks in the plan.
    pub tasks: Vec<String>,
    /// Mapping from task ID to current status label.
    pub task_status_map: HashMap<String, String>,
}

/// Information about a task currently being executed.
///
/// Returned by `GET /execution/running-tasks`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RunningTaskResponse {
    /// ID of the task.
    pub task_id: String,
    /// Name of the task.
    pub task_name: String,
    /// ID of the parent plan.
    pub plan_id: String,
    /// Git branch being used.
    pub branch: String,
    /// Agent currently executing the task, if any.
    pub agent: Option<AgentLeaseResponse>,
    /// ISO-8601 timestamp when execution started.
    pub started_at: Option<String>,
    /// ISO-8601 timestamp of the last heartbeat.
    pub heartbeat_at: Option<String>,
}

// ---------------------------------------------------------------------------
// Config DTOs
// ---------------------------------------------------------------------------

/// Response payload for the server configuration.
///
/// Returned by `GET /config`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ConfigResponse {
    /// Host the server binds to.
    pub server_host: String,
    /// Port the server listens on.
    pub server_port: u16,
    /// Maximum number of tasks that may execute in parallel.
    pub max_parallel: u16,
    /// Default timeout in seconds for task execution.
    pub default_timeout_seconds: u64,
    /// Log level (e.g. "info", "debug", "warn").
    pub log_level: String,
    /// Whether yolo mode is enabled (skips safety checks).
    pub yolo_mode: bool,
    /// Whether authentication is enabled on this server.
    pub auth_enabled: bool,
    /// Whether read endpoints require authentication.
    pub auth_require_read: bool,
    /// Number of configured API keys (for client awareness).
    pub auth_keys_count: usize,
}

/// Response payload for a single registered agent.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentRegistrationResponse {
    /// Human-readable name of the agent.
    pub name: String,
    /// Type / role of the agent.
    pub r#type: String,
    /// Command used to spawn the agent process.
    pub spawn_command: String,
    /// Optional list of tool permissions.
    pub tool_permissions: Option<Vec<String>>,
    /// Optional per-agent timeout in seconds.
    pub timeout_seconds: Option<u64>,
}

/// Response payload listing all registered agents.
///
/// Returned by `GET /config/agents`.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AgentsResponse {
    /// List of registered agents.
    pub agents: Vec<AgentRegistrationResponse>,
}

// ---------------------------------------------------------------------------
// Generic wrappers
// ---------------------------------------------------------------------------

/// Generic wrapper for paginated / list endpoints.
///
/// Wraps a vector of items together with the total count.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ListResponse<T> {
    /// The items in this page / list.
    pub items: Vec<T>,
    /// Total number of items matching the query.
    pub total: usize,
}

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

/// Shared application state accessible to all API handlers.
///
/// Cloned and attached to each request via Axum's `State` extractor.
#[derive(Clone)]
pub struct AppState {
    /// Absolute path to the Nexum repository root.
    pub repo_root: PathBuf,
    /// Loaded server configuration.
    pub config: config::Config,
    /// Builder workflow orchestrator for task execution.
    pub orchestrator: Option<Arc<WorkflowOrchestrator>>,
}

// ---------------------------------------------------------------------------
// Query parameter types
// ---------------------------------------------------------------------------

/// Query parameters for listing plans.
#[derive(Deserialize, Debug)]
pub struct PlanListQuery {
    /// Filter by git branch name.
    pub branch: Option<String>,
    /// Filter by plan status.
    pub status: Option<String>,
}

/// Query parameters for listing tasks.
#[derive(Deserialize, Debug)]
pub struct TaskListQuery {
    /// Filter by task status.
    pub status: Option<String>,
}
