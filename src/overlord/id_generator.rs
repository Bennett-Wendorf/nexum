//! Sequential ID generation for plans and tasks.
//!
//! `PlanIdGenerator` tracks the next plan ID per branch by scanning existing
//! plan directories. `TaskIdGenerator` tracks the next task ID per plan by
//! scanning existing task directories. IDs are stable — once assigned, they
//! never change.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use super::errors::{OverlordError, Result};
use crate::persistence::{specs_dir, plan_dir};

static PLAN_ID_DIR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^PLAN-(\d{3})-").unwrap());
static PLAN_ID_PARSE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^PLAN-(\d{3})$").unwrap());
static TASK_ID_DIR_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^TASK-(\d{3})-").unwrap());
static TASK_ID_PARSE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^TASK-(\d{3})$").unwrap());

// ── Plan ID Generator ───────────────────────────────────────────────────────

/// Generates stable sequential plan IDs per branch.
pub struct PlanIdGenerator;

impl PlanIdGenerator {
    /// Generate the next plan ID for a branch.
    ///
    /// Scans `.agent/specs/<branch>/` for existing plan directories matching
    /// `PLAN-<NNN>-*` pattern, finds the maximum NNN, and returns the next
    /// sequential ID zero-padded to 3 digits.
    pub fn next_plan_id(repo_root: &Path, branch: &str) -> Result<String> {
        let specs = specs_dir(repo_root, branch);
        if !specs.exists() {
            return Ok("PLAN-001".to_string());
        }

        let entries: Vec<u32> = std::fs::read_dir(&specs)
            .map_err(|e| OverlordError::IdGenerationError(e.to_string()))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .filter_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
            .filter_map(|name| {
                PLAN_ID_DIR_RE.captures(&name)
                    .and_then(|caps| caps.get(1))
                    .and_then(|m| m.as_str().parse::<u32>().ok())
            })
            .collect();

        let max_num = entries.into_iter().max().unwrap_or(0);

        Ok(format!("PLAN-{:03}", max_num + 1))
    }

    /// Parse a plan ID like `PLAN-005` to extract the numeric portion (5).
    pub fn parse_plan_id(id: &str) -> Result<u32> {
        let caps = PLAN_ID_PARSE_RE.captures(id).ok_or_else(|| {
            OverlordError::IdGenerationError(format!("Invalid plan ID format: {}", id))
        })?;
        caps.get(1)
            .and_then(|m| m.as_str().parse::<u32>().ok())
            .ok_or_else(|| {
                OverlordError::IdGenerationError(format!("Invalid plan ID number: {}", id))
            })
    }
}

// ── Task ID Generator ───────────────────────────────────────────────────────

/// Generates stable sequential task IDs per plan.
pub struct TaskIdGenerator;

impl TaskIdGenerator {
    /// Generate the next task ID for a plan.
    ///
    /// Scans `.agent/specs/<branch>/<plan_id>-<plan_name>/tasks/` for existing
    /// task directories matching `TASK-<NNN>-*` pattern, finds the maximum NNN,
    /// and returns the next sequential ID zero-padded to 3 digits.
    pub fn next_task_id(
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<String> {
        let tasks_dir = plan_dir(repo_root, branch, plan_id, plan_name).join("tasks");

        if !tasks_dir.exists() {
            return Ok("TASK-001".to_string());
        }

        let entries: Vec<u32> = std::fs::read_dir(&tasks_dir)
            .map_err(|e| OverlordError::IdGenerationError(e.to_string()))?
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false))
            .filter_map(|entry| entry.file_name().to_str().map(|s| s.to_string()))
            .filter_map(|name| {
                TASK_ID_DIR_RE.captures(&name)
                    .and_then(|caps| caps.get(1))
                    .and_then(|m| m.as_str().parse::<u32>().ok())
            })
            .collect();

        let max_num = entries.into_iter().max().unwrap_or(0);

        Ok(format!("TASK-{:03}", max_num + 1))
    }

    /// Parse a task ID like `TASK-010` to extract the numeric portion (10).
    pub fn parse_task_id(id: &str) -> Result<u32> {
        let caps = TASK_ID_PARSE_RE.captures(id).ok_or_else(|| {
            OverlordError::IdGenerationError(format!("Invalid task ID format: {}", id))
        })?;
        caps.get(1)
            .and_then(|m| m.as_str().parse::<u32>().ok())
            .ok_or_else(|| {
                OverlordError::IdGenerationError(format!("Invalid task ID number: {}", id))
            })
    }
}

// ── Slug Utilities ──────────────────────────────────────────────────────────

/// Convert text into a URL/path-safe kebab-case slug.
///
/// Lowercases the text, replaces spaces with hyphens, and removes all
/// characters except alphanumeric, hyphens, and underscores.
pub fn slugify(text: &str) -> String {
    crate::persistence::slugify(text)
}

/// Build a directory name from ID and slug: `<id>-<slug>`.
///
/// E.g., `format_dir_name("PLAN-001", "oauth-flow")` → `"PLAN-001-oauth-flow"`
pub fn format_dir_name(id: &str, slug: &str) -> String {
    format!("{}-{}", id, slug)
}
