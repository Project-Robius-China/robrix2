# Team Collaboration in Practice

> **Scope**: This chapter tours six collaboration scenarios and labels system behavior versus workflow convention. Prerequisite: Chapter 4.

Once deployment is done, your HAgency space looks roughly like this:

- One (or more) **project board rooms** (e.g. `robrix2-board`): humans plus one or more agent teams in the same room;
- Each task may unfold in its own **thread**; replies with trusted `reply_to` context continue the thread;
- Ordinary **DMs** are created on first one-to-one use;
- An **`Approval:` room** is created/reused per `(Agent, owner)` for protected operations.

Robrix2's multi-tab workspace is designed for exactly this shape — the row of tabs below is a live snapshot of a real collaboration session:

```text
robrix2-board │ [Thread] robrix2-board │ DM: wf_coordinator │ Approval: wf_coordinator │ Approval: wf_codex
```

## The Six Scenarios

| Chapter | Scenario | What you will see |
|------|------|---------|
| [5.1 Inviting Agents into Your Space](onboarding-agents.md) | Onboarding | Agent Access settings, framework selection, accepting bridge invitations |
| [5.2 The Project Board Room](board-room.md) | Same-room collaboration | Mixed membership of humans and multiple agent teams, workflow slash commands |
| [5.3 Thread Collaboration](threads.md) | Task tracking | Dispatching into threads, progress follow-ups, the Threads panel |
| [5.4 Owner Approval](approvals.md) | Authorization | Encrypted approval cards, Approve once / Deny, fail-closed |
| [5.5 issue-workflow](issue-workflow.md) | Full workflow | A four-role team delivering a feature end to end |
| [5.6 Project Board](project-board.md) | Global audit | Team status, specs, and issues on the dashboard |

## The Rhythm of a Typical Day

Stringing the scenarios together, a real workday might look like this:

**In the morning**, you send `@wf_coordinator /go 012`. Robrix2 sends plain text; whether it drafts a spec and updates a thread depends on an installed workflow skill.

**During the day**, the coordinator may ask for direction in a thread, and `Approval: wf_final_reviewer` may receive a protected-operation request. The first is a workflow convention; the second is protocol-enforced when the runtime, owner binding, and E2EE channel are valid.

**Before signing off**, compare Threads, Project Board, backend tasks, Git state, test evidence, and the target provider. The demo workflow may not write its internal stage to durable backend tasks, so no one view is the whole truth.

Keep human checkpoints around decisions, authorization, and acceptance. The system enforces authorization boundaries; decisions and proactive reporting still require workflow configuration and operational health.
