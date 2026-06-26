//! Merge coordination for builder workflow.
//!
//! This module handles merging task branches into the plan branch,
//! handling merge conflicts, updating execution state, and triggering
//! dependency auto-queue for downstream tasks.
//!
//! See `design/persistence.md` branch strategy section.

use std::path::PathBuf;

use crate::builder::dispatcher::TaskContext;
use crate::builder::errors::{BuilderError, Result};
use crate::builder::worktree_manager::TaskWorktree;
use crate::git;
use crate::overlord::{DependencyResolver, OverlordScheduler, TransitionTaskStatusParams};
use crate::persistence::{
    read_execution_state, update_execution_state, TaskStatusValue,
};

/// Result of a merge operation.
#[derive(Debug, Clone)]
pub enum MergeResult {
    /// Merge succeeded.
    Success,
    /// Merge produced conflicts.
    Conflict { conflicted_files: Vec<String> },
}

/// Coordinates merging task branches into the plan branch.
pub struct MergeCoordinator {
    /// Repository root path.
    repo_root: PathBuf,
    /// Reference to the Overlord scheduler.
    overlord: std::sync::Arc<OverlordScheduler>,
    /// Serializes merge operations to prevent concurrent git state corruption.
    merge_lock: tokio::sync::Mutex<()>,
}

impl MergeCoordinator {
    /// Create a new merge coordinator.
    pub fn new(repo_root: PathBuf, overlord: std::sync::Arc<OverlordScheduler>) -> Self {
        Self {
            repo_root,
            overlord,
            merge_lock: tokio::sync::Mutex::new(()),
        }
    }

    /// Merge task branch into plan branch.
    ///
    /// # Steps
    /// 1. Acquire merge lock to serialize concurrent merge operations
    /// 2. Switch to plan branch
    /// 3. Merge task branch with --no-ff
    /// 4. On success: return MergeResult::Success
    /// 5. On conflict: return MergeResult::Conflict with conflicted files
    ///
    /// The merge lock prevents concurrent git state corruption when multiple
    /// tasks attempt to merge into the same plan branch simultaneously.
    pub async fn merge_task_branch(
        &self,
        task_context: &TaskContext,
        worktree: &TaskWorktree,
    ) -> Result<MergeResult> {
        // Acquire merge lock to serialize concurrent merge operations
        let _lock = self.merge_lock.lock().await;

        let repo_root = self.repo_root.as_path();

        // Checkout plan branch
        git::checkout_branch(repo_root, &task_context.branch).await.map_err(BuilderError::MergeError)?;

        // Merge task branch with --no-ff
        let branch_name = &worktree.branch_name;
        let merge_message = format!("Merge {}: {}", branch_name, task_context.task_name);

        // Use git merge command directly
        match git::git(repo_root, &[
            "merge",
            branch_name,
            "--no-ff",
            "-m",
            &merge_message,
        ])
        .await
        {
            Ok(_) => {
                tracing::info!(
                    "Merged task {} ({}) into plan branch {}",
                    task_context.task_id,
                    task_context.task_name,
                    task_context.branch
                );
                Ok(MergeResult::Success)
            }
            Err(git::GitError::SubprocessFailure { exit_code, stderr, .. }) if exit_code == 1 => {
                // Check for merge conflicts
                let conflicted_files = git::list_conflicted_files(repo_root).await.map_err(BuilderError::MergeError)?;

                if conflicted_files.is_empty() {
                    // Merge failed but no conflicts — abort and return error
                    let _ = git::abort_merge(repo_root).await;
                    Err(BuilderError::MergeError(git::GitError::SubprocessFailure {
                        command: format!("merge {} --no-ff", branch_name),
                        exit_code,
                        stdout: String::new(),
                        stderr,
                    }))
                } else {
                    // Merge conflict detected
                    tracing::warn!(
                        "Merge conflict for task {}: conflicts in {:?}",
                        task_context.task_id,
                        conflicted_files
                    );
                    // Abort the merge to leave repo in clean state
                    let _ = git::abort_merge(repo_root).await;
                    Ok(MergeResult::Conflict {
                        conflicted_files,
                    })
                }
            }
            Err(e) => {
                // Other error — abort merge
                let _ = git::abort_merge(repo_root).await;
                Err(BuilderError::MergeError(e))
            }
        }
    }

    /// Clean up after successful merge.
    ///
    /// # Steps
    /// 1. Delete task branch
    /// 2. Remove worktree
    /// 3. Force remove directory if needed
    pub async fn post_merge_cleanup(
        &self,
        task_context: &TaskContext,
        worktree: &TaskWorktree,
    ) -> Result<()> {
        let repo_root = self.repo_root.as_path();

        // Delete task branch
        git::delete_branch(repo_root, &worktree.branch_name).await.map_err(BuilderError::WorktreeError)?;

        // Remove worktree
        let wt_path = &worktree.path;
        if let Err(_e) = git::git(repo_root, &["worktree", "remove", &wt_path.to_string_lossy()]).await {
            // Try force
            if git::git(repo_root, &["worktree", "remove", "--force", &wt_path.to_string_lossy()]).await.is_err() {
                // Fall back to manual removal
                if let Err(e) = tokio::fs::remove_dir_all(wt_path).await {
                    tracing::error!("Failed to remove worktree directory: {}", e);
                }
            }
        }

        // Prune stale metadata
        let _ = git::git(repo_root, &["worktree", "prune"]).await;

        tracing::info!(
            "Cleaned up worktree for task {}",
            task_context.task_id
        );

        Ok(())
    }

    /// Update execution.json task status map.
    pub async fn update_execution_state(
        &self,
        task_context: &TaskContext,
        new_status: TaskStatusValue,
    ) -> Result<()> {
        let repo_root = self.repo_root.as_path();

        let mut state = read_execution_state(
            repo_root,
            &task_context.branch,
            &task_context.plan_id,
            &task_context.plan_name,
        )
        .map_err(BuilderError::PersistenceError)?;

        state
            .task_status_map
            .insert(task_context.task_id.clone(), new_status);

        update_execution_state(
            repo_root,
            &task_context.branch,
            &task_context.plan_id,
            &task_context.plan_name,
            &state,
        )
        .map_err(BuilderError::PersistenceError)?;

        Ok(())
    }

    /// Handle a merge conflict.
    ///
    /// # Steps
    /// 1. Abort the merge
    /// 2. Transition task to waiting-manual-review
    /// 3. Record conflict details in status.json
    /// 4. Actor: "overlord-merge-conflict"
    pub async fn handle_merge_conflict(&self, task_context: &TaskContext) -> Result<()> {
        // Transition to waiting-manual-review
        self.overlord
            .transition_task_status(&TransitionTaskStatusParams {
                branch: task_context.branch.clone(),
                plan_id: task_context.plan_id.clone(),
                plan_name: task_context.plan_name.clone(),
                task_id: task_context.task_id.clone(),
                task_name: task_context.task_name.clone(),
                new_status: TaskStatusValue::WaitingManualReview,
                by: "overlord-merge-conflict".to_string(),
            })
            .await
            .map_err(BuilderError::OverlordError)?;

        tracing::warn!(
            "Task {} transitioned to waiting-manual-review due to merge conflict",
            task_context.task_id
        );

        Ok(())
    }

    /// Trigger dependency auto-queue after successful merge.
    ///
    /// Checks if dependent tasks are now eligible for dispatch.
    /// The resolver handles auto-queueing internally, so we return `()` on success.
    pub async fn trigger_dependency_auto_queue(
        &self,
        task_context: &TaskContext,
    ) -> Result<()> {
        let resolver = DependencyResolver::new();
        resolver
            .auto_queue_tasks(
                self.repo_root.as_path(),
                &task_context.branch,
                &task_context.plan_id,
                &task_context.plan_name,
            )
            .map_err(|e| BuilderError::OverlordError(crate::overlord::OverlordError::DependencyError(e.to_string())))?;

        Ok(())
    }
}
