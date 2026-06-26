//! Task dispatch logic for the builder workflow.
//!
//! This module selects queued tasks respecting concurrency limits and
//! dependency constraints. It integrates with the Overlord's concurrency
//! checker and manages the pool of active builder sessions.
//!
//! See `design/resource-constraints.md` for concurrency enforcement rules.

use std::collections::HashMap;
use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::builder::errors::{BuilderError, Result};
use crate::builder::worktree_manager::TaskWorktree;
use crate::overlord::{ConcurrencyChecker, OverlordScheduler, TransitionTaskStatusParams};
use crate::persistence::{parse_slug, read_execution_state, read_task_status, TaskStatusValue};

/// Context about a task ready for dispatch.
#[derive(Debug, Clone)]
pub struct TaskContext {
    /// Task identifier, e.g. `"TASK-001"`.
    pub task_id: String,
    /// Short task name, e.g. `"add-auth-flow"`.
    pub task_name: String,
    /// Plan identifier, e.g. `"PLAN-001"`.
    pub plan_id: String,
    /// Plan name, e.g. `"auth-overhaul"`.
    pub plan_name: String,
    /// Git branch for the plan.
    pub branch: String,
    /// Path to the task directory under `.agent/specs/`.
    pub task_dir: PathBuf,
    /// The full task prompt extracted from task.md.
    pub task_prompt: String,
}

/// Information about an active builder session.
#[derive(Debug, Clone)]
pub struct ActiveSession {
    /// The task context for this session.
    pub task_context: TaskContext,
    /// The worktree assigned to this task.
    pub worktree: TaskWorktree,
    /// ACP session identifier.
    pub session_id: String,
    /// Process ID of the agent subprocess.
    pub pid: Option<u32>,
    /// Timestamp when the session started.
    pub started_at: DateTime<Utc>,
}

/// Task dispatch logic.
///
/// Selects queued tasks respecting concurrency limits and dependency
/// constraints. Integrates with the Overlord for status transitions.
pub struct TaskDispatcher {
    /// Repository root path.
    repo_root: PathBuf,
    /// Reference to the Overlord scheduler.
    overlord: std::sync::Arc<OverlordScheduler>,
    /// Active session tracking.
    active_sessions: HashMap<String, ActiveSession>,
}

impl TaskDispatcher {
    /// Create a new task dispatcher.
    pub fn new(repo_root: PathBuf, overlord: std::sync::Arc<OverlordScheduler>) -> Self {
        Self {
            repo_root,
            overlord,
            active_sessions: HashMap::new(),
        }
    }

    /// Find the next dispatchable task.
    ///
    /// # Steps
    /// 1. Read `execution.json` to get task list and status map
    /// 2. Filter tasks with status `queued`
    /// 3. For each queued task, check `ConcurrencyChecker::can_dispatch()`
    /// 4. If no dispatchable tasks, return `None`
    /// 5. Return `TaskContext` for the first eligible task
    pub async fn find_next_task(
        &self,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<Option<TaskContext>> {
        let repo_root = self.repo_root.as_path();

        // Read execution state
        let exec_state = match read_execution_state(repo_root, branch, plan_id, plan_name) {
            Ok(state) => state,
            Err(e) => {
                tracing::debug!("No execution state found: {}", e);
                return Ok(None);
            }
        };

        // Find queued tasks
        for task_id in &exec_state.tasks {
            // Check status map first (fast path)
            if let Some(status) = exec_state.task_status_map.get(task_id) {
                if !matches!(status, TaskStatusValue::Queued) {
                    continue;
                }
            }

            // List tasks to get task_name from slug
            let tasks = match crate::persistence::list_tasks(repo_root, branch, plan_id, plan_name) {
                Ok(tasks) => tasks,
                Err(e) => {
                    tracing::debug!("Failed to list tasks: {}", e);
                    continue;
                }
            };

            let task_name: String = 'found: {
                for slug in &tasks {
                    if let (Some(tid), Some(tname)) = parse_slug(slug) {
                        if tid == *task_id {
                            break 'found tname.to_string();
                        }
                    }
                }
                // Try to find task name from directory listing
                let plan_dir = crate::persistence::plan_dir(repo_root, branch, plan_id, plan_name);
                let tasks_dir = plan_dir.join("tasks");
                if let Ok(mut entries) = tokio::fs::read_dir(&tasks_dir).await {
                    while let Ok(Some(entry)) = entries.next_entry().await {
                        let name_str = entry.file_name().to_string_lossy().into_owned();
                        let parts: Vec<&str> = name_str.splitn(2, '-').collect();
                        if parts.len() == 2 && parts[0] == task_id {
                            break 'found parts[1].to_string();
                        }
                    }
                }
                // Fallback
                break 'found String::from("unknown");
            };

            // Verify status by reading status.json
            let status = match read_task_status(repo_root, branch, plan_id, plan_name, task_id, &task_name) {
                Ok(s) => s,
                Err(_) => continue,
            };

            if !matches!(status.status, TaskStatusValue::Queued) {
                continue;
            }

            // Check concurrency limit
            match ConcurrencyChecker::can_dispatch(repo_root, branch, plan_id, plan_name) {
                Ok(can) => {
                    if !can {
                        tracing::debug!("Concurrency limit reached, cannot dispatch {}", task_id);
                        return Ok(None);
                    }
                }
                Err(e) => {
                    tracing::warn!("Concurrency check failed: {}", e);
                    continue;
                }
            }

            // Build task context
            let task_dir = crate::persistence::task_dir(
                repo_root,
                branch,
                plan_id,
                plan_name,
                task_id,
                &task_name,
            );

            // Read task prompt from task.md
            let task_prompt = match crate::persistence::read_task(
                repo_root,
                branch,
                plan_id,
                plan_name,
                task_id,
                &task_name,
            ) {
                Ok(task) => {
                    format!(
                        "## Task: {}\n\n### Description\n{}\n\n### Acceptance Criteria\n{}\n\n### Files to Modify\n{}\n\n### Notes\n{}",
                        task.name,
                        task.description,
                        task.acceptance_criteria.join("\n"),
                        task.files_to_modify.join("\n"),
                        task.notes
                    )
                }
                Err(e) => {
                    tracing::warn!("Failed to read task {}: {}", task_id, e);
                    continue;
                }
            };

            let task_context = TaskContext {
                task_id: task_id.clone(),
                task_name,
                plan_id: plan_id.to_string(),
                plan_name: plan_name.to_string(),
                branch: branch.to_string(),
                task_dir,
                task_prompt,
            };

            return Ok(Some(task_context));
        }

        Ok(None)
    }

    /// Dispatch the next available task.
    ///
    /// # Steps
    /// 1. Find next task via `find_next_task()`
    /// 2. If found, transition `queued` → `running` via overlord
    /// 3. Actor: `"overlord-dispatch"`
    /// 4. Return the task context
    pub async fn dispatch_task(
        &self,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<Option<TaskContext>> {
        let task_context = match self.find_next_task(branch, plan_id, plan_name).await? {
            Some(ctx) => ctx,
            None => return Ok(None),
        };

        // Transition queued → running
        self.overlord
            .transition_task_status(&TransitionTaskStatusParams {
                branch: branch.to_string(),
                plan_id: plan_id.to_string(),
                plan_name: plan_name.to_string(),
                task_id: task_context.task_id.clone(),
                task_name: task_context.task_name.clone(),
                new_status: TaskStatusValue::Running,
                by: "overlord-dispatch".to_string(),
            })
            .await
            .map_err(BuilderError::OverlordError)?;

        tracing::info!(
            "Dispatched task {} ({}) from plan {} on branch {}",
            task_context.task_id,
            task_context.task_name,
            plan_id,
            branch
        );

        Ok(Some(task_context))
    }

    /// Track an active session.
    pub fn add_session(&mut self, session: ActiveSession) {
        let task_id = session.task_context.task_id.clone();
        self.active_sessions.insert(task_id, session);
    }

    /// Remove a completed session.
    pub fn remove_session(&mut self, task_id: &str) -> Option<ActiveSession> {
        self.active_sessions.remove(task_id)
    }

    /// Count active sessions.
    pub fn get_active_count(&self) -> usize {
        self.active_sessions.len()
    }

    /// Get session by task ID.
    pub fn get_session(&self, task_id: &str) -> Option<&ActiveSession> {
        self.active_sessions.get(task_id)
    }

    /// List all active sessions.
    pub fn list_active(&self) -> Vec<&ActiveSession> {
        self.active_sessions.values().collect()
    }
}
