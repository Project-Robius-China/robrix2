# The Security Model

> **Scope**: This chapter gathers the security mechanisms scattered across earlier chapters into five principles, plus a "threat → defense" matrix. Prerequisites: Chapter 3 and Chapter 5.4. Evaluators may read this alongside Path B in the preface.

HAgency gives agents a great deal of freedom to act; that freedom must be matched by equally solid boundaries.

## Five Principles

**1. Robrix2 is never a source of authorization.** Robrix2 does only two things: display (approval cards, workflow status) and initiate (turn your click into a structured Matrix event). All authorization decisions happen on the agent-chat server side: the verdict's real sender (`event.sender`) must equal the bound owner account — no trust in display names, no trust in identities self-declared in the payload; room, agent, project, request_id, and input_digest must match item by item; approval binding fields are read only from the **original event**, so an `m.replace` edit cannot tamper with a card already sent. Even if the client is replaced or forged, server-side validation still holds.

**2. Approvals are single-use, time-limited, and replay-proof.** `Approve once` allows each card exactly one pass; the server consumes the approval before notifying the runtime, so an allow cannot be replayed. Default expiry is 5 minutes. `input_digest` is a SHA-256 over the canonical fields — agent, runtime, project, project room, owner, approval room, upstream request, tool description, and an input preview of at most 8KB — pinning the verdict to that one request record stored on the server.

**3. Fail-closed: every anomaly equals denial.** From the Codex hook to the Claude channel, a pipeline failure never turns into an allow. The Codex hook is bound to the script's SHA-256; on first enablement or when the hash changes, `TRUST` must be typed in a local TTY; the hook timeout is derived from the approval TTL plus a buffer. Claude relies on the managed `auto` mode and explicit Ask rules to route protected commands into the channel.

**4. Encrypted channels and key hygiene.** Approval-room content uses end-to-end encryption (Megolm); under normal key assumptions the homeserver cannot read the content, but it still sees membership, timing, and traffic metadata. Robrix2 refreshes the bridge's device keys and rotates the outbound session before sending a verdict, reducing UTD caused by device rotation; the bridge does bounded persistence for temporarily undecryptable verdicts while waiting for room keys. Failure at any step never results in a pass.

**5. Managed runtimes and minimal project scope.** Claude Code uses `--permission-mode auto` + the channel; Codex uses `workspace-write` + the `on-request` hook. The launcher refuses to take over a same-named tmux session lacking the managed marker and filters out permission-policy override arguments; it cannot stop a user from manually starting a wild CLI under another name or in another terminal. All guarantees therefore assume "the task was launched by the agent-chat launcher." With `agentchat project add`, only the designated repository or worktree is exposed, and the write-back boundary between `copy` and `symlink` must be chosen explicitly by the owner.

## Threat → Defense Matrix

| Threat | Defense | Source |
|------|------|------|
| Someone impersonates the owner in the group and says "agreed" | Text replies are not approvals; the verdict's `event.sender` is validated server-side | Principle 1 |
| Replaying an old approval | Single-use consumption + TTL + request_id binding | Principle 2 |
| Approve command A, actually execute command B | Content-level binding via `input_digest` | Principle 2 |
| Approval-pipeline failure causing a "default allow" | Fail-closed end to end; anomalies are always deny | Principle 3 |
| Homeserver or network snooping on approval content | Approval-room content is E2EE; the server still sees membership/timing metadata | Principle 4 |
| Tampering with the approval hook / bypassing managed launch | Hook SHA-256 self-check + TRUST confirmation + managed PID marker | Principles 3 / 5 |
| Editing (m.replace) an already-sent approval card | Binding fields are read only from the original event | Principle 1 |
| Running `!ctl` / `!agentctl` in the public project room to bypass | These control commands are explicitly rejected in project and approval rooms | Principle 1 |
| Letting an admin approve on the owner's behalf when no owner is configured | Empty/ambiguous owner binding is rejected outright, with no admin fallback | Principles 1 / 3 |
| Ordinary room messages waking every agent | `MATRIX_DEFAULT_WAKE=off`, explicit @ target routing | Principle 5 |

## Boundaries and Residual Risks

- If the owner's device is compromised, the attacker can send verdicts with the real MXID;
- A backend/bridge host or root-level attacker is beyond the application-layer threat model;
- Input beyond the 8KB preview is not fully displayed; the owner should reject unreadable, dynamically concatenated, or unattributed commands;
- The project board room is currently an unencrypted room; do not send secrets that must not be stored by the homeserver / federation members;
- E2EE hides content, not membership, timing, event size, or other metadata;
- Approval protects operations captured by the launcher/Ask/hook; it does not prove that arbitrary third-party tools are wired into approval;
- Workflow roles, proactive reporting, and the dual-review order are currently skill conventions and do not enjoy the same level of enforcement as the approval protocol.

Release acceptance should cover: unique owner, empty-owner rejection, wrong sender/room/digest rejection, expiry and replay rejection, both Claude/Codex runtime paths, redacted notification in the public room, control-command bypass prohibition, and no pass-through when bridge E2EE fails temporarily.
