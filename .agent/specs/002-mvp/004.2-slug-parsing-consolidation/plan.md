# Plan: 004.2 - Slug Parsing Consolidation

## Task Description
Consolidate duplicate slug parsing logic across overlord modules into shared functions in the persistence layer.

## Objective
Eliminate 7 instances of duplicate slug parsing code by providing shared `parse_task_slug` and `parse_plan_slug` functions.

## Problem Statement
`parse_task_slug` and `parse_plan_slug` are implemented identically in dependency_resolver.rs, heartbeat_monitor.rs, and duplicated inline in scheduler.rs at 3 locations. Any change to the slug format requires updates in 4+ places.

## Solution Approach
Add `parse_task_slug` and `parse_plan_slug` to `persistence/directory.rs` alongside the existing `slugify` function. Re-export from `persistence/mod.rs`. Import from `crate::persistence` in all overlord modules.

## Relevant Files
- `src/persistence/directory.rs` — add shared functions
- `src/persistence/mod.rs` — re-export
- `src/overlord/dependency_resolver.rs` — remove local parse_task_slug, import shared
- `src/overlord/heartbeat_monitor.rs` — remove local parse_plan_slug and parse_task_slug, import shared
- `src/overlord/scheduler.rs` — replace inline parsing with shared functions

## Step by Step Tasks

### 1. Add shared functions to persistence/directory.rs
- `pub fn parse_plan_slug(slug: &str) -> Option<(&str, &str)>` — returns (plan_id, plan_name)
- `pub fn parse_task_slug(slug: &str) -> Option<(&str, &str)>` — returns (task_id, task_name)
- Use same logic as existing implementations

### 2. Re-export from persistence/mod.rs

### 3. Migrate callers
- dependency_resolver.rs: remove local function, import from crate::persistence
- heartbeat_monitor.rs: remove both local functions, import from crate::persistence
- scheduler.rs: replace all 4 inline occurrences with function calls

### 4. Update tests if needed

### 5. Final validation
- cargo check, cargo test, cargo clippy

## Acceptance Criteria
- parse_plan_slug and parse_task_slug in persistence/directory.rs
- Re-exported from persistence/mod.rs
- No duplicate slug parsing in overlord modules
- All tests pass
