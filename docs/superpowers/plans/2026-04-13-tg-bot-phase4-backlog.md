# TG Bot UI Alignment — Phase 4 Backlog

> **Date:** 2026-04-13
> **Status:** Active planning doc. Supersedes the Phase 2 explicit-target-chip
> direction. Written after Claude+Codex synced on the current state of the
> `tg-align` branch.

## Where we are right now

Phase 3 (`mention/reply-first`) is merged and is the current active design.
Phases 1 and 2 type model work (`ExplicitOverride`, `ResolvedTarget`,
`is_known_or_likely_bot()`) survived the pivot; the persistent target chip UI
did not. The relevant historical docs are marked superseded:

- `specs/task-tg-bot-explicit-targeting.spec.md` (Phase 2) — SUPERSEDED
- `docs/superpowers/plans/2026-04-11-tg-bot-architecture-review.md` — SUPERSEDED

The authoritative active spec for the current behavior is
`specs/task-tg-bot-mention-reply-first.spec.md`.

### What actually shipped on `tg-align`

| Commit | Scope |
|--------|-------|
| `86eb0626` | Octos health checks, room restore handling |
| `77a8ee4c` | Mention input display names + Event Source rendering (CJK) |
| `297b34da` | Splash card rendering for bot messages |
| `4ed6ac4d` | Clippy clone_on_copy fix for Cmd+Enter handler |
| `9e00e197` | Cmd/Ctrl+Enter sends messages in multiline input |
| `c8e448d2` | File download support for Matrix file messages |
| `1a339985` | Bot timeline card rendering polish |
| `bbaa4ef0` | Matrix bot UX → mention-first routing (Phase 3 pivot) |
| `7481a007` | Explicit Matrix bot targeting UX (Phase 2 foundation) |
| `98e38b50` | Phase 1 bot badge + slash commands (WIP merged forward) |

Also on the OctOS side: `3370cb7` added bidirectional media support to the
Matrix appservice channel (file/image/audio/video upload+download).

### Behavior summary

- **Multi-member bot-bound rooms:** default to `ExplicitRoom` (room-first).
  Plain messages go to the room; `explicit_room` flag suppresses OctOS fallback.
- **DMs (direct rooms):** default to `ExplicitBot(bound_bot_user_id)` so users
  can chat with a bot without @mentioning it every line.
- **Reply-to-bot:** always resolves to `ReplyBot(bot_user_id)` regardless of
  override state.
- **Reply-to-human:** never triggers bot targeting, even in a bot-bound room.
- **Persisted `ExplicitOverride`:** discarded on restore, since the chip UI
  that would have let users correct it no longer exists.
- **Target chip UI:** hidden. The DSL scaffolding (`target_indicator`,
  `TargetChipButton`, etc.) remains in `room_input_bar.rs` as dead code.

## P0 — Historical residue to clean up

These are mechanical cleanups. They do not change behavior, but leaving them
misleads reviewers and future agents into thinking the target chip is still
on the roadmap.

| File | Action |
|------|--------|
| `src/room/room_input_bar.rs` | Delete `TargetChipPresentation` struct, `#[cfg(test)] fn format_target_chip_presentation`, `target_chip_button` / `target_chip_dismiss_button` / `target_indicator` DSL blocks, empty `sync_target_indicator()` stub |
| `src/shared/mentionable_text_input.rs:1404` | Replace `if false /* TODO: add is_direct_room to RoomScreenProps */` with `if room_props.is_direct_room` — the field already exists and is used by `resolve_target` |
| `specs/task-tg-bot-explicit-targeting.spec.md` | Already marked superseded in frontmatter banner |
| `docs/superpowers/plans/2026-04-11-tg-bot-architecture-review.md` | Already marked superseded in banner |

## P1 — Pre-existing test failures (not blocking)

Two `room_input_bar` tests fail both with and without any current change
on `tg-align`, so they are not regressions from Phase 3:

- `test_message_bot_mention_suppresses_explicit_bot_target`
- `test_room_bot_mention_overrides_selected_explicit_bot`

Both expect `routing_directives_for_message(ExplicitBot, mentions_bot=true)`
to return `(None, false)`, but the function currently returns `(None, true)`
because of `explicit_room = target_user_id.is_none()`. Either the test
expectation or the function semantics is wrong; the team should decide which
and reconcile. Low priority — no user-visible impact, since mention-first
flow already keeps the `explicit_room` flag on mention messages.

## P2 — Next TG parity work (new specs to write)

These are the three things Codex flagged as the real remaining TG alignment
work after the mention/reply-first pivot. Each will get its own spec.

### Phase 4a: Bot menu button + pure command send-on-select

- **Problem:** The current entry point for bot commands is the local `/bot`
  shortcut text command. That is a power-user affordance, not a TG-style
  menu button, and it does not match the user's mental model of "click the
  bot icon to see what I can do."
- **Also:** Pure commands like `/listbots` and `/bothelp` currently insert
  the command text into the input and wait for the user to press enter.
  TG sends these immediately on select.
- **Spec file:** `specs/task-tg-bot-menu-button.spec.md` (Phase 4a)
- **Key design questions to lock down in the spec:**
  - Where does the menu button live? (next to the input bar? inside it?)
  - How is "pure" vs "parameterized" classified? (new field on
    `SlashCommand`? a hardcoded list?)
  - Does the menu button work in all bot-bound rooms or only DMs?
  - Does clicking the menu button open the same command popup that typing
    `/` already opens, or a distinct UI?

### Phase 4b: `/command@bot` explicit addressing

- **Problem:** In multi-bot rooms, `/listbots` is ambiguous — which bot does
  it target? TG uses `/command@BotName` syntax to disambiguate.
- **Spec file:** `specs/task-tg-bot-command-at-addressing.spec.md` (Phase 4b)
- **Key design questions:**
  - Parser: where does the `@bot` suffix parsing live? In
    `mentionable_text_input` or a shared utility?
  - Routing: does the parsed target become an `ExplicitBot` override, a
    one-shot `target_user_id` on the outgoing message, or both?
  - UI: does typing `/command@` trigger a bot-name autocomplete?
  - Fallback: in a single-bot room, is the `@bot` suffix optional?

### Phase 4c: Bot message action buttons / inline keyboard UX

- **Problem:** Bots currently communicate only through plain text + the
  new Splash card prototype. They have no way to present "click to confirm",
  "click to retry file generation", or similar action affordances. The
  real-world PPT regeneration incidents show this is painful.
- **Spec file:** `specs/task-tg-bot-action-buttons.spec.md` (Phase 4c)
- **Key design questions:**
  - Transport: how does a bot attach action data to a Matrix message?
    (new custom field like `org.octos.actions`? reuse Splash card for this?)
  - Rendering: inline keyboard (buttons under the message) vs reply keyboard
    (buttons replacing the input area) — we should start with inline.
  - Click handler: when a user clicks a button, what gets sent back? A new
    Matrix message with a `org.octos.action_response` field? A direct HTTP
    callback to OctOS?
  - Integration with Splash cards: the Splash renderer already exists; should
    action buttons just be clickable widgets inside a Splash body, or a
    separate concept?

## Division of labor

Based on Codex's handoff (2026-04-12T23:04:47Z session) and the fact that
Codex is more familiar with `room_input_bar.rs` DSL internals:

| Domain | Owner |
|--------|-------|
| Spec/product direction, backlog maintenance | Claude |
| Cleanup of historical residue (P0) | Codex |
| Implementation of Phase 4a/4b/4c | Codex |
| Spec review after each phase ships | Claude (via mempal peek) |

Claude's next action: write the three Phase 4 specs listed above, in order.

Codex's next action (once Claude's spec lands): execute P0 cleanup, then
start Phase 4a implementation against the new spec.
