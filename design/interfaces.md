# Interface(s?)

- Modular?
- The bundled web UI is just one consumer of the backend API. Third-party clients (Slack, Discord, custom UIs, CLI tools) connect via the same documented REST + WebSocket API (see backend-api.md).

## First iteration
- Create some mockups
- UI connects to the backend API, not directly to files or agents

## Ideas
- Kanban
- Mind map
- Hierarchical list

## Third-party integrations
The API must support these without backend modification:
- Slack bot (plan creation, status updates via Slack events)
- Discord bot (same as Slack, embed-formatted messages)
- Custom dashboards (any frontend framework)
- CLI tools (JSON-in, JSON-out)
- CI/CD pipelines (trigger plans, wait for completion) 
