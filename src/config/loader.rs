use std::fs;
use std::io;
use std::path::PathBuf;

use super::schema::Config;
use thiserror::Error;

/// Custom error type for configuration operations
#[derive(Debug, Error)]
pub enum ConfigError {
    /// TOML parsing error
    #[error("Failed to parse configuration: {0}")]
    Parse(String),

    /// Validation failure
    #[error("Validation error: {0}")]
    Validation(String),

    /// Default config creation error
    #[error("Failed to create default config at {0}: {1}")]
    Create(PathBuf, #[source] io::Error),

    /// Environment variable error
    #[error("{0}")]
    Env(String),

    /// Configuration was already initialized
    #[error("Configuration has already been initialized")]
    AlreadyInitialized,
}

/// Default configuration template
pub const DEFAULT_CONFIG_TEMPLATE: &str = r#"# Nexum Configuration
# Location: ~/.config/nexum/config.toml

[global]
server_host = "127.0.0.1"
server_port = 3000
max_parallel = 4
default_timeout_seconds = 3600
log_level = "info"

[preferences]
yolo_mode = false
# default_builder_agent = "my-builder"
# default_reviewer_agent = "my-reviewer"
# default_planner_agent = "my-planner"

[[agents]]
name = "Example Agent"
type = "opencode"
spawn_command = "opencode acp"
# model = "llama3.1"
# tool_permissions = ["read", "write", "shell"]
# timeout_seconds = 1800
# working_dir = "/path/to/project"
"#;

/// Resolves the configuration file path using XDG conventions.
///
/// Checks `XDG_CONFIG_HOME` env var, falls back to `~/.config`.
/// Returns `{config_home}/nexum/config.toml`.
/// Returns an error if `HOME` is not set and `XDG_CONFIG_HOME` is unavailable.
pub fn config_path() -> Result<PathBuf, ConfigError> {
    let config_home = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if xdg.starts_with('~') {
            expand_tilde(&xdg)?
        } else {
            PathBuf::from(xdg)
        }
    } else {
        let home = std::env::var("HOME")
            .map_err(|_| ConfigError::Env("HOME environment variable is not set".to_string()))?;
        PathBuf::from(home).join(".config")
    };
    Ok(config_home.join("nexum").join("config.toml"))
}

/// Expand `~` to the user's home directory.
///
/// Only handles `~` for the current user (via the `HOME` environment variable).
/// Does not support `~username` expansion for other users.
fn expand_tilde(path: &str) -> Result<PathBuf, ConfigError> {
    let home = std::env::var("HOME")
        .map_err(|_| ConfigError::Env("HOME environment variable is not set".to_string()))?;
    if path == "~" {
        Ok(PathBuf::from(home))
    } else if path.starts_with("~/") {
        Ok(PathBuf::from(home).join(path.strip_prefix("~/").unwrap()))
    } else {
        Ok(PathBuf::from(path))
    }
}

/// Loads configuration from the config file.
///
/// If the config file doesn't exist, creates it with the default template.
/// Supports environment variable overrides with `NEXUM_` prefix.
pub fn load() -> Result<Config, ConfigError> {
    let path = config_path()?;

    // Attempt to read the file first. If it doesn't exist, create default and retry.
    // This avoids a TOCTOU race between checking existence and creating the file.
    match fs::read_to_string(&path) {
        Ok(_) => {} // file exists, proceed to load
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            tracing::info!("Config file not found, creating default at {:?}", path);
            create_default_config()?;
        }
        Err(e) => {
            return Err(ConfigError::Parse(format!("Failed to read config: {}", e)));
        }
    }

    let config_builder = config::Config::builder()
        // Load the TOML file (optional, not required)
        .add_source(config::File::from(path.as_path()).required(false))
        // Environment variables with NEXUM_ prefix
        .add_source(config::Environment::with_prefix("NEXUM").separator("__"));

    let store = config_builder
        .build()
        .map_err(|e| ConfigError::Parse(e.to_string()))?;

    let cfg: Config = store
        .try_deserialize()
        .map_err(|e| ConfigError::Parse(e.to_string()))?;

    Ok(cfg)
}

/// Creates a default configuration file if it doesn't exist.
pub fn create_default_config() -> Result<(), ConfigError> {
    let path = config_path()?;
    let parent = path.parent().ok_or_else(|| {
        ConfigError::Create(
            path.clone(),
            io::Error::new(io::ErrorKind::NotFound, "No parent directory"),
        )
    })?;

    fs::create_dir_all(parent).map_err(|e| ConfigError::Create(path.clone(), e))?;

    fs::write(&path, DEFAULT_CONFIG_TEMPLATE).map_err(|e| ConfigError::Create(path, e))?;

    Ok(())
}

/// Validates a loaded configuration.
///
/// Checks:
/// - At least one agent registered (warns if empty, doesn't error)
/// - spawn_command is non-empty for each agent
/// - server_port is in valid range (1-65535)
/// - max_parallel > 0
/// - Agent names are unique
/// - log_level is one of: trace, debug, info, warn, error
/// - server_host is not empty
pub fn validate(config: &Config) -> Result<(), ConfigError> {
    // Warn if no agents registered
    if config.agents.is_empty() {
        tracing::warn!("No agents registered in configuration");
    }

    // Check for duplicate agent names
    let mut seen_names = std::collections::HashSet::new();
    for agent in &config.agents {
        if !seen_names.insert(&agent.name) {
            return Err(ConfigError::Validation(format!(
                "Duplicate agent name: {}",
                agent.name
            )));
        }
    }

    // Validate each agent
    for agent in &config.agents {
        if agent.spawn_command.trim().is_empty() {
            return Err(ConfigError::Validation(format!(
                "Agent '{}' has empty spawn_command",
                agent.name
            )));
        }
    }

    // Validate server port
    if config.global.server_port == 0 {
        return Err(ConfigError::Validation(
            "server_port must be between 1 and 65535".to_string(),
        ));
    }

    // Validate max_parallel
    if config.global.max_parallel == 0 {
        return Err(ConfigError::Validation(
            "max_parallel must be greater than 0".to_string(),
        ));
    }

    // Validate log_level
    let valid_levels = ["trace", "debug", "info", "warn", "error"];
    if !valid_levels.contains(&config.global.log_level.as_str()) {
        return Err(ConfigError::Validation(format!(
            "Invalid log_level '{}'. Must be one of: trace, debug, info, warn, error",
            config.global.log_level
        )));
    }

    // Validate server_host
    if config.global.server_host.trim().is_empty() {
        return Err(ConfigError::Validation(
            "server_host must not be empty".to_string(),
        ));
    }

    Ok(())
}
