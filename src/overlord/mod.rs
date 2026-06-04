//! Overlord Deterministic Core
//!
//! The rules-based, non-LLM engine that orchestrates plan and task lifecycle
//! management. Provides deterministic status machine transitions, sequential
//! ID generation, concurrency limit enforcement, dependency-driven auto-queueing,
//! stale heartbeat detection for orphaned task recovery, and a periodic scheduler
//! loop that ties all behaviors together.
//!
//! See `design/agent-roles.md` for the Overlord role definition.

mod concurrency_checker;
mod dependency_resolver;
mod errors;
mod heartbeat_monitor;
mod id_generator;
mod scheduler;
mod status_machine;

#[cfg(test)]
mod tests;

pub use concurrency_checker::{ConcurrencyChecker, DispatchInfo};
pub use concurrency_checker::validate_transition_concurrency;
pub use dependency_resolver::DependencyResolver;
pub use errors::{OverlordError, Result};
pub use heartbeat_monitor::{HeartbeatMonitor, StaleTask};
pub use id_generator::{PlanIdGenerator, TaskIdGenerator};
pub use scheduler::OverlordScheduler;
pub use status_machine::{PlanStateMachine, TaskStateMachine};
pub use status_machine::{task_status_to_string, plan_status_to_string};
