//! Error handling middleware, request validation, and concurrency utilities for the Nexum REST API.
//!
//! This module provides:
//!
//! - **Request ID middleware** — assigns a unique identifier to each HTTP
//!   request and echoes it in the `X-Request-ID` response header.
//! - **Status transition maps** — compile-time lookup tables defining the
//!   allowed status transitions for plans and tasks (per
//!   [`design/work-statuses.md`]).
//! - **Validation helpers** — functions for validating status values,
//!   status transitions, branch names, and slugs.
//! - **Concurrency helpers** — per-plan locking functions for serializing
//!   read-modify-write operations on plan data files.

use axum::http::{HeaderName, HeaderValue, Request};
use axum::middleware::Next;
use axum::response::IntoResponse;

use crate::api::errors::ApiError;
use crate::api::types::AppState;

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

// ── Request ID Middleware ──────────────────────────────────────────────

/// Monotonically increasing counter used to disambiguate concurrent
/// requests that share the same millisecond timestamp.
static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Response header name carrying the unique request identifier.
pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Axum middleware handler that generates a unique request ID for each
/// incoming request and appends it to the outgoing response headers.
///
/// The ID format is `req-{timestamp_ms}-{counter}`, which guarantees
/// uniqueness across time and within the same millisecond via the
/// atomic counter.
pub async fn request_id_middleware(
    req: Request<axum::body::Body>,
    next: Next,
) -> impl IntoResponse {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let counter = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    let request_id = format!("req-{}-{}", timestamp, counter);

    let mut response = next.run(req).await;
    response.headers_mut().insert(
        HeaderName::from_static(REQUEST_ID_HEADER),
        HeaderValue::from_str(&request_id)
            .unwrap_or_else(|_| HeaderValue::from_static("req-error")),
    );
    response
}

/// Returns an Axum layer wrapping [`request_id_middleware`].
///
/// Apply this layer to a router to automatically tag every request
/// with a unique ID:
///
/// ```no_run
/// use axum::Router;
/// use nexum::api::middleware::request_id_layer;
///
/// let app = Router::new().layer(request_id_layer());
/// ```
pub fn request_id_layer() -> impl tower::Layer<axum::routing::MethodRouter> + Clone {
    axum::middleware::from_fn::<_, axum::body::Body>(request_id_middleware)
}

// ── Status Transition Maps ─────────────────────────────────────────────

/// Allowed status transitions for plans.
///
/// Each tuple is `(current_status, &[allowed_next_statuses])`.
/// Covers: `draft → queued → planning → reviewing → approved|queued → complete|rejected`.
const PLAN_TRANSITIONS: &[(&str, &[&str])] = &[
    ("draft", &["queued"]),
    ("queued", &["planning"]),
    ("planning", &["reviewing"]),
    ("reviewing", &["approved", "queued"]),
    ("approved", &["complete", "rejected"]),
];

/// Terminal plan statuses — no further transitions are permitted.
const PLAN_TERMINAL_STATUSES: &[&str] = &["complete", "rejected"];

/// Allowed status transitions for tasks.
///
/// Each tuple is `(current_status, &[allowed_next_statuses])`.
/// Covers the full lifecycle:
/// `backlog → queued → running → reviewing → waiting-manual-review|merge-queue → completed`.
///
/// Tasks may be `abandoned` from any non-terminal state, and both
/// `abandoned` and `completed` are terminal states.
const TASK_TRANSITIONS: &[(&str, &[&str])] = &[
    ("backlog", &["queued", "abandoned"]),
    ("queued", &["running"]),
    ("running", &["reviewing", "abandoned"]),
    (
        "reviewing",
        &["waiting-manual-review", "merge-queue", "abandoned"],
    ),
    ("waiting-manual-review", &["merge-queue", "abandoned"]),
    ("merge-queue", &["completed"]),
];

/// Terminal task statuses — no further transitions are permitted.
const TASK_TERMINAL_STATUSES: &[&str] = &["abandoned", "completed"];

// ── Validation Functions ───────────────────────────────────────────────

/// Validates that a plan status value is one of the recognised states.
///
/// # Errors
///
/// Returns [`ApiError::Validation`] if `status` is not a valid plan state.
pub fn validate_plan_status(status: &str) -> Result<(), ApiError> {
    const VALID: &[&str] = &[
        "draft",
        "queued",
        "planning",
        "reviewing",
        "approved",
        "complete",
        "rejected",
    ];
    if !VALID.contains(&status) {
        return Err(ApiError::Validation(format!(
            "Invalid plan status: {status}"
        )));
    }
    Ok(())
}

/// Validates that a task status value is one of the recognised states.
///
/// # Errors
///
/// Returns [`ApiError::Validation`] if `status` is not a valid task state.
pub fn validate_task_status(status: &str) -> Result<(), ApiError> {
    const VALID: &[&str] = &[
        "backlog",
        "queued",
        "running",
        "reviewing",
        "waiting-manual-review",
        "merge-queue",
        "abandoned",
        "completed",
    ];
    if !VALID.contains(&status) {
        return Err(ApiError::Validation(format!(
            "Invalid task status: {status}"
        )));
    }
    Ok(())
}

/// Validates that a status transition is allowed for the given entity type.
///
/// The `entity_type` parameter should be `"plan"` or `"task"`. For plans,
/// both pre-planning and post-planning transition maps are consulted.
/// For tasks, only the task transition map is used.
///
/// Terminal statuses (`complete`/`rejected` for plans, `abandoned`/`completed`
/// for tasks) reject all transitions.
///
/// # Errors
///
/// Returns [`ApiError::Validation`] if:
/// - `entity_type` is not `"plan"` or `"task"`.
/// - `current` is a terminal status.
/// - The transition from `current` to `requested` is not in the allowed set.
pub fn validate_status_transition(
    current: &str,
    requested: &str,
    entity_type: &str,
) -> Result<(), ApiError> {
    match entity_type {
        "plan" => {
            // Check terminal statuses first
            if PLAN_TERMINAL_STATUSES.contains(&current) {
                return Err(ApiError::Validation(format!(
                    "Cannot transition from terminal plan status '{current}' to '{requested}'"
                )));
            }

            // Consult plan transition map
            let allowed = PLAN_TRANSITIONS
                .iter()
                .find(|&&(from, _)| from == current)
                .map(|&(_, to)| to);

            match allowed {
                Some(targets) if targets.contains(&requested) => Ok(()),
                Some(_) => Err(ApiError::Validation(format!(
                    "Invalid plan status transition: '{current}' -> '{requested}'"
                ))),
                None => Err(ApiError::Validation(format!(
                    "Unknown plan status '{current}' or no transitions defined"
                ))),
            }
        }
        "task" => {
            // Check terminal statuses first
            if TASK_TERMINAL_STATUSES.contains(&current) {
                return Err(ApiError::Validation(format!(
                    "Cannot transition from terminal task status '{current}' to '{requested}'"
                )));
            }

            let allowed = TASK_TRANSITIONS
                .iter()
                .find(|&&(from, _)| from == current)
                .map(|&(_, to)| to);

            match allowed {
                Some(targets) if targets.contains(&requested) => Ok(()),
                Some(_) => Err(ApiError::Validation(format!(
                    "Invalid task status transition: '{current}' -> '{requested}'"
                ))),
                None => Err(ApiError::Validation(format!(
                    "Unknown task status '{current}' or no transitions defined"
                ))),
            }
        }
        _ => Err(ApiError::Validation(format!(
            "Unknown entity type '{entity_type}' for status transition validation (expected 'plan' or 'task')"
        ))),
    }
}

/// Validates a git branch name.
///
/// The branch name must:
/// - Be non-empty.
/// - Not contain `..` (path traversal).
/// - Only contain alphanumeric characters, hyphens, underscores, or forward slashes.
/// - Not start or end with a forward slash.
///
/// # Errors
///
/// Returns [`ApiError::Validation`] if the branch name is invalid.
pub fn validate_branch_name(name: &str) -> Result<(), ApiError> {
    if name.is_empty() {
        return Err(ApiError::Validation(
            "Branch name cannot be empty".to_string(),
        ));
    }

    if name.contains("..") {
        return Err(ApiError::Validation(
            "Branch name must not contain '..' (path traversal)".to_string(),
        ));
    }

    if name.starts_with('/') || name.ends_with('/') {
        return Err(ApiError::Validation(
            "Branch name cannot start or end with '/'".to_string(),
        ));
    }

    for c in name.chars() {
        if !c.is_alphanumeric() && c != '-' && c != '_' && c != '/' {
            return Err(ApiError::Validation(format!(
                "Invalid character in branch name: '{c}'"
            )));
        }
    }

    Ok(())
}

/// Validates a slug string (kebab-case identifier).
///
/// The slug must be non-empty and contain only alphanumeric characters
/// and hyphens.
///
/// # Errors
///
/// Returns [`ApiError::Validation`] if the slug is invalid.
pub fn validate_slug(name: &str) -> Result<(), ApiError> {
    if name.is_empty() {
        return Err(ApiError::Validation("Slug cannot be empty".to_string()));
    }

    for c in name.chars() {
        if !c.is_alphanumeric() && c != '-' {
            return Err(ApiError::Validation(format!(
                "Invalid character in slug: '{c}'"
            )));
        }
    }

    Ok(())
}

/// Locate a plan directory and extract the plan name from its slug.
///
/// Searches for a plan directory whose slug starts with `<plan_id>-`
/// under `.agent/specs/<branch>/`. Returns the resolved path and plan
/// name, or a 404 error if the plan does not exist.
///
/// # Errors
///
/// Returns [`ApiError::NotFound`] if the plan cannot be located or
/// its directory slug cannot be parsed.
pub fn resolve_plan_path(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
) -> Result<(PathBuf, String), ApiError> {
    let plan_dir = crate::persistence::find_plan_by_id(repo_root, branch, plan_id)
        .map_err(|e| ApiError::NotFound(format!("plan not found: {e}")))?;

    let slug = plan_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| ApiError::NotFound("plan not found".to_string()))?;

    let plan_name = match crate::persistence::parse_slug(slug) {
        (Some(_), Some(name)) => name.to_string(),
        _ => return Err(ApiError::NotFound("plan not found".to_string())),
    };

    Ok((plan_dir, plan_name))
}

/// Locate a task directory within a plan and extract its ID and name.
///
/// Searches all subdirectories of the plan's `tasks/` directory for a
/// slug that starts with `<task_id>-`. Returns the resolved path, task
/// ID, and task name, or a 404 error if the task does not exist.
///
/// # Errors
///
/// Returns [`ApiError::NotFound`] if the task cannot be located or
/// its directory slug cannot be parsed.
pub fn resolve_task_path(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
) -> Result<(PathBuf, String, String), ApiError> {
    let tasks_dir =
        crate::persistence::plan_dir(repo_root, branch, plan_id, plan_name).join("tasks");

    if !tasks_dir.exists() {
        return Err(ApiError::NotFound("task not found".to_string()));
    }

    let entries = crate::persistence::list_dir(&tasks_dir)
        .map_err(|e| ApiError::NotFound(format!("task not found: {e}")))?;

    let prefix = format!("{}-", task_id);

    for entry in entries {
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&prefix) {
                    let (parsed_id, parsed_name) = match crate::persistence::parse_slug(name) {
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

// ── Per-Plan Async Locking ─────────────────────────────────────────────

/// Acquire the per-plan lock for the given plan_id.
///
/// Looks up the per-plan mutex in the AppState lock map. If no entry
/// exists, creates one. Returns the `Arc<tokio::sync::Mutex<()>>` for
/// the plan; the caller should lock it (e.g. `lock_plan(state, id).await.lock().await`)
/// to serialize all read-modify-write operations on this plan's data files.
///
/// This prevents TOCTOU races where concurrent handlers read the same
/// plan state, make independent modifications, and write back — causing
/// lost updates or duplicate entries.
pub async fn lock_plan(state: &AppState, plan_id: &str) -> Arc<tokio::sync::Mutex<()>> {
    let mut locks = state.plan_locks.write().await;
    locks
        .entry(plan_id.to_string())
        .or_insert_with(|| Arc::new(tokio::sync::Mutex::new(())))
        .clone()
}

/// Remove the per-plan lock entry for the given plan_id.
///
/// Call this after deleting a plan to prevent memory leaks from
/// accumulated mutex entries for deleted plans.
pub async fn remove_plan_lock(state: &AppState, plan_id: &str) {
    let mut locks = state.plan_locks.write().await;
    locks.remove(plan_id);
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Plan status validation ─────────────────────────────────────────

    #[test]
    fn test_validate_plan_status_valid() {
        for status in &[
            "draft",
            "queued",
            "planning",
            "reviewing",
            "approved",
            "complete",
            "rejected",
        ] {
            assert!(
                validate_plan_status(status).is_ok(),
                "status '{}' should be valid",
                status
            );
        }
    }

    #[test]
    fn test_validate_plan_status_invalid() {
        assert!(validate_plan_status("invalid").is_err());
        assert!(validate_plan_status("").is_err());
    }

    // ── Task status validation ─────────────────────────────────────────

    #[test]
    fn test_validate_task_status_valid() {
        for status in &[
            "backlog",
            "queued",
            "running",
            "reviewing",
            "waiting-manual-review",
            "merge-queue",
            "abandoned",
            "completed",
        ] {
            assert!(
                validate_task_status(status).is_ok(),
                "status '{}' should be valid",
                status
            );
        }
    }

    #[test]
    fn test_validate_task_status_invalid() {
        assert!(validate_task_status("invalid").is_err());
    }

    // ── Status transition validation ───────────────────────────────────

    #[test]
    fn test_plan_transition_valid() {
        assert!(validate_status_transition("draft", "queued", "plan").is_ok());
        assert!(validate_status_transition("queued", "planning", "plan").is_ok());
        assert!(validate_status_transition("planning", "reviewing", "plan").is_ok());
        assert!(validate_status_transition("reviewing", "approved", "plan").is_ok());
        assert!(validate_status_transition("reviewing", "queued", "plan").is_ok());
        assert!(validate_status_transition("approved", "complete", "plan").is_ok());
        assert!(validate_status_transition("approved", "rejected", "plan").is_ok());
    }

    #[test]
    fn test_plan_transition_invalid() {
        assert!(validate_status_transition("draft", "planning", "plan").is_err());
        assert!(validate_status_transition("complete", "approved", "plan").is_err());
        assert!(validate_status_transition("rejected", "draft", "plan").is_err());
    }

    #[test]
    fn test_task_transition_valid() {
        assert!(validate_status_transition("backlog", "queued", "task").is_ok());
        assert!(validate_status_transition("backlog", "abandoned", "task").is_ok());
        assert!(validate_status_transition("queued", "running", "task").is_ok());
        assert!(validate_status_transition("running", "reviewing", "task").is_ok());
        assert!(validate_status_transition("reviewing", "merge-queue", "task").is_ok());
        assert!(validate_status_transition("waiting-manual-review", "merge-queue", "task").is_ok());
        assert!(validate_status_transition("merge-queue", "completed", "task").is_ok());
    }

    #[test]
    fn test_task_transition_invalid() {
        assert!(validate_status_transition("backlog", "running", "task").is_err());
        assert!(validate_status_transition("completed", "backlog", "task").is_err());
        assert!(validate_status_transition("abandoned", "queued", "task").is_err());
    }

    #[test]
    fn test_unknown_entity_type() {
        assert!(validate_status_transition("draft", "queued", "unknown").is_err());
    }

    // ── Branch name validation ─────────────────────────────────────────

    #[test]
    fn test_validate_branch_name_valid() {
        assert!(validate_branch_name("feature/my-branch").is_ok());
        assert!(validate_branch_name("main").is_ok());
        assert!(validate_branch_name("feature_test/1").is_ok());
    }

    #[test]
    fn test_validate_branch_name_invalid() {
        assert!(validate_branch_name("").is_err());
        assert!(validate_branch_name("fea../ture").is_err());
        assert!(validate_branch_name("/start-with-slash").is_err());
        assert!(validate_branch_name("end-with-slash/").is_err());
        assert!(validate_branch_name("has space").is_err());
    }

    // ── Slug validation ────────────────────────────────────────────────

    #[test]
    fn test_validate_slug_valid() {
        assert!(validate_slug("my-plan").is_ok());
        assert!(validate_slug("plan-123").is_ok());
        assert!(validate_slug("a").is_ok());
    }

    #[test]
    fn test_validate_slug_invalid() {
        assert!(validate_slug("").is_err());
        assert!(validate_slug("has space").is_err());
        assert!(validate_slug("has/slash").is_err());
    }
}
