# Agent Pools, Models, and Multi-User Boundaries

> **Scope**: This chapter explains where models are selected, the role×capability scheduling foundation agent-chat already has, and which capabilities are not shared across instances when several people share one Matrix room. Prerequisites: Chapters 5.2 and 5.5.

## How Models Are Selected Today

A Claude/Codex model is **runtime process configuration**. It can currently be specified at startup:

```bash
bin/agentchat up wf_implementer /path/to/worktree claude --model <model>
bin/agentchat up wf_final_reviewer /path/to/review-worktree codex --model <model>
```

The dashboard's runtime profile can also save startup configuration; a running tmux session will not switch models because of one natural-language sentence in Robrix2 — after a change, a managed restart is required. Do not let a coordinator claim it has "switched to some model" without verifying the actual runtime/profile.

At any given moment, one Agent has exactly one working directory and one model process. When you need implementation, review, and final review in parallel, the recommendation is to maintain multiple Agents, each bound to its own project path or Git worktree, rather than repeatedly restarting the same Agent.

## The Agent Pool That Already Exists

The agent-chat backend already has a role×capability pool and a `/api/dispatch` foundation:

| role | Default capability |
|------|--------------------|
| architect / review | `strong` |
| coding / testing / integration | `medium` |
| documentation | `lightweight` |

The scheduler prefers "the cheapest idle Agent that meets the requirements"; when there is no candidate, it returns a provision plan or enters a queue. Dispatch uses owner-bound, renewable leases; when owner/lease/agent do not match, renew/release is not possible. The queue and in-flight leases are currently still in-process state — after a backend restart this is not a fully durable scheduler.

This pool is the current backend's resource pool, not a public pool of all members of a Matrix room. A teammate instance's `UNREGISTERED` Agents, even when they appear in the same room, cannot be assigned local paths, tokens, or task leases by this backend.

## The Target Shape of Task-Level Model Scheduling

The target interaction discussed in this session:

```text
"medium for implementation, strong Claude for review, strong Codex for final review"
                         ↓
Robrix2 shows a structured dispatch preview (Agent / runtime / model / project / worktree)
                         ↓
User confirms
                         ↓
agent-chat selects from its own model pool and establishes a dispatch lease
```

This is a reasonable design, but the **Robrix2 natural language → structured preview → user confirmation → `/api/dispatch`** pipeline is not yet wired up, and should be marked as planned. The currently reproducible approach is for the owner to pre-create multiple profiles/Agents, and for the coordinator to select explicit Agent names within the workflow convention.

## The Multi-User Security Model

When several people invite their own Agents into the same public project room:

- Each Agent's owner still comes from the full MXID of "whoever invited this Agent";
- Each backend manages only its own Agents, project paths, runtime profiles, API tokens, and dispatch leases;
- The shared room only shares explicitly published messages and sanitized approval status;
- Detailed approvals go into each `(agent, owner)` pair's E2EE approval room;
- Robrix2 may display cross-instance members, but must not grant, transfer, or infer permissions on that basis;
- Auto-inviting your own Agents can serve as a UI convenience, but the currently logged-in owner must explicitly confirm the target Agent and room, and the server must establish provenance from the real invite event — a coordinator or an ordinary member must not silently do it on the owner's behalf.

So "inviting an Agent into the group" is both a membership operation and a security operation. If a bulk-invite UI is ever implemented, it should first display the full MXID, the backend each Agent belongs to, the target project room, and the owner relationship about to be established, and only then let the user confirm.
