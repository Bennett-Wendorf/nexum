//! The main scheduler loop for the Overlord deterministic core.
//!
//! A periodic async loop that runs all deterministic checks: heartbeat
//! monitoring, dependency auto-queue, and concurrency-based dispatch.
//! Configurable interval (default 30 seconds).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use super::concurrency_checker::{
    validate_transition_concurrency, ConcurrencyChecker, DispatchInfo,
};
use super::dependency_resolver::DependencyResolver;
use super::errors::{OverlordError, Result};
use super::heartbeat_monitor::{HeartbeatMonitor, StaleTask};
use super::id_generator::{PlanIdGenerator, TaskIdGenerator};
use super::status_machine::{PlanStateMachine, TaskStateMachine};

use crate::persistence::{list_branches, list_plans, list_tasks, parse_slug};
use crate::persistence::{read_plan, read_task_status, update_plan, update_task_status};
use crate::persistence::{
    Plan, PlanStatus, TaskPathParams, TaskStatusValue, UpdateTaskStatusParams,
};

/// Parameters for transitioning a task status.
#[derive(Debug, Clone)]
pub struct TransitionTaskStatusParams {
    pub branch: String,
    pub plan_id: String,
    pub plan_name: String,
    pub task_id: String,
    pub task_name: String,
    pub new_status: TaskStatusValue,
    pub by: String,
}

/// The main scheduler that ties all deterministic checks together.
pub struct OverlordScheduler {
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
            // Check running flag during sleep to allow prompt shutdown
            tokio::select! {
                _ = tokio::time::sleep(self.interval) => {},
                _ = self.check_shutdown() => break,
            }
        }

        tracing::info!("Overlord scheduler stopped");
        Ok(())
    }

    /// Stop the scheduler loop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Poll the running flag with a short interval for responsive shutdown.
    async fn check_shutdown(&self) {
        loop {
            if !self.running.load(Ordering::SeqCst) {
                return;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Check scheduler state.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    // ── Tick ──────────────────────────────────────────────────────────────

    /// Run all deterministic checks in one iteration.
    async fn tick(&self) -> Result<()> {
        let repo_root = self.repo_root.as_path();

        // Compute branches once and reuse
        let branches = list_branches(repo_root).map_err(OverlordError::PersistenceError)?;

        // 1. Heartbeat recovery
        for branch in &branches {
            if let Err(e) = self
                .heartbeat_monitor
                .recover_stale_tasks(repo_root, branch)
            {
                tracing::warn!("Heartbeat recovery error in branch {}: {}", branch, e);
            }
        }

        // 2. Dependency auto-queue
        for branch in &branches {
            let plans = list_plans(repo_root, branch).map_err(OverlordError::PersistenceError)?;

            for plan_slug in &plans {
                let (Some(plan_id), Some(plan_name)) = parse_slug(plan_slug) else {
                    continue;
                };

                if let Err(e) = self
                    .dependency_resolver
                    .auto_queue_tasks(repo_root, branch, plan_id, plan_name)
                {
                    tracing::warn!("Auto-queue error in plan {}: {}", plan_id, e);
                }
            }
        }

        // 3. Dispatch queued tasks
        self.try_dispatch_queued_tasks(&branches).await?;

        Ok(())
    }

    /// Dispatch queued tasks respecting concurrency limits.
    async fn try_dispatch_queued_tasks(&self, branches: &[String]) -> Result<()> {
        let repo_root = self.repo_root.as_path();

        for branch in branches {
            let plans = list_plans(repo_root, branch).map_err(OverlordError::PersistenceError)?;

            for plan_slug in &plans {
                let (Some(plan_id), Some(plan_name)) = parse_slug(plan_slug) else {
                    continue;
                };

                // Find queued tasks
                let tasks = list_tasks(repo_root, branch, plan_id, plan_name)
                    .map_err(OverlordError::PersistenceError)?;

                for task_slug in &tasks {
                    // Re-check concurrency before each dispatch
                    match ConcurrencyChecker::can_dispatch(repo_root, branch, plan_id, plan_name) {
                        Ok(can) => {
                            if !can {
                                break;
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Concurrency check failed for plan {}: {}", plan_id, e);
                            break;
                        }
                    }

                    let (Some(task_id), Some(task_name)) = parse_slug(task_slug) else {
                        continue;
                    };

                    let status =
                        read_task_status(repo_root, branch, plan_id, plan_name, task_id, task_name)
                            .map_err(OverlordError::PersistenceError)?;

                    if matches!(status.status, TaskStatusValue::Queued) {
                        if let Err(e) = update_task_status(&UpdateTaskStatusParams {
                            path: TaskPathParams {
                                repo_root: repo_root.to_path_buf(),
                                branch: branch.to_string(),
                                plan_id: plan_id.to_string(),
                                plan_name: plan_name.to_string(),
                                task_id: task_id.to_string(),
                                task_name: task_name.to_string(),
                            },
                            new_status: TaskStatusValue::Running,
                            by: "overlord-dispatch".to_string(),
                        }) {
                            tracing::warn!("Dispatch error for task {}: {}", task_id, e);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // ── Public API ────────────────────────────────────────────────────────

    /// Transition a task status with validation and concurrency check.
    pub async fn transition_task_status(&self, params: &TransitionTaskStatusParams) -> Result<()> {
        let repo_root = self.repo_root.as_path();

        // Read current status
        let status = read_task_status(
            repo_root,
            &params.branch,
            &params.plan_id,
            &params.plan_name,
            &params.task_id,
            &params.task_name,
        )
        .map_err(OverlordError::PersistenceError)?;

        // Validate transition
        if !TaskStateMachine::can_transition(&status.status, &params.new_status) {
            return Err(OverlordError::InvalidTransition {
                from: status.status.to_string(),
                to: params.new_status.to_string(),
                entity: "task".to_string(),
            });
        }

        // Check concurrency if target is running or reviewing
        if matches!(
            params.new_status,
            TaskStatusValue::Running | TaskStatusValue::Reviewing
        ) {
            validate_transition_concurrency(
                repo_root,
                &params.branch,
                &params.plan_id,
                &params.plan_name,
                &params.new_status,
            )?;
        }

        // Perform transition
        update_task_status(&UpdateTaskStatusParams {
            path: TaskPathParams {
                repo_root: repo_root.to_path_buf(),
                branch: params.branch.clone(),
                plan_id: params.plan_id.clone(),
                plan_name: params.plan_name.clone(),
                task_id: params.task_id.clone(),
                task_name: params.task_name.clone(),
            },
            new_status: params.new_status.clone(),
            by: params.by.clone(),
        })
        .map_err(OverlordError::PersistenceError)?;

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
            .map_err(OverlordError::PersistenceError)?;

        // Validate transition
        if !PlanStateMachine::can_transition(&plan.status, &new_status) {
            return Err(OverlordError::InvalidTransition {
                from: plan.status.to_string(),
                to: new_status.to_string(),
                entity: "plan".to_string(),
            });
        }

        // Update plan status using struct update syntax
        let updated_plan = Plan {
            status: new_status.clone(),
            ..plan
        };

        update_plan(repo_root, branch, plan_id, plan_name, &updated_plan)
            .map_err(OverlordError::PersistenceError)?;

        // If transitioning to approved, move all tasks to backlog
        if matches!(new_status, PlanStatus::Approved) {
            let tasks = list_tasks(repo_root, branch, plan_id, plan_name)
                .map_err(OverlordError::PersistenceError)?;

            for task_slug in &tasks {
                let (Some(task_id), Some(task_name)) = parse_slug(task_slug) else {
                    continue;
                };

                let _ = update_task_status(&UpdateTaskStatusParams {
                    path: TaskPathParams {
                        repo_root: repo_root.to_path_buf(),
                        branch: branch.to_string(),
                        plan_id: plan_id.to_string(),
                        plan_name: plan_name.to_string(),
                        task_id: task_id.to_string(),
                        task_name: task_name.to_string(),
                    },
                    new_status: TaskStatusValue::Backlog,
                    by: "overlord-plan-approved".to_string(),
                });
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
        TaskIdGenerator::next_task_id(self.repo_root.as_path(), branch, plan_id, plan_name)
    }

    /// Get concurrency dispatch info for a plan.
    pub async fn get_dispatch_info(
        &self,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<DispatchInfo> {
        ConcurrencyChecker::dispatch_info(self.repo_root.as_path(), branch, plan_id, plan_name)
    }

    /// Get stale task list for a branch.
    pub async fn get_stale_tasks(&self, branch: &str) -> Result<Vec<StaleTask>> {
        self.heartbeat_monitor
            .detect_stale_tasks(self.repo_root.as_path(), branch)
    }
}
