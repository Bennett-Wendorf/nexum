//! Dependency-based auto-queue logic for the Overlord deterministic core.
//!
//! For each task in `backlog` status, checks whether ALL dependencies are
//! in `completed` status. If so, transitions the task from `backlog` to
//! `queued`. Provides `auto_queue_eligible_tasks()` and `auto_queue_tasks()`
//! methods.

use std::collections::HashMap;
use std::path::Path;

use crate::persistence::{parse_slug, TaskStatusValue};

use super::errors::{OverlordError, Result};

/// Main dependency resolution struct.
pub struct DependencyResolver;

impl DependencyResolver {
    /// Initialize the dependency resolver.
    pub fn new() -> Self {
        Self
    }

    /// Check if all dependencies of a task are completed.
    ///
    /// Reads `status.json` for the task, gets its `dependencies` list,
    /// and checks if ALL dependency tasks have status `completed`.
    pub fn are_all_dependencies_met(
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
        task_id: &str,
        task_name: &str,
    ) -> Result<bool> {
        let status = crate::persistence::read_task_status(repo_root, branch, plan_id, plan_name, task_id, task_name)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        if status.dependencies.is_empty() {
            return Ok(true);
        }

        // For each dependency, check if it's completed
        for dep_id in &status.dependencies {
            // Find the dependency task by scanning the plan's tasks directory
            let dep_status = Self::find_task_status_by_id(
                repo_root,
                branch,
                plan_id,
                plan_name,
                dep_id,
            )?;

            if let Some(dep_status) = dep_status {
                if !matches!(dep_status.status, TaskStatusValue::Completed) {
                    return Ok(false);
                }
            } else {
                // Dependency task not found — treat as unmet
                return Ok(false);
            }
        }

        Ok(true)
    }

    /// Find a task's status by its ID within a plan.
    fn find_task_status_by_id(
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
        task_id: &str,
    ) -> Result<Option<crate::persistence::TaskStatus>> {
        let task_slugs = crate::persistence::list_tasks(repo_root, branch, plan_id, plan_name)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        for slug in &task_slugs {
            // Extract task name from slug: TASK-NNN-task-name
            if let Some(task_name) = slug.strip_prefix(&format!("{}-", task_id)) {
                let status = crate::persistence::read_task_status(
                    repo_root,
                    branch,
                    plan_id,
                    plan_name,
                    task_id,
                    task_name,
                ).map_err(|e| OverlordError::PersistenceError(e))?;
                return Ok(Some(status));
            }
        }

        Ok(None)
    }

    /// Find tasks eligible for auto-queue (backlog with all deps met).
    ///
    /// Returns a list of (task_id, task_name) tuples for eligible tasks.
    pub fn auto_queue_eligible_tasks(
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<Vec<(String, String)>> {
        let task_slugs = crate::persistence::list_tasks(repo_root, branch, plan_id, plan_name)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        let mut eligible = Vec::new();

        for slug in &task_slugs {
            let (Some(task_id), Some(task_name)) = parse_slug(slug) else { continue };

            let status = crate::persistence::read_task_status(
                repo_root,
                branch,
                plan_id,
                plan_name,
                task_id,
                task_name,
            ).map_err(|e| OverlordError::PersistenceError(e))?;

            if matches!(status.status, TaskStatusValue::Backlog)
                && Self::are_all_dependencies_met(
                    repo_root,
                    branch,
                    plan_id,
                    plan_name,
                    task_id,
                    task_name,
                )?
            {
                eligible.push((task_id.to_string(), task_name.to_string()));
            }
        }

        Ok(eligible)
    }

    /// Perform auto-queue transitions for eligible tasks.
    ///
    /// Transitions eligible tasks from `backlog` to `queued` with actor
    /// `"overlord-auto-queue"`.
    pub fn auto_queue_tasks(
        &self,
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<Vec<String>> {
        let eligible = Self::auto_queue_eligible_tasks(repo_root, branch, plan_id, plan_name)?;

        let mut queued = Vec::new();

        for (task_id, task_name) in &eligible {
            // Perform the transition
            crate::persistence::update_task_status(
                repo_root,
                branch,
                plan_id,
                plan_name,
                task_id,
                task_name,
                TaskStatusValue::Queued,
                "overlord-auto-queue",
            ).map_err(|e| OverlordError::PersistenceError(e))?;

            queued.push(task_id.clone());
        }

        Ok(queued)
    }

    /// When a task completes, find tasks that depended on it and check if they're now eligible.
    pub fn resolve_dependent_tasks(
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
        completed_task_id: &str,
    ) -> Result<Vec<String>> {
        let task_slugs = crate::persistence::list_tasks(repo_root, branch, plan_id, plan_name)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        let mut newly_eligible = Vec::new();

        for slug in &task_slugs {
            let (Some(task_id), Some(task_name)) = parse_slug(slug) else { continue };

            if task_id == completed_task_id {
                continue;
            }

            let status = crate::persistence::read_task_status(
                repo_root,
                branch,
                plan_id,
                plan_name,
                task_id,
                task_name,
            ).map_err(|e| OverlordError::PersistenceError(e))?;

            // Check if this task depends on the completed task
            if status.dependencies.contains(&completed_task_id.to_string())
                && matches!(status.status, TaskStatusValue::Backlog)
                && Self::are_all_dependencies_met(
                    repo_root,
                    branch,
                    plan_id,
                    plan_name,
                    task_id,
                    task_name,
                )?
            {
                newly_eligible.push(task_id.to_string());
            }
        }

        Ok(newly_eligible)
    }


    /// Build the full dependency graph for a plan.
    pub fn build_dependency_graph(
        repo_root: &Path,
        branch: &str,
        plan_id: &str,
        plan_name: &str,
    ) -> Result<HashMap<String, Vec<String>>> {
        let task_slugs = crate::persistence::list_tasks(repo_root, branch, plan_id, plan_name)
            .map_err(|e| OverlordError::PersistenceError(e))?;

        let mut graph: HashMap<String, Vec<String>> = HashMap::new();

        for slug in &task_slugs {
            let (Some(task_id), Some(task_name)) = parse_slug(slug) else { continue };

            let status = crate::persistence::read_task_status(
                repo_root,
                branch,
                plan_id,
                plan_name,
                task_id,
                task_name,
            ).map_err(|e| OverlordError::PersistenceError(e))?;

            graph.insert(task_id.to_string(), status.dependencies.clone());
        }

        Ok(graph)
    }

    /// Detect circular dependencies in a dependency graph.
    ///
    /// Uses DFS-based cycle detection. Returns true if a cycle is found.
    pub fn detect_cycles(graph: &HashMap<String, Vec<String>>) -> bool {
        let mut visited = std::collections::HashSet::new();
        let mut recursion_stack = std::collections::HashSet::new();

        fn dfs(
            node: &str,
            graph: &HashMap<String, Vec<String>>,
            visited: &mut std::collections::HashSet<String>,
            recursion_stack: &mut std::collections::HashSet<String>,
        ) -> bool {
            visited.insert(node.to_string());
            recursion_stack.insert(node.to_string());

            if let Some(neighbors) = graph.get(node) {
                for neighbor in neighbors {
                    if !visited.contains(neighbor) {
                        if dfs(neighbor, graph, visited, recursion_stack) {
                            return true;
                        }
                    } else if recursion_stack.contains(neighbor) {
                        return true;
                    }
                }
            }

            recursion_stack.remove(node);
            false
        }

        for node in graph.keys() {
            if !visited.contains(node) {
                if dfs(node, graph, &mut visited, &mut recursion_stack) {
                    return true;
                }
            }
        }

        false
    }
}
