# Appendix: Glossary and Capability Status

> **Scope**: This appendix standardizes Matrix, agent-chat, and workflow terms that are easy to conflate, followed by a current maturity table.

## Glossary

| Term | Meaning |
|------|---------|
| MXID | full Matrix ID such as `@alex:matrix.example.com`; authorization never uses display name |
| bridge bot | companion Matrix account for commands, bridge transport, and encrypted approval delivery |
| puppet | an Agent's `@ac_<name>` Matrix account |
| trusted inviter | full MXID allowed to invite the bridge |
| operator | MXID allowed to run management commands; not automatically an Agent owner |
| owner | real invite `event.sender` for a specific `(room, agent)` |
| group | agent-chat backend member/message grouping |
| project room | Matrix room bound to a group; currently unencrypted for Agent outbound |
| ordinary DM | on-demand one-to-one human/Agent room |
| approval room | E2EE room created/reused for `(agent, owner)` |
| request/verdict | structured approval request and one-shot decision events |
| digest | SHA-256 binding over canonical server request fields |
| TTL | approval validity window, five minutes by default |
| Olm/Megolm/OTK | Matrix device sessions, room encryption, and one-time keys |
| managed project | copy/symlink path exposed by `agentchat project add` |
| worktree | independent Git checkout; not created by project binding |
| workflow binding | read-only group→project/workflow Board configuration, not a role-authorization API |
| capability | `strong`, `medium`, or `lightweight` scheduling tier |
| dispatch lease | owner-bound renewable reservation for one pool dispatch |

## Current Capability Status

| Capability | Status |
|------------|--------|
| Matrix group mention routing | implemented; explicit mentions by default |
| owner approval, TTL, one-shot, server validation | protocol-enforced with managed runtime and unique owner |
| E2EE approval room | implemented; key delivery can delay, failure denies |
| unencrypted project-room thread continuity | implemented with trusted `reply_to` |
| automatic proactive workflow reports | workflow convention |
| four-role issue-workflow | experimental shared skill, name-based roles |
| persistent role binding/workflow engine | planned |
| Project Board | `feat/project-board` preview, read-only |
| GitHub + AtomGit artifact observation | implemented in the Board preview |
| role×capability backend pool | foundation implemented; queue not restart-durable |
| Robrix natural-language per-task model selection | planned |
| Agent thread outbound in encrypted project rooms | unsupported |

## Implementation Evidence Index

Start code or security review from these authoritative artifacts:

- agent-chat `bridge-matrix.js`: invite provenance, mention routing, thread relations, approval rooms, and E2EE;
- agent-chat `lib/approval-store.js`: owner selection, digest, TTL, single consume, and verdict validation;
- agent-chat `lib/agent-launch-policy.js` and the Codex permission hook: managed runtime policy;
- agent-chat `specs/task-matrix-thread-continuity.spec.md`: happy path, fallback, cross-room rejection, and restart window;
- agent-chat `specs/task-project-board.spec.md`: Board v1 privacy, providers, and out-of-scope;
- agent-chat `lib/matrix-agent.js`: role×capability pool;
- Robrix2 `src/sliding_sync.rs` and `src/home/room_screen.rs`: verdict sending, device refresh, and approval-card UI;
- Robrix2 `roadmap/agentchat-demo/issue-workflow/SKILL.md`: current experimental name-based workflow.

If code and book conflict, the pinned implementation and spec win. Treat the difference as a documentation bug; screenshots do not override code facts.
