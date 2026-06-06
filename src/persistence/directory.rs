//! Directory helpers for the persistence layer.
//!
//! Provides path resolution functions that construct paths under the
//! `.agent/` directory hierarchy, as well as directory creation and
//! traversal utilities.

use std::path::{Path, PathBuf};

use super::errors::{PersistenceError, Result};
use super::io::create_dir_all;

// ── Path Resolution ─────────────────────────────────────────────────────────

/// Return the path to the `.agent/` directory inside the repo root.
pub fn agent_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".agent")
}

/// Return the path to the specs directory for a given branch.
///
/// Returns `<repo_root>/.agent/specs/<branch>/`.
pub fn specs_dir(repo_root: &Path, branch: &str) -> PathBuf {
    agent_dir(repo_root).join("specs").join(branch)
}

/// Return the path to the state directory for a given branch.
///
/// Returns `<repo_root>/.agent/state/<branch>/`.
pub fn state_dir(repo_root: &Path, branch: &str) -> PathBuf {
    agent_dir(repo_root).join("state").join(branch)
}

/// Return the path to a plan's specs directory.
///
/// Returns `<specs>/<branch>/<plan_id>-<plan_name>/`.
pub fn plan_dir(repo_root: &Path, branch: &str, plan_id: &str, plan_name: &str) -> PathBuf {
    let slug = plan_slug(plan_id, plan_name);
    specs_dir(repo_root, branch).join(&slug)
}

/// Return the path to a task's directory within a plan.
///
/// Returns `<plan_dir>/tasks/<task_id>-<task_name>/`.
pub fn task_dir(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    task_name: &str,
) -> PathBuf {
    let slug = task_slug(task_id, task_name);
    plan_dir(repo_root, branch, plan_id, plan_name).join("tasks").join(&slug)
}

/// Return the path to a plan's markdown file (`plan.md`).
pub fn plan_markdown_path(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
) -> PathBuf {
    plan_dir(repo_root, branch, plan_id, plan_name).join("plan.md")
}

/// Return the path to a task's markdown file (`task.md`).
pub fn task_markdown_path(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    task_name: &str,
) -> PathBuf {
    task_dir(repo_root, branch, plan_id, plan_name, task_id, task_name).join("task.md")
}

/// Return the path to a task's status JSON file (`status.json`).
pub fn task_status_path(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    task_name: &str,
) -> PathBuf {
    task_dir(repo_root, branch, plan_id, plan_name, task_id, task_name).join("status.json")
}

/// Return the path to a plan's execution state file (`execution.json`).
pub fn execution_state_path(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
) -> PathBuf {
    let slug = plan_slug(plan_id, plan_name);
    state_dir(repo_root, branch).join(&slug).join("execution.json")
}

/// Return the path to a task's log directory.
///
/// Returns `<state>/<branch>/<plan_id>-<plan_name>/logs/<task_id>-<task_name>/`.
pub fn task_log_dir(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    task_name: &str,
) -> PathBuf {
    let slug = plan_slug(plan_id, plan_name);
    let task_slug = task_slug(task_id, task_name);
    state_dir(repo_root, branch)
        .join(&slug)
        .join("logs")
        .join(&task_slug)
}

// ── Directory Creation ──────────────────────────────────────────────────────

/// Ensure the `.agent/` directory exists under the repo root.
pub fn ensure_agent_dir(repo_root: &Path) -> Result<()> {
    create_dir_all(&agent_dir(repo_root))
}

/// Ensure the plan directory exists (both specs and state).
pub fn ensure_plan_dir(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
) -> Result<()> {
    // Create specs plan dir
    create_dir_all(&plan_dir(repo_root, branch, plan_id, plan_name))?;
    // Create state plan dir (parent of execution.json)
    let slug = plan_slug(plan_id, plan_name);
    create_dir_all(&state_dir(repo_root, branch).join(&slug))
}

/// Ensure the task directory exists within a plan.
pub fn ensure_task_dir(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    task_name: &str,
) -> Result<()> {
    create_dir_all(&task_dir(repo_root, branch, plan_id, plan_name, task_id, task_name))
}

// ── Directory Traversal ─────────────────────────────────────────────────────

/// Shared helper: list subdirectory names under the given path.
fn list_subdirs(path: &Path) -> Result<Vec<String>> {
    if !path.exists() {
        return Err(PersistenceError::DirectoryNotFound(path.to_path_buf()));
    }
    let entries = super::io::list_dir(path)?;
    let mut names = entries
        .into_iter().filter_map(|e| {
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

/// List all branch names under `.agent/specs/`.
///
/// Returns the names of subdirectories (one per branch).
pub fn list_branches(repo_root: &Path) -> Result<Vec<String>> {
    let specs_root = agent_dir(repo_root).join("specs");
    if !specs_root.exists() {
        return Ok(Vec::new());
    }
    list_subdirs(&specs_root)
}

/// List all plan directory slugs under a branch's specs directory.
pub fn list_plans(repo_root: &Path, branch: &str) -> Result<Vec<String>> {
    let branch_specs = specs_dir(repo_root, branch);
    if !branch_specs.exists() {
        return Ok(Vec::new());
    }
    list_subdirs(&branch_specs)
}

/// List all task directory slugs under a plan's tasks directory.
pub fn list_tasks(
    repo_root: &Path,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
) -> Result<Vec<String>> {
    let tasks_dir = plan_dir(repo_root, branch, plan_id, plan_name).join("tasks");
    if !tasks_dir.exists() {
        return Ok(Vec::new());
    }
    list_subdirs(&tasks_dir)
}

/// Find the plan directory for a given plan ID within a branch.
///
/// Searches all subdirectories of the branch's specs directory and returns
/// the path of the first one whose name starts with `<plan_id>-`.
pub fn find_plan_by_id(repo_root: &Path, branch: &str, plan_id: &str) -> Result<PathBuf> {
    let branch_specs = specs_dir(repo_root, branch);
    if !branch_specs.exists() {
        return Err(PersistenceError::DirectoryNotFound(branch_specs));
    }
    let entries = super::io::list_dir(&branch_specs)?;
    for entry in entries {
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&format!("{}-", plan_id)) {
                    return Ok(entry.path());
                }
            }
        }
    }
    Err(PersistenceError::SchemaValidation(format!(
        "Plan '{}' not found in branch '{}'",
        plan_id, branch
    )))
}

// ── Slug Generation ─────────────────────────────────────────────────────────

/// Convert text into a URL/path-safe slug.
///
/// Lowercases the text, replaces spaces with hyphens, and removes all
/// characters except alphanumeric, hyphens, and underscores.
pub fn slugify(text: &str) -> String {
    let mut result = String::new();
    let mut prev_was_hyphen = false;
    for c in text.to_lowercase().chars() {
        if c.is_alphanumeric() || c == '_' {
            result.push(c);
            prev_was_hyphen = false;
        } else if c.is_whitespace() || c == '-' {
            if !prev_was_hyphen {
                result.push('-');
                prev_was_hyphen = true;
            }
        }
        // else: skip the character
    }
    result.trim_end_matches('-').to_string()
}

/// Build a plan slug from plan ID and name.
fn plan_slug(plan_id: &str, plan_name: &str) -> String {
    format!("{}-{}", plan_id, slugify(plan_name))
}

/// Build a task slug from task ID and name.
fn task_slug(task_id: &str, task_name: &str) -> String {
    format!("{}-{}", task_id, slugify(task_name))
}

/// Parse plan ID and name from a slug like "PLAN-001-my-plan".
///
/// Returns `(plan_id, plan_name)` where `plan_id` includes the trailing hyphen
/// (e.g., `"PLAN-001"`). Returns `(None, None)` if the slug format is invalid.
pub fn parse_plan_slug(slug: &str) -> (Option<&str>, Option<&str>) {
    if let Some(first_hyphen) = slug.find('-') {
        let after_first = &slug[first_hyphen + 1..];
        if let Some(second_hyphen) = after_first.find('-') {
            let plan_id = &slug[..first_hyphen + 1 + second_hyphen];
            let plan_name = &slug[first_hyphen + 1 + second_hyphen + 1..];
            (Some(plan_id), Some(plan_name))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    }
}

/// Parse task ID and name from a slug like "TASK-001-my-task".
///
/// Returns `(task_id, task_name)` where `task_id` includes the trailing hyphen
/// (e.g., `"TASK-001"`). Returns `(None, None)` if the slug format is invalid.
pub fn parse_task_slug(slug: &str) -> (Option<&str>, Option<&str>) {
    if let Some(first_hyphen) = slug.find('-') {
        let after_first = &slug[first_hyphen + 1..];
        if let Some(second_hyphen) = after_first.find('-') {
            let task_id = &slug[..first_hyphen + 1 + second_hyphen];
            let task_name = &slug[first_hyphen + 1 + second_hyphen + 1..];
            (Some(task_id), Some(task_name))
        } else {
            (None, None)
        }
    } else {
        (None, None)
    }
}
