# Persistence Layer

## Overview

The persistence layer is the file-based storage engine for the Nexum agent system. It manages all reading and writing of structured data under the `.agent/` directory hierarchy — plans, tasks, runtime status, and execution state — without any database dependency.

Nexum stores all work items as files on disk. The `.agent/` directory is split into two hierarchies:

| Directory | Purpose | Git-tracked |
|-----------|---------|-------------|
| `.agent/specs/` | Human-readable markdown specifications | Yes (committed) |
| `.agent/state/` | Ephemeral runtime state (JSON) | No (gitignored) |

Multiple agents may read and write these files concurrently, so the persistence layer provides **atomic writes** for all JSON state files to prevent partial reads, and an **optimistic locking** pattern to detect concurrent modifications.

---

## File Schemas

### `plan.md`

The plan markdown file lives at `.agent/specs/<branch>/<plan_id>-<slug>/plan.md`. It is the authoritative, human-readable specification for a plan.

```markdown
# Plan: oauth2-flow

**ID: PLAN-001**
**Status: planning**
**Created: 2026-05-21**
**Branch: feature/auth-overhaul**

## Goal

Implement OAuth2 authentication flow for the API gateway.

## Scope

- Token issuance and validation
- Refresh token rotation
- Scope-based authorization

## Background

The current auth system uses API keys which lack fine-grained access control.

## Tasks

- [ ] [TASK-001] implement-auth-middleware
- [x] [TASK-002] define-token-schema
```

**Fields extracted by the parser:**

| Field | Source |
|-------|--------|
| `id` | `**ID:**` metadata line |
| `name` | `# Plan: <name>` heading |
| `status` | `**Status:**` metadata line |
| `created` | `**Created:**` metadata line |
| `branch` | `**Branch:**` metadata line |
| `goal` | Content under `## Goal` |
| `scope` | Content under `## Scope` |
| `background` | Content under `## Background` |
| `tasks` | Checkbox list items (`- [ ] [TASK-NNN] name`) |

### `task.md`

The task markdown file lives at `.agent/specs/<branch>/<plan>/tasks/<task_id>-<slug>/task.md`.

```markdown
# TASK-001: implement-auth-middleware

**Parent plan: PLAN-001**
**Dependencies: TASK-002**

## Description

Build the authentication middleware that validates bearer tokens.

## Acceptance criteria

- Rejects requests without a valid token
- Extracts user ID from validated token
- Logs authentication failures

## Files to modify

- src/auth.rs
- src/middleware.rs

## Background

The middleware sits between the router and handlers.

## Notes

Use the oauth2 crate for token validation.
```

**Fields extracted by the parser:**

| Field | Source |
|-------|--------|
| `id` | `# TASK-<NNN>:` heading prefix |
| `name` | `# TASK-<NNN>: <name>` heading suffix |
| `parent_plan` | `**Parent plan:**` metadata line |
| `dependencies` | `**Dependencies:**` metadata line (comma-separated) |
| `description` | Content under `## Description` |
| `acceptance_criteria` | Bullet list under `## Acceptance criteria` |
| `files_to_modify` | Bullet list under `## Files to modify` |
| `background` | Content under `## Background` |
| `notes` | Content under `## Notes` |

### `status.json`

The task status file lives at `.agent/specs/<branch>/<plan>/tasks/<task_id>-<slug>/status.json`. It is written **atomically** and is **not** committed to git.

```json
{
  "id": "TASK-001",
  "status": "running",
  "agent": {
    "role": "builder",
    "pid": 12345,
    "leased_at": "2026-05-21T14:30:00+00:00"
  },
  "transitions": [
    {
      "from": "backlog",
      "to": "queued",
      "at": "2026-05-21T10:00:00+00:00",
      "by": "overlord"
    },
    {
      "from": "queued",
      "to": "running",
      "at": "2026-05-21T14:30:00+00:00",
      "by": "builder"
    }
  ],
  "started_at": "2026-05-21T14:30:00+00:00",
  "completed_at": null,
  "attempts": 1,
  "dependencies": ["TASK-002"],
  "dependent_tasks": [],
  "heartbeat_at": null
}
```

### `execution.json`

The execution state file lives at `.agent/state/<branch>/<plan_id>-<slug>/execution.json`. It is written **atomically** and is **not** committed to git.

```json
{
  "plan_id": "PLAN-001",
  "branch": "feature/auth-overhaul",
  "tasks": ["TASK-001", "TASK-002"],
  "task_status_map": {
    "TASK-001": "running",
    "TASK-002": "completed"
  }
}
```

---

## Directory Structure

```
<repo_root>/
└── .agent/
    ├── specs/                          # Committed to git
    │   └── <branch>/
    │       └── <plan_id>-<slug>/       # e.g., PLAN-001-oauth2-flow
    │           ├── plan.md
    │           └── tasks/
    │               └── <task_id>-<slug>/  # e.g., TASK-001-implement-auth
    │                   ├── task.md
    │                   └── status.json   # Gitignored
    │
    └── state/                          # Entirely gitignored
        └── <branch>/
            └── <plan_id>-<slug>/
                ├── execution.json
                └── logs/
                    └── <task_id>-<slug>/
```

### Naming Convention

All plan and task directories use the `<id>-<slug>` format:

| Example ID | Example Name | Resulting Slug |
|-----------|-------------|----------------|
| `PLAN-001` | `oauth2-flow` | `PLAN-001-oauth2-flow` |
| `TASK-003` | `OAuth2 Flow` | `TASK-003-oauth2-flow` |

The `slugify()` function lowercases text, replaces whitespace with hyphens, and strips non-alphanumeric characters (except hyphens and underscores).

---

## Atomic Write Mechanism

All JSON state files (`status.json`, `execution.json`) are written using an **atomic write** pattern to prevent partial reads during concurrent access.

### How It Works

1. A random temp filename is generated in the **same directory** as the target file: `{parent}/.tmp-{random:8}`
2. Content is written to the temp file using `fs::write`
3. `fs::rename(temp_path, target_path)` atomically replaces the target file
4. If rename fails, the temp file is cleaned up

### Why It Matters

On POSIX systems, `rename()` is atomic — the target file is replaced instantly. Any reader will see either the old content or the new content, never a partial write. This is critical because:

- Multiple agents may read `status.json` concurrently
- A crash during a non-atomic write could leave a corrupted JSON file
- The Overlord relies on consistent reads for stale lease detection

### Temp File Cleanup

If the initial `fs::write` to the temp file fails, the temp file is removed. If `fs::rename` fails, the temp file is also removed. This prevents orphaned `.tmp-*` files from accumulating.

---

## Rust API Reference

### Error Types (`errors`)

| Type | Description |
|------|-------------|
| `PersistenceError::Io(PathBuf, io::Error)` | File system I/O error with path context |
| `PersistenceError::IoBare(io::Error)` | Bare I/O error without path context |
| `PersistenceError::JsonParse(PathBuf, serde_json::Error)` | JSON deserialization error |
| `PersistenceError::JsonParseBare(serde_json::Error)` | Bare JSON parse error |
| `PersistenceError::MarkdownParse(PathBuf, String)` | Markdown parsing error |
| `PersistenceError::DirectoryNotFound(PathBuf)` | Expected directory missing |
| `PersistenceError::FileNotFound(PathBuf)` | Expected file missing |
| `PersistenceError::SchemaValidation(String)` | Schema validation failure |
| `PersistenceError::PathResolution(String)` | Path resolution error |
| `PersistenceError::AtomicWrite(PathBuf, io::Error)` | Atomic write (rename) failure |
| `PersistenceError::ConcurrencyConflict(PathBuf)` | File modified by another process |

The `Result<T>` type alias is defined as `std::result::Result<T, PersistenceError>`.

### Schema Types (`schema`)

| Type | Description |
|------|-------------|
| `Plan` | High-level plan grouping related tasks |
| `PlanStatus` | Plan lifecycle: `Backlog`, `Queued`, `Planning`, `Reviewing`, `PlanComplete` |
| `TaskReference` | Lightweight task reference (id, name, completed) |
| `Task` | Single task within a plan |
| `TaskStatus` | Runtime status of a task (stored in `status.json`) |
| `TaskStatusValue` | Task status: `Backlog`, `Queued`, `Running`, `Reviewing`, `WaitingManualReview`, `MergeQueue`, `Abandoned`, `Completed` |
| `AgentLease` | Active agent lease (role, pid, leased_at) |
| `StatusTransition` | Status change record (from, to, at, by) |
| `ExecutionState` | Plan-level execution state (stored in `execution.json`) |

All schema types derive `Serialize`, `Deserialize`, `Debug`, and `Clone`. Enum variants use `#[serde(rename_all = "kebab-case")]` for JSON serialization.

### File I/O (`io`)

| Function | Description |
|----------|-------------|
| `read_file(path)` | Read file contents as UTF-8 string |
| `write_file(path, content)` | Write string to file (non-atomic) |
| `atomic_write(path, content)` | Write file atomically (temp + rename) |
| `file_exists(path)` | Check if file exists |
| `directory_exists(path)` | Check if directory exists |
| `read_json::<T>(path)` | Read and deserialize JSON |
| `write_json(path, data)` | Serialize and write JSON (non-atomic) |
| `atomic_write_json(path, data)` | Serialize and atomically write JSON |
| `create_dir_all(path)` | Create directory and parents |
| `list_dir(path)` | List directory entries (sorted) |
| `remove_file(path)` | Remove a file |
| `remove_dir_all(path)` | Remove directory recursively |

### Markdown Parsing (`markdown`)

| Function | Description |
|----------|-------------|
| `parse_plan_markdown(path)` | Parse `plan.md` into a `Plan` struct |
| `parse_task_markdown(path)` | Parse `task.md` into a `Task` struct |
| `render_plan_markdown(plan)` | Render a `Plan` struct to markdown string |
| `render_task_markdown(task)` | Render a `Task` struct to markdown string |

### Directory Helpers (`directory`)

**Path Resolution:**

| Function | Returns |
|----------|---------|
| `agent_dir(repo_root)` | `<repo_root>/.agent/` |
| `specs_dir(repo_root, branch)` | `<repo_root>/.agent/specs/<branch>/` |
| `state_dir(repo_root, branch)` | `<repo_root>/.agent/state/<branch>/` |
| `plan_dir(repo_root, branch, plan_id, plan_name)` | `<specs>/<branch>/<id>-<slug>/` |
| `task_dir(repo_root, branch, plan_id, plan_name, task_id, task_name)` | `<plan>/tasks/<id>-<slug>/` |
| `plan_markdown_path(...)` | `<plan_dir>/plan.md` |
| `task_markdown_path(...)` | `<task_dir>/task.md` |
| `task_status_path(...)` | `<task_dir>/status.json` |
| `execution_state_path(...)` | `<state>/<branch>/<id>-<slug>/execution.json` |
| `task_log_dir(...)` | `<state>/<branch>/<plan>/logs/<task>/` |

**Directory Creation:**

| Function | Description |
|----------|-------------|
| `ensure_agent_dir(repo_root)` | Create `.agent/` directory |
| `ensure_plan_dir(...)` | Create plan dirs in both specs/ and state/ |
| `ensure_task_dir(...)` | Create task directory under plan |

**Directory Traversal:**

| Function | Description |
|----------|-------------|
| `list_branches(repo_root)` | List branch directories under specs/ |
| `list_plans(repo_root, branch)` | List plan directories under branch |
| `list_tasks(repo_root, branch, plan_id, plan_name)` | List task directories under plan |
| `find_plan_by_id(repo_root, branch, plan_id)` | Find plan directory by ID prefix |

**Utilities:**

| Function | Description |
|----------|-------------|
| `slugify(text)` | Convert text to kebab-case slug |

### High-Level Operations (`operations`)

**Plan Operations:**

| Function | Description |
|----------|-------------|
| `create_plan(repo_root, plan)` | Create plan.md + execution.json |
| `read_plan(repo_root, branch, plan_id, plan_name)` | Read and parse plan.md |
| `update_plan(repo_root, branch, plan_id, plan_name, plan)` | Rewrite plan.md |

**Task Operations:**

| Function | Description |
|----------|-------------|
| `create_task(repo_root, branch, plan_id, plan_name, task)` | Create task.md + status.json |
| `read_task(repo_root, branch, plan_id, plan_name, task_id, task_name)` | Read and parse task.md |
| `read_task_status(repo_root, branch, plan_id, plan_name, task_id, task_name)` | Read status.json |
| `update_task_status(repo_root, branch, plan_id, plan_name, task_id, task_name, new_status, by)` | Update task status with transition recording |

**Execution State Operations:**

| Function | Description |
|----------|-------------|
| `read_execution_state(repo_root, branch, plan_id, plan_name)` | Read execution.json |
| `update_execution_state(repo_root, branch, plan_id, plan_name, state)` | Write execution.json atomically |
| `add_task_to_execution(repo_root, branch, plan_id, plan_name, task_id, status)` | Add task to execution state |

---

## Markdown Parsing

### How It Works

The parser uses `pulldown-cmark`'s event-based iterator to walk through markdown events:

1. **H1 headings** — Extract plan name (`# Plan: <name>`) or task ID/name (`# TASK-001: <name>`)
2. **H2 headings** — Determine which section content belongs to (Goal, Scope, Description, etc.)
3. **Text events** — Collected into section buffers or used for metadata extraction
4. **List events** — Task list items parsed with a regex for checkbox format
5. **HTML events** — Also checked for metadata (some renderers output `<strong>` tags)

### Metadata Format

Metadata fields use the `**Key:** Value` pattern. The parser strips bold markers (`**` or `<strong>`) before matching.

### Assumptions and Limitations

- **Metadata must appear before section headings** — The parser does not look for metadata inside section content
- **Section content is concatenated text** — Formatting within sections (bold, links, code) is preserved as raw text
- **Missing sections return empty strings** — If `## Background` is absent, the field is `""`
- **Task list regex** — Matches `[ ] [TASK-NNN] description` or `[x] [TASK-NNN] description` patterns
- **Plan status defaults to `Backlog`** — Unknown status strings are parsed as `Backlog`
- **Acceptance criteria are single bullets** — Each bullet point becomes one string; sub-bullets are concatenated

---

## Error Handling

### `PersistenceError` Variants

| Variant | When It Occurs |
|---------|---------------|
| `Io(path, error)` | Any file system operation fails (read, write, create_dir, etc.) |
| `IoBare(error)` | I/O error without path context (from `?` operator on bare `io::Error`) |
| `JsonParse(path, error)` | JSON deserialization fails for a file at the given path |
| `JsonParseBare(error)` | JSON parse error without path context |
| `MarkdownParse(path, message)` | Markdown parsing encounters unexpected structure |
| `DirectoryNotFound(path)` | A function expected a directory to exist but it doesn't |
| `FileNotFound(path)` | `read_file` encounters `NotFound` error kind |
| `SchemaValidation(message)` | Schema validation fails (e.g., plan ID not found) |
| `PathResolution(message)` | Cannot resolve a path (e.g., no parent directory for atomic write) |
| `AtomicWrite(path, error)` | `fs::rename` fails during atomic write |
| `ConcurrencyConflict(path)` | Optimistic lock check detects file was modified by another process |

### `From` Implementations

- `From<io::Error>` → `PersistenceError::IoBare` — enables `?` operator with `std::fs` functions
- `From<serde_json::Error>` → `PersistenceError::JsonParseBare` — enables `?` operator with `serde_json` functions

---

## Concurrency

### Optimistic Locking (TOCTOU Mitigation)

The `update_task_status()` and `add_task_to_execution()` functions implement an optimistic locking pattern to mitigate Time-Of-Check-Time-Of-Use (TOCTOU) races:

1. **Read** the current file content and record it
2. **Parse** the content into a typed struct
3. **Modify** the struct in memory
4. **Re-read** the file to check if it has changed
5. **Compare** the new content with the original
6. **If different**, return `PersistenceError::ConcurrencyConflict`
7. **If same**, write the updated content atomically

This pattern ensures that if another agent modified the file between steps 1 and 4, the write is rejected rather than silently overwriting the other agent's changes.

### Atomic Writes

All JSON state files use `atomic_write_json()` which combines JSON serialization with the temp-file + rename pattern. This guarantees that readers never see partially-written JSON.

### File Write Strategy

| File Type | Write Method | Reason |
|-----------|-------------|--------|
| `plan.md` | Non-atomic (`write_file`) | Committed specs; no concurrent runtime reads |
| `task.md` | Non-atomic (`write_file`) | Committed specs; no concurrent runtime reads |
| `status.json` | Atomic (`atomic_write_json`) | Runtime state; read by multiple agents |
| `execution.json` | Atomic (`atomic_write_json`) | Runtime state; read by multiple agents |

---

## Status Machines

### Plan Status (5 variants)

```
Backlog → Queued → Planning → Reviewing → PlanComplete
```

| Status | Meaning |
|--------|---------|
| `backlog` | Not yet prioritized |
| `queued` | Queued for planning |
| `planning` | Actively being planned |
| `reviewing` | Under review |
| `plan-complete` | Fully specified, ready for execution |

### Task Status (8 variants)

```
Backlog → Queued → Running → Reviewing → WaitingManualReview → MergeQueue → Completed
                                              ↘ Abandoned (from any state)
```

| Status | Meaning |
|--------|---------|
| `backlog` | Not yet prioritized |
| `queued` | Queued for execution |
| `running` | Currently being worked on |
| `reviewing` | Under automated review |
| `waiting-manual-review` | Waiting for human review |
| `merge-queue` | In the merge queue |
| `abandoned` | Abandoned (can transition from any state) |
| `completed` | Successfully completed |

### Special Transitions

- **`started_at`** is set when transitioning to `Running`
- **`attempts`** is incremented on every transition to `Running`
- **`completed_at`** is set when transitioning to `Completed`
- All transitions are recorded in the `transitions` array with `from`, `to`, `at` (timestamp), and `by` (actor) fields

---

## Testing

### Test Infrastructure

All tests use `tempfile::tempdir()` to create isolated temporary directories, ensuring no interference with the real filesystem. The `temp_repo()` helper creates a temp directory and calls `.keep()` to prevent automatic cleanup during test execution.

### Test Coverage Summary

| Module | Tests | Coverage |
|--------|-------|----------|
| **Slugify** | `test_slugify_basic`, `test_slugify_special_chars` | Slug generation with special chars |
| **Path Resolution** | `test_agent_dir`, `test_specs_dir`, `test_state_dir`, `test_plan_dir`, `test_task_dir` | All path construction functions |
| **File I/O** | `test_read_write_file`, `test_atomic_write`, `test_read_json_not_found`, `test_write_read_json` | Read/write, atomic write, JSON round-trip, error cases |
| **Markdown Rendering** | `test_render_plan_markdown`, `test_render_task_markdown` | Struct-to-markdown rendering |
| **CRUD Operations** | `test_create_and_read_plan`, `test_create_and_read_task`, `test_update_task_status`, `test_add_task_to_execution` | Full lifecycle: create → read → update |
| **Directory Traversal** | `test_list_branches`, `test_find_plan_by_id` | Directory listing and lookup |

### Key Test Scenarios

- **Status transitions**: Verifies `started_at` is set on `Running` transition, `completed_at` on `Completed`, and `attempts` increments
- **Transition recording**: Verifies transitions array grows correctly with each status change
- **Atomic write**: Verifies content is correctly written via temp + rename pattern
- **JSON round-trip**: Verifies `write_json` → `read_json` produces equivalent data
- **Concurrent modification detection**: The `ConcurrencyConflict` error path is exercised through the `update_task_status` and `add_task_to_execution` implementations

---

## Dependencies

| Crate | Purpose |
|-------|---------|
| `serde` + `serde_json` | JSON serialization/deserialization |
| `pulldown-cmark` | Markdown parsing |
| `thiserror` | Error type derivation |
| `chrono` | Timestamp generation (`Utc::now().to_rfc3339()`) |
| `fastrand` | Random temp filename generation |
| `regex` | Task list item parsing |
| `tempfile` (dev) | Test isolation |

---

## Files

| File | Purpose |
|------|---------|
| `src/persistence/mod.rs` | Module root, re-exports all public types and functions |
| `src/persistence/errors.rs` | `PersistenceError` enum and `Result<T>` type alias |
| `src/persistence/schema.rs` | Typed structs: `Plan`, `Task`, `TaskStatus`, `ExecutionState`, etc. |
| `src/persistence/io.rs` | File I/O: `read_file`, `write_file`, `atomic_write`, JSON helpers |
| `src/persistence/markdown.rs` | Markdown parsing and rendering for plans and tasks |
| `src/persistence/directory.rs` | Path resolution, directory creation, traversal helpers |
| `src/persistence/operations.rs` | High-level CRUD: `create_plan`, `read_task`, `update_task_status`, etc. |
| `src/persistence/tests.rs` | Unit tests for all modules |
