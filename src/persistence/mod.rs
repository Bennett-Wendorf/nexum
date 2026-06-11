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

#[allow(unused_imports)]
pub use directory::*;
#[allow(unused_imports)]
pub use errors::*;
#[allow(unused_imports)]
pub use io::*;
#[allow(unused_imports)]
pub use markdown::*;
#[allow(unused_imports)]
pub use operations::*;
#[allow(unused_imports)]
pub use schema::*;
