//! Git worktree lifecycle management.
//!
//! This module provides functions for spawning, removing, listing, and
//! querying git worktrees. Worktrees are stored under `<repo_root>/.worktrees/<task_id>/`
//! and are used to isolate each task's working directory.

use std::path::{Path, PathBuf};

use tokio::fs;

use super::errors::{GitError, Result};
use super::subprocess::git;

/// Information about a single git worktree.
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// The absolute path to the worktree directory.
    pub path: PathBuf,
    /// The branch name checked out in this worktree.
    pub branch: String,
    /// Whether the worktree is a bare repository.
    pub is_bare: bool,
}

/// Compute the worktree path for a given task ID.
///
/// Returns `<repo_root>/.worktrees/<task_id>/`.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `task_id` - The task identifier (e.g., `TASK-001`).
pub fn worktree_path(repo_root: &Path, task_id: &str) -> PathBuf {
    repo_root.join(".worktrees").join(task_id)
}

/// Return the base directory containing all worktrees.
///
/// Returns `<repo_root>/.worktrees/`.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
pub fn worktrees_dir(repo_root: &Path) -> PathBuf {
    repo_root.join(".worktrees")
}

// Check whether a worktree directory exists on disk.
// Unlike the public `worktree_exists`, this only checks for the directory itself,
// not for the `.git` pointer file. This is useful for cleanup operations
// where a worktree may be corrupted (missing `.git` file).
async fn worktree_dir_exists(wt_path: &Path) -> bool {
    fs::try_exists(wt_path).await.unwrap_or_else(|e| {
        tracing::debug!("worktree_dir_exists check failed for {}: {}", wt_path.display(), e);
        false
    })
}

// Prune stale worktree metadata from git's internal database.
// After a manual `fs::remove_dir_all` fallback, the directory is gone but
// git's `.git/worktrees/` metadata still references it. Running `git worktree
// prune` cleans up those stale entries. Errors are logged but not propagated,
// since the directory removal (the critical cleanup) already succeeded.
async fn prune_stale_worktree_metadata(repo_root: &Path) {
    if let Err(e) = git(repo_root, &["worktree", "prune"]).await {
        tracing::debug!("Failed to prune stale worktree metadata: {}", e);
    }
}

/// Spawn a new git worktree for the given task.
///
/// Creates a new worktree at `<repo_root>/.worktrees/<task_id>/` checked
/// out to the specified branch. If the worktree already exists, returns
/// an error.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `task_id` - The task identifier used to name the worktree directory.
/// * `branch_name` - The branch to checkout in the new worktree.
///
/// # Returns
///
/// The absolute path to the newly created worktree directory.
///
/// # Errors
///
/// Returns [`GitError::WorktreeExists`] if a worktree already exists at
/// the target path. Returns [`GitError::SubprocessFailure`] if the git
/// command fails. On failure, any partially created directory is cleaned up.
pub async fn spawn_worktree(repo_root: &Path, task_id: &str, branch_name: &str) -> Result<PathBuf> {
    let wt_path = worktree_path(repo_root, task_id);

    // Check if worktree already exists
    if worktree_exists(repo_root, task_id).await? {
        return Err(GitError::WorktreeExists {
            path: wt_path.clone(),
        });
    }

    // Ensure parent directory exists
    if let Some(parent) = wt_path.parent() {
        fs::create_dir_all(parent).await.map_err(|e| GitError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }

    // Spawn the worktree
    let output = git(
        repo_root,
        &["worktree", "add", &wt_path.to_string_lossy(), branch_name],
    )
    .await;

    match output {
        Ok(_) => Ok(wt_path),
        Err(e) => {
            // Clean up partial directory creation
            let _ = fs::remove_dir_all(&wt_path).await;
            Err(e)
        }
    }
}

/// Remove a worktree gracefully.
///
/// Attempts `git worktree remove` first. If the worktree is stale
/// (has a lock file), falls back to force removal. Cleans up any
/// remaining directory afterward.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `task_id` - The task identifier whose worktree should be removed.
///
/// # Errors
///
/// Returns [`GitError::WorktreeNotFound`] if the worktree does not exist.
pub async fn remove_worktree(repo_root: &Path, task_id: &str) -> Result<()> {
    let wt_path = worktree_path(repo_root, task_id);

    // Check directory existence (handles corrupted worktrees without .git pointer).
    if !worktree_dir_exists(&wt_path).await {
        return Err(GitError::WorktreeNotFound {
            path: wt_path.clone(),
        });
    }

    // Try normal removal first
    let output = git(
        repo_root,
        &["worktree", "remove", &wt_path.to_string_lossy()],
    )
    .await;

    match output {
        Ok(_) => {
            // Clean up any remaining directory
            let _ = fs::remove_dir_all(&wt_path).await;
            Ok(())
        }
        Err(_) => {
            // Normal removal failed — likely stale (lock file present).
            // Try force removal.
            let force_output = git(
                repo_root,
                &["worktree", "remove", "--force", &wt_path.to_string_lossy()],
            )
            .await;

            match force_output {
                Ok(_) => {
                    // Clean up any remaining directory
                    let _ = fs::remove_dir_all(&wt_path).await;
                    Ok(())
                }
                Err(_) => {
                    // Force removal also failed — fall back to manual cleanup
                    fs::remove_dir_all(&wt_path)
                        .await
                        .map_err(|e| GitError::Io {
                            path: wt_path.clone(),
                            source: e,
                        })?;
                    prune_stale_worktree_metadata(repo_root).await;
                    Ok(())
                }
            }
        }
    }
}

/// Force remove a worktree, bypassing stale-state checks.
///
/// Attempts `git worktree remove --force` first. If that fails, falls
/// back to directly removing the directory with `fs::remove_dir_all`.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `task_id` - The task identifier whose worktree should be force-removed.
///
/// # Errors
///
/// Returns [`GitError::WorktreeNotFound`] if the worktree does not exist.
pub async fn remove_worktree_force(repo_root: &Path, task_id: &str) -> Result<()> {
    let wt_path = worktree_path(repo_root, task_id);

    // Check directory existence (handles corrupted worktrees without .git pointer).
    if !worktree_dir_exists(&wt_path).await {
        return Err(GitError::WorktreeNotFound {
            path: wt_path.clone(),
        });
    }

    // Try force removal via git
    let output = git(
        repo_root,
        &["worktree", "remove", "--force", &wt_path.to_string_lossy()],
    )
    .await;

    match output {
        Ok(_) => {
            // Clean up any remaining directory
            let _ = fs::remove_dir_all(&wt_path).await;
            Ok(())
        }
        Err(_) => {
            // Git force removal failed — fall back to manual directory removal
            fs::remove_dir_all(&wt_path)
                .await
                .map_err(|e| GitError::Io {
                    path: wt_path.clone(),
                    source: e,
                })?;
            prune_stale_worktree_metadata(repo_root).await;
            Ok(())
        }
    }
}

/// List all worktrees in the repository.
///
/// Runs `git worktree list --porcelain` and parses the output to extract
/// the path and branch for each worktree.
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
///
/// # Returns
///
/// A vector of [`WorktreeInfo`] structs, one per worktree.
///
/// # Errors
///
/// Returns an error if the git command fails or output cannot be parsed.
pub async fn list_worktrees(repo_root: &Path) -> Result<Vec<WorktreeInfo>> {
    let output = git(repo_root, &["worktree", "list", "--porcelain"]).await?;

    let mut worktrees = Vec::new();
    let mut current = Option::<WorktreeInfo>::None;

    for line in output.stdout.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("worktree ") {
            // Format: "worktree <path>\n"
            let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
            if parts.len() == 2 {
                current = Some(WorktreeInfo {
                    path: PathBuf::from(parts[1]),
                    branch: String::new(),
                    is_bare: false,
                });
            }
        } else if trimmed.starts_with("branch ") {
            // Format: "branch <sha> <branch_name>\n"
            if let Some(ref mut wt) = current {
                let parts: Vec<&str> = trimmed.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    let rest = parts[1];
                    let branch_parts: Vec<&str> = rest.splitn(2, ' ').collect();
                    if branch_parts.len() == 2 {
                        wt.branch = branch_parts[1].to_string();
                    }
                }
            }
        } else if trimmed == "bare" {
            // Bare repository marker
            if let Some(ref mut wt) = current {
                wt.is_bare = true;
            }
        } else if trimmed.is_empty() {
            // Empty line separates worktree blocks
            if let Some(wt) = current.take() {
                worktrees.push(wt);
            }
        }
    }

    // Push the last worktree if present
    if let Some(wt) = current.take() {
        worktrees.push(wt);
    }

    Ok(worktrees)
}

/// Check whether a worktree exists for the given task ID.
///
/// A worktree is considered to exist if its directory is present and
/// contains a `.git` file (the git worktree pointer file).
///
/// # Arguments
///
/// * `repo_root` - The root directory of the git repository.
/// * `task_id` - The task identifier to check.
///
/// # Returns
///
/// `true` if the worktree directory exists and contains a `.git` file,
/// `false` otherwise.
pub async fn worktree_exists(repo_root: &Path, task_id: &str) -> Result<bool> {
    let wt_path = worktree_path(repo_root, task_id);

    // Check if the directory exists
    let dir_meta = match fs::metadata(&wt_path).await {
        Ok(meta) => meta,
        Err(_) => return Ok(false),
    };

    if !dir_meta.is_dir() {
        return Ok(false);
    }

    // Check for .git file (worktree pointer)
    let git_pointer = wt_path.join(".git");
    let git_meta = match fs::metadata(&git_pointer).await {
        Ok(meta) => meta,
        Err(_) => return Ok(false),
    };

    Ok(git_meta.is_file())
}
