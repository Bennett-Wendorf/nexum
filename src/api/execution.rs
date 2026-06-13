//! Execution state and running task endpoints for the Nexum REST API.
//!
//! This module provides HTTP handlers for inspecting execution state and
//! discovering currently running tasks:
//!
//! | Method | Path                                                 | Handler                    |
//! |--------|------------------------------------------------------|----------------------------|
//! | GET    | `/api/plans/:branch/:plan_id/execution`              | [`get_execution_state`]    |
//! | GET    | `/api/running`                                       | [`list_running_tasks`]     |
//! | GET    | `/api/health`                                        | [`health_check`]           |
//!
//! Execution state is stored in `execution.json` files under
//! `.agent/state/<branch>/<plan_slug>/`, while task-level runtime data
//! (agent leases, heartbeats, transitions) lives in `status.json` files
//! under `.agent/specs/<branch>/<plan_slug>/tasks/<task_slug>/`.

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};

use crate::api::errors::ApiError;
use crate::api::types::*;
use crate::persistence::*;

// ── Helper Functions ──────────────────────────────────────────────────

/// Convert a persistence-layer [`ExecutionState`] into an API
/// [`ExecutionStateResponse`].
///
/// Maps the plan ID, branch, and task list directly. The
/// `task_status_map` values (which are [`TaskStatusValue`] enums) are
/// converted to their kebab-case string representations via
/// [`task_status_to_string`].
fn execution_state_to_response(state: &ExecutionState) -> ExecutionStateResponse {
    ExecutionStateResponse {
        plan_id: state.plan_id.clone(),
        branch: state.branch.clone(),
        tasks: state.tasks.clone(),
        task_status_map: state
            .task_status_map
            .iter()
            .map(|(k, v)| (k.clone(), task_status_to_string(v).to_string()))
            .collect(),
    }
}

/// Shared helper: locate a plan directory and extract the plan name from
/// its slug. Returns the resolved path and plan name, or a 404 error.
fn resolve_plan_path(
    repo_root: &std::path::Path,
    branch: &str,
    plan_id: &str,
) -> std::result::Result<(std::path::PathBuf, String), ApiError> {
    let plan_dir = find_plan_by_id(repo_root, branch, plan_id)
        .map_err(|_| ApiError::NotFound("plan not found"))?;

    let slug = plan_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| ApiError::NotFound("plan not found"))?;

    let plan_name = match parse_slug(slug) {
        (Some(_), Some(name)) => name.to_string(),
        _ => return Err(ApiError::NotFound("plan not found")),
    };

    Ok((plan_dir, plan_name))
}

/// List all plan directory slugs under a branch's state directory.
///
/// Returns the names of subdirectories (one per plan) under
/// `.agent/state/<branch>/`. Returns an empty vector if the directory
/// does not exist.
fn list_state_plans(
    repo_root: &std::path::Path,
    branch: &str,
) -> std::result::Result<Vec<String>, ApiError> {
    let state_branch_dir = state_dir(repo_root, branch);
    if !state_branch_dir.exists() {
        return Ok(Vec::new());
    }

    let entries = crate::persistence::list_dir(&state_branch_dir)
        .map_err(ApiError::from)?;

    let mut names = entries
        .into_iter()
        .filter_map(|e| {
            if e.file_type().ok()?.is_dir() {
                Some(e.file_name().into_string().ok()?)
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    names.sort();
    Ok(names)
}

/// Locate the task directory within a plan's specs directory and extract
/// the task ID and name from its slug.
///
/// Searches all subdirectories of the plan's `tasks/` directory for a
/// slug that starts with `<task_id>-`. Returns the resolved path, task
/// ID, and task name, or a 404 error if the task does not exist.
fn resolve_task_path(
    repo_root: &std::path::Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
) -> std::result::Result<(std::path::PathBuf, String, String), ApiError> {
    let tasks_dir = plan_dir(repo_root, branch, plan_id, plan_name).join("tasks");

    if !tasks_dir.exists() {
        return Err(ApiError::NotFound("task not found"));
    }

    let entries = crate::persistence::list_dir(&tasks_dir)
        .map_err(|_| ApiError::NotFound("task not found"))?;

    for entry in entries {
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&format!("{}-", task_id)) {
                    let (parsed_id, parsed_name) = match parse_slug(name) {
                        (Some(id), Some(n)) => (id.to_string(), n.to_string()),
                        _ => continue,
                    };
                    return Ok((entry.path(), parsed_id, parsed_name));
                }
            }
        }
    }

    Err(ApiError::NotFound("task not found"))
}

// ── Handlers ──────────────────────────────────────────────────────────

/// Get the execution state for a specific plan.
///
/// Locates the plan directory using [`find_plan_by_id`], extracts the plan
/// name from the directory slug via [`parse_slug`], reads the `execution.json`
/// file using [`read_execution_state`], and converts it to an
/// [`ExecutionStateResponse`].
///
/// Returns **404 Not Found** if the plan does not exist or if there is no
/// execution state file for the plan.
pub async fn get_execution_state(
    State(state): State<AppState>,
    Path((branch, plan_id)): Path<(String, String)>,
) -> std::result::Result<Json<ExecutionStateResponse>, ApiError> {
    // Resolve plan directory and extract plan name
    let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    // Read execution state
    let exec_state = read_execution_state(&state.repo_root, &branch, &plan_id, &plan_name)
        .map_err(ApiError::from)?;

    // Convert to API response
    let response = execution_state_to_response(&exec_state);
    Ok(Json(response))
}

/// List all tasks currently in the "running" state across all branches and plans.
///
/// Scans every branch under `.agent/specs/` (via [`list_branches`]), then for
/// each branch iterates over plan directories in `.agent/state/<branch>/`. For
/// each plan, reads `execution.json` and inspects the `task_status_map`. Any
/// task whose status is [`TaskStatusValue::Running`] is included in the result.
///
/// For each running task, the handler:
/// 1. Finds the task directory under `.agent/specs/<branch>/<plan_slug>/tasks/`
/// 2. Reads `status.json` to retrieve agent lease, start time, and heartbeat
/// 3. Builds a [`RunningTaskResponse`] with that data
///
/// Returns an empty vector if no running tasks are found.
pub async fn list_running_tasks(
    State(state): State<AppState>,
) -> std::result::Result<Json<Vec<RunningTaskResponse>>, ApiError> {
    let mut running_tasks = Vec::new();

    // List all branches under .agent/specs/
    let branches = crate::persistence::list_branches(&state.repo_root)
        .map_err(ApiError::from)?;

    for branch in &branches {
        // List plan slugs under .agent/state/<branch>/
        let plan_slugs = list_state_plans(&state.repo_root, branch)?;

        for slug in &plan_slugs {
            // Parse slug to get plan_id and plan_name
            let (plan_id, plan_name) = match parse_slug(slug) {
                (Some(id), Some(name)) => (id.to_string(), name.to_string()),
                _ => continue, // Skip directories that don't match the slug format
            };

            // Read execution state
            let exec_state = match read_execution_state(
                &state.repo_root,
                &branch,
                &plan_id,
                &plan_name,
            ) {
                Ok(state) => state,
                Err(_) => continue, // Skip plans without execution state
            };

            // Check each task in the status map for Running status
            for (task_id, task_status) in &exec_state.task_status_map {
                if !matches!(task_status, TaskStatusValue::Running) {
                    continue;
                }

                // Find the task directory and read status.json
                let (_task_dir, resolved_task_id, task_name) = match resolve_task_path(
                    &state.repo_root,
                    &branch,
                    &plan_id,
                    &plan_name,
                    task_id,
                ) {
                    Ok(result) => result,
                    Err(_) => continue, // Skip tasks that can't be resolved
                };

                // Read task status to get agent lease info
                let task_status_data = read_task_status(
                    &state.repo_root,
                    &branch,
                    &plan_id,
                    &plan_name,
                    &resolved_task_id,
                    &task_name,
                )
                .map_err(ApiError::from)?;

                // Build the RunningTaskResponse
                running_tasks.push(RunningTaskResponse {
                    task_id: resolved_task_id,
                    task_name,
                    plan_id: plan_id.clone(),
                    branch: branch.clone(),
                    agent: task_status_data.agent.as_ref().map(|a| AgentLeaseResponse {
                        role: a.role.clone(),
                        pid: a.pid,
                        leased_at: a.leased_at.clone(),
                    }),
                    started_at: task_status_data.started_at.clone(),
                    heartbeat_at: task_status_data.heartbeat_at.clone(),
                });
            }
        }
    }

    Ok(Json(running_tasks))
}

/// Health check endpoint.
///
/// Returns a simple JSON response with `{"status": "ok"}` and HTTP 200
/// status code. Used by load balancers and monitoring systems to verify
/// that the server is alive and accepting requests.
pub async fn health_check() -> impl IntoResponse {
    Json(serde_json::json!({"status": "ok"}))
}
