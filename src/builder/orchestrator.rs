//! Workflow orchestrator for the builder system.
//!
//! This module provides the central coordinator that ties together all
//! builder sub-components into a complete task execution workflow:
//! dispatch → worktree setup → session creation → execution → completion
//! → merge → cleanup.
//!
//! The orchestrator holds references to all sub-components and implements
//! the full lifecycle of a builder task as a single `execute_task()` call.
//!
//! See `design/agent-harness-integration.md` for the full workflow design.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use tokio::task::JoinHandle;

use crate::acp::AgentConfig;
use crate::builder::completion_handler::CompletionHandler;
use crate::builder::dispatcher::{ActiveSession, TaskContext, TaskDispatcher};
use crate::builder::error_recovery::ErrorRecovery;
use crate::builder::errors::{BuilderError, Result};
use crate::builder::event_bus::{BuilderEvent, BuilderEventBus, CompletionResult};
use crate::builder::heartbeat::HeartbeatManager;
use crate::builder::merge_coordinator::MergeCoordinator;
use crate::builder::session_manager::{ACPSessionHandle, SessionManager};
use crate::builder::worktree_manager::{TaskWorktree, WorktreeManager};
use crate::overlord::OverlordScheduler;

/// Central orchestrator coordinating all builder sub-components.
///
/// Holds Arc-references to all sub-components, enabling shared ownership
/// across concurrent task executions. Components requiring `&mut self`
/// access (dispatcher) are wrapped in `tokio::sync::Mutex`. The event bus
/// uses interior mutability via `tokio::sync::Mutex` for session tracking,
/// so it can be shared directly as `Arc<BuilderEventBus>`.
pub struct WorkflowOrchestrator {
    /// Repository root path.
    repo_root: PathBuf,
    /// Maximum number of concurrent task executions.
    concurrency_limit: usize,
    /// Timeout duration for individual task execution.
    task_timeout: Duration,
    /// Task dispatcher — wrapped in Mutex for `&mut self` session tracking.
    dispatcher: Arc<tokio::sync::Mutex<TaskDispatcher>>,
    /// Worktree lifecycle manager.
    worktree_manager: Arc<WorktreeManager>,
    /// ACP session manager.
    session_manager: Arc<SessionManager>,
    /// Heartbeat update manager.
    heartbeat_manager: Arc<HeartbeatManager>,
    /// Builder event bus (uses interior mutability for session tracking).
    event_bus: Arc<BuilderEventBus>,
    /// Merge coordination handler.
    merge_coordinator: Arc<MergeCoordinator>,
    /// Error recovery handler.
    error_recovery: Arc<ErrorRecovery>,
    /// Completion processing handler.
    completion_handler: Arc<CompletionHandler>,
}

impl WorkflowOrchestrator {
    /// Create a new workflow orchestrator.
    ///
    /// # Arguments
    /// * `repo_root` — Absolute path to the repository root
    /// * `overlord` — Shared reference to the Overlord scheduler
    /// * `agent_config` — Configuration for spawning agent subprocesses
    /// * `concurrency_limit` — Maximum concurrent task executions
    /// * `task_timeout` — Timeout duration per task
    /// * `event_bus_capacity` — Broadcast channel capacity for the event bus
    pub fn new(
        repo_root: PathBuf,
        overlord: Arc<OverlordScheduler>,
        agent_config: AgentConfig,
        concurrency_limit: usize,
        task_timeout: Duration,
        event_bus_capacity: usize,
    ) -> Self {
        let worktree_manager = Arc::new(WorktreeManager::new(repo_root.clone()));
        let session_manager = Arc::new(SessionManager::new(
            repo_root.clone(),
            agent_config,
        ));
        let heartbeat_manager =
            Arc::new(HeartbeatManager::with_default_interval(repo_root.clone()));
        let event_bus = Arc::new(BuilderEventBus::new(event_bus_capacity));
        let merge_coordinator =
            Arc::new(MergeCoordinator::new(repo_root.clone(), overlord.clone()));
        let error_recovery = Arc::new(ErrorRecovery::new(
            repo_root.clone(),
            overlord.clone(),
            worktree_manager.clone(),
            session_manager.clone(),
            event_bus.clone(),
        ));
        let completion_handler = Arc::new(CompletionHandler::new(
            repo_root.clone(),
            overlord.clone(),
            merge_coordinator.clone(),
            error_recovery.clone(),
            event_bus.clone(),
        ));
        let dispatcher = Arc::new(tokio::sync::Mutex::new(TaskDispatcher::new(
            repo_root.clone(),
            overlord.clone(),
        )));

        Self {
            repo_root,
            concurrency_limit,
            task_timeout,
            dispatcher,
            worktree_manager,
            session_manager,
            heartbeat_manager,
            event_bus,
            merge_coordinator,
            error_recovery,
            completion_handler,
        }
    }

    /// Execute a single task through the complete workflow.
    ///
    /// This is the single entry point for task execution. The workflow
    /// proceeds through six phases:
    ///
    /// 1. **Setup phase**: Create worktree, emit `TaskStarted`
    /// 2. **Session phase**: Create ACP session, start heartbeat, start event relay
    /// 3. **Execution phase**: Wait for completion with timeout
    /// 4. **Completion phase**: Handle completion, destroy session, emit events
    /// 5. **Error phase**: Handle errors via `error_recovery`, destroy session, cleanup
    /// 6. **Cleanup phase**: Stop heartbeat, unregister session, cleanup worktree, emit `TaskCleanedUp`
    ///
    /// Cleanup is guaranteed to run regardless of success or failure.
    #[allow(unused_assignments)] // Initial None values overwritten before read for state tracking
    pub async fn execute_task(&self, task_context: TaskContext) -> Result<()> {
        let task_id = task_context.task_id.clone();
        let task_name = task_context.task_name.clone();
        let branch = task_context.branch.clone();

        // ── State tracking for conditional cleanup ──
        let mut worktree: Option<TaskWorktree> = None;
        let mut session: Option<ACPSessionHandle> = None;
        let mut heartbeat_handle: Option<JoinHandle<()>> = None;
        let mut error_recovery_ran = false;

        // ============================================================
        // SETUP PHASE: Create worktree, emit TaskStarted
        // ============================================================
        let wt = self
            .worktree_manager
            .setup_and_commit_agent_dir(&task_id, &task_name, &branch)
            .await?;
        worktree = Some(wt.clone());

        self.event_bus
            .emit(BuilderEvent::TaskStarted(task_context.clone()))?;

        tracing::info!(
            task_id = %task_id,
            worktree_path = %worktree.as_ref().unwrap().path.display(),
            "Setup phase complete"
        );

        // ============================================================
        // SESSION PHASE: Create ACP session, start heartbeat, start event relay
        // ============================================================
        let sess = self
            .session_manager
            .create_session(&task_context, worktree.as_ref().unwrap().path.as_path())
            .await?;
        let session_id = sess.session_id.clone();
        session = Some(sess);

        // Start periodic heartbeat
        let hb_handle = self
            .heartbeat_manager
            .start_periodic_heartbeat(&task_context)
            .await?;
        heartbeat_handle = Some(hb_handle);

        // Start event relay from session to builder event bus
        let event_rx = self.session_manager.get_event_stream(session.as_ref().unwrap());
        let _relay_handle = self
            .event_bus
            .relay_session_events(&task_id, event_rx)
            .await;

        // Register session in event bus for per-session subscription
        let session_tx = session.as_ref().unwrap().event_stream.sender();
        self.event_bus.register_session(&task_id, session_tx).await;

        // Track session in dispatcher
        self.dispatcher.lock().await.add_session(ActiveSession {
            task_context: task_context.clone(),
            worktree: worktree.clone().unwrap(),
            session_id: session_id.clone(),
            pid: session
                .as_ref()
                .unwrap()
                .subprocess
                .child
                .as_ref()
                .and_then(|c| c.id()),
            started_at: session.as_ref().unwrap().started_at,
        });

        tracing::info!(
            task_id = %task_id,
            session_id = %session_id,
            "Session phase complete"
        );

        // ============================================================
        // EXECUTION PHASE: Wait for completion with timeout
        // ============================================================
        let completion_result = self
            .session_manager
            .wait_for_completion(session.as_ref().unwrap(), self.task_timeout)
            .await?;

        tracing::info!(
            task_id = %task_id,
            ?completion_result,
            "Execution phase complete"
        );

        // ============================================================
        // COMPLETION / ERROR PHASE
        // ============================================================

        match completion_result {
            CompletionResult::Completed => {
                // Successful completion — handle via completion handler
                let _outcome = self
                    .completion_handler
                    .handle_completion(&task_context, worktree.as_ref().unwrap())
                    .await?;

                self.event_bus
                    .emit(BuilderEvent::TaskCompleted {
                        task_id: task_id.clone(),
                        result: CompletionResult::Completed,
                    })?;
            }
            CompletionResult::Timeout(timeout) => {
                // Timeout — handle via error recovery
                self.error_recovery
                    .handle_timeout(
                        &task_context,
                        worktree.as_ref().unwrap(),
                        timeout,
                        self.task_timeout,
                    )
                    .await?;
                error_recovery_ran = true;
            }
            CompletionResult::Crashed { exit_code: _ } => {
                // Agent crash — handle via error recovery
                self.error_recovery
                    .handle_agent_crash(&task_context, worktree.as_ref().unwrap())
                    .await?;
                error_recovery_ran = true;
            }
        }

        // ============================================================
        // CLEANUP PHASE: Stop heartbeat, destroy session, cleanup worktree
        // ============================================================

        // Stop heartbeat background task
        if let Some(handle) = heartbeat_handle.take() {
            handle.abort();
        }

        // Destroy ACP session
        if let Some(sess) = session.take() {
            self.session_manager.destroy_session(sess).await?;
        }

        // Unregister session from event bus
        self.event_bus.unregister_session(&task_id).await;

        // Remove from dispatcher active sessions
        self.dispatcher.lock().await.remove_session(&task_id);

        // Cleanup worktree if error recovery didn't already do it
        // (error_recovery::handle_agent_crash and handle_timeout both clean up)
        if !error_recovery_ran {
            if let Some(wt) = worktree.take() {
                // Only clean up if the worktree directory still exists.
                // The completion handler's post_merge_cleanup handles cleanup
                // on successful merge. If we reach here, it means either
                // a merge conflict occurred or the worktree wasn't cleaned.
                if tokio::fs::try_exists(&wt.path).await.unwrap_or(false) {
                    self.worktree_manager.cleanup(&wt).await?;
                }
            }
        }

        // Emit cleanup event
        self.event_bus
            .emit(BuilderEvent::TaskCleanedUp {
                task_id: task_id.clone(),
            })?;

        tracing::info!(task_id = %task_id, "Cleanup phase complete");

        Ok(())
    }

    /// Claim a task from the dispatcher and execute it.
    ///
    /// Combines task dispatch (queued → running transition) with full
    /// workflow execution in a single call.
    ///
    /// # Returns
    /// * `Ok(())` if the task was dispatched and executed successfully
    /// * `Err(BuilderError::WorkflowError)` if no queued tasks found
    pub async fn dispatch_and_execute(
        &self,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<()> {
        let task_context = self
            .dispatcher
            .lock()
            .await
            .dispatch_task(branch, plan_id, plan_name)
            .await?
            .ok_or_else(|| BuilderError::WorkflowError("No task available for dispatch".into()))?;

        tracing::info!(
            task_id = %task_context.task_id,
            task_name = %task_context.task_name,
            "Dispatched task for execution"
        );

        self.execute_task(task_context).await
    }

    /// Execute all ready tasks up to the concurrency limit.
    ///
    /// Dispatches tasks from the queue until the concurrency limit is
    /// reached or no more tasks are available. Spawns each task as a
    /// concurrent tokio task and waits for all to complete.
    pub async fn execute_all_ready_tasks(
        &self,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<()> {
        // Collect all dispatchable task contexts
        let mut task_contexts = Vec::new();

        loop {
            let ctx = self
                .dispatcher
                .lock()
                .await
                .dispatch_task(branch, plan_id, plan_name)
                .await?;

            match ctx {
                Some(context) => {
                    tracing::info!(
                        task_id = %context.task_id,
                        task_name = %context.task_name,
                        "Queued task for parallel execution"
                    );
                    task_contexts.push(context);
                }
                None => {
                    tracing::info!(
                        branch = %branch,
                        plan_id = %plan_id,
                        "No more tasks available for dispatch"
                    );
                    break;
                }
            }
        }

        // Spawn all tasks concurrently
        let mut handles = Vec::new();
        for ctx in task_contexts {
            let self_clone = Arc::new(self.clone_components());
            let handle = tokio::spawn(async move { self_clone.execute_task(ctx).await });
            handles.push(handle);
        }

        // Wait for all tasks to complete
        for handle in handles {
            handle.await.expect("Task execution panicked")?;
        }

        Ok(())
    }

    /// Clone all Arc-held components for use in spawned tasks.
    fn clone_components(&self) -> Self {
        Self {
            repo_root: self.repo_root.clone(),
            concurrency_limit: self.concurrency_limit,
            task_timeout: self.task_timeout,
            dispatcher: self.dispatcher.clone(),
            worktree_manager: self.worktree_manager.clone(),
            session_manager: self.session_manager.clone(),
            heartbeat_manager: self.heartbeat_manager.clone(),
            event_bus: self.event_bus.clone(),
            merge_coordinator: self.merge_coordinator.clone(),
            error_recovery: self.error_recovery.clone(),
            completion_handler: self.completion_handler.clone(),
        }
    }

    /// Get a list of currently active sessions.
    pub async fn get_active_sessions(&self) -> Vec<ActiveSession> {
        let dispatcher = self.dispatcher.lock().await;
        dispatcher
            .list_active()
            .into_iter()
            .map(|s| s.clone())
            .collect()
    }

    /// Get a reference to the event bus for subscribing to builder events.
    pub fn get_event_bus(&self) -> &Arc<BuilderEventBus> {
        &self.event_bus
    }
}

impl Clone for WorkflowOrchestrator {
    fn clone(&self) -> Self {
        self.clone_components()
    }
}
