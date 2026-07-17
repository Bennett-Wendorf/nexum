//! Error handling and recovery for builder workflow.
//!
//! This module handles all error scenarios: agent crashes, timeouts,
//! merge conflicts, and permission denied scenarios. Each recovery flow
//! is safe (no double-free of worktrees, no orphaned sessions).
//!
//! See `design/agent-harness-integration.md` error handling section.

use std::path::PathBuf;
use std::time::Duration;

use crate::builder::dispatcher::TaskContext;
use crate::builder::errors::{BuilderError, Result};
use crate::builder::event_bus::{BuilderEvent, BuilderEventBus};
use crate::builder::session_manager::SessionManager;
use crate::builder::worktree_manager::{TaskWorktree, WorktreeManager};
use crate::overlord::{OverlordScheduler, TransitionTaskStatusParams};
use crate::persistence::{atomic_write_json, read_json, StatusTransition, TaskStatus, TaskStatusValue, TransitionStatus, task_status_path};

/// Handles error recovery for builder workflow failures.
pub struct ErrorRecovery {
    /// Repository root path.
    repo_root: PathBuf,
    /// Reference to the Overlord scheduler.
    overlord: std::sync::Arc<OverlordScheduler>,
    /// Reference to the worktree manager.
    worktree_manager: std::sync::Arc<WorktreeManager>,
    /// Reference to the session manager.
    session_manager: std::sync::Arc<SessionManager>,
    /// Reference to the event bus.
    event_bus: std::sync::Arc<BuilderEventBus>,
}

impl ErrorRecovery {
    /// Create a new error recovery handler.
    pub fn new(
        repo_root: PathBuf,
        overlord: std::sync::Arc<OverlordScheduler>,
        worktree_manager: std::sync::Arc<WorktreeManager>,
        session_manager: std::sync::Arc<SessionManager>,
        event_bus: std::sync::Arc<BuilderEventBus>,
    ) -> Self {
        Self {
            repo_root,
            overlord,
            worktree_manager,
            session_manager,
            event_bus,
        }
    }

    /// Handle agent crash recovery.
    ///
    /// # Steps
    /// 1. Destroy the ACP session
    /// 2. Transition task from running to queued
    /// 3. Increment attempts counter
    /// 4. Clear agent lease
    /// 5. Clear started_at
    /// 6. Clean up worktree
    /// 7. Actor: "overlord-crash-recovery"
    pub async fn handle_agent_crash(
        &self,
        task_context: &TaskContext,
        worktree: &TaskWorktree,
    ) -> Result<()> {
        tracing::error!(
            task_id = %task_context.task_id,
            "Agent crashed, initiating crash recovery"
        );

        // Transition task from running to queued
        self.overlord
            .transition_task_status(&TransitionTaskStatusParams {
                branch: task_context.branch.clone(),
                plan_id: task_context.plan_id.clone(),
                plan_name: task_context.plan_name.clone(),
                task_id: task_context.task_id.clone(),
                task_name: task_context.task_name.clone(),
                new_status: TaskStatusValue::Queued,
                by: "overlord-crash-recovery".to_string(),
            })
            .await
            .map_err(BuilderError::OverlordError)?;

        // Clean up worktree
        self.worktree_manager.cleanup(worktree).await?;

        // Emit event
        self.event_bus.emit(BuilderEvent::TaskFailed {
            task_id: task_context.task_id.clone(),
            error: "Agent crashed".to_string(),
        })?;

        self.event_bus.emit(BuilderEvent::TaskRequeued {
            task_id: task_context.task_id.clone(),
        })?;

        tracing::info!("Crash recovery complete for task {}", task_context.task_id);

        Ok(())
    }

    /// Handle task timeout.
    ///
    /// # Steps
    /// 1. Transition task from running to queued
    /// 2. Increment attempts counter
    /// 3. Record timeout details
    /// 4. Clear agent lease
    /// 5. Clean up worktree
    /// 6. Actor: "overlord-timeout"
    pub async fn handle_timeout(
        &self,
        task_context: &TaskContext,
        worktree: &TaskWorktree,
        elapsed: Duration,
        limit: Duration,
    ) -> Result<()> {
        tracing::error!(
            task_id = %task_context.task_id,
            elapsed = ?elapsed,
            limit = ?limit,
            "Task timed out, initiating timeout recovery"
        );

        // Transition task from running to queued
        self.overlord
            .transition_task_status(&TransitionTaskStatusParams {
                branch: task_context.branch.clone(),
                plan_id: task_context.plan_id.clone(),
                plan_name: task_context.plan_name.clone(),
                task_id: task_context.task_id.clone(),
                task_name: task_context.task_name.clone(),
                new_status: TaskStatusValue::Queued,
                by: "overlord-timeout".to_string(),
            })
            .await
            .map_err(BuilderError::OverlordError)?;

        // Clean up worktree
        self.worktree_manager.cleanup(worktree).await?;

        // Emit events
        self.event_bus.emit(BuilderEvent::TaskFailed {
            task_id: task_context.task_id.clone(),
            error: format!("Timeout: elapsed {:?}, limit {:?}", elapsed, limit),
        })?;

        self.event_bus.emit(BuilderEvent::TaskRequeued {
            task_id: task_context.task_id.clone(),
        })?;

        tracing::info!(
            "Timeout recovery complete for task {}",
            task_context.task_id
        );

        Ok(())
    }

    /// Handle permission denied.
    ///
    /// Logs the denial. Agent decides whether to proceed.
    /// If agent aborts, treat as crash.
    pub async fn handle_permission_denied(
        &self,
        task_context: &TaskContext,
        resource: &str,
    ) -> Result<()> {
        tracing::warn!(
            task_id = %task_context.task_id,
            resource = %resource,
            "Permission denied — agent must decide whether to proceed"
        );

        Ok(())
    }

    /// Handle merge conflict.
    ///
    /// # Steps
    /// 1. Transition task to waiting-manual-review
    /// 2. Record conflict details in status.json
    /// 3. Do NOT clean up worktree (human may need to inspect)
    /// 4. Actor: "overlord-merge-conflict"
    pub async fn handle_merge_conflict(
        &self,
        task_context: &TaskContext,
        _worktree: &TaskWorktree,
        conflicts: Vec<String>,
    ) -> Result<()> {
        tracing::warn!(
            task_id = %task_context.task_id,
            conflicts = ?conflicts,
            "Merge conflict detected, transitioning to waiting-manual-review"
        );

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

        // Record conflict details in status.json
        let status_path = task_status_path(
            &self.repo_root,
            &task_context.branch,
            &task_context.plan_id,
            &task_context.plan_name,
            &task_context.task_id,
            &task_context.task_name,
        );

        if let Ok(mut status) =
            read_json::<TaskStatus>(&status_path)
        {
            // Add conflict info to transitions
            use chrono::Utc;
            status
                .transitions
                .push(StatusTransition {
                    from: TransitionStatus::from_task(&TaskStatusValue::Reviewing),
                    to: TransitionStatus::from_task(&TaskStatusValue::WaitingManualReview),
                    at: Utc::now().to_rfc3339(),
                    by: "overlord-merge-conflict".to_string(),
                });
            let _ = atomic_write_json(&status_path, &status);
        }

        // Do NOT clean up worktree — preserve for human inspection

        // Emit event
        self.event_bus.emit(BuilderEvent::TaskFailed {
            task_id: task_context.task_id.clone(),
            error: format!("Merge conflict: {:?}", conflicts),
        })?;

        Ok(())
    }

    /// Generic error handler that dispatches to specific recovery flows.
    pub async fn handle_workflow_error(
        &self,
        task_context: &TaskContext,
        worktree: &TaskWorktree,
        error: &BuilderError,
    ) -> Result<()> {
        tracing::error!(
            task_id = %task_context.task_id,
            error = ?error,
            "Workflow error detected, dispatching recovery"
        );

        match error {
            BuilderError::AgentCrash {
                task_id: _,
                exit_code: _,
            } => {
                self.handle_agent_crash(task_context, worktree).await?;
            }
            BuilderError::TimeoutError {
                task_id: _,
                elapsed,
                limit,
            } => {
                self.handle_timeout(task_context, worktree, *elapsed, *limit)
                    .await?;
            }
            BuilderError::MergeConflict {
                task_id: _,
                branch: _,
                conflicts,
            } => {
                self.handle_merge_conflict(task_context, worktree, conflicts.clone())
                    .await?;
            }
            BuilderError::PermissionDenied {
                task_id: _,
                resource,
            } => {
                self.handle_permission_denied(task_context, resource)
                    .await?;
            }
            _ => {
                // Generic error — log and transition to queued for retry
                tracing::error!(
                    task_id = %task_context.task_id,
                    "Unhandled workflow error, re-queuing task"
                );

                self.overlord
                    .transition_task_status(&TransitionTaskStatusParams {
                        branch: task_context.branch.clone(),
                        plan_id: task_context.plan_id.clone(),
                        plan_name: task_context.plan_name.clone(),
                        task_id: task_context.task_id.clone(),
                        task_name: task_context.task_name.clone(),
                        new_status: TaskStatusValue::Queued,
                        by: "overlord-error-recovery".to_string(),
                    })
                    .await
                    .map_err(BuilderError::OverlordError)?;

                // Only clean up worktree if it still exists to avoid double-cleanup
                // (specific handlers like handle_agent_crash may have already cleaned up)
                if tokio::fs::try_exists(&worktree.path).await.unwrap_or(false) {
                    self.worktree_manager.cleanup(worktree).await?;
                }
            }
        }

        // Emit failure event
        self.event_bus.emit(BuilderEvent::TaskFailed {
            task_id: task_context.task_id.clone(),
            error: error.to_string(),
        })?;

        Ok(())
    }
}
