//! Stale heartbeat detection and orphaned task recovery.
//!
//! Scans all tasks in `running` status, checks `heartbeat_at` in `status.json`
//! against the current time. If elapsed time exceeds the configured threshold
//! (default 30 minutes), transitions the task back to `queued` and increments
//! `attempts`.

use std::path::Path;

use chrono::{DateTime, Utc};

use crate::persistence::TaskStatusValue;

use super::errors::{OverlordError, Result};
use super::status_machine::{TaskStateMachine, task_status_to_string};

/// A task with a stale heartbeat.
#[derive(Debug, Clone)]
pub struct StaleTask {
    /// Task identifier.
    pub task_id: String,
    /// Plan identifier.
    pub plan_id: String,
    /// Branch name.
    pub branch: String,
    /// Plan name.
    pub plan_name: String,
    /// Task name.
    pub task_name: String,
    /// Last heartbeat timestamp.
    pub last_heartbeat: String,
    /// Minutes elapsed since last heartbeat.
    pub elapsed_minutes: f64,
}

/// Stale heartbeat detection struct.
pub struct HeartbeatMonitor {
    stale_threshold_minutes: u64,
    task_machine: TaskStateMachine,
}

impl HeartbeatMonitor {
    /// Initialize with configurable stale threshold (default: 30 minutes).
    pub fn new(stale_threshold_minutes: u64) -> Self {
        Self {
            stale_threshold_minutes,
            task_machine: TaskStateMachine::new(),
        }
    }

    /// Returns the stale threshold as a Duration.
    pub fn get_stale_threshold(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.stale_threshold_minutes * 60)
    }

    /// Check if a heartbeat timestamp is stale.
    ///
    /// Parses `heartbeat_at` as RFC 3339 timestamp and computes elapsed time.
    /// Returns `(is_stale, elapsed_minutes)`.
    pub fn is_heartbeat_stale(&self, heartbeat_at: &str) -> Result<(bool, f64)> {
        let dt: DateTime<Utc> = chrono::DateTime::parse_from_rfc3339(heartbeat_at)
            .map_err(|e| OverlordError::HeartbeatError(format!("Invalid timestamp: {}", e)))?
            .into();

        let elapsed = Utc::now().signed_duration_since(dt);
        let elapsed_minutes = elapsed.num_minutes() as f64;

        Ok((elapsed_minutes > self.stale_threshold_minutes as f64, elapsed_minutes))
    }

    /// Scan all plans and tasks for stale heartbeats across a branch.
    pub fn detect_stale_tasks(&self, repo_root: &Path, branch: &str) -> Result<Vec<StaleTask>> {
        let mut stale_tasks = Vec::new();

        // List all plans in this branch
        let plan_slugs = crate::persistence::list_plans(repo_root, branch)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        for plan_slug in &plan_slugs {
            let (plan_id, plan_name) = Self::parse_plan_slug(plan_slug);
            let plan_id = match plan_id {
                Some(id) => id,
                None => continue,
            };
            let plan_name = match plan_name {
                Some(name) => name,
                None => continue,
            };
            let stale = self.detect_stale_in_plan(repo_root, branch, plan_id, plan_name)?;
            stale_tasks.extend(stale);
        }

        Ok(stale_tasks)
    }

    /// Scan a single plan for stale tasks.
    pub fn detect_stale_in_plan(
        &self,
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<Vec<StaleTask>> {
        let mut stale_tasks = Vec::new();

        let task_slugs = crate::persistence::list_tasks(repo_root, branch, plan_id, plan_name)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        for slug in &task_slugs {
            let (task_id, task_name) = Self::parse_task_slug(slug);
            let task_id = match task_id {
                Some(id) => id,
                None => continue,
            };
            let task_name = match task_name {
                Some(name) => name,
                None => continue,
            };

            let status = crate::persistence::read_task_status(
                repo_root,
                branch,
                plan_id,
                plan_name,
                task_id,
                task_name,
            ).map_err(|e| OverlordError::PersistenceError(e))?;

            // Only check running tasks
            if !matches!(status.status, TaskStatusValue::Running) {
                continue;
            }

            // Check heartbeat
            let (is_stale, elapsed_minutes) = if let Some(heartbeat_at) = &status.heartbeat_at {
                self.is_heartbeat_stale(heartbeat_at)?
            } else {
                // Missing heartbeat treated as stale if in running status
                (true, f64::MAX)
            };

            if is_stale {
                let last_heartbeat = status.heartbeat_at.clone().unwrap_or_default();

                stale_tasks.push(StaleTask {
                    task_id: task_id.to_string(),
                    plan_id: plan_id.to_string(),
                    branch: branch.to_string(),
                    plan_name: plan_name.to_string(),
                    task_name: task_name.to_string(),
                    last_heartbeat,
                    elapsed_minutes,
                });
            }
        }

        Ok(stale_tasks)
    }

    /// Re-queue stale tasks in a branch.
    ///
    /// Transitions stale tasks from `running` to `queued`, increments `attempts`,
    /// clears `agent` lease and `started_at`. Actor: `"overlord-heartbeat-recovery"`.
    pub fn recover_stale_tasks(&self, repo_root: &Path, branch: &str) -> Result<Vec<String>> {
        let stale = self.detect_stale_tasks(repo_root, branch)?;
        let mut recovered = Vec::new();

        for task in &stale {
            self.recover_single_task(
                repo_root,
                &task.branch,
                &task.plan_id,
                &task.plan_name,
                &task.task_id,
                &task.task_name,
            )?;
            recovered.push(task.task_id.clone());
        }

        Ok(recovered)
    }

    /// Recover stale tasks in a single plan.
    pub fn recover_stale_in_plan(
        &self,
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<Vec<String>> {
        let stale = self.detect_stale_in_plan(repo_root, branch, plan_id, plan_name)?;
        let mut recovered = Vec::new();

        for task in &stale {
            self.recover_single_task(
                repo_root,
                branch,
                plan_id,
                plan_name,
                &task.task_id,
                &task.task_name,
            )?;
            recovered.push(task.task_id.clone());
        }

        Ok(recovered)
    }

    /// Recover a single stale task.
    fn recover_single_task(
        &self,
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
        task_id: &str,
        task_name: &str,
    ) -> Result<()> {
        // Read current status
        let mut status = crate::persistence::read_task_status(
            repo_root,
            branch,
            plan_id,
            plan_name,
            task_id,
            task_name,
        ).map_err(|e| OverlordError::PersistenceError(e))?;

        // Increment attempts
        status.attempts += 1;

        // Clear agent lease
        status.agent = None;

        // Clear started_at
        status.started_at = None;

        // Clear heartbeat
        status.heartbeat_at = None;

        // Record transition
        let now = Utc::now().to_rfc3339();
        status.transitions.push(crate::persistence::StatusTransition {
            from: task_status_to_string(&status.status).to_string(),
            to: "queued".to_string(),
            at: now,
            by: "overlord-heartbeat-recovery".to_string(),
        });

        // Update status
        status.status = TaskStatusValue::Queued;

        // Write atomically via persistence layer
        // We need to write the status directly since this is a recovery action
        let status_path = crate::persistence::task_status_path(
            repo_root, branch, plan_id, plan_name, task_id, task_name,
        );
        crate::persistence::atomic_write_json(&status_path, &status)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        Ok(())
    }

    /// Parse plan ID and name from a slug like "PLAN-001-my-plan".
    fn parse_plan_slug(slug: &str) -> (Option<&str>, Option<&str>) {
        if let Some(first_hyphen) = slug.find('-') {
            let after_first = &slug[first_hyphen + 1..];
            if let Some(second_hyphen) = after_first.find('-') {
                let plan_id = &slug[..first_hyphen + 1 + second_hyphen];
                let plan_name = &slug[first_hyphen + 1 + second_hyphen + 1..];
                (Some(plan_id), Some(plan_name))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    }

    /// Parse task ID and name from a slug like "TASK-001-my-task".
    fn parse_task_slug(slug: &str) -> (Option<&str>, Option<&str>) {
        if let Some(first_hyphen) = slug.find('-') {
            let after_first = &slug[first_hyphen + 1..];
            if let Some(second_hyphen) = after_first.find('-') {
                let task_id = &slug[..first_hyphen + 1 + second_hyphen];
                let task_name = &slug[first_hyphen + 1 + second_hyphen + 1..];
                (Some(task_id), Some(task_name))
            } else {
                (None, None)
            }
        } else {
            (None, None)
        }
    }
}
