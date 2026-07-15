//! Configuration module for Nexum.
//!
//! Provides typed configuration structs, file loading with XDG path resolution,
//! environment variable overrides, and validation.

pub mod accessor;
pub mod loader;
pub mod schema;

#[cfg(test)]
mod tests;

#[allow(unused_imports)]
pub use accessor::{
    get, get_agent_by_name, get_agent_by_type, get_agents_by_role, get_default_timeout,
    get_max_parallel, get_server_addr, get_unchecked, init, is_yolo_mode,
};
#[allow(unused_imports)]
pub use loader::{
    config_path, create_default_config, load, validate, ConfigError, DEFAULT_CONFIG_TEMPLATE,
};
#[allow(unused_imports)]
pub use schema::{
    AgentRegistration, ApiKeyEntry, AuthenticationSettings, Config, GlobalSettings, Preferences,
};
