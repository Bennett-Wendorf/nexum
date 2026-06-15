# Frontend Dashboard

## Overview

The Nexum frontend dashboard is a vanilla Svelte 5 single-page application (SPA) that provides a web interface for managing AI agent orchestration plans and tasks. It consumes the REST API built in chunk 007 and renders hierarchical plan/task structures, real-time status information, markdown content, and kanban-style task boards — all styled with a dark GitHub-inspired theme powered by Tailwind CSS.

## What Was Built

A complete Svelte SPA in `web/` with:

- **Build pipeline**: Vite + Svelte plugin + Tailwind CSS + PostCSS
- **Client-side routing**: `svelte-spa-router` with 5 page routes and wildcard fallback
- **API client**: Typed module wrapping all REST API endpoints with `/api/v1` prefix
- **Component library**: 8 reusable components (Layout, StatusBadge, MarkdownRenderer, Breadcrumb, PlanCard, TaskCard, TaskColumn, MarkdownViewer)
- **Reactive state**: Svelte writable stores for app-wide state management
- **Status system**: Color-coded badges for 7 plan statuses and 8 task statuses
- **Dark theme**: Tailwind configuration matching GitHub's dark palette
- **Test suite**: Vitest unit tests + Playwright E2E tests

## Technical Implementation

### Files Created

#### Build Configuration

| File | Purpose |
|------|---------|
| `web/package.json` | npm dependencies (Svelte 5, svelte-spa-router, markdown-it, Tailwind, Vite, vitest, Playwright) |
| `web/vite.config.ts` | Vite config: Svelte plugin, build output to `../static/`, dev proxy `/api` → `localhost:3000`, `$lib` alias |
| `web/tsconfig.json` | TypeScript config: ES2022, bundler module resolution, strict mode |
| `web/tailwind.config.js` | Tailwind config with dark theme color palette |
| `web/postcss.config.js` | PostCSS: tailwindcss + autoprefixer |
| `web/index.html` | SPA shell with `#app` mount point |
| `web/vitest.config.ts` | Vitest config: jsdom environment, test file patterns |
| `web/playwright.config.ts` | Playwright config: Chromium, webServer preview on port 4173 |

#### Application Entry

| File | Purpose |
|------|---------|
| `web/src/main.ts` | App mount point — mounts `App` to `#app`, imports `app.css` |
| `web/src/App.svelte` | Root component — wraps `<Router {routes} />` inside `<Layout>` |
| `web/src/app.css` | Tailwind directives + base styles + `.md-content` component class |

#### Routing

| File | Purpose |
|------|---------|
| `web/src/routes/index.ts` | Route definitions for `svelte-spa-router` |

Route table:

| Path | Component |
|------|-----------|
| `/` | `PlanList` |
| `/plans` | `PlanList` |
| `/plans/:branch/:planId` | `PlanDetail` |
| `/plans/:branch/:planId/tasks` | `TaskList` |
| `/plans/:branch/:planId/tasks/:taskId` | `TaskDetail` |
| `*` (wildcard) | `PlanList` (fallback) |

#### Pages

| File | Purpose |
|------|---------|
| `web/src/pages/PlanList.svelte` | Plans overview with stats row (total/active plans, total/completed tasks) and 2-column card grid |
| `web/src/pages/PlanDetail.svelte` | Single plan view with breadcrumb, metadata, progress bar, markdown sections (goal/scope/background), and task list |
| `web/src/pages/TaskList.svelte` | Kanban board with 8 status columns + list view toggle, breadcrumb, "Add Task" button |
| `web/src/pages/TaskDetail.svelte` | Task detail with breadcrumb, status transition dropdown, tabbed interface (definition + status history), and sidebar (agent assignment, dependencies, status info) |
| `web/src/pages/Home.svelte` | Placeholder home page (unused by active routes) |
| `web/src/pages/NotFound.svelte` | Placeholder 404 page (unused — wildcard route falls back to PlanList) |

#### Components

| File | Purpose |
|------|---------|
| `web/src/components/Layout.svelte` | Three-panel shell: 56px icon sidebar + 220px agent pool sidebar + scrollable main content |
| `web/src/components/StatusBadge.svelte` | Pill badge with status-specific colors; props: `status`, `type` (`'plan'` \| `'task'`), `label` (optional override) |
| `web/src/components/MarkdownRenderer.svelte` | Renders markdown via `markdown-it` (html: false, breaks: true, linkify: true, typographer: true); props: `content`, `className` |
| `web/src/components/Breadcrumb.svelte` | Navigation trail with `›` separators; linked items clickable, last item bold/non-clickable |
| `web/src/components/PlanCard.svelte` | Clickable plan card with name, status badge, branch (monospace), goal (line-clamp-2), and progress bar |
| `web/src/components/TaskCard.svelte` | Compact kanban task card with name, status badge, and assigned agent avatar |
| `web/src/components/TaskColumn.svelte` | Kanban column (min-width 260px) with header (label + count badge) and scrollable task card list |
| `web/src/components/MarkdownViewer.svelte` | Alternative markdown viewer with prose styling (used in some contexts) |

#### Library Modules

| File | Purpose |
|------|---------|
| `web/src/lib/api.ts` | API client with `request()` base helper and all REST endpoint functions |
| `web/src/lib/store.ts` | Svelte writable stores for reactive state |
| `web/src/lib/types.ts` | TypeScript interfaces for Plan, Task, TaskStatus, AgentLease, StatusTransition, etc. |
| `web/src/lib/statusColors.ts` | Status color mappings and kanban column definitions |
| `web/src/lib/markdown.ts` | Shared `markdown-it` instance and `renderMarkdown()` utility |

#### Tests

| File | Purpose |
|------|---------|
| `web/tests/api.test.ts` | API client tests: URL construction, query parameters, error handling, JSON body |
| `web/tests/StatusBadge.test.ts` | Status color tests: all plan/task statuses, fallback, kanban columns, labels |
| `web/tests/MarkdownRenderer.test.ts` | Markdown rendering tests: headings, lists, code blocks, HTML sanitization, linkify, line breaks |
| `web/tests/PlanCard.test.ts` | PlanCard logic tests: progress calculation, empty tasks, href generation |
| `web/tests/TaskCard.test.ts` | TaskCard logic tests: href generation, agent name extraction, missing agent |
| `web/src/__tests__/App.test.ts` | App structure test: Router + Layout presence, route definitions |
| `web/tests/e2e/planList.spec.ts` | E2E: page load, stats row, empty state |
| `web/tests/e2e/planDetail.spec.ts` | E2E: page load, breadcrumb |
| `web/tests/e2e/taskList.spec.ts` | E2E: kanban view, toggle buttons, view mode switching |
| `web/tests/e2e/taskDetail.spec.ts` | E2E: page load, breadcrumb, tabs, status dropdown |

### Key APIs

#### `api.ts` — API Client Functions

**Base helper:**
```typescript
request<T>(method, path, body?, queryParams?): Promise<T>
```

**Plan API:**
```typescript
listPlans(branch?, status?): Promise<ListResponse<Plan>>
getPlan(branch, planId): Promise<Plan>
createPlan(req): Promise<Plan>
updatePlan(branch, planId, req): Promise<Plan>
deletePlan(branch, planId): Promise<void>
transitionPlanStatus(branch, planId, req): Promise<Plan>
```

**Task API:**
```typescript
listTasks(branch, planId, status?): Promise<ListResponse<Task>>
getTask(branch, planId, taskId): Promise<Task>
createTask(branch, planId, req): Promise<Task>
updateTask(branch, planId, taskId, req): Promise<Task>
deleteTask(branch, planId, taskId): Promise<void>
transitionTaskStatus(branch, planId, taskId, req): Promise<Task>
```

**Execution API:**
```typescript
getExecutionState(branch, planId): Promise<ExecutionState>
listRunningTasks(): Promise<RunningTask[]>
healthCheck(): Promise<{ status: string }>
```

**Config API:**
```typescript
getConfig(): Promise<Config>
listAgents(): Promise<AgentsResponse>
```

#### `store.ts` — Svelte Stores

**Context stores:**
- `currentPlan: Writable<Plan | null>`
- `currentBranch: Writable<string | null>`
- `currentTask: Writable<Task | null>`

**Data stores:**
- `plans: Writable<Plan[]>`
- `tasks: Writable<Task[]>`
- `runningTasks: Writable<RunningTask[]>`
- `agents: Writable<AgentRegistration[]>`

**UI state stores:**
- `loading: Writable<boolean>`
- `error: Writable<string | null>`
- `viewMode: Writable<'kanban' | 'list'>`

**Helper:**
```typescript
setError(message: string): void  // Auto-clears after 5 seconds
```

## Status Color Scheme

### Plan Statuses (7)

| Status | Text Color | Background |
|--------|-----------|------------|
| `draft` | `text-text-muted` | `bg-border-default` |
| `queued` | `text-accent-blue` | `bg-[#1f6feb22]` |
| `planning` | `text-accent-purple` | `bg-[#a371f722]` |
| `reviewing` | `text-accent-yellow` | `bg-[#d2992222]` |
| `approved` | `text-accent-green` | `bg-[#3fb95022]` |
| `complete` | `text-accent-green` | `bg-[#3fb95022]` |
| `rejected` | `text-accent-red` | `bg-[#f8514922]` |

### Task Statuses (8)

| Status | Text Color | Background |
|--------|-----------|------------|
| `backlog` | `text-text-muted` | `bg-border-default` |
| `queued` | `text-accent-blue` | `bg-[#1f6feb22]` |
| `running` | `text-accent-blue` | `bg-[#1f6feb22]` |
| `reviewing` | `text-accent-yellow` | `bg-[#d2992222]` |
| `waiting-manual-review` | `text-accent-yellow` | `bg-[#d2992222]` |
| `merge-queue` | `text-accent-purple` | `bg-[#a371f722]` |
| `abandoned` | `text-accent-red` | `bg-[#f8514922]` |
| `completed` | `text-accent-green` | `bg-[#3fb95022]` |

### Kanban Column Order

1. Backlog
2. Queued
3. Running
4. Reviewing
5. Manual Review
6. Merge Queue
7. Completed
8. Abandoned

Unknown statuses fall back to muted gray (`bg-border-default text-text-muted`).

## Tailwind Theme Configuration

Dark GitHub-inspired palette defined in `web/tailwind.config.js`:

### Background Colors
| Token | Hex |
|-------|-----|
| `bg-primary` | `#0d1117` |
| `bg-secondary` | `#161b22` |
| `bg-tertiary` | `#1c2129` |

### Border Colors
| Token | Hex |
|-------|-----|
| `border-default` | `#30363d` |
| `border-muted` | `#21262d` |

### Text Colors
| Token | Hex |
|-------|-----|
| `text-primary` | `#e1e4e8` |
| `text-secondary` | `#c9d1d9` |
| `text-muted` | `#8b949e` |
| `text-faint` | `#484f58` |

### Accent Colors
| Token | Hex |
|-------|-----|
| `accent-blue` | `#58a6ff` |
| `accent-green` | `#3fb950` |
| `accent-yellow` | `#d29922` |
| `accent-red` | `#f85149` |
| `accent-purple` | `#a371f7` |
| `accent-pink` | `#f778ba` |

Font family: Inter → -apple-system → BlinkMacSystemFont → Segoe UI → sans-serif.

Custom component class `.md-content` provides styled markdown rendering containers with dark theme appropriate paddings, border styling, and typography.

## Build Process and Deployment

### Development

```bash
cd web && npm install
npm run dev        # Vite dev server on port 5173, proxies /api → localhost:3000
```

### Production Build

```bash
cd web && npm run build    # Outputs to ../static/
```

Build output lands in the project root `static/` directory with:
- `index.html` — SPA entry point
- JS/CSS asset files (hashed filenames)

### Deployment

The Axum backend serves the static files via `tower_http::services::ServeDir`. Unknown routes fall back to `index.html` for client-side routing (SPA fallback). In production, both the API and static files are served from the same origin, eliminating the need for a proxy.

### Preview

```bash
cd web && npm run preview   # Vite preview server on port 4173
```

## Test Setup

### Unit Tests (Vitest)

```bash
cd web && npm test:run      # Run all unit tests once
cd web && npm test           # Watch mode
```

Configuration (`vitest.config.ts`):
- Environment: `jsdom`
- Test patterns: `src/**/__tests__/**/*.test.{ts,js}` and `tests/**/*.test.{ts,js}`
- Svelte preprocessing enabled

Test coverage:
- **API client**: URL construction, query parameters, error handling, JSON body serialization
- **Status colors**: All plan/task status mappings, fallback behavior, kanban column order, human-readable labels
- **Markdown rendering**: Headings, lists, code blocks, HTML sanitization, auto-linking, line breaks
- **PlanCard**: Progress calculation, empty task handling, href generation
- **TaskCard**: Href generation, agent name extraction, missing agent handling
- **App structure**: Router + Layout integration, route definitions

### E2E Tests (Playwright)

```bash
cd web && npm run test:e2e
```

Configuration (`playwright.config.ts`):
- Browser: Chromium (Desktop)
- Base URL: `http://localhost:4173` (Vite preview)
- Auto-spawns preview server before tests
- Retries: 2 in CI, 0 locally

Test coverage:
- **Plan List**: Page load, stats row visibility, empty state
- **Plan Detail**: Page load, breadcrumb navigation
- **Task List**: Kanban view rendering, toggle buttons existence, view mode switching
- **Task Detail**: Page load, breadcrumb, tab buttons, status transition dropdown

## Usage

### Accessing the Dashboard

- **Development**: `http://localhost:5173` (Vite dev server)
- **Production**: Same origin as the Axum backend (e.g., `http://localhost:3000`)

### Navigation

- **Dashboard / Plans list**: `/` or `/plans`
- **Plan detail**: `/plans/:branch/:planId`
- **Task kanban board**: `/plans/:branch/:planId/tasks`
- **Task detail**: `/plans/:branch/:planId/tasks/:taskId`

All pages show loading states during data fetch and appropriate empty/error states when data is unavailable.

### Status Transitions

On the Task Detail page, use the status dropdown to transition a task to a new status. This calls the `PATCH /api/v1/plans/:branch/:planId/tasks/:taskId/status` endpoint. Backend validation enforces valid transitions.

### View Modes

On the Task List page, toggle between:
- **Kanban**: Horizontal scrollable columns grouped by task status
- **List**: Table view with task name, status badge, agent, and dependency count
