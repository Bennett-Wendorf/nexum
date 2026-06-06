//! Git operations module.
//!
//! This module provides abstractions for interacting with git repositories
//! through subprocess management. All git interactions within Nexum flow
//! through the [`subprocess`] module, ensuring consistent error handling
//! and timeout enforcement.

pub mod branch;
pub mod errors;
pub mod merge;
pub mod subprocess;
pub mod worktree;

pub use errors::{GitError, Result};
pub use subprocess::{git, GitCommand, GitOutput};
pub use worktree::WorktreeInfo;
