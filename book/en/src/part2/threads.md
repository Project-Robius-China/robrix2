# Thread Collaboration: Every Task Gets Its Own Thread

> **Scope**: This chapter establishes HAgency's most important collaboration habit — tasks live in Threads, and the main timeline keeps only the cover; it also gives the routing rule for "what goes in a Thread, what goes on the main timeline, and what goes to a DM". Prerequisite: Chapter 5.2.

A board room quickly ends up with several things happening at once. If every bit of progress scrolled by on the main timeline, the room would become unreadable within a day. HAgency's convention: **one Thread per task** — process details go into the thread, and the main timeline keeps the task's "cover". Under thread-session mode this is more than a collaboration convention: on the backend, each Thread is an isolated agent session (see below).

The complete journey of a message from your Thread, to the Agent, and back into the same Thread:

```mermaid
sequenceDiagram
    participant H as You (Robrix2, inside the Thread)
    participant M as Matrix (Palpo)
    participant B as agent-chat bridge
    participant BE as backend :8090
    participant R as one-shot runner

    H->>M: @wf_coordinator how is it going? (m.thread relation)
    M->>B: room event
    B->>BE: converted to a backend message (thread context recorded)
    BE->>BE: routed to this (agent, thread)'s isolated session
    BE->>R: spawn a runner, feed it this thread's rebuilt context
    R->>BE: reply (a runner cannot choose its own reply target)
    BE->>B: outbound message (reply outbox; the target is derived by the backend)
    B->>M: rebuild m.thread + m.in_reply_to
    M->>H: the reply appears inside the Thread
```

## One isolated session per Thread

A Thread is not just a collapsed strand in the UI — on the backend, **every (agent, thread) pair is a persistent, isolated session**:

- **Context isolation**: constraints, code words, and preferences stated in Thread A never leak into Thread B. Ask the same agent the same question in two threads and each answer draws only on that thread's own history.
- **Processes are day labourers**: agents have no resident process. Each turn the backend spawns a one-shot runner (`claude -p` / `codex app-server`), feeds it this thread's context rebuilt from the database, and the runner exits after answering. A thread's "memory" lives in SQLite — restart the backend, ask again, and the agent still remembers what this thread discussed.
- **Parallel, not queued**: while Thread A runs a long task, Thread B's question is handled by another runner in parallel.
- **Reply targeting is a transport guarantee**: replies leave through the backend's reply outbox, and the thread attribution is derived by the backend from session records — the model cannot misdirect its own reply.
- **What stays shared is identity**: the agent's name, Matrix puppet, working directory, and long-term memory remain one — like a human assistant who stays focused inside each strand but remembers who they are.

### Configuring a session in-thread: `/thread` directives

An operator (an account in `MATRIX_OPERATOR_MXIDS`) can adjust **this one thread's** session from inside it:

```text
/thread model claude-haiku-4-5   # switch this thread's model (default restores)
/thread mode auto                # grant this thread write access (plan revokes)
```

Directive messages never enter the conversation context; the agent answers with a confirmation notice. Three caveats: nothing else may follow the directive on the same message (you get a usage hint otherwise); `mode auto` is an audited write grant — every later turn in that thread may write the workspace (concurrent writes stay serialized by the workspace lease); and the model must match the agent's framework — in a thread with several agents the directive applies to **all** of their sessions, and naming a Claude model for a Codex agent fails at runtime.

## Dispatch Goes into a Thread

After the coordinator takes on a task, it posts a dispatch summary on the main timeline (who it was dispatched to, what the scope is, what the process is after completion), and that message immediately becomes the Thread root:

![Dispatch summary on the main timeline with a 4-replies thread card](../images/board-room-dispatch.png)

Note the **`4 replies`** card below the message — that is the collapsed Thread. All subsequent progress lives inside it, and the main timeline is no longer flooded.

## Following Up and Interjecting in a Thread

Open the Thread (it becomes its own tab, `[Thread] robrix2-board`), and you can `@` an Agent to follow up just as in a normal room:

![Following up on progress in a Thread; the coordinator reports with structure](../images/thread-progress.png)

In the screenshot alex sends just one line — `@wf_coordinator how's it going?` — and the coordinator gives structured status: who took the task, when the task went active, whether the branch has new commits, what the process looks like next, and a promise to post proactive updates.

Reply targeting is a transport guarantee (see the session model above); the **cadence of proactive reporting**, however, is still a workflow-skill convention — an agent busy with a long task may go quiet for a while. Humans should still use `/status`, the Project Board, the dashboard's task/heartbeat, and Git status as fallbacks.

## How Thread Continuity Actually Works

The current implementation does not "guess the reply location because it sees the Agent is in some Thread"; it saves and rebuilds trusted relations:

1. On inbound, the bridge parses the `m.thread` root and the `m.in_reply_to` target and writes them into the backend message's `matrixContext`;
2. The Agent's reply references a trusted backend message ID;
3. The bridge recovers the Matrix event ID from that message's `matrixDelivery`;
4. When the source message is inside a Thread, the outbound message constructs both `m.thread` and a rich reply; when the source message is top-level, it makes only a top-level rich reply and never opens a new Thread on its own;
5. The outbound event ID is written to a local delivery journal first, then idempotently written back to the backend, so a bridge restarted between "sent to Matrix" and "not yet recorded in the backend" can still replay;
6. When the reply target and the target room do not match, it fails closed; when an old message has no delivery record, it degrades to a top-level message and logs a warning.

This also explains why multi-hop scenarios need the write-back: when Agent A's outbound message becomes Agent B's reply target, the second hop can still find the same Thread. Attachments may produce multiple events; `primaryEventId` determines the target for subsequent replies.

## The Threads Panel Overview

The Threads button in the top-right corner opens the room-wide thread panel, letting you scan the latest status of every thread at a glance:

![Threads panel](../images/threads-panel.png)

Combined with the multi-tab workspace, a typical working posture is: one tab for the main room, one tab each for two or three active Threads, and one tab for the approval room — every collaboration venue laid out on the same screen.

## Routing Rules: Thread, Main Timeline, DM

| Message type | Where it goes | Examples |
|---------|------|------|
| Task process: progress, rejections, decision requests, test evidence | **Thread** | "Fix round 4 in progress", "Go straight to a draft PR once both reviews pass?" |
| Conclusions the whole room needs to know | **Main timeline** | Task covers, final delivery summaries |
| One-on-one handoffs to a single Agent | **DM: <agent>** | Small items not worth the board room's attention |
| Approval requests and details | **Approval: <agent>** (automatic) | See Chapter 5.4; not something you choose |

> **One boundary note**: Robrix2's main timeline hides Thread messages by default (they only show in the Thread tab). So when an Agent's replies land correctly in the Thread, the main timeline "can't see" them — this is expected behavior, not lost messages. Conclusions that need room-wide broadcast are explicitly posted to the main timeline by the coordinator.

The current Thread outbound path covers **non-encrypted group rooms** only; encrypted approval DMs take a separate path. Do not enable E2EE on a project room in the current version and expect Agents to keep posting Thread replies normally.
