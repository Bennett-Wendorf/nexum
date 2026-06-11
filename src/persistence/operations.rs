//! High-level CRUD operations for plans, tasks, and execution state.
//!
//! This module provides the primary API for interacting with persisted
//! data. It combines path resolution, markdown parsing/rendering, and
//! file I/O into convenient functions.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use chrono::Utc;

use super::directory::*;
use super::errors::{PersistenceError, Result};
use super::io::*;
use super::markdown::*;
use super::schema::*;

// ── Params Structs ──────────────────────────────────────────────────────────

/// Path parameters for locating a task within a plan.
#[derive(Debug, Clone)]
pub struct TaskPathParams {
    pub repo_root: PathBuf,
    pub branch: String,
    pub plan_id: String,
    pub plan_name: String,
    pub task_id: String,
    pub task_name: String,
}

/// Parameters for updating a task status.
#[derive(Debug, Clone)]
pub struct UpdateTaskStatusParams {
    pub path: TaskPathParams,
    pub new_status: TaskStatusValue,
    pub by: String,
}

/// Parameters for recovering a task status.
#[derive(Debug, Clone)]
pub struct RecoverTaskStatusParams {
    pub path: TaskPathParams,
    pub target_status: TaskStatusValue,
    pub by: String,
}

// ── Plan Operations ─────────────────────────────────────────────────────────

/// Create a new plan on disk.
///
/// This function:
/// 1. Ensures the plan directory exists (both specs and state).
/// 2. Writes `plan.md` (non-atomically) via markdown rendering.
/// 3. Creates `execution.json` atomically with an empty task list.
pub fn create_plan(repo_root: &Path, plan: &Plan) -> Result<()> {
    // Ensure directories exist
    ensure_plan_dir(repo_root, &plan.branch, &plan.id, &plan.name)?;

    // Write plan.md (non-atomic is fine for initial creation)
    let plan_path = plan_markdown_path(repo_root, &plan.branch, &plan.id, &plan.name);
    let markdown = render_plan_markdown(plan);
    write_file(&plan_path, &markdown)?;

    // Create execution.json atomically
    let exec_path = execution_state_path(repo_root, &plan.branch, &plan.id, &plan.name);
    let exec_state = ExecutionState {
        plan_id: plan.id.clone(),
        branch: plan.branch.clone(),
        tasks: Vec::new(),
        task_status_map: HashMap::new(),
    };
    atomic_write_json(&exec_path, &exec_state)
}

/// Read a plan from disk by parsing its markdown file.
pub fn read_plan(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> Result<Plan> {
    let plan_path = plan_markdown_path(repo_root, branch, plan_id, plan_name);
    parse_plan_markdown(&plan_path)
}

/// Update an existing plan by rewriting its markdown file.
pub fn update_plan(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    plan: &Plan,
) -> Result<()> {
    let plan_path = plan_markdown_path(repo_root, branch, plan_id, plan_name);
    let markdown = render_plan_markdown(plan);
    write_file(&plan_path, &markdown)
}

// ── Task Operations ─────────────────────────────────────────────────────────

/// Create a new task within an existing plan.
///
/// This function:
/// 1. Ensures the task directory exists.
/// 2. Writes `task.md` (non-atomically) via markdown rendering.
/// 3. Creates `status.json` atomically with the initial backlog status.
pub fn create_task(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task: &Task,
) -> Result<()> {
    // Ensure task directory exists
    ensure_task_dir(repo_root, branch, plan_id, plan_name, &task.id, &task.name)?;

    // Write task.md (non-atomic for initial creation)
    let task_path = task_markdown_path(repo_root, branch, plan_id, plan_name, &task.id, &task.name);
    let markdown = render_task_markdown(task);
    write_file(&task_path, &markdown)?;

    // Create status.json atomically with backlog status
    let status_path = task_status_path(repo_root, branch, plan_id, plan_name, &task.id, &task.name);
    let initial_status = TaskStatus {
        id: task.id.clone(),
        status: TaskStatusValue::Backlog,
        agent: None,
        transitions: Vec::new(),
        started_at: None,
        completed_at: None,
        attempts: 0,
        dependencies: task.dependencies.clone(),
        dependent_tasks: Vec::new(),
        heartbeat_at: None,
    };
    atomic_write_json(&status_path, &initial_status)
}

/// Read a task from disk by parsing its markdown file.
pub fn read_task(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    task_name: &str,
) -> Result<Task> {
    let task_path = task_markdown_path(repo_root, branch, plan_id, plan_name, task_id, task_name);
    parse_task_markdown(&task_path)
}

/// Read a task's current status from its `status.json` file.
pub fn read_task_status(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    task_name: &str,
) -> Result<TaskStatus> {
    let status_path = task_status_path(repo_root, branch, plan_id, plan_name, task_id, task_name);
    read_json(&status_path)
}

/// Update a task's status, recording the transition.
///
/// This function:
/// 1. Reads the current status.
/// 2. Records a transition from the old status to the new status.
/// 3. Sets `started_at` when transitioning to `Running`.
/// 4. Sets `completed_at` when transitioning to `Completed`.
/// 5. Increments `attempts` on EVERY transition to `Running`.
/// 6. Verifies file hasn't changed before writing (TOCTOU mitigation).
/// 7. Writes the updated status atomically.
pub fn update_task_status(params: &UpdateTaskStatusParams) -> Result<TaskStatus> {
    let status_path = task_status_path(
        &params.path.repo_root,
        &params.path.branch,
        &params.path.plan_id,
        &params.path.plan_name,
        &params.path.task_id,
        &params.path.task_name,
    );

    // Read current status
    let content = read_file(&status_path)?;
    let mut status: TaskStatus = serde_json::from_str(&content)
        .map_err(|e| PersistenceError::JsonParse(status_path.clone(), e))?;

    let old_status = status.status.clone();
    let now = Utc::now().to_rfc3339();

    // Record transition (from/to are strings now)
    status.transitions.push(StatusTransition {
        from: task_status_to_string(&old_status).to_string(),
        to: task_status_to_string(&params.new_status).to_string(),
        at: now.clone(),
        by: params.by.clone(),
    });

    // Set started_at and increment attempts on EVERY transition to Running
    if matches!(params.new_status, TaskStatusValue::Running) {
        status.attempts += 1;
        status.started_at = Some(now.clone());
    }

    // Set completed_at when transitioning to Completed
    if matches!(params.new_status, TaskStatusValue::Completed) {
        status.completed_at = Some(now.clone());
    }

    status.status = params.new_status.clone();

    // Re-check before writing: verify file hasn't changed
    let new_content = fs::read_to_string(&status_path)
        .map_err(|e| PersistenceError::Io(status_path.clone(), e))?;
    if new_content != content {
        return Err(PersistenceError::ConcurrencyConflict(status_path));
    }

    // Write atomically
    atomic_write_json(&status_path, &status)?;

    Ok(status)
}

/// Recover a task from stale heartbeat by transitioning it to the target status.
///
/// This function:
/// 1. Reads the current status.
/// 2. Records the transition using correct kebab-case strings.
/// 3. Clears `agent`, `started_at`, and `heartbeat_at`.
/// 4. Increments `attempts`.
/// 5. Sets the new status to `target_status`.
/// 6. Verifies file hasn't changed before writing (TOCTOU mitigation).
/// 7. Writes the updated status atomically.
pub fn recover_task_status(params: &RecoverTaskStatusParams) -> Result<TaskStatus> {
    let status_path = task_status_path(
        &params.path.repo_root,
        &params.path.branch,
        &params.path.plan_id,
        &params.path.plan_name,
        &params.path.task_id,
        &params.path.task_name,
    );

    // Read current status (single read — validation happens here)
    let content = read_file(&status_path)?;
    let mut status: TaskStatus = serde_json::from_str(&content)
        .map_err(|e| PersistenceError::JsonParse(status_path.clone(), e))?;

    let old_status = status.status.clone();
    let now = Utc::now().to_rfc3339();

    // Record transition with correct kebab-case strings
    status.transitions.push(StatusTransition {
        from: task_status_to_string(&old_status).to_string(),
        to: task_status_to_string(&params.target_status).to_string(),
        at: now.clone(),
        by: params.by.clone(),
    });

    // Clear recovery fields
    status.agent = None;
    status.started_at = None;
    status.heartbeat_at = None;

    // Increment attempts
    status.attempts += 1;

    // Set new status
    status.status = params.target_status.clone();

    // Re-check before writing: verify file hasn't changed
    let new_content = fs::read_to_string(&status_path)
        .map_err(|e| PersistenceError::Io(status_path.clone(), e))?;
    if new_content != content {
        return Err(PersistenceError::ConcurrencyConflict(status_path));
    }

    // Write atomically
    atomic_write_json(&status_path, &status)?;

    Ok(status)
}

// ── Execution State Operations ──────────────────────────────────────────────

/// Read the execution state for a plan.
pub fn read_execution_state(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
) -> Result<ExecutionState> {
    let exec_path = execution_state_path(repo_root, branch, plan_id, plan_name);
    read_json(&exec_path)
}

/// Update the execution state for a plan atomically.
pub fn update_execution_state(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    state: &ExecutionState,
) -> Result<()> {
    let exec_path = execution_state_path(repo_root, branch, plan_id, plan_name);
    atomic_write_json(&exec_path, state)
}

/// Add a task to the execution state's task list and status map.
///
/// Includes TOCTOU mitigation: verifies file hasn't changed before writing.
pub fn add_task_to_execution(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    status: TaskStatusValue,
) -> Result<()> {
    let exec_path = execution_state_path(repo_root, branch, plan_id, plan_name);

    // Read current state
    let content = read_file(&exec_path)?;
    let mut state: ExecutionState = serde_json::from_str(&content)
        .map_err(|e| PersistenceError::JsonParse(exec_path.clone(), e))?;

    state.tasks.push(task_id.to_string());
    state.task_status_map.insert(task_id.to_string(), status);

    // Re-check before writing: verify file hasn't changed
    let new_content =
        fs::read_to_string(&exec_path).map_err(|e| PersistenceError::Io(exec_path.clone(), e))?;
    if new_content != content {
        return Err(PersistenceError::ConcurrencyConflict(exec_path));
    }

    // Write atomically
    atomic_write_json(&exec_path, &state)
}
