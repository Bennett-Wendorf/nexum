# Plan: 002 - Configuration System

## Task Description
Implement the nexum configuration system that reads from `~/.config/nexum/config.toml` (XDG conventions). The system provides agent registration, global settings, and preferences. It uses the `config` crate (Sergio Benitez) for TOML parsing and provides a typed Rust API for other modules to query configuration data.

## Objective
Create a complete configuration module (`src/config/`) that:
- Loads and parses `~/.config/nexum/config.toml` using the `config` crate
- Provides typed Rust structs for all configuration data
- Validates configuration and applies sensible defaults
- Exports a global configuration accessor for use by other modules
- Supports agent registration with spawn commands, capabilities, and preferences
- Supports global settings (concurrency limits, logging, server bind address)

## Problem Statement
Nexum needs a centralized configuration system to manage agent registrations, global operational settings, and user preferences. Without this, the ACP client cannot know which agents are available or how to spawn them, the Overlord cannot enforce concurrency limits, and the REST API has no server configuration. The configuration must be human-readable (TOML), follow XDG conventions, and provide a clean typed API for all other modules.

## Solution Approach
Build a `config` module in `src/config/` that defines typed structs matching the TOML schema, uses the `config` crate for file loading and environment variable overrides, and provides a global singleton accessor via `std::sync::LazyLock`. The configuration schema covers three areas:

1. **Agent registrations** — Each agent entry specifies type, spawn command, human-readable name, and optional settings (model, tool permissions, timeout, working directory)
2. **Global settings** — Server bind address/port, max parallel agents, log level, default timeouts
3. **Preferences** — Yolo mode toggle, default agent assignments per role

A comprehensive set of unit tests validates schema parsing, default value application, and error handling for malformed configurations.

## Relevant Files

### Existing Files
- `design/agent-harness-integration.md` — Configuration section defines what's configurable per agent and per task
- `design/tech-stack.md` — Specifies `config` crate (Sergio Benitez) for configuration
- `design/resource-constraints.md` — Defines concurrency limit configuration
- `design/permissions.md` — Defines yolo mode preference
- `design/implementation-chunks.md` — Defines this as chunk 2 of the MVP
- `Cargo.toml` — Will contain `config` dependency (added by chunk 001)
- `src/config/mod.rs` — Module stub (created by chunk 001)

### New Files (if needed)
- `src/config/mod.rs` — Module root, re-exports public types and `Config` accessor
- `src/config/schema.rs` — Typed configuration structs (AgentConfig, GlobalSettings, Preferences, etc.)
- `src/config/loader.rs` — Config file loading, XDG path resolution, default values, validation
- `src/config/accessor.rs` — Global singleton accessor and query API for other modules
- `src/config/tests.rs` — Unit tests for parsing, validation, defaults, error handling

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: config-builder
  - Role: Implement the configuration schema, loader, accessor, and tests
  - Agent: builder

- **Validator**
  - Name: config-validator
  - Role: Verify config loading, validation, defaults, and API correctness
  - Agent: validator

- **Documenter**
  - Name: config-documenter
  - Role: Generate documentation for completed configuration system
  - Agent: documenter

## Step by Step Tasks

### 1. Design Configuration Schema Structs
- **Task ID**: config-schema-design
- **Depends On**: none
- **Assigned To**: config-builder
- **Agent**: builder
- **Actions**:
  - Create `src/config/schema.rs` with the following typed structs, all deriving `Serialize`, `Deserialize`, and `Debug`:
    - `Config` — Top-level config with fields: `agents`, `global`, `preferences`
    - `AgentRegistration` — Per-agent entry with fields:
      - `name: String` — Human-readable name
      - `type: String` — Agent type identifier (e.g., "opencode", "kiro", "claude")
      - `spawn_command: String` — Binary path or adapter command (e.g., "opencode acp")
      - `model: Option<String>` — Model selection (optional, agent-specific)
      - `tool_permissions: Option<Vec<String>>` — Allowed tools (optional, defaults to all)
      - `timeout_seconds: Option<u64>` — Per-agent timeout (optional, falls back to global default)
      - `working_dir: Option<PathBuf>` — Working directory (optional, maps to task worktree)
    - `GlobalSettings` — Global operational settings with fields:
      - `server_host: String` — Default "127.0.0.1"
      - `server_port: u16` — Default 3000
      - `max_parallel: u16` — Max concurrent agent sessions, default 4
      - `default_timeout_seconds: u64` — Default task timeout, default 3600 (1 hour)
      - `log_level: String` — Default "info" (trace, debug, info, warn, error)
      - `nexum_config_dir: Option<PathBuf>` — Override for config directory (useful for testing)
    - `Preferences` — User preferences with fields:
      - `yolo_mode: bool` — Bypass permission prompts, default false
      - `default_builder_agent: Option<String>` — Default agent for builder role
      - `default_reviewer_agent: Option<String>` — Default agent for reviewer role
      - `default_planner_agent: Option<String>` — Default agent for planner role
  - Each field must have a `#[serde(default = "...")]` attribute pointing to a default value function
  - Document each field with rustdoc comments referencing the design doc
- **Acceptance Criteria**:
  - All structs compile with `serde` derive macros
  - Default values are defined for all optional fields
  - Structs match the configuration requirements from `design/agent-harness-integration.md`
  - Agent auth fields (API keys, OAuth) are explicitly excluded per design

### 2. Implement Configuration Loader
- **Task ID**: config-loader
- **Depends On**: config-schema-design
- **Assigned To**: config-builder
- **Agent**: builder
- **Actions**:
  - Create `src/config/loader.rs` with:
    - `fn config_path() -> PathBuf` — Resolves `~/.config/nexum/config.toml` using XDG conventions:
      - Check `XDG_CONFIG_HOME` env var, fall back to `~/.config`
      - Construct path: `{config_home}/nexum/config.toml`
      - Expand `~` using `std::env::var("HOME")`
    - `fn load() -> Result<Config, ConfigError>` — Loads configuration:
      - Build a `config::Config` builder
      - Add default values via `Config::try_from` or builder defaults
      - Attempt to load TOML file from `config_path()`
      - If file doesn't exist, create it with a default template (see step 3)
      - Parse environment variables with prefix `NEXUM_` (e.g., `NEXUM_SERVER_PORT=8080`)
      - Merge all sources and deserialize into `Config` struct
    - `fn validate(config: &Config) -> Result<(), ConfigError>` — Validates loaded config:
      - At least one agent registered (warn if empty, don't error)
      - `spawn_command` is non-empty for each agent
      - `server_port` is in valid range (1-65535)
      - `max_parallel` > 0
      - Agent names are unique (no duplicate `name` fields)
    - `enum ConfigError` — Custom error type using `thiserror`:
      - `FileIo(PathBuf, io::Error)` — File read/write error
      - `ParseError(String)` — TOML parsing error
      - `ValidationError(String)` — Validation failure
      - `CreateError(PathBuf, io::Error)` — Default config creation error
  - Implement `std::fmt::Display` for `ConfigError`
- **Acceptance Criteria**:
  - `config_path()` returns correct path based on XDG conventions
  - `load()` handles missing config file gracefully (creates default)
  - `load()` handles malformed TOML with descriptive error
  - `validate()` catches all defined validation rules
  - `ConfigError` derives `thiserror::Error` and implements `std::error::Error`

### 3. Create Default Configuration Template
- **Task ID**: config-default-template
- **Depends On**: config-schema-design
- **Assigned To**: config-builder
- **Agent**: builder
- **Actions**:
  - Add a `DEFAULT_CONFIG_TEMPLATE` constant in `loader.rs` containing a TOML string:
    ```toml
    # Nexum Configuration
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
    ```
  - Implement `fn create_default_config() -> Result<(), ConfigError>`:
    - Create parent directories (`~/.config/nexum/`) if they don't exist using `fs::create_dir_all`
    - Write the template to `config_path()` using `fs::write`
    - Return error if write fails
  - Call `create_default_config()` from `load()` when the config file doesn't exist
- **Acceptance Criteria**:
  - Template TOML is valid and parseable by the schema
  - `create_default_config()` creates directory and file if missing
  - Template includes comments explaining each field
  - Template matches the schema defined in `schema.rs`

### 4. Implement Global Config Accessor
- **Task ID**: config-accessor
- **Depends On**: config-loader
- **Assigned To**: config-builder
- **Agent**: builder
- **Actions**:
  - Create `src/config/accessor.rs` with:
    - A `LazyLock<Config>` global singleton initialized via `Config::load()`
    - `pub fn get() -> &'static Config` — Returns reference to loaded config
    - `pub fn init() -> Result<(), ConfigError>` — Explicit initialization function:
      - Loads config, validates, stores in `LazyLock`
      - Returns error if loading or validation fails
      - Panics if called twice (or returns an error for re-init)
    - Query helper functions:
      - `pub fn get_agent_by_name(name: &str) -> Option<&'static AgentRegistration>` — Find agent by name
      - `pub fn get_agent_by_type(agent_type: &str) -> Option<&'static AgentRegistration>` — Find agent by type
      - `pub fn get_agents_by_role(role: &str) -> Vec<&'static AgentRegistration>` — Filter agents suitable for a role (based on type or preferences)
      - `pub fn get_server_addr() -> String` — Returns "host:port" string
      - `pub fn get_max_parallel() -> u16` — Returns concurrency limit
      - `pub fn get_default_timeout() -> u64` — Returns default timeout
      - `pub fn is_yolo_mode() -> bool` — Returns yolo mode preference
  - Use `std::sync::LazyLock` (stable in Rust 1.80+) or `once_cell::sync::Lazy` as fallback
- **Acceptance Criteria**:
  - `get()` returns a valid config reference after `init()` is called
  - Query functions return correct results for loaded configuration
  - `get_agent_by_name` and `get_agent_by_type` handle missing agents gracefully
  - Global singleton is thread-safe

### 5. Wire Up Module Exports
- **Task ID**: config-module-wiring
- **Depends On**: config-accessor
- **Assigned To**: config-builder
- **Agent**: builder
- **Actions**:
  - Update `src/config/mod.rs` to:
    - Declare submodules: `mod schema; mod loader; mod accessor;`
    - Re-export public types: `pub use schema::*; pub use loader::*; pub use accessor::*;`
    - Re-export `ConfigError` for external use
  - Update `src/main.rs` to call `config::init()` during startup:
    - Wrap in `match` or `?` to handle errors
    - Log config loading status via `tracing`
    - If config loading fails, print error and exit with non-zero code
- **Acceptance Criteria**:
  - `src/config/mod.rs` properly exports all public types
  - `src/main.rs` initializes config on startup
  - Config errors during startup are properly logged and handled
  - All types are accessible as `nexum::config::Config`, etc.

### 6. Write Unit Tests
- **Task ID**: config-tests
- **Depends On**: config-module-wiring
- **Assigned To**: config-builder
- **Agent**: builder
- **Actions**:
  - Create `src/config/tests.rs` with comprehensive tests:
    - **Schema tests**:
      - `test_default_values` — Verify all default values are applied correctly
      - `test_agent_registration_parse` — Parse a sample agent registration
      - `test_global_settings_parse` — Parse global settings with defaults
    - **Loader tests**:
      - `test_config_path_xdg` — Verify XDG path resolution with `XDG_CONFIG_HOME` set
      - `test_config_path_home` — Verify fallback to `~/.config` when XDG not set
      - `test_load_missing_file_creates_default` — Verify default config creation
      - `test_load_malformed_toml` — Verify error on invalid TOML
      - `test_load_valid_file` — Verify successful loading of valid config
      - `test_env_override` — Verify environment variable overrides (e.g., `NEXUM_SERVER_PORT`)
    - **Validation tests**:
      - `test_validate_empty_agents_warns` — No agents triggers a warning, not error
      - `test_validate_duplicate_agent_names` — Duplicate names cause validation error
      - `test_validate_empty_spawn_command` — Empty spawn command causes validation error
      - `test_validate_invalid_port` — Port outside 1-65535 causes validation error
      - `test_validate_zero_parallel` — `max_parallel = 0` causes validation error
    - **Accessor tests**:
      - `test_get_agent_by_name` — Find agent by name
      - `test_get_agent_by_type` — Find agent by type
      - `test_get_server_addr` — Verify server address format
      - `test_is_yolo_mode` — Verify yolo mode preference
    - **Helper utilities**:
      - `fn create_test_config_dir() -> tempfile::TempDir` — Create temporary config directory
      - `fn write_test_config(dir: &Path, content: &str)` — Write test TOML file
  - Use `tempfile` crate for isolated test config directories
  - Use `std::env::var` manipulation for XDG and HOME testing
- **Acceptance Criteria**:
  - All tests pass with `cargo test --package nexum config`
  - Tests cover schema, loader, validation, and accessor functionality
  - Tests use temporary directories to avoid polluting `~/.config/`
  - Test coverage includes error paths (missing file, malformed TOML, validation failures)

### 7. Create Example Configuration
- **Task ID**: config-example
- **Depends On**: config-tests
- **Assigned To**: config-builder
- **Agent**: builder
- **Actions**:
  - Create `examples/config.toml` (or include in `README.md` section) with a fully populated example configuration:
    - Multiple agent registrations of different types (opencode, kiro, claude)
    - All global settings explicitly set
    - All preferences set
    - Comments explaining each section
  - This serves as a reference for users who want to customize their configuration
- **Acceptance Criteria**:
  - Example config is valid TOML parseable by the schema
  - Example includes agents of different types
  - Comments explain purpose of each field

### 8. Final Validation
- **Task ID**: validate-all
- **Depends On**: config-tests, config-example
- **Assigned To**: config-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed
  - Run `cargo test --package nexum config` — all config tests must pass
  - Run `cargo clippy --package nexum` — no warnings in config module
  - Verify `~/.config/nexum/config.toml` is created with default template when missing
  - Verify environment variable overrides work (e.g., `NEXUM_SERVER_PORT=8080`)
  - Verify validation catches: duplicate agent names, empty spawn commands, invalid ports
  - Verify accessor query functions return correct data
  - Verify config module is accessible from `src/main.rs`
  - Verify all public types are re-exported from `mod.rs`
  - Verify no agent authentication fields (API keys, OAuth) exist in the config schema

### 9. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: config-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`
  - Document the TOML schema format, default values, and environment variable overrides
  - Document the Rust API (`Config::load()`, `config::get()`, query functions)

## Acceptance Criteria
- `cargo check` succeeds with no errors in the config module
- `cargo test --package nexum config` passes all tests
- `cargo clippy` produces no warnings in config module
- Configuration loads from `~/.config/nexum/config.toml` using XDG conventions
- Missing config file triggers automatic creation of default template
- Environment variables with `NEXUM_` prefix override config file values
- Validation catches: duplicate agent names, empty spawn commands, invalid ports, zero parallel limit
- Default values are applied for all optional fields
- Global config singleton is thread-safe and accessible via `config::get()`
- Query functions (`get_agent_by_name`, `get_agent_by_type`, etc.) work correctly
- Agent authentication fields (API keys, OAuth) are explicitly excluded from schema
- Config module is properly wired into `src/main.rs` startup
- All public types are re-exported from `src/config/mod.rs`
- `thiserror` is used for `ConfigError` with descriptive variants

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo test --package nexum config` — Run configuration module tests
- `cargo clippy --package nexum` — Check for Rust lint warnings
- `rm -rf ~/.config/nexum && cargo run -- --version` — Verify default config creation on startup
- `NEXUM_SERVER_PORT=8080 cargo run -- --version` — Verify environment variable override
- `cargo doc --package nexum --no-deps` — Verify rustdoc generation succeeds

## Notes
- This plan depends on chunk 001 (Project Scaffolding) being completed first. The `src/config/mod.rs` stub, `Cargo.toml` with `config` dependency, and `src/main.rs` entry point must exist before this plan can be executed.
- The `config` crate (Sergio Benitez, crate name `config`) supports TOML natively via the `toml` feature. Ensure the `Cargo.toml` includes `config = { version = "0.14", features = ["toml"] }`.
- The `tempfile` crate should be added as a dev-dependency for testing.
- `std::sync::LazyLock` is stable since Rust 1.80. If the project targets an older edition, use `once_cell::sync::Lazy` instead.
- The configuration schema is designed to be extensible. Future chunks may add fields (e.g., git settings, persistence paths) without breaking the existing structure.
- Agent auth (API keys, OAuth) is intentionally excluded from nexum's config. Each agent manages its own authentication through its own configuration files.
- The `config` crate supports a merge order: defaults → config file → environment variables. This allows users to override any setting via environment without modifying the TOML file.
- Consider adding a `--config` CLI flag in a future enhancement to allow specifying an alternate config path.
