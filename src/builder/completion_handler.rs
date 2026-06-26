//! Completion detection and post-execution logic for builder tasks.
//!
//! This module handles the completion phase of the builder workflow:
//! detecting completion from ACP events, triggering merge workflow,
//! handling merge conflicts, updating status, and cleaning up resources.
//!
//! See `design/agent-harness-integration.md` session lifecycle section.

use std::path::PathBuf;

use crate::builder::dispatcher::TaskContext;
use crate::builder::error_recovery::ErrorRecovery;
use crate::builder::errors::{BuilderError, Result};
use crate::builder::event_bus::{BuilderEvent, BuilderEventBus};
use crate::builder::merge_coordinator::{MergeCoordinator, MergeResult};
use crate::builder::worktree_manager::TaskWorktree;
use crate::overlord::{OverlordScheduler, TransitionTaskStatusParams};
use crate::persistence::TaskStatusValue;

/// Outcome of the completion handling process.
#[derive(Debug, Clone)]
pub enum CompletionOutcome {
    /// Task was successfully merged into the plan branch.
    Merged,
    /// Merge conflict detected, task requires manual review.
    Conflict,
}

/// Handles the completion phase of the builder workflow.
pub struct CompletionHandler {
    /// Repository root path.
    repo_root: PathBuf,
    /// Reference to the Overlord scheduler.
    overlord: std::sync::Arc<OverlordScheduler>,
    /// Reference to the merge coordinator.
    merge_coordinator: std::sync::Arc<MergeCoordinator>,
    /// Reference to the error recovery handler.
    error_recovery: std::sync::Arc<ErrorRecovery>,
    /// Reference to the event bus.
    event_bus: std::sync::Arc<BuilderEventBus>,
}

impl CompletionHandler {
    /// Create a new completion handler.
    pub fn new(
        repo_root: PathBuf,
        overlord: std::sync::Arc<OverlordScheduler>,
        merge_coordinator: std::sync::Arc<MergeCoordinator>,
        error_recovery: std::sync::Arc<ErrorRecovery>,
        event_bus: std::sync::Arc<BuilderEventBus>,
    ) -> Self {
        Self {
            repo_root,
            overlord,
            merge_coordinator,
            error_recovery,
            event_bus,
        }
    }

    /// Process task completion.
    ///
    /// # Steps
    /// 1. Transition task from running to reviewing (or merge-queue if auto-merge)
    /// 2. Record completed_at timestamp in status.json
    /// 3. Actor: "builder-completion"
    /// 4. Attempt merge via merge_coordinator
    /// 5. On merge success: transition to completed, cleanup, trigger dependency auto-queue
    /// 6. On merge conflict: handle via error_recovery
    pub async fn handle_completion(
        &self,
        task_context: &TaskContext,
        worktree: &TaskWorktree,
    ) -> Result<CompletionOutcome> {
        tracing::info!(
            task_id = %task_context.task_id,
            "Processing task completion"
        );

        // Step 1: Transition from running to reviewing
        self.overlord
            .transition_task_status(&TransitionTaskStatusParams {
                branch: task_context.branch.clone(),
                plan_id: task_context.plan_id.clone(),
                plan_name: task_context.plan_name.clone(),
                task_id: task_context.task_id.clone(),
                task_name: task_context.task_name.clone(),
                new_status: TaskStatusValue::Reviewing,
                by: "builder-completion".to_string(),
            })
            .await
            .map_err(BuilderError::OverlordError)?;

        // Step 2: Attempt merge
        match self.merge_coordinator
            .merge_task_branch(task_context, worktree)
            .await?
        {
            MergeResult::Success => {
                tracing::info!(
                    task_id = %task_context.task_id,
                    "Merge successful, transitioning to completed"
                );

                // Transition to completed
                self.overlord
                    .transition_task_status(&TransitionTaskStatusParams {
                        branch: task_context.branch.clone(),
                        plan_id: task_context.plan_id.clone(),
                        plan_name: task_context.plan_name.clone(),
                        task_id: task_context.task_id.clone(),
                        task_name: task_context.task_name.clone(),
                        new_status: TaskStatusValue::Completed,
                        by: "builder-completion".to_string(),
                    })
                    .await
                    .map_err(BuilderError::OverlordError)?;

                // Update execution state
                self.merge_coordinator
                    .update_execution_state(task_context, TaskStatusValue::Completed)
                    .await?;

                // Clean up worktree and branch
                self.merge_coordinator
                    .post_merge_cleanup(task_context, worktree)
                    .await?;

                // Trigger dependency auto-queue
                let _ = self.merge_coordinator
                    .trigger_dependency_auto_queue(task_context)
                    .await;

                // Emit events
                self.event_bus.emit(BuilderEvent::TaskCompleted {
                    task_id: task_context.task_id.clone(),
                    result: crate::builder::event_bus::CompletionResult::Completed,
                })?;

                self.event_bus.emit(BuilderEvent::TaskMerged {
                    task_id: task_context.task_id.clone(),
                })?;

                Ok(CompletionOutcome::Merged)
            }
            MergeResult::Conflict { conflicted_files } => {
                tracing::warn!(
                    task_id = %task_context.task_id,
                    conflicts = ?conflicted_files,
                    "Merge conflict detected"
                );

                // Handle conflict via error recovery
                self.error_recovery
                    .handle_merge_conflict(task_context, worktree, conflicted_files)
                    .await?;

                // Emit event
                self.event_bus.emit(BuilderEvent::TaskFailed {
                    task_id: task_context.task_id.clone(),
                    error: "Merge conflict".to_string(),
                })?;

                Ok(CompletionOutcome::Conflict)
            }
        }
    }

    /// Emit completion events to the event bus.
    pub async fn emit_completion_events(
        &self,
        task_context: &TaskContext,
        outcome: &CompletionOutcome,
    ) -> Result<()> {
        match outcome {
            CompletionOutcome::Merged => {
                self.event_bus.emit(BuilderEvent::TaskMerged {
                    task_id: task_context.task_id.clone(),
                })?;
            }
            CompletionOutcome::Conflict => {
                self.event_bus.emit(BuilderEvent::TaskFailed {
                    task_id: task_context.task_id.clone(),
                    error: "Merge conflict".to_string(),
                })?;
            }
        }

        Ok(())
    }
}
