use std::fs;
use std::io;
use std::path::PathBuf;

use super::schema::Config;
use thiserror::Error;

/// Custom error type for configuration operations
#[derive(Debug, Error)]
pub enum ConfigError {
    /// File read/write error
    #[error("File I/O error at {0}: {1}")]
    FileIo(PathBuf, #[source] io::Error),

    /// TOML parsing error
    #[error("Failed to parse configuration: {0}")]
    ParseError(String),

    /// Validation failure
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Default config creation error
    #[error("Failed to create default config at {0}: {1}")]
    CreateError(PathBuf, #[source] io::Error),
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
pub fn config_path() -> PathBuf {
    let config_home = if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if xdg.starts_with('~') {
            expand_tilde(&xdg)
        } else {
            PathBuf::from(xdg)
        }
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
        PathBuf::from(home).join(".config")
    };
    config_home.join("nexum").join("config.toml")
}

/// Expand `~` to the user's home directory
fn expand_tilde(path: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "~".to_string());
    if path == "~" {
        PathBuf::from(home)
    } else if path.starts_with("~/") {
        PathBuf::from(home).join(path.strip_prefix("~/").unwrap())
    } else {
        PathBuf::from(path)
    }
}

/// Loads configuration from the config file.
///
/// If the config file doesn't exist, creates it with the default template.
/// Supports environment variable overrides with `NEXUM_` prefix.
pub fn load() -> Result<Config, ConfigError> {
    let path = config_path();

    if !path.exists() {
        tracing::info!("Config file not found, creating default at {:?}", path);
        create_default_config()?;
    }

    let config_builder = config::Config::builder()
        // Load the TOML file (optional, not required)
        .add_source(config::File::from(path.as_path()).required(false))
        // Environment variables with NEXUM_ prefix
        .add_source(config::Environment::with_prefix("NEXUM").separator("__"));

    let store = config_builder
        .build()
        .map_err(|e| ConfigError::ParseError(e.to_string()))?;

    let cfg: Config = store
        .try_deserialize()
        .map_err(|e| ConfigError::ParseError(e.to_string()))?;

    Ok(cfg)
}

/// Creates a default configuration file if it doesn't exist.
pub fn create_default_config() -> Result<(), ConfigError> {
    let path = config_path();
    let parent = path.parent().ok_or_else(|| {
        ConfigError::CreateError(
            path.clone(),
            io::Error::new(io::ErrorKind::NotFound, "No parent directory"),
        )
    })?;

    fs::create_dir_all(parent).map_err(|e| ConfigError::CreateError(path.clone(), e))?;

    fs::write(&path, DEFAULT_CONFIG_TEMPLATE)
        .map_err(|e| ConfigError::CreateError(path, e))?;

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
pub fn validate(config: &Config) -> Result<(), ConfigError> {
    // Warn if no agents registered
    if config.agents.is_empty() {
        tracing::warn!("No agents registered in configuration");
    }

    // Check for duplicate agent names
    let mut seen_names = std::collections::HashSet::new();
    for agent in &config.agents {
        if !seen_names.insert(&agent.name) {
            return Err(ConfigError::ValidationError(format!(
                "Duplicate agent name: {}",
                agent.name
            )));
        }
    }

    // Validate each agent
    for agent in &config.agents {
        if agent.spawn_command.trim().is_empty() {
            return Err(ConfigError::ValidationError(format!(
                "Agent '{}' has empty spawn_command",
                agent.name
            )));
        }
    }

    // Validate server port
    if config.global.server_port == 0 {
        return Err(ConfigError::ValidationError(
            "server_port must be between 1 and 65535".to_string(),
        ));
    }

    // Validate max_parallel
    if config.global.max_parallel == 0 {
        return Err(ConfigError::ValidationError(
            "max_parallel must be greater than 0".to_string(),
        ));
    }

    Ok(())
}
