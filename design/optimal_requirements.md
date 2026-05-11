# My requirements for an optimal agentic coding orchestration tool

## Interoperability
- Must never obscure planning or ticket storage in a format difficult for a typical coding agent to understand
    - Means probably no database structure
- Must work with multiple agents
    - No features specific to one agent or another

## Baby step mode
- This is the key that possibly sets this apart
- Every level of orchestration should be manually manageable
- Users that aren't comfortable with AI can add levels as they learn

## Permissions
- Allow yolo mode, or not
- Expert users probably have things set up enough that they don't need to monitor individual tasks
- New users want to check permissions

## Orchestration levels
Repo - One may want to work on multiple repos at the same time
    Branch - One may want to work on multiple epics at once
        Group of groups (Epic?) - Likely manually decided, breaks down into multiple projects. Think new major feature with parts = UI, backend, security, etc.
            Group of tasks (project?) - AI planning done at this stage
                Individual task - Written by AI, executed by one agent

## Agent roles (need better names)
- Builder
    - Responsible for executing a single task, and only one task at any given time
- Reviewer
    - Reviews anything and everything
    - Invoked after task execution
    - Optionally after planning
    - Invoked after project completion
    - Invoked after epic completion
- Security consultant (optional)
    - Like the reviewer, but specialize to security
- Planner
    - Planning, that's their job and they do it well
- Researcher (optional)
    - Responsible for pre-planning research
- Overlord (maybe not needed? How deterministic should it be?)
    - Main interaction with the user
    - Can handle all the things when it comes to orchestration only
    - Cannot complete tasks, but can create tasks or other agents

## Agent teams
- 1 planner, 1 researcher, 1+ builders, 1+ reviewers
- Configurable (part of resource constraints?)

## Resource constraints
- Should be able to limit parallel agents
    - Local LLMs have major compute restraints
    - Paid services would eat tokens very quickly without throttling
    - Too much going on can be stressful for the user
- Standard for auto-compaction to deal with context limits?

## Interface(s?)
- Modular?
### First iteration
- Create some mockups
### Ideas
- Kanban
- Mind map
- Hierarchical list 

## Operation structure
- Web app
- Can run on localhost like Cline Kanban for local work
- Docker container for remote management (research how OpenClaw does this?)
- TUI? Is there a benefit?

## Work statuses
- Need orchestration levels here. Should be minimum per repo, branch, and epic, but at every level would be ideal
- Can move tasks between certain statuses. For example, if auto-queued accidentally, should be able to move back to backlog.
### Pre-planning
- Backlog
- Queued
- Planning (This is part of where concurrency limits are handled)
- Reviewing (Concurrency limit)
- Plan complete -> post-planning backlog
### Post-planning
- Backlog
- Queued
- Running (This is part of where concurrency limits are handled)
- Reviewing (Concurrency limit)
- Waiting manual review
- Merge-queue
- Abandoned
- Completed (How to clear this out occasionally?)

## Unit of work
### Definition
### Storage
- Markdown as source of truth on work for max interoperability
- Directory structure to handle orchestration levels?
    - Should there also be metadata files for this?
- Additional json data for handling control-plane pieces (agent leases on tasks, queues, statuses, etc.
### Creation
### Dependency chaining
- Need to be able to auto-queue tasks when *ALL* dependencies are completed
### Execution plans?
- For parallelization?
