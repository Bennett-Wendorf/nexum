//! Plan CRUD endpoints for the Nexum REST API.
//!
//! This module provides HTTP handlers for managing plans:
//!
//! | Method | Path                                      | Handler                    |
//! |--------|-------------------------------------------|----------------------------|
//! | GET    | `/api/plans`                              | [`list_plans`]             |
//! | GET    | `/api/plans/:branch/:plan_id`             | [`get_plan`]               |
//! | POST   | `/api/plans`                              | [`create_plan`]            |
//! | PUT    | `/api/plans/:branch/:plan_id`             | [`update_plan`]            |
//! | DELETE | `/api/plans/:branch/:plan_id`             | [`delete_plan`]            |
//! | PATCH  | `/api/plans/:branch/:plan_id/status`      | [`transition_plan_status`] |
//!
//! Plans are stored as markdown files under
//! `.agent/specs/<branch>/<plan_id>-<plan_name>/plan.md`, with associated
//! execution state under `.agent/state/<branch>/<plan_id>-<plan_name>/`.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::Utc;

use crate::api::errors::ApiError;
use crate::api::middleware::{validate_branch_name, validate_status_transition};
use crate::api::types::*;
use crate::persistence::*;

// ── Helper Functions ──────────────────────────────────────────────────

/// Convert a [`PlanStatus`] enum variant to its kebab-case display string.
///
/// This mirrors the `#[serde(rename_all = "kebab-case")]` attribute on
/// [`PlanStatus`] so that API responses use the same casing as the
/// status transition maps in [`crate::api::middleware`].
fn plan_status_display(status: &PlanStatus) -> &'static str {
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

/// Convert a persistence-layer [`Plan`] to an API [`PlanResponse`].
///
/// Maps each field from the internal struct to the DTO, converting the
/// [`PlanStatus`] enum to its kebab-case string representation and
/// mapping [`TaskReference`] entries to [`TaskReferenceResponse`] objects.
fn plan_to_response(plan: &Plan) -> PlanResponse {
    PlanResponse {
        id: plan.id.clone(),
        name: plan.name.clone(),
        status: plan_status_display(&plan.status).to_string(),
        created: plan.created.clone(),
        branch: plan.branch.clone(),
        goal: plan.goal.clone(),
        scope: plan.scope.clone(),
        background: plan.background.clone(),
        tasks: plan
            .tasks
            .iter()
            .map(|t| TaskReferenceResponse {
                id: t.id.clone(),
                name: t.name.clone(),
                completed: t.completed,
            })
            .collect(),
    }
}

/// Parse a kebab-case status string into a [`PlanStatus`] enum variant.
///
/// Returns [`ApiError::Validation`] if the string does not match any
/// recognised plan status.
fn parse_plan_status_from_string(status: &str) -> std::result::Result<PlanStatus, ApiError> {
    match status {
        "draft" => Ok(PlanStatus::Draft),
        "queued" => Ok(PlanStatus::Queued),
        "planning" => Ok(PlanStatus::Planning),
        "reviewing" => Ok(PlanStatus::Reviewing),
        "approved" => Ok(PlanStatus::Approved),
        "complete" => Ok(PlanStatus::Complete),
        "rejected" => Ok(PlanStatus::Rejected),
        _ => Err(ApiError::Validation(format!(
            "Invalid plan status: {status}"
        ))),
    }
}

/// Generate the next available plan ID by scanning existing plans.
///
/// Iterates over all branches and plan directory slugs, extracts the
/// numeric suffix from each plan ID (e.g. `001` from `PLAN-001`), and
/// returns the next sequential number formatted as `PLAN-{N}` with
/// zero-padding to three digits.
fn generate_plan_id(
    repo_root: &std::path::Path,
) -> std::result::Result<String, ApiError> {
    let branches = crate::persistence::list_branches(repo_root)
        .map_err(ApiError::from)?;
    let mut max_num: u32 = 0;

    for branch in &branches {
        // If listing plans for a branch fails, skip it gracefully.
        let plans = crate::persistence::list_plans(repo_root, branch).unwrap_or_default();
        for slug in plans {
            if let (Some(id), _) = parse_slug(&slug) {
                if let Some(num_str) = id.strip_prefix("PLAN-") {
                    if let Ok(num) = num_str.parse::<u32>() {
                        max_num = max_num.max(num);
                    }
                }
            }
        }
    }

    Ok(format!("PLAN-{:03}", max_num + 1))
}

/// Shared helper: locate a plan directory and extract the plan name from
/// its slug. Returns the resolved path and plan name, or a 404 error.
fn resolve_plan_path(
    repo_root: &std::path::Path,
    branch: &str,
    plan_id: &str,
) -> std::result::Result<(std::path::PathBuf, String), ApiError> {
    let plan_dir = find_plan_by_id(repo_root, branch, plan_id)
        .map_err(|_| ApiError::NotFound("plan not found".to_string()))?;

    let slug = plan_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| ApiError::NotFound("plan not found".to_string()))?;

    let plan_name = match parse_slug(slug) {
        (Some(_), Some(name)) => name.to_string(),
        _ => return Err(ApiError::NotFound("plan not found".to_string())),
    };

    Ok((plan_dir, plan_name))
}

// ── Handlers ──────────────────────────────────────────────────────────

/// List all plans, optionally filtered by branch and/or status.
///
/// Iterates over every branch in `.agent/specs/` (or a single branch if
/// `query.branch` is provided), reads each plan's markdown file via
/// [`read_plan`], and converts it to a [`PlanResponse`]. If `query.status`
/// is provided, only plans whose current status matches the filter are
/// included.
///
/// Returns a [`ListResponse`] containing the matching plans and the total
/// count.
pub async fn list_plans(
    State(state): State<AppState>,
    Query(query): Query<PlanListQuery>,
) -> std::result::Result<Json<ListResponse<PlanResponse>>, ApiError> {
    let branches = crate::persistence::list_branches(&state.repo_root)
        .map_err(ApiError::from)?;

    let mut items = Vec::new();
    let mut total = 0usize;

    for branch in &branches {
        // Filter by branch if specified
        if let Some(ref filter_branch) = query.branch {
            if branch != filter_branch {
                continue;
            }
        }

        let plan_slugs = crate::persistence::list_plans(&state.repo_root, branch)
            .map_err(ApiError::from)?;

        for slug in plan_slugs {
            let (plan_id, plan_name) = match parse_slug(&slug) {
                (Some(id), Some(name)) => (id.to_string(), name.to_string()),
                _ => continue, // Skip directories that don't match the slug format
            };

            let plan = read_plan(&state.repo_root, branch, &plan_id, &plan_name)
                .map_err(ApiError::from)?;

            // Count total before filtering
            total += 1;

            // Filter by status if specified
            if let Some(ref filter_status) = query.status {
                if plan_status_display(&plan.status) != filter_status.as_str() {
                    continue;
                }
            }

            items.push(plan_to_response(&plan));
        }
    }
    Ok(Json(ListResponse {
        items,
        total,
    }))
}

/// Get a specific plan by branch and plan ID.
///
/// Locates the plan directory using [`find_plan_by_id`], extracts the plan
/// name from the directory slug via [`parse_slug`], reads the plan markdown
/// with [`read_plan`], and returns it as a [`PlanResponse`].
///
/// Returns **404 Not Found** if the plan does not exist in the specified
/// branch.
pub async fn get_plan(
    State(state): State<AppState>,
    Path((branch, plan_id)): Path<(String, String)>,
) -> std::result::Result<Json<PlanResponse>, ApiError> {
    let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    let plan = read_plan(&state.repo_root, &branch, &plan_id, &plan_name)
        .map_err(ApiError::from)?;

    Ok(Json(plan_to_response(&plan)))
}

/// Create a new plan.
///
/// Validates that the plan name is non-empty and the branch name is valid
/// (via [`validate_branch_name`]). Generates a unique plan ID by scanning
/// existing plans and picking the next sequential number. Creates the plan
/// directory structure (both specs and state), writes the plan markdown,
/// and initializes the execution state.
///
/// Returns **201 Created** on success with the newly created [`PlanResponse`].
pub async fn create_plan(
    State(state): State<AppState>,
    Json(req): Json<CreatePlanRequest>,
) -> std::result::Result<impl IntoResponse, ApiError> {
    // Validate request fields
    if req.name.trim().is_empty() {
        return Err(ApiError::Validation(
            "Plan name cannot be empty".to_string(),
        ));
    }

    validate_branch_name(&req.branch)?;

    // Generate a unique plan ID
    let plan_id = generate_plan_id(&state.repo_root)?;

    // Create the plan struct
    let plan = Plan {
        id: plan_id.clone(),
        name: req.name.clone(),
        status: PlanStatus::Draft,
        created: Utc::now().to_rfc3339(),
        branch: req.branch.clone(),
        goal: req.goal.clone(),
        scope: req.scope.unwrap_or_default(),
        background: req.background.unwrap_or_default(),
        tasks: Vec::new(),
    };

    // Persist the plan (creates directories, writes plan.md, initializes execution.json)
    crate::persistence::create_plan(&state.repo_root, &plan)
        .map_err(ApiError::from)?;

    let response = plan_to_response(&plan);
    Ok((StatusCode::CREATED, Json(response)))
}

/// Update an existing plan.
///
/// Locates the plan by branch and ID, reads the current plan markdown,
/// applies partial updates from the request fields (only fields that are
/// `Some` are changed), and writes the updated plan back to disk via
/// [`crate::persistence::update_plan`].
///
/// Returns the updated [`PlanResponse`]. Returns **404 Not Found** if the
/// plan does not exist.
pub async fn update_plan(
    State(state): State<AppState>,
    Path((branch, plan_id)): Path<(String, String)>,
    Json(req): Json<UpdatePlanRequest>,
) -> std::result::Result<Json<PlanResponse>, ApiError> {
    let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    let mut plan = read_plan(&state.repo_root, &branch, &plan_id, &plan_name)
        .map_err(ApiError::from)?;

    // Apply partial updates
    if let Some(name) = &req.name {
        if name.trim().is_empty() {
            return Err(ApiError::Validation(
                "Plan name cannot be empty".to_string(),
            ));
        }
        plan.name = name.clone();
    }
    if let Some(goal) = &req.goal {
        plan.goal = goal.clone();
    }
    if let Some(scope) = &req.scope {
        plan.scope = scope.clone();
    }
    if let Some(background) = &req.background {
        plan.background = background.clone();
    }

    // Write the updated plan back to disk
    crate::persistence::update_plan(&state.repo_root, &branch, &plan_id, &plan_name, &plan)
        .map_err(ApiError::from)?;

    Ok(Json(plan_to_response(&plan)))
}

/// Delete a plan and all associated data.
///
/// Locates the plan directory, then removes both:
///
/// - The **specs** directory at `.agent/specs/<branch>/<slug>/` (containing
///   `plan.md` and any task subdirectories).
/// - The **state** directory at `.agent/state/<branch>/<slug>/` (containing
///   `execution.json` and any log subdirectories).
///
/// Returns **204 No Content** on success. Returns **404 Not Found** if the
/// plan does not exist.
pub async fn delete_plan(
    State(state): State<AppState>,
    Path((branch, plan_id)): Path<(String, String)>,
) -> std::result::Result<StatusCode, ApiError> {
    let (plan_dir, plan_name) =
        resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    // Remove the specs directory (plan.md + tasks)
    crate::persistence::remove_dir_all(&plan_dir)
        .map_err(ApiError::from)?;

    // Remove the state directory (execution.json + logs)
    let plan_name_slug = slugify(&plan_name);
    let state_slug = format!("{}-{}", plan_id, plan_name_slug);
    let state_path = state_dir(&state.repo_root, &branch).join(&state_slug);
    crate::persistence::remove_dir_all(&state_path)
        .map_err(ApiError::from)?;

    Ok(StatusCode::NO_CONTENT)
}

/// Transition a plan to a new status.
///
/// Reads the current plan, validates the requested status transition against
/// the allowed transition map using [`validate_status_transition`], updates
/// the plan's status field, and writes the updated plan back to disk.
///
/// Returns the updated [`PlanResponse`]. Returns **404 Not Found** if the
/// plan does not exist, or **422 Unprocessable Entity** if the transition
/// is not permitted from the current status.
pub async fn transition_plan_status(
    State(state): State<AppState>,
    Path((branch, plan_id)): Path<(String, String)>,
    Json(req): Json<TransitionPlanStatusRequest>,
) -> std::result::Result<Json<PlanResponse>, ApiError> {
    let (_plan_dir, plan_name) = resolve_plan_path(&state.repo_root, &branch, &plan_id)?;

    let mut plan = read_plan(&state.repo_root, &branch, &plan_id, &plan_name)
        .map_err(ApiError::from)?;

    // Get current status display string for transition validation
    let current_status = plan_status_display(&plan.status);

    // Validate the transition against the plan transition map
    validate_status_transition(current_status, &req.status, "plan")?;

    // Parse and apply the new status
    let new_status = parse_plan_status_from_string(&req.status)?;
    plan.status = new_status;

    // Write the updated plan back to disk
    crate::persistence::update_plan(&state.repo_root, &branch, &plan_id, &plan_name, &plan)
        .map_err(ApiError::from)?;

    Ok(Json(plan_to_response(&plan)))
}
