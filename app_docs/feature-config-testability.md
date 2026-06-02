# Config Testability

## Overview

Replaced the global configuration singleton's backing store from `std::sync::OnceLock<Config>` to `std::sync::RwLock<Option<Config>>`, enabling test isolation. Tests can now reset and re-inject configuration between test cases without process restarts.

## What Was Built

The global config accessor (`src/config/accessor.rs`) was redesigned to use `RwLock<Option<Config>>` instead of `OnceLock<Config>`. This provides:

- **Concurrent reads** — Multiple threads can hold shared read locks simultaneously
- **Exclusive writes** — `init()` and test-only functions acquire a write lock
- **Reset capability** — `#[cfg(test)]` `reset()` clears the config to `None`
- **Dependency injection** — `#[cfg(test)]` `with_config(Config)` injects a specific config for tests
- **Production safety** — `init()` still enforces single-initialization semantics by returning `ConfigError::AlreadyInitialized` on duplicate calls

Two new `ConfigError` variants (`AlreadyInitialized`, `NotInitialized`) were added to `src/config/loader.rs`. The `AgentRegistration` struct now derives `Clone` to support owned-value returns from accessor query functions.

## Technical Implementation

### Files Modified

| File | Changes |
|------|---------|
| `src/config/accessor.rs` | Replaced `OnceLock<Config>` with `RwLock<Option<Config>>`; rewrote all functions to use lock guards; added `reset()` and `with_config()` test functions |
| `src/config/loader.rs` | Added `AlreadyInitialized` and `NotInitialized` variants to `ConfigError` |
| `src/config/schema.rs` | `AgentRegistration` now derives `Clone` |
| `src/config/tests.rs` | Added 15 new accessor tests (39 total) covering init, reset, with_config, and all query functions through the singleton |

### Key API Functions

#### Production API (unchanged callers, different return types)

| Function | Signature | Description |
|----------|-----------|-------------|
| `init()` | `-> Result<(), ConfigError>` | Loads, validates, and stores config. Returns `AlreadyInitialized` if called twice. |
| `get()` | `-> Option<Config>` | Returns a **clone** of the loaded config, or `None` if not initialized. |
| `get_unchecked()` | `-> Config` | Returns a **clone** of the loaded config. Panics if not initialized. |
| `get_agent_by_name(name)` | `-> Option<AgentRegistration>` | Finds an agent by human-readable name. |
| `get_agent_by_type(type)` | `-> Option<AgentRegistration>` | Finds an agent by type identifier (e.g., `"opencode"`). |
| `get_agents_by_role(role)` | `-> Vec<AgentRegistration>` | Returns agents for a role, checking preferences first, then type fallback. |
| `get_server_addr()` | `-> Option<String>` | Returns `"host:port"` string. |
| `get_max_parallel()` | `-> Option<u16>` | Returns max concurrent agent sessions. |
| `get_default_timeout()` | `-> Option<u64>` | Returns default task timeout in seconds. |
| `is_yolo_mode()` | `-> bool` | Returns whether permission-prompt bypass is enabled (safe default: `false`). |

#### Test-Only API (`#[cfg(test)]`)

| Function | Signature | Description |
|----------|-----------|-------------|
| `reset()` | `-> ()` | Clears the global config to `None`. Use at the start of each test. |
| `with_config(config)` | `(Config) -> ()` | Injects a specific `Config` instance for testing. |

### New Error Variants

```rust
ConfigError::AlreadyInitialized  // init() called twice without reset
ConfigError::NotInitialized      // get() called before init()
```

## Usage

### In Tests

Call `reset()` at the start of every test to ensure isolation, then use `with_config()` to inject known configuration:

```rust
#[test]
fn test_agent_lookup() {
    config::accessor::reset();
    config::accessor::with_config(make_test_config());

    let agent = config::get_agent_by_name("Builder");
    assert!(agent.is_some());
    assert_eq!(agent.unwrap().r#type, "opencode");
}
```

### In Production

The production API is unchanged. Call `init()` once at startup:

```rust
fn main() {
    config::init().expect("Failed to load configuration");
    // Use query functions freely:
    let server_addr = config::get_server_addr();
    let agents = config::get_agents_by_role("builder");
}
```

## Configuration

No new configuration options or environment variables were introduced. This feature is purely internal to the config accessor implementation.

## Migration Notes

### Return Types Changed from References to Owned Values

The previous `OnceLock`-based accessor returned references (`&Config`, `&AgentRegistration`) tied to the singleton's lifetime. The `RwLock`-based accessor returns **owned clones** because the lock guard cannot outlive the function scope.

| Before | After |
|--------|-------|
| `get() -> Option<&Config>` | `get() -> Option<Config>` |
| `get_agent_by_name() -> Option<&AgentRegistration>` | `get_agent_by_name() -> Option<AgentRegistration>` |
| `get_agents_by_role() -> Vec<&AgentRegistration>` | `get_agents_by_role() -> Vec<AgentRegistration>` |
| `get_server_addr() -> Option<&str>` | `get_server_addr() -> Option<String>` |

Callers that stored references to config data must now store owned values. In practice, this affects very little code since config lookups are typically used inline (e.g., `if let Some(addr) = get_server_addr()`).

### No New Dependencies

`std::sync::RwLock` is part of the Rust standard library. No new crates were added.
