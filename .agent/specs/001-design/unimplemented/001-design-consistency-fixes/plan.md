# Plan: 001 - Design Document Consistency Fixes

## Task Description
Fix 8 identified consistency issues across 12 design documents in `/home/bennett/files/programming/nexum/design/`. The documents define the architecture for Nexum, an AI agent orchestration system. Several contradictions, ambiguities, and undefined references exist between documents that need resolution before the design can serve as a reliable specification.

## Objective
Produce a set of minimal, targeted edits across the design documents so that:
- All documents reference the same canonical status sets
- All hierarchy levels are consistently defined
- Agent responsibilities are clearly scoped and consolidated
- All referenced concepts are defined
- Cross-document references are coherent

## Problem Statement
The 12 design documents contain 8 consistency issues: status mismatches, undefined hierarchy levels (epic), ambiguous agent scopes, scattered responsibilities, and undefined concepts (execution plans, context.md). These inconsistencies make the design unreliable as a specification and will cause confusion during implementation.

## Solution Approach
Apply minimal, surgical edits to each affected document. Each fix targets only the inconsistency at hand — no redesigns, no new concepts introduced unless strictly necessary for resolution. Fixes are ordered so that earlier changes (e.g., status unification) don't get overwritten by later ones.

## Relevant Files

### Existing Files (all in `/home/bennett/files/programming/nexum/design/`)
| File | Issue(s) Affected |
|------|-------------------|
| `work-statuses.md` | Issue 1, Issue 3, Issue 6 |
| `persistence.md` | Issue 1, Issue 2, Issue 3, Issue 4, Issue 5, Issue 6, Issue 8 |
| `agent-roles.md` | Issue 2, Issue 4, Issue 5 |
| `orchestration-levels.md` | Issue 2 |
| `agent-teams.md` | Issue 4 |
| `resource-constraints.md` | Issue 6 |
| `unit-of-work.md` | Issue 7 |

## Team Orchestration

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: design-editor
  - Role: Apply targeted edits to design documents per the plan
  - Agent: builder

- **Validator**
  - Name: consistency-checker
  - Role: Verify all 8 issues are resolved and no new inconsistencies were introduced
  - Agent: validator

- **Documenter**
  - Name: change-log-writer
  - Role: Generate a changelog summarizing all edits made
  - Agent: documenter

## Step by Step Tasks

### 1. Clarify Plan vs. Task Status Boundary (Issue 3)
- **Task ID**: clarify-status-boundary
- **Depends On**: none
- **Assigned To**: design-editor
- **Agent**: builder
- **Actions**:
  - In `work-statuses.md`: Add a clear heading structure that separates "Plan-level statuses" from "Task-level statuses". Currently both pre-planning and post-planning share `Planning` and `Reviewing` — explicitly state that pre-planning statuses apply to plans and post-planning statuses apply to tasks.
  - In `work-statuses.md`: Add a note under the post-planning section: "These statuses apply to individual tasks within a plan. Plan-level statuses are defined separately above."
  - In `persistence.md` line 60 (task.md template): Verify the task status set matches the post-planning set from work-statuses.md (will be finalized in Task 2).
  - In `persistence.md` line 106 (plan.md template): Verify the plan status set matches the pre-planning set from work-statuses.md.
  - In `persistence.md`: Add a section "Status machines" that explicitly states: "Plans and tasks have separate status machines. Plans: `[plan statuses]`. Tasks: `[task statuses]`."
- **Acceptance Criteria**:
  - `work-statuses.md` has clear separation between plan-level and task-level statuses
  - `persistence.md` explicitly states that plans and tasks have separate status machines
  - No ambiguity remains about which statuses apply to which entity

### 2. Unify Status Sets (Issue 1)
- **Task ID**: unify-status-sets
- **Depends On**: clarify-status-boundary
- **Assigned To**: design-editor
- **Agent**: builder
- **Actions**:
  - **Decision**: Adopt the `work-statuses.md` post-planning set as canonical for tasks since it's more complete. The canonical task status set becomes: `Backlog, Queued, Running, Reviewing, Waiting manual review, Merge-queue, Abandoned, Completed`.
  - In `persistence.md` line 60 (task.md template): Replace `queued | planning | running | reviewing | completed | abandoned` with `backlog | queued | running | reviewing | waiting-manual-review | merge-queue | abandoned | completed`.
  - In `persistence.md` line 83 (status.json schema): Update the status value from `"running"` to use the new canonical set. Add a comment: `// Status values: backlog | queued | running | reviewing | waiting-manual-review | merge-queue | abandoned | completed`
  - In `persistence.md` line 132 (task_status_map in execution.json): Update example values to use canonical statuses.
  - In `work-statuses.md`: Ensure capitalization is consistent. Change all status names to lowercase with hyphens (e.g., `waiting manual review` → `waiting-manual-review`, `Merge-queue` → `merge-queue`) for consistency with the JSON/markdown convention.
  - In `persistence.md` "Status machines" section (added in Task 1): Update to reflect the unified canonical sets.
- **Acceptance Criteria**:
  - Task status set is identical in `persistence.md` (task.md template, status.json, execution.json) and `work-statuses.md` (post-planning)
  - `waiting-manual-review` and `merge-queue` appear in all relevant locations
  - Capitalization and formatting are consistent (lowercase, hyphenated)

### 3. Remove Epic Reference (Issue 2)
- **Task ID**: remove-epic-reference
- **Depends On**: none
- **Assigned To**: design-editor
- **Agent**: builder
- **Actions**:
  - In `agent-roles.md` line 10: Replace "Invoked after epic completion" with "Invoked after plan completion" (since plan completion already exists on line 9, remove the duplicate entirely — line 9 already says "Invoked after plan completion").
  - Specifically: Delete line 10 (`Invoked after epic completion`) since line 9 already covers plan-level review invocation.
  - In `orchestration-levels.md`: No changes needed — it already correctly defines the hierarchy without epic.
  - In `persistence.md` line 211: No changes needed — it already states "No epic layer."
- **Acceptance Criteria**:
  - No document references "epic" as a hierarchy level
  - `agent-roles.md` line 10 is removed (duplicate of line 9)
  - `orchestration-levels.md` and `persistence.md` remain unchanged (already correct)

### 4. Remove Undefined "Execution Plans" Reference (Issue 7)
- **Task ID**: remove-execution-plans
- **Depends On**: none
- **Assigned To**: design-editor
- **Agent**: builder
- **Actions**:
  - In `unit-of-work.md` lines 12-13: Replace the section:
    - Remove: `## Execution plans?` and `- For parallelization?`
    - Replace with: `## Parallelization` and `- Handled via concurrency limits in execution.json (see persistence.md). No separate execution plan concept is needed — the dependency graph and concurrency.max_parallel in execution.json govern parallel execution.`
  - This connects the concept to the existing mechanism in `persistence.md` without introducing a new undefined concept.
- **Acceptance Criteria**:
  - `unit-of-work.md` no longer references "execution plans" as an undefined concept
  - Parallelization is explicitly connected to the mechanism described in `persistence.md`

### 5. Clarify Researcher Scope (Issue 4)
- **Task ID**: clarify-researcher-scope
- **Depends On**: none
- **Assigned To**: design-editor
- **Agent**: builder
- **Actions**:
  - In `agent-roles.md` line 15-16: After "Responsible for pre-planning research", add: "— Attached to a plan. One researcher per plan, invoked before the planner begins."
  - In `agent-teams.md` line 3: After "1 researcher", add a note: "(one per plan)". Change: `- 1 planner, 1 researcher (one per plan), 1+ builders, 1+ reviewers`
  - In `persistence.md` line 117: The "Research findings" section in plan.md already implies plan-level attachment. Add a clarifying comment: `## Research findings` → `## Research findings {#researcher output, one researcher per plan}`
- **Acceptance Criteria**:
  - Researcher is explicitly defined as plan-level (one per plan)
  - `agent-roles.md`, `agent-teams.md`, and `persistence.md` all agree on the scope
  - No ambiguity remains about whether researcher is per-plan, per-branch, or per-repo

### 6. Consolidate Overlord Responsibilities (Issue 5)
- **Task ID**: consolidate-overlord
- **Depends On**: none
- **Assigned To**: design-editor
- **Agent**: builder
- **Actions**:
  - In `agent-roles.md` lines 17-20: Expand the Overlord description to include all responsibilities currently scattered across documents. Replace the current description with:
    ```
  - Overlord
      - Main interaction with the user
      - Orchestrates all agent activity:
        - Creates tasks and assigns agents (see agent-teams.md)
        - Transitions task status (see persistence.md status.json transitions)
        - Generates stable IDs for plans and tasks (see persistence.md ID scheme)
        - Merges task branches into plan branch after task completion (see persistence.md branch strategy)
        - Detects stale heartbeats and re-queues orphaned tasks (see persistence.md recovery)
      - Cannot complete tasks directly — delegates to builders
      - **Future: Scratchpad agent** — A separate on-demand agent the user can invoke to fix random things, research questions, or handle ad-hoc tasks outside the main workflow. Likely spawned by or alongside the Overlord.
    ```
  - In `persistence.md`: No edits to lines 91, 151, 204, or 238 — they remain as technical implementation details. Add a cross-reference note at the top of the "ID scheme" section: "IDs are generated by the Overlord (see agent-roles.md)."
  - In `persistence.md`: Add a cross-reference note in the "Recovery" section: "Stale lease detection is performed by the Overlord (see agent-roles.md)."
- **Acceptance Criteria**:
  - `agent-roles.md` contains a complete, single listing of all Overlord responsibilities
  - `persistence.md` sections reference agent-roles.md for Overlord responsibilities
  - No Overlord responsibility is mentioned in one document but missing from agent-roles.md

### 7. Connect Concurrency Limits (Issue 6)
- **Task ID**: connect-concurrency-limits
- **Depends On**: unify-status-sets, clarify-status-boundary
- **Assigned To**: design-editor
- **Agent**: builder
- **Actions**:
  - In `resource-constraints.md`: Expand the document to explain the enforcement mechanism. Replace the current content with:
    ```
  # Resource constraints

  ## Parallel agent limits
  - Should be able to limit parallel agents
      - Local LLMs have major compute restraints
      - Paid services would eat tokens very quickly without throttling
      - Too much going on can be stressful for the user
  - **Enforcement**: The Overlord enforces concurrency limits by checking `execution.json`'s `concurrency.currently_running` against `concurrency.max_parallel` before transitioning a task from `queued` to `running`.
  - **Configurable per plan**: `max_parallel` is set in `execution.json` at plan level (see persistence.md).
  - **Concurrency-sensitive statuses**: The following task statuses are gated by concurrency limits (see work-statuses.md):
      - `planning` — limited during pre-planning phase
      - `reviewing` — limited to prevent review bottlenecks
      - `running` — primary concurrency gate during execution
  - **Who enforces**: The Overlord checks limits before task dispatch. Agents cannot self-transition to `running` without Overlord approval.

  ## Context limits
  - Standard for auto-compaction to deal with context limits?
    ```
  - In `work-statuses.md` lines 8, 9, 14, 15: The existing notes "(This is part of where concurrency limits are handled)" and "(Concurrency limit)" are already correct. Add a cross-reference: "See resource-constraints.md for enforcement details."
  - In `persistence.md` line 137-140 (concurrency block in execution.json): Add a comment above the block: "// Concurrency limits enforced by Overlord before task dispatch. See resource-constraints.md."
- **Acceptance Criteria**:
  - `resource-constraints.md` explains who enforces limits (Overlord), how (checking execution.json), and where (before status transitions)
  - `work-statuses.md` references resource-constraints.md for enforcement details
  - `persistence.md` execution.json concurrency block references resource-constraints.md
  - The three documents form a coherent chain: work-statuses.md → resource-constraints.md → persistence.md

### 8. Define context.md (Issue 8)
- **Task ID**: define-context-md
- **Depends On**: none
- **Assigned To**: design-editor
- **Agent**: builder
- **Actions**:
  - In `persistence.md`: After the `context.md` mention in the directory layout (line 24), add a new subsection under "File formats":
    ```
  ### `context.md` — Task context document

  **Purpose**: Provides the builder agent with relevant background information for executing the task.

  **Contents**:
  - List of relevant source files and their roles
  - Background information (prior art, related tasks, architectural decisions)
  - Research notes from the researcher (if applicable)
  - Any constraints or considerations the builder should know

  **Lifecycle**:
  - **Created by**: The planner (or researcher, if one is assigned to the plan) when generating tasks
  - **Updated by**: The planner if context changes mid-plan; builders may append notes but should not modify existing content
  - **Committed**: Yes — lives in `.agent/specs/`, tracked in git alongside `task.md`
  - **Format**: Free-form markdown, no strict schema required

  ```
  - In `agent-roles.md`: Under the Planner role (line 13-14), add: "— Generates `task.md` and `context.md` for each task."
- **Acceptance Criteria**:
  - `persistence.md` contains a dedicated section defining context.md's purpose, contents, lifecycle, and format
  - `agent-roles.md` explicitly states the planner creates context.md
  - The lifecycle (who creates, who updates, whether committed) is clearly specified

### 9. Final Validation
- **Task ID**: validate-all
- **Depends On**: clarify-status-boundary, unify-status-sets, remove-epic-reference, remove-execution-plans, clarify-researcher-scope, consolidate-overlord, connect-concurrency-limits, define-context-md
- **Assigned To**: consistency-checker
- **Agent**: validator
- **Checks**:
  - Verify no document references "epic" as a hierarchy level
  - Verify task status set is identical across `persistence.md` (task.md template, status.json, execution.json) and `work-statuses.md` (post-planning)
  - Verify plan status set is consistent between `persistence.md` (plan.md template) and `work-statuses.md` (pre-planning)
  - Verify `persistence.md` explicitly states plans and tasks have separate status machines
  - Verify researcher scope is consistently defined as plan-level across `agent-roles.md`, `agent-teams.md`, and `persistence.md`
  - Verify Overlord responsibilities in `agent-roles.md` cover all responsibilities mentioned in `persistence.md`
  - Verify `resource-constraints.md` explains enforcement mechanism and references are connected across `work-statuses.md`, `resource-constraints.md`, and `persistence.md`
  - Verify "execution plans" is no longer an undefined concept in `unit-of-work.md`
  - Verify `context.md` is defined in `persistence.md` with purpose, contents, and lifecycle
  - Verify no new inconsistencies were introduced (cross-check all 12 documents)

### 10. Documentation
- **Task ID**: generate-changelog
- **Depends On**: validate-all
- **Assigned To**: change-log-writer
- **Agent**: documenter
- **Actions**:
  - Create a changelog file at `/home/bennett/files/programming/nexum/design/consistency-fix-plan.md` summarizing all 8 fixes with:
    - Issue number and description
    - Files modified
    - Specific changes made
    - Cross-references added

## Acceptance Criteria
- All 8 consistency issues are resolved
- No new inconsistencies introduced
- All 12 design documents pass cross-reference validation
- Status sets are unified and documented as separate machines for plans vs tasks
- All agent responsibilities are clearly scoped
- All referenced concepts are defined
- Cross-document references form coherent chains

## Validation Commands
- `grep -r "epic" design/` — Should return no results (epic removed)
- `grep -r "execution plan" design/` — Should only return the clarified reference in unit-of-work.md
- `grep -r "context.md" design/` — Should show definition in persistence.md and reference in agent-roles.md
- Manual review: Compare status sets across persistence.md and work-statuses.md

## Notes
- **Ordering rationale**: Tasks 1 (Issue 3) and 2 (Issue 1) are ordered because status boundary clarification must precede status unification — you can't unify what you haven't separated. Task 7 (Issue 6) depends on both since concurrency limits are tied to specific statuses.
- **Minimal changes principle**: No document is rewritten. Each edit is a targeted insertion, deletion, or replacement. If a document is already correct for a given issue, it is not touched.
- **Capitalization convention**: All status values in JSON and code contexts use lowercase with hyphens (e.g., `waiting-manual-review`). Human-facing contexts (like work-statuses.md headings) may use title case for readability, but the canonical form is lowercase-hyphenated.
