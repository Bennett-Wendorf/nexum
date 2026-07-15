//! Worktree setup and teardown for builder tasks.
//!
//! This module manages the creation and cleanup of isolated git worktrees
//! for each builder task. Each task gets its own worktree spawned from the
//! plan branch, ensuring complete isolation between parallel tasks.
//!
//! See `design/persistence.md` branch strategy section for the full design.

use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};

use crate::builder::errors::{BuilderError, Result};
use crate::git;

/// Metadata about a task's isolated worktree.
#[derive(Debug, Clone)]
pub struct TaskWorktree {
    /// Task identifier, e.g. `"TASK-001"`.
    pub task_id: String,
    /// Short task name, e.g. `"add-auth-flow"`.
    pub task_name: String,
    /// Task branch name, e.g. `"task/TASK-001"`.
    pub branch_name: String,
    /// Absolute path to the worktree directory.
    pub path: PathBuf,
    /// Timestamp when the worktree was created.
    pub created_at: DateTime<Utc>,
}

impl TaskWorktree {
    /// Generate the task branch name from a task ID.
    ///
    /// Returns `task/<TASK_ID>` (e.g., `task/TASK-001`).
    pub fn branch_name(task_id: &str) -> String {
        git::task_branch_name(task_id)
    }

    /// Compute the worktree path for a task.
    ///
    /// Returns `.worktrees/<TASK_ID>-<task_name>/` relative to repo root.
    pub fn worktree_path(repo_root: &Path, task_id: &str, task_name: &str) -> PathBuf {
        repo_root
            .join(".worktrees")
            .join(format!("{}-{}", task_id, task_name))
    }
}

/// Manages worktree lifecycle for builder tasks.
///
/// Creates isolated git worktrees per task from the plan branch,
/// and handles cleanup after task completion or failure.
pub struct WorktreeManager {
    /// Absolute path to the repository root.
    repo_root: PathBuf,
}

impl WorktreeManager {
    /// Create a new worktree manager.
    pub fn new(repo_root: PathBuf) -> Self {
        Self { repo_root }
    }

    /// Set up a worktree for a task.
    ///
    /// # Steps
    /// 1. Compute branch name: `task/<TASK_ID>`
    /// 2. Compute worktree path: `.worktrees/<TASK_ID>-<task_name>/`
    /// 3. Create task branch from plan branch
    /// 4. Spawn worktree at the computed path
    /// 5. Return `TaskWorktree` with all metadata
    ///
    /// On failure, cleans up any partial state (branch, directory).
    pub async fn setup(
        &self,
        task_id: &str,
        task_name: &str,
        plan_branch: &str,
    ) -> Result<TaskWorktree> {
        let repo_root = self.repo_root.as_path();
        let branch_name = TaskWorktree::branch_name(task_id);
        let wt_path = TaskWorktree::worktree_path(repo_root, task_id, task_name);

        // Ensure parent directory exists
        if let Some(parent) = wt_path.parent() {
            tokio::fs::create_dir_all(parent).await.map_err(|e| {
                BuilderError::WorkflowError(format!(
                    "Failed to create worktree parent directory: {}",
                    e
                ))
            })?;
        }

        // Step 1: Create task branch from plan branch
        match git::create_task_branch(repo_root, task_id, plan_branch).await {
            Ok(_) => {}
            Err(e) => {
                // If branch already exists, that's okay — we can still use it
                if let git::GitError::SubprocessFailure { stderr, .. } = &e {
                    if stderr.contains("already exists") {
                        tracing::warn!("Task branch {} already exists, reusing", branch_name);
                    } else {
                        return Err(BuilderError::WorktreeError(e));
                    }
                } else {
                    return Err(BuilderError::WorktreeError(e));
                }
            }
        }

        // Step 2: Spawn worktree
        // We need to spawn at our custom path, not the default git worktree path
        // So we do it manually
        let wt_path_str = wt_path.to_string_lossy().to_string();
        match git::git(repo_root, &["worktree", "add", &wt_path_str, &branch_name]).await {
            Ok(_) => {}
            Err(e) => {
                // If worktree already exists, that's okay
                if let git::GitError::SubprocessFailure { stderr, .. } = &e {
                    if stderr.contains("already exists") {
                        tracing::warn!("Worktree already exists at {}, reusing", wt_path.display());
                    } else {
                        // Clean up branch on failure
                        let _ = git::delete_branch(repo_root, &branch_name).await;
                        return Err(BuilderError::WorktreeError(e));
                    }
                } else {
                    // Clean up branch on failure
                    let _ = git::delete_branch(repo_root, &branch_name).await;
                    return Err(BuilderError::WorktreeError(e));
                }
            }
        }

        Ok(TaskWorktree {
            task_id: task_id.to_string(),
            task_name: task_name.to_string(),
            branch_name,
            path: wt_path,
            created_at: Utc::now(),
        })
    }

    /// Set up a worktree and ensure `.agent/` directory structure exists.
    ///
    /// This is a convenience wrapper around `setup()` that also ensures
    /// the `.agent/` directory is present in the worktree.
    pub async fn setup_and_commit_agent_dir(
        &self,
        task_id: &str,
        task_name: &str,
        plan_branch: &str,
    ) -> Result<TaskWorktree> {
        let worktree = self.setup(task_id, task_name, plan_branch).await?;

        // Ensure .agent directory exists in worktree
        let agent_dir = worktree.path.join(".agent");
        tokio::fs::create_dir_all(&agent_dir).await.map_err(|e| {
            BuilderError::WorkflowError(format!("Failed to create .agent directory: {}", e))
        })?;

        Ok(worktree)
    }

    /// Clean up a worktree: remove worktree and delete task branch.
    ///
    /// # Steps
    /// 1. `git worktree remove <path>` — Remove the worktree
    /// 2. `git branch -D task/<TASK_ID>` — Delete the task branch
    /// 3. Force remove directory if git command fails
    pub async fn cleanup(&self, worktree: &TaskWorktree) -> Result<()> {
        let repo_root = self.repo_root.as_path();

        // Step 1: Try to remove worktree via git
        // We use the custom path, so we need to handle this carefully
        let wt_path = &worktree.path;

        // Try git worktree remove first
        match git::git(
            repo_root,
            &["worktree", "remove", &wt_path.to_string_lossy()],
        )
        .await
        {
            Ok(_) => {
                tracing::info!("Worktree removed via git for task {}", worktree.task_id);
            }
            Err(_) => {
                // Git worktree remove failed — try force
                match git::git(
                    repo_root,
                    &["worktree", "remove", "--force", &wt_path.to_string_lossy()],
                )
                .await
                {
                    Ok(_) => {
                        tracing::info!(
                            "Worktree force-removed via git for task {}",
                            worktree.task_id
                        );
                    }
                    Err(_) => {
                        // Force also failed — fall back to manual directory removal
                        tracing::warn!(
                            "Git worktree remove failed for {}, falling back to manual removal",
                            worktree.task_id
                        );
                        if let Err(e) = tokio::fs::remove_dir_all(wt_path).await {
                            tracing::error!("Manual worktree removal failed: {}", e);
                        }
                        // Prune stale metadata
                        let _ = git::git(repo_root, &["worktree", "prune"]).await;
                    }
                }
            }
        }

        // Step 2: Delete task branch (no-op if already gone)
        git::delete_branch(repo_root, &worktree.branch_name)
            .await
            .map_err(BuilderError::WorktreeError)?;

        // Force remove directory if it still exists
        if tokio::fs::try_exists(wt_path).await.unwrap_or(false) {
            if let Err(e) = tokio::fs::remove_dir_all(wt_path).await {
                tracing::warn!(
                    "Failed to remove worktree directory {}: {}",
                    wt_path.display(),
                    e
                );
            }
        }

        Ok(())
    }

    /// Check if a worktree exists for a task.
    ///
    /// Checks if the worktree directory exists at the expected path.
    pub async fn exists(&self, task_id: &str, task_name: &str) -> bool {
        let wt_path = TaskWorktree::worktree_path(self.repo_root.as_path(), task_id, task_name);
        tokio::fs::try_exists(&wt_path).await.unwrap_or(false)
    }

    /// List all active worktrees.
    ///
    /// Scans the `.worktrees/` directory and returns metadata for each
    /// worktree that matches the task worktree naming pattern.
    pub async fn list_active(&self) -> Result<Vec<TaskWorktree>> {
        let worktrees_dir = self.repo_root.join(".worktrees");

        if !tokio::fs::try_exists(&worktrees_dir).await.unwrap_or(false) {
            return Ok(Vec::new());
        }

        let mut entries = Vec::new();
        let mut reader = tokio::fs::read_dir(&worktrees_dir).await.map_err(|e| {
            BuilderError::WorkflowError(format!("Failed to read worktrees directory: {}", e))
        })?;

        while let Some(entry) = reader.next_entry().await.map_err(|e| {
            BuilderError::WorkflowError(format!("Failed to read directory entry: {}", e))
        })? {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }

            // Check for .git file (valid worktree marker)
            let git_pointer = path.join(".git");
            if !tokio::fs::try_exists(&git_pointer).await.unwrap_or(false) {
                continue;
            }

            // Parse directory name: "<TASK_ID>-<task_name>"
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                let parts: Vec<&str> = dir_name.splitn(2, '-').collect();
                if parts.len() == 2 {
                    let task_id = parts[0];
                    let task_name = parts[1];
                    let branch_name = TaskWorktree::branch_name(task_id);

                    // Try to derive created_at from filesystem metadata.
                    // Note: birth time is not reliably available on all filesystems
                    // (e.g., ext4), so we fall back to modification time or Utc::now().
                    let created_at = if let Ok(meta) = entry.metadata().await {
                        meta.created()
                            .ok()
                            .or_else(|| meta.modified().ok())
                            .map(DateTime::<Utc>::from)
                            .unwrap_or_else(Utc::now)
                    } else {
                        Utc::now()
                    };

                    entries.push(TaskWorktree {
                        task_id: task_id.to_string(),
                        task_name: task_name.to_string(),
                        branch_name,
                        path: path.clone(),
                        created_at,
                    });
                }
            }
        }

        Ok(entries)
    }
}
