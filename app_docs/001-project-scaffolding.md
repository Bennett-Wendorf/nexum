# Project Scaffolding

## Overview

Nexum is an **AI agent orchestration command center** for software engineers. This document describes the initial project scaffolding — the foundational monorepo structure, build tooling, and module layout that supports all future development.

The scaffolding establishes a **Rust + Axum backend** paired with a **Svelte 5 + Vite + Tailwind frontend**, unified under a single repository with a `Makefile`-driven build system.

---

## Directory Structure

```
nexum/
├── Cargo.toml              # Rust package manifest
├── Cargo.lock              # Dependency lock file
├── Makefile                # Unified build commands
├── README.md               # Project description
├── LICENSE                 # Apache 2.0
├── .gitignore              # Git exclusions
├── src/                    # Rust backend source
│   ├── main.rs             # Binary entry point
│   ├── api/                # HTTP route handlers
│   │   └── mod.rs
│   ├── acp/                # ACP (Agent Communication Protocol) client
│   │   └── mod.rs
│   ├── git/                # Git operations (subprocess)
│   │   └── mod.rs
│   ├── persistence/        # File read/write operations
│   │   └── mod.rs
│   └── config/             # Application configuration
│       └── mod.rs
├── web/                    # Svelte frontend
│   ├── package.json        # Node dependencies & scripts
│   ├── index.html          # HTML entry point
│   ├── vite.config.ts      # Vite build configuration
│   ├── tailwind.config.js  # Tailwind CSS configuration
│   ├── postcss.config.js   # PostCSS configuration
│   ├── vitest.config.js    # Vitest unit test configuration
│   ├── playwright.config.js# Playwright E2E test configuration
│   ├── e2e/                # End-to-end test files
│   └── src/                # Frontend source
│       ├── main.js         # App mount point
│       ├── App.svelte      # Root component (router wrapper)
│       ├── app.css         # Tailwind directives
│       ├── routes/         # SPA route definitions
│       │   ├── index.js    # Route map
│       │   └── index.svelte
│       ├── pages/          # Page components
│       │   ├── Home.svelte
│       │   └── NotFound.svelte
│       ├── components/     # Reusable UI components
│       │   └── MarkdownViewer.svelte
│       ├── lib/            # Utility modules
│       │   └── markdown.js # Markdown-to-HTML renderer
│       └── __tests__/      # Unit tests
│           └── App.test.js
├── static/                 # Frontend build output (generated)
├── design/                 # Design documents & architecture specs
├── mockups/                # UI mockup images
├── target/                 # Rust build artifacts (git-ignored)
└── .agent/                 # Agent orchestration state & specs
```

---

## Backend Setup (Rust + Axum)

### Package Configuration

Defined in `Cargo.toml`:

| Setting | Value |
|---|---|
| Package name | `nexum` |
| Version | `0.1.0` |
| Edition | `2021` |

### Dependencies

| Crate | Purpose |
|---|---|
| `axum` 0.8 | Web framework (routing, handlers, WebSocket) |
| `tokio` 1 (full) | Async runtime |
| `serde` + `serde_json` | JSON serialization (API bodies, JSON-RPC) |
| `tower-http` | Static file serving, SPA fallback |
| `tracing` + `tracing-subscriber` | Structured logging |
| `config` 0.14 | Configuration management |
| `jsonrpsee` 0.24 | JSON-RPC 2.0 for ACP agent communication |
| `pulldown-cmark` 0.12 | Markdown parsing (structured data extraction) |
| `notify` 7 | File system watching |
| `thiserror` 2 + `anyhow` 1 | Error handling |

### Module Layout

| Module | Responsibility |
|---|---|
| `api` | HTTP endpoints (REST + WebSocket) |
| `acp` | ACP client — JSON-RPC over stdio to external agents |
| `git` | Git operations via subprocess calls to `git` CLI |
| `persistence` | File-based read/write (markdown plans, JSON state) |
| `config` | Application configuration loading |

### Entry Point

`src/main.rs` initializes the Tokio runtime and tracing subscriber, then delegates to module-specific startup logic (currently a placeholder).

---

## Frontend Setup (Svelte 5 + Vite + Tailwind)

### Package Configuration

Defined in `web/package.json`:

| Setting | Value |
|---|---|
| Package name | `nexum-web` |
| Version | `0.1.0` |
| Type | `module` (ESM) |

### Dependencies

| Package | Purpose |
|---|---|
| `svelte` ^5 | UI framework (runes reactivity) |
| `svelte-spa-router` ^4 | Client-side routing |
| `markdown-it` ^14 | Markdown-to-HTML rendering |

### Dev Dependencies

| Package | Purpose |
|---|---|
| `vite` ^6 | Build tool & dev server |
| `@sveltejs/vite-plugin-svelte` ^5 | Svelte + Vite integration |
| `tailwindcss` ^3.4 | Utility-first CSS framework |
| `autoprefixer` ^10.4 | CSS vendor prefixing |
| `postcss` ^8.4 | CSS processing pipeline |
| `vitest` ^3 | Unit testing framework |
| `jsdom` ^29 | DOM environment for unit tests |
| `@playwright/test` ^1.49 | End-to-end testing |

### Build Configuration (`vite.config.ts`)

- **Plugin**: Svelte compiler
- **Output directory**: `../static/` (served by Rust backend)
- **Dev server port**: `3000`
- **Empty output on build**: `true` (clean slate each build)

---

## Build Commands (Makefile)

| Target | Description |
|---|---|
| `make dev` | Starts both backend (`cargo run`) and frontend (`npm run dev`) dev servers |
| `make build` | Builds frontend to `static/`, then builds backend in release mode |
| `make test` | Runs `cargo test` (backend) and `npm test -- --run` (frontend unit tests) |
| `make clean` | Removes `target/`, `web/node_modules/`, and `static/` |

### Frontend NPM Scripts

| Script | Command |
|---|---|
| `npm run dev` | `vite` — dev server with HMR |
| `npm run build` | `vite build` — production build to `../static/` |
| `npm run preview` | `vite preview` — preview production build |
| `npm test` | `vitest` — unit tests |
| `npm run test:e2e` | `playwright test` — end-to-end tests |
| `npm run lint` | `eslint .` — linting |

---

## Testing Setup

### Unit Tests (Frontend)

- **Framework**: Vitest with jsdom environment
- **Configuration**: `web/vitest.config.js`
- **File pattern**: `src/**/__tests__/**/*.test.{js,ts}`
- **Svelte support**: Uses `@sveltejs/vite-plugin-svelte` for component transforms
- **Entry file**: `web/src/__tests__/App.test.js` (placeholder)

### End-to-End Tests (Frontend)

- **Framework**: Playwright
- **Configuration**: `web/playwright.config.js`
- **Test directory**: `web/e2e/`
- **Base URL**: `http://localhost:3000`
- **Browser**: Chromium (Desktop)
- **Auto-spawns dev server**: `npm run dev` before test run
- **CI mode**: 2 retries, `forbidOnly` enabled

### Backend Tests

- **Framework**: Rust built-in testing (`cargo test`)
- **Coverage**: Module-level tests for `api`, `acp`, `git`, `persistence`, `config`

---

## How to Run the Project

### Prerequisites

- **Rust** (stable, with `cargo`)
- **Node.js** (18+, with `npm`)
- **Git** (system binary, for git operations)

### Development Mode

```bash
# Single command starts both backend and frontend
make dev

# Or manually:
# Terminal 1 (backend):
cargo run

# Terminal 2 (frontend):
cd web && npm install && npm run dev
```

The frontend dev server runs at `http://localhost:3000`. The backend serves the built frontend from `static/` in production.

### Production Build

```bash
make build
```

This produces:
- Frontend assets in `static/`
- Backend binary in `target/release/nexum`

### Running Tests

```bash
# All tests (backend + frontend unit)
make test

# Frontend unit tests only
cd web && npm test -- --run

# Frontend E2E tests
cd web && npm run test:e2e

# Backend tests only
cargo test
```

### Cleaning Build Artifacts

```bash
make clean
```

---

## Key Components

### MarkdownViewer Component

`web/src/components/MarkdownViewer.svelte` — Renders markdown content to styled HTML using the `markdown-it` library. Uses Svelte 5 runes (`$props()`, `$derived()`) and includes scoped Tailwind-compatible prose styles.

### Markdown Utility

`web/src/lib/markdown.js` — Exports `renderMarkdown(text)` function that converts markdown strings to HTML. Configured with `html`, `linkify`, `typographer`, and `breaks` enabled.

### SPA Router

`web/src/routes/index.js` — Defines client-side routes:
- `/` → `Home.svelte`
- `*` → `NotFound.svelte` (catch-all 404)
