# Agent Harness Integration

## Guiding principles

- **ACP is the foundation** — Agent Client Protocol is the standard interface between nexum and agents
- **ACP-only integration** — nexum speaks ACP exclusively; adapters for non-ACP agents are community-maintained and outside nexum scope
- **Nexum handles orchestration** — git, worktrees, status tracking, concurrency are nexum concerns, not agent concerns
- **Minimum capability requirement** — agents must support: start session, send messages, stream events, abort, detect completion

---

## Architecture

```
Nexum (ACP client)
    |
    | ACP (JSON-RPC over stdio or HTTP/WebSocket)
    v
Agent (ACP server)
    |
    +-- OpenCode  (native)
    +-- Kiro      (native)
    +-- Pi        (community adapter)
    +-- Claude    (community adapter)
    +-- Gemini    (native)
    +-- Cursor    (native)
    +-- ... any ACP-compatible agent
```

Nexum speaks ACP to all agents. Any agent that implements ACP (natively or via community adapter) works with nexum.

---

## ACP as the interface

ACP (Agent Client Protocol) standardizes communication between clients and coding agents. Nexum acts as the ACP client, agents are the ACP servers.

**Transport:**
- Local: JSON-RPC over stdio (subprocess, via `tokio::process` — see tech-stack.md)
- Remote: HTTP or WebSocket (future, via `reqwest`)

**JSON-RPC implementation:** `jsonrpsee` (see tech-stack.md for rationale)

**Core capabilities nexum relies on:**
- `initialize` — negotiate protocol version and capabilities
- `sessions/create` — start a new agent session
- `sessions/message` — send prompts, follow-ups, feedback to a running session
- `sessions/destroy` — end a session
- Streaming events — agent progress, tool calls, results, questions
- Permission handling — agent requests approval, nexum responds

**What ACP doesn't cover (nexum handles these):**
- Git branch and worktree management
- Task status tracking and state machine
- Concurrency limits and queue management
- Task dependencies and ordering
- Review workflow and merge process

---

## ACP requirement

Nexum supports any agent that implements ACP. Adapters for agents without native ACP support are built and maintained by the community, not by nexum. If an agent you want to use doesn't support ACP, an adapter should be created outside of nexum.

**Existing community adapters:**
- `pi-acp` — community adapter for Pi
- `claude-agent-acp` — Zed adapter wrapping Claude Agent SDK
- `codex-acp` — Zed adapter for OpenAI Codex CLI

---

## Role-to-session mapping

Different agent roles (see agent-roles.md) use ACP sessions differently:

- **Builder** — receives task prompt, executes code changes, streams tool calls and results
- **Reviewer** — receives diff or code for review, produces structured feedback
- **Planner** — receives scope description, generates task definitions (`task.md` files)
- **Security consultant** — like reviewer, focused on security analysis

Each role may require different session configurations (tool permissions, timeouts, model selection). The Overlord assigns roles to agents based on task requirements (see resource-constraints.md for concurrency limits, work-statuses.md for role transitions).

---

## Priority agents

| Agent | ACP support | Integration path |
|-------|------------|------------------|
| OpenCode | Native | `opencode acp` subprocess |
| Kiro | Native | `kiro acp` subprocess |
| Pi | Adapter | `pi-acp` community adapter |
| Claude Code | Adapter | Zed SDK adapter (wraps Agent SDK) |
| Gemini CLI | Native | `gemini acp` subprocess |
| Cursor | Native | `cursor acp` subprocess |

---

## Session lifecycle

Nexum manages the agent session lifecycle per task:

1. **Create** — nexum calls `sessions/create` with the task prompt
2. **Run** — agent works, streams events back to nexum
3. **Interact** — nexum can send follow-up messages, respond to permissions, ask questions
4. **Complete** — agent signals completion, nexum detects via event stream
5. **Destroy** — nexum calls `sessions/destroy`, cleans up worktree

**Error handling:**
- Agent crash or stale session — ACP progress events trigger heartbeat updates in `status.json` (see persistence.md recovery section). If events stop flowing, the Overlord detects stale heartbeat, destroys session, and re-queues the task
- Permission denied — nexum responds to agent's permission request, agent decides whether to proceed (see permissions.md)
- Timeout — configurable per task, nexum destroys session if exceeded

---

## Subprocess management

Agent subprocesses (see tech-stack.md: `tokio::process`) are managed as follows:

- **Spawn** — nexum launches agent binary with stdio pipes for JSON-RPC communication
- **Graceful shutdown** — send SIGTERM, wait up to 5 seconds, then SIGKILL
- **Zombie prevention** — `tokio::process::Child` handles are tracked and reaped on exit
- **Pipe management** — stdin/stdout used for JSON-RPC; stderr forwarded to logs (see tech-stack.md: tracing)
- **Crash detection** — subprocess exit without ACP destroy triggers stale heartbeat recovery

---

## Configuration

Agent configuration lives in `~/.config/nexum/config.toml` (see tech-stack.md: config crate), not in individual agent configs. This lets nexum control agent behavior consistently across tasks.

**Configurable per agent:**
- Binary path or adapter command
- Model selection (if agent supports multiple)
- Tool permissions (what tools the agent can use)
- Timeout limits
- Working directory (maps to task worktree)

**Configurable per task:**
- Which agent to use (builder, reviewer, planner roles)
- Task-specific tool restrictions
- Task-specific timeout

---

## Agent discovery

Agents are registered in nexum's config file. Each entry specifies the agent type, spawn command, and a human-readable name. Agent-specific configuration (API keys, models, OAuth) lives in the agent's own config, not in nexum. Nexum only knows how to spawn the process and communicate via ACP.

## Agent authentication

Agent auth (API keys, OAuth) is handled by the agent's own configuration, not by nexum. Nexum does not manage or forward credentials.

## Multi-instance support

Multiple instances of the same agent type can run in parallel. Each task gets its own subprocess, worktree, and ACP session. No coordination needed between instances — they're independent.

## Event relay

ACP events from agent subprocesses flow into a central event bus, which broadcasts them to the Svelte frontend via WebSocket. The frontend filters events by task/view client-side.

## Configuration location

Single config file at `~/.config/nexum/config.toml` (XDG conventions). Stores agent registrations, global settings, and preferences.
