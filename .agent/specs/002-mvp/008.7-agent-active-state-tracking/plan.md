# Plan: 008.7 - Agent Active/Idle State Tracking

## Task Description
Fix the meaningless utilization bar in `Layout.svelte` by computing `activeCount` from actual running task data. Currently, `activeCount` is hardcoded to `0`, making the utilization bar always show 0%. This plan uses the existing `GET /running` endpoint to determine which agents are actively executing tasks, enabling accurate utilization tracking and per-agent active/idle status indicators.

## Objective
Transform the hardcoded `activeCount = 0` in `Layout.svelte` into a live value derived from `listRunningTasks()`, which returns running tasks with their `agent?.AgentLease.role` field. The plan will:
- Fetch running tasks periodically alongside the existing agent list fetch
- Compute `activeCount` from unique agent roles found in running tasks
- Mark individual agents as "active" or "idle" based on whether their type matches any running task's agent role
- Update the utilization bar to reflect real utilization
- Color-code agent status dots (amber for active, green for idle)

## Problem Statement
The utilization bar in `Layout.svelte` (line 43) computes `utilizationPercent` as `(activeCount / totalCount) * 100` where `activeCount` is hardcoded to `0` (line 42). This means:
- The bar always shows 0% regardless of actual system load
- Users cannot gauge agent capacity or busy-ness
- The "X active" / "Y idle" labels display "0 active" and "{N} idle" regardless of reality

The backend already has the data needed:
- `GET /running` returns `RunningTask[]` where each task has `agent?: AgentLease`
- `AgentLease` has a `role: string` field (e.g., "coder", "reviewer")
- `AgentRegistration` has a `type: string` field that corresponds to `AgentLease.role`
- The frontend already has `listRunningTasks()` in `api.ts` and the `RunningTask`/`AgentLease` types in `types.ts`

## Solution Approach
Reuse the existing `GET /running` endpoint (Option 1 from the code review). This requires zero backend changes.

### How It Works
1. On mount (and periodically via polling), fetch both `listAgents()` and `listRunningTasks()`
2. Extract the set of active agent roles from running tasks: `new Set(runningTasks.map(t => t.agent?.role).filter(Boolean))`
3. For each registered agent, check if its `type` matches any role in the active set
4. Compute `activeCount` as the number of agents whose type is in the active set
5. Compute `idleCount = totalCount - activeCount`
6. Update the utilization bar and per-agent status dots accordingly

### Key Design Decisions

| Decision | Rationale |
|----------|-----------|
| Reuse `GET /running` endpoint | Zero backend changes; endpoint already returns exactly the data needed |
| Poll every 5 seconds | Agent activity changes frequently; 5s balances freshness with API load |
| Match by `agent.type` === `AgentLease.role` | These fields are semantically equivalent (both represent agent type/role) |
| Count unique roles, not tasks | One agent type may run multiple tasks simultaneously; we want to know how many *agents* are active, not how many tasks |
| Amber dot for active, green for idle | Visually distinguishes busy agents from available ones |
| Abort controller for stale fetches | Prevents state updates from cancelled polls when component unmounts |

### Data Flow
```
GET /running → RunningTask[] → extract agent?.role → Set<activeRoles>
GET /agents  → AgentRegistration[] → check agent.type ∈ activeRoles → active/idle per agent
→ activeCount = agents.filter(a => activeRoles.has(a.type)).length
→ utilizationPercent = (activeCount / totalCount) * 100
```

## Relevant Files

### Existing Files to Modify
- `web/src/components/Layout.svelte` — Primary target; add running tasks fetch, compute active counts, update status dots

### Reference Files (read-only)
- `web/src/lib/api.ts` — Contains `listRunningTasks()` (line 153) and `listAgents()` (line 167)
- `web/src/lib/types.ts` — Contains `RunningTask`, `AgentLease`, `AgentRegistration` interfaces (lines 67-71, 113-121, 133-139)
- `src/api/execution.rs` — Backend `list_running_tasks` handler (reference for response structure)
- `src/api/types.rs` — Backend `RunningTaskResponse` and `AgentLeaseResponse` structs (reference for fields)
- `src/persistence/schema.rs` — Backend `AgentLease` struct (reference for `role` field semantics)

### New Files (none needed)
No new files are required. All changes are confined to `Layout.svelte`.

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: active-state-builder
  - Role: Implement running tasks polling, active count computation, and per-agent status indicators in Layout.svelte
  - Agent: builder

- **Validator**
  - Name: active-state-validator
  - Role: Verify active/idle counts are computed correctly, status dots reflect real state, and no regressions in sidebar
  - Agent: validator

- **Documenter**
  - Name: active-state-documenter
  - Role: Generate documentation for the agent active/idle tracking feature
  - Agent: documenter

## Step by Step Tasks

### 1. Add running tasks state and polling to Layout.svelte
- **Task ID**: add-running-tasks-state
- **Depends On**: none
- **Assigned To**: active-state-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/components/Layout.svelte` script section:
    - Add import: `import { listRunningTasks } from '$lib/api';`
    - Add import: `import type { RunningTask } from '$lib/types';`
    - Declare reactive state: `let runningTasks = $state<RunningTask[]>([]);`
    - Add a polling interval in `onMount` that fetches running tasks every 5 seconds:
      ```ts
      let pollInterval: ReturnType<typeof setInterval> | null = null;
      let fetchController: AbortController | null = null;

      async function fetchRunningTasks() {
        // Cancel any in-flight fetch
        if (fetchController) fetchController.abort();
        fetchController = new AbortController();
        try {
          const tasks = await listRunningTasks();
          if (destroyed) return;
          runningTasks = tasks;
        } catch (e) {
          // Silently ignore polling errors (network blips are expected)
          if (!destroyed && e instanceof DOMException && e.name !== 'AbortError') {
            console.warn('Failed to fetch running tasks:', e);
          }
        }
      }

      // Initial fetch
      await fetchRunningTasks();

      // Poll every 5 seconds
      pollInterval = setInterval(fetchRunningTasks, 5000);
      ```
    - In `onCleanup`, clear the interval and abort any in-flight fetch:
      ```ts
      onCleanup(() => {
        destroyed = true;
        if (pollInterval) clearInterval(pollInterval);
        if (fetchController) fetchController.abort();
      });
      ```
  - Move the existing `onCleanup(() => { destroyed = true; });` into the combined cleanup logic above
- **Acceptance Criteria**:
  - `Layout.svelte` imports `listRunningTasks` and `RunningTask` type
  - `runningTasks` reactive state is declared
  - Polling interval fetches running tasks every 5 seconds
  - Cleanup properly clears interval and aborts in-flight fetch
  - `destroyed` flag prevents state updates after unmount
  - No TypeScript compilation errors

### 2. Compute active agent count from running tasks
- **Task ID**: compute-active-count
- **Depends On**: add-running-tasks-state
- **Assigned To**: active-state-builder
- **Agent**: builder
- **Actions**:
  - Replace the hardcoded `activeCount = 0` (line 42) with a derived computation:
    ```ts
    // Extract unique agent roles from running tasks
    const activeRoles = $derived(
      new Set(
        runningTasks
          .map(t => t.agent?.role)
          .filter((role): role is string => Boolean(role))
      )
    );

    // Count agents whose type matches an active role
    const activeCount = $derived(
      agents.filter(agent => activeRoles.has(agent.type)).length
    );
    const idleCount = $derived(totalCount - activeCount);
    ```
  - Remove the old `const activeCount = 0;` line
  - The existing `utilizationPercent` derivation will automatically update since it depends on `activeCount`
  - Remove the old `idleCount` derivation if it exists (it was not in the original code, but the label says "idle" while showing `totalCount`)
- **Acceptance Criteria**:
  - `activeRoles` is a Set of unique agent roles from running tasks
  - `activeCount` counts agents whose `type` matches an active role
  - `idleCount = totalCount - activeCount`
  - `utilizationPercent` correctly reflects the new active/idle ratio
  - When no tasks are running, `activeCount = 0` and `utilizationPercent = 0`
  - When all agent types are running, `activeCount = totalCount` and `utilizationPercent = 100`

### 3. Add per-agent active status tracking
- **Task ID**: per-agent-status
- **Depends On**: compute-active-count
- **Assigned To**: active-state-builder
- **Agent**: builder
- **Actions**:
  - Add a helper to check if an agent is active:
    ```ts
    function isAgentActive(agent: AgentRegistration): boolean {
      return activeRoles.has(agent.type);
    }
    ```
  - In the agent list rendering (the `{#each groupAgents as agent}` block), update the status dot to reflect active/idle state:
    - Replace the static green dot:
      ```svelte
      <div class="w-1.5 h-1.5 rounded-full bg-accent-green flex-shrink-0"></div>
      ```
    - With a conditional dot:
      ```svelte
      <div class="w-1.5 h-1.5 rounded-full flex-shrink-0 {isAgentActive(agent) ? 'bg-accent-amber' : 'bg-accent-green'}"></div>
      ```
  - The amber color (`accent-amber`) should be used for active agents; green (`accent-green`) for idle agents
  - If `accent-amber` is not defined in the Tailwind config, use `bg-yellow-500` as a fallback or define it
- **Acceptance Criteria**:
  - Active agents show an amber status dot
  - Idle agents show a green status dot
  - Status dots update reactively when running tasks change
  - No visual regression for agents when no tasks are running (all green)

### 4. Fix the idle count label
- **Task ID**: fix-idle-label
- **Depends On**: compute-active-count
- **Assigned To**: active-state-builder
- **Agent**: builder
- **Actions**:
  - The current label reads `{totalCount} idle` (line 80) which is incorrect — it should show `idleCount`
  - Update the label:
    ```svelte
    <span><strong class="text-text-secondary">{idleCount}</strong> idle</span>
    ```
- **Acceptance Criteria**:
  - Idle count label shows the correct computed value
  - Active + Idle = Total always holds

### 5. Handle edge cases and error resilience
- **Task ID**: handle-edge-cases
- **Depends On**: fix-idle-label
- **Assigned To**: active-state-builder
- **Agent**: builder
- **Actions**:
  - Ensure the `listRunningTasks()` fetch failure does not break the sidebar (running tasks should default to empty array)
  - Verify that when `runningTasks` is empty, all agents show as idle (green dots)
  - Verify that when `agents` is empty but `runningTasks` has data, no errors occur (activeCount = 0)
  - Ensure the polling interval is properly cleaned up on component destroy
  - Ensure no memory leaks from accumulated fetch controllers
  - Test that rapid navigation away from and back to the layout component doesn't cause stale state updates
- **Acceptance Criteria**:
  - Sidebar renders correctly even when `GET /running` fails
  - Empty running tasks list shows all agents as idle
  - Empty agents list with running tasks shows 0 active, 0 idle, 0% utilization
  - Cleanup properly aborts in-flight fetches and clears intervals
  - No memory leaks or stale state updates on rapid navigation

### 6. Final Validation
- **Task ID**: validate-all
- **Depends On**: handle-edge-cases
- **Assigned To**: active-state-validator
- **Agent**: validator
- **Checks**:
  - Run `cd web && npm run check` — Svelte/TypeScript type checking must pass
  - Run `cd web && npm run build` — Vite build must succeed
  - Run `cd web && npm test` — Unit tests must pass (vitest)
  - Verify `Layout.svelte` imports `listRunningTasks` from `$lib/api`
  - Verify `Layout.svelte` has a polling interval for running tasks
  - Verify `activeCount` is a derived value, not hardcoded
  - Verify `idleCount` is computed as `totalCount - activeCount`
  - Verify status dots use conditional coloring (amber for active, green for idle)
  - Verify the idle count label shows `idleCount`, not `totalCount`
  - Verify cleanup properly aborts fetches and clears intervals
  - Run backend with running tasks and verify the UI updates correctly

### 7. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: active-state-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation changes
  - Document the agent active/idle tracking feature in `app_docs/`
  - Note the polling interval and data flow

## Acceptance Criteria
- `Layout.svelte` fetches running tasks from `/api/v1/running` on mount and polls every 5 seconds
- `activeCount` is derived from unique agent roles in running tasks (not hardcoded)
- `idleCount = totalCount - activeCount`
- Utilization bar reflects the real active/idle ratio
- Per-agent status dots show amber (active) or green (idle) based on running task data
- The idle count label shows `idleCount`, not `totalCount`
- Polling is properly cleaned up on component destroy (interval cleared, fetch aborted)
- Stale state updates are prevented via `destroyed` flag and abort controller
- Sidebar renders correctly when `GET /running` fails (graceful degradation)
- `npm run check` passes with no TypeScript errors
- `npm run build` succeeds
- `npm test` passes all tests
- No memory leaks from polling intervals or fetch controllers

## Validation Commands
- `cd web && npm run check` — Svelte/TypeScript type checking
- `cd web && npm run build` — Vite production build
- `cd web && npm test` — Run vitest unit tests
- `grep -n "activeCount = 0" web/src/components/Layout.svelte` — Verify hardcoded activeCount is removed (should find nothing)
- `grep -n "listRunningTasks" web/src/components/Layout.svelte` — Verify running tasks are fetched
- `grep -n "setInterval" web/src/components/Layout.svelte` — Verify polling is implemented

## Notes
- **Polling interval**: 5 seconds is a reasonable default for MVP. If agent activity changes very rapidly, this can be tuned down. If API load is a concern, it can be tuned up.
- **No backend changes**: This plan requires zero changes to the Rust backend. The `GET /running` endpoint already returns exactly the data needed.
- **Role-to-type mapping**: `AgentLease.role` (from running tasks) semantically matches `AgentRegistration.type` (from agent registrations). Both represent the agent type/role (e.g., "coder", "reviewer"). This mapping is implicit and relies on consistent naming in the configuration.
- **Multiple tasks per agent type**: If multiple tasks are running with the same agent role, `activeRoles` (a Set) ensures we count that agent type only once. This is correct because we want to know how many *agent types* are busy, not how many tasks are running.
- **Amber color**: If `accent-amber` is not defined in `tailwind.config.js`, the builder should either add it or use an existing Tailwind color like `yellow-500`.
- **Error handling**: Running tasks fetch failures are silently ignored in the polling loop to prevent UI disruption from transient network issues. The initial fetch on mount should still propagate errors if needed, but for the running tasks specifically, graceful degradation is preferred since the sidebar can still function with stale data.
