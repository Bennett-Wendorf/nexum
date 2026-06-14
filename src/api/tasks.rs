//! Task CRUD endpoints for the Nexum REST API.
//!
//! This module provides HTTP handlers for managing tasks within plans:
//!
//! | Method | Path                                                      | Handler                        |
//! |--------|-----------------------------------------------------------|--------------------------------|
//! | GET    | `/api/plans/:branch/:plan_id/tasks`                       | [`list_tasks`]                 |
//! | GET    | `/api/plans/:branch/:plan_id/tasks/:task_id`              | [`get_task`]                   |
//! | POST   | `/api/plans/:branch/:plan_id/tasks`                       | [`create_task`]                |
//! | PUT    | `/api/plans/:branch/:plan_id/tasks/:task_id`              | [`update_task`]                |
//! | DELETE | `/api/plans/:branch/:plan_id/tasks/:task_id`              | [`delete_task`]                |
//! | PATCH  | `/api/plans/:branch/:plan_id/tasks/:task_id/status`       | [`transition_task_status`]     |
//! | POST   | `/api/plans/:branch/:plan_id/tasks/:task_id/claim`        | [`claim_task`]                 |
//!
//! Tasks are stored as markdown files under
//! `.agent/specs/<branch>/<plan_slug>/tasks/<task_slug>/task.md`, with
//! associated runtime status under `status.json` in the same directory.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;

use crate::api::errors::ApiError;
use crate::api::middleware::validate_status_transition;
use crate::api::types::*;
use crate::persistence::*;

// ── Helper Functions ──────────────────────────────────────────────────

/// Convert a persistence-layer [`Task`] and [`TaskStatus`] into an API
/// [`TaskResponse`].
///
/// Merges the structural fields from [`Task`] with runtime status fields
/// from [`TaskStatus`], converting the [`TaskStatusValue`] enum to its
/// kebab-case string representation.
fn task_to_response(task: &Task, status: &TaskStatus) -> TaskResponse {
    TaskResponse {
        id: task.id.clone(),
        name: task.name.clone(),
        parent_plan: task.parent_plan.clone(),
        dependencies: task.dependencies.clone(),
        description: task.description.clone(),
        acceptance_criteria: task.acceptance_criteria.clone(),
        files_to_modify: task.files_to_modify.clone(),
        background: task.background.clone(),
        notes: task.notes.clone(),
        status: TaskStatusResponse {
            id: status.id.clone(),
            status: task_status_to_string(&status.status).to_string(),
            agent: status.agent.as_ref().map(|a| AgentLeaseResponse {
                role: a.role.clone(),
                pid: a.pid,
                leased_at: a.leased_at.clone(),
            }),
            transitions: status
                .transitions
                .iter()
                .map(|t| StatusTransitionResponse {
                    from: t.from.clone(),
                    to: t.to.clone(),
                    at: t.at.clone(),
                    by: t.by.clone(),
                })
                .collect(),
            started_at: status.started_at.clone(),
            completed_at: status.completed_at.clone(),
            attempts: status.attempts,
            dependencies: status.dependencies.clone(),
            dependent_tasks: status.dependent_tasks.clone(),
            heartbeat_at: status.heartbeat_at.clone(),
        },
    }
}

/// Locate a task directory within a plan and extract its ID and name.
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
        return Err(ApiError::NotFound("task not found".to_string()));
    }

    let entries = crate::persistence::list_dir(&tasks_dir)
        .map_err(|_| ApiError::NotFound("task not found".to_string()))?;

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

    Err(ApiError::NotFound("task not found".to_string()))
}

/// Parse a kebab-case status string into a [`TaskStatusValue`] enum variant.
///
/// Returns [`ApiError::Validation`] if the string does not match any
/// recognised task status.
fn parse_task_status_value(s: &str) -> std::result::Result<TaskStatusValue, ApiError> {
    match s {
        "backlog" => Ok(TaskStatusValue::Backlog),
        "queued" => Ok(TaskStatusValue::Queued),
        "running" => Ok(TaskStatusValue::Running),
        "reviewing" => Ok(TaskStatusValue::Reviewing),
        "waiting-manual-review" => Ok(TaskStatusValue::WaitingManualReview),
        "merge-queue" => Ok(TaskStatusValue::MergeQueue),
        "abandoned" => Ok(TaskStatusValue::Abandoned),
        "completed" => Ok(TaskStatusValue::Completed),
        _ => Err(ApiError::Validation(format!(
            "Invalid task status: {s}"
        ))),
    }
}

/// Generate the next available task ID by scanning existing tasks in a plan.
///
/// Iterates over task directory slugs, extracts the numeric suffix from
/// each task ID (e.g. `001` from `TASK-001`), and returns the next
/// sequential number formatted as `TASK-{N}` with zero-padding to three
/// digits.
fn generate_task_id(
    repo_root: &std::path::Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
) -> std::result::Result<String, ApiError> {
    let tasks_dir = plan_dir(repo_root, branch, plan_id, plan_name).join("tasks");

    let mut max_num: u32 = 0;

    if tasks_dir.exists() {
        let entries = crate::persistence::list_dir(&tasks_dir)
            .map_err(ApiError::from)?;

        for entry in entries {
            if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                if let Some(name) = entry.file_name().to_str() {
                    if let (Some(id), _) = parse_slug(name) {
                        if let Some(num_str) = id.strip_prefix("TASK-") {
                            if let Ok(num) = num_str.parse::<u32>() {
                                max_num = max_num.max(num);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(format!("TASK-{:03}", max_num + 1))
}

/// Shared helper: locate a plan directory and extract the plan name from
/// its slug. Returns the resolved path and plan name, or a 404 error.
fn resolve_plan_path(
    repo_root: &std::path::Path,
    branch: &str,
    plan_id: &str,
) -> std::result::Result<(std::path::PathBuf, String), ApiError> {
    let plan_dir_path = find_plan_by_id(repo_root, branch, plan_id)
        .map_err(|_| ApiError::NotFound("plan not found".to_string()))?;

    let slug = plan_dir_path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| ApiError::NotFound("plan not found".to_string()))?;

    let plan_name = match parse_slug(slug) {
        (Some(_), Some(name)) => name.to_string(),
        _ => return Err(ApiError::NotFound("plan not found".to_string())),
    };

    Ok((plan_dir_path, plan_name))
}

// ── Handlers ──────────────────────────────────────────────────────────

/// List all tasks for a plan, optionally filtered by status.
///
/// Resolves the plan directory using [`find_plan_by_id`], lists task
/// slugs using [`list_tasks`], and for each task parses the slug to
/// extract the ID and name, reads `task.md` and `status.json`, and
/// combines them into a [`TaskResponse`]. If `query.status` is provided,
/// only tasks whose current status matches the filter are included.
///
/// Returns a [`ListResponse`] containing the matching tasks and the total
/// count.
pub async fn list_tasks(
    State(state): State<AppState>,
    Path((branch, plan_id)): Path<(String, String)>,
    Query(query): Query<TaskListQuery>,
) -> std::result::Result<Json<ListResponse<TaskResponse>>, ApiError> {
    // Verify plan exists
    let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    // List task directory slugs
    let task_slugs = crate::persistence::list_tasks(&state.repo_root, &branch, &plan_id, &plan_name)
        .map_err(ApiError::from)?;

    let mut items = Vec::new();
    let mut total = 0usize;

    for slug in task_slugs {
        let (task_id, task_name) = match parse_slug(&slug) {
            (Some(id), Some(name)) => (id.to_string(), name.to_string()),
            _ => continue,
        };

        let task = read_task(&state.repo_root, &branch, &plan_id, &plan_name, &task_id, &task_name)
            .map_err(ApiError::from)?;

        let status = read_task_status(
            &state.repo_root,
            &branch,
            &plan_id,
            &plan_name,
            &task_id,
            &task_name,
        )
        .map_err(ApiError::from)?;

        // Count total before filtering
        total += 1;

        // Filter by status if specified
        if let Some(ref filter_status) = query.status {
            if task_status_to_string(&status.status) != filter_status.as_str() {
                continue;
            }
        }

        items.push(task_to_response(&task, &status));
    }
    Ok(Json(ListResponse {
        items,
        total,
    }))
}

/// Get a specific task by branch, plan ID, and task ID.
///
/// Resolves the plan directory, then locates the task within the plan's
/// `tasks/` directory. Reads `task.md` and `status.json`, combines them
/// into a [`TaskResponse`], and returns it.
///
/// Returns **404 Not Found** if the plan or task does not exist.
pub async fn get_task(
    State(state): State<AppState>,
    Path((branch, plan_id, task_id)): Path<(String, String, String)>,
) -> std::result::Result<Json<TaskResponse>, ApiError> {
    // Verify plan exists
    let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    // Resolve task path
    let (_task_dir, resolved_task_id, task_name) =
        resolve_task_path(&state.repo_root, &branch, &plan_id, &plan_name, &task_id)?;

    let task = read_task(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &resolved_task_id,
        &task_name,
    )
    .map_err(ApiError::from)?;

    let status = read_task_status(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &resolved_task_id,
        &task_name,
    )
    .map_err(ApiError::from)?;

    Ok(Json(task_to_response(&task, &status)))
}

/// Create a new task within an existing plan.
///
/// Validates that the plan exists and the task name is non-empty. Generates
/// a unique task ID by scanning existing tasks in the plan. Creates the
/// task directory, writes `task.md`, initializes `status.json` with backlog
/// status, updates the plan's task list (adds a [`TaskReference`]), and
/// updates the execution state (adds the task to `task_status_map`).
///
/// Returns **201 Created** on success with the newly created [`TaskResponse`].
pub async fn create_task(
    State(state): State<AppState>,
    Path((branch, plan_id)): Path<(String, String)>,
    Json(req): Json<CreateTaskRequest>,
) -> std::result::Result<impl IntoResponse, ApiError> {
    // Verify plan exists
    let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    // Validate request fields
    if req.name.trim().is_empty() {
        return Err(ApiError::Validation(
            "Task name cannot be empty".to_string(),
        ));
    }

    // Generate a unique task ID
    let task_id = generate_task_id(&state.repo_root, &branch, &plan_id, &plan_name)?;

    // Create the task struct
    let task = Task {
        id: task_id.clone(),
        name: req.name.clone(),
        parent_plan: plan_id.clone(),
        dependencies: req.dependencies.clone(),
        description: req.description.clone(),
        acceptance_criteria: req.acceptance_criteria.clone(),
        files_to_modify: req.files_to_modify.clone(),
        background: req.background.unwrap_or_default(),
        notes: req.notes.unwrap_or_default(),
    };

    // Persist the task (creates directories, writes task.md, initializes status.json)
    crate::persistence::create_task(&state.repo_root, &branch, &plan_id, &plan_name, &task)
        .map_err(ApiError::from)?;

    // Read initial status that was created by create_task
    let status = read_task_status(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &task_id,
        &task.name,
    )
    .map_err(ApiError::from)?;

    // Update plan's task list: add TaskReference
    let mut plan = read_plan(&state.repo_root, &branch, &plan_id, &plan_name)
        .map_err(ApiError::from)?;
    plan.tasks.push(TaskReference {
        id: task_id.clone(),
        name: task.name.clone(),
        completed: false,
    });
    crate::persistence::update_plan(&state.repo_root, &branch, &plan_id, &plan_name, &plan)
        .map_err(ApiError::from)?;

    // Update execution state: add task to task_status_map with backlog status
    crate::persistence::add_task_to_execution(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &task_id,
        TaskStatusValue::Backlog,
    )
    .map_err(ApiError::from)?;

    let response = task_to_response(&task, &status);
    Ok((StatusCode::CREATED, Json(response)))
}

/// Update an existing task's metadata.
///
/// Locates the task, reads the current markdown, applies partial updates
/// from the request (only fields that are `Some` are changed), and writes
/// the updated task back to disk via [`render_task_markdown`].
///
/// Returns the updated [`TaskResponse`]. Returns **404 Not Found** if the
/// plan or task does not exist.
pub async fn update_task(
    State(state): State<AppState>,
    Path((branch, plan_id, task_id)): Path<(String, String, String)>,
    Json(req): Json<UpdateTaskRequest>,
) -> std::result::Result<Json<TaskResponse>, ApiError> {
    // Verify plan exists
    let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    // Resolve task path
    let (_task_dir, resolved_task_id, task_name) =
        resolve_task_path(&state.repo_root, &branch, &plan_id, &plan_name, &task_id)?;

    let mut task = read_task(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &resolved_task_id,
        &task_name,
    )
    .map_err(ApiError::from)?;

    // Apply partial updates
    if let Some(name) = &req.name {
        if name.trim().is_empty() {
            return Err(ApiError::Validation(
                "Task name cannot be empty".to_string(),
            ));
        }
        task.name = name.clone();
    }
    if let Some(description) = &req.description {
        task.description = description.clone();
    }
    if let Some(acceptance_criteria) = &req.acceptance_criteria {
        task.acceptance_criteria = acceptance_criteria.clone();
    }
    if let Some(files_to_modify) = &req.files_to_modify {
        task.files_to_modify = files_to_modify.clone();
    }
    if let Some(background) = &req.background {
        task.background = background.clone();
    }
    if let Some(notes) = &req.notes {
        task.notes = notes.clone();
    }

    // Rewrite task.md with render_task_markdown
    let task_path = task_markdown_path(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &resolved_task_id,
        &task_name,
    );
    let markdown = render_task_markdown(&task);
    write_file(&task_path, &markdown).map_err(ApiError::from)?;

    // Read current status
    let status = read_task_status(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &resolved_task_id,
        &task_name,
    )
    .map_err(ApiError::from)?;

    Ok(Json(task_to_response(&task, &status)))
}

/// Delete a task and remove it from the plan's task list and execution state.
///
/// Locates the task directory, removes it, then updates the plan's task
/// list (removes the [`TaskReference`]) and the execution state (removes
/// the task from `task_status_map`).
///
/// Returns **204 No Content** on success. Returns **404 Not Found** if the
/// plan or task does not exist.
pub async fn delete_task(
    State(state): State<AppState>,
    Path((branch, plan_id, task_id)): Path<(String, String, String)>,
) -> std::result::Result<StatusCode, ApiError> {
    // Verify plan exists
    let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    // Resolve task path
    let (task_dir, resolved_task_id, _task_name) =
        resolve_task_path(&state.repo_root, &branch, &plan_id, &plan_name, &task_id)?;

    // Remove task directory
    crate::persistence::remove_dir_all(&task_dir)
        .map_err(ApiError::from)?;

    // Update plan's task list: remove reference
    let mut plan = read_plan(&state.repo_root, &branch, &plan_id, &plan_name)
        .map_err(ApiError::from)?;
    plan.tasks.retain(|t| t.id != resolved_task_id);
    crate::persistence::update_plan(&state.repo_root, &branch, &plan_id, &plan_name, &plan)
        .map_err(ApiError::from)?;

    // Update execution state: remove from task_status_map
    let mut exec_state = read_execution_state(&state.repo_root, &branch, &plan_id, &plan_name)
        .map_err(ApiError::from)?;
    exec_state.tasks.retain(|t| *t != resolved_task_id);
    exec_state.task_status_map.remove(&resolved_task_id);
    crate::persistence::update_execution_state(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &exec_state,
    )
    .map_err(ApiError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Transition a task to a new status.
///
/// Reads the current task status from `status.json`, validates the
/// requested transition using [`validate_status_transition`], parses the
/// target status string to a [`TaskStatusValue`] enum, calls
/// [`update_task_status`] to persist the change (with TOCTOU protection),
/// and updates the execution state's `task_status_map`.
///
/// Returns the updated [`TaskResponse`]. Returns **404 Not Found** if the
/// plan or task does not exist, or **422 Unprocessable Entity** if the
/// transition is not permitted from the current status.
pub async fn transition_task_status(
    State(state): State<AppState>,
    Path((branch, plan_id, task_id)): Path<(String, String, String)>,
    Json(req): Json<TransitionTaskStatusRequest>,
) -> std::result::Result<Json<TaskResponse>, ApiError> {
    // Verify plan exists
    let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    // Resolve task path
    let (_task_dir, resolved_task_id, task_name) =
        resolve_task_path(&state.repo_root, &branch, &plan_id, &plan_name, &task_id)?;

    // Read current status
    let current_status = read_task_status(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &resolved_task_id,
        &task_name,
    )
    .map_err(ApiError::from)?;

    // Validate the transition against the task transition map
    let current_status_str = task_status_to_string(&current_status.status);
    validate_status_transition(current_status_str, &req.status, "task")?;

    // Parse target status string to TaskStatusValue enum
    let new_status = parse_task_status_value(&req.status)?;

    // Call persistence::update_task_status with TOCTOU protection
    let params = UpdateTaskStatusParams {
        path: TaskPathParams {
            repo_root: state.repo_root.clone(),
            branch: branch.clone(),
            plan_id: plan_id.clone(),
            plan_name: plan_name.clone(),
            task_id: resolved_task_id.clone(),
            task_name: task_name.clone(),
        },
        new_status: new_status.clone(),
        by: req.by.clone().unwrap_or_else(|| "api".to_string()),
    };
    let updated_status = crate::persistence::update_task_status(&params)
        .map_err(ApiError::from)?;

    // Update execution state task_status_map
    let mut exec_state = read_execution_state(&state.repo_root, &branch, &plan_id, &plan_name)
        .map_err(ApiError::from)?;
    exec_state
        .task_status_map
        .insert(resolved_task_id.clone(), new_status);
    crate::persistence::update_execution_state(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &exec_state,
    )
    .map_err(ApiError::from)?;

    // Read updated task
    let task = read_task(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &resolved_task_id,
        &task_name,
    )
    .map_err(ApiError::from)?;

    Ok(Json(task_to_response(&task, &updated_status)))
}

/// Claim (lease) a task for execution by an agent.
///
/// Reads the current status from `status.json`, verifies the task is in
/// the "queued" state, creates an [`AgentLease`] with the provided role
/// and PID, and calls [`update_task_status`] to transition the task to
/// "running". If the status was already changed by another process (TOCTOU
/// conflict), returns **409 Conflict**.
///
/// Returns the updated [`TaskResponse`]. Returns **404 Not Found** if the
/// plan or task does not exist, **422 Unprocessable Entity** if the task
/// is not in "queued" status, or **409 Conflict** if a concurrency
/// conflict occurs.
pub async fn claim_task(
    State(state): State<AppState>,
    Path((branch, plan_id, task_id)): Path<(String, String, String)>,
    Json(req): Json<ClaimTaskRequest>,
) -> std::result::Result<Json<TaskResponse>, ApiError> {
    // Verify plan exists
    let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    // Resolve task path
    let (_task_dir, resolved_task_id, task_name) =
        resolve_task_path(&state.repo_root, &branch, &plan_id, &plan_name, &task_id)?;

    // Read current status
    let current_status = read_task_status(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &resolved_task_id,
        &task_name,
    )
    .map_err(ApiError::from)?;

    // Verify current status is "queued"
    if !matches!(current_status.status, TaskStatusValue::Queued) {
        return Err(ApiError::Validation(format!(
            "Task is in '{}' status, must be 'queued' to claim",
            task_status_to_string(&current_status.status)
        )));
    }

    // Create AgentLease
    let lease = AgentLease {
        role: req.agent_role.clone(),
        pid: req.agent_pid,
        leased_at: Utc::now().to_rfc3339(),
    };

    // Build updated status with the lease
    let params = UpdateTaskStatusParams {
        path: TaskPathParams {
            repo_root: state.repo_root.clone(),
            branch: branch.clone(),
            plan_id: plan_id.clone(),
            plan_name: plan_name.clone(),
            task_id: resolved_task_id.clone(),
            task_name: task_name.clone(),
        },
        new_status: TaskStatusValue::Running,
        by: format!("agent:{}:{}", req.agent_role, req.agent_pid),
    };

    // Attempt status transition with TOCTOU protection
    let updated_status = match crate::persistence::update_task_status(&params) {
        Ok(status) => status,
        Err(crate::persistence::PersistenceError::ConcurrencyConflict(_)) => {
            return Err(ApiError::Conflict(
                "task status was modified by another process".to_string(),
            ));
        }
        Err(e) => return Err(ApiError::from(e)),
    };

    // Inject the agent lease into the returned status
    let mut final_status = updated_status;
    final_status.agent = Some(lease);

    // Re-write status.json with the agent lease (with TOCTOU verification)
    let status_path = task_status_path(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &resolved_task_id,
        &task_name,
    );

    // Re-read status.json to verify it still has status=Running (TOCTOU check)
    let re_read_status: TaskStatus = read_json(&status_path)
        .map_err(ApiError::from)?;
    if !matches!(re_read_status.status, TaskStatusValue::Running) {
        return Err(ApiError::Conflict(
            "task status was modified by another process".to_string(),
        ));
    }

    atomic_write_json(&status_path, &final_status).map_err(ApiError::from)?;

    // Update execution state task_status_map
    let mut exec_state = read_execution_state(&state.repo_root, &branch, &plan_id, &plan_name)
        .map_err(ApiError::from)?;
    exec_state
        .task_status_map
        .insert(resolved_task_id.clone(), TaskStatusValue::Running);
    crate::persistence::update_execution_state(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &exec_state,
    )
    .map_err(ApiError::from)?;

    // Read updated task
    let task = read_task(
        &state.repo_root,
        &branch,
        &plan_id,
        &plan_name,
        &resolved_task_id,
        &task_name,
    )
    .map_err(ApiError::from)?;

    Ok(Json(task_to_response(&task, &final_status)))
}
