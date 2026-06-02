//! Configuration module for Nexum.
//!
//! Provides typed configuration structs, file loading with XDG path resolution,
//! environment variable overrides, and validation.

pub mod loader;
pub mod schema;

pub use loader::{config_path, create_default_config, load, validate, ConfigError, DEFAULT_CONFIG_TEMPLATE};
pub use schema::{AgentRegistration, Config, GlobalSettings, Preferences};
