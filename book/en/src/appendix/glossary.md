# Appendix: Glossary and Capability Status

> **Scope**: This appendix standardizes the easily conflated terms across Matrix, agent-chat, and workflow, and provides a quick reference of current capability maturity.

## Glossary

| Term | Meaning |
|------|------|
| MXID | Full Matrix user ID, e.g. `@alex:matrix.example.com`; authorization checks never use display names |
| bridge bot | The agent-chat instance's Matrix companion account, responsible for commands, bridging, and encrypted approval sends |
| puppet | Each agent's `@ac_<name>` Matrix account |
| trusted inviter | A full MXID the bridge allows to invite it into rooms |
| operator | An MXID allowed to run admin commands such as `!bindroom`; this does not grant approval rights over any agent |
| owner | The real `event.sender` who invited a specific agent into a specific project room; the relationship is `(room, agent)→MXID` |
| group | An agent-chat backend membership/message grouping |
| project room | The Matrix board room bound to a group; agent outbound currently requires it to be unencrypted |
| ordinary DM | A one-on-one message room between a human and an agent, created on demand |
| approval room | An E2EE room created/reused per `(agent, owner)`, carrying only structured approvals |
| request / verdict | The approval request event and the single-use decision event |
| digest | A SHA-256 binding over the server-side canonicalized request fields |
| TTL | The approval validity window, 5 minutes by default |
| Olm / Megolm / OTK | Matrix's device-session, room-encryption, and one-time-key mechanisms |
| managed project | A copy/symlink project exposed to an agent via `agentchat project add` |
| worktree | A native Git independent checkout; not created automatically by `project add` |
| workflow binding | The Project Board's read-only group→project/workflow configuration; currently not a role-authorization API |
| capability | The `strong` / `medium` / `lightweight` dispatch tiers |
| dispatch lease | An owner-bound, renewable claim the backend establishes for one pool dispatch |

## Current Capability Status

| Capability | Status |
|------|------|
| Matrix group mention routing | Current implementation; shared rooms default to explicit mention |
| owner approval, TTL, single-use, server validation | Protocol-enforced; preconditioned on a managed runtime and a unique owner |
| E2EE approval room | Current implementation; may be affected by device/key delivery delays, denies on failure |
| Thread reply continuity in unencrypted project rooms | Current implementation; requires a trusted `reply_to` |
| Automatic proactive workflow reporting | Workflow convention, not a transport guarantee |
| Four-role issue-workflow | Experimental shared skill; the name determines the role |
| Persistent role binding / workflow engine | Planned |
| Project Board | `feat/project-board` preview; read-only, does not auto-load demo state |
| GitHub + AtomGit artifact observation | Implemented in the Project Board preview |
| role×capability pool and backend dispatch | Current backend foundation; the queue does not survive restarts |
| Natural-language per-task model selection with confirmed dispatch in Robrix | Planned |
| Agent Thread outbound in encrypted project rooms | Not yet supported |

## Implementation Evidence Index

When reading the code or doing a security review, start from these authoritative artifacts:

- agent-chat `bridge-matrix.js`: Matrix invite provenance, mention routing, Thread relations, the approval room, and E2EE;
- agent-chat `lib/approval-store.js`: owner selection, digest, TTL, single consume, and verdict validation;
- agent-chat `lib/agent-launch-policy.js` and the Codex permission hook: managed runtime policy;
- agent-chat `specs/task-matrix-thread-continuity.spec.md`: Thread normal path, degradation, cross-room, and restart windows;
- agent-chat `specs/task-project-board.spec.md`: Project Board v1 privacy, providers, and out-of-scope;
- agent-chat `lib/matrix-agent.js`: the role×capability pool;
- Robrix2 `src/sliding_sync.rs` and `src/home/room_screen.rs`: verdict sending, device refresh, and the approval-card UI;
- Robrix2 `roadmap/agentchat-demo/issue-workflow/SKILL.md`: the current experimental workflow's name-based role branching and reporting conventions.

When the code and this book conflict, the pinned-commit implementation and specs win, and the discrepancy is fixed as a documentation bug; do not let screenshots override code facts.
