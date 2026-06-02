//! Configuration module for Nexum.
//!
//! Provides typed configuration structs, file loading with XDG path resolution,
//! environment variable overrides, and validation.

pub mod accessor;
pub mod loader;
pub mod schema;

pub use accessor::{get, init, get_agent_by_name, get_agent_by_type, get_agents_by_role, get_server_addr, get_max_parallel, get_default_timeout, is_yolo_mode};
pub use loader::{config_path, create_default_config, load, validate, ConfigError, DEFAULT_CONFIG_TEMPLATE};
pub use schema::{AgentRegistration, Config, GlobalSettings, Preferences};
