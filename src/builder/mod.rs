//! Builder Workflow module.
//!
//! The end-to-end orchestration pipeline that assigns a queued task to a
//! builder agent, spawns an isolated worktree, manages the ACP session
//! lifecycle, executes the task, detects completion, merges the task branch
//! into the plan branch, and cleans up all resources.
//!
//! See `design/agent-harness-integration.md` and `design/agent-roles.md`.

pub mod completion_handler;
pub mod dispatcher;
pub mod error_recovery;
pub mod errors;
pub mod event_bus;
pub mod heartbeat;
pub mod merge_coordinator;
pub mod orchestrator;
pub mod session_manager;
pub mod worktree_manager;

#[cfg(test)]
mod tests;

// Re-export public types
#[allow(unused_imports)]
pub use completion_handler::{CompletionHandler, CompletionOutcome};
#[allow(unused_imports)]
pub use dispatcher::{ActiveSession, TaskContext, TaskDispatcher};
#[allow(unused_imports)]
pub use error_recovery::ErrorRecovery;
#[allow(unused_imports)]
pub use errors::{BuilderError, Result};
#[allow(unused_imports)]
pub use event_bus::{BuilderEvent, BuilderEventBus, CompletionResult};
#[allow(unused_imports)]
pub use heartbeat::HeartbeatManager;
#[allow(unused_imports)]
pub use merge_coordinator::{MergeCoordinator, MergeResult};
#[allow(unused_imports)]
pub use orchestrator::WorkflowOrchestrator;
#[allow(unused_imports)]
pub use session_manager::{ACPSessionHandle, SessionManager};
#[allow(unused_imports)]
pub use worktree_manager::{TaskWorktree, WorktreeManager};
