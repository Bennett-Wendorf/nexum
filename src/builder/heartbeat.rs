//! Heartbeat management for builder tasks.
//!
//! This module periodically updates the `heartbeat_at` timestamp in `status.json`
//! based on ACP progress events and configurable intervals. The Overlord's
//! `HeartbeatMonitor` uses these timestamps for stale detection and orphaned
//! task recovery.
//!
//! See `design/persistence.md` heartbeat section for the full design.

use std::path::PathBuf;
use std::time::Duration;

use chrono::Utc;
use tokio::task::JoinHandle;

use crate::acp::events::ACPEvent;
use crate::builder::dispatcher::TaskContext;
use crate::builder::errors::{BuilderError, Result};
use crate::persistence::{atomic_write_json, read_json, task_status_path, TaskStatus};

/// Manages heartbeat updates for running tasks.
///
/// Updates `heartbeat_at` in `status.json` on progress events and at
/// configurable periodic intervals. The Overlord's `HeartbeatMonitor`
/// detects stale heartbeats and re-queues orphaned tasks.
pub struct HeartbeatManager {
    /// Repository root path.
    repo_root: PathBuf,
    /// Interval for periodic heartbeat updates (default: 5 minutes).
    interval: Duration,
}

impl HeartbeatManager {
    /// Create a new heartbeat manager with the given interval.
    ///
    /// Default interval is 5 minutes if not specified.
    pub fn new(repo_root: PathBuf, interval: Duration) -> Self {
        Self {
            repo_root,
            interval,
        }
    }

    /// Create with default 5-minute interval.
    pub fn with_default_interval(repo_root: PathBuf) -> Self {
        Self::new(repo_root, Duration::from_secs(300))
    }

    /// Update the heartbeat timestamp for a task.
    ///
    /// # Steps
    /// 1. Read `status.json` from the task directory
    /// 2. Set `heartbeat_at` to current UTC timestamp (RFC 3339)
    /// 3. Atomically write via temp + rename
    pub async fn update_heartbeat(
        &self,
        task_id: &str,
        task_name: &str,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<()> {
        let status_path = task_status_path(
            &self.repo_root,
            branch,
            plan_id,
            plan_name,
            task_id,
            task_name,
        );

        // Read current status
        let mut status: TaskStatus = match read_json(&status_path) {
            Ok(s) => s,
            Err(e) => {
                return Err(BuilderError::PersistenceError(e));
            }
        };

        // Update heartbeat timestamp
        status.heartbeat_at = Some(Utc::now().to_rfc3339());

        // Atomic write
        atomic_write_json(&status_path, &status).map_err(BuilderError::PersistenceError)?;

        tracing::debug!(
            "Heartbeat updated for task {} at {}",
            task_id,
            status.heartbeat_at.as_deref().unwrap_or("unknown")
        );

        Ok(())
    }

    /// Start a background task that periodically updates the heartbeat.
    ///
    /// The background task loops: update heartbeat, sleep for interval.
    /// It stops when the returned JoinHandle is aborted.
    pub async fn start_periodic_heartbeat(
        &self,
        task_context: &TaskContext,
    ) -> Result<JoinHandle<()>> {
        let repo_root = self.repo_root.clone();
        let interval = self.interval;
        let task_id = task_context.task_id.clone();
        let task_name = task_context.task_name.clone();
        let branch = task_context.branch.clone();
        let plan_id = task_context.plan_id.clone();
        let plan_name = task_context.plan_name.clone();

        let handle = tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            interval_timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                interval_timer.tick().await;

                let status_path = task_status_path(
                    &repo_root, &branch, &plan_id, &plan_name, &task_id, &task_name,
                );

                let mut status: TaskStatus = match read_json(&status_path) {
                    Ok(s) => s,
                    Err(e) => {
                        tracing::warn!("Failed to read status for heartbeat: {}", e);
                        continue;
                    }
                };

                status.heartbeat_at = Some(Utc::now().to_rfc3339());

                if let Err(e) = atomic_write_json(&status_path, &status) {
                    tracing::warn!("Failed to write heartbeat: {}", e);
                }
            }
        });

        Ok(handle)
    }

    /// Update heartbeat on ACP progress events.
    ///
    /// Triggers heartbeat for progress events (tool_call, tool_result, progress,
    /// question, permission_request). Does NOT trigger for completion events.
    pub async fn update_on_event(
        &self,
        event: &ACPEvent,
        task_context: &TaskContext,
    ) -> Result<()> {
        if !Self::is_progress_event(event) {
            return Ok(());
        }

        self.update_heartbeat(
            &task_context.task_id,
            &task_context.task_name,
            &task_context.branch,
            &task_context.plan_id,
            &task_context.plan_name,
        )
        .await
    }

    /// Check if an ACP event should trigger a heartbeat update.
    ///
    /// Progress events: Progress, ToolCall, ToolResult, Question, PermissionRequest
    /// Non-progress events: Completion, Error
    pub fn is_progress_event(event: &ACPEvent) -> bool {
        matches!(
            event,
            ACPEvent::Progress { .. }
                | ACPEvent::ToolCall { .. }
                | ACPEvent::ToolResult { .. }
                | ACPEvent::Question { .. }
                | ACPEvent::PermissionRequest { .. }
        )
    }
}
