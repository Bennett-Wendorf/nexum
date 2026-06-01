# Plan: 008 - Frontend Dashboard

## Task Description
Build the Svelte SPA frontend dashboard for nexum. A vanilla Svelte application (no SvelteKit) providing a web interface for managing plans, tasks, and monitoring agent orchestration. Consumes the REST API from chunk 007, renders plans/tasks in a hierarchical list view with kanban-style task status columns, includes markdown rendering, status badges, and a dark-themed UI matching existing mockup designs.

## Objective
Create a fully functional Svelte SPA in `web/` that:
- Sets up Svelte + Vite + Tailwind CSS build pipeline
- Implements client-side routing via `svelte-spa-router` with SPA fallback
- Renders plan list page with status, branch, and task summary
- Renders plan detail page with metadata and markdown-rendered content
- Renders task list page with kanban-style columns grouped by task status
- Renders task detail page with markdown definition, status history, dependencies
- Provides API client module for all REST API interactions
- Implements reusable status badge components for plan and task statuses
- Includes markdown rendering component using `markdown-it`
- Uses Tailwind CSS for all styling (dark GitHub-inspired theme from mockups)
- Serves as static files via Axum backend's `ServeDir` middleware
- Passes unit tests (vitest) and E2E tests (playwright)

## Problem Statement
Nexum needs a web dashboard so users can view and manage plans and tasks without CLI or filesystem interaction. The dashboard must display hierarchical plan/task structures, show real-time status information, render markdown content, support status transitions via UI, match the dark-themed mockup design, work as a pure SPA served statically, load data from chunk 007 REST endpoints, and function on localhost.

## Solution Approach
Build a vanilla Svelte SPA in `web/` with this structure:

```
web/
  package.json, vite.config.ts, svelte.config.js
  tailwind.config.js, postcss.config.js, index.html
  src/
    main.js, App.svelte, app.css, routes.js
    pages/
      PlanList.svelte, PlanDetail.svelte
      TaskList.svelte, TaskDetail.svelte
    components/
      Layout.svelte, StatusBadge.svelte, MarkdownRenderer.svelte
      PlanCard.svelte, TaskCard.svelte, Breadcrumb.svelte, TaskColumn.svelte
    lib/
      api.js, store.js, types.js, statusColors.js
  static/  (static assets)
```

Key decisions: Vanilla Svelte (no SvelteKit), svelte-spa-router for routing, Tailwind CSS for dark theme, markdown-it for frontend rendering, npm as package manager, vitest/playwright for testing.

## Relevant Files

### Existing Files (references)
- `design/tech-stack.md` — Svelte, svelte-spa-router, Tailwind, markdown-it, npm, vitest, playwright
- `design/interfaces.md` — Kanban, Mind map, Hierarchical list interface ideas
- `design/operation-structure.md` — Web app, localhost operation
- `design/work-statuses.md` — Status definitions for plans and tasks
- `mockups/refined/` — HTML mockups showing target UI design
- `mockups/refined/styles.css` — Dark theme palette
- `.agent/specs/002-mvp/unimplemented/007-rest-api/plan.md` — REST API endpoints

### New Files (to be created)
- `web/package.json` — npm dependencies
- `web/vite.config.ts` — Vite config (output to `../static/`, proxy `/api` to localhost:3000)
- `web/svelte.config.js` — Svelte compiler config
- `web/tailwind.config.js` — Tailwind config with dark theme colors
- `web/postcss.config.js` — PostCSS config
- `web/index.html` — SPA entry shell
- `web/src/main.js` — App mount point
- `web/src/App.svelte` — Root with router
- `web/src/routes.js` — Route definitions
- `web/src/app.css` — Base styles + Tailwind directives
- `web/src/pages/PlanList.svelte` — Plans overview
- `web/src/pages/PlanDetail.svelte` — Single plan view
- `web/src/pages/TaskList.svelte` — Kanban task board
- `web/src/pages/TaskDetail.svelte` — Task detail view
- `web/src/components/Layout.svelte` — App shell
- `web/src/components/StatusBadge.svelte` — Status badge
- `web/src/components/MarkdownRenderer.svelte` — Markdown rendering
- `web/src/components/PlanCard.svelte` — Plan list card
- `web/src/components/TaskCard.svelte` — Kanban task card
- `web/src/components/Breadcrumb.svelte` — Breadcrumb nav
- `web/src/components/TaskColumn.svelte` — Kanban column
- `web/src/lib/api.js` — API client module
- `web/src/lib/store.js` — Svelte stores
- `web/src/lib/types.js` — JSDoc type definitions
- `web/src/lib/statusColors.js` — Status color mapping
- `web/vitest.config.js` — Vitest config
- `web/playwright.config.js` — Playwright config
- `web/tests/api.test.js` — API client tests
- `web/tests/StatusBadge.test.js` — Status badge tests
- `web/tests/MarkdownRenderer.test.js` — Markdown renderer tests

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

### Team Members

- **Builder**
  - Name: frontend-setup-builder
  - Role: Project scaffolding, build tooling, Tailwind CSS, routing infrastructure
  - Agent: builder

- **Builder**
  - Name: frontend-core-builder
  - Role: API client, stores, status badges, markdown renderer, layout, card components
  - Agent: builder

- **Builder**
  - Name: frontend-pages-builder
  - Role: All page components (plan list, plan detail, task list, task detail)
  - Agent: builder

- **Builder**
  - Name: frontend-tests-builder
  - Role: Unit tests (vitest) and E2E tests (playwright)
  - Agent: builder

- **Validator**
  - Name: frontend-validator
  - Role: Verify build, pages, API integration, tests
  - Agent: validator

- **Documenter**
  - Name: frontend-documenter
  - Role: Generate documentation for completed frontend
  - Agent: documenter

## Step by Step Tasks

### 1. Project Scaffolding and Build Tooling
- **Task ID**: frontend-scaffolding
- **Depends On**: none
- **Assigned To**: frontend-setup-builder
- **Agent**: builder
- **Actions**:
  - Create `web/` directory structure
  - Create `web/package.json` with dependencies: `svelte@^4.2.19`, `svelte-spa-router@^4.0.1`, `markdown-it@^14.1.0`; devDependencies: `@sveltejs/vite-plugin-svelte@^4.0.0`, `tailwindcss@^3.4.10`, `postcss@^8.4.41`, `autoprefixer@^10.4.20`, `vite@^5.4.0`, `vitest@^2.0.0`, `@playwright/test@^1.45.0`, `jsdom@^24.1.0`
  - Create `web/vite.config.ts`: Svelte plugin, build output to `../static/`, dev server proxy `/api` → `localhost:3000`, vitest jsdom environment
  - Create `web/svelte.config.js`: vitePreprocess
  - Create `web/tailwind.config.js`: content patterns, extended colors matching mockup palette (bg: #0d1117/#161b22/#1c2129, border: #30363d/#21262d, text: #e1e4e8/#c9d1d9/#8b949e/#484f58, accent: blue/green/yellow/red/purple/pink)
  - Create `web/postcss.config.js`: tailwindcss + autoprefixer
  - Create `web/index.html`: SPA shell with `#app` div, dark theme classes, module script import
  - Create `web/src/main.js`: App mount
  - Create `web/src/app.css`: Tailwind directives + Inter font family
  - Create `web/src/App.svelte` skeleton and `web/src/routes.js` skeleton
  - Run `npm install`, verify `npm run dev` and `npm run build` succeed
- **Acceptance Criteria**:
  - `npm install` completes without errors
  - `npm run dev` starts Vite dev server
  - `npm run build` outputs to `../static/`
  - Tailwind processes without errors
  - Svelte compiler processes `.svelte` files
  - `npm test:run` runs vitest without errors
  - Vite proxy forwards `/api` to `localhost:3000`

### 2. API Client Module
- **Task ID**: frontend-api-client
- **Depends On**: frontend-scaffolding
- **Assigned To**: frontend-core-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/lib/api.js` with base `request()` helper (fetch wrapper with `/api` prefix, JSON serialization, error throwing from `data.message`)
  - Plan API: `listPlans(branch?, status?)`, `getPlan(branch, planId)`, `createPlan(...)`, `updatePlan(...)`, `deletePlan(...)`, `transitionPlanStatus(...)`
  - Task API: `listTasks(branch, planId, status?)`, `getTask(branch, planId, taskId)`, `createTask(...)`, `updateTask(...)`, `deleteTask(...)`, `transitionTaskStatus(...)`
  - Execution/Config: `getExecutionState(...)`, `listRunningTasks()`, `getConfig()`, `listAgents()`, `healthCheck()`
  - Create `web/src/lib/types.js` with JSDoc typedefs for Plan, TaskRef, Task, TaskStatus, AgentLease, StatusTransition
- **Acceptance Criteria**:
  - All API functions exported and callable
  - `request()` constructs URLs with `/api` prefix
  - Errors throw with `data.message`
  - All CRUD operations for plans and tasks implemented
  - Query parameters encoded in list endpoints
  - JSDoc types document all response structures

### 3. Svelte Stores and Status Color Mapping
- **Task ID**: frontend-stores
- **Depends On**: frontend-api-client
- **Assigned To**: frontend-core-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/lib/store.js`: writable stores for `currentPlan`, `currentBranch`, `currentTask`, `plans`, `tasks`, `runningTasks`, `agents`, `loading`, `error`, `viewMode`; `setError()` helper with 5s auto-clear
  - Create `web/src/lib/statusColors.js`:
    - `planStatusColors`/`planStatusBgColors` mapping all 7 plan statuses (draft, queued, planning, reviewing, approved, complete, rejected) to Tailwind color classes
    - `taskStatusColors`/`taskStatusBgColors` mapping all 8 task statuses (backlog, queued, running, reviewing, waiting-manual-review, merge-queue, abandoned, completed)
    - `kanbanColumns` array in display order
    - `kanbanColumnLabels` with human-readable labels (e.g., "waiting-manual-review" → "Manual Review")
    - `getPlanStatusColor(status)` and `getTaskStatusColor(status)` helpers with fallback
  - Import `app.css` in `main.js`
- **Acceptance Criteria**:
  - All stores writable and importable
  - Color maps cover all plan/task statuses
  - Badge color pairs provide background + text color
  - `kanbanColumns` in correct display order
  - Helper functions return fallback colors for unknown statuses
  - Error store auto-clears after 5 seconds

### 4. Layout Component and Navigation
- **Task ID**: frontend-layout
- **Depends On**: frontend-stores
- **Assigned To**: frontend-core-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/components/Layout.svelte` with three-panel design:
    - Icon sidebar (56px): Dashboard, Kanban, Agent Teams, Settings icons with active state highlighting
    - Agent pool sidebar (220px): "Agent Pool" header, active/idle count summary, simplified for MVP
    - Main content area: `<slot />`, scrollable
  - Update `web/src/App.svelte`: wrap `<Router {routes} />` in `<Layout>`
  - Create `web/src/routes.js`: `/` → PlanList, `/plans` → PlanList, `/plans/:branch/:planId` → PlanDetail, `/plans/:branch/:planId/tasks` → TaskList, `/plans/:branch/:planId/tasks/:taskId` → TaskDetail, `*` → PlanList
- **Acceptance Criteria**:
  - Three-panel layout renders correctly
  - Icon sidebar 56px with nav icons and active highlighting
  - Agent pool sidebar 220px with simplified count display
  - Main content fills remaining space, scrolls independently
  - Router integrates with Layout wrapper
  - All route definitions cover required pages
  - Wildcard route falls back to PlanList

### 5. Status Badge Component
- **Task ID**: frontend-status-badge
- **Depends On**: frontend-stores
- **Assigned To**: frontend-core-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/components/StatusBadge.svelte`:
    - Props: `status` (string), `type` ('plan' | 'task'), `label` (optional override)
    - Uses `getPlanStatusColor`/`getTaskStatusColor` for color class
    - Uses `kanbanColumnLabels` for display label (falls back to raw status)
    - Renders as pill badge: `inline-flex items-center px-2 py-0.5 rounded-full text-xs font-semibold`
- **Acceptance Criteria**:
  - Renders badge with correct color based on status and type
  - Plan badges use `planStatusBgColors`, task badges use `taskStatusBgColors`
  - Custom `label` prop overrides default
  - Unknown statuses fall back to muted color
  - Matches mockup badge style

### 6. Markdown Renderer Component
- **Task ID**: frontend-markdown-renderer
- **Depends On**: frontend-scaffolding
- **Assigned To**: frontend-core-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/components/MarkdownRenderer.svelte`:
    - Imports `markdown-it`, configures with `html: false`, `breaks: true`, `linkify: true`, `typographer: true`
    - Props: `content` (string), `className` (optional, defaults to styled container class)
    - Reactive `$: html = md.render(content)` with empty content placeholder
    - Renders in dark-themed container matching mockup `.md-content` style
- **Acceptance Criteria**:
  - Renders markdown as HTML
  - No raw HTML passthrough (html: false)
  - Single newlines render as line breaks
  - URLs auto-linked
  - Empty content shows "No content" placeholder
  - Styled in dark theme container
  - `className` prop allows customization

### 7. Breadcrumb Component
- **Task ID**: frontend-breadcrumb
- **Depends On**: frontend-layout
- **Assigned To**: frontend-core-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/components/Breadcrumb.svelte`:
    - Props: `items` array of `{ label, href? }`
    - Renders separator `›` between items
    - Linked items clickable, last item (no href) styled as current (bold)
- **Acceptance Criteria**:
  - Renders breadcrumb with separators
  - Linked items clickable, last item non-clickable and bold
  - Separator styled in border color
  - Matches mockup breadcrumb style

### 8. Plan Card Component
- **Task ID**: frontend-plan-card
- **Depends On**: frontend-status-badge
- **Assigned To**: frontend-core-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/components/PlanCard.svelte`:
    - Props: `plan` object with id, name, status, branch, goal, tasks
    - Displays plan name, StatusBadge, branch (monospace), goal (line-clamp-2)
    - Progress bar showing completed/total tasks
    - Clickable card linking to `/plans/:branch/:planId`
    - Hover highlights border to accent-blue
- **Acceptance Criteria**:
  - Renders name, status badge, branch, goal
  - Progress bar shows completion percentage
  - Links to plan detail page
  - Hover border color change
  - No progress bar when no tasks
  - Goal clamped to 2 lines

### 9. Task Card Component
- **Task ID**: frontend-task-card
- **Depends On**: frontend-status-badge
- **Assigned To**: frontend-core-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/components/TaskCard.svelte`:
    - Props: `task`, `planId`, `branch`
    - Displays task name, StatusBadge, assigned agent badge
    - Clickable linking to `/plans/:branch/:planId/tasks/:taskId`
    - Compact design for kanban columns
- **Acceptance Criteria**:
  - Renders name, status badge, agent assignment
  - Links to task detail page
  - Agent badge shown when assigned
  - Compact sizing for kanban placement

### 10. Task Column Component (Kanban)
- **Task ID**: frontend-task-column
- **Depends On**: frontend-task-card
- **Assigned To**: frontend-core-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/components/TaskColumn.svelte`:
    - Props: `status`, `tasks`, `planId`, `branch`
    - Header: status label (from `kanbanColumnLabels`) + count badge
    - Body: TaskCard for each task, "No tasks" placeholder when empty
    - Min-width 260px, scrollable body, dark background with border
- **Acceptance Criteria**:
  - Header shows label and count
  - Task cards rendered per task
  - Empty placeholder text
  - Scrollable body
  - Min-width 260px
  - Matches mockup column style

### 11. Plan List Page
- **Task ID**: frontend-plan-list-page
- **Depends On**: frontend-plan-card, frontend-api-client
- **Assigned To**: frontend-pages-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/pages/PlanList.svelte`:
    - Fetches plans via `listPlans()` on mount
    - Page header: title + "New Plan" button
    - Stats row: 4 stat cards (total plans, active, total tasks, completed)
    - 2-column grid of PlanCard components
    - Loading, error, and empty states
- **Acceptance Criteria**:
  - Fetches and displays plans on mount
  - Stats row with plan/task counts
  - 2-column card grid
  - Loading/error/empty states handled
  - Cards link to plan detail pages

### 12. Plan Detail Page
- **Task ID**: frontend-plan-detail-page
- **Depends On**: frontend-plan-list-page, frontend-markdown-renderer
- **Assigned To**: frontend-pages-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/pages/PlanDetail.svelte`:
    - Props: `branch`, `planId` (from route params)
    - Fetches plan via `getPlan()` and tasks via `listTasks()` on mount
    - Top bar: back link, Breadcrumb, StatusBadge
    - Plan metadata: name, branch, created date
    - Progress bar with completion percentage
    - MarkdownRenderer for goal, scope, background sections
    - Task list with status badges and agent assignments
    - Links to kanban board and task detail pages
- **Acceptance Criteria**:
  - Fetches plan and tasks on mount
  - Breadcrumb navigation
  - Markdown sections rendered
  - Task list with status badges
  - Progress bar
  - Links to task pages
  - Loading/error/empty states

### 13. Task List Page (Kanban Board)
- **Task ID**: frontend-task-list-page
- **Depends On**: frontend-task-column, frontend-api-client
- **Assigned To**: frontend-pages-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/pages/TaskList.svelte`:
    - Props: `branch`, `planId`
    - Fetches plan and tasks on mount
    - Top bar: back link, Breadcrumb, kanban/list toggle, "Add Task" button
    - Kanban view: horizontal scrollable TaskColumn for each status in `kanbanColumns`
    - List view: table with task name, status badge, agent, dependencies
    - Loading/error/empty states
- **Acceptance Criteria**:
  - Fetches tasks on mount
  - Kanban view with scrollable columns
  - List view as alternative
  - View mode toggle works
  - Breadcrumb navigation
  - Empty column placeholders
  - Cards link to task detail pages

### 14. Task Detail Page
- **Task ID**: frontend-task-detail-page
- **Depends On**: frontend-plan-detail-page, frontend-markdown-renderer
- **Assigned To**: frontend-pages-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/pages/TaskDetail.svelte`:
    - Props: `branch`, `planId`, `taskId`
    - Fetches task via `getTask()` on mount
    - Top bar: back link, Breadcrumb, status transition dropdown
    - Main content: task name, status badge, dependency/file counts
    - Tabbed interface: "Task Definition" tab (markdown description, acceptance criteria list, background, notes, files to modify), "Status History" tab (timeline of transitions)
    - Sidebar: agent assignment card, dependency list, status timeline
- **Acceptance Criteria**:
  - Fetches task on mount
  - Breadcrumb navigation
  - Status transition dropdown
  - Tabbed interface (definition + status history)
  - Markdown-rendered sections
  - Acceptance criteria as bullet list
  - Agent assignment sidebar
  - Status transition timeline
  - Loading/error states

### 15. Unit Tests
- **Task ID**: frontend-unit-tests
- **Depends On**: frontend-task-detail-page
- **Assigned To**: frontend-tests-builder
- **Agent**: builder
- **Actions**:
  - Create `web/vitest.config.js` if not in vite.config.ts
  - Create `web/tests/api.test.js`:
    - Mock `fetch` to test `request()` helper
    - Test error handling (non-200 responses throw)
    - Test URL construction with query parameters
    - Test all API function signatures
  - Create `web/tests/StatusBadge.test.js`:
    - Test rendering with various plan statuses
    - Test rendering with various task statuses
    - Test custom label prop
    - Test fallback for unknown status
  - Create `web/tests/MarkdownRenderer.test.js`:
    - Test rendering basic markdown (headings, lists, code blocks)
    - Test empty content placeholder
    - Test linkify (auto-detect URLs)
    - Test `className` prop override
  - Create `web/tests/PlanCard.test.js`:
    - Test rendering with plan data
    - Test progress bar calculation
    - Test empty task list (no progress bar)
  - Create `web/tests/TaskCard.test.js`:
    - Test rendering with task data
    - Test agent badge display
  - Run `npm test:run` to verify all tests pass
- **Acceptance Criteria**:
  - All test files created and importable
  - `npm test:run` passes all tests
  - API client error handling tested
  - StatusBadge covers plan, task, custom label, and unknown status
  - MarkdownRenderer covers basic rendering, empty content, linkify
  - PlanCard covers rendering and progress calculation
  - TaskCard covers rendering and agent display

### 16. E2E Tests
- **Task ID**: frontend-e2e-tests
- **Depends On**: frontend-unit-tests
- **Assigned To**: frontend-tests-builder
- **Agent**: builder
- **Actions**:
  - Create `web/playwright.config.js`: testDir, webServer pointing to vite preview, browser configs
  - Create `web/tests/e2e/planList.spec.js`:
    - Navigate to plan list page
    - Verify page loads without errors
    - Verify stats row renders
    - Verify plan cards render (if mock data available)
    - Verify empty state when no plans
  - Create `web/tests/e2e/planDetail.spec.js`:
    - Navigate to plan detail page
    - Verify breadcrumb renders
    - Verify plan metadata displays
    - Verify markdown sections render
    - Verify task list renders
  - Create `web/tests/e2e/taskList.spec.js`:
    - Navigate to task list (kanban view)
    - Verify kanban columns render
    - Toggle to list view and verify table renders
    - Verify view mode toggle works
  - Create `web/tests/e2e/taskDetail.spec.js`:
    - Navigate to task detail page
    - Verify breadcrumb, tabs, sidebar
    - Verify markdown content renders
    - Verify status history timeline
  - Run `npx playwright install` for browser binaries
  - Run `npm run test:e2e` to verify tests pass
- **Acceptance Criteria**:
  - Playwright config valid
  - All E2E test files created
  - `npm run test:e2e` passes all tests
  - Plan list, detail, task list, task detail pages tested
  - View mode toggle tested
  - Breadcrumb navigation tested
  - Markdown rendering tested in E2E context

### 17. Final Validation
- **Task ID**: validate-all
- **Depends On**: frontend-e2e-tests
- **Assigned To**: frontend-validator
- **Agent**: validator
- **Checks**:
  - Run `cd web && npm run build` — must succeed, output in `../static/`
  - Run `cd web && npm test:run` — all vitest tests must pass
  - Run `cd web && npm run test:e2e` — all playwright tests must pass
  - Verify `static/` directory contains `index.html` and asset files
  - Verify all pages render: PlanList, PlanDetail, TaskList, TaskDetail
  - Verify router works: all routes accessible, wildcard fallback to PlanList
  - Verify Layout renders three-panel design
  - Verify StatusBadge renders correct colors for all plan/task statuses
  - Verify MarkdownRenderer renders markdown content safely
  - Verify Breadcrumb renders navigation trail
  - Verify PlanCard shows name, status, branch, goal, progress
  - Verify TaskCard shows name, status, agent
  - Verify TaskColumn renders kanban columns correctly
  - Verify kanbanColumns order matches design
  - Verify Tailwind dark theme matches mockup colors
  - Verify API client functions match chunk 007 REST endpoints
  - Verify stores are writable and reactive
  - Verify SPA fallback works (unknown routes → index.html)
  - Verify Vite dev server proxy works for `/api` requests
  - Verify all components use Tailwind classes (no custom CSS except app.css base)
  - Verify no SvelteKit-specific patterns (no server functions, no SSR)

### 18. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: frontend-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`
  - Document the SPA architecture and routing
  - Document the API client module and available functions
  - Document component library (StatusBadge, MarkdownRenderer, PlanCard, TaskCard, etc.)
  - Document status color scheme and badge conventions
  - Document Tailwind theme configuration
  - Document build process and deployment (Vite → static → Axum ServeDir)
  - Document test setup (vitest + playwright)

## Acceptance Criteria
- `cd web && npm run build` succeeds with output in `../static/`
- `cd web && npm test:run` passes all unit tests
- `cd web && npm run test:e2e` passes all E2E tests
- SPA serves correctly with `svelte-spa-router` and SPA fallback
- All pages render: PlanList, PlanDetail, TaskList, TaskDetail
- Plan List page: plan grid, stats row, loading/error/empty states
- Plan Detail page: breadcrumb, metadata, markdown sections, task list, progress bar
- Task List page: kanban columns (all 8 statuses), list view toggle, breadcrumb
- Task Detail page: breadcrumb, tabs (definition/status history), markdown content, sidebar
- StatusBadge renders correct colors for all 7 plan statuses and 8 task statuses
- MarkdownRenderer renders markdown safely (no raw HTML injection)
- API client covers all endpoints from chunk 007 REST API plan
- Svelte stores provide reactive app state management
- Tailwind dark theme matches mockup color palette
- Layout renders three-panel design (icon sidebar, agent sidebar, main content)
- Breadcrumb navigation works on all detail pages
- No SvelteKit patterns used (pure vanilla Svelte SPA)
- Vite dev server proxies `/api` to `localhost:3000`
- Build output compatible with Axum `ServeDir` static file serving

## Validation Commands
- `cd web && npm install` — Install dependencies
- `cd web && npm run dev` — Start dev server
- `cd web && npm run build` — Build to `../static/`
- `cd web && npm test:run` — Run vitest unit tests
- `cd web && npm run test:e2e` — Run playwright E2E tests
- `ls -la static/` — Verify build output exists
- `grep -r "svelte-spa-router" web/src/` — Verify router integration
- `grep -r "markdown-it" web/src/` — Verify markdown rendering
- `grep -r "tailwind" web/src/` — Verify Tailwind usage
- `grep -r "StatusBadge" web/src/` — Verify status badges used in pages
- `grep -r "MarkdownRenderer" web/src/` — Verify markdown renderer used in pages

## Notes
- This plan depends on chunk 001 (Project Scaffolding) for the `web/` directory existence and chunk 007 (REST API) for the API endpoints the frontend consumes.
- The frontend builds to `../static/` which the Axum backend serves via `tower_http::services::ServeDir` with SPA fallback.
- The Vite dev server proxies `/api` to `localhost:3000` for development; in production, the Axum backend serves both the API and static files.
- `markdown-it` is used only on the frontend; the backend uses `pulldown-cmark` for parsing markdown files.
- For MVP, the agent pool sidebar is simplified (just shows counts); full agent management UI is a future enhancement.
- Status transition dropdowns in the UI call the PATCH endpoints; actual transition validation happens on the backend.
- The dark theme color palette is derived from the mockup CSS (`mockups/refined/styles.css`) and matches GitHub's dark theme.
- `svelte-spa-router` uses history mode by default; the Axum SPA fallback handles unknown routes by serving `index.html`.
- The `web/` directory is the frontend source; `static/` is the build output (gitignored or committed depending on project convention).
