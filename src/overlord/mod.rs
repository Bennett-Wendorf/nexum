//! Overlord deterministic core.
//!
//! This module provides the deterministic core of the Overlord scheduler:
//! error types, status machines for plans and tasks, sequential ID
//! generation, concurrency enforcement, dependency resolution, and
//! heartbeat monitoring.

pub mod concurrency_checker;
pub mod dependency_resolver;
pub mod errors;
pub mod heartbeat_monitor;
pub mod id_generator;
pub mod status_machine;

pub use concurrency_checker::{ConcurrencyChecker, DispatchInfo, is_concurrency_gated, validate_transition_concurrency};
pub use dependency_resolver::DependencyResolver;
pub use errors::{OverlordError, Result};
pub use heartbeat_monitor::{HeartbeatMonitor, StaleTask};
pub use id_generator::{PlanIdGenerator, TaskIdGenerator, slugify, format_dir_name};
pub use status_machine::{PlanStateMachine, TaskStateMachine, StatusTransitionRecord, is_concurrency_sensitive, is_plan_concurrency_sensitive};
