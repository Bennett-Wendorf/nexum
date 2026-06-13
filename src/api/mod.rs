//! REST API module for Nexum.
//!
//! This module defines the HTTP routes, request/response types, and error
//! handling for the Nexum REST API.

pub mod errors;
pub mod middleware;
pub mod plans;
pub mod tasks;
pub mod types;

#[allow(unused_imports)]
pub use errors::{ApiError, ApiErrorResponse};
#[allow(unused_imports)]
pub use middleware::*;
#[allow(unused_imports)]
pub use types::*;
