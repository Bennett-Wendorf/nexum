# Plan: 003 - Config Testability

## Task Description
Redesign the global configuration accessor (`src/config/accessor.rs`) to support test isolation. The current design uses `std::sync::OnceLock<Config>`, which accepts a value only once. After `init()` succeeds, any subsequent call returns an error, making integration testing impossible — you cannot reset configuration between tests. This plan replaces the singleton backing store with `std::sync::RwLock<Option<Config>>` and adds a `#[cfg(test)]` `reset()` function.

## Objective
Modify the config accessor so that:
- The global singleton remains thread-safe and works correctly in production
- Tests can reset the configuration between test cases
- The public API (`init()`, `get()`, query functions) remains unchanged for production callers
- A `#[cfg(test)]` `reset()` function enables test isolation
- A `#[cfg(test)]` `with_config(Config)` function enables dependency injection in tests

## Problem Statement
The `OnceLock` primitive in Rust accepts a value only once. After `init()` succeeds, any subsequent call returns an error. This makes integration testing difficult — you cannot reset configuration between tests. The current test suite avoids this by testing query logic directly on constructed `Config` objects rather than through the accessor singleton.

## Solution Approach
Replace `OnceLock<Config>` with `std::sync::RwLock<Option<Config>>` as the backing store for the global config singleton. This provides:

1. **Thread-safe reads** — Multiple threads can hold shared reads simultaneously
2. **Exclusive writes** — `init()` and `reset()` acquire a write lock
3. **Reset capability** — `#[cfg(test)] fn reset()` clears the `Option<Config>` to `None`
4. **Dependency injection** — `#[cfg(test)] fn with_config(Config)` sets a specific config for tests
5. **Production safety** — `init()` still enforces "initialize once" semantics by returning an error if already initialized

The public API remains identical for production callers. Test-only functions are gated behind `#[cfg(test)]`.

## Relevant Files

### Existing Files
- `src/config/accessor.rs` — Current OnceLock-based accessor (created by plan 002)
- `src/config/loader.rs` — ConfigError enum (needs new variants)
- `src/config/mod.rs` — Module re-exports
- `src/config/tests.rs` — Test file (needs accessor tests)

### Modified Files
- `src/config/accessor.rs` — Replace OnceLock with RwLock<Option<Config>>
- `src/config/loader.rs` — Add AlreadyInitialized and NotInitialized error variants
- `src/config/mod.rs` — Re-export test-only functions under #[cfg(test)]

## Step by Step Tasks

### 1. Implement RwLock-Based Config Accessor
- **Task ID**: config-accessor-rwlock
- **Depends On**: none
- **Assigned To**: builder
- **Agent**: builder
- **Actions**:
  - Replace `static CONFIG: OnceLock<Config>` with `static CONFIG: RwLock<Option<Config>> = RwLock::new(None)`
  - Update `init()`: acquire write lock, check if Some (return AlreadyInitialized error), load + validate, set to Some
  - Update `get()`: acquire read lock, return Some(config) or None
  - Update all query functions to use new RwLock pattern
  - Add `#[cfg(test)] pub fn reset()` — acquire write lock, set to None
  - Add `#[cfg(test)] pub fn with_config(config: Config)` — acquire write lock, set to Some(config)
- **Acceptance Criteria**:
  - `init()` succeeds on first call, returns AlreadyInitialized error on subsequent calls
  - `get()` returns valid config after init, None before
  - Query functions return correct results
  - `reset()` clears config (test-only)
  - `with_config()` injects specific config (test-only)

### 2. Add ConfigError Variants
- **Task ID**: config-error-variants
- **Depends On**: none
- **Assigned To**: builder
- **Agent**: builder
- **Actions**:
  - Add `AlreadyInitialized` variant to ConfigError
  - Add `NotInitialized` variant to ConfigError
  - Implement Display messages for new variants
- **Acceptance Criteria**:
  - ConfigError compiles with all variants
  - Display messages are descriptive

### 3. Write Accessor Unit Tests
- **Task ID**: config-accessor-tests
- **Depends On**: config-accessor-rwlock, config-error-variants
- **Assigned To**: builder
- **Agent**: builder
- **Actions**:
  - Add init tests: success, already initialized, get before init
  - Add reset tests: clears config, reinit after reset, reset between tests
  - Add with_config tests: injection, query functions with injected config
  - Add query function tests through the singleton (not direct Config construction)
  - Each test calls `reset()` at start for isolation
- **Acceptance Criteria**:
  - All tests pass with `cargo test --package nexum config`
  - Tests cover init, reset, with_config, query functions
  - Tests use reset() for isolation

### 4. Final Validation
- **Task ID**: validate-all
- **Depends On**: config-accessor-tests
- **Assigned To**: validator
- **Agent**: validator
- **Checks**:
  - `cargo check` succeeds
  - `cargo test --package nexum config` all tests pass
  - `cargo clippy --package nexum` no warnings in config module
  - `init()` returns AlreadyInitialized on second call
  - `get()` returns None before init
  - `reset()` clears config (test build)
  - `with_config()` injects config (test build)
  - `#[cfg(test)]` functions NOT available in release builds

## Acceptance Criteria
- `cargo check` succeeds with no errors
- `cargo test --package nexum config` passes all tests
- `cargo clippy --package nexum` produces no warnings in config module
- `init()` succeeds on first call, returns AlreadyInitialized on subsequent calls
- `get()` returns valid config after init, None before init
- `#[cfg(test)] reset()` clears config, enabling test isolation
- `#[cfg(test)] with_config(Config)` injects specific config for tests
- Query functions work correctly through the singleton
- Global singleton is thread-safe (RwLock provides concurrent reads, exclusive writes)
- `#[cfg(test)]` functions are NOT available in release builds

## Validation Commands
- `cargo check`
- `cargo test --package nexum config`
- `cargo clippy --package nexum`
- `cargo build --release` — verify release build excludes #[cfg(test)] functions
- `grep -r "cfg(test)" src/config/` — verify test-only functions are gated

## Notes
- This plan amends Task 4 of plan 002 (Configuration System).
- The RwLock approach keeps the existing singleton API for production code while enabling test isolation.
- No new production dependencies are needed — `std::sync::RwLock` is in the standard library.
- The `#[cfg(test)]` gate ensures reset functions are only available in test builds.
