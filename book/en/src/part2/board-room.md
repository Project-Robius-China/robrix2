# The Project Board Room: Humans and Multiple Agent Teams in One Room

> **Scope**: This chapter introduces the board room — HAgency's primary collaboration venue: who is in the room, how conversations work, and where the workflow commands come from. Prerequisite: Chapter 5.1.

The **board room** is an **unencrypted** Matrix room bound to an agent-chat group. `!bindroom` establishes room→group; human invitations establish room+agent→owner. Once both are correct, explicitly routed messages enter the backend and public Agent replies return under puppet identities.

## Who Is in the Room?

Type `@` in the input box and the member picker shows you the makeup of the space:

![@ member picker: humans, bridges, multiple Agent teams](../images/mention-picker-multi-team.png)

The `robrix2-board` room in this screenshot is home to:

- **Two humans**: alex (the screenshot's viewpoint) and Tyrese Luo;
- **Two bridge bots**: `agent-bridge-alexlocal` and `agent-bridge-tyrese` — each representing an independent agent-chat instance;
- **alex's Agent team**: `wf_coordinator`, `wf_codex`;
- **Tyrese's Agent team**: `tyrese_coordinator`, `tyrese_implementer`, `tyrese_reviewer`, `tyrese_final_reviewer`.

The two instances belong to different people and can publish into the same room. Human→Agent routing uses explicit mentions. Agent→Agent dispatch inside one backend uses MCP/backend messaging. The bridge drops senders whose MXID starts with `@ac_` to prevent loops, so cross-instance Agent mentions are not a reliable execution channel; use a human handoff or treat them as public status only.

**Permission boundary**: another user's Agent may answer discussion according to their instance policy, but protected operations go only to its owner. Each backend controls only its own Agents, local paths, tokens, models, and dispatch leases.

## @ Is Execution Routing, Not Just a Notification

With `MATRIX_DEFAULT_WAKE=off`, an unmentioned top-level message may be stored but wakes no Agent. A rich reply may currently infer the replied-to puppet; teams that require an explicit @ every time should verify that behavior separately rather than treat reply inference as an authorization boundary.

| Input | Expected behavior |
|------|------|
| Top-level message, no @ | no Agent wake |
| `@wf_coordinator ...` | wake that Agent |
| Mention two Agents | both targets receive work |
| Agent posts publicly | humans can read it; another instance does not automatically treat it as a task |

## Workflow Slash Commands

When a `*_coordinator` Agent is present in the room, Robrix2's `/` command palette gains a group of **Workflow Commands** (provided you built with `--features agent_chat` per Chapter 4.1 and enabled the agent-chat toggle in Preferences):

![workflow slash commands](../images/workflow-slash-commands.png)

- `/create-issue` — open an issue: draft a spec and ask you to confirm;
- `/go` — run an issue end to end: plan → implement → review → final review;
- `/review` — rerun review + Codex final review for a given issue;
- `/status` — query the current state of an issue / workflow.

**These commands are plain text sent to the coordinator.** Robrix2 only completes them; only a coordinator with a compatible workflow skill interprets them. No skill, offline Agent, or missing mention means no automatic backend workflow run.

```text
@wf_coordinator /create-issue Add alias management to room settings
@wf_coordinator /go 012
```

What happens after `/go` is the subject of [Chapter 5.5](issue-workflow.md). But before that, let's look at how a task unfolds inside a Thread.
