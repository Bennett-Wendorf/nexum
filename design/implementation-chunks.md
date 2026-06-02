# Implementation Chunks

Branch-level implementation chunks, each independently plannable.

---

## MVP Critical

### 1. Project Scaffolding
Rust + Svelte monorepo setup, Cargo/Axum backend, Vite/Svelte frontend, build tooling, directory structure per `tech-stack.md`

### 2. Configuration System
`~/.config/nexum/config.toml`, agent registration, global settings (from `agent-harness-integration.md`)

### 3. Persistence Layer
File I/O, atomic writes, `.agent/` directory structure, `plan.md`/`task.md`/`status.json`/`execution.json` schemas (from `persistence.md`)

### 4. Overlord Deterministic Core
Status machines (plan + task), ID generation, concurrency enforcement, dependency auto-queue, stale heartbeat detection (from `agent-roles.md`, `work-statuses.md`, `resource-constraints.md`)

### 5. Git Operations
Branch creation, worktree spawn/cleanup, task-branch-into-plan-branch merging, dependency-ordered merge (from `persistence.md` branch strategy)

### 6. ACP Client
JSON-RPC over stdio via `jsonrpsee`, subprocess management via `tokio::process`, session lifecycle (create/run/destroy), event streaming (from `agent-harness-integration.md`)

### 7. REST API + OpenAPI Spec
Plan/task CRUD, status transitions, execution state endpoints via Axum. Every endpoint documented in OpenAPI 3.1 spec (from `backend-api.md`, `tech-stack.md` architecture)

### 8. Frontend Dashboard
Svelte SPA: plan/task list views, status display, basic markdown rendering (from `interfaces.md`, `tech-stack.md`)

### 9. Builder Workflow
End-to-end: assign queued task → spawn agent worktree → ACP session → execute → detect completion → merge branch → cleanup

---

## Future Enhancements

### 10. Planner Integration
AI planning workflow: user draft → planner agent → generates `task.md` files → plan review

### 11. Reviewer Integration
Post-execution review workflow, `waiting-manual-review` status handling

### 12. Overlord Agent Layer
LLM-powered user interaction, prioritization, exception handling (from `agent-roles.md`)

### 13. WebSocket Event Streaming
Real-time agent events via axum WS. Event envelope and types documented in OpenAPI spec (from `backend-api.md`, `agent-harness-integration.md` event relay)

### 14. Remote Agent Support
ACP over HTTP/WebSocket via `reqwest`

### 15. Docker Support
Remote management container (from `operation-structure.md`)

### 16. Yolo Mode
Permission bypass for expert users (from `permissions.md`)

### 17. Scratchpad Agent
Ad-hoc tasks outside main workflow (from `agent-roles.md`)

### 18. TUI
Terminal UI option (from `operation-structure.md`)

### 19. Third-party Integration Examples
Reference integrations: Slack bot, Discord bot, CLI tool (validates API is usable by non-web clients)
