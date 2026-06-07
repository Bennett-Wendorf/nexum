# Plan: 004.3 - Status Machine Optimization

## Task Description
Replace HashMap-backed transition maps in PlanStateMachine and TaskStateMachine with compile-time match expressions.

## Objective
Eliminate all heap allocations associated with status machine construction by replacing HashMap with match expressions. Resulting structs should be zero-sized (ZST).

## Problem Statement
Both state machines store HashMap<K, Vec<V>> fields with hardcoded transition data rebuilt on every new() call. This wastes allocations at construction sites in scheduler, heartbeat monitor, and dependency resolver.

## Solution Approach
Replace HashMap with match expressions in can_transition. Both structs become ZSTs. Remove all_transitions() (unused). No external dependencies needed.

## Relevant Files
- `src/overlord/status_machine.rs` — primary target

## Step by Step Tasks

### 1. Refactor PlanStateMachine to ZST with match
- Remove HashMap field, make struct unit struct
- can_transition uses match on (from, to) tuple
- Remove all_transitions()
- Make can_transition and transition static methods

### 2. Refactor TaskStateMachine to ZST with match
- Same approach as PlanStateMachine

### 3. Remove HashMap import

### 4. Update documentation

### 5. Final validation
- cargo check, cargo test, cargo clippy
- Verify size_of::<PlanStateMachine>() == 0

## Acceptance Criteria
- Both state machines are ZSTs
- can_transition uses match expressions
- No HashMap in status_machine.rs
- All tests pass
