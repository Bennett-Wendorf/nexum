# Plan: 008.6 - Agent Pool Sidebar Live Data

## Task Description
Wire up the Agent Pool sidebar in `Layout.svelte` to dynamically display registered agent data fetched from the `GET /api/v1/agents` REST endpoint. Currently, the sidebar shows hardcoded "0 active", "0 idle", and "No agents configured" — it never makes an API request. This plan makes the sidebar fetch agent registrations on mount, display live counts, and render each agent with name, type, and status indicator.

## Objective
Transform the hardcoded Agent Pool sidebar in `web/src/components/Layout.svelte` (lines 30-50) into a reactive component that:
- Fetches agent registrations from `/api/v1/agents` on mount
- Displays dynamic active/idle agent counts (all agents are "idle" for now since active status tracking is not yet implemented)
- Renders each agent with name, type label, and a status dot indicator
- Handles edge cases: empty agent list, API fetch errors, loading state
- Matches the existing dark GitHub-like theme using the established Tailwind CSS variables

## Problem Statement
The Agent Pool sidebar in `Layout.svelte` is entirely static. It displays:
- Hardcoded "0 active" and "0 idle" counts
- A 0% utilization bar
- "No agents configured" placeholder text

This means the sidebar provides no useful information regardless of how many agents are actually registered in the nexum configuration. Users cannot see which agents are available, what types they are, or how many are registered.

The `GET /api/v1/agents` endpoint already exists and returns an `AgentsResponse` containing an array of `AgentRegistrationResponse` objects with `name`, `type`, `spawn_command`, `tool_permissions`, and `timeout_seconds`. The frontend API client (`web/src/lib/api.ts`) already has a `listAgents()` function, and the TypeScript types (`AgentRegistration`, `AgentsResponse`) are already defined in `web/src/lib/types.ts`.

## Solution Approach
Add reactive state and an `onMount` lifecycle hook to `Layout.svelte` that:
1. Fetches agents via the existing `listAgents()` API function
2. Computes derived counts (total, active, idle)
3. Renders the agent list with proper styling

Since active status tracking is not yet implemented on the backend, all agents will be classified as "idle". The active count will be 0. This is consistent with the current hardcoded behavior and can be updated later when active tracking is added.

The sidebar will render agents grouped by their `type` field (e.g., "coder", "reviewer", "planner"), with each agent showing its name, type badge, and a status dot (green for idle).

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Fetch on mount only (no polling) | MVP scope; polling can be added later. Agents are configured at startup and don't change dynamically. |
| All agents shown as "idle" | Active status tracking is not yet implemented on the backend. This matches the current hardcoded "0 active" behavior. |
| Inline rendering in Layout.svelte | The sidebar is small (220px wide) and the agent list is a core layout feature. Creating a separate component adds unnecessary abstraction for the MVP. |
| Group agents by type | Agents of the same type are logically related; grouping reduces visual clutter. |
| Use existing CSS variables | Maintains consistency with the dark GitHub-like theme used throughout the app. |

## Relevant Files

### Existing Files to Modify
- `web/src/components/Layout.svelte` — Primary target; add reactive state, API fetch, and dynamic rendering to the Agent Pool sidebar section (lines 30-50)

### Reference Files (read-only)
- `web/src/lib/api.ts` — Contains `listAgents()` function already defined (line 167-169)
- `web/src/lib/types.ts` — Contains `AgentRegistration` and `AgentsResponse` interfaces (lines 133-144)
- `web/src/lib/errorUtils.ts` — Error handling utility with auto-clear timeout
- `web/tailwind.config.js` — CSS variable definitions for dark theme colors
- `src/api/config.rs` — Backend `list_agents` handler (reference for response structure)
- `src/api/types.rs` — Backend `AgentRegistrationResponse` struct (reference for fields)

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: agent-pool-builder
  - Role: Implement the reactive Agent Pool sidebar with API integration, agent list rendering, and error handling
  - Agent: builder

- **Validator**
  - Name: agent-pool-validator
  - Role: Verify the sidebar renders correctly, handles edge cases, and matches the design theme
  - Agent: validator

- **Documenter**
  - Name: agent-pool-documenter
  - Role: Generate documentation for the completed Agent Pool sidebar feature
  - Agent: documenter

## Step by Step Tasks

### 1. Add reactive state and API fetch to Layout.svelte
- **Task ID**: add-agent-state-and-fetch
- **Depends On**: none
- **Assigned To**: agent-pool-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/components/Layout.svelte` script section (inside existing `<script lang="ts">` block):
    - Add imports: `import { onMount } from 'svelte';` and `import { listAgents } from '$lib/api';` and `import type { AgentRegistration } from '$lib/types';`
    - Declare reactive state variables:
      ```ts
      let agents = $state<AgentRegistration[]>([]);
      let loading = $state(false);
      let error = $state<string | null>(null);
      ```
    - Add `onMount` hook that calls `listAgents()` and populates `agents`:
      ```ts
      onMount(async () => {
        loading = true;
        try {
          const response = await listAgents();
          agents = response.agents;
        } catch (e) {
          if (e instanceof Error) {
            error = e.message;
          }
        } finally {
          loading = false;
        }
      });
      ```
  - Add derived computed values:
    ```ts
    const totalAgents = $derived(agents.length);
    const idleCount = $derived(totalAgents); // all agents are idle for now
    const activeCount = $derived(0); // active tracking not yet implemented
    const utilizationPercent = $derived(totalAgents > 0 ? (activeCount / totalAgents) * 100 : 0);
    ```
  - Group agents by type for rendering:
    ```ts
    const agentsByType = $derived(
      agents.reduce<Record<string, AgentRegistration[]>>((groups, agent) => {
        const type = agent.type;
        if (!groups[type]) groups[type] = [];
        groups[type].push(agent);
        return groups;
      }, {})
    );
    ```
- **Acceptance Criteria**:
  - `Layout.svelte` imports `listAgents` and `AgentRegistration` type
  - Reactive state variables (`agents`, `loading`, `error`) are declared
  - `onMount` fetches agents from API and populates state
  - Derived values (`totalAgents`, `idleCount`, `activeCount`, `utilizationPercent`, `agentsByType`) are computed correctly
  - No TypeScript compilation errors

### 2. Render dynamic counts and utilization bar
- **Task ID**: render-dynamic-counts
- **Depends On**: add-agent-state-and-fetch
- **Assigned To**: agent-pool-builder
- **Agent**: builder
- **Actions**:
  - Replace the hardcoded counts section (current lines 35-42) with dynamic rendering:
    ```svelte
    <div class="p-2.5 border-b border-border-default">
      <div class="flex justify-between text-[11px] text-text-muted">
        <span><strong class="text-text-secondary">{activeCount}</strong> active</span>
        <span><strong class="text-text-secondary">{idleCount}</strong> idle</span>
      </div>
      <div class="h-1.5 bg-border-default rounded-sm overflow-hidden mt-1.5">
        <div class="h-full bg-accent-green rounded-sm" style="width: {utilizationPercent}%"></div>
      </div>
    </div>
    ```
  - The utilization bar uses `accent-green` for the active portion, matching the existing design
- **Acceptance Criteria**:
  - Active and idle counts display the computed values instead of hardcoded "0"
  - Utilization bar width reflects the derived `utilizationPercent`
  - Styling matches existing theme (text-text-secondary for numbers, text-text-muted for labels)

### 3. Render agent list grouped by type
- **Task ID**: render-agent-list
- **Depends On**: render-dynamic-counts
- **Assigned To**: agent-pool-builder
- **Agent**: builder
- **Actions**:
  - Replace the hardcoded "Groups" section and "No agents configured" text (current lines 44-49) with conditional rendering:
    ```svelte
    {#if loading}
      <div class="px-3.5 py-4 text-xs text-text-muted">Loading agents...</div>
    {:else if error}
      <div class="px-3.5 py-4 text-xs text-accent-red">Failed to load agents</div>
    {:else if agents.length === 0}
      <div class="px-3.5 py-4 text-xs text-text-muted italic">No agents configured</div>
    {:else}
      <div class="text-[10px] uppercase tracking-widest text-text-faint font-semibold px-3.5 py-2.5">
        Groups
      </div>
      {#each Object.entries(agentsByType) as [type, groupAgents]}
        <div class="mb-2">
          <!-- Type group header -->
          <div class="flex items-center gap-1.5 px-3 pb-1">
            <span class="text-[10px] uppercase tracking-wider text-text-faint font-semibold">{type}</span>
            <span class="text-[10px] text-text-faint">({groupAgents.length})</span>
          </div>
          <!-- Agent items -->
          {#each groupAgents as agent}
            <div class="flex items-center gap-2 px-3.5 py-1.5 rounded-md hover:bg-border-muted transition-colors cursor-default">
              <!-- Status dot (green = idle) -->
              <div class="w-1.5 h-1.5 rounded-full bg-accent-green flex-shrink-0"></div>
              <!-- Agent name -->
              <span class="text-xs text-text-secondary truncate" title="{agent.name}">{agent.name}</span>
            </div>
          {/each}
        </div>
      {/each}
    {/if}
    ```
  - Each agent shows a green status dot (idle), the agent name, and truncates long names with `truncate`
  - Groups are displayed with a type header showing the type name and count
- **Acceptance Criteria**:
  - Loading state shows "Loading agents..." message
  - Error state shows "Failed to load agents" in accent-red
  - Empty state shows "No agents configured" in italic text-text-muted
  - When agents exist, they are grouped by type with type headers
  - Each agent row shows a green status dot and the agent name
  - Agent rows have hover highlight (bg-border-muted)
  - Long agent names are truncated with a tooltip

### 4. Handle edge cases and polish
- **Task ID**: handle-edge-cases
- **Depends On**: render-agent-list
- **Assigned To**: agent-pool-builder
- **Agent**: builder
- **Actions**:
  - Ensure the sidebar `overflow-y-auto` still works when many agents are registered
  - Verify that the sidebar width (220px) accommodates the agent list layout without horizontal overflow
  - Add `title` attribute to agent names for tooltip on hover (already included in Task 3)
  - Ensure the status dot is `flex-shrink-0` so it doesn't get squished
  - Verify that the utilization bar shows 0% when there are no agents (no division by zero)
  - Test that the layout works with the `RouterLink` components in the icon sidebar (no interference)
- **Acceptance Criteria**:
  - Sidebar scrolls vertically when agent list exceeds available space
  - No horizontal overflow in the 220px sidebar
  - Utilization bar shows 0% (not NaN or Infinity) when no agents are configured
  - Status dots are properly sized and don't shrink
  - No layout interference with the icon sidebar or main content area

### 5. Final Validation
- **Task ID**: validate-all
- **Depends On**: handle-edge-cases
- **Assigned To**: agent-pool-validator
- **Agent**: validator
- **Checks**:
  - Run `cd web && npm run check` — Svelte/TypeScript type checking must pass
  - Run `cd web && npm run build` — Vite build must succeed
  - Run `cd web && npm test` — Unit tests must pass (vitest)
  - Verify `Layout.svelte` has no hardcoded "0 active" or "0 idle" text
  - Verify `Layout.svelte` imports `listAgents` from `$lib/api`
  - Verify `Layout.svelte` has `onMount` that fetches agents
  - Verify the sidebar renders loading, error, empty, and populated states correctly
  - Verify CSS classes use existing theme variables (text-text-secondary, text-text-muted, accent-green, etc.)
  - Verify no new CSS files or Tailwind config changes are needed
  - Verify the utilization bar percentage is computed correctly (0% when no agents, 0% when all idle)

### 6. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: agent-pool-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation changes
  - Document the Agent Pool sidebar feature in `app_docs/`
  - Note that active status tracking is not yet implemented (all agents shown as idle)

## Acceptance Criteria
- `Layout.svelte` fetches agents from `/api/v1/agents` on mount
- Active and idle counts are dynamically computed from fetched data
- Utilization bar reflects the active/idle ratio
- Agents are grouped by type with type headers
- Each agent displays a status dot (green for idle) and name
- Loading state shows "Loading agents..." message
- Error state shows "Failed to load agents" in red
- Empty state shows "No agents configured" in italic muted text
- All styling uses existing Tailwind CSS theme variables
- `npm run check` passes with no TypeScript errors
- `npm run build` succeeds
- `npm test` passes all tests
- No hardcoded values remain in the Agent Pool sidebar section
- Sidebar scrolls properly with many agents (no horizontal overflow)

## Validation Commands
- `cd web && npm run check` — Svelte/TypeScript type checking
- `cd web && npm run build` — Vite production build
- `cd web && npm test` — Run vitest unit tests
- `grep -n "0 active\|0 idle\|No agents configured" web/src/components/Layout.svelte` — Verify no hardcoded values remain (should only find the empty state fallback text)

## Notes
- **Active status tracking**: The backend does not yet track which agents are actively executing tasks. All agents are classified as "idle" in this implementation. When active status tracking is added to the backend (e.g., via a new endpoint or field in `AgentRegistrationResponse`), the `activeCount` derivation and status dot colors can be updated accordingly.
- **No new components**: The implementation is kept inline in `Layout.svelte` rather than extracting a separate `AgentPool.svelte` component. This avoids unnecessary abstraction for the MVP. If the sidebar grows in complexity later, it can be refactored into its own component.
- **No polling**: Agents are fetched once on mount. Since agent registrations are configured at startup and don't change dynamically, polling is unnecessary. If dynamic agent registration is added later, polling or WebSocket-based updates can be introduced.
- **API client already exists**: The `listAgents()` function in `web/src/lib/api.ts` and the `AgentRegistration`/`AgentsResponse` types in `web/src/lib/types.ts` are already defined. No changes to the API client or types are needed.
- **CSS theme consistency**: All styling uses existing Tailwind CSS classes and theme variables defined in `web/tailwind.config.js`. No new CSS or config changes are required.
