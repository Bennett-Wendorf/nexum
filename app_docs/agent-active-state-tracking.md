# Agent Active/Idle State Tracking

## Overview

The Agent Pool sidebar in the Nexum frontend displays registered agents and a utilization bar. Previously, `activeCount` was hardcoded to `0`, so the utilization bar always showed 0% and the "active" label always read "0 active" regardless of actual system load.

This feature replaces the hardcoded value with live data fetched from the backend. By polling the `GET /running` endpoint, the sidebar now accurately reflects which agents are actively executing tasks and which are idle, providing users with real-time visibility into agent capacity.

## What Was Built

A polling-based mechanism in `Layout.svelte` that:

- Fetches running tasks from the backend every 5 seconds
- Extracts unique agent roles from running tasks to determine which agent types are active
- Computes `activeCount` by matching agent `type` against active roles
- Displays per-agent status indicators — amber dot for active agents, green for idle
- Updates the utilization bar to reflect the real active/idle ratio

No backend changes were required; the existing `GET /running` endpoint already provides the necessary data.

## Data Flow

```
GET /running → RunningTask[] → extract agent?.role → Set<activeRoles>
GET /agents  → AgentRegistration[] → check agent.type ∈ activeRoles → active/idle per agent
→ activeCount = agents.filter(a => activeRoles.has(a.type)).length
→ idleCount = totalCount - activeCount
→ utilizationPercent = (activeCount / totalCount) * 100
```

1. `listRunningTasks()` fetches `RunningTask[]` from `GET /api/v1/running`
2. Each task has an optional `agent` field (`AgentLease`) with a `role` string (e.g., `"coder"`)
3. Active roles are collected into a `Set` to count each agent type only once
4. `listAgents()` fetches `AgentRegistration[]` from `GET /api/v1/agents`
5. Each agent has a `type` field that corresponds to the `role` field from running tasks
6. Agents whose `type` exists in the active roles set are counted as active
7. Derived values (`idleCount`, `utilizationPercent`) update reactively via Svelte 5 `$derived` runes

## Polling Mechanism

- **Interval**: 5 seconds — balances freshness against API load
- **AbortController**: Each poll cycle creates a new `AbortController`. Before starting a new fetch, any in-flight fetch is aborted, preventing stale responses from overwriting fresh data
- **Cleanup on unmount**: The `onMount` callback returns a cleanup function that:
  - Sets `destroyed = true` to prevent state updates after unmount
  - Clears the `setInterval` via `clearInterval`
  - Aborts any in-flight fetch via `fetchController.abort()`
- **Stale update guard**: The `destroyed` flag is checked after each async fetch completes. If the component has unmounted, the state update is skipped

## Per-Agent Status Indicators

Each agent in the sidebar has a small status dot:

| State | Color | CSS Class |
|-------|-------|-----------|
| Active | Amber/Yellow | `bg-accent-yellow` |
| Idle | Green | `bg-accent-green` |

The `isAgentActive(agent)` helper checks whether the agent's `type` exists in the `activeRoles` set. The dot color updates reactively when running tasks change.

## Edge Cases Handled

| Edge Case | Handling |
|-----------|----------|
| `GET /running` fails | Error is silently ignored during polling; `runningTasks` retains its previous value. Non-abort `DOMException` errors are logged to console. |
| No running tasks | `activeRoles` is an empty `Set`; all agents show as idle (green dots); utilization is 0% |
| No agents registered | `totalCount = 0`; `activeCount = 0`; `utilizationPercent = 0`; sidebar shows "No agents configured" |
| Running tasks exist but no agents | `activeCount = 0` (no agents to match against); no errors |
| Multiple tasks per agent type | `Set` deduplication ensures each agent type is counted only once |
| Rapid navigation/unmount | AbortController cancels in-flight fetch; `destroyed` flag prevents stale state writes; interval is cleared |
| Memory leaks | Each poll cycle reuses the same `fetchController` variable (aborting the previous one); interval is cleared on cleanup |

## API Endpoints Used

| Endpoint | Method | Function | Purpose |
|----------|--------|----------|---------|
| `/api/v1/running` | `GET` | `listRunningTasks()` | Returns `RunningTask[]` with optional `agent` (`AgentLease`) field |
| `/api/v1/agents` | `GET` | `listAgents()` | Returns `AgentsResponse` containing `AgentRegistration[]` |

## Technical Implementation

### Files Modified

- **`web/src/components/Layout.svelte`** — Primary implementation file

### Key Changes in `Layout.svelte`

| Line(s) | Description |
|---------|-------------|
| 5 | Added `listRunningTasks` import from `$lib/api` |
| 7 | Added `RunningTask` type import from `$lib/types` |
| 16 | Added `runningTasks` reactive state: `$state<RunningTask[]>([])` |
| 42–72 | Polling setup: `fetchRunningTasks()` function, `setInterval(5000)`, cleanup return |
| 78–84 | `$derived` `activeRoles` — `Set` of unique agent roles from running tasks |
| 87–90 | `$derived` `activeCount`, `idleCount` — computed from agents and active roles |
| 92 | `$derived` `utilizationPercent` — updated to use new `activeCount` |
| 102–104 | `isAgentActive()` helper function |
| 133 | Idle label fixed to show `{idleCount}` instead of `{totalCount}` |
| 157 | Status dot uses conditional class: `isAgentActive(agent) ? 'bg-accent-yellow' : 'bg-accent-green'` |

### Dependencies

No new dependencies were added. The feature reuses existing API functions and types.

## Configuration

No configuration is required. The polling interval (5 seconds) is hardcoded in the implementation. To adjust it, modify the `setInterval` call in `Layout.svelte` (line 65).
