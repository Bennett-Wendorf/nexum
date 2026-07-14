# Agent Pool Sidebar

## Overview

The Agent Pool sidebar is a reactive panel in the application layout that displays live agent registration data fetched from the backend. It replaces the previously hardcoded static display with dynamic counts, a utilization bar, and a grouped agent list — all populated from the `GET /api/v1/agents` REST endpoint.

## What Was Built

The sidebar, rendered inline within `Layout.svelte`, fetches agent registrations on mount and displays:

- **Active/idle counts** — dynamically computed from the fetched agent data
- **Utilization bar** — a visual progress bar reflecting the active/idle ratio
- **Grouped agent list** — agents organized by their `type` field (e.g., "coder", "reviewer"), each showing a status dot and name

The sidebar handles four rendering states: loading, error, empty (no agents), and populated (agents present).

## Technical Implementation

### File Modified

- **`web/src/components/Layout.svelte`** — The Agent Pool sidebar section (lines 64–103) was transformed from static hardcoded values to a reactive component with API integration.

### Reactive State Management

The implementation uses Svelte 5 runes for reactive state:

| Rune | Purpose |
|------|---------|
| `$state` | Mutable state for `agents`, `loading`, and `error` |
| `$derived` | Computed values: `totalAgents`, `idleCount`, `activeCount`, `utilizationPercent`, `agentsByType` |
| `onMount` | Lifecycle hook that triggers the initial API fetch |

### Key Functions and APIs

- **`listAgents()`** — Imported from `$lib/api`; calls `GET /api/v1/agents` and returns an `AgentsResponse` containing an array of `AgentRegistration` objects.
- **`AgentRegistration`** — TypeScript type from `$lib/types` with fields: `name`, `type`, `spawn_command`, `tool_permissions`, `timeout_seconds`.

### Derived Computations

```ts
const totalAgents = $derived(agents.length);
const idleCount = $derived(totalAgents);       // all agents idle (see notes)
const activeCount = $derived(0);               // active tracking not yet implemented
const utilizationPercent = $derived(totalAgents > 0 ? (activeCount / totalAgents) * 100 : 0);
const agentsByType = $derived(
  agents.reduce<Record<string, AgentRegistration[]>>((groups, agent) => {
    const type = agent.type;
    if (!groups[type]) groups[type] = [];
    groups[type].push(agent);
    return groups;
  }, {})
);
```

### Conditional Rendering States

The sidebar uses a `{#if}` block to handle four states:

| State | Condition | Display |
|-------|-----------|---------|
| **Loading** | `loading === true` | "Loading agents..." in muted text |
| **Error** | `error !== null` | "Failed to load agents" in accent-red |
| **Empty** | `agents.length === 0` | "No agents configured" in italic muted text |
| **Populated** | agents exist | Grouped agent list with type headers and status dots |

## Usage

The sidebar requires no configuration or manual interaction. It automatically fetches agent data when the `Layout` component mounts. No API calls need to be triggered from other components.

## Configuration

No configuration is required. The sidebar reads agent data directly from the backend `/api/v1/agents` endpoint, which reflects the agents defined in the nexum configuration.

## Important Notes

- **Active status tracking not yet implemented** — All agents are displayed as "idle" with a green status dot. The `activeCount` is hardcoded to `0`. When the backend adds active status tracking, the derivation logic and dot colors will need updating.
- **No polling** — Agents are fetched once on mount. If dynamic agent registration is added to the backend later, polling or WebSocket-based updates can be introduced.
- **Inline rendering** — The agent list is rendered directly inside `Layout.svelte` rather than as a separate component. This avoids unnecessary abstraction for the MVP. If the sidebar grows in complexity, it can be refactored into its own `AgentPool.svelte` component.
- **No new dependencies** — The implementation reuses the existing `listAgents()` API function, `AgentRegistration` types, and established Tailwind CSS theme variables. No new packages or CSS changes were needed.
