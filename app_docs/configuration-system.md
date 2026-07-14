# Configuration System

## Overview

The Nexum configuration system provides a centralized, typed configuration layer that manages agent registrations, global operational settings, and user preferences. Configuration is stored in TOML format at `~/.config/nexum/config.toml` following XDG Base Directory conventions. The system supports environment variable overrides and automatically creates a default configuration file if none exists.

## TOML Schema Format

The configuration file consists of three sections: `[global]`, `[preferences]`, and one or more `[[agents]]` arrays.

### `[global]` — Global Settings

Controls server configuration, concurrency limits, and logging.

| Field                        | Type   | Default        | Description                                      |
|------------------------------|--------|----------------|--------------------------------------------------|
| `server_host`                | string | `"127.0.0.1"`  | Server bind address                              |
| `server_port`                | u16    | `3000`         | Server port (must be 1–65535)                    |
| `max_parallel`               | u16    | `4`            | Maximum concurrent agent sessions (must be > 0)  |
| `default_timeout_seconds`    | u64    | `3600`         | Default task timeout in seconds (1 hour)         |
| `log_level`                  | string | `"info"`       | Log level: `trace`, `debug`, `info`, `warn`, `error` |
| `nexum_config_dir`           | string | _(unset)_      | Override for config directory (useful for testing) |

```toml
[global]
server_host = "127.0.0.1"
server_port = 3000
max_parallel = 4
default_timeout_seconds = 3600
log_level = "info"
```

### `[preferences]` — User Preferences

Controls yolo mode and default agent assignments per role.

| Field                      | Type   | Default | Description                                    |
|----------------------------|--------|---------|------------------------------------------------|
| `yolo_mode`                | bool   | `false` | Bypass permission prompts                      |
| `default_builder_agent`    | string | _(unset)| Default agent name for builder role            |
| `default_reviewer_agent`   | string | _(unset)| Default agent name for reviewer role           |
| `default_planner_agent`    | string | _(unset)| Default agent name for planner role            |

```toml
[preferences]
yolo_mode = false
default_builder_agent = "opencode-builder"
default_reviewer_agent = "kiro-reviewer"
default_planner_agent = "opencode-planner"
```

### `[[agents]]` — Agent Registrations

Each `[[agents]]` entry defines an agent that Nexum can spawn and communicate with via ACP. Agent authentication (API keys, OAuth) is intentionally excluded — each agent manages its own credentials through its own configuration files.

| Field              | Type     | Required | Description                                          |
|--------------------|----------|----------|------------------------------------------------------|
| `name`             | string   | yes      | Human-readable name (must be unique across agents)   |
| `type`             | string   | yes      | Agent type identifier (e.g., `"opencode"`, `"kiro"`, `"claude"`) |
| `spawn_command`    | string   | yes      | Binary path or adapter command (e.g., `"opencode acp"`) |
| `tool_permissions` | [string] | no       | Allowed tools; if omitted, all tools are permitted   |
| `timeout_seconds`  | u64      | no       | Per-agent timeout; falls back to global default      |
| `working_dir`      | string   | no       | Working directory (maps to task worktree)            |

```toml
[[agents]]
name = "opencode-builder"
type = "opencode"
spawn_command = "opencode acp"
tool_permissions = ["read", "write", "shell", "fs_read", "fs_write"]
timeout_seconds = 1800

[[agents]]
name = "kiro-reviewer"
type = "kiro"
spawn_command = "kiro acp"
tool_permissions = ["read", "fs_read"]
timeout_seconds = 900
```

## Environment Variable Overrides

Any configuration value can be overridden via environment variables using the `NEXUM_` prefix. Nested keys use `__` (double underscore) as a separator.

| Environment Variable              | Config Path                        |
|-----------------------------------|------------------------------------|
| `NEXUM_GLOBAL__SERVER_PORT`       | `global.server_port`               |
| `NEXUM_GLOBAL__LOG_LEVEL`         | `global.log_level`                 |
| `NEXUM_PREFERENCES__YOLO_MODE`    | `preferences.yolo_mode`            |
| `NEXUM_GLOBAL__MAX_PARALLEL`      | `global.max_parallel`              |

### Examples

```bash
# Override the server port
NEXUM_GLOBAL__SERVER_PORT=8080 nexum

# Enable yolo mode
NEXUM_PREFERENCES__YOLO_MODE=true nexum

# Override log level
NEXUM_GLOBAL__LOG_LEVEL=debug nexum
```

The merge order is: **defaults → config file → environment variables**. Environment variables take highest precedence.

## Rust API

### Initialization

```rust
use nexum::config;

// Initialize configuration at application startup.
// Loads from ~/.config/nexum/config.toml, validates, and caches globally.
config::init()?;
```

- **`config::init() -> Result<(), ConfigError>`** — Loads configuration from the TOML file, runs validation, and stores it in a thread-safe global singleton. Returns an error if loading, parsing, or validation fails. Cannot be called twice — the second call returns a `ConfigError::Validation` error.

### Accessing Configuration

```rust
use nexum::config;

// Safe access — returns None if not initialized
let cfg = config::get();

// Unchecked access — panics if not initialized
let cfg = config::get_unchecked();
```

- **`config::get() -> Option<&'static Config>`** — Returns a reference to the loaded configuration, or `None` if `init()` has not been called.
- **`config::get_unchecked() -> &'static Config`** — Returns a reference to the loaded configuration. Panics if `init()` has not been called.

### Query Functions

```rust
use nexum::config;

// Find agent by name
let agent = config::get_agent_by_name("opencode-builder");

// Find agent by type
let agent = config::get_agent_by_type("kiro");

// Get agents suitable for a role (checks preferences first, then type matching)
let agents = config::get_agents_by_role("builder");

// Get server address string ("host:port")
let addr = config::get_server_addr(); // Some("127.0.0.1:3000")

// Get concurrency limit
let max = config::get_max_parallel(); // Some(4)

// Get default timeout
let timeout = config::get_default_timeout(); // Some(3600)

// Check yolo mode
let yolo = config::is_yolo_mode(); // false (default)
```

| Function                              | Return Type                          | Description                                          |
|---------------------------------------|--------------------------------------|------------------------------------------------------|
| `get_agent_by_name(name: &str)`       | `Option<&'static AgentRegistration>` | Find agent by human-readable name                    |
| `get_agent_by_type(agent_type: &str)` | `Option<&'static AgentRegistration>` | Find agent by type identifier                        |
| `get_agents_by_role(role: &str)`      | `Vec<&'static AgentRegistration>`    | Get agents for a role (checks preferences first)     |
| `get_server_addr()`                   | `Option<String>`                     | Returns `"host:port"` string                         |
| `get_max_parallel()`                  | `Option<u16>`                        | Returns concurrency limit                            |
| `get_default_timeout()`               | `Option<u64>`                        | Returns default timeout in seconds                   |
| `is_yolo_mode()`                      | `bool`                               | Returns `true` if yolo mode is enabled               |

### Lower-Level Functions

```rust
use nexum::config;

// Manually load configuration (without global caching)
let cfg = config::load()?;

// Validate a configuration instance
config::validate(&cfg)?;

// Get the config file path
let path = config::config_path()?;

// Create a default config file
config::create_default_config()?;
```

## Default Configuration

When `~/.config/nexum/config.toml` does not exist, the system automatically creates it with a default template containing:

- One example agent of type `"opencode"` named `"Example Agent"`
- All global settings at their default values
- Yolo mode disabled
- Default agent assignments commented out

The parent directory (`~/.config/nexum/`) is created if it does not exist. The default template is available as the constant `config::DEFAULT_CONFIG_TEMPLATE`.

The config path resolution follows XDG conventions:
1. Check `XDG_CONFIG_HOME` environment variable
2. Fall back to `~/.config` (resolved via `HOME`)
3. Final path: `{config_home}/nexum/config.toml`

## Validation Rules

The `validate()` function checks the following rules after loading:

| Rule                              | Behavior                                    |
|-----------------------------------|---------------------------------------------|
| No agents registered              | Warning (logged via `tracing`), not an error |
| Duplicate agent names             | **Error** — all agent `name` fields must be unique |
| Empty `spawn_command`             | **Error** — every agent must have a non-empty spawn command |
| `server_port` = 0                 | **Error** — port must be between 1 and 65535 |
| `max_parallel` = 0                | **Error** — must be greater than 0          |
| Invalid `log_level`               | **Error** — must be one of: `trace`, `debug`, `info`, `warn`, `error` |
| Empty `server_host`               | **Error** — host must not be empty          |

## Error Types

The `ConfigError` enum (using `thiserror`) defines all configuration-related errors:

| Variant      | Description                                    |
|--------------|------------------------------------------------|
| `Parse(String)`      | TOML parsing or file read error                |
| `Validation(String)` | Validation failure (descriptive message)       |
| `Create(PathBuf, io::Error)` | Failed to create default config file |
| `Env(String)`        | Environment variable error (e.g., `HOME` not set) |

### Example Error Handling

```rust
match config::init() {
    Ok(()) => println!("Configuration loaded successfully"),
    Err(ConfigError::Parse(msg)) => eprintln!("Parse error: {}", msg),
    Err(ConfigError::Validation(msg)) => eprintln!("Validation error: {}", msg),
    Err(ConfigError::Create(path, io_err)) => eprintln!("Cannot create config at {}: {}", path.display(), io_err),
    Err(ConfigError::Env(msg)) => eprintln!("Environment error: {}", msg),
}
```

## Files

| File                              | Purpose                                          |
|-----------------------------------|--------------------------------------------------|
| `src/config/mod.rs`               | Module root; re-exports all public types and functions |
| `src/config/schema.rs`            | Typed configuration structs (`Config`, `AgentRegistration`, `GlobalSettings`, `Preferences`) |
| `src/config/loader.rs`            | Config file loading, XDG path resolution, default template, validation, `ConfigError` |
| `src/config/accessor.rs`          | Global singleton accessor and query helper functions |
| `examples/config.toml`            | Fully populated example configuration with multiple agent types |

## Dependencies

- **`config`** (Sergio Benitez) — TOML parsing and environment variable merging
- **`serde`** — Serialization/deserialization of configuration structs
- **`thiserror`** — `ConfigError` error type with `#[derive(Error)]`
- **`tracing`** — Logging for config load warnings and info messages
