# Plan: 011 - New Plan Creation Form

## Task Description
Implement the "+ New Plan" button and creation form in `PlanList.svelte`. Currently the button is hardcoded as `disabled` with a "Coming soon" tooltip. The goal is to make it functional — clicking it should open a modal where the user can fill out a form to create a new plan, which then calls the existing `createPlan` API endpoint and adds the new plan to the displayed list.

## Objective
Enable users to create new plans directly from the PlanList page through a modal form, with proper validation, error handling, and UI consistency with the existing dark GitHub-inspired theme.

## Problem Statement
The "+ New Plan" button in `PlanList.svelte` (line 48) is disabled with `disabled` attribute and a "Coming soon" tooltip. Users cannot create plans through the web UI, which is a core functionality gap in the MVP. The `createPlan` API function already exists in `api.ts` (line 75) and the `CreatePlanRequest` type is defined in `types.ts`, so the backend is ready — only the frontend UI is missing.

## Solution Approach
Create a modal-based form experience:

1. **CreatePlanForm component**: A new Svelte component containing the form fields (name, branch, goal, scope, background) with client-side validation.
2. **Modal overlay**: A simple modal overlay rendered conditionally in `PlanList.svelte` when the form is open.
3. **Button handler**: Replace the disabled button with a clickable one that toggles the modal visibility.
4. **API integration**: On form submit, call `createPlan()` and add the returned plan to `planList`.
5. **Error handling**: Use the existing `setError` pattern for API errors, with inline validation feedback for form errors.

The modal will use a fixed-position overlay with a centered card, consistent with the existing dark theme (bg-secondary background, border-default borders, text-primary/secondary/muted text colors).

## Relevant Files

### Files to Modify
| File | Purpose |
|------|---------|
| `web/src/pages/PlanList.svelte` | Replace disabled button, add modal state, import and render CreatePlanForm |
| `web/src/lib/api.ts` | Import `createPlan` in PlanList (already exists, just needs import) |

### New Files
| File | Purpose |
|------|---------|
| `web/src/components/CreatePlanForm.svelte` | The form component with fields, validation, and submit logic |

### Reference Files
| File | Purpose |
|------|---------|
| `web/src/lib/types.ts` | `CreatePlanRequest` type definition (name, branch, goal, scope?, background?) |
| `web/src/lib/errorUtils.ts` | `setError` pattern for error handling |
| `web/src/components/PlanCard.svelte` | Reference for UI styling patterns |
| `web/tailwind.config.js` | Theme color references |
| `.agent/specs/002-mvp/unimplemented/010-plan-list-loading-fix/plan.md` | Reference for plan format and conventions |

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: plan-form-builder
  - Role: Create the CreatePlanForm component and integrate it into PlanList
  - Agent: builder

- **Validator**
  - Name: plan-form-validator
  - Role: Verify the form works correctly, validates inputs, and handles errors properly
  - Agent: validator

## Step by Step Tasks

### 1. Create CreatePlanForm Component
- **Task ID**: create-plan-form-component
- **Depends On**: none
- **Assigned To**: plan-form-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/components/CreatePlanForm.svelte`
  - Define form state using `$state` for each field: `name`, `branch`, `goal`, `scope`, `background`
  - Define validation error state: `errors = $state<Record<string, string>>({})`
  - Define loading state: `submitting = $state(false)`
  - Accept props via `$props()`: `{ onSubmit, onCancel }` where `onSubmit: (data: CreatePlanRequest) => Promise<void>` and `onCancel: () => void`
  - Implement form fields:
    - **name** (required): `<input type="text">` with label "Plan Name", placeholder "e.g., Implement authentication"
    - **branch** (required): `<input type="text">` with label "Branch", placeholder "e.g., main"
    - **goal** (required): `<textarea>` with label "Goal", placeholder "Describe what this plan aims to achieve"
    - **scope** (optional): `<textarea>` with label "Scope", placeholder "Define the boundaries of this plan"
    - **background** (optional): `<textarea>` with label "Background", placeholder "Context or background information"
  - Implement validation on submit: check required fields are non-empty, trim whitespace, set inline error messages
  - On valid submit: set `submitting = true`, call `onSubmit(formData)`, handle success/failure
  - Use consistent dark theme styling:
    - Container: `bg-bg-secondary border border-border-default rounded-lg`
    - Labels: `text-text-secondary text-sm font-medium`
    - Inputs: `bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full focus:border-accent-blue focus:outline-none transition-colors`
    - Error text: `text-accent-red text-xs mt-1`
    - Submit button: `bg-btn-green hover:bg-btn-green-hover text-white px-4 py-2 rounded-md text-sm font-medium transition-colors` (disabled when `submitting`)
    - Cancel button: `bg-bg-tertiary border border-border-default text-text-secondary hover:text-text-primary px-4 py-2 rounded-md text-sm font-medium transition-colors`
  - Add a close button (X icon) in the top-right corner of the form
  - Wire `onCancel` to the close button and the cancel button in the form footer
- **Acceptance Criteria**:
  - Component renders with all five form fields
  - Required fields show validation errors when empty on submit attempt
  - Optional fields can be left empty without errors
  - Submit button is disabled during submission
  - Form styling matches the dark GitHub-inspired theme
  - `onSubmit` is called with valid `CreatePlanRequest` data on successful validation
  - `onCancel` is called when cancel/close is triggered
  - Svelte compiler produces no warnings or errors

### 2. Integrate Modal into PlanList
- **Task ID**: integrate-modal-into-plan-list
- **Depends On**: create-plan-form-component
- **Assigned To**: plan-form-builder
- **Agent**: builder
- **Actions**:
  - In `web/src/pages/PlanList.svelte`:
    - Add import: `import CreatePlanForm from '$lib/components/CreatePlanForm.svelte'`
    - Add import: `import { createPlan } from '$lib/api'` (alongside existing `listPlans` import)
    - Add import: `import type { CreatePlanRequest } from '$lib/types'`
    - Add state: `showCreateForm = $state(false)`
    - Add state: `creating = $state(false)`
    - Replace the disabled button (line 48-50) with:
      ```svelte
      <button type="button" class="px-4 py-2 bg-btn-green hover:bg-btn-green-hover text-white rounded-md text-sm font-medium transition-colors cursor-pointer" on:click={() => showCreateForm = true}>
        + New Plan
      </button>
      ```
    - Add modal overlay markup after the main content div (or inside it, at the end):
      ```svelte
      {#if showCreateForm}
        <div class="fixed inset-0 z-50 flex items-center justify-center bg-bg-primary/80 backdrop-blur-sm" on:click={() => showCreateForm = false}>
          <div class="bg-bg-secondary border border-border-default rounded-lg shadow-2xl w-full max-w-lg mx-4" on:click|stopPropagation>
            <CreatePlanForm
              onSubmit={async (data: CreatePlanRequest) => {
                creating = true;
                try {
                  const newPlan = await createPlan(data);
                  planList.unshift(newPlan);
                  showCreateForm = false;
                } catch (e) {
                  if (e instanceof Error) {
                    errorCleanup = setError(
                      () => { error = e.message; },
                      () => { error = null; }
                    );
                  }
                } finally {
                  creating = false;
                }
              }}
              onCancel={() => showCreateForm = false}
            />
          </div>
        </div>
      {/if}
      ```
    - Add error display area if not already present (after the stats row or before the plan grid):
      ```svelte
      {#if error}
        <div class="mb-4 p-3 bg-accent-red-subtle border border-accent-red/30 rounded-md text-accent-red text-sm">
          {error}
        </div>
      {/if}
      ```
- **Acceptance Criteria**:
  - "+ New Plan" button is no longer disabled and has proper styling (green button matching theme)
  - Clicking the button opens the modal overlay
  - Modal overlay dims the background and centers the form
  - Clicking outside the form (on the overlay) closes the modal
  - Clicking inside the form does NOT close the modal (stopPropagation)
  - Form is rendered correctly within the modal
  - Svelte compiler produces no warnings or errors

### 3. Wire API Integration and List Update
- **Task ID**: wire-api-and-list-update
- **Depends On**: integrate-modal-into-plan-list
- **Assigned To**: plan-form-builder
- **Agent**: builder
- **Actions**:
  - Ensure the `onSubmit` handler in PlanList (defined in step 2) correctly:
    - Sets `creating = true` before the API call
    - Calls `createPlan(data)` with the form data
    - On success: prepends the new plan to `planList` using `planList.unshift(newPlan)`
    - On success: closes the modal by setting `showCreateForm = false`
    - On error: uses `setError` to display the error message (existing pattern)
    - In `finally`: sets `creating = false`
  - Verify that the new plan appears at the top of the list immediately after creation
  - Verify that the stats (totalPlans, activePlans, etc.) update reactively since they use `$derived`
- **Acceptance Criteria**:
  - Successful plan creation adds the new plan to the top of the list
  - Stats counters update immediately after creation
  - API errors are displayed to the user with auto-clear after 5 seconds
  - The modal closes after successful creation
  - The modal stays open on error (so user can retry)

### 4. Add Keyboard Accessibility
- **Task ID**: add-keyboard-accessibility
- **Depends On**: wire-api-and-list-update
- **Assigned To**: plan-form-builder
- **Agent**: builder
- **Actions**:
  - Add an `$effect` in PlanList that listens for Escape key when modal is open:
    ```svelte
    $effect(() => {
      if (!showCreateForm) return;
      function handleKeydown(e: KeyboardEvent) {
        if (e.key === 'Escape') {
          showCreateForm = false;
        }
      }
      document.addEventListener('keydown', handleKeydown);
      return () => document.removeEventListener('keydown', handleKeydown);
    });
    ```
  - Optionally: add `tabindex="-1"` and focus management to the modal container for screen reader accessibility
- **Acceptance Criteria**:
  - Pressing Escape closes the modal when it's open
  - Pressing Escape when modal is closed has no effect
  - Event listener is cleaned up when modal closes or component unmounts

### 5. Final Validation
- **Task ID**: validate-all
- **Depends On**: add-keyboard-accessibility
- **Assigned To**: plan-form-validator
- **Agent**: validator
- **Checks**:
  - Run `cd web && npm run check` — Svelte type checking passes
  - Run `cd web && npm run build` — build succeeds
  - Verify the "+ New Plan" button is styled correctly (green, not disabled)
  - Verify clicking the button opens the modal form
  - Verify all five form fields render correctly
  - Verify submitting with empty required fields shows validation errors
  - Verify submitting with valid data creates a plan and updates the list
  - Verify the new plan appears at the top of the list
  - Verify stats update reactively after creation
  - Verify Escape key closes the modal
  - Verify clicking outside the form closes the modal
  - Verify clicking inside the form does NOT close the modal
  - Verify error messages display correctly on API failure
  - Verify error messages auto-clear after 5 seconds
  - Verify the form styling matches the dark theme consistently

## Acceptance Criteria
- The "+ New Plan" button is functional and styled consistently with the theme
- Clicking the button opens a modal with a creation form
- The form has five fields: name (required), branch (required), goal (required), scope (optional), background (optional)
- Required field validation prevents submission with empty values
- Successful submission calls `createPlan` API and adds the new plan to the list
- The new plan appears at the top of the list immediately
- Stats counters update reactively after creation
- API errors are displayed with auto-clear
- The modal can be closed by: clicking outside, pressing Escape, or clicking cancel
- All existing functionality remains intact (no regressions)
- Svelte type checking and build pass without errors or warnings

## Validation Commands
- `cd web && npm run check` — Run Svelte type checking
- `cd web && npm run build` — Build the frontend (verifies no compilation errors)
- Manual testing in browser:
  - Open `/plans` page
  - Click "+ New Plan" button
  - Fill out required fields and submit
  - Verify new plan appears in the list
  - Try submitting with empty required fields
  - Press Escape to close modal
  - Click outside form to close modal

## Notes
- The `createPlan` API endpoint returns a `Plan` object, which matches the type used in `planList` — no type conversion needed.
- Using `planList.unshift(newPlan)` is the correct approach since `planList` is `$state` and Svelte 5 runes will track the array mutation reactively.
- The modal overlay uses `fixed inset-0` positioning with a semi-transparent backdrop and backdrop blur, consistent with modern UI patterns.
- The form component is designed to be reusable — it accepts `onSubmit` and `onCancel` callbacks as props, making it easy to use in other contexts if needed.
- Consider future enhancement: adding a success toast notification after plan creation (out of scope for this plan).
- The `CreatePlanRequest` type from `types.ts` exactly matches the form fields, so no data transformation is needed between form state and API call.
