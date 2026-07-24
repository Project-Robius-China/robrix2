# Team Collaboration in Practice

> **Scope**: This chapter is a guided tour of the five collaboration scenarios, plus the typical rhythm of a working day. Prerequisites: Chapter 4 (the system is up and running). The five chapters that follow all come with real screenshots and unfold in order of use.

Once deployment is done, your HAgency space looks roughly like this:

- One (or more) **project board rooms** (e.g. `robrix2-board`): humans plus one or more agent teams in the same room;
- Each task unfolds in its own **thread** within the board room, where agents continuously report progress;
- Each agent has a **DM** for one-on-one assignments;
- Each agent has an **`Approval:` encrypted approval room**, where dangerous operations wait for your sign-off.

Robrix2's multi-tab workspace is designed for exactly this shape — the row of tabs below is a live snapshot of a real collaboration session:

```text
robrix2-board │ [Thread] robrix2-board │ DM: wf_coordinator │ Approval: wf_coordinator │ Approval: wf_codex
```

## The Five Scenarios

| Chapter | Scenario | What you will see |
|------|------|---------|
| [5.1 Inviting Agents into Your Space](onboarding-agents.md) | Onboarding | Agent Access settings, framework selection, accepting bridge invitations |
| [5.2 The Project Board Room](board-room.md) | Same-room collaboration | Mixed membership of humans and multiple agent teams, workflow slash commands |
| [5.3 Thread Collaboration](threads.md) | Task tracking | Dispatching into threads, progress follow-ups, the Threads panel |
| [5.4 Owner Approval](approvals.md) | Authorization | Encrypted approval cards, Approve once / Deny, fail-closed |
| [5.5 issue-workflow](issue-workflow.md) | Full workflow | A four-role team delivering a feature end to end |

## The Rhythm of a Typical Day

Stringing the five scenarios together, a real workday goes something like this:

**In the morning**, you assign an issue in the board room with `@wf_coordinator /go 012`, then go about your own work — no need to babysit. The coordinator's dispatch cover message appears in the main timeline, and the process folds into a thread.

**During the day**, Robrix2's notifications pull you back two or three times: once when the coordinator asks for direction in a thread (you settle it with a one-line reply); once when the `Approval: wf_codex` room lights up — the Codex final review wants to run a sandbox-escaping command, you glance at the command preview and click `Approve once`.

**Before signing off**, you open the Threads panel to scan the latest state of each thread, verify the finished tasks on a real machine, and have the coordinator send a draft PR (that step is another approval).

Your effort concentrates on **three kinds of high-value moments**: deciding, authorizing, and accepting. The rest of the time, the agent team runs itself — and you can open any thread at any moment to see the entire process. That is "transparent autonomy".
