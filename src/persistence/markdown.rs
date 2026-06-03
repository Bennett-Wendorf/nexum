//! Markdown parsing and rendering for plans and tasks.
//!
//! Uses [`pulldown_cmark`] to extract structured data from the markdown
//! files stored under `.agent/specs/` and to render structured data back
//! into markdown format.

use std::path::Path;

use pulldown_cmark::{Event, HeadingLevel, Parser, Tag, TagEnd};
use regex::Regex;

use super::errors::Result;
use super::io::read_file;
use super::schema::{Plan, PlanStatus, Task, TaskReference};

// ── Plan parsing ────────────────────────────────────────────────────────────

/// Parse a plan markdown file into a [`Plan`] struct.
///
/// Expected structure:
/// - `# Plan: <name>` heading for the plan name
/// - `**ID:**`, `**Status:**`, `**Created:**`, `**Branch:**` metadata lines
/// - `## Goal`, `## Scope`, `## Background` sections
/// - Task list items like `- [ ] [TASK-001] description`
pub fn parse_plan_markdown(path: &Path) -> Result<Plan> {
    let content = read_file(path)?;
    let parser = Parser::new(&content);

    let mut name = String::new();
    let mut id = String::new();
    let mut status = PlanStatus::Backlog;
    let mut created = String::new();
    let mut branch = String::new();
    let mut goal = String::new();
    let mut scope = String::new();
    let mut background = String::new();
    let mut tasks: Vec<TaskReference> = Vec::new();

    let mut current_section = Section::None;
    let mut section_buffer = String::new();

    // For text-based task list parsing
    let in_list = &mut false;
    let mut list_item_text = String::new();
    let task_list_re = Regex::new(r"^\[([ xX])\]\s+\[([^\]]+)\]\s+(.+)$").unwrap();

    for event in parser {
        match event {
            Event::Start(Tag::Heading {
                level: HeadingLevel::H1,
                ..
            }) => {
                current_section = Section::None;
            }
            Event::End(TagEnd::Heading(HeadingLevel::H1)) => {}

            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                id: heading_id,
                ..
            }) => {
                // Flush previous section
                flush_section(&mut current_section, &section_buffer, &mut goal, &mut scope, &mut background);
                section_buffer.clear();

                // Determine which section this heading starts
                let id_str = heading_id
                    .as_ref()
                    .map(|s| s.to_string().to_lowercase())
                    .unwrap_or_default();
                if id_str == "goal" {
                    current_section = Section::Goal;
                } else if id_str == "scope" {
                    current_section = Section::Scope;
                } else if id_str == "background" {
                    current_section = Section::Background;
                }
            }

            // Section content buffering
            Event::Text(text) if current_section == Section::Goal => {
                section_buffer.push_str(&text);
            }
            Event::Text(text) if current_section == Section::Scope => {
                section_buffer.push_str(&text);
            }
            Event::Text(text) if current_section == Section::Background => {
                section_buffer.push_str(&text);
            }

            // List handling for task list items
            Event::Start(Tag::List(_)) => {
                *in_list = true;
                list_item_text.clear();
            }
            Event::End(TagEnd::Item) => {
                // Try to parse the collected list item text as a task reference
                if let Some(task) = parse_task_list_text(&list_item_text, &task_list_re) {
                    tasks.push(task);
                }
                list_item_text.clear();
            }
            Event::End(TagEnd::List(_)) => {
                *in_list = false;
                list_item_text.clear();
            }

            // All other text events: extract plan name and metadata, and collect list text
            Event::Text(text) => {
                let text = text.to_string();

                // If inside a list, collect text for task list parsing
                if *in_list {
                    list_item_text.push_str(&text);
                }

                // Extract plan name from "# Plan: <name>" heading text
                if name.is_empty() && text.starts_with("Plan: ") {
                    name = text["Plan: ".len()..].to_string();
                }

                // Parse metadata like **ID:** PLAN-001
                if let Some(value) = extract_metadata_value(&text, "ID:") {
                    id = value;
                } else if let Some(value) = extract_metadata_value(&text, "Status:") {
                    status = parse_plan_status(&value);
                } else if let Some(value) = extract_metadata_value(&text, "Created:") {
                    created = value;
                } else if let Some(value) = extract_metadata_value(&text, "Branch:") {
                    branch = value;
                }
            }

            Event::Html(html) => {
                let html = html.to_string();
                // Also try to parse metadata from HTML (sometimes bold is rendered as HTML)
                if let Some(value) = extract_metadata_value(&html, "ID:") {
                    id = value;
                } else if let Some(value) = extract_metadata_value(&html, "Status:") {
                    status = parse_plan_status(&value);
                } else if let Some(value) = extract_metadata_value(&html, "Created:") {
                    created = value;
                } else if let Some(value) = extract_metadata_value(&html, "Branch:") {
                    branch = value;
                }
            }

            _ => {}
        }
    }

    // Flush last section
    flush_section(&mut current_section, &section_buffer, &mut goal, &mut scope, &mut background);

    Ok(Plan {
        id,
        name,
        status,
        created,
        branch,
        goal,
        scope,
        background,
        tasks,
    })
}

// ── Task parsing ────────────────────────────────────────────────────────────

/// Parse a task markdown file into a [`Task`] struct.
///
/// Expected structure:
/// - `# TASK-<NNN>: <name>` heading for task ID and name
/// - `**Parent plan:**`, `**Dependencies:**`, `**Status:**` metadata lines
/// - `## Description`, `## Acceptance criteria`, `## Files to modify`,
///   `## Background`, `## Notes` sections
pub fn parse_task_markdown(path: &Path) -> Result<Task> {
    let content = read_file(path)?;
    let parser = Parser::new(&content);

    let mut id = String::new();
    let mut name = String::new();
    let mut parent_plan = String::new();
    let mut dependencies: Vec<String> = Vec::new();
    let mut description = String::new();
    let mut acceptance_criteria: Vec<String> = Vec::new();
    let mut files_to_modify: Vec<String> = Vec::new();
    let mut background = String::new();
    let mut notes = String::new();

    let mut current_section = Section::None;
    let mut section_buffer = String::new();
    let mut bullet_buffer = String::new();

    for event in parser {
        match event {
            Event::Text(text) => {
                let text = text.to_string();

                // Parse "# TASK-001: name" heading text
                if id.is_empty() && text.starts_with("TASK-") {
                    let colon_pos = text.find(':');
                    if let Some(pos) = colon_pos {
                        id = text[..pos].trim().to_string();
                        name = text[pos + 1..].trim().to_string();
                    } else {
                        id = text.trim().to_string();
                    }
                }

                // Parse metadata
                if let Some(value) = extract_metadata_value(&text, "Parent plan:") {
                    parent_plan = value;
                } else if let Some(value) = extract_metadata_value(&text, "Dependencies:") {
                    dependencies = parse_comma_list(&value);
                }

                // Section content
                if current_section == Section::Description {
                    section_buffer.push_str(&text);
                } else if current_section == Section::AcceptanceCriteria {
                    bullet_buffer.push_str(&text);
                } else if current_section == Section::FilesToModify {
                    bullet_buffer.push_str(&text);
                } else if current_section == Section::Background {
                    section_buffer.push_str(&text);
                } else if current_section == Section::Notes {
                    section_buffer.push_str(&text);
                }
            }

            Event::Start(Tag::Heading {
                level: HeadingLevel::H2,
                id: heading_id,
                ..
            }) => {
                // Flush previous section
                flush_task_section(
                    &mut current_section,
                    &section_buffer,
                    &mut description,
                    &mut background,
                    &mut notes,
                );
                section_buffer.clear();
                bullet_buffer.clear();

                // Determine which section this heading starts
                let id_str = heading_id
                    .as_ref()
                    .map(|s| s.to_string().to_lowercase())
                    .unwrap_or_default();
                if id_str == "description" {
                    current_section = Section::Description;
                } else if id_str == "acceptance-criteria" || id_str == "acceptance criteria" {
                    current_section = Section::AcceptanceCriteria;
                } else if id_str == "files-to-modify" || id_str == "files to modify" {
                    current_section = Section::FilesToModify;
                } else if id_str == "background" {
                    current_section = Section::Background;
                } else if id_str == "notes" {
                    current_section = Section::Notes;
                }
            }

            Event::End(TagEnd::Heading(HeadingLevel::H2)) => {}

            Event::Start(Tag::List(None)) | Event::Start(Tag::List(Some(_))) => {
                bullet_buffer.clear();
            }
            Event::End(TagEnd::List(_)) => {
                // End of bullet list - process collected bullets
                if current_section == Section::AcceptanceCriteria && !bullet_buffer.is_empty() {
                    acceptance_criteria.push(bullet_buffer.trim().to_string());
                    bullet_buffer.clear();
                } else if current_section == Section::FilesToModify && !bullet_buffer.is_empty() {
                    files_to_modify.push(bullet_buffer.trim().to_string());
                    bullet_buffer.clear();
                }
            }

            _ => {}
        }
    }

    // Flush last section
    flush_task_section(
        &mut current_section,
        &section_buffer,
        &mut description,
        &mut background,
        &mut notes,
    );

    Ok(Task {
        id,
        name,
        parent_plan,
        dependencies,
        description,
        acceptance_criteria,
        files_to_modify,
        background,
        notes,
    })
}

// ── Markdown generation ─────────────────────────────────────────────────────

/// Render a [`Plan`] struct back into markdown format.
pub fn render_plan_markdown(plan: &Plan) -> String {
    let mut out = String::new();

    // Title
    out.push_str(&format!("# Plan: {}\n", plan.name));

    // Metadata
    out.push_str(&format!("**ID: {}**\n", plan.id));
    out.push_str(&format!(
        "**Status: {}**\n",
        plan_status_display(&plan.status)
    ));
    out.push_str(&format!("**Created: {}**\n", plan.created));
    out.push_str(&format!("**Branch: {}**\n", plan.branch));

    out.push('\n');

    // Sections
    out.push_str("## Goal\n\n");
    out.push_str(&plan.goal);
    out.push_str("\n\n");

    out.push_str("## Scope\n\n");
    out.push_str(&plan.scope);
    out.push_str("\n\n");

    out.push_str("## Background\n\n");
    out.push_str(&plan.background);
    out.push_str("\n\n");

    // Task list
    out.push_str("## Tasks\n\n");
    for task in &plan.tasks {
        let checkbox = if task.completed { "[x]" } else { "[ ]" };
        out.push_str(&format!("- {} [{}] {}\n", checkbox, task.id, task.name));
    }

    out
}

/// Render a [`Task`] struct back into markdown format.
pub fn render_task_markdown(task: &Task) -> String {
    let mut out = String::new();

    // Title
    out.push_str(&format!("# {}: {}\n", task.id, task.name));

    // Metadata
    out.push_str(&format!("**Parent plan: {}**\n", task.parent_plan));
    if !task.dependencies.is_empty() {
        out.push_str(&format!(
            "**Dependencies: {}**\n",
            task.dependencies.join(", ")
        ));
    }

    out.push('\n');

    // Description
    out.push_str("## Description\n\n");
    out.push_str(&task.description);
    out.push_str("\n\n");

    // Acceptance criteria
    if !task.acceptance_criteria.is_empty() {
        out.push_str("## Acceptance criteria\n\n");
        for criterion in &task.acceptance_criteria {
            out.push_str(&format!("- {}\n", criterion));
        }
        out.push('\n');
    }

    // Files to modify
    if !task.files_to_modify.is_empty() {
        out.push_str("## Files to modify\n\n");
        for file in &task.files_to_modify {
            out.push_str(&format!("- {}\n", file));
        }
        out.push('\n');
    }

    // Background
    if !task.background.is_empty() {
        out.push_str("## Background\n\n");
        out.push_str(&task.background);
        out.push_str("\n\n");
    }

    // Notes
    if !task.notes.is_empty() {
        out.push_str("## Notes\n\n");
        out.push_str(&task.notes);
        out.push_str("\n\n");
    }

    out
}

// ── Helper functions ────────────────────────────────────────────────────────

/// Extract a metadata value from a text chunk that contains a label.
///
/// E.g., given `"**ID:** PLAN-001"` and label `"ID:"`, returns `"PLAN-001"`.
fn extract_metadata_value(text: &str, label: &str) -> Option<String> {
    // Strip HTML/bold tags for matching
    let clean = text
        .replace("<strong>", "")
        .replace("</strong>", "")
        .replace("**", "");
    if let Some(pos) = clean.find(label) {
        let value = clean[pos + label.len()..].trim().to_string();
        if !value.is_empty() {
            return Some(value);
        }
    }
    None
}

/// Convert a PlanStatus to a display string (kebab-case, no quotes).
fn plan_status_display(status: &PlanStatus) -> &'static str {
    match status {
        PlanStatus::Backlog => "backlog",
        PlanStatus::Queued => "queued",
        PlanStatus::Planning => "planning",
        PlanStatus::Reviewing => "reviewing",
        PlanStatus::PlanComplete => "plan-complete",
    }
}

/// Parse a task list item from text like `- [ ] [TASK-001] description`.
fn parse_task_list_text(text: &str, re: &Regex) -> Option<TaskReference> {
    let text = text.trim();
    if let Some(caps) = re.captures(text) {
        let checkbox = &caps[1];
        let completed = checkbox == "x" || checkbox == "X";
        let task_id = &caps[2];
        let description = &caps[3];
        if task_id.starts_with("TASK-") {
            return Some(TaskReference {
                id: task_id.to_string(),
                name: description.trim().to_string(),
                completed,
            });
        }
    }
    None
}

/// Parse a comma-separated list into individual trimmed strings.
fn parse_comma_list(text: &str) -> Vec<String> {
    text.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Parse a plan status string into a [`PlanStatus`] enum.
fn parse_plan_status(text: &str) -> PlanStatus {
    match text.trim().to_lowercase().as_str() {
        "backlog" => PlanStatus::Backlog,
        "queued" => PlanStatus::Queued,
        "planning" => PlanStatus::Planning,
        "reviewing" => PlanStatus::Reviewing,
        "plan-complete" => PlanStatus::PlanComplete,
        _ => PlanStatus::Backlog,
    }
}

/// Flush accumulated section content into the appropriate field.
fn flush_section(
    section: &mut Section,
    buffer: &str,
    goal: &mut String,
    scope: &mut String,
    background: &mut String,
) {
    let trimmed = buffer.trim().to_string();
    match *section {
        Section::Goal => *goal = trimmed,
        Section::Scope => *scope = trimmed,
        Section::Background => *background = trimmed,
        _ => {}
    }
    *section = Section::None;
}

/// Flush accumulated section content for task parsing.
fn flush_task_section(
    section: &mut Section,
    buffer: &str,
    description: &mut String,
    background: &mut String,
    notes: &mut String,
) {
    let trimmed = buffer.trim().to_string();
    match *section {
        Section::Description => *description = trimmed,
        Section::Background => *background = trimmed,
        Section::Notes => *notes = trimmed,
        _ => {}
    }
    *section = Section::None;
}

/// Track which markdown section we are currently parsing.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Section {
    None,
    Goal,
    Scope,
    Background,
    Description,
    AcceptanceCriteria,
    FilesToModify,
    Notes,
}
