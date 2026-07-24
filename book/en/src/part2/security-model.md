# The Security Model

> **Scope**: This chapter gathers the security mechanisms scattered across the preceding chapters into five principles, plus a Threat → Defense matrix. Prerequisites: Chapters 3 and 5.4. Evaluators may want to read this alongside Path B in the preface.

HAgency gives Agents a great deal of freedom to act; that freedom must be matched by equally solid boundaries.

## The Five Principles

**1. Robrix2 is never a source of authorization.** Robrix2 does exactly two things: display (approval cards, workflow status) and initiate (turn your click into a structured Matrix event). All authorization decisions happen on the agent-chat server: the verdict's actual sender (`event.sender`) must equal the bound owner account — no trusting display names, no trusting whatever identity the payload claims; room, agent, project, request_id, and input_digest must match item by item; approval binding fields are read only from the **original event**, so an `m.replace` edit cannot tamper with a card already sent. Even if the client is replaced or forged, server-side verification still holds.

**2. Approvals are one-shot, time-bounded, and replay-resistant.** The server consumes before notifying the runtime. The default TTL is five minutes. `input_digest` hashes canonical fields including agent/runtime/project/project-room/owner/approval-room/request IDs, tool description, and up to 8KB of input preview, binding the verdict to the stored request record.

**3. Fail-closed: every anomaly equals denial.** Codex uses a SHA-256-bound hook that requires local `TRUST` on first use or hash change, with hook timeout derived from approval TTL plus buffer. Claude relies on managed auto mode and Ask rules. Failures do not become allow.

**4. Encrypted channels and key hygiene.** Approval bodies use Megolm E2EE, though membership, timing, and traffic metadata remain visible. Robrix2 refreshes bridge devices and rotates outbound sessions to reduce device-rotation UTD; the bridge queues temporarily undecryptable verdicts within bounded storage. Any failure remains fail-closed.

**5. Managed runtimes and least project scope.** Claude uses auto + channel; Codex uses `workspace-write` + `on-request`. The launcher rejects same-name tmux sessions without its marker and filters policy-overriding arguments, but cannot stop a user from launching an unrelated wild CLI elsewhere. All guarantees assume an agent-chat-launched runtime. `agentchat project add` should expose only required repositories/worktrees, with an explicit copy-vs-symlink choice.

## Threat → Defense Matrix

| Threat | Defense | Source |
|------|------|------|
| Someone impersonates the owner in the group and says "approved" | Text replies are not approval; server-side verification of the verdict's `event.sender` | Principle 1 |
| Replaying an old approval | Single consume + TTL + request_id binding | Principle 2 |
| Approve command A, actually execute command B | Content-level binding via `input_digest` | Principle 2 |
| An approval-chain failure turning into "allow by default" | Fail-closed end to end; every anomaly is a deny | Principle 3 |
| Homeserver or network snooping on approval bodies | Bodies are E2EE; membership/timing metadata remains visible | Principle 4 |
| Tampering with the approval hook / bypassing the managed launcher | Hook SHA-256 self-verification + TRUST confirmation + managed PID tagging | Principles 3 / 5 |
| Editing (m.replace) an approval card already sent | Binding fields read only from the original event | Principle 1 |
| Using `!ctl` / `!agentctl` in a project or approval room | Control commands are explicitly rejected in those rooms | Principle 1 |
| Falling back to admins when no owner exists | missing/ambiguous owner binding denies | Principles 1 / 3 |
| Waking every Agent with ordinary room text | `MATRIX_DEFAULT_WAKE=off` and explicit target mentions | Principle 5 |

## Boundaries and Residual Risks

- A compromised owner device can emit a verdict from the real MXID.
- A compromised backend/bridge host or root attacker is outside this application-layer model.
- Input after the 8KB preview is not fully displayed; reject opaque or dynamically assembled commands.
- Project rooms are currently unencrypted; do not post secrets.
- E2EE hides bodies, not membership/timing/size metadata.
- Approval protects operations captured by managed launcher/Ask/hook paths, not arbitrary third-party tools.
- Workflow roles, proactive reports, and review order are conventions, not approval-protocol guarantees.

Release acceptance should cover unique owner, empty-owner denial, sender/room/digest mismatch, expiry/replay denial, both runtime adapters, redacted public notice, blocked control-command bypass, and fail-closed behavior during temporary E2EE failure.
