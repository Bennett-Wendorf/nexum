# Plan: 002.6 - Test Serialization

## Task Description
Add test serialization to the config accessor tests in `src/config/tests.rs` to prevent intermittent failures caused by concurrent test threads interleaving operations on the global `RwLock<Option<Config>>` state. Rust runs test threads concurrently by default, so two tests can interleave: Test A calls `reset()`, Test B calls `with_config()`, Test A calls `get()` and gets Test B's config. This produces intermittent assertion failures that are extremely difficult to debug.

## Objective
Eliminate race conditions in config accessor tests by adding the `serial_test::serial` attribute to all 16 tests that interact with the global `CONFIG` singleton, ensuring they execute sequentially and cannot interleave with each other.

## Problem Statement
Every config accessor test calls `reset()` and `with_config()` on the same global `RwLock<Option<Config>>`. While `RwLock` prevents data corruption, it does not prevent logical interleaving. The interleaving scenario:

1. Test A: `reset()` — clears global config
2. Test B: `with_config(cfg)` — sets global config to B's config
3. Test A: `get()` — reads B's config instead of A's expected state
4. Test A: assertion fails intermittently

This produces flaky tests that pass most of the time but fail unpredictably under CI or when test execution order changes.

## Solution Approach
Add the `serial_test::serial` attribute macro to all tests that touch the global accessor. The `serial` attribute uses a global mutex to ensure tests with the same lock key execute one at a time. Tests that do NOT touch the global accessor (schema tests, validation tests, direct query logic tests) remain parallel for fast execution.

**Scope:** 16 tests need `#[serial]`, 22 tests remain parallel.

The 16 accessor tests requiring serialization:
- `test_init_success`
- `test_init_already_initialized`
- `test_get_before_init_returns_none`
- `test_get_after_with_config`
- `test_reset_clears_config`
- `test_reinit_after_reset`
- `test_with_config_injection`
- `test_with_config_overwrite`
- `test_singleton_query_agent_by_name`
- `test_singleton_query_agent_by_type`
- `test_singleton_query_agents_by_role`
- `test_singleton_server_addr`
- `test_singleton_max_parallel`
- `test_singleton_default_timeout`
- `test_singleton_yolo_mode`
- `test_singleton_query_returns_none_before_init`

## Relevant Files

### Existing Files
- `src/config/tests.rs` — Test file containing 39 tests; 16 touch the global accessor
- `src/config/accessor.rs` — Global config singleton (`RwLock<Option<Config>>`)
- `Cargo.toml` — Dev dependencies (needs `serial_test` added)

### Modified Files
- `src/config/tests.rs` — Add `#[serial]` to 16 accessor tests
- `Cargo.toml` — Add `serial_test = "3"` to `[dev-dependencies]`

## Step by Step Tasks

### 1. Add serial_test Dev Dependency
- **Task ID**: add-serial-test-dep
- **Depends On**: none
- **Agent**: builder
- **Actions**:
  - Add `serial_test = "3"` to `[dev-dependencies]` in `Cargo.toml`
  - Run `cargo check` to verify dependency resolves
- **Acceptance Criteria**:
  - `Cargo.toml` contains `serial_test = "3"` under `[dev-dependencies]`
  - `cargo check` succeeds with the new dependency

### 2. Add #[serial] to Accessor Tests
- **Task ID**: add-serial-attributes
- **Depends On**: add-serial-test-dep
- **Agent**: builder
- **Actions**:
  - Add `use serial_test::serial;` to the top of `src/config/tests.rs`
  - Add `#[serial]` attribute to each of the 16 accessor tests listed above
  - Use the default lock key (module-level) — all accessor tests will share one lock since they all live in the same test module
  - Verify that non-accessor tests (schema, validation, direct query logic) do NOT get `#[serial]`
- **Acceptance Criteria**:
  - All 16 accessor tests have `#[serial]` attribute
  - All 23 non-accessor tests remain parallel (no `#[serial]`)
  - Code compiles without errors
  - `cargo test --package nexum config` passes all tests

### 3. Stress Test for Race Conditions
- **Task ID**: stress-test-races
- **Depends On**: add-serial-attributes
- **Agent**: builder
- **Actions**:
  - Run `cargo test --package nexum config` multiple times (at least 5 runs) to verify no intermittent failures
  - Run with `--test-threads=4` to maximize concurrency pressure
- **Acceptance Criteria**:
  - All test runs pass consistently with no intermittent failures

### 4. Final Validation
- **Task ID**: validate-all
- **Depends On**: add-serial-attributes, stress-test-races
- **Agent**: validator
- **Checks**:
  - `cargo check` succeeds
  - `cargo test --package nexum config` all tests pass
  - `cargo clippy --package nexum` no warnings in config module
  - `#[serial]` is present on exactly 16 tests (the accessor tests)
  - `#[serial]` is absent from all non-accessor tests
  - `serial_test` appears only in `[dev-dependencies]`, not `[dependencies]`

## Acceptance Criteria
- `cargo check` succeeds with no errors
- `cargo test --package nexum config` passes all tests
- `cargo clippy --package nexum` produces no warnings in config module
- `#[serial]` attribute applied to exactly 16 accessor tests
- Non-accessor tests remain parallel for fast execution
- `serial_test` is a dev-dependency only (not in production dependencies)
- Tests pass consistently across multiple consecutive runs (no intermittent failures)

## Validation Commands
- `cargo check`
- `cargo test --package nexum config`
- `cargo test --package nexum config --test-threads=4`
- `cargo clippy --package nexum`
- `cargo build --release` — verify release build is unaffected
- `grep -c "#\[serial\]" src/config/tests.rs` — should return 16

## Notes
- This plan addresses the code review finding: "Tests share global mutable state without serialization."
- The `serial_test` crate is the simplest and most targeted fix.
- Alternative approaches considered but rejected: per-test config injection (needs substantial refactoring), thread-local storage (breaks production singleton), `--test-threads=1` (slows all tests).
- The `serial_test` dependency is dev-only, zero impact on production.
- This plan depends on plan 002.5 (Config Testability) being completed first.
