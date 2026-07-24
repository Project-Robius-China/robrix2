# The Security Model

> **Scope**: This chapter gathers the security mechanisms scattered across the preceding chapters into five principles, plus a Threat → Defense matrix. Prerequisites: Chapters 3 and 5.4. Evaluators may want to read this alongside Path B in the preface.

HAgency gives Agents a great deal of freedom to act; that freedom must be matched by equally solid boundaries.

## The Five Principles

**1. Robrix2 is never a source of authorization.** Robrix2 does exactly two things: display (approval cards, workflow status) and initiate (turn your click into a structured Matrix event). All authorization decisions happen on the agent-chat server: the verdict's actual sender (`event.sender`) must equal the bound owner account — no trusting display names, no trusting whatever identity the payload claims; room, agent, project, request_id, and input_digest must match item by item; approval binding fields are read only from the **original event**, so an `m.replace` edit cannot tamper with a card already sent. Even if the client is replaced or forged, server-side verification still holds.

**2. Approvals are one-shot, time-bounded, and replay-proof.** `Approve once` means what it says — each card allows exactly one execution; the server "consumes" the approval before notifying the runtime, so an allow can never be replayed. Approvals expire after 5 minutes by default, and an expired click is rejected on both client and server. The `input_digest` (a SHA-256 over the canonicalized request content — including the tool name and command input preview) pins the authorization to **this one request**: change a single character within the preview and nothing matches (the preview is truncated at 8KB, covering the vast majority of real commands).

**3. Fail-closed: every anomaly equals denial.** From Codex's approval hook to Claude's permission channel, any failure anywhere in the chain — timeout, parse failure, channel unavailable, integrity check failing — leaves the runtime with an explicit **deny**, never a silent allow or an indefinite wait. The Codex approval hook is bound to the script's SHA-256 and self-verifies its integrity; enabling the hook for the first time requires explicitly typing `TRUST` in a local terminal.

**4. Encrypted channels and key hygiene.** Approval rooms enforce end-to-end encryption (Megolm), so approval details are invisible to the homeserver. Before sending a verdict, Robrix2 explicitly refreshes the bridge's device keys and rotates the outbound room key, guaranteeing the bridge's current device can decrypt; a verdict the bridge cannot yet decrypt is persisted and queued while waiting for the room key, rather than dropped or misjudged.

**5. Managed runtimes.** Agent coding runtimes are launched in managed mode: Claude Code uses `--permission-mode auto` plus the approval channel, with sensitive commands (`gh *`, `git push *`) configured to always ask; Codex uses the `workspace-write` sandbox plus `on-request` approvals. tmux sessions are started by a managed script and tagged — a "wild" instance manually restarted without the approval parameters is rejected by the launcher.

## Threat → Defense Matrix

| Threat | Defense | Source |
|------|------|------|
| Someone impersonates the owner in the group and says "approved" | Text replies are not approval; server-side verification of the verdict's `event.sender` | Principle 1 |
| Replaying an old approval | Single consume + TTL + request_id binding | Principle 2 |
| Approve command A, actually execute command B | Content-level binding via `input_digest` | Principle 2 |
| An approval-chain failure turning into "allow by default" | Fail-closed end to end; every anomaly is a deny | Principle 3 |
| Homeserver or network snooping on approval content | Approval room is E2EE; the server sees only ciphertext | Principle 4 |
| Tampering with the approval hook / bypassing the managed launcher | Hook SHA-256 self-verification + TRUST confirmation + managed PID tagging | Principles 3 / 5 |
| Editing (m.replace) an approval card already sent | Binding fields read only from the original event | Principle 1 |

**Scope of the model**: this model defends against "Agents executing beyond their authority" and "the authorization channel being forged / replayed / bypassed". It does not defend against the owner approving the wrong thing (which is why the card must show you the command preview clearly), nor against a root-level attacker on your machine (that is already beyond the assumptions of any application-layer security).

---

**An Agent's capabilities can be outsourced; a human's authorization cannot.** HAgency uses protocol design and cryptography to make that sentence a property of the system, not a nice sentiment.
