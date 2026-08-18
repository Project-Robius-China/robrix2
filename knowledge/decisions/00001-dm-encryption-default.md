---
kind: decision
id: ADR-001
title: "New direct messages are encrypted unless the target is a positively identified bot"
status: Accepted
---

## Context

Robrix2 integrates appservice bots and agents (BotFather, `/listbots`-discovered
bots, room-bound bots, the Agent registry) that generally cannot participate in
end-to-end-encrypted rooms. Upstream Robrix creates every DM with
`Client::create_dm()` (encrypted). PR #81 made `should_create_encrypted_dm`
return `false` unconditionally so bots could always receive messages, which
silently created **unencrypted** DMs with ordinary users (issue #306).

## Decision

Let `E(t)` be "a new DM with target `t` is created encrypted" and `B(t)` be
"`t` is a positively identified bot":

```
B(t) := t = resolved BotFather MXID (only when resolution succeeds)
      ∨ t ∈ known_bot_user_ids
      ∨ t ∈ { room_bindings[*].bot_user_id }
      ∨ t ∈ AgentRegistry

E(t) ⇔ ¬B(t)
```

1. Ordinary users always get an encrypted DM (`create_dm()`); only a bot as
   defined above gets `create_room()` without encryption.
2. Identification is local and positive. Failure to resolve the BotFather MXID
   is **not** evidence; unknown means encrypted (fail-closed).
3. The decision is computed once, in `AppState::should_create_encrypted_dm`,
   and every user-facing DM entry point calls it. Entry points never hardcode
   `create_encrypted: false`. The only exception is the agent-binding modal,
   whose target is a bot by construction.
4. Whenever a DM will be created unencrypted, the user sees a localized notice
   before the room is created.

## Consequences

Good, because ordinary DMs regain confidentiality by default and bots keep a
supported plaintext path.
Good, because the invariant is executable: it is bound to example unit tests,
`proptest` property tests and a structural guard in
`specs/task-dm-encryption-default.spec.md`, and lifted into
`specs/capabilities/dm-encryption.spec.md`.
Bad, because a bot that Robrix2 has never seen (not configured, discovered,
bound or registered) gets an encrypted DM and may not answer; the user must
register it as an agent first.

## Alternatives Considered

- Keep "always plaintext" (status quo before #306): rejected — silent loss of
  confidentiality for ordinary users.
- Detect bots server-side (appservice registration, `m.room.member` metadata):
  out of scope; not reliably observable from a client.
- Per-DM encryption toggle in the UI: deferred; adds a decision the user
  rarely wants to make.

## Next

Governed by the capability spec `specs/capabilities/dm-encryption.spec.md`
(promoted from `task-dm-encryption-default` Rules `dm-enc-1` and `dm-enc-2`);
task specs that touch DM creation should declare `satisfies: [ADR-001]`.
