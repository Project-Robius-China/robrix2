# Project Board: A Global View of Tasks and Artifacts

> **Scope**: This chapter covers the Project Board preview: a read-only projection of backend state and project artifacts. Baseline: unmerged `feat/project-board` commit `3102a5f`.

Project Board (`/projects`) aggregates backend durable tasks/graphs/heartbeat with artifact inspection of an explicitly bound project. Navigation stays consistent with Monitor / Tasks / Pool / Alerts / Config, and Agent cards link to Monitor.

It does not read `.agentchat-demo/state.json`. If the demo workflow creates no durable task/graph, `/go` stages do not automatically appear. Before publishing this chapter as shipped documentation, merge the Board branch and provide a supported group→project binding write path.

## Team Overview

![Project Board: project group, stats, and member cards](../images/project-board.png)

At the top of the board you pick a **project group** (here `robrix2-board`, bound to the `robrix2` project and the `issue-workflow@1` workflow). The row of stat tiles below answers the most common questions at a glance:

- **Members / Online**: group membership and how many are online;
- **Working / Blocked / Open Tasks**: how many are working and how many are stuck (`waiting` / `stale` states are called out separately — in the screenshot the coordinator has been waiting 7 hours for wf_codex's final review; this kind of *silent stall* is precisely what the board exists to expose);
- **Worktrees**: managed projects/worktrees and Git dirty state. `0 dirty` only means no uncommitted `git status --porcelain` entries;
- **Specs / Changes**: spec and local/remote issue counts (expanded in the next section).

Member cards show runtime, backend-known tasks, and heartbeat. **UNREGISTERED** room members are not controlled by this backend and gain no scheduling or approval authority. Because v1 tasks lack a project ID, an Agent in several groups may project the same task into more than one project.

## Specs & Changes: The Spec-Driven Artifact Panel

![Specifications and Issues side by side](../images/board-specs-issues.png)

The lower half of the board puts the project's two core artifact types side by side:

**Specifications**: scans `specs/*.spec.md` and counts declared Scenarios and `Test:` mappings. It does not run tests or report coverage/pass. The displayed Agent is the managed worktree that supplied the inspection, not a formal spec owner.

**Changes**, aggregated in provider-neutral form:

![Local issues and GitHub issues aggregated](../images/board-specs-github.png)

- **LOCAL** issue documents plus publish-target metadata; the Board does not publish them;
- **GitHub** issues and pull requests;
- **AtomGit** issues and merge/pull requests via the [AtomGit OpenAPI](https://docs.openatom.tech/en/category/api/); private tokens stay backend-only in `ATOMGIT_TOKEN`;
- unsupported/unavailable providers remain visible as unsynced without exposing tokens, absolute paths, or raw upstream errors.

The common term is **change request** for GitHub PR / AtomGit MR. Remote writes remain Agent tool operations behind owner approval and are out of scope for Board v1.

## Where the Board Sits in HAgency

Project Board is a **read-only projection**, never an authorization source. It includes only the bound group's members/tasks/graphs and project artifacts; DMs, approval details, full bodies, API keys, and absolute paths must stay out of responses.

It answers what the backend recorded, what files declare, and what a provider currently exposes. Delivery completion still requires Thread/task evidence, Git commits, actual test results, and PR/MR state.
