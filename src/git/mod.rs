//! Git operations module.
//!
//! This module provides abstractions for interacting with git repositories
//! through subprocess management. All git interactions within Nexum flow
//! through the [`subprocess`] module, ensuring consistent error handling
//! and timeout enforcement.

pub mod errors;
pub mod subprocess;

pub use errors::{GitError, Result};
pub use subprocess::{git, GitCommand, GitOutput};
