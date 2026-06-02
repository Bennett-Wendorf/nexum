//! Global configuration accessor.
//!
//! Provides a thread-safe singleton accessor to the loaded configuration
//! and query helper functions for common lookups.

use std::sync::OnceLock;

use super::loader::{load, validate, ConfigError};
use super::schema::{AgentRegistration, Config};

/// Global configuration singleton.
static CONFIG: OnceLock<Config> = OnceLock::new();

/// Returns a reference to the loaded configuration.
///
/// # Panics
/// Panics if `init()` has not been called yet or if initialization failed.
pub fn get() -> &'static Config {
    CONFIG.get().expect("Configuration has not been initialized. Call config::init() first.")
}

/// Explicitly initializes the global configuration.
///
/// Loads config from `~/.config/nexum/config.toml`, validates it, and stores it globally.
/// Returns an error if loading or validation fails.
/// If called twice, the second call returns an error since `OnceLock` only accepts once.
pub fn init() -> Result<(), ConfigError> {
    let cfg = load()?;
    validate(&cfg)?;
    
    tracing::info!("Configuration loaded successfully from {:?}", 
        super::loader::config_path()?);
    
    match CONFIG.set(cfg) {
        Ok(()) => Ok(()),
        Err(_) => Err(ConfigError::ValidationError(
            "Configuration has already been initialized".to_string()
        )),
    }
}

/// Finds an agent registration by its human-readable name.
pub fn get_agent_by_name(name: &str) -> Option<&'static AgentRegistration> {
    get().agents.iter().find(|a| a.name == name)
}

/// Finds an agent registration by its type identifier (e.g., "opencode", "kiro").
pub fn get_agent_by_type(agent_type: &str) -> Option<&'static AgentRegistration> {
    get().agents.iter().find(|a| a.r#type == agent_type)
}

/// Returns agents suitable for a given role.
///
/// First checks preferences for a default agent assignment, then falls back
/// to filtering by agent type matching the role name.
pub fn get_agents_by_role(role: &str) -> Vec<&'static AgentRegistration> {
    let cfg = get();
    
    // Check preferences for default agent assignment
    let default_name = match role {
        "builder" => cfg.preferences.default_builder_agent.as_deref(),
        "reviewer" => cfg.preferences.default_reviewer_agent.as_deref(),
        "planner" => cfg.preferences.default_planner_agent.as_deref(),
        _ => None,
    };
    
    if let Some(name) = default_name {
        if let Some(agent) = cfg.agents.iter().find(|a| a.name == name) {
            return vec![agent];
        }
    }
    
    // Fall back to type matching
    cfg.agents.iter()
        .filter(|a| a.r#type == role)
        .collect()
}

/// Returns the server address as "host:port" string.
pub fn get_server_addr() -> String {
    let cfg = get();
    format!("{}:{}", cfg.global.server_host, cfg.global.server_port)
}

/// Returns the maximum number of parallel agent sessions.
pub fn get_max_parallel() -> u16 {
    get().global.max_parallel
}

/// Returns the default task timeout in seconds.
pub fn get_default_timeout() -> u64 {
    get().global.default_timeout_seconds
}

/// Returns whether yolo mode (bypass permission prompts) is enabled.
pub fn is_yolo_mode() -> bool {
    get().preferences.yolo_mode
}
