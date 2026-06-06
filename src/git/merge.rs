//! Git merge operations and conflict handling.
//!
//! This module provides functions for merging branches, detecting and listing
//! merge conflicts, and managing merge state. All merge operations use `--no-ff`
//! to ensure merge commits are created for audit trail purposes.

use std::path::Path;

use super::branch::{branch_exists, checkout_branch, current_branch, task_branch_name};
use super::errors::{GitError, Result};
use super::subprocess::{git, GitCommand, GitOutput};

/// Merge a source branch into a target branch.
///
/// 1. Records the current branch for later restoration.
/// 2. Checks out the target branch.
/// 3. Runs `git merge --no-ff <source_branch>`.
/// 4. On success (exit 0), restores to the original branch and returns output.
/// 5. On failure (exit 1), checks for merge conflicts:
///    - If conflicts exist, returns `GitError::MergeConflict`.
///    - Otherwise, returns `GitError::SubprocessFailure`.
///
/// The `--no-ff` flag ensures merge commits are always created for audit trail.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `source_branch` - The branch to merge from.
/// * `target_branch` - The branch to merge into.
///
/// # Errors
///
/// Returns `GitError::MergeConflict` if the merge produces conflicts,
/// or `GitError::SubprocessFailure` for other merge failures.
pub async fn merge_branch(
    repo_root: &Path,
    source_branch: &str,
    target_branch: &str,
) -> Result<GitOutput> {
    // Step 1: Get current branch to restore later
    let original_branch = current_branch(repo_root).await?;

    // Step 2: Checkout target branch
    checkout_branch(repo_root, target_branch).await?;

    // Step 3: Run git merge --no-ff <source_branch>
    let merge_result = GitCommand::new(repo_root)
        .args(&["merge", "--no-ff", source_branch])
        .execute()
        .await;

    match merge_result {
        Ok(output) => {
            // Step 4: Merge succeeded — restore to original branch
            checkout_branch(repo_root, &original_branch).await?;
            Ok(output)
        }
        Err(GitError::SubprocessFailure {
            exit_code,
            stdout,
            stderr,
            command,
        }) if exit_code == 1 => {
            // Step 5: Exit code 1 — check for merge conflicts
            let conflict_output =
                git(repo_root, &["diff", "--name-only", "--diff-filter=U"]).await;

            let conflicted_files: Vec<String> = match conflict_output {
                Ok(out) => out
                    .stdout
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| line.trim().to_string())
                    .collect(),
                Err(_) => Vec::new(),
            };

            if !conflicted_files.is_empty() {
                // Step 5a: Conflicts detected
                Err(GitError::MergeConflict {
                    branch: source_branch.to_string(),
                    plan_branch: target_branch.to_string(),
                    conflicts: conflicted_files,
                })
            } else {
                // Step 5b: No conflicts — generic failure
                Err(GitError::SubprocessFailure {
                    command,
                    exit_code,
                    stdout,
                    stderr,
                })
            }
        }
        Err(e) => {
            // Other error — restore to original branch
            let _ = checkout_branch(repo_root, &original_branch).await;
            Err(e)
        }
    }
}

/// Abort an in-progress merge.
///
/// Runs `git merge --abort` to cancel an ongoing merge operation.
/// If no merge is in progress, this is a no-op.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
///
/// # Errors
///
/// Returns an error only if the abort operation itself fails for a
/// reason unrelated to "no merge in progress".
pub async fn abort_merge(repo_root: &Path) -> Result<()> {
    // If no merge is in progress, skip the abort (no-op)
    if !is_merging(repo_root).await? {
        return Ok(());
    }

    let _output = git(repo_root, &["merge", "--abort"]).await?;
    Ok(())
}

/// Check if the current branch has unresolved merge conflicts.
///
/// Runs `git diff --name-only --diff-filter=U` and returns `true` if
/// any conflicted files are found.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
///
/// # Returns
///
/// `true` if merge conflicts exist, `false` otherwise.
pub async fn has_merge_conflicts(repo_root: &Path) -> Result<bool> {
    let output = git(repo_root, &["diff", "--name-only", "--diff-filter=U"]).await?;
    Ok(!output.stdout.trim().is_empty())
}

/// List files with unresolved merge conflicts.
///
/// Runs `git diff --name-only --diff-filter=U` and parses the output
/// into a list of file paths.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
///
/// # Returns
///
/// A vector of file path strings that have merge conflicts.
pub async fn list_conflicted_files(repo_root: &Path) -> Result<Vec<String>> {
    let output = git(repo_root, &["diff", "--name-only", "--diff-filter=U"]).await?;
    let files: Vec<String> = output
        .stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.trim().to_string())
        .collect();
    Ok(files)
}

/// Check if a merge is currently in progress.
///
/// Checks for the existence of the `.git/MERGE_HEAD` file, which git
/// creates during an active merge operation.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
///
/// # Returns
///
/// `true` if a merge is in progress, `false` otherwise.
pub async fn is_merging(repo_root: &Path) -> Result<bool> {
    let merge_head = repo_root.join(".git").join("MERGE_HEAD");
    tokio::fs::try_exists(&merge_head).await.map_err(|e| GitError::Io {
        path: merge_head,
        source: e,
    })
}

/// Merge a task branch into the plan branch.
///
/// Validates that the task branch exists, checks out the plan branch,
/// and merges the task branch into it using `--no-ff`.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `task_id` - The task identifier (e.g., `TASK-001`).
/// * `plan_branch` - The plan branch to merge into.
/// * `merged_tasks` - List of already-merged tasks (caller handles dependency checking).
///
/// # Errors
///
/// Returns `GitError::MergeConflict` if the merge produces conflicts,
/// or `GitError::BranchNotFound` if the task branch doesn't exist.
pub async fn merge_task_branch(
    repo_root: &Path,
    task_id: &str,
    plan_branch: &str,
    merged_tasks: &[String],
) -> Result<()> {
    let _ = merged_tasks; // Accepted for context; caller handles dependency checking

    // Step 1: Validate task branch exists
    let branch_name = task_branch_name(task_id);
    if !branch_exists(repo_root, &branch_name).await? {
        return Err(GitError::BranchNotFound { branch: branch_name });
    }

    // Record original branch for restoration
    let original_branch = current_branch(repo_root).await?;

    // Step 3: Checkout plan branch
    checkout_branch(repo_root, plan_branch).await?;

    // Step 4: Run git merge --no-ff task/<task_id>
    let merge_result = GitCommand::new(repo_root)
        .args(&["merge", "--no-ff", &branch_name])
        .execute()
        .await;

    match merge_result {
        Ok(_) => {
            // Merge succeeded — restore to original branch
            let _ = checkout_branch(repo_root, &original_branch).await;
            Ok(())
        }
        Err(GitError::SubprocessFailure {
            exit_code,
            stdout,
            stderr,
            command,
        }) if exit_code == 1 => {
            // Check for conflicts
            let conflict_output =
                git(repo_root, &["diff", "--name-only", "--diff-filter=U"]).await;

            let conflicted_files: Vec<String> = match conflict_output {
                Ok(out) => out
                    .stdout
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| line.trim().to_string())
                    .collect(),
                Err(_) => Vec::new(),
            };

            if !conflicted_files.is_empty() {
                Err(GitError::MergeConflict {
                    branch: branch_name,
                    plan_branch: plan_branch.to_string(),
                    conflicts: conflicted_files,
                })
            } else {
                Err(GitError::SubprocessFailure {
                    command,
                    exit_code,
                    stdout,
                    stderr,
                })
            }
        }
        Err(e) => {
            let _ = checkout_branch(repo_root, &original_branch).await;
            Err(e)
        }
    }
}
