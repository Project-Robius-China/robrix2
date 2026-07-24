# Agent Pools, Models, and Multi-User Boundaries

> **Scope**: This chapter explains where models are selected, agent-chat's role×capability scheduling foundation, and what does not cross instance boundaries when several users share a Matrix room.

## How Models Are Selected Today

Claude/Codex models are **runtime process configuration**:

```bash
bin/agentchat up wf_implementer /path/to/worktree claude --model <model>
bin/agentchat up wf_final_reviewer /path/to/review-worktree codex --model <model>
```

Dashboard runtime profiles can persist launch configuration. An already-running tmux does not change models because someone states a preference in Robrix2; apply a model change through a managed restart. A coordinator should not claim a model switch without checking the actual runtime/profile.

For parallel implementation and reviews, maintain several Agents with separate managed projects or Git worktrees rather than repeatedly restarting one Agent.

## Existing Agent Pool

The backend already has a role×capability pool and `/api/dispatch` foundation:

| role | default capability |
|------|--------------------|
| architect / review | `strong` |
| coding / testing / integration | `medium` |
| documentation | `lightweight` |

The scheduler picks the cheapest sufficient idle Agent, otherwise returning a provision plan or queueing work. Dispatch uses an owner-bound renewable lease; mismatched owner/lease/agent tuples cannot renew or release it. Queues and in-flight leases remain process-local, so backend restart is not yet a fully durable scheduler.

This is one backend's pool, not a shared pool of every Matrix room member. A teammate's **UNREGISTERED** Agent cannot be assigned your paths, tokens, or leases.

## Target Per-Task Model Flow

The target discussed in this project is:

```text
"medium implementation, strong Claude review, strong Codex final review"
                              ↓
Robrix2 shows a structured plan (Agent/runtime/model/project/worktree)
                              ↓
human confirms
                              ↓
agent-chat selects from its pool and creates dispatch leases
```

The Robrix2 natural-language → structured preview → confirmation → `/api/dispatch` path is **planned, not shipped**. Today, owners pre-create runtime profiles/Agents and the workflow selects explicit Agent names.

## Multi-User Security Model

When several users invite their Agents to one public project room:

- each Agent's owner is still the full MXID that invited that Agent;
- each backend manages only its Agents, paths, profiles, tokens, and leases;
- the public room shares only explicitly posted messages and redacted approval status;
- details go to each `(agent, owner)` approval room;
- Robrix2 may display cross-instance members but cannot infer or transfer authority;
- a future batch-invite UI may assist, but must show exact MXIDs, Agent/backend, room, and the owner relationship before the logged-in owner confirms.

Inviting an Agent is both membership management and a security operation.
