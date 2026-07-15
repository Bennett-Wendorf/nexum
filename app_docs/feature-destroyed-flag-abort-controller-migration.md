# Destroyed Flag to AbortController Migration

## Overview

Refactored `Layout.svelte` to replace the `$state` `destroyed` flag pattern with a single `AbortController` that governs the entire `onMount` lifecycle. The API layer (`api.ts`) was updated to accept and propagate `AbortSignal`, enabling idiomatic Svelte 5 lifecycle management through request cancellation.

## What Was Built

The `destroyed` reactive state variable was removed from `Layout.svelte` and replaced with a single `AbortController` instance. The `request()` base function in `api.ts` now accepts an optional `AbortSignal` parameter, which is passed through to `fetch()`. The `listAgents()` and `listRunningTasks()` API functions were updated to accept and forward the signal. When the component unmounts, `controller.abort()` cancels all in-flight requests, preventing post-unmount state mutations without explicit guard checks.

## Technical Implementation

### Files Modified

- **`web/src/lib/api.ts`** — Added `signal?: AbortSignal` parameter to `request()`, `listAgents()`, and `listRunningTasks()`
- **`web/src/components/Layout.svelte`** — Replaced `destroyed` flag with `AbortController`-based lifecycle management

### Key Changes in `api.ts`

- `request()` function signature now includes `signal?: AbortSignal` as its fifth parameter
- The signal is passed directly to `fetch(url, { ..., signal })`, leveraging native browser abort support
- `listAgents(options?: { signal?: AbortSignal })` — accepts an options object with an optional signal, forwarded to `request()`
- `listRunningTasks(options?: { signal?: AbortSignal })` — same pattern as `listAgents()`
- Callers that omit the signal continue to work unchanged (backwards-compatible)

### Key Changes in `Layout.svelte`

- **Removed**: `let destroyed = $state(false);` and all `if (destroyed) return` guards
- **Added**: `const controller = new AbortController()` at the top of `onMount`
- All API calls (`listAgents`, `listRunningTasks`) receive `{ signal: controller.signal }`
- The `onMount` cleanup function calls `controller.abort()` instead of `destroyed = true`
- The inner `fetchController` (previously used for per-poll cancellation) was removed in favor of the single controller
- `AbortError` exceptions from `listAgents` are caught and silently ignored
- Polling errors are logged only when the signal is not aborted and the error is not an `AbortError`

### Error Handling

- **Agent fetch errors**: `AbortError` is silently ignored; other errors trigger the existing `$effect`-based `setError` cleanup pattern
- **Polling errors**: Non-abort errors are logged to the console via `console.warn` only when the component is still mounted (`!controller.signal.aborted`)
- The `$effect` error cleanup pattern (`errorCleanup`) is preserved unchanged

## Usage

### Passing an AbortSignal to API calls

```typescript
// In a component's onMount:
onMount(async () => {
  const controller = new AbortController();

  // Pass signal to API calls
  const agents = await listAgents({ signal: controller.signal });
  const tasks = await listRunningTasks({ signal: controller.signal });

  // Return cleanup function
  return () => {
    controller.abort();
  };
});
```

### Handling abort errors

```typescript
try {
  const data = await listAgents({ signal: controller.signal });
  // ... update state
} catch (e) {
  if (e instanceof DOMException && e.name === 'AbortError') {
    // Silently ignore — component is unmounting
  } else {
    // Handle real errors
  }
}
```

## Migration Notes

### For developers applying this pattern to other components

1. **Identify the `destroyed` flag pattern**: Look for `let destroyed = $state(false)` paired with `if (destroyed) return` guards in async callbacks.

2. **Create a single AbortController**: Instantiate it at the top of `onMount` (or the relevant lifecycle hook).

3. **Pass the signal to all API calls**: Ensure every `fetch`-based operation receives `controller.signal`.

4. **Update the cleanup function**: Replace `destroyed = true` with `controller.abort()`.

5. **Remove all destroyed guards**: The aborted signal naturally prevents post-unmount mutations by rejecting in-flight promises with `AbortError`.

6. **Handle AbortError in catch blocks**: Silently ignore `AbortError` exceptions — they indicate normal lifecycle cancellation, not a failure.

7. **Consider per-operation controllers**: If you need fine-grained cancellation (e.g., cancelling a specific poll cycle without cancelling everything), create additional `AbortController` instances scoped to that operation. For unmount-guard use cases, a single controller is sufficient.

### Benefits over the destroyed flag pattern

- **Separation of concerns**: Lifecycle management stays out of business logic
- **No reactive surface pollution**: No lifecycle state variables in the component
- **Native cancellation**: In-flight network requests are actually cancelled, not just ignored after completion
- **Idiomatic Svelte 5**: Aligns with the framework's recommended patterns (`$effect` cleanup + `AbortController`)
- **Better resource management**: Aborted fetches release network resources immediately

## Code Review Fixes Applied

The following refinements were applied after the initial implementation based on code review feedback:

1. **`errorCleanup` made reactive** — Changed from a plain `let` declaration to `$state(null)` so the `$effect` cleanup pattern works correctly. Without reactivity, the cleanup callback would capture a stale reference and fail to clear the error state when set.

2. **`agentsByType` declaration order** — Moved the `agentsByType` `$derived` declaration to the top of the component alongside the other `$state` and `$derived` variables, improving readability and ensuring consistent declaration ordering.

3. **`activeRoles` simplified** — Removed the intermediate `activeRolesKey` string roundtrip. The `$derived` now constructs the `Set` directly via `new Set(...)`, eliminating an unnecessary serialization/deserialization step.

4. **Redundant `AbortError` check removed** — The catch condition was simplified since checking `controller.signal.aborted` already covers the `AbortError` case. The explicit `e.name === 'AbortError'` check was redundant.

5. **`loading` renamed to `agentsLoading`** — The flag was renamed to clarify that it only covers the agent fetch operation, not overall component readiness. This prevents confusion when other async operations are added.

6. **Explicit `AbortError` early return** — Added a clear early return for `AbortError` in the `fetchRunningTasks` catch block, making the abort handling path explicit rather than relying on implicit flow.

7. **`request()` switched to options object** — Changed the `request()` function signature from five positional parameters to an options object with `{ body?, queryParams?, signal? }`. This improves readability at call sites and avoids argument-order mistakes.

8. **Unchecked cast documented** — Added a JSDoc comment on the `as T` type cast in `request()` noting that it is unchecked. The cast relies on the API contract for correctness; TypeScript cannot verify the runtime shape.
