//! Global configuration accessor.
//!
//! Provides a thread-safe singleton accessor to the loaded configuration
//! and query helper functions for common lookups.

use std::sync::RwLock;

use super::loader::{load, validate, ConfigError};
use super::schema::{AgentRegistration, Config};

/// Global configuration singleton.
static CONFIG: RwLock<Option<Config>> = RwLock::new(None);

/// Cached server address. Reset alongside config during `reset()`.
static SERVER_ADDR_CACHE: RwLock<Option<String>> = RwLock::new(None);

/// Returns a clone of the loaded configuration, or `None` if not yet initialized.
pub fn get() -> Option<Config> {
    let guard = CONFIG.read().expect("Config RwLock poisoned");
    guard.clone()
}

/// Returns a clone of the loaded configuration.
///
/// # Panics
/// Panics if `init()` has not been called yet or if initialization failed.
pub fn get_unchecked() -> Config {
    let guard = CONFIG.read().expect("Config RwLock poisoned");
    guard.clone().expect("Configuration has not been initialized. Call config::init() first.")
}

/// Explicitly initializes the global configuration.
///
/// Loads config from `~/.config/nexum/config.toml`, validates it, and stores it globally.
/// Returns an error if loading or validation fails.
/// If called twice, the second call returns an AlreadyInitialized error.
pub fn init() -> Result<(), ConfigError> {
    let cfg = load()?;
    validate(&cfg)?;

    tracing::info!("Configuration loaded successfully from {:?}",
        super::loader::config_path()?);

    let mut guard = CONFIG.write().expect("Config RwLock poisoned");
    if guard.is_some() {
        return Err(ConfigError::AlreadyInitialized);
    }
    *guard = Some(cfg);
    Ok(())
}

/// Finds an agent registration by its human-readable name.
pub fn get_agent_by_name(name: &str) -> Option<AgentRegistration> {
    let guard = CONFIG.read().expect("Config RwLock poisoned");
    guard.as_ref()?.agents.iter().find(|a| a.name == name).cloned()
}

/// Finds an agent registration by its type identifier (e.g., "opencode", "kiro").
pub fn get_agent_by_type(agent_type: &str) -> Option<AgentRegistration> {
    let guard = CONFIG.read().expect("Config RwLock poisoned");
    guard.as_ref()?.agents.iter().find(|a| a.r#type == agent_type).cloned()
}

/// Returns agents suitable for a given role.
///
/// First checks preferences for a default agent assignment, then falls back
/// to filtering by agent type matching the role name.
pub fn get_agents_by_role(role: &str) -> Vec<AgentRegistration> {
    let guard = CONFIG.read().expect("Config RwLock poisoned");
    let cfg = match guard.as_ref() {
        Some(cfg) => cfg,
        None => return Vec::new(),
    };

    // Check preferences for default agent assignment
    let default_name = match role {
        "builder" => cfg.preferences.default_builder_agent.as_deref(),
        "reviewer" => cfg.preferences.default_reviewer_agent.as_deref(),
        "planner" => cfg.preferences.default_planner_agent.as_deref(),
        _ => None,
    };

    if let Some(name) = default_name {
        if let Some(agent) = cfg.agents.iter().find(|a| a.name == name) {
            return vec![agent.clone()];
        }
    }

    // Fall back to type matching
    cfg.agents.iter()
        .filter(|a| a.r#type == role)
        .cloned()
        .collect()
}

/// Returns the server address as "host:port" string, or `None` if not initialized.
/// The result is cached after the first call and cleared on `reset()`.
pub fn get_server_addr() -> Option<String> {
    // Check cache first
    {
        let cache = SERVER_ADDR_CACHE.read().expect("Server addr cache RwLock poisoned");
        if let Some(addr) = cache.as_ref() {
            return Some(addr.clone());
        }
    }

    // Compute and cache
    let guard = CONFIG.read().expect("Config RwLock poisoned");
    let addr = guard.as_ref().map(|cfg| {
        format!("{}:{}", cfg.global.server_host, cfg.global.server_port)
    });

    if let Some(ref a) = addr {
        let mut cache = SERVER_ADDR_CACHE.write().expect("Server addr cache RwLock poisoned");
        *cache = Some(a.clone());
    }

    addr
}

/// Returns the maximum number of parallel agent sessions, or `None` if not initialized.
pub fn get_max_parallel() -> Option<u16> {
    let guard = CONFIG.read().expect("Config RwLock poisoned");
    guard.as_ref().map(|cfg| cfg.global.max_parallel)
}

/// Returns the default task timeout in seconds, or `None` if not initialized.
pub fn get_default_timeout() -> Option<u64> {
    let guard = CONFIG.read().expect("Config RwLock poisoned");
    guard.as_ref().map(|cfg| cfg.global.default_timeout_seconds)
}

/// Returns whether yolo mode (bypass permission prompts) is enabled.
/// Returns `false` if configuration has not been initialized (safe default).
pub fn is_yolo_mode() -> bool {
    let guard = CONFIG.read().expect("Config RwLock poisoned");
    guard.as_ref().map(|cfg| cfg.preferences.yolo_mode).unwrap_or(false)
}

/// Resets the configuration to uninitialized state.
///
/// # Test Only
/// This function is only available in test builds (`#[cfg(test)]`).
#[cfg(test)]
pub fn reset() {
    let mut guard = CONFIG.write().expect("Config RwLock poisoned");
    *guard = None;
    // Also clear the server address cache
    let mut cache = SERVER_ADDR_CACHE.write().expect("Server addr cache RwLock poisoned");
    *cache = None;
}

/// Sets a specific configuration for testing purposes.
///
/// **Note:** This function bypasses `validate()`. Tests may inject
/// minimal or edge-case configs that would fail production validation.
///
/// # Test Only
/// This function is only available in test builds (`#[cfg(test)]`).
#[cfg(test)]
pub fn with_config(config: Config) {
    let mut guard = CONFIG.write().expect("Config RwLock poisoned");
    *guard = Some(config);
    // Clear the server address cache since config may have changed
    let mut cache = SERVER_ADDR_CACHE.write().expect("Server addr cache RwLock poisoned");
    *cache = None;
}
