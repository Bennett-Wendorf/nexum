# Backend API

## Core principle

The entire backend API is the public contract. The bundled web UI is just one consumer. Any third party must be able to build their own UI, Slack integration, Discord bot, CLI tool, or any other client using only the documented API.

This principle applies to all backend design decisions:

- Every feature reachable from the web UI must be reachable via API
- API responses are never UI-coupled (no HTML, no frontend-specific formatting)
- API endpoints are versioned and documented before implementation
- Breaking changes to the API require explicit version bumps, not silent changes

## Documentation requirement

Every API endpoint must be documented with:

- HTTP method and path
- Request body schema (JSON, with types and required/optional fields)
- Query parameters and path parameters
- Response body schema (JSON, with types and required/optional fields)
- All possible HTTP status codes and their meanings
- Authentication requirements (if any)
- Example request and response

Documentation lives as a machine-readable OpenAPI 3.1 spec at `docs/api/openapi.json`. Human-readable docs are generated from the spec.

## API surface

### Plans

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/v1/plans` | List all plans |
| `GET` | `/api/v1/plans/{plan_id}` | Get plan details |
| `POST` | `/api/v1/plans` | Create a new plan |
| `PATCH` | `/api/v1/plans/{plan_id}` | Update plan |
| `DELETE` | `/api/v1/plans/{plan_id}` | Delete plan |
| `POST` | `/api/v1/plans/{plan_id}/plan` | Trigger planner agent |
| `POST` | `/api/v1/plans/{plan_id}/approve` | Approve plan for execution |
| `POST` | `/api/v1/plans/{plan_id}/reject` | Reject plan |

### Tasks

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/v1/plans/{plan_id}/tasks` | List tasks for a plan |
| `GET` | `/api/v1/tasks/{task_id}` | Get task details |
| `PATCH` | `/api/v1/tasks/{task_id}` | Update task |
| `POST` | `/api/v1/tasks/{task_id}/queue` | Move task to queued status |
| `POST` | `/api/v1/tasks/{task_id}/abandon` | Abandon task |

### Execution

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/v1/plans/{plan_id}/execution` | Get execution state |
| `GET` | `/api/v1/agents` | List active agents |
| `GET` | `/api/v1/agents/{agent_id}` | Get agent details |
| `GET` | `/api/v1/agents/{agent_id}/logs` | Get agent run logs |

### System

| Method | Path | Description |
|---|---|---|
| `GET` | `/api/v1/status` | System health and summary |
| `GET` | `/api/v1/config` | Current configuration |

### Real-time

| Protocol | Path | Description |
|---|---|---|
| WebSocket | `/api/v1/ws/events` | Stream real-time agent events |
| WebSocket | `/api/v1/ws/agent/{agent_id}` | Stream single agent events |

## WebSocket event schema

All WebSocket events follow a consistent envelope:

```json
{
  "type": "string",
  "timestamp": "ISO-8601",
  "payload": {}
}
```

Event types:

- `plan.status_changed` — Plan status transition
- `task.status_changed` — Task status transition
- `task.queued` — Task entered queue
- `task.running` — Builder started working
- `task.completed` — Task finished execution
- `task.reviewing` — Reviewer started
- `task.review_passed` — Review approved
- `task.review_failed` — Review rejected
- `task.merged` — Task branch merged into plan branch
- `agent.started` — Agent subprocess launched
- `agent.stopped` — Agent subprocess exited
- `agent.log_line` — Agent output line
- `agent.token_usage` — Token usage update
- `system.heartbeat` — Periodic heartbeat

## Authentication

Nexum supports optional API key authentication. When enabled, write
operations (POST, PUT, PATCH, DELETE) require a valid API key. Read
operations (GET) are open by default but can be protected via config.

### Configuration

In `~/.config/nexum/config.toml`:

```toml
[global.authentication]
enabled = true
authenticate_read = false
api_keys = [
  { name = "cli", secret = "your-api-key-here" },
  { name = "slack-bot", secret = "another-api-key" },
]
```

### Usage

Include the API key in the `Authorization` header:

```
Authorization: Bearer your-api-key-here
```

### Endpoints

- `GET /api/v1/auth/status` — Check auth requirements (unauthenticated)
- All write endpoints — Require auth when `enabled=true`
- All read endpoints — Require auth when `enabled=true` AND `authenticate_read=true`

### Default (MVP)

Authentication is disabled by default. All endpoints are publicly accessible
on localhost. Enable authentication for remote or production deployments.

## Versioning

- URL-based versioning: `/api/v1/...`
- Major version bump for breaking changes
- Minor additions (new fields, new endpoints) within same version are non-breaking
- Deprecation: endpoints are marked deprecated in docs for at least one major version before removal

## Third-party integration examples

The API must support these use cases without modification:

- **Slack bot**: User posts a plan description, bot creates plan via API, posts status updates on transitions
- **Discord bot**: Same as Slack, with embed-formatted status messages
- **Custom dashboard**: Frontend framework of choice consuming REST + WebSocket
- **CLI tool**: JSON-in, JSON-out for scripting and automation
- **CI/CD pipeline**: Trigger plan approval, wait for completion, report results
