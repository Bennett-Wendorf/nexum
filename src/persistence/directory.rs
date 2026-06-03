//! Directory helpers for the persistence layer.
//!
//! Provides path resolution functions that construct paths under the
//! `.agent/` directory hierarchy, as well as directory creation and
//! traversal utilities.

use std::fs;
use std::path::PathBuf;

use super::errors::{PersistenceError, Result};
use super::io::create_dir_all;

// ── Path Resolution ─────────────────────────────────────────────────────────

/// Return the path to the `.agent/` directory inside the repo root.
pub fn agent_dir(repo_root: &str) -> PathBuf {
    PathBuf::from(repo_root).join(".agent")
}

/// Return the path to the specs directory for a given branch.
///
/// Returns `<repo_root>/.agent/specs/<branch>/`.
pub fn specs_dir(repo_root: &str, branch: &str) -> PathBuf {
    agent_dir(repo_root).join("specs").join(branch)
}

/// Return the path to the state directory for a given branch.
///
/// Returns `<repo_root>/.agent/state/<branch>/`.
pub fn state_dir(repo_root: &str, branch: &str) -> PathBuf {
    agent_dir(repo_root).join("state").join(branch)
}

/// Return the path to a plan's specs directory.
///
/// Returns `<specs>/<branch>/<plan_id>-<plan_name>/`.
pub fn plan_dir(repo_root: &str, branch: &str, plan_id: &str, plan_name: &str) -> PathBuf {
    let slug = format!("{}-{}", plan_id, slugify(plan_name));
    specs_dir(repo_root, branch).join(&slug)
}

/// Return the path to a task's directory within a plan.
///
/// Returns `<plan_dir>/tasks/<task_id>-<task_name>/`.
pub fn task_dir(
    repo_root: &str,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    task_name: &str,
) -> PathBuf {
    let slug = format!("{}-{}", task_id, slugify(task_name));
    plan_dir(repo_root, branch, plan_id, plan_name).join("tasks").join(&slug)
}

/// Return the path to a plan's markdown file (`plan.md`).
pub fn plan_markdown_path(
    repo_root: &str,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
) -> PathBuf {
    plan_dir(repo_root, branch, plan_id, plan_name).join("plan.md")
}

/// Return the path to a task's markdown file (`task.md`).
pub fn task_markdown_path(
    repo_root: &str,
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
    repo_root: &str,
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
    repo_root: &str,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
) -> PathBuf {
    let slug = format!("{}-{}", plan_id, slugify(plan_name));
    state_dir(repo_root, branch).join(&slug).join("execution.json")
}

/// Return the path to a task's log directory.
///
/// Returns `<state>/<branch>/<plan_id>-<plan_name>/logs/<task_id>-<task_name>/`.
pub fn task_log_dir(
    repo_root: &str,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    task_name: &str,
) -> PathBuf {
    let slug = format!("{}-{}", plan_id, slugify(plan_name));
    let task_slug = format!("{}-{}", task_id, slugify(task_name));
    state_dir(repo_root, branch)
        .join(&slug)
        .join("logs")
        .join(&task_slug)
}

// ── Directory Creation ──────────────────────────────────────────────────────

/// Ensure the `.agent/` directory exists under the repo root.
pub fn ensure_agent_dir(repo_root: &str) -> Result<()> {
    create_dir_all(&agent_dir(repo_root))
}

/// Ensure the plan directory exists (both specs and state).
pub fn ensure_plan_dir(
    repo_root: &str,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
) -> Result<()> {
    // Create specs plan dir
    create_dir_all(&plan_dir(repo_root, branch, plan_id, plan_name))?;
    // Create state plan dir (parent of execution.json)
    let slug = format!("{}-{}", plan_id, slugify(plan_name));
    create_dir_all(&state_dir(repo_root, branch).join(&slug))
}

/// Ensure the task directory exists within a plan.
pub fn ensure_task_dir(
    repo_root: &str,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
    task_id: &str,
    task_name: &str,
) -> Result<()> {
    create_dir_all(&task_dir(repo_root, branch, plan_id, plan_name, task_id, task_name))
}

// ── Directory Traversal ─────────────────────────────────────────────────────

/// List all branch names under `.agent/specs/`.
///
/// Returns the names of subdirectories (one per branch).
pub fn list_branches(repo_root: &str) -> Result<Vec<String>> {
    let specs_root = agent_dir(repo_root).join("specs");
    if !specs_root.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(&specs_root)
        .map_err(|e| PersistenceError::Io(specs_root.clone(), e))?;
    let mut branches = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| PersistenceError::Io(specs_root.clone(), e))?;
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                branches.push(name.to_string());
            }
        }
    }
    branches.sort();
    Ok(branches)
}

/// List all plan directory slugs under a branch's specs directory.
pub fn list_plans(repo_root: &str, branch: &str) -> Result<Vec<String>> {
    let branch_specs = specs_dir(repo_root, branch);
    if !branch_specs.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(&branch_specs)
        .map_err(|e| PersistenceError::Io(branch_specs.clone(), e))?;
    let mut plans = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| PersistenceError::Io(branch_specs.clone(), e))?;
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                plans.push(name.to_string());
            }
        }
    }
    plans.sort();
    Ok(plans)
}

/// List all task directory slugs under a plan's tasks directory.
pub fn list_tasks(
    repo_root: &str,
    branch: &str,
    plan_id: &str,
    plan_name: &str,
) -> Result<Vec<String>> {
    let tasks_dir = plan_dir(repo_root, branch, plan_id, plan_name).join("tasks");
    if !tasks_dir.exists() {
        return Ok(Vec::new());
    }
    let entries = fs::read_dir(&tasks_dir)
        .map_err(|e| PersistenceError::Io(tasks_dir.clone(), e))?;
    let mut tasks = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|e| PersistenceError::Io(tasks_dir.clone(), e))?;
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                tasks.push(name.to_string());
            }
        }
    }
    tasks.sort();
    Ok(tasks)
}

/// Find the plan directory for a given plan ID within a branch.
///
/// Searches all subdirectories of the branch's specs directory and returns
/// the path of the first one whose name starts with `<plan_id>-`.
pub fn find_plan_by_id(repo_root: &str, branch: &str, plan_id: &str) -> Result<PathBuf> {
    let branch_specs = specs_dir(repo_root, branch);
    if !branch_specs.exists() {
        return Err(PersistenceError::DirectoryNotFound(branch_specs));
    }
    let entries = fs::read_dir(&branch_specs)
        .map_err(|e| PersistenceError::Io(branch_specs.clone(), e))?;
    for entry in entries {
        let entry = entry.map_err(|e| PersistenceError::Io(branch_specs.clone(), e))?;
        if entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with(&format!("{}-", plan_id)) {
                    return Ok(entry.path());
                }
            }
        }
    }
    Err(PersistenceError::FileNotFound(
        branch_specs.join(format!("{}-*", plan_id)),
    ))
}

// ── Slug Generation ─────────────────────────────────────────────────────────

/// Convert text into a URL/path-safe slug.
///
/// Lowercases the text, replaces spaces with hyphens, and removes all
/// characters except alphanumeric, hyphens, and underscores.
pub fn slugify(text: &str) -> String {
    text.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else if c.is_whitespace() {
                '-'
            } else {
                '\0' // will be filtered out
            }
        })
        .filter(|&c| c != '\0')
        .collect()
}
