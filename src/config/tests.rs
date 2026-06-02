//! Unit tests for the configuration module.

use std::path::Path;
use tempfile::TempDir;

use super::schema::{AgentRegistration, Config, GlobalSettings, Preferences};
use super::loader::{validate, ConfigError, DEFAULT_CONFIG_TEMPLATE};

// ─── Helper utilities ───────────────────────────────────────────────

/// Creates a temporary directory for isolated test config operations.
fn create_test_config_dir() -> TempDir {
    TempDir::with_prefix("nexum-test-config").expect("Failed to create temp dir")
}

/// Writes a TOML config file into the given directory.
fn write_test_config(dir: &Path, content: &str) {
    let config_dir = dir.join("nexum");
    std::fs::create_dir_all(&config_dir).expect("Failed to create config dir");
    std::fs::write(config_dir.join("config.toml"), content)
        .expect("Failed to write config file");
}

// ─── Schema tests ──────────────────────────────────────────────────

#[test]
fn test_default_values() {
    let global = GlobalSettings::default();
    assert_eq!(global.server_host, "127.0.0.1");
    assert_eq!(global.server_port, 3000);
    assert_eq!(global.max_parallel, 4);
    assert_eq!(global.default_timeout_seconds, 3600);
    assert_eq!(global.log_level, "info");
    assert!(global.nexum_config_dir.is_none());

    let prefs = Preferences::default();
    assert!(!prefs.yolo_mode);
    assert!(prefs.default_builder_agent.is_none());
    assert!(prefs.default_reviewer_agent.is_none());
    assert!(prefs.default_planner_agent.is_none());
}

#[test]
fn test_agent_registration_parse() {
    let toml_str = r#"
name = "Test Agent"
type = "opencode"
spawn_command = "opencode acp"
model = "llama3.1"
tool_permissions = ["read", "write"]
timeout_seconds = 1800
"#;
    let agent: AgentRegistration = toml::from_str(toml_str)
        .expect("Failed to parse agent registration");
    
    assert_eq!(agent.name, "Test Agent");
    assert_eq!(agent.r#type, "opencode");
    assert_eq!(agent.spawn_command, "opencode acp");
    assert_eq!(agent.model, Some("llama3.1".to_string()));
    assert_eq!(agent.tool_permissions, Some(vec!["read".to_string(), "write".to_string()]));
    assert_eq!(agent.timeout_seconds, Some(1800));
    assert!(agent.working_dir.is_none());
}

#[test]
fn test_global_settings_parse() {
    let toml_str = r#"
server_host = "0.0.0.0"
server_port = 8080
max_parallel = 8
default_timeout_seconds = 7200
log_level = "debug"
"#;
    let settings: GlobalSettings = toml::from_str(toml_str)
        .expect("Failed to parse global settings");
    
    assert_eq!(settings.server_host, "0.0.0.0");
    assert_eq!(settings.server_port, 8080);
    assert_eq!(settings.max_parallel, 8);
    assert_eq!(settings.default_timeout_seconds, 7200);
    assert_eq!(settings.log_level, "debug");
}

// ─── Config parsing tests ──────────────────────────────────────────

#[test]
fn test_config_parse_full() {
    let toml_str = r#"
[global]
server_host = "127.0.0.1"
server_port = 3000
max_parallel = 4
default_timeout_seconds = 3600
log_level = "info"

[preferences]
yolo_mode = true
default_builder_agent = "builder-agent"

[[agents]]
name = "Builder"
type = "opencode"
spawn_command = "opencode acp"
"#;
    let config: Config = toml::from_str(toml_str)
        .expect("Failed to parse full config");
    
    assert_eq!(config.agents.len(), 1);
    assert_eq!(config.agents[0].name, "Builder");
    assert_eq!(config.global.server_port, 3000);
    assert!(config.preferences.yolo_mode);
    assert_eq!(config.preferences.default_builder_agent, Some("builder-agent".to_string()));
}

#[test]
fn test_config_parse_minimal() {
    // Empty config should parse with all defaults
    let toml_str = "";
    let config: Config = toml::from_str(toml_str)
        .expect("Failed to parse empty config");
    
    assert!(config.agents.is_empty());
    assert_eq!(config.global.server_host, "127.0.0.1");
    assert_eq!(config.global.server_port, 3000);
    assert!(!config.preferences.yolo_mode);
}

// ─── Validation tests ──────────────────────────────────────────────

#[test]
fn test_validate_empty_agents_warns() {
    let config = Config {
        agents: vec![],
        global: GlobalSettings::default(),
        preferences: Preferences::default(),
    };
    // Empty agents should warn but NOT error
    assert!(validate(&config).is_ok());
}

#[test]
fn test_validate_duplicate_agent_names() {
    let agent1 = AgentRegistration {
        name: "Same".to_string(),
        r#type: "opencode".to_string(),
        spawn_command: "opencode acp".to_string(),
        model: None,
        tool_permissions: None,
        timeout_seconds: None,
        working_dir: None,
    };
    let config = Config {
        agents: vec![agent1.clone(), agent1],
        global: GlobalSettings::default(),
        preferences: Preferences::default(),
    };
    let result = validate(&config);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ConfigError::ValidationError(_)));
    let msg = format!("{}", err);
    assert!(msg.contains("Duplicate"));
}

#[test]
fn test_validate_empty_spawn_command() {
    let agent = AgentRegistration {
        name: "Bad Agent".to_string(),
        r#type: "opencode".to_string(),
        spawn_command: "".to_string(),
        model: None,
        tool_permissions: None,
        timeout_seconds: None,
        working_dir: None,
    };
    let config = Config {
        agents: vec![agent],
        global: GlobalSettings::default(),
        preferences: Preferences::default(),
    };
    let result = validate(&config);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ConfigError::ValidationError(_)));
}

#[test]
fn test_validate_whitespace_spawn_command() {
    let agent = AgentRegistration {
        name: "Bad Agent".to_string(),
        r#type: "opencode".to_string(),
        spawn_command: "   ".to_string(),
        model: None,
        tool_permissions: None,
        timeout_seconds: None,
        working_dir: None,
    };
    let config = Config {
        agents: vec![agent],
        global: GlobalSettings::default(),
        preferences: Preferences::default(),
    };
    let result = validate(&config);
    assert!(result.is_err());
}

#[test]
fn test_validate_invalid_port() {
    let mut config = Config {
        agents: vec![],
        global: GlobalSettings::default(),
        preferences: Preferences::default(),
    };
    config.global.server_port = 0;
    let result = validate(&config);
    assert!(result.is_err());
}

#[test]
fn test_validate_zero_parallel() {
    let mut config = Config {
        agents: vec![],
        global: GlobalSettings::default(),
        preferences: Preferences::default(),
    };
    config.global.max_parallel = 0;
    let result = validate(&config);
    assert!(result.is_err());
}

#[test]
fn test_validate_valid_config() {
    let agent = AgentRegistration {
        name: "Good Agent".to_string(),
        r#type: "opencode".to_string(),
        spawn_command: "opencode acp".to_string(),
        model: None,
        tool_permissions: None,
        timeout_seconds: None,
        working_dir: None,
    };
    let config = Config {
        agents: vec![agent],
        global: GlobalSettings::default(),
        preferences: Preferences::default(),
    };
    assert!(validate(&config).is_ok());
}

// ─── Default template tests ────────────────────────────────────────

#[test]
fn test_default_template_parseable() {
    let config: Config = toml::from_str(DEFAULT_CONFIG_TEMPLATE)
        .expect("Default template should be valid TOML parseable by schema");
    
    assert_eq!(config.agents.len(), 1);
    assert_eq!(config.agents[0].name, "Example Agent");
    assert_eq!(config.agents[0].r#type, "opencode");
    assert_eq!(config.agents[0].spawn_command, "opencode acp");
}

// ─── ConfigError display tests ─────────────────────────────────────

#[test]
fn test_config_error_display() {
    let err = ConfigError::ValidationError("test error".to_string());
    assert!(format!("{}", err).contains("test error"));
    
    let err = ConfigError::ParseError("bad toml".to_string());
    assert!(format!("{}", err).contains("bad toml"));
}

// ─── File I/O tests ────────────────────────────────────────────────

#[test]
fn test_write_and_read_config() {
    let dir = create_test_config_dir();
    
    let config_content = r#"
[global]
server_port = 9999

[[agents]]
name = "Test Agent"
type = "opencode"
spawn_command = "opencode acp"
"#;
    write_test_config(dir.path(), config_content);
    
    let path = dir.path().join("nexum").join("config.toml");
    assert!(path.exists());
    
    let content = std::fs::read_to_string(&path).expect("Failed to read config");
    assert!(content.contains("server_port = 9999"));
    assert!(content.contains("Test Agent"));
}
