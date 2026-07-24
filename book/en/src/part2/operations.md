# Operations Acceptance and Troubleshooting

> **Scope**: This chapter traces Matrix ingress through tmux and approval verdicts. Use it for missing replies, misplaced thread responses, absent cards, and approvals that still expire.

## Release Acceptance Checklist

Record facts rather than guessing from display names. Save this table with each test run so changes in homeserver, bridge device, or runtime version can be reconciled:

| Area | Record |
|------|--------|
| Versions | Robrix2 commit, agent-chat commit, homeserver version/date |
| Accounts | full MXIDs for human, bridge, and every `@ac_*` |
| Bindings | room→group; every `(room, agent)→owner`; optional group→project |
| Runtime | name, Claude/Codex, model, managed marker, project path/mode |
| Rooms | unencrypted project room; E2EE approval room per `(agent, owner)` |
| Workflow | skill version, role names, worktree/commit SHA |

Minimum end-to-end acceptance:

1. unmentioned top-level text wakes no Agent; explicit @ wakes only the target;
2. a direct reply to a Thread mention stays in that Thread;
3. a second-hop reply still works after bridge restart;
4. Claude protected operation produces a redacted public wait plus a private pending card;
5. `Approve once` executes once and replay is denied;
6. Codex completes `TRUST`; in-sandbox work is quiet and an escape requests approval;
7. wrong owner/room, expiry, and empty owner all fail closed;
8. `!ctl` / `!agentctl` cannot bypass approval in project/approval rooms;
9. dashboard Agent/Tasks/Pool agrees with Git/worktree reality;
10. final delivery is checked against commit, real test output, and PR/MR state.

## Agent Does Not Reply

Trace one layer at a time:

```text
Matrix event
  → explicit mention / trusted room
  → bridge ingestion
  → backend message
  → push relay
  → managed tmux
  → Agent check_inbox
  → backend reply
  → Matrix puppet send
```

Verify joined membership, the explicit target, invite polling (often about 60 seconds), `agentchat ls`, heartbeat, relay health, and whether the runtime was manually reopened. Recover with managed `down/up`, not a raw CLI inside tmux.

## Reply Falls Out of the Thread

Inspect inbound `matrixContext`, target `matrixDelivery.primaryEventId`, and the delivery journal:

- target in another room: reject as a security error;
- legacy/missing delivery: top-level fallback plus warning;
- proactive `post(group=...)` without `reply_to`: workflow context is missing, not a Robrix loss.

Encrypted project rooms are not supported by the current Agent group outbound path.

## Approval Card Is Missing

1. Decide whether the runtime TUI is an agent-chat wait or a local prompt by checking for a backend pending record;
2. verify managed Claude auto+Ask or trusted Codex hook;
3. verify a unique owner binding;
4. verify the owner joined the approval room;
5. inspect bridge request send and queued UTD;
6. verify Robrix synced/decrypted the custom event.

Do not approve with text or choose a local TUI Yes merely to make the Matrix test pass.

## Approved but Runtime Reports Expired/Denied

Reconcile request ID, expiry, `event.sender`, approval room, agent/project/project room, and digest. A retry creates a new request; an older visible card cannot approve it. Read backend audit rejection codes and bridge verdict logs. E2EE delays may outlive the default five-minute TTL; key refresh/rotation reduces risk but cannot guarantee delivery.

Never automatically retry an external write until the prior request is terminal and side effects are checked.

## Which Source Answers Which Question?

| Question | Primary evidence |
|----------|------------------|
| Is the Agent online? | managed process/tmux + backend heartbeat |
| Did backend receive the message? | backend message/inbox |
| Where is the workflow? | workflow state + durable task if present + Thread; no single source today |
| Who may approve? | bridge owner binding + original Matrix invite sender |
| What was the approval result? | backend approval store/audit |
| Is code complete? | Git commit/worktree + actual test results |
| Was it published? | GitHub/AtomGit issue or change request |

Project Board is an aggregate view, not a replacement for these records.
