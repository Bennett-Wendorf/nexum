//! The main scheduler loop for the Overlord deterministic core.
//!
//! A periodic async loop that runs all deterministic checks: heartbeat
//! monitoring, dependency auto-queue, and concurrency-based dispatch.
//! Configurable interval (default 30 seconds).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::concurrency_checker::{ConcurrencyChecker, DispatchInfo, validate_transition_concurrency};
use super::dependency_resolver::DependencyResolver;
use super::errors::{OverlordError, Result};
use super::heartbeat_monitor::{HeartbeatMonitor, StaleTask};
use super::id_generator::{PlanIdGenerator, TaskIdGenerator};
use super::status_machine::{PlanStateMachine, TaskStateMachine};

use crate::persistence::{list_branches, list_plans, list_tasks};
use crate::persistence::{read_plan, read_task_status, update_task_status, update_plan};
use crate::persistence::{PlanStatus, TaskStatusValue};

/// The main scheduler that ties all deterministic checks together.
pub struct OverlordScheduler {
    task_machine: TaskStateMachine,
    plan_machine: PlanStateMachine,
    plan_id_generator: PlanIdGenerator,
    task_id_generator: TaskIdGenerator,
    concurrency_checker: ConcurrencyChecker,
    dependency_resolver: DependencyResolver,
    heartbeat_monitor: HeartbeatMonitor,
    repo_root: PathBuf,
    interval: Duration,
    running: AtomicBool,
}

impl OverlordScheduler {
    /// Initialize all sub-components with defaults.
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            task_machine: TaskStateMachine::new(),
            plan_machine: PlanStateMachine::new(),
            plan_id_generator: PlanIdGenerator,
            task_id_generator: TaskIdGenerator,
            concurrency_checker: ConcurrencyChecker::new(),
            dependency_resolver: DependencyResolver::new(),
            heartbeat_monitor: HeartbeatMonitor::new(30), // 30 min default
            repo_root,
            interval: Duration::from_secs(30),
            running: AtomicBool::new(false),
        }
    }

    /// Builder pattern for interval.
    pub fn with_interval(mut self, interval: Duration) -> Self {
        self.interval = interval;
        self
    }

    /// Builder pattern for heartbeat threshold.
    pub fn with_heartbeat_threshold(mut self, minutes: u64) -> Self {
        self.heartbeat_monitor = HeartbeatMonitor::new(minutes);
        self
    }

    // ── Lifecycle ─────────────────────────────────────────────────────────

    /// Start the scheduler loop.
    pub async fn start(&self) -> Result<()> {
        self.running.store(true, Ordering::SeqCst);
        tracing::info!("Overlord scheduler started");

        while self.running.load(Ordering::SeqCst) {
            if let Err(e) = self.tick().await {
                tracing::error!("Scheduler tick error: {}", e);
                // Continue on error — don't crash the loop
            }
            tokio::time::sleep(self.interval).await;
        }

        tracing::info!("Overlord scheduler stopped");
        Ok(())
    }

    /// Stop the scheduler loop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Check scheduler state.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    // ── Tick ──────────────────────────────────────────────────────────────

    /// Run all deterministic checks in one iteration.
    async fn tick(&self) -> Result<()> {
        let repo_root = self.repo_root.as_path();

        // 1. Heartbeat recovery
        let branches = list_branches(repo_root)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        for branch in &branches {
            if let Err(e) = self.heartbeat_monitor.recover_stale_tasks(repo_root, branch) {
                tracing::warn!("Heartbeat recovery error in branch {}: {}", branch, e);
            }
        }

        // 2. Dependency auto-queue
        for branch in &branches {
            let plans = list_plans(repo_root, branch)
                .map_err(|e| OverlordError::PersistenceError(e))?;

            for plan_slug in &plans {
                if let Some(pos) = plan_slug.find('-') {
                    let after_first = &plan_slug[pos + 1..];
                    if let Some(second_pos) = after_first.find('-') {
                        let plan_id = &plan_slug[..pos + 1 + second_pos];
                        let plan_name = &plan_slug[pos + 1 + second_pos + 1..];

                        if let Err(e) = self.dependency_resolver.auto_queue_tasks(
                            repo_root, branch, plan_id, plan_name,
                        ) {
                            tracing::warn!("Auto-queue error in plan {}: {}", plan_id, e);
                        }
                    }
                }
            }
        }

        // 3. Dispatch queued tasks
        self.try_dispatch_queued_tasks().await?;

        Ok(())
    }

    /// Dispatch queued tasks respecting concurrency limits.
    async fn try_dispatch_queued_tasks(&self) -> Result<()> {
        let repo_root = self.repo_root.as_path();

        let branches = list_branches(repo_root)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        for branch in &branches {
            let plans = list_plans(repo_root, branch)
                .map_err(|e| OverlordError::PersistenceError(e))?;

            for plan_slug in &plans {
                if let Some(pos) = plan_slug.find('-') {
                    let after_first = &plan_slug[pos + 1..];
                    if let Some(second_pos) = after_first.find('-') {
                        let plan_id = &plan_slug[..pos + 1 + second_pos];
                        let plan_name = &plan_slug[pos + 1 + second_pos + 1..];

                        // Check if we can dispatch
                        if !ConcurrencyChecker::can_dispatch(
                            repo_root, branch, plan_id, plan_name,
                        ).unwrap_or(false) {
                            continue;
                        }

                        // Find queued tasks
                        let tasks = list_tasks(repo_root, branch, plan_id, plan_name)
                            .map_err(|e| OverlordError::PersistenceError(e))?;

                        for task_slug in &tasks {
                            // Re-check concurrency before each dispatch
                            if !ConcurrencyChecker::can_dispatch(
                                repo_root, branch, plan_id, plan_name,
                            ).unwrap_or(false) {
                                break;
                            }

                            if let Some(pos) = task_slug.find('-') {
                                let after_first = &task_slug[pos + 1..];
                                if let Some(second_pos) = after_first.find('-') {
                                    let task_id = &task_slug[..pos + 1 + second_pos];
                                    let task_name = &task_slug[pos + 1 + second_pos + 1..];

                                    let status = read_task_status(
                                        repo_root, branch, plan_id, plan_name,
                                        task_id, task_name,
                                    ).map_err(|e| OverlordError::PersistenceError(e))?;

                                    if matches!(status.status, TaskStatusValue::Queued) {
                                        // Validate transition
                                        if self.task_machine.can_transition(
                                            &TaskStatusValue::Queued,
                                            &TaskStatusValue::Running,
                                        ) {
                                            if let Err(e) = update_task_status(
                                                repo_root, branch, plan_id, plan_name,
                                                task_id, task_name,
                                                TaskStatusValue::Running,
                                                "overlord-dispatch",
                                            ) {
                                                tracing::warn!(
                                                    "Dispatch error for task {}: {}",
                                                    task_id, e
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // ── Public API ────────────────────────────────────────────────────────

    /// Transition a task status with validation and concurrency check.
    pub async fn transition_task_status(
        &self,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
        task_id: &str,
        task_name: &str,
        new_status: TaskStatusValue,
        by: &str,
    ) -> Result<()> {
        let repo_root = self.repo_root.as_path();

        // Read current status
        let status = read_task_status(
            repo_root, branch, plan_id, plan_name, task_id, task_name,
        ).map_err(|e| OverlordError::PersistenceError(e))?;

        // Validate transition
        if !self.task_machine.can_transition(&status.status, &new_status) {
            return Err(OverlordError::InvalidTransition {
                from: status.status.as_str().to_string(),
                to: new_status.as_str().to_string(),
                entity: "task".to_string(),
            });
        }

        // Check concurrency if target is running or reviewing
        if matches!(new_status, TaskStatusValue::Running | TaskStatusValue::Reviewing) {
            validate_transition_concurrency(
                repo_root, branch, plan_id, plan_name, &new_status,
            )?;
        }

        // Perform transition
        update_task_status(
            repo_root, branch, plan_id, plan_name, task_id, task_name,
            new_status, by,
        ).map_err(|e| OverlordError::PersistenceError(e))?;

        Ok(())
    }

    /// Transition a plan status with validation.
    pub async fn transition_plan_status(
        &self,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
        new_status: PlanStatus,
        _by: &str,
    ) -> Result<()> {
        let repo_root = self.repo_root.as_path();

        // Read current plan
        let plan = read_plan(repo_root, branch, plan_id, plan_name)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        // Validate transition
        if !self.plan_machine.can_transition(&plan.status, &new_status) {
            return Err(OverlordError::InvalidTransition {
                from: format!("{:?}", plan.status).to_lowercase(),
                to: format!("{:?}", new_status).to_lowercase(),
                entity: "plan".to_string(),
            });
        }

        // Check concurrency if target is planning or reviewing
        if matches!(new_status, PlanStatus::Planning | PlanStatus::Reviewing) {
            // For now, no concurrency check on plans (future enhancement)
        }

        // Update plan
        let mut updated_plan = plan.clone();
        updated_plan.status = new_status.clone();

        update_plan(
            repo_root, branch, plan_id, plan_name, &updated_plan,
        ).map_err(|e| OverlordError::PersistenceError(e))?;

        // If transitioning to approved, move all tasks to backlog
        if matches!(new_status, PlanStatus::Approved) {
            let tasks = list_tasks(repo_root, branch, plan_id, plan_name)
                .map_err(|e| OverlordError::PersistenceError(e))?;

            for task_slug in &tasks {
                if let Some(pos) = task_slug.find('-') {
                    let after_first = &task_slug[pos + 1..];
                    if let Some(second_pos) = after_first.find('-') {
                        let task_id = &task_slug[..pos + 1 + second_pos];
                        let task_name = &task_slug[pos + 1 + second_pos + 1..];

                        let _ = update_task_status(
                            repo_root, branch, plan_id, plan_name,
                            task_id, task_name,
                            TaskStatusValue::Backlog,
                            "overlord-plan-approved",
                        );
                    }
                }
            }
        }

        Ok(())
    }

    /// Generate next plan ID for a branch.
    pub async fn generate_plan_id(&self, branch: &str) -> Result<String> {
        PlanIdGenerator::next_plan_id(self.repo_root.as_path(), branch)
    }

    /// Generate next task ID for a plan.
    pub async fn generate_task_id(
        &self,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<String> {
        TaskIdGenerator::next_task_id(
            self.repo_root.as_path(),
            branch,
            plan_id,
            plan_name,
        )
    }

    /// Get concurrency dispatch info for a plan.
    pub async fn get_dispatch_info(
        &self,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<DispatchInfo> {
        ConcurrencyChecker::dispatch_info(
            self.repo_root.as_path(),
            branch,
            plan_id,
            plan_name,
        )
    }

    /// Get stale task list for a branch.
    pub async fn get_stale_tasks(&self, branch: &str) -> Result<Vec<StaleTask>> {
        self.heartbeat_monitor
            .detect_stale_tasks(self.repo_root.as_path(), branch)
    }
}

// Helper trait for converting enums to string
trait AsStr {
    fn as_str(&self) -> &str;
}

impl AsStr for TaskStatusValue {
    fn as_str(&self) -> &str {
        match self {
            TaskStatusValue::Backlog => "backlog",
            TaskStatusValue::Queued => "queued",
            TaskStatusValue::Running => "running",
            TaskStatusValue::Reviewing => "reviewing",
            TaskStatusValue::WaitingManualReview => "waiting-manual-review",
            TaskStatusValue::MergeQueue => "merge-queue",
            TaskStatusValue::Abandoned => "abandoned",
            TaskStatusValue::Completed => "completed",
        }
    }
}
