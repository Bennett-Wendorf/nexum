# Technology Stack

## Guiding Principles

- **Type safety** — Catch bugs at compile time, not runtime
- **Dependency hygiene** — Minimal, well-maintained dependencies
- **Web-first** — Dashboard UI with real-time updates
- **File-based persistence** — No database; markdown is the source of truth

---

## Architecture Overview

```
Browser
    │
    │ HTTP + WebSocket
    ▼
Axum (Rust API Server)
    │
    ├── REST endpoints (plans, tasks, status)
    ├── WebSocket (real-time agent events)
    └── Static file serving (Svelte frontend)
    │
    ├── File system (markdown plans/tasks, JSON state)
    │
    │ JSON-RPC over stdio
    ▼
ACP-compatible agents (subprocess)
```

Nexum is a web application with a Rust API backend and Svelte frontend. The backend handles orchestration logic (git, ACP, file management) and serves the frontend as static files. Real-time agent events stream to the frontend via WebSocket.

---

## Backend

| Layer | Technology | Version |
|---|---|---|
| Language | Rust | Edition 2021 |
| Web Framework | Axum | 0.8 |
| Async Runtime | Tokio | 1 (full features) |
| Serialization | serde + serde_json | 1 |
| Static Files | tower-http | 0.6 |
| Logging | tracing + tracing-subscriber | 0.1 / 0.3 |
| Configuration | config | 0.14 |
| JSON-RPC (ACP) | jsonrpsee | 0.24 |
| Markdown Parsing | pulldown-cmark | 0.12 |
| File Watching | notify | 7 |
| Error Handling | thiserror + anyhow | 2 / 1 |

### Rationale

**Rust + Axum** — Rust provides strong type safety, zero-cost abstractions, and clean dependency management via Cargo. Axum offers ergonomic routing, native async support, and integrates naturally with the Tokio ecosystem needed for async subprocess management and JSON-RPC communication.

**jsonrpsee** — ACP (Agent Communication Protocol) communicates over JSON-RPC 2.0 via stdio. Robust implementation requires handling request/response correlation, batch requests, notifications, error codes, and stdio framing. `jsonrpsee` is the most mature Rust JSON-RPC library and handles these edge cases.

**pulldown-cmark** — Used on the backend to parse structured data from markdown files (extract headings, fields, etc.). The backend never renders HTML — markdown rendering is a frontend concern.

**Git via subprocess** — Uses direct subprocess calls to the system `git` binary rather than a git library. The git CLI is the reference implementation and handles edge cases we don't want to reimplement.

### Planned Future Dependencies

- **`reqwest`** — HTTP client for remote agent support (ACP over HTTP/WebSocket)
- **Auth library** — Needed when accessing Nexum over a network (MVP is localhost-only)

---

## Frontend

| Layer | Technology | Version |
|---|---|---|
| Framework | Svelte (vanilla) | 5 |
| Router | svelte-spa-router | 4.0 |
| Build Tool | Vite | 6 |
| CSS Framework | Tailwind CSS | 3.4 |
| PostCSS | postcss + autoprefixer | 8.4 / 10.4 |
| Markdown Rendering | markdown-it | 14 |
| Package Manager | npm | — |

### Rationale

**Vanilla Svelte (not SvelteKit)** — Nexum is a SPA dashboard with all API work handled by the Rust backend. Vanilla Svelte avoids unused SvelteKit server features (SSR, server functions) while keeping component reactivity, scoped styles, and the new Svelte 5 runes API (`$props()`, `$derived()`).

**svelte-spa-router** — Provides lightweight client-side routing without the overhead of a full framework router. Fits the SPA architecture where the backend serves static files with SPA fallback.

**Tailwind CSS** — Utility-first CSS keeps the codebase smaller for common patterns while allowing custom CSS for complex components. Svelte's scoped styles work well with Tailwind's class-based approach.

**markdown-it (frontend only)** — Renders markdown to HTML for display in the UI. The backend uses `pulldown-cmark` for parsing structured data; the frontend uses `markdown-it` for rendering. Each side uses the tool best suited for its role.

---

## Testing

| Layer | Technology | Purpose |
|---|---|---|
| Backend | `cargo test` (built-in) | Unit & integration tests |
| Frontend Unit | Vitest + jsdom | Component & utility tests |
| Frontend E2E | Playwright | Browser-level integration tests |

### Rationale

**Vitest** — Shares Vite's configuration and plugin ecosystem, enabling seamless Svelte component testing with minimal setup. The jsdom environment provides a browser-like DOM for component rendering.

**Playwright** — Full browser automation for end-to-end testing. Auto-spawns the dev server, supports multiple browsers, and provides reliable cross-platform test execution.

---

## Build System

| Tool | Purpose |
|---|---|
| Makefile | Unified entry point for dev, build, test, and clean |
| Cargo | Rust dependency management and compilation |
| npm + Vite | Frontend dependency management and bundling |

The `Makefile` provides convenience wrappers (`dev`, `build`, `test`, `clean`) that coordinate both the Rust backend and Node frontend toolchains from a single interface.

---

## Repository Structure

Monorepo layout with Rust backend and Svelte frontend in the same repository:

- `src/` — Rust source code
- `web/` — Svelte frontend source
- `static/` — Frontend build output (generated, not committed)
- `design/` — Architecture design documents
- `mockups/` — UI design mockups
- `.agent/` — Agent orchestration specs and state
