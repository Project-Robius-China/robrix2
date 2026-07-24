# Operations Acceptance and Troubleshooting

> **Scope**: This chapter provides layer-by-layer checklists from Matrix ingress to tmux and back through the approval return path. Use it for problems like "the agent doesn't reply, Threads go astray, cards don't appear, clicking still shows expired."

## Pre-Release Acceptance Checklist

First record the following facts — do not guess state from display names or screenshots. Save this table as a short report with every test run; only then can old and new results be reconciled when the homeserver, bridge device, or runtime version changes:

| Item | What to record |
|------|---------|
| Versions | Robrix2 commit, agent-chat commit, homeserver version and date |
| Accounts | Full MXIDs of the human, the bridge, and every `@ac_*` |
| Bindings | room→group; every `(room, agent)→owner`; optionally group→project |
| Runtimes | agent name, Claude/Codex, model, managed marker, project path/mode |
| Rooms | Unencrypted project room; the E2EE approval room for every `(agent, owner)` |
| workflow | skill version, the four role names, worktree/commit SHA |

Minimal end-to-end acceptance:

1. Top-level messages without an @ do not wake agents; an explicit @ wakes only the target agent;
2. After an @ inside a Thread, direct replies stay in the same Thread with no duplicate display in the main timeline;
3. After a bridge restart, perform a second-hop reply; the delivery journal keeps the Thread continuous;
4. Have Claude trigger one protected command: the public room shows only a redacted waiting message while the owner's approval room shows a pending card;
5. After `Approve once`, the command executes exactly once; replaying the same verdict is rejected;
6. Codex completes `TRUST` on first launch; ordinary in-sandbox reads/writes do not raise approvals, while boundary-crossing operations produce a card;
7. Wrong owner, wrong room, expired card, and empty owner binding all fail closed;
8. `!ctl` / `!agentctl` cannot bypass approval in either the project room or the approval room;
9. The dashboard's Agent/Tasks/Pool reconciles against actual Git/worktree state;
10. Verify a workflow's final conclusion against the commit, the test-command results, and the PR/MR — not just the agent's prose.

## The Agent Doesn't Reply

Check the pipeline layer by layer; do not just restart everything:

```text
Matrix event
  → explicit mention / trusted room
  → bridge ingestion
  → backend message
  → push relay notification
  → managed tmux
  → Agent check_inbox
  → backend post/reply
  → Matrix puppet send
```

- Was the full target actually @-mentioned in the room; is `MATRIX_DEFAULT_WAKE` set to `off`;
- Are both the agent and its companion bridge joined; invite polling may default to roughly 60 seconds;
- Is `agentchat ls` showing online; is the dashboard heartbeat fresh;
- Backend inbox has the message but tmux is not advancing: check the push relay and the idle gate;
- Claude/Codex was manually restarted inside tmux: use `agentchat down/up` to restore a managed launch;
- The agent posts to the wrong place: check whether its outbound message references the original backend `reply_to`.

## Thread Replies Fall Into the Main Timeline

Inspect the inbound message's `matrixContext`, the referenced message's `matrixDelivery.primaryEventId`, and the local delivery journal. Distinguish three outcomes:

- The reply target belongs to another room: a security error — the send is refused;
- Delivery info is missing due to an old message or a failed write-back: degrade to a top-level send and log a warning;
- The agent called `post(group=...)` on its own without `reply_to`: this is a workflow call missing context, not a Matrix client dropping the Thread.

If the project room has E2EE enabled, the current agent group outbound path does not support it — migrate to an unencrypted board room; approval E2EE is not affected by this limitation.

## The Approval Card Doesn't Appear

Confirm from the entry point downward:

1. If tmux shows the runtime's own local permission selection box, first determine whether this is the waiting UI taken over by agent-chat; the criterion is whether a pending approval actually appears in the backend;
2. Was Claude started by the launcher with auto + Ask rules; is the Codex hook trusted and its hash matching;
3. Did the approval store find a unique `(room, agent)→owner`; otherwise it is `owner_binding_missing/ambiguous`;
4. Has the owner joined the approval room; otherwise it is `owner_invite_pending`;
5. Did the bridge send `com.agentchat.approval.request.v1`; is there a queued UTD in E2EE;
6. Has Robrix2 synced, decrypted, and rendered the custom event.

Do not substitute a chat-text "approve" for the card, and do not select Yes inside tmux to skip Matrix-side acceptance.

## Clicked Approve, but the Runtime Still Shows Expired/Denied

Reconcile `request_id`, `expires_at`, the Matrix `event.sender`, the approval room ID, the agent/project/project room, and the `input_digest`. The default TTL is 5 minutes; an agent retry generates a new request, and an old card — even if still visible in the UI — cannot approve the new request.

Check the final rejection code in the backend audit and the bridge's verdict logs. E2EE key delays can mean the verdict arrives after expiry; device refresh / session rotation lowers the probability but is not "guaranteed decryption." Do not auto-retry external write operations — first confirm the old request has reached a terminal state and produced no side effects.

## Where to Look for Operational State

| What you want to know | Authoritative / primary evidence |
|------------|---------------|
| Is the agent process online | managed process/tmux + backend heartbeat |
| Did the message reach the backend | backend message/inbox |
| Where the workflow stands | workflow state + durable task (if any) + Thread; there is currently no single authoritative source |
| Who can approve | bridge owner binding + the original Matrix invite sender |
| Approval outcome | backend approval store/audit |
| Is the code done | Git commit/worktree + actual test results |
| Is it published remotely | GitHub/AtomGit issue or change request status |

The Project Board is an aggregate view; it does not replace these source records. When states conflict, localize the problem layer by layer using the authoritative evidence in this table.
