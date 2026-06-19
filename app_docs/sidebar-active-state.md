# Sidebar Active State Fix

## Overview

The icon sidebar in the Layout component highlights the currently active navigation item based on the route. This fix ensures the active highlight updates reactively whenever SPA navigation occurs, including programmatic navigation (`push()`) and browser back/forward navigation.

## What Was Built

A minimal two-line fix in `Layout.svelte` that replaces the non-reactive `window.location.href` approach with `svelte-spa-router`'s reactive `location` store. This ensures the `currentPath` variable — which drives the sidebar's `isActive()` logic — updates on every route change.

## Technical Implementation

### Files Modified
- **`web/src/components/Layout.svelte`** — Replaced `window.location` usage with the `location` store from `svelte-spa-router`.

### Changes Made
1. Added import: `import { location } from 'svelte-spa-router';`
2. Replaced `const currentPath = $derived(new URL(window.location.href).pathname);` with `const currentPath = $derived($location);`

### Before
```svelte
const currentPath = $derived(new URL(window.location.href).pathname);
```

### After
```svelte
import { location } from 'svelte-spa-router';
const currentPath = $derived($location);
```

## The Problem

The original implementation derived `currentPath` from `window.location.href`. This caused two issues:

1. **Non-reactive**: `window.location` is a browser global, not a Svelte reactive value. `$derived` computed `currentPath` once at component mount and never re-evaluated it after SPA navigation.

2. **Hash-routing incompatibility**: The app uses hash-based routing (`svelte-spa-router` v4). Navigating from `/` to `/plans` changes the URL from `http://localhost/#/` to `http://localhost/#/plans`. The `pathname` portion (`/`) never changes — only the hash fragment changes. So `new URL(window.location.href).pathname` always returned `/`, regardless of the actual route.

As a result, the sidebar active state was permanently stale after any in-app navigation.

## The Solution

The `location` store from `svelte-spa-router` is a readable store that updates reactively on every navigation event. It returns the current route path as a string (e.g., `"/plans"`) without the hash prefix.

Using Svelte's `$` auto-subscription syntax, `$derived($location)` tracks the `location` store as a reactive dependency, so `currentPath` re-computes automatically whenever the route changes.

## How `isActive()` Works

The `isActive()` function determines whether a sidebar item should be highlighted:

```ts
function isActive(path: string): boolean {
  return currentPath === path || currentPath.startsWith(path + '/');
}
```

It matches routes in two ways:
- **Exact match**: `currentPath === path` (e.g., `/plans` matches `/plans`)
- **Prefix match**: `currentPath.startsWith(path + '/')` (e.g., `/plans/123` matches `/plans`)

Since `currentPath` is now reactively derived from `$location`, this function correctly evaluates after every navigation event.

## How It Works End to End

1. User navigates to a new route (via `RouterLink`, `push()`, or browser back/forward)
2. `svelte-spa-router` updates its internal `location` store with the new route path
3. `$location` auto-subscription detects the change
4. `$derived($location)` re-computes `currentPath`
5. `isActive()` re-evaluates for each sidebar item
6. The active CSS class (`bg-accent-blue-subtle text-accent-blue`) is applied to the correct icon

## Configuration

No configuration required. The fix uses the default `location` store provided by `svelte-spa-router`.
