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

// ── Re-exported public API (unused within crate, but exposed for external consumers) ──
#[allow(unused_imports)]
pub use errors::{ACPError, Result};
#[allow(unused_imports)]
pub use subprocess::{spawn_agent, AgentConfig, AgentProcess};
#[allow(unused_imports)]
pub use client::{
    ACPCapabilities, ACPClient, MessageType, SessionCreateParams, SessionCreateResult,
};
#[allow(unused_imports)]
pub use events::{
    is_terminal_event, log_event, requires_response, session_id, ACPEvent, CompletionStatus,
    EventStream,
};
#[allow(unused_imports)]
pub use session::{ACPSession, AgentRole, SessionState};
#[allow(unused_imports)]
pub use permissions::{
    default_policy_for_role, handle_permission, PermissionAction, PermissionDecision,
    PermissionHandler, PermissionPolicy, PermissionRequest,
};
#[allow(unused_imports)]
pub use config::{
    default_role_config, merge_with_task_config, to_session_params, RoleConfig, TaskConfigOverrides,
};

#[cfg(test)]
mod tests;
