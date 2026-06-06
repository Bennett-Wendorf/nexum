//! Git branch operations.
//!
//! This module provides functions for creating, deleting, checking out,
//! listing, and querying branches. Task branches follow the `task/<task_id>`
//! naming convention.

use std::path::Path;

use super::errors::Result;
use super::subprocess::git;

/// Create a new branch from an existing branch.
///
/// Runs `git branch <branch_name> <from_branch>` to create the new branch
/// pointing at the same commit as `from_branch`.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `branch_name` - The name of the new branch to create.
/// * `from_branch` - The existing branch to base the new branch on.
///
/// # Errors
///
/// Returns an error if the git command fails (e.g., `from_branch` doesn't
/// exist, or `branch_name` already exists).
pub async fn create_branch(repo_root: &Path, branch_name: &str, from_branch: &str) -> Result<()> {
    let _output = git(repo_root, &["branch", branch_name, from_branch]).await?;
    Ok(())
}

/// Create a task branch from an existing branch.
///
/// Constructs a branch name in the form `task/<task_id>` and creates it
/// from the given base branch.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `task_id` - The task identifier (e.g., `TASK-001`).
/// * `from_branch` - The existing branch to base the task branch on.
///
/// # Returns
///
/// The constructed branch name (e.g., `task/TASK-001`).
///
/// # Errors
///
/// Returns an error if the git command fails.
pub async fn create_task_branch(repo_root: &Path, task_id: &str, from_branch: &str) -> Result<String> {
    let branch_name = task_branch_name(task_id);
    create_branch(repo_root, &branch_name, from_branch).await?;
    Ok(branch_name)
}

/// Delete a branch (force delete).
///
/// Runs `git branch -D <branch_name>` to force-delete the branch regardless
/// of its merge status. If the branch does not exist, this is a no-op.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `branch_name` - The name of the branch to delete.
pub async fn delete_branch(repo_root: &Path, branch_name: &str) -> Result<()> {
    let output = git(repo_root, &["branch", "-D", branch_name]).await;
    match output {
        Ok(_) => Ok(()),
        Err(_) => {
            // If the branch doesn't exist, treat it as a no-op.
            // `git branch -D` returns exit code 1 for non-existent branches,
            // so a failure here is acceptable when the branch is absent.
            Ok(())
        }
    }
}

/// Checkout a branch.
///
/// Runs `git checkout <branch_name>` to switch to the specified branch.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `branch_name` - The name of the branch to checkout.
///
/// # Errors
///
/// Returns an error if the branch doesn't exist or checkout fails.
pub async fn checkout_branch(repo_root: &Path, branch_name: &str) -> Result<()> {
    let _output = git(repo_root, &["checkout", branch_name]).await?;
    Ok(())
}

/// List all local branches.
///
/// Runs `git branch` and parses the output to return a list of branch names.
/// The current branch is indicated with a `*` prefix in git's output; this
/// prefix is stripped in the returned list.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
///
/// # Returns
///
/// A vector of branch name strings.
pub async fn list_local_branches(repo_root: &Path) -> Result<Vec<String>> {
    let output = git(repo_root, &["branch"]).await?;
    let branches: Vec<String> = output
        .stdout
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            // Lines are either "* branch_name" or "  branch_name"
            Some(trimmed.strip_prefix('*').unwrap_or(trimmed).trim().to_string())
        })
        .collect();
    Ok(branches)
}

/// Check if a branch exists in the repository.
///
/// Runs `git branch --list <branch_name>` and checks whether any output
/// is returned.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `branch_name` - The name of the branch to check.
///
/// # Returns
///
/// `true` if the branch exists, `false` otherwise.
pub async fn branch_exists(repo_root: &Path, branch_name: &str) -> Result<bool> {
    let output = git(repo_root, &["branch", "--list", branch_name]).await?;
    Ok(!output.stdout.trim().is_empty())
}

/// Get the name of the currently checked-out branch.
///
/// Runs `git rev-parse --abbrev-ref HEAD` to determine the current branch.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
///
/// # Returns
///
/// The current branch name (e.g., `main`, `task/TASK-001`).
pub async fn current_branch(repo_root: &Path) -> Result<String> {
    let output = git(repo_root, &["rev-parse", "--abbrev-ref", "HEAD"]).await?;
    Ok(output.stdout.trim().to_string())
}

/// Generate a task branch name from a task ID.
///
/// Task branches always use the `task/` namespace prefix.
///
/// # Arguments
///
/// * `task_id` - The task identifier (e.g., `TASK-001`).
///
/// # Returns
///
/// The branch name string (e.g., `task/TASK-001`).
pub fn task_branch_name(task_id: &str) -> String {
    format!("task/{}", task_id)
}

/// Check if a branch name matches the task branch pattern.
///
/// A task branch name starts with the `task/` prefix.
///
/// # Arguments
///
/// * `branch_name` - The branch name to check.
///
/// # Returns
///
/// `true` if the branch name starts with `task/`, `false` otherwise.
pub fn is_task_branch(branch_name: &str) -> bool {
    branch_name.starts_with("task/")
}
