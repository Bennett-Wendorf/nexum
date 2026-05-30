# Agent roles (need better names)

- Builder
    - Responsible for executing a single task, and only one task at any given time
- Reviewer
    - Reviews anything and everything
    - Invoked after task execution
    - Optionally after planning
    - Invoked after plan completion
- Security consultant (optional)
    - Like the reviewer, but specialize to security
- Planner
    - Planning, that's their job and they do it well
    - Performs any necessary research (internet, codebase) before generating the plan
    - Generates `task.md` for each task (including scope, acceptance criteria, files, and background context)
- Overlord (maybe not needed? How deterministic should it be?)
    - Main interaction with the user
    - Can handle all the things when it comes to orchestration only
    - Cannot complete tasks, but can create tasks or other agents
    - **Future: Scratchpad agent** — A separate on-demand agent the user can invoke to fix random things, research questions, or handle ad-hoc tasks outside the main workflow. Likely spawned by or alongside the Overlord.
