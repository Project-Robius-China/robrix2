spec: capability
name: "dm-encryption"
tags: [capability, security, dm, e2ee]
satisfies: [ADR-001]
---

## Intent

Long-lived behavioral truth for direct-message encryption in Robrix2, lifted
from `task-dm-encryption-default` (issue #306) and governed by
`knowledge/decisions/00001-dm-encryption-default.md` (ADR-001).

<!--
E(t) = "a new DM with target t is created encrypted"
B(t) = "t is a positively identified bot"
B(t) := t = resolved BotFather (only when resolution succeeds)
      ∨ t ∈ known_bot_user_ids ∨ t ∈ room-bound bots ∨ t ∈ AgentRegistry
dm-enc-1  E(t) ⇔ ¬B(t)
dm-enc-2  ¬E(t) ⇒ positive local evidence; resolution failure is never evidence
-->

## Acceptance Criteria

### Rule: dm-enc-1 — Encrypt iff the target is not an identified bot

Scenario: Ordinary user DM is encrypted
  Tags: critical
  Test: should_create_encrypted_dm_encrypts_ordinary_user
  Given default bot settings with BotFather resolved to `@bot:example.org`
  When `should_create_encrypted_dm` is evaluated for `@alice:example.org`
  Then it returns `true`

Scenario: Configured BotFather DM is unencrypted
  Test: should_create_encrypted_dm_plaintext_for_configured_botfather
  Given bot settings whose BotFather resolves to `@bot:example.org`
  When `should_create_encrypted_dm` is evaluated for `@bot:example.org`
  Then it returns `false`

Scenario: Unrelated user stays encrypted even when bots are configured
  Tags: critical
  Test: should_create_encrypted_dm_encrypts_unrelated_user_with_bots_configured
  Given bot settings with a known bot, a room-bound bot, and a resolved BotFather
  When `should_create_encrypted_dm` is evaluated for a user matching none of them
  Then it returns `true`

Scenario: Property — encryption decision equals negated bot identification for all inputs
  Tags: critical
  Test: prop_should_create_encrypted_dm_iff_not_identified_bot
  Given generated bot settings, target user and optional current user
  When `should_create_encrypted_dm` and an independent evidence oracle are both evaluated
  Then `should_create_encrypted_dm` equals the negation of the oracle for every generated case

Scenario: Property — AppState decision equals negated (agent ∨ bot) for all inputs
  Test: prop_app_state_encrypts_unless_agent_or_bot
  Given generated bot settings and 0-3 registered agents
  When `AppState::should_create_encrypted_dm` is evaluated
  Then it equals `!(target ∈ agents ∨ oracle_is_bot(target))` for every generated case

### Rule: dm-enc-2 — Plaintext requires positive evidence; unknown is safe

Scenario: Unresolvable BotFather does not disable encryption
  Test: should_create_encrypted_dm_encrypts_when_botfather_unresolvable
  Given bot settings with a localpart-only BotFather and no current user id
  When `should_create_encrypted_dm` is evaluated for `@alice:example.org`
  Then it returns `true`

Scenario: Property — every plaintext decision is backed by local evidence
  Tags: critical
  Test: prop_plaintext_requires_positive_evidence
  Given generated bot settings, target user and optional current user
  When `should_create_encrypted_dm` returns `false`
  Then the target equals the successfully resolved BotFather, or is a known bot, or is a room-bound bot
  And when BotFather resolution fails and the target is neither known nor bound the decision is `true`
