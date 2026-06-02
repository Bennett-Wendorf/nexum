# Operation structure

- API server (Rust/Axum) with documented REST + WebSocket endpoints (see backend-api.md)
- Bundled web dashboard (Svelte SPA served as static files)
- Can run on localhost like Cline Kanban for local work
- Docker container for remote management (research how OpenClaw does this?)
- TUI? Third-party client option via API
- Slack/Discord bots? Third-party client options via API
