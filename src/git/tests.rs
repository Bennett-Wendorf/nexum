#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::git::branch::{
        branch_exists, checkout_branch, create_branch, create_plan_branch, create_task_branch,
        current_branch, delete_branch, is_task_branch, list_local_branches, setup_task_workspace,
        task_branch_name, teardown_task_workspace,
    };
    use crate::git::errors::GitError;
    use crate::git::merge::{
        abort_merge, cleanup_merged_task, determine_merge_order, execute_merge_sequence,
        has_merge_conflicts, is_merging, list_conflicted_files, merge_branch, merge_task_branch,
        next_mergeable_tasks, MergePlan,
    };
    use crate::git::subprocess::{git, GitCommand, GitOutput};
    use crate::git::worktree::{
        list_worktrees, remove_worktree, remove_worktree_force, spawn_worktree, worktree_exists,
        worktree_path,
    };

    // --- Test Infrastructure ---

    async fn create_test_repo() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::with_prefix("nexum-test-").unwrap();
        let path = dir.path().to_path_buf();
        let _ = git(&path, &["init"]).await.unwrap();
        let _ = git(&path, &["config", "user.email", "test@nexum.local"]).await.unwrap();
        let _ = git(&path, &["config", "user.name", "Test User"]).await.unwrap();
        std::fs::write(path.join("README.md"), "# Test Repo").unwrap();
        let _ = git(&path, &["add", "."]).await.unwrap();
        let _ = git(&path, &["commit", "-m", "Initial commit"]).await.unwrap();
        (dir, path)
    }

    // --- Subprocess Tests ---

    #[tokio::test]
    async fn test_git_command_success() {
        let (_dir, repo) = create_test_repo().await;
        let output = GitCommand::new(&repo).args(&["status"]).execute().await.unwrap();
        assert!(output.success());
        assert_eq!(output.exit_code, 0);
    }

    #[tokio::test]
    async fn test_git_command_failure() {
        let (_dir, repo) = create_test_repo().await;
        let result = GitCommand::new(&repo)
            .args(&["not-a-command"])
            .execute()
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_git_convenience_function() {
        let (_dir, repo) = create_test_repo().await;
        let output = git(&repo, &["status"]).await.unwrap();
        assert!(output.success());
    }

    // --- Branch Tests ---

    #[tokio::test]
    async fn test_create_branch() {
        let (_dir, repo) = create_test_repo().await;
        create_branch(&repo, "feature/test", "main").await.unwrap();
        assert!(branch_exists(&repo, "feature/test").await.unwrap());
    }

    #[tokio::test]
    async fn test_create_task_branch() {
        let (_dir, repo) = create_test_repo().await;
        let name = create_task_branch(&repo, "TASK-001", "main").await.unwrap();
        assert_eq!(name, "task/TASK-001");
        assert!(branch_exists(&repo, &name).await.unwrap());
    }

    #[tokio::test]
    async fn test_delete_branch() {
        let (_dir, repo) = create_test_repo().await;
        create_branch(&repo, "to-delete", "main").await.unwrap();
        assert!(branch_exists(&repo, "to-delete").await.unwrap());
        delete_branch(&repo, "to-delete").await.unwrap();
        assert!(!branch_exists(&repo, "to-delete").await.unwrap());
    }

    #[tokio::test]
    async fn test_branch_exists() {
        let (_dir, repo) = create_test_repo().await;
        assert!(branch_exists(&repo, "main").await.unwrap());
        assert!(!branch_exists(&repo, "nonexistent").await.unwrap());
    }

    #[tokio::test]
    async fn test_list_local_branches() {
        let (_dir, repo) = create_test_repo().await;
        create_branch(&repo, "feature/a", "main").await.unwrap();
        create_branch(&repo, "feature/b", "main").await.unwrap();
        let branches = list_local_branches(&repo).await.unwrap();
        assert!(branches.contains(&"main".to_string()));
        assert!(branches.contains(&"feature/a".to_string()));
        assert!(branches.contains(&"feature/b".to_string()));
    }

    #[tokio::test]
    async fn test_current_branch() {
        let (_dir, repo) = create_test_repo().await;
        let branch = current_branch(&repo).await.unwrap();
        assert_eq!(branch, "main");
    }

    #[test]
    fn test_task_branch_name() {
        assert_eq!(task_branch_name("TASK-001"), "task/TASK-001");
        assert_eq!(task_branch_name("TASK-42"), "task/TASK-42");
    }

    #[test]
    fn test_is_task_branch() {
        assert!(is_task_branch("task/TASK-001"));
        assert!(!is_task_branch("feature/auth"));
        assert!(!is_task_branch("main"));
    }

    // --- Worktree Tests ---

    #[test]
    fn test_worktree_path() {
        let path = worktree_path(&PathBuf::from("/repo"), "TASK-001");
        assert_eq!(path.to_string_lossy(), "/repo/.worktrees/TASK-001");
    }

    #[tokio::test]
    async fn test_spawn_worktree() {
        let (_dir, repo) = create_test_repo().await;
        create_task_branch(&repo, "TASK-001", "main").await.unwrap();
        let wt_path = spawn_worktree(&repo, "TASK-001", "task/TASK-001").await.unwrap();
        assert!(wt_path.exists());
        assert!(worktree_exists(&repo, "TASK-001").await.unwrap());
    }

    #[tokio::test]
    async fn test_remove_worktree() {
        let (_dir, repo) = create_test_repo().await;
        create_task_branch(&repo, "TASK-002", "main").await.unwrap();
        spawn_worktree(&repo, "TASK-002", "task/TASK-002").await.unwrap();
        remove_worktree(&repo, "TASK-002").await.unwrap();
        assert!(!worktree_exists(&repo, "TASK-002").await.unwrap());
    }

    #[tokio::test]
    async fn test_worktree_exists() {
        let (_dir, repo) = create_test_repo().await;
        assert!(!worktree_exists(&repo, "NONEXISTENT").await.unwrap());
    }

    // --- Merge Tests ---

    #[tokio::test]
    async fn test_merge_branch_success() {
        let (_dir, repo) = create_test_repo().await;
        create_branch(&repo, "feature/merge-test", "main").await.unwrap();
        let output = merge_branch(&repo, "feature/merge-test", "main").await.unwrap();
        assert!(output.success());
    }

    #[tokio::test]
    async fn test_is_merging() {
        let (_dir, repo) = create_test_repo().await;
        assert!(!is_merging(&repo).await.unwrap());
    }

    #[tokio::test]
    async fn test_abort_merge_noop() {
        let (_dir, repo) = create_test_repo().await;
        // No merge in progress, should be no-op
        abort_merge(&repo).await.unwrap();
    }

    // --- Coordination Tests ---

    #[test]
    fn test_determine_merge_order() {
        let mut deps = std::collections::HashMap::new();
        deps.insert("TASK-002".to_string(), vec!["TASK-001".to_string()]);
        deps.insert("TASK-003".to_string(), vec!["TASK-001".to_string(), "TASK-002".to_string()]);

        let plan = MergePlan {
            repo_root: PathBuf::from("/tmp"),
            plan_branch: "feature/test".to_string(),
            pending_tasks: vec![
                "TASK-001".to_string(),
                "TASK-002".to_string(),
                "TASK-003".to_string(),
            ],
            merged_tasks: vec![],
            dependencies: deps,
        };

        let order = determine_merge_order(&plan).unwrap();
        assert_eq!(order[0], "TASK-001");
        // TASK-002 and TASK-003 must come after TASK-001
        let idx_001 = order.iter().position(|t| t == "TASK-001").unwrap();
        let idx_002 = order.iter().position(|t| t == "TASK-002").unwrap();
        let idx_003 = order.iter().position(|t| t == "TASK-003").unwrap();
        assert!(idx_001 < idx_002);
        assert!(idx_002 < idx_003);
    }

    #[test]
    fn test_next_mergeable_tasks() {
        let mut deps = std::collections::HashMap::new();
        deps.insert("TASK-002".to_string(), vec!["TASK-001".to_string()]);

        let plan = MergePlan {
            repo_root: PathBuf::from("/tmp"),
            plan_branch: "feature/test".to_string(),
            pending_tasks: vec![
                "TASK-001".to_string(),
                "TASK-002".to_string(),
            ],
            merged_tasks: vec![],
            dependencies: deps,
        };

        // Only TASK-001 should be mergeable (TASK-002 depends on it)
        let ready = next_mergeable_tasks(&plan).unwrap();
        assert_eq!(ready, vec!["TASK-001"]);
    }

    #[test]
    fn test_next_mergeable_tasks_blocked() {
        let mut deps = std::collections::HashMap::new();
        deps.insert("TASK-001".to_string(), vec!["TASK-000".to_string()]);

        let plan = MergePlan {
            repo_root: PathBuf::from("/tmp"),
            plan_branch: "feature/test".to_string(),
            pending_tasks: vec!["TASK-001".to_string()],
            merged_tasks: vec![],
            dependencies: deps,
        };

        let ready = next_mergeable_tasks(&plan).unwrap();
        assert!(ready.is_empty());
    }

    // --- Workspace Tests ---

    #[tokio::test]
    async fn test_setup_task_workspace() {
        let (_dir, repo) = create_test_repo().await;
        create_branch(&repo, "feature/workspace", "main").await.unwrap();
        let wt_path = setup_task_workspace(&repo, "TASK-WS1", "feature/workspace").await.unwrap();
        assert!(wt_path.exists());
        assert!(branch_exists(&repo, "task/TASK-WS1").await.unwrap());
    }

    #[tokio::test]
    async fn test_teardown_task_workspace() {
        let (_dir, repo) = create_test_repo().await;
        create_branch(&repo, "feature/teardown", "main").await.unwrap();
        setup_task_workspace(&repo, "TASK-TD1", "feature/teardown").await.unwrap();
        teardown_task_workspace(&repo, "TASK-TD1").await.unwrap();
        assert!(!branch_exists(&repo, "task/TASK-TD1").await.unwrap());
        assert!(!worktree_exists(&repo, "TASK-TD1").await.unwrap());
    }

    #[tokio::test]
    async fn test_setup_then_teardown() {
        let (_dir, repo) = create_test_repo().await;
        create_branch(&repo, "feature/lifecycle", "main").await.unwrap();
        let wt_path = setup_task_workspace(&repo, "TASK-LC1", "feature/lifecycle").await.unwrap();
        assert!(wt_path.exists());
        teardown_task_workspace(&repo, "TASK-LC1").await.unwrap();
        assert!(!worktree_exists(&repo, "TASK-LC1").await.unwrap());
        assert!(!branch_exists(&repo, "task/TASK-LC1").await.unwrap());
    }

    // --- FIX #16: test_delete_branch_nonexistent_noop ---

    #[tokio::test]
    async fn test_delete_branch_nonexistent_noop() {
        let (_dir, repo) = create_test_repo().await;
        // Deleting a non-existent branch should succeed (no-op)
        delete_branch(&repo, "nonexistent-branch").await.unwrap();
    }

    // --- FIX #17: test_merge_conflict_detection ---

    #[tokio::test]
    async fn test_merge_conflict_detection() {
        let (_dir, repo) = create_test_repo().await;

        // Create a feature branch from main
        create_branch(&repo, "feature/conflict", "main").await.unwrap();

        // Write conflicting content to README.md on main
        std::fs::write(repo.join("README.md"), "# Main content\n").unwrap();
        let _ = git(&repo, &["add", "."]).await.unwrap();
        let _ = git(&repo, &["commit", "-m", "Update main"]).await.unwrap();

        // Checkout feature branch and write conflicting content
        checkout_branch(&repo, "feature/conflict").await.unwrap();
        std::fs::write(repo.join("README.md"), "# Feature content\n").unwrap();
        let _ = git(&repo, &["add", "."]).await.unwrap();
        let _ = git(&repo, &["commit", "-m", "Update feature"]).await.unwrap();

        // Attempt merge — should produce a MergeConflict error
        let result = merge_branch(&repo, "feature/conflict", "main").await;
        match result {
            Err(GitError::MergeConflict { conflicts, .. }) => {
                assert!(!conflicts.is_empty(), "Expected at least one conflicted file");
                assert!(conflicts.contains(&"README.md".to_string()));
            }
            Ok(_) => panic!("Expected merge conflict, but merge succeeded"),
            Err(e) => panic!("Expected MergeConflict, got: {:?}", e),
        }

        // Clean up the failed merge state
        abort_merge(&repo).await.unwrap();
    }

    #[tokio::test]
    async fn test_merge_branch_conflict_cleanup() {
        let (_dir, repo) = create_test_repo().await;

        // Create a feature branch from main
        create_branch(&repo, "feature/cleanup-conflict", "main").await.unwrap();

        // Write conflicting content to README.md on main
        std::fs::write(repo.join("README.md"), "# Main content\n").unwrap();
        let _ = git(&repo, &["add", "."]).await.unwrap();
        let _ = git(&repo, &["commit", "-m", "Update main"]).await.unwrap();

        // Checkout feature branch and write conflicting content
        checkout_branch(&repo, "feature/cleanup-conflict").await.unwrap();
        std::fs::write(repo.join("README.md"), "# Feature content\n").unwrap();
        let _ = git(&repo, &["add", "."]).await.unwrap();
        let _ = git(&repo, &["commit", "-m", "Update feature"]).await.unwrap();

        // Record original branch before merge attempt
        let original = current_branch(&repo).await.unwrap();

        // Attempt merge — should produce a MergeConflict error
        let result = merge_branch(&repo, "feature/cleanup-conflict", "main").await;
        match result {
            Err(GitError::MergeConflict { conflicts, .. }) => {
                assert!(!conflicts.is_empty(), "Expected at least one conflicted file");
            }
            Ok(_) => panic!("Expected merge conflict, but merge succeeded"),
            Err(e) => panic!("Expected MergeConflict, got: {:?}", e),
        }

        // Verify cleanup: repo is NOT in a merging state
        assert!(
            !is_merging(&repo).await.unwrap(),
            "Repository should NOT be in a merging state after conflict cleanup"
        );

        // Verify cleanup: current branch is restored to original
        let current = current_branch(&repo).await.unwrap();
        assert_eq!(
            current, original,
            "Current branch '{}' should be restored to original '{}'",
            current, original
        );
    }

    #[tokio::test]
    async fn test_merge_task_branch_conflict_cleanup() {
        let (_dir, repo) = create_test_repo().await;

        // Create a plan branch
        create_plan_branch(&repo, "plan/test", "main", false).await.unwrap();

        // Create a task branch from the plan branch
        create_task_branch(&repo, "TASK-CLR", "plan/test").await.unwrap();

        // Write conflicting content to README.md on plan branch
        checkout_branch(&repo, "plan/test").await.unwrap();
        std::fs::write(repo.join("README.md"), "# Plan content\n").unwrap();
        let _ = git(&repo, &["add", "."]).await.unwrap();
        let _ = git(&repo, &["commit", "-m", "Update plan"]).await.unwrap();

        // Checkout task branch and write conflicting content
        checkout_branch(&repo, "task/TASK-CLR").await.unwrap();
        std::fs::write(repo.join("README.md"), "# Task content\n").unwrap();
        let _ = git(&repo, &["add", "."]).await.unwrap();
        let _ = git(&repo, &["commit", "-m", "Update task"]).await.unwrap();

        // Record original branch before merge attempt
        let original = current_branch(&repo).await.unwrap();

        // Attempt merge — should produce a MergeConflict error
        let result = merge_task_branch(&repo, "TASK-CLR", "plan/test").await;
        match result {
            Err(GitError::MergeConflict { conflicts, .. }) => {
                assert!(!conflicts.is_empty(), "Expected at least one conflicted file");
            }
            Ok(_) => panic!("Expected merge conflict, but merge succeeded"),
            Err(e) => panic!("Expected MergeConflict, got: {:?}", e),
        }

        // Verify cleanup: repo is NOT in a merging state
        assert!(
            !is_merging(&repo).await.unwrap(),
            "Repository should NOT be in a merging state after conflict cleanup"
        );

        // Verify cleanup: current branch is restored to original
        let current = current_branch(&repo).await.unwrap();
        assert_eq!(
            current, original,
            "Current branch '{}' should be restored to original '{}'",
            current, original
        );
    }

    // --- FIX #18: test_circular_dependency ---

    #[test]
    fn test_circular_dependency() {
        let mut deps = std::collections::HashMap::new();
        // TASK-001 depends on TASK-002, and TASK-002 depends on TASK-001
        deps.insert("TASK-001".to_string(), vec!["TASK-002".to_string()]);
        deps.insert("TASK-002".to_string(), vec!["TASK-001".to_string()]);

        let plan = MergePlan {
            repo_root: PathBuf::from("/tmp"),
            plan_branch: "feature/test".to_string(),
            pending_tasks: vec![
                "TASK-001".to_string(),
                "TASK-002".to_string(),
            ],
            merged_tasks: vec![],
            dependencies: deps,
        };

        let result = determine_merge_order(&plan);
        assert!(result.is_err());
        let err = result.unwrap_err();
        let err_msg = format!("{}", err);
        assert!(
            err_msg.contains("circular dependency"),
            "Expected circular dependency error, got: {}",
            err_msg
        );
    }
}
