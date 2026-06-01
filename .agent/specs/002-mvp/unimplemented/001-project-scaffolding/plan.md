# Plan: 001 - Project Scaffolding

## Task Description
Set up the complete project scaffolding for nexum: a Rust + Svelte monorepo with Cargo/Axum backend and Vite/Svelte frontend. This establishes the foundational directory structure, build tooling, dependencies, and configuration files required for all subsequent MVP development.

## Objective
Create a fully functional monorepo skeleton that compiles (backend) and builds (frontend) with no errors, following the architecture defined in `design/tech-stack.md`.

## Problem Statement
The nexum project currently has only design documentation. There is no buildable codebase — no Rust project, no frontend project, no build configuration, and no directory structure. All subsequent development depends on this scaffolding being in place.

## Solution Approach
Initialize a Rust binary crate with Axum and all backend dependencies, scaffold a Svelte + Vite + Tailwind frontend, and create the full directory structure per the tech stack design. Configure build scripts, .gitignore, and all necessary configuration files.

## Relevant Files

### Existing Files
- `design/tech-stack.md` — Defines the complete tech stack and build structure
- `design/implementation-chunks.md` — Defines this as chunk 1 of the MVP
- `README.md` — Project description

### New Files (if needed)
- `Cargo.toml` — Rust workspace/project manifest with all backend dependencies
- `src/main.rs` — Binary entry point (minimal, `tokio::main` async fn)
- `src/api/mod.rs` — API module stub
- `src/acp/mod.rs` — ACP client module stub
- `src/git/mod.rs` — Git operations module stub
- `src/persistence/mod.rs` — File persistence module stub
- `src/config/mod.rs` — Configuration module stub
- `.gitignore` — Ignore patterns for Rust, Node, and `.agent/state/`
- `web/package.json` — Frontend dependencies
- `web/vite.config.ts` — Vite configuration
- `web/tailwind.config.js` — Tailwind CSS configuration
- `web/postcss.config.js` — PostCSS configuration
- `web/src/main.js` — Svelte app entry point
- `web/src/App.svelte` — Root Svelte component
- `web/src/routes/` — Router configuration
- `web/src/pages/` — Page components
- `web/src/components/` — Reusable components
- `web/src/lib/` — Utility modules
- `web/src/app.css` — Global Tailwind styles
- `web/index.html` — HTML entry point
- `static/` — Directory for Svelte build output

## Team Orchestration

> **Worktree Isolation**: The team-lead creates an isolated git worktree for this spec using `~/.config/opencode/scripts/worktree-create.sh`. All builders work inside this worktree. After final validation, changes are merged back via `~/.config/opencode/scripts/worktree-merge.sh`.

The team-lead agent will orchestrate execution using these team members:

### Team Members

- **Builder**
  - Name: scaffold-builder
  - Role: Set up Rust backend and Svelte frontend scaffolding
  - Agent: builder

- **Validator**
  - Name: scaffold-validator
  - Role: Verify builds succeed and directory structure matches design
  - Agent: validator

- **Documenter**
  - Name: scaffold-documenter
  - Role: Generate documentation for completed scaffolding
  - Agent: documenter

## Step by Step Tasks

### 1. Initialize Rust Backend
- **Task ID**: init-rust-backend
- **Depends On**: none
- **Assigned To**: scaffold-builder
- **Agent**: builder
- **Actions**:
  - Create `Cargo.toml` with package name `nexum`, edition 2021
  - Add dependencies: `axum`, `tokio` (with `full` feature), `serde`, `serde_json`, `tower-http` (with `fs` feature), `tracing`, `tracing-subscriber`, `config`, `jsonrpsee` (with `server` and `macros` features), `pulldown-cmark`, `notify`, `thiserror`, `anyhow`
  - Create `src/main.rs` with a minimal `#[tokio::main]` async fn that prints a startup message
  - Create module stubs:
    - `src/api/mod.rs` — empty module
    - `src/acp/mod.rs` — empty module
    - `src/git/mod.rs` — empty module
    - `src/persistence/mod.rs` — empty module
    - `src/config/mod.rs` — empty module
  - Create `static/` directory (for Svelte build output)
- **Acceptance Criteria**:
  - `cargo check` succeeds with no errors
  - All module stubs exist and compile
  - `src/main.rs` runs and prints output

### 2. Create .gitignore
- **Task ID**: create-gitignore
- **Depends On**: none
- **Assigned To**: scaffold-builder
- **Agent**: builder
- **Actions**:
  - Create `.gitignore` with patterns for:
    - Rust build artifacts: `target/`, `**/*.rs.bk`
    - Node frontend: `web/node_modules/`, `web/.svelte-kit/`
    - Agent state: `.agent/state/`, `.agent/specs/**/*.status.json`, `.agent/specs/**/tasks/**/status.json`
    - IDE: `.idea/`, `.vscode/`, `*.swp`
    - OS: `.DS_Store`, `Thumbs.db`
    - Environment: `.env`, `.env.*`
- **Acceptance Criteria**:
  - `.gitignore` exists at repo root
  - All specified patterns are present
  - `git status` does not show ignored files

### 3. Initialize Svelte Frontend
- **Task ID**: init-svelte-frontend
- **Depends On**: none
- **Assigned To**: scaffold-builder
- **Agent**: builder
- **Actions**:
  - Create `web/` directory
  - Create `web/package.json` with:
    - name: `nexum-web`
    - dependencies: `svelte`, `svelte-spa-router`
    - devDependencies: `@sveltejs/vite-plugin-svelte`, `vite`, `tailwindcss`, `postcss`, `autoprefixer`, `markdown-it`, `vitest`, `@playwright/test`
    - scripts: `dev`, `build`, `preview`, `test`, `lint`
  - Create `web/vite.config.ts` with Svelte plugin and basic configuration
  - Create `web/index.html` with root div and script tag
  - Create `web/src/main.js` that mounts the Svelte app
  - Create `web/src/App.svelte` with a minimal layout including base stylesheet import and a slot for route rendering
  - Create directory structure:
    - `web/src/routes/` — SPA router configuration
    - `web/src/pages/` — page components
    - `web/src/components/` — reusable components
    - `web/src/lib/` — utility modules
  - Create `web/src/routes/index.svelte` as a basic home page placeholder
- **Acceptance Criteria**:
  - `npm install` in `web/` succeeds
  - `npm run build` in `web/` produces output without errors
  - All directories exist

### 4. Configure Tailwind CSS
- **Task ID**: setup-tailwind
- **Depends On**: init-svelte-frontend
- **Assigned To**: scaffold-builder
- - **Agent**: builder
- **Actions**:
  - Create `web/tailwind.config.js` with content paths for `src/**/*.{js,svelte}` and default theme
  - Create `web/postcss.config.js` with Tailwind and autoprefixer plugins
  - Create `web/src/app.css` with `@tailwind base`, `@tailwind components`, `@tailwind utilities` directives
  - Import `app.css` in `main.js`
- **Acceptance Criteria**:
  - Tailwind directives are properly configured
  - `npm run build` still succeeds after Tailwind integration
  - Tailwind utility classes can be used in Svelte components

### 5. Configure Frontend Router
- **Task ID**: setup-router
- **Depends On**: init-svelte-frontend
- **Assigned To**: scaffold-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/routes/index.js` using `svelte-spa-router`'s `createRouter` with a basic route map
  - Update `web/src/App.svelte` to render the router component
  - Create `web/src/pages/Home.svelte` as a placeholder page
  - Create `web/src/pages/NotFound.svelte` for 404 handling
- **Acceptance Criteria**:
  - Router is configured with at least a home route
  - `npm run build` succeeds
  - Router component renders in App.svelte

### 6. Configure Frontend Testing
- **Task ID**: setup-testing
- **Depends On**: init-svelte-frontend
- **Assigned To**: scaffold-builder
- **Agent**: builder
- **Actions**:
  - Create `web/vitest.config.js` (or configure within `vite.config.ts`) with Svelte plugin
  - Add `test` script to `package.json` running `vitest`
  - Create `web/src/__tests__/App.test.js` with a basic placeholder test
  - Create `web/playwright.config.js` with basic Playwright configuration
  - Add `test:e2e` script to `package.json` running `playwright test`
- **Acceptance Criteria**:
  - `npm test` runs vitest and passes
  - `npm run test:e2e` runs playwright without errors (no tests to fail yet)

### 7. Create Build Scripts
- **Task ID**: create-build-scripts
- **Depends On**: init-rust-backend, init-svelte-frontend
- **Assigned To**: scaffold-builder
- **Agent**: builder
- **Actions**:
  - Create `Makefile` (or shell scripts) with targets:
    - `dev`: Run both backend (`cargo run`) and frontend (`npm -w web dev`) concurrently
    - `build`: Build frontend (`npm -w web build`), then copy output to `static/`, then build backend (`cargo build --release`)
    - `test`: Run `cargo test` and `npm -w web test`
    - `clean`: Remove `target/`, `web/node_modules/`
  - Alternatively, if npm workspaces are used, add scripts to root `package.json`
- **Acceptance Criteria**:
  - `make dev` (or equivalent) starts both servers
  - `make build` produces a release binary and static files
  - `make test` runs all tests

### 8. Set Up Frontend Markdown Rendering
- **Task ID**: setup-markdown-rendering
- **Depends On**: init-svelte-frontend
- **Assigned To**: scaffold-builder
- **Agent**: builder
- **Actions**:
  - Create `web/src/lib/markdown.js` that wraps `markdown-it` for rendering markdown to HTML
  - Export a `renderMarkdown(text)` function
  - Create `web/src/components/MarkdownViewer.svelte` component that uses the renderer
- **Acceptance Criteria**:
  - `renderMarkdown()` function exists and returns HTML string
  - `MarkdownViewer` component renders markdown content
  - `npm run build` still succeeds

### 9. Final Validation
- **Task ID**: validate-all
- **Depends On**: init-rust-backend, create-gitignore, init-svelte-frontend, setup-tailwind, setup-router, setup-testing, create-build-scripts, setup-markdown-rendering
- **Assigned To**: scaffold-validator
- **Agent**: validator
- **Checks**:
  - Run `cargo check` — must succeed
  - Run `cargo build` — must succeed
  - Run `cargo test` — must pass (no tests yet, but should pass)
  - Run `npm install` in `web/` — must succeed
  - Run `npm run build` in `web/` — must succeed
  - Run `npm test` in `web/` — must pass
  - Verify directory structure matches `tech-stack.md` specification
  - Verify `.gitignore` patterns work correctly
  - Verify all module stubs exist in `src/`

### 10. Documentation
- **Task ID**: generate-docs
- **Depends On**: validate-all
- **Assigned To**: scaffold-documenter
- **Agent**: documenter
- **Actions**:
  - Read the plan file and implementation files
  - Generate documentation in `app_docs/`

## Acceptance Criteria
- `cargo check` succeeds with no errors
- `cargo build` produces a working binary
- `cargo test` passes
- `npm run build` in `web/` produces static output
- `npm test` in `web/` passes
- Directory structure matches `design/tech-stack.md` specification exactly:
  - `src/main.rs`, `src/api/`, `src/acp/`, `src/git/`, `src/persistence/`, `src/config/`
  - `web/src/routes/`, `web/src/pages/`, `web/src/components/`, `web/src/lib/`
  - `static/` directory exists
- `.gitignore` covers Rust, Node, and agent state patterns
- Tailwind CSS is configured and working
- svelte-spa-router is configured with basic routes
- vitest and playwright are configured for testing
- Build scripts (`Makefile` or npm scripts) exist for dev, build, test, clean

## Validation Commands
- `cargo check` — Verify Rust project compiles
- `cargo build` — Build Rust backend
- `cargo test` — Run Rust tests
- `cd web && npm install` — Install frontend dependencies
- `cd web && npm run build` — Build Svelte frontend
- `cd web && npm test` — Run frontend unit tests
- `cd web && npm run test:e2e` — Run frontend E2E tests
- `find src/ -type f` — Verify backend directory structure
- `find web/src/ -type d` — Verify frontend directory structure

## Notes
- The `static/` directory is where the Svelte build output will be copied for Axum to serve. The scaffolding builder should ensure the build script copies `web/.svelte-kit/` output to `static/`.
- For the `dev` script, consider using `cargo watch` or `just` for concurrent dev server management, but a simple `Makefile` is sufficient for MVP.
- The `web/` directory uses npm (not pnpm or yarn) per the tech stack specification.
- jsonrpsee is included in dependencies but not actively used until the ACP Client chunk. It should compile without errors.
- The `config` crate will be used for `~/.config/nexum/config.toml` — the scaffolding just includes the dependency; actual configuration parsing is a later chunk.
- All module stubs (`mod.rs` files) should be empty modules (just `pub mod ...` if needed, or just the file itself) — no logic until subsequent chunks.
