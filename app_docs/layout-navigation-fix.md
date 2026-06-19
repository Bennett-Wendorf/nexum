# Layout Navigation Fix

## Overview

Fixed the `Layout.svelte` component's reactive path tracking so that the sidebar active-state highlight correctly updates after every SPA navigation event.

## What Was Built

A minimal two-line fix in `Layout.svelte` that replaces a non-reactive `window.location.href` read with `svelte-spa-router`'s reactive `location` store. This ensures the sidebar icon highlight stays in sync with the current route during single-page navigation.

## Technical Implementation

### Problem

The original code derived `currentPath` from `window.location.href`:

```svelte
const currentPath = $derived(new URL(window.location.href).pathname);
```

`$derived` in Svelte 5 tracks reactive dependencies. Since `window.location` is a browser global — not a Svelte reactive value — `$derived` computed `currentPath` once at component mount and never re-evaluated it. After any in-app navigation (e.g., `/` → `/plans`), the sidebar active state remained stale.

### Solution

The fix imports `location` from `svelte-spa-router` and derives `currentPath` from the `$location` store:

```svelte
import { location } from 'svelte-spa-router';

const currentPath = $derived($location);
```

- The `$` auto-subscription syntax accesses the store reactively.
- `$derived` tracks `$location` as a reactive dependency, re-computing `currentPath` on every navigation event.
- The `location` store returns the path as a string (e.g., `"/plans"`), matching the format of the previous `new URL(...).pathname` approach, so the existing `isActive()` logic requires no changes.

### Files Changed

| File | Change |
|------|--------|
| `web/src/components/Layout.svelte` | Added `import { location } from 'svelte-spa-router'`; replaced `currentPath` derivation from `window.location.href` with `$location` store |

### Dependencies

No new dependencies. `svelte-spa-router` was already a project dependency.

## Usage

The fix is transparent to end users and other components. After navigation between routes (e.g., `/` → `/plans`), the sidebar icon highlight updates immediately without any code changes in consuming components.

## Configuration

No configuration required. The fix relies entirely on `svelte-spa-router`'s built-in `location` store, which is automatically updated by the router on every navigation event.
