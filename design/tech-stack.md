# Tech Stack

## Guiding principles

- **Type safety** — catch bugs at compile time, not runtime
- **Dependency hygiene** — minimal, well-maintained dependencies
- **Web-first** — dashboard UI with real-time updates
- **File-based persistence** — no database, markdown is source of truth

---

## Backend

| | Choice |
|---|---|
| Language | Rust |
| Web framework | Axum |
| Async runtime | Tokio |
| Serialization | serde + serde_json (JSON-RPC, status.json, API bodies) |
| Static files | tower-http (ServeDir, SPA fallback) |
| Logging | tracing + tracing-subscriber |
| Configuration | config (Sergio Benitez) |
| WebSocket | axum::extract::ws (built-in) |
| JSON-RPC (ACP) | jsonrpsee |
| Markdown parsing | pulldown-cmark (backend) |
| Subprocess | tokio::process |
| File watching | notify |
| Error handling | thiserror + anyhow |
| Testing | cargo test (built-in) |

### Why Rust + Axum

Rust provides strong type safety and clean dependency management via Cargo. Axum offers ergonomic routing, native async support, and integrates naturally with the Tokio ecosystem needed for async subprocess and JSON-RPC work.

### Why jsonrpsee

ACP communicates over JSON-RPC 2.0 via stdio. While the envelope is simple, robust implementation requires handling request/response correlation, batch requests, notifications, error codes, and stdio framing. `jsonrpsee` is the most mature Rust JSON-RPC library and handles these edge cases.

### Future dependencies

- **`reqwest`** — HTTP client for remote agent support (ACP over HTTP/WebSocket)
- **Auth** — needed when accessing nexum over network (MVP is localhost-only)

### Markdown handling

Backend uses `pulldown-cmark` to parse structured data from markdown files (extract headings, fields, etc.). Frontend uses `markdown-it` to render markdown to HTML for display. Backend never renders HTML.

### Git operations

Subprocess calls to the system git binary. Not using a git library — the git CLI is the reference implementation and handles edge cases we don't want to reimplement.

---

## Frontend

| | Choice |
|---|---|
| Framework | Svelte (vanilla) |
| Router | svelte-spa-router |
| Package manager | npm |
| CSS | Tailwind CSS |
| Markdown rendering | markdown-it (frontend only) |
| Unit testing | vitest |
| E2E testing | playwright |

### Why vanilla Svelte

Nexum is a SPA dashboard with all API work handled by the Rust backend. Vanilla Svelte avoids unused SvelteKit server features (SSR, server functions) while keeping component reactivity and scoped styles. `svelte-spa-router` provides client-side routing.

### Why Tailwind

Utility-first CSS keeps the codebase smaller for common patterns while allowing custom CSS for complex components. Svelte's scoped styles work well with Tailwind.

---

## Architecture

```
Browser
    |
    | HTTP + WebSocket
    v
Axum (Rust API server)
    |
    +-- REST endpoints (plans, tasks, status)
    +-- WebSocket (real-time agent events)
    +-- Static file serving (Svelte frontend)
    |
    +-- File system (markdown plans/tasks, JSON state)
    |
    | JSON-RPC over stdio
    v
ACP-compatible agents (subprocess)
```

Nexum is a web application with a Rust API backend and Svelte frontend. The backend handles orchestration logic (git, ACP, file management) and serves the frontend as static files. Real-time agent events stream to the frontend via WebSocket.

---

## Build structure

```
nexum/
  Cargo.toml
  src/
    main.rs              # Binary entry point
    api/                 # HTTP routes
    acp/                 # ACP client (jsonrpsee)
    git/                 # Git operations (subprocess)
    persistence/         # File read/write
    config/              # Configuration
  static/                # Svelte build output
  web/                   # Svelte frontend
    package.json
    vite.config.ts
    src/
      routes/
      pages/
      components/
      lib/
```

Monorepo with Rust backend and Svelte frontend in the same repository.
