//! Event relay for builder workflow.
//!
//! This module collects ACP events from all active sessions and broadcasts
//! them to the central event bus for WebSocket frontend relay.
//!
//! Event flow: ACP session → session event stream → BuilderEventBus → central event bus → WebSocket → frontend
//!
//! See `design/agent-harness-integration.md` event relay section.

use std::collections::HashMap;

use tokio::sync::broadcast;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::acp::events::ACPEvent;
use crate::builder::dispatcher::TaskContext;
use crate::builder::errors::{BuilderError, Result};

/// Builder lifecycle events broadcast to subscribers.
#[derive(Debug, Clone)]
pub enum BuilderEvent {
    /// A task has started execution.
    TaskStarted(TaskContext),
    /// Progress event from a running task.
    TaskProgress { task_id: String, event: ACPEvent },
    /// A task has completed (success, timeout, or crash).
    TaskCompleted { task_id: String, result: CompletionResult },
    /// A task has failed with an error.
    TaskFailed { task_id: String, error: String },
    /// A task branch has been merged into the plan branch.
    TaskMerged { task_id: String },
    /// A task's worktree and resources have been cleaned up.
    TaskCleanedUp { task_id: String },
    /// A task has been re-queued after crash or timeout recovery.
    TaskRequeued { task_id: String },
}

/// Result of task completion detection.
#[derive(Debug, Clone)]
pub enum CompletionResult {
    /// Task completed successfully.
    Completed,
    /// Task exceeded its timeout.
    Timeout(std::time::Duration),
    /// Agent subprocess crashed.
    Crashed { exit_code: Option<i32> },
}

/// Event bus for builder workflow events.
///
/// Uses `tokio::sync::broadcast` for many-to-many event distribution.
/// Events flow from ACP sessions through this bus to frontend subscribers.
/// Per-session broadcasters are protected by a `tokio::sync::Mutex` to allow
/// shared ownership via `Arc` while still supporting mutable operations.
pub struct BuilderEventBus {
    /// Broadcaster for builder lifecycle events.
    tx: broadcast::Sender<BuilderEvent>,
    /// Per-session event broadcasters (protected by Mutex for shared ownership).
    active_sessions: Mutex<HashMap<String, broadcast::Sender<ACPEvent>>>,
}

impl BuilderEventBus {
    /// Create a new event bus with the given broadcast channel capacity.
    pub fn new(capacity: usize) -> Self {
        let (tx, _) = broadcast::channel(capacity);
        Self {
            tx,
            active_sessions: Mutex::new(HashMap::new()),
        }
    }

    /// Subscribe to builder lifecycle events.
    pub fn subscribe(&self) -> broadcast::Receiver<BuilderEvent> {
        self.tx.subscribe()
    }

    /// Emit a builder lifecycle event to all subscribers.
    pub fn emit(&self, event: BuilderEvent) -> Result<()> {
        self.tx.send(event)
            .map(|_| ())
            .map_err(|e| {
                BuilderError::WorkflowError(format!("Failed to emit event: {}", e))
            })
    }

    /// Relay ACP events from a session to the central builder event bus.
    ///
    /// Spawns a background task that:
    /// 1. Receives ACP events from the session
    /// 2. Wraps each in BuilderEvent::TaskProgress
    /// 3. Broadcasts to central bus
    /// 4. Stops when session event stream ends
    pub async fn relay_session_events(
        &self,
        task_id: &str,
        mut event_rx: broadcast::Receiver<ACPEvent>,
    ) -> JoinHandle<()> {
        let tx = self.tx.clone();
        let task_id = task_id.to_string();

        tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        // Log the event
                        crate::acp::events::log_event(&event);

                        // Wrap in builder event and broadcast
                        let builder_event = BuilderEvent::TaskProgress {
                            task_id: task_id.clone(),
                            event,
                        };
                        let _ = tx.send(builder_event);
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(
                            task_id = %task_id,
                            lagged = n,
                            "Event relay lagged, skipping missed events"
                        );
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!(task_id = %task_id, "Event relay stopped: session closed");
                        break;
                    }
                }
            }
        })
    }

    /// Register a session's event stream broadcaster.
    ///
    /// Uses interior mutability via `tokio::sync::Mutex`, so this takes `&self`
    /// and can be called from behind an `Arc`.
    pub async fn register_session(&self, task_id: &str, tx: broadcast::Sender<ACPEvent>) {
        self.active_sessions
            .lock()
            .await
            .insert(task_id.to_string(), tx);
    }

    /// Unregister a session's event stream.
    ///
    /// Uses interior mutability via `tokio::sync::Mutex`, so this takes `&self`
    /// and can be called from behind an `Arc`.
    pub async fn unregister_session(&self, task_id: &str) {
        self.active_sessions.lock().await.remove(task_id);
    }

    /// Get a subscriber for a specific session's events.
    pub async fn subscribe_session(&self, task_id: &str) -> Option<broadcast::Receiver<ACPEvent>> {
        self.active_sessions
            .lock()
            .await
            .get(task_id)
            .map(|tx| tx.subscribe())
    }
}
