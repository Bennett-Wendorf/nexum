//! Overlord deterministic core.
//!
//! This module provides the deterministic core of the Overlord scheduler:
//! error types, status machines for plans and tasks, and sequential ID
//! generation.

pub mod errors;
pub mod id_generator;
pub mod status_machine;

pub use errors::{OverlordError, Result};
pub use id_generator::{PlanIdGenerator, TaskIdGenerator, slugify, format_dir_name};
pub use status_machine::{PlanStateMachine, TaskStateMachine, StatusTransitionRecord, is_concurrency_sensitive, is_plan_concurrency_sensitive};
