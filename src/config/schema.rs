use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Default values helper functions
mod defaults {
    pub fn server_host() -> String {
        "127.0.0.1".to_string()
    }
    pub fn server_port() -> u16 {
        3000
    }
    pub fn max_parallel() -> u16 {
        4
    }
    pub fn default_timeout_seconds() -> u64 {
        3600
    }
    pub fn log_level() -> String {
        "info".to_string()
    }
    pub fn yolo_mode() -> bool {
        false
    }
}

/// Top-level configuration struct
///
/// Represents the complete nexum configuration loaded from `~/.config/nexum/config.toml`.
/// Contains agent registrations, global operational settings, and user preferences.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    /// Registered agents. Each entry defines an agent type, spawn command, and optional settings.
    /// See: design/agent-harness-integration.md
    #[serde(default)]
    pub agents: Vec<AgentRegistration>,

    /// Global operational settings (server, concurrency, logging).
    /// See: design/resource-constraints.md
    #[serde(default)]
    pub global: GlobalSettings,

    /// User preferences (yolo mode, default agent assignments).
    /// See: design/permissions.md
    #[serde(default)]
    pub preferences: Preferences,
}

/// Per-agent registration entry
///
/// Defines how nexum spawns and configures an individual agent via ACP.
/// Agent auth (API keys, OAuth) is intentionally excluded — each agent manages its own authentication.
/// See: design/agent-harness-integration.md
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AgentRegistration {
    /// Human-readable name for this agent registration
    pub name: String,

    /// Agent type identifier (e.g., "opencode", "kiro", "claude")
    pub r#type: String,

    /// Binary path or adapter command (e.g., "opencode acp")
    pub spawn_command: String,

    /// Model selection (optional, agent-specific)
    #[serde(default)]
    pub model: Option<String>,

    /// Allowed tools (optional, defaults to all)
    #[serde(default)]
    pub tool_permissions: Option<Vec<String>>,

    /// Per-agent timeout in seconds (optional, falls back to global default)
    #[serde(default)]
    pub timeout_seconds: Option<u64>,

    /// Working directory (optional, maps to task worktree)
    #[serde(default)]
    pub working_dir: Option<PathBuf>,
}

/// Global operational settings
///
/// Controls server configuration, concurrency limits, and logging.
/// See: design/resource-constraints.md
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct GlobalSettings {
    /// Server bind address (default: "127.0.0.1")
    #[serde(default = "defaults::server_host")]
    pub server_host: String,

    /// Server port (default: 3000)
    #[serde(default = "defaults::server_port")]
    pub server_port: u16,

    /// Max concurrent agent sessions (default: 4)
    #[serde(default = "defaults::max_parallel")]
    pub max_parallel: u16,

    /// Default task timeout in seconds (default: 3600 = 1 hour)
    #[serde(default = "defaults::default_timeout_seconds")]
    pub default_timeout_seconds: u64,

    /// Log level: trace, debug, info, warn, error (default: "info")
    #[serde(default = "defaults::log_level")]
    pub log_level: String,

    /// Override for config directory (useful for testing)
    #[serde(default)]
    pub nexum_config_dir: Option<PathBuf>,
}

/// User preferences
///
/// Controls yolo mode and default agent assignments per role.
/// See: design/permissions.md
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct Preferences {
    /// Bypass permission prompts (default: false)
    #[serde(default = "defaults::yolo_mode")]
    pub yolo_mode: bool,

    /// Default agent name for builder role
    #[serde(default)]
    pub default_builder_agent: Option<String>,

    /// Default agent name for reviewer role
    #[serde(default)]
    pub default_reviewer_agent: Option<String>,

    /// Default agent name for planner role
    #[serde(default)]
    pub default_planner_agent: Option<String>,
}
