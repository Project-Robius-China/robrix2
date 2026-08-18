spec: task
name: "DM Encryption Default — Encrypt Ordinary-User DMs, Plaintext Only For Identified Bots"
inherits: project
tags: [bugfix, security, dm, e2ee, bot]
estimate: 0.5d
---

## Intent

Fix GitHub issue #306: every new direct-message creation path that consults
`BotSettingsState::should_create_encrypted_dm()` receives `false`, so a DM
started with an ordinary Matrix user from People search or a user profile is
silently created without end-to-end encryption.

The desired behavior is:

- a new DM with an ordinary user is created encrypted (`client.create_dm()`),
- a new DM is created unencrypted only when the target is a positively
  identified bot / appservice / agent that requires plaintext, and
- whenever Robrix is about to create an unencrypted DM, the user sees that
  choice before the room is created.

## Constraints

- Keep the public shape of `MatrixRequest::OpenOrCreateDirectMessage` (fields
  `user_profile`, `allow_create`, `create_encrypted`) unchanged
- Keep the signature of `BotSettingsState::should_create_encrypted_dm(&self, &UserId, Option<&UserId>) -> bool` unchanged
- Do not change the worker branch in `src/sliding_sync.rs` that maps
  `create_encrypted == true` to `client.create_dm()` and `false` to
  `client.create_room()`
- Do not introduce new dependencies
- Do not change Makepad DSL layout; only Rust logic and locale strings change
- Every new user-facing string must have both `en` and `zh-CN` keys in `resources/i18n/`

## Decisions

- Bot identification is positive and local: a target user is a bot when it is
  exactly one of (a) the resolved BotFather MXID from
  `resolved_bot_user_id(current_user_id)`, (b) an entry in
  `known_bot_user_ids`, or (c) a bot MXID recorded in `room_bindings`
- `BotSettingsState::should_create_encrypted_dm` returns `false` only for a
  bot as defined above and `true` for every other user, including when the
  BotFather MXID cannot be resolved
- Add `AppState::should_create_encrypted_dm(&self, target, current) -> bool`
  which additionally treats every MXID registered in `AgentRegistry` as a bot;
  the three DM entry points that previously called the `BotSettingsState`
  method directly (`src/app.rs` people-search + `DidNotExist` confirmation,
  `src/profile/user_profile.rs` DM button) call the `AppState` method instead
- `StartChatModal` in `src/home/add_room.rs` replaces its hardcoded
  `create_encrypted: false` with the `AppState` decision read from `scope`,
  defaulting to encrypted when `AppState` is unavailable
- The `DidNotExist` confirmation modal body appends a localized notice
  (`dm.create.unencrypted_notice`) when the DM will be created unencrypted;
  `StartChatModal` shows the same notice as an info popup before submitting an
  unencrypted request
- The Agents-settings "Open chat" row action (`src/settings/agent_settings.rs`)
  sends `allow_create: false` with the `AppState` decision, so an existing DM
  opens directly and a missing one goes through the same confirmation dialog
  (and notice) as People search; the agent-binding "add friend" flow in
  `src/settings/agent_add_modal.rs` keeps its explicit plaintext request
  because that modal exists solely to bind a bot/agent
- Each invariant below is proven by example unit tests plus at least one
  `proptest` property test that asserts the invariant literally over generated
  `BotSettingsState` / target / current-user inputs; the property tests live in
  `src/app.rs` `mod tests::dm_encryption_props`
- The "single decision point" invariant is additionally enforced mechanically
  by `agent-spec check-structure --forbid "create_encrypted: false" --in "src/{home,profile}/**"` (plus `src/settings/agent_settings.rs`)

## Boundaries

### Allowed Changes
- `src/app.rs`
- `src/profile/user_profile.rs`
- `src/home/add_room.rs`
- `src/settings/agent_settings.rs`
- `resources/i18n/en.json`
- `resources/i18n/zh-CN.json`
- `specs/task-dm-encryption-default.spec.md`
- `specs/project.spec.md`
- ./Cargo.toml
- ./Cargo.lock
- ./CLAUDE.md

### Forbidden
- Do not modify `src/sliding_sync.rs`
- Do not modify `src/settings/agent_add_modal.rs`
- Do not modify matrix-sdk `create_dm()` / `create_room()` usage
- Do not add a global "always plaintext" toggle
- Do not add `proptest` to `[dependencies]` (dev-dependency only); `Cargo.toml` / `Cargo.lock` / `specs/project.spec.md` may change only for that dev-dependency and its recorded decision
- Do not run `cargo fmt`

## Acceptance Criteria

<!--
Invariants (E(t) = "new DM with t is encrypted", B(t) = "t is a positively identified bot"):

  B(t) := t = resolved_botfather(current)   (only when resolution succeeds)
        ∨ t ∈ known_bot_user_ids
        ∨ t ∈ { room_bindings[*].bot_user_id }
        ∨ t ∈ agent_registry                 (AppState level)

  dm-enc-1  E(t) ⇔ ¬B(t)                                   (encrypt iff not a bot)
  dm-enc-2  ¬E(t) ⇒ positive evidence; resolve = Err ⇒ clause false   (fail-closed)
  dm-enc-3  ¬E(t) ⇒ notice shown before the room is created           (visibility)
  dm-enc-4  create_encrypted is decided once in AppState; user-facing entry points never hardcode false
            (sole exception: the agent-binding modal, whose target is a bot by construction)
-->

### Rule: dm-enc-1 — Encrypt iff the target is not an identified bot

Scenario: Ordinary user DM is encrypted
  Tags: critical
  Test: should_create_encrypted_dm_encrypts_ordinary_user
  Given default bot settings with BotFather resolved to `@bot:example.org`
  And the target user is `@alice:example.org`
  When `should_create_encrypted_dm` is evaluated
  Then it returns `true`

Scenario: Configured BotFather DM is unencrypted
  Test: should_create_encrypted_dm_plaintext_for_configured_botfather
  Given bot settings whose BotFather resolves to `@bot:example.org`
  And the target user is `@bot:example.org`
  When `should_create_encrypted_dm` is evaluated
  Then it returns `false`

Scenario: Room-bound bot DM is unencrypted
  Test: should_create_encrypted_dm_plaintext_for_room_bound_bot
  Given bot settings with a room binding to `@helper:example.org`
  And `@helper:example.org` is not the BotFather and not in known bots
  When `should_create_encrypted_dm` is evaluated for `@helper:example.org`
  Then it returns `false`

Scenario: Discovered bot DM is unencrypted
  Test: should_create_encrypted_dm_plaintext_for_known_bot
  Given bot settings whose `known_bot_user_ids` contains `@weather:example.org`
  When `should_create_encrypted_dm` is evaluated for `@weather:example.org`
  Then it returns `false`

Scenario: Unrelated user stays encrypted even when bots are configured
  Tags: critical
  Test: should_create_encrypted_dm_encrypts_unrelated_user_with_bots_configured
  Given bot settings with a known bot, a room-bound bot, and a resolved BotFather
  And the target user `@carol:example.org` matches none of them
  When `should_create_encrypted_dm` is evaluated
  Then it returns `true`

Scenario: Registered agent DM is unencrypted at the AppState level
  Test: app_state_should_create_encrypted_dm_plaintext_for_registered_agent
  Given an `AppState` whose `agent_registry` contains `@agent:example.org`
  And bot settings that do not know `@agent:example.org`
  When `AppState::should_create_encrypted_dm` is evaluated for `@agent:example.org`
  Then it returns `false`
  And it returns `true` for `@dave:example.org`

Scenario: Property — encryption decision equals negated bot identification for all inputs
  Tags: critical
  Test: prop_should_create_encrypted_dm_iff_not_identified_bot
  Given generated bot settings (BotFather as localpart, full MXID, empty, or malformed; 0-3 known bots; 0-3 room-bound bots)
  And a generated target user and optional current user drawn from an overlapping alphabet
  When `should_create_encrypted_dm` and an independent evidence oracle are both evaluated
  Then `should_create_encrypted_dm` equals the negation of the oracle for every generated case

Scenario: Property — AppState decision equals negated (agent ∨ bot) for all inputs
  Test: prop_app_state_encrypts_unless_agent_or_bot
  Given generated bot settings and 0-3 registered agents
  And a generated target user and optional current user
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

### Rule: dm-enc-3 — An unencrypted DM is visible before creation

Scenario: Unencrypted notice is localized in every shipped locale
  Test: dm_unencrypted_notice_i18n_keys_exist_in_all_locales
  Given the locale catalogs `en` and `zh-CN`
  When the key `dm.create.unencrypted_notice` is looked up
  Then each catalog contains a non-empty translation with the `{user}` placeholder substituted

Scenario: Confirmation dialog shows the unencrypted notice before creation
  Test: manual_test_dm_confirmation_shows_unencrypted_notice_for_bot
  Given the user starts a DM with a known bot from People search
  When the "Create New Direct Message" confirmation appears
  Then the body includes the unencrypted notice
  And starting a DM with an ordinary user shows no such notice

Scenario: Start-chat modal defaults to encrypted and warns before an unencrypted bot DM
  Test: manual_test_start_chat_modal_encryption_decision
  Given the "Direct Messages" start-chat modal is open
  When the user submits an ordinary user MXID
  Then the DM request is sent with `create_encrypted == true` and no notice appears
  When the user submits the MXID of a known bot
  Then an info popup with the unencrypted notice appears before the request is sent

Scenario: Agents-settings "Open chat" reuses the confirmation dialog and notice
  Test: manual_test_agent_settings_open_chat_uses_confirmation_notice
  Given a registered agent with no existing DM
  When the user clicks "Open chat" on its row in Settings → Agents
  Then the "Create New Direct Message" confirmation appears with the unencrypted notice
  And clicking "Open chat" for an agent with an existing DM opens that room directly

Scenario: Created room encryption state matches the decision
  Test: manual_test_created_dm_room_encryption_state
  Given a homeserver with E2EE available
  When the user creates a DM with an ordinary user and a DM with the configured BotFather
  Then the ordinary-user room has an `m.room.encryption` state event
  And the BotFather room has none

### Rule: dm-enc-4 — The encryption decision is made in one place

Scenario: User-facing DM entry points never hardcode a plaintext request
  Tags: critical
  Test: dm_entry_points_do_not_hardcode_plaintext
  Given the sources `src/home/add_room.rs`, `src/profile/user_profile.rs`, `src/settings/agent_settings.rs` and `src/app.rs`
  When they are scanned for the literal `create_encrypted: false`
  Then no non-comment occurrence exists
  And `add_room.rs`, `user_profile.rs` and `agent_settings.rs` call `should_create_encrypted_dm(`

## Out Of Scope

- A per-DM encryption toggle in the UI
- Detecting bots by server-side appservice registration or `m.room.member` metadata
- Migrating or re-encrypting existing unencrypted DM rooms
- Changing agent-settings / Octos onboarding DM flows
- Coverage-guided fuzzing (`cargo-fuzz`); the decision function has no byte-level parsing surface
