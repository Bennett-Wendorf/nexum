# Markdown Round-Trip Fix (Spec 019)

## Overview

Fixed a format inconsistency between `render_plan_markdown` and `parse_plan_markdown` in the persistence layer's markdown rendering. The renderer was outputting metadata with values inside bold markers (`**ID: PLAN-001**`), while the documented spec and parser expected values outside the bold markers (`**ID:** PLAN-001`). A round-trip test was added to guarantee `parse(render(plan))` preserves all field values.

## What Was Built

- **Format fix**: Updated four `format!` calls in `render_plan_markdown` so metadata lines follow the `**Label:** value` pattern, matching the template documented in `design/persistence.md`.
- **Heading detection fix**: The H2 heading detection was switched from relying on `pulldown_cmark`'s `heading_id` field (which is always `None` in version 0.12) to collecting heading text content and matching by lowercase text. This fix was applied to both plan and task parsing.
- **Round-trip test**: Added `test_plan_markdown_roundtrip` to verify that rendering a plan to markdown then parsing it back produces identical field values.

## Technical Implementation

### Files Modified

| File | Changes |
|------|---------|
| `src/persistence/markdown.rs` | Fixed four metadata format strings in `render_plan_markdown` (lines 386–392). H2 section detection uses heading text matching instead of `heading_id`. |
| `src/persistence/tests.rs` | Added `test_plan_markdown_roundtrip` (lines 147–196). |

### Format Changes (render_plan_markdown)

| Before | After |
|--------|-------|
| `**ID: {}**` | `**ID:** {}` |
| `**Status: {}**` | `**Status:** {}` |
| `**Created: {}**` | `**Created:** {}` |
| `**Branch: {}**` | `**Branch:** {}` |

### Heading Detection Fix

`pulldown_cmark` 0.12 always reports `heading_id` as `None` in the `Tag::Heading` variant. The original code relied on `heading_id` to determine which section (Goal, Scope, Background) a heading started. The fix collects heading text via `Event::Text` while inside a heading, then matches the lowercase text against known section names:

- Plan parsing: `"goal"`, `"scope"`, `"background"`
- Task parsing: `"description"`, `"acceptance criteria"`, `"files to modify"`, `"background"`, `"notes"`

### Round-Trip Test (`test_plan_markdown_roundtrip`)

The test:
1. Creates a `Plan` with known values (ID, name, status, created, branch, goal, scope, background, and two task references with mixed completion states).
2. Renders it to markdown via `render_plan_markdown`.
3. Writes the markdown to a temporary file.
4. Parses it back via `parse_plan_markdown`.
5. Asserts all fields match the original — id, name, status, created, branch, goal, scope, background, and each task reference's id, name, and completed flag.

## Validation Results

- `cargo test` — all tests pass, including the new `test_plan_markdown_roundtrip`
- `cargo clippy` — no warnings
- Rendered plan markdown matches the template format in `design/persistence.md`
- Round-trip test exercises all metadata fields and task references

## Notes

- The `render_task_markdown` function still uses the old format (`**Parent plan: {}**`, `**Dependencies: {}**`). This is tracked as a follow-up item.
- The `extract_metadata_value` parser already handled both formats (it strips `**` before matching), so the fix is about conformance to the documented spec rather than fixing a parsing bug.
