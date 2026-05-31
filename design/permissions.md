# Permissions

## Default: permission gates

Agents that support ACP permission requests must prompt for approval before performing restricted actions (file writes, command execution, network requests). This is the default behavior.

## Yolo mode (future)

Configurable option to bypass permission prompts for expert users who trust their agents. Disabled by default.

## User models

- **Novice** — wants permission gates on all actions, monitors individual tasks
- **Expert** — may want broader permissions, fewer prompts (yolo mode candidate)
