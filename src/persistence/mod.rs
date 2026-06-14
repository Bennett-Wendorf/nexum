//! Persistence layer for the Nexum agent system.
//!
//! This module provides all functionality for reading and writing the
//! structured data stored under the `.agent/` directory hierarchy.
//! It includes typed schemas, file I/O utilities, markdown parsing,
//! path resolution, and high-level CRUD operations.

mod directory;
mod errors;
mod io;
mod markdown;
mod operations;
mod schema;

#[cfg(test)]
mod tests;

pub use directory::*;
pub use errors::*;
pub use io::*;
pub use markdown::*;
pub use operations::*;
pub use schema::*;
