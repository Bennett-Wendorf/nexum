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

#[allow(unused_imports)]
pub use concurrency_checker::validate_transition_concurrency;
#[allow(unused_imports)]
pub use concurrency_checker::{ConcurrencyChecker, DispatchInfo};
#[allow(unused_imports)]
pub use dependency_resolver::DependencyResolver;
#[allow(unused_imports)]
pub use errors::{OverlordError, Result};
#[allow(unused_imports)]
pub use heartbeat_monitor::{HeartbeatMonitor, StaleTask};
#[allow(unused_imports)]
pub use id_generator::{PlanIdGenerator, TaskIdGenerator};
#[allow(unused_imports)]
pub use scheduler::OverlordScheduler;
#[allow(unused_imports)]
pub use scheduler::TransitionTaskStatusParams;
#[allow(unused_imports)]
pub use status_machine::{PlanStateMachine, TaskStateMachine};
