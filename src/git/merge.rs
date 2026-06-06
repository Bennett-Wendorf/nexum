//! Git merge operations and conflict handling.
//!
//! This module provides functions for merging branches, detecting and listing
//! merge conflicts, and managing merge state. All merge operations use `--no-ff`
//! to ensure merge commits are created for audit trail purposes.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};

use super::branch::{branch_exists, checkout_branch, current_branch, delete_branch, task_branch_name};
use super::errors::{GitError, Result};
use super::subprocess::{git, GitCommand, GitOutput};
use super::worktree::{remove_worktree, remove_worktree_force};

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

/// Merge plan for coordinating dependency-ordered task merges.
///
/// Contains the repository root, plan branch name, lists of pending and
/// merged tasks, and a dependency graph mapping task IDs to their
/// dependency task IDs.
#[derive(Debug)]
pub struct MergePlan {
    /// The root directory of the git repository.
    pub repo_root: PathBuf,
    /// The plan branch into which tasks will be merged.
    pub plan_branch: String,
    /// Task IDs waiting to be merged.
    pub pending_tasks: Vec<String>,
    /// Task IDs already merged into the plan branch.
    pub merged_tasks: Vec<String>,
    /// Dependency graph: task_id -> list of dependency task IDs.
    pub dependencies: HashMap<String, Vec<String>>,
}

/// Compute dependency-ordered merge sequence using Kahn's algorithm.
///
/// Performs a topological sort of pending tasks respecting the dependency
/// graph. Returns an ordered list of task IDs to merge.
///
/// # Errors
///
/// Returns `GitError::SubprocessFailure` if a circular dependency is detected.
pub fn determine_merge_order(plan: &MergePlan) -> Result<Vec<String>> {
    let pending: HashSet<&str> = plan.pending_tasks.iter().map(|s| s.as_str()).collect();
    
    // Build in-degree map for pending tasks only
    let mut in_degree: HashMap<String, usize> = HashMap::new();
    let mut adj: HashMap<String, Vec<String>> = HashMap::new();
    
    for task in &plan.pending_tasks {
        in_degree.entry(task.clone()).or_insert(0);
        adj.entry(task.clone()).or_insert_with(Vec::new);
    }
    
    for (task, deps) in &plan.dependencies {
        if !pending.contains(task.as_str()) {
            continue;
        }
        for dep in deps {
            if pending.contains(dep.as_str()) {
                *in_degree.entry(task.clone()).or_insert(0) += 1;
                adj.entry(dep.clone()).or_insert_with(Vec::new).push(task.clone());
            }
        }
    }
    
    // Kahn's algorithm
    let mut queue: VecDeque<String> = VecDeque::new();
    for (task, degree) in &in_degree {
        if *degree == 0 {
            queue.push_back(task.clone());
        }
    }
    
    let mut result = Vec::new();
    while let Some(task) = queue.pop_front() {
        result.push(task);
        if let Some(neighbors) = adj.get(&task) {
            for neighbor in neighbors {
                let degree = in_degree.get_mut(neighbor).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    queue.push_back(neighbor.clone());
                }
            }
        }
    }
    
    if result.len() != pending.len() {
        return Err(GitError::SubprocessFailure {
            command: "determine_merge_order".to_string(),
            exit_code: -1,
            stdout: String::new(),
            stderr: "Circular dependency detected in merge order".to_string(),
        });
    }
    
    Ok(result)
}

/// Determine which pending tasks can be merged next.
///
/// A task is mergeable when ALL of its dependencies are in the
/// `merged_tasks` list. Returns tasks that can be merged in parallel.
pub fn next_mergeable_tasks(plan: &MergePlan) -> Result<Vec<String>> {
    let merged: HashSet<&str> = plan.merged_tasks.iter().map(|s| s.as_str()).collect();
    
    let mut ready = Vec::new();
    for task in &plan.pending_tasks {
        let deps = plan.dependencies.get(task).unwrap_or(&Vec::new());
        let all_satisfied = deps.iter().all(|dep| merged.contains(dep.as_str()));
        if all_satisfied {
            ready.push(task.clone());
        }
    }
    
    Ok(ready)
}

/// Execute merges in dependency order.
///
/// Repeatedly finds mergeable tasks via `next_mergeable_tasks()`, merges
/// them into the plan branch, and updates the merged_tasks list.
/// Continues until no more tasks are mergeable or all are merged.
///
/// Returns the list of successfully merged task IDs.
pub async fn execute_merge_sequence(plan: &mut MergePlan) -> Result<Vec<String>> {
    let mut merged = Vec::new();
    
    loop {
        let ready = next_mergeable_tasks(plan)?;
        if ready.is_empty() {
            break;
        }
        
        for task_id in &ready {
            merge_task_branch(&plan.repo_root, task_id, &plan.plan_branch, &plan.merged_tasks).await?;
            
            // Move task from pending to merged
            plan.pending_tasks.retain(|t| t != task_id);
            plan.merged_tasks.push(task_id.clone());
            merged.push(task_id.clone());
        }
    }
    
    Ok(merged)
}

/// Cleanup after successful merge: delete task branch and remove worktree.
///
/// 1. Delete the task branch.
/// 2. Remove the worktree (with force fallback for stale worktrees).
pub async fn cleanup_merged_task(repo_root: &Path, task_id: &str) -> Result<()> {
    // Delete task branch
    delete_branch(repo_root, &task_branch_name(task_id)).await?;
    
    // Remove worktree
    if let Err(e) = remove_worktree(repo_root, task_id).await {
        // Fallback to force removal
        tracing::warn!("Worktree removal failed for {}, attempting force removal: {}", task_id, e);
        remove_worktree_force(repo_root, task_id).await?;
    }
    
    Ok(())
}
