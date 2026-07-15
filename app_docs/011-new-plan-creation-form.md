# Feature: New Plan Creation Form

## Overview

Replaces the disabled "Coming soon" "+ New Plan" button on the Plans page with a fully functional modal form. Users can create new plans directly from the web UI by filling out a form with name, branch, goal, scope, and background fields. On submission, the form calls the existing `createPlan` API endpoint and prepends the new plan to the displayed list.

## What Was Built

Two components work together to deliver this feature:

- **`CreatePlanForm.svelte`** — A reusable form component with five fields, client-side validation, loading state, and callback-based submission.
- **`PlanList.svelte`** (modified) — The Plans page now opens a modal overlay containing the form when the "+ New Plan" button is clicked. It handles the API call, list update, error display, and keyboard accessibility (Escape to close).

## Technical Implementation

### Files Created or Modified

| File | Action | Purpose |
|------|--------|---------|
| `web/src/components/CreatePlanForm.svelte` | **Created** | Form component with fields, validation, and submit logic |
| `web/src/pages/PlanList.svelte` | **Modified** | Added modal state, button handler, API integration, keyboard listener, and error display |

### Key Functions and APIs

**CreatePlanForm.svelte**
- `validate()` — Checks that `name`, `branch`, and `goal` are non-empty after trimming. Populates an `errors` record with inline messages.
- `handleSubmit()` — Runs validation, then calls the `onSubmit` prop callback with a `CreatePlanRequest` object. Manages `submitting` loading state.
- Props: `{ onSubmit: (data: CreatePlanRequest) => Promise<void>; onCancel: () => void }`

**PlanList.svelte**
- Modal toggle via `showCreateForm = $state(false)` flag.
- `onSubmit` handler calls `createPlan(data)`, prepends the result with `planList.unshift(newPlan)`, and closes the modal on success.
- Error handling uses the existing `setError` pattern from `$lib/errorUtils`, displaying errors in a red alert banner with auto-clear.
- `$effect` block registers an `Escape` key listener when the modal is open, with proper cleanup on unmount/close.

### Dependencies

No new dependencies were added. The feature uses existing imports:
- `$lib/api` — `listPlans`, `createPlan`
- `$lib/types` — `Plan`, `CreatePlanRequest`
- `$lib/errorUtils` — `setError`

## Form Fields and Validation

| Field | Type | Required | Widget |
|-------|------|----------|--------|
| Plan Name | `string` | Yes | `<input type="text">` |
| Branch | `string` | Yes | `<input type="text">` |
| Goal | `string` | Yes | `<textarea>` |
| Scope | `string` | No | `<textarea>` |
| Background | `string` | No | `<textarea>` |

**Validation rules:**
- Required fields (`name`, `branch`, `goal`) must be non-empty after trimming whitespace.
- Inline error messages appear below the offending field in `text-accent-red text-xs`.
- Optional fields (`scope`, `background`) are trimmed; empty values become `undefined` in the API payload.

## Modal Behavior

### Opening
- Clicking the "+ New Plan" button (green, top-right of the page header) sets `showCreateForm = true`.

### Closing methods
1. **Overlay click** — Clicking the dimmed backdrop (`bg-bg-primary/80`) closes the modal.
2. **Cancel button** — The "Cancel" button in the form footer calls `onCancel`.
3. **Close icon (×)** — The SVG × button in the form header calls `onCancel`.
4. **Escape key** — A `$effect`-registered `keydown` listener closes the modal when `Escape` is pressed.

### Event propagation
The modal card uses `on:click|stopPropagation` so clicks inside the form do not bubble to the overlay and accidentally close the modal.

## API Integration

The `onSubmit` callback in PlanList performs:

```typescript
const newPlan = await createPlan(data);
planList.unshift(newPlan);   // prepends to list
showCreateForm = false;       // closes modal
```

- On **success**: the new plan appears at the top of the list immediately. Stats counters (`$derived` values) update reactively.
- On **error**: the modal stays open so the user can retry. The error message displays in a red banner and auto-clears after 5 seconds via `setError`.

## Error Handling Pattern

Uses the existing `setError` utility from `$lib/errorUtils`:

```typescript
errorCleanup = setError(
  () => { error = e.message; },   // setter
  () => { error = null; }          // cleanup (auto-clear after 5s)
);
```

A separate `$effect` block in PlanList watches `errorCleanup` and registers it as a cleanup function, ensuring the auto-clear timer is properly managed across reactive updates.

## Styling Conventions (Dark GitHub Theme)

The form follows the established dark theme token classes:

| Element | Classes |
|---------|---------|
| Form container | `bg-bg-secondary border border-border-default rounded-lg p-6` |
| Labels | `text-text-secondary text-sm font-medium` |
| Inputs / Textareas | `bg-bg-primary border border-border-default text-text-primary rounded-md px-3 py-2 text-sm w-full focus:border-accent-blue focus:outline-none transition-colors` |
| Error text | `text-accent-red text-xs mt-1` |
| Submit button | `bg-btn-green hover:bg-btn-green-hover text-white px-4 py-2 rounded-md text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed` |
| Cancel button | `bg-bg-tertiary border border-border-default text-text-secondary hover:text-text-primary px-4 py-2 rounded-md text-sm font-medium transition-colors` |
| Modal overlay | `fixed inset-0 z-50 flex items-center justify-center bg-bg-primary/80 backdrop-blur-sm` |
| Modal card | `bg-bg-secondary border border-border-default rounded-lg shadow-2xl w-full max-w-lg mx-4` |

## Known Warnings and Notes

### Svelte 5 Event Handler Syntax
- `CreatePlanForm.svelte` uses the Svelte 5 `onclick={...}` syntax for the close button and cancel button.
- `PlanList.svelte` uses the Svelte 4 `on:click={...}` syntax for the "+ New Plan" button and modal overlay.
- Both syntaxes are valid, but mixing styles within the same project may trigger deprecation warnings from the Svelte compiler. Consider standardizing on `on:click` (Svelte 4 style) or migrating to `onclick` (Svelte 5 style) in a future cleanup pass.

### Accessibility Notes
- The modal overlay does not use `role="dialog"` or `aria-modal="true"` attributes.
- Focus trapping is not implemented — tab navigation can move focus outside the modal while it's open.
- The modal container does not have `tabindex="-1"` for programmatic focus management.
- These are minor a11y gaps that could be addressed in a follow-up spec.

### Deviation from Original Plan
- The plan specified a `creating` state variable in PlanList to track submission progress. The actual implementation relies on the `submitting` state inside `CreatePlanForm` instead, which is sufficient since the form's submit button handles the disabled state.
- The plan specified a `finally` block in the PlanList `onSubmit` handler to reset `creating`. This was omitted since `creating` was not implemented in PlanList.

## Usage

1. Navigate to the Plans page (`/plans`).
2. Click the **+ New Plan** button in the top-right corner.
3. Fill in the required fields (Plan Name, Branch, Goal). Optionally fill in Scope and Background.
4. Click **Create** to submit, or **Cancel** / the **×** button / **Escape** to close.
5. On success, the new plan appears at the top of the list and stats update immediately.
