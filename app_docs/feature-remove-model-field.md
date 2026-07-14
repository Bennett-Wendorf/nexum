# Breaking Config Change: Removed `model` Field from `AgentRegistration`

## Overview

The `model: Option<String>` field has been removed from the `AgentRegistration` configuration schema. Model selection is now handled entirely by the ACP (Agent Client Protocol) during session setup — the agent advertises available models, and the client picks one.

## What Was Built

The `model` field was removed from every layer of the codebase that referenced it:

- **Config schema** — `AgentRegistration` struct no longer declares a `model` field
- **API response** — `AgentRegistrationResponse` DTO no longer includes `model`
- **API handler** — The `list_agents` endpoint no longer maps `model`
- **Default config template** — New default config templates do not include model commentary
- **Example config** — `examples/config.toml` no longer contains `model = ...` lines
- **OpenAPI spec** — `AgentRegistrationResponse` schema no longer documents the `model` property
- **Tests** — All test fixtures updated to remove `model` references

## Technical Implementation

### Files Modified

| File | Change |
|---|---|
| `src/config/schema.rs` | Removed `model: Option<String>` field from `AgentRegistration` |
| `src/config/loader.rs` | Removed `# model = "llama3.1"` comment from `DEFAULT_CONFIG_TEMPLATE` |
| `src/config/tests.rs` | Removed all `model` field references from test fixtures and assertions |
| `src/api/types.rs` | Removed `model: Option<String>` from `AgentRegistrationResponse` |
| `src/api/config.rs` | Removed `model` mapping from `list_agents` handler; updated doc comment |
| `examples/config.toml` | Removed `model = ...` lines from all agent entries |
| `docs/api/openapi.json` | Removed `model` property from `AgentRegistrationResponse` schema |

### Why This Change

The `model` field hardcoded a model name (e.g., `"llama3.1"`) per agent registration. This was fragile because:

- Different ACP agents expose different model names
- Model names change over time (e.g., `"llama3.1"` → `"llama3.2"`)
- The ACP spec already provides a proper mechanism for model selection via session config options
- Hardcoding model names created unnecessary coupling between nexum and agent-specific details

### How Model Selection Works Now

Model selection is handled by the ACP protocol during session setup:

1. The agent advertises its available models via the ACP session config
2. The client (nexum) picks a model from the advertised list
3. The selected model is passed as a session-level `model_preference` override

This is implemented in `src/acp/config.rs` where `task_overrides.model_preference` is applied to the session config.

## Migration Notes

### Impact

This is technically a **breaking change** for users who have `model = ...` in their `~/.config/nexum/config.toml`.

### No Action Required

**No migration is needed.** Because the removed field was annotated with `#[serde(default)]`, serde will silently ignore the `model` key on deserialization. Existing configs that contain `model = "..."` will still load without error — the value will simply be ignored.

If you want to clean up your config, you can optionally remove the `model = ...` lines from your `~/.config/nexum/config.toml` file. The lines are harmless and will continue to work.

### Before (old config)
```toml
[[agents]]
name = "my-builder"
type = "opencode"
spawn_command = "opencode acp"
model = "llama3.1"          # ← ignored after this change
```

### After (cleaned up)
```toml
[[agents]]
name = "my-builder"
type = "opencode"
spawn_command = "opencode acp"
```

## API Changes

The `GET /api/agents` endpoint response no longer includes a `model` field in each agent registration object.

### Before
```json
{
  "name": "my-builder",
  "type": "opencode",
  "spawn_command": "opencode acp",
  "model": "llama3.1",
  "tool_permissions": ["read", "write"],
  "timeout_seconds": 1800
}
```

### After
```json
{
  "name": "my-builder",
  "type": "opencode",
  "spawn_command": "opencode acp",
  "tool_permissions": ["read", "write"],
  "timeout_seconds": 1800
}
```
