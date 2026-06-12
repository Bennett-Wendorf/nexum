//! ACP (Agent Communication Protocol) client module.
//!
//! This module provides a complete client for communicating with ACP-compatible
//! coding agents via JSON-RPC 2.0 over stdio. It handles subprocess spawning,
//! session lifecycle management, event streaming, permission handling, and
//! per-role configuration.
//!
//! # Architecture
//!
//! ```text
//! nexum (Overlord/Builder)
//!   │
//!   ├── src/acp/subprocess.rs  → Agent subprocess spawning & management
//!   ├── src/acp/client.rs      → JSON-RPC 2.0 client over stdio pipes
//!   ├── src/acp/session.rs     → Session lifecycle (create → run → destroy)
//!   ├── src/acp/events.rs      → Event streaming & types
//!   ├── src/acp/permissions.rs → Permission request handling
//!   └── src/acp/config.rs      → Per-role session configuration
//! ```
//!
//! See `design/agent-harness-integration.md` for the full design specification.

pub mod client;
pub mod config;
pub mod errors;
pub mod events;
pub mod permissions;
pub mod session;
pub mod subprocess;

#[cfg(test)]
mod tests;
