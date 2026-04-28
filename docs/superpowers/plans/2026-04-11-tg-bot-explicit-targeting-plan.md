# Telegram Bot Explicit Target Model Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement an explicit bot target model in the room input bar so users can see and control whether a message goes to the room, the bound bot, or a reply-to-bot target, while preserving normal Matrix reply semantics.

**Architecture:** Keep the existing Matrix send pipeline and `target_user_id` transport field, but replace the sticky `active_target_user_id` model with persisted `ExplicitOverride` plus runtime `ResolvedTarget`. `RoomScreen` precomputes bot-classification context and passes it through `RoomScreenProps`; `RoomInputBar` owns target resolution, chip/menu UI, persistence, and send-path integration.

**Tech Stack:** Rust, Makepad 2.0 `script_mod!`, `matrix-sdk`/`ruma`, serde-backed UI state, `cargo test`, `cargo build`, `agent-spec`.

**Repo rules for this plan:**
- Do not run `cargo fmt`.
- Do not commit during implementation until the user has manually tested the feature.
- `agent-spec lifecycle` is a whole-spec gate, not a per-selector tool. Use targeted `cargo test <filter>` commands during each task, then run full lifecycle verification at the end.

---

## File Map

- `src/home/room_screen.rs`
  - Add the new `is_known_or_likely_bot()` helper.
  - Expand `RoomScreenProps` with precomputed bot-classification context.
  - Resolve `resolved_parent_bot_user_id` from `AppState` when building room props.
  - Add unit tests to the existing `#[cfg(test)] mod tests`.

- `src/room/room_input_bar.rs`
  - Replace `active_target_user_id` with persisted `ExplicitOverride`.
  - Add `ResolvedTarget` and pure target-resolution helpers.
  - Update both send paths to resolve `target_user_id` from `ResolvedTarget`.
  - Add `TargetIndicator` DSL, chip/menu interaction, and formatting helpers.
  - Keep `replying_to` as the single source of truth for reply state.
  - Add unit tests to the existing `#[cfg(test)] mod tests`.

- `src/room/reply_preview.rs`
  - Only adjust spacing/layout if `TargetIndicator` and `ReplyingPreview` do not stack cleanly.
  - Do not change reply lifecycle logic here.

- `resources/i18n/en.json`
- `resources/i18n/zh-CN.json`
  - Add target-chip and target-menu strings under the existing `room_input_bar.*` namespace.

---

## Task 1: Bot Classification Context in `RoomScreen`

**Files:**
- Modify: `src/home/room_screen.rs` (`detected_bot_binding_for_members()` near lines 360-429, `is_likely_bot_user_id()` near lines 432-448, `RoomScreenProps` construction near lines 3522-3575, struct definition near lines 6471-6482, test module near lines 9035+)
- Test: `src/home/room_screen.rs`

- [ ] **Step 1: Add failing bot-detection tests in `room_screen.rs`**

Add these exact spec-bound tests to the existing `#[cfg(test)] mod tests`:

```rust
#[test]
fn test_bot_detection_configured_parent() { /* ... */ }

#[test]
fn test_bot_detection_heuristic_fallback() { /* ... */ }

#[test]
fn test_bot_detection_child_bot() { /* ... */ }

#[test]
fn test_bot_detection_rejects_normal_user() { /* ... */ }
```

Focus them on the pure helper signature from the spec:

```rust
is_known_or_likely_bot(user_id, resolved_parent_bot_user_id.as_deref(), &known_bot_user_ids)
```

- [ ] **Step 2: Run the new tests and confirm they fail**

Run:

```bash
cargo test test_bot_detection_
```

Expected:
- FAIL because `is_known_or_likely_bot()` does not exist yet, or because current logic does not cover all three detection paths.

- [ ] **Step 3: Implement `is_known_or_likely_bot()` without changing room-binding detection responsibilities**

Add a new helper adjacent to `is_likely_bot_user_id()`:

```rust
fn is_known_or_likely_bot(
    user_id: &UserId,
    resolved_parent_bot_user_id: Option<&UserId>,
    known_bot_user_ids: &[OwnedUserId],
) -> bool {
    known_bot_user_ids.iter().any(|known| known.as_str() == user_id.as_str())
        || resolved_parent_bot_user_id.is_some_and(|parent| parent == user_id)
        || is_likely_bot_user_id(user_id, resolved_parent_bot_user_id)
}
```

Keep `detected_bot_binding_for_members()` as the room-level binding detector. Do not collapse it into the new helper.

- [ ] **Step 4: Extend `RoomScreenProps` with precomputed bot context**

Add these fields:

```rust
pub resolved_parent_bot_user_id: Option<OwnedUserId>,
pub known_bot_user_ids: Vec<OwnedUserId>,
```

Populate them when constructing `RoomScreenProps` from `AppState`, next to the existing `bound_bot_user_id` logic:

```rust
let resolved_parent_bot_user_id = app_state
    .bot_settings
    .resolved_bot_user_id(current_user_id().as_deref())
    .ok();
let known_bot_user_ids = app_state.bot_settings.known_bot_user_ids();
```

Dummy/fallback props should use `None` / `Vec::new()`.

- [ ] **Step 5: Re-run the bot-detection tests**

Run:

```bash
cargo test test_bot_detection_
```

Expected:
- PASS for all four detection tests.

- [ ] **Step 6: Checkpoint the diff without committing**

Run:

```bash
git diff --stat -- src/home/room_screen.rs
```

Expected:
- Only `src/home/room_screen.rs` is touched for this task.

---

## Task 2: Replace Sticky Target State with Explicit Model

**Files:**
- Modify: `src/room/room_input_bar.rs` (`RoomInputBar` fields near lines 603-605, `resolve_target_user_id()` near lines 861-876, save/restore near lines 1588-1648, `RoomInputBarState` near lines 1770-1779, test module near lines 1797+)
- Test: `src/room/room_input_bar.rs`

- [ ] **Step 1: Add failing pure-resolution tests**

Add these spec-bound tests to `room_input_bar.rs`:

```rust
#[test]
fn test_reply_to_human_no_bot_targeting() { /* ... */ }

#[test]
fn test_reply_bot_overrides_explicit_room() { /* ... */ }

#[test]
fn test_chip_dismiss_returns_to_room_default() { /* ... */ }

#[test]
fn test_chip_dismiss_explicit_room_to_room_default() { /* ... */ }

#[test]
fn test_chip_dismiss_no_bound_bot() { /* ... */ }
```

Implement them against pure helpers, not Makepad widget rendering. The goal is to lock the precedence chain before wiring UI.

- [ ] **Step 2: Run the new tests and confirm they fail**

Run:

```bash
cargo test test_reply_to_human_no_bot_targeting
cargo test test_reply_bot_overrides_explicit_room
cargo test test_chip_dismiss_
```

Expected:
- FAIL because the current state model is still `active_target_user_id: Option<OwnedUserId>`.

- [ ] **Step 3: Introduce `ExplicitOverride` and `ResolvedTarget`**

Add pure enums:

```rust
#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum ExplicitOverride {
    #[default]
    None,
    Bot(OwnedUserId),
    Room,
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum ResolvedTarget {
    NoTarget,
    RoomDefault(OwnedUserId),
    ExplicitBot(OwnedUserId),
    ExplicitRoom,
    ReplyBot(OwnedUserId),
}
```

Also add helper functions:

```rust
fn resolve_target(
    explicit_override: &ExplicitOverride,
    replying_to_sender: Option<&UserId>,
    bound_bot_user_id: Option<&UserId>,
    resolved_parent_bot_user_id: Option<&UserId>,
    known_bot_user_ids: &[OwnedUserId],
) -> ResolvedTarget

fn resolved_target_user_id(target: &ResolvedTarget) -> Option<OwnedUserId>

fn clear_explicit_override_result(
    bound_bot_user_id: Option<&UserId>,
) -> ResolvedTarget
```

Design rule:
- `ReplyBot` is derived, never persisted.
- `ExplicitRoom` means “skip fallback bot”, not “no explicit state”.

- [ ] **Step 4: Replace the persisted state slot**

Rename/replace:

```rust
#[rust] active_target_user_id: Option<OwnedUserId>
```

with:

```rust
#[rust] explicit_override: ExplicitOverride
```

Do the same in `RoomInputBarState` save/restore. Keep `replying_to` exactly as-is; only the target intent slot changes.

- [ ] **Step 5: Re-run the precedence tests**

Run:

```bash
cargo test test_reply_to_human_no_bot_targeting
cargo test test_reply_bot_overrides_explicit_room
cargo test test_chip_dismiss_
```

Expected:
- PASS for the pure precedence/dismiss tests.

- [ ] **Step 6: Checkpoint the diff without committing**

Run:

```bash
git diff --stat -- src/room/room_input_bar.rs
```

Expected:
- Only `src/room/room_input_bar.rs` changes in this task.

---

## Task 3: Wire the Send Paths and Restore Semantics

**Files:**
- Modify: `src/room/room_input_bar.rs` (location send near lines 1037-1083, text send near lines 1095-1159, reply clear near lines 887-894, restore logic near lines 1641-1648)
- Test: `src/room/room_input_bar.rs`

- [ ] **Step 1: Add failing integration-oriented state tests**

Add these tests:

```rust
#[test]
fn test_explicit_bot_with_reply_to_human() { /* ... */ }

#[test]
fn test_cancel_reply_clears_reply_bot() { /* ... */ }

#[test]
fn test_explicit_override_persists_navigation() { /* ... */ }

#[test]
fn test_reply_bot_restores_with_replying_to() { /* ... */ }
```

Keep them deterministic by testing helper/state transitions directly where possible. Do not wait for full widget-render assertions.

- [ ] **Step 2: Run the new tests and confirm they fail**

Run:

```bash
cargo test test_explicit_bot_with_reply_to_human
cargo test test_cancel_reply_clears_reply_bot
cargo test test_explicit_override_persists_navigation
cargo test test_reply_bot_restores_with_replying_to
```

Expected:
- FAIL because reply send paths still use `replying_to.sender()` directly and persistence still assumes sticky target state.

- [ ] **Step 3: Add a bot-aware reply-target helper**

Add a helper that derives reply targeting only when the replied-to sender is actually a bot:

```rust
fn reply_bot_target_user_id(
    &self,
    room_screen_props: &RoomScreenProps,
) -> Option<OwnedUserId> {
    let reply_sender = self.replying_to
        .as_ref()
        .map(|(event_tl_item, _)| event_tl_item.sender());

    match resolve_target(
        &self.explicit_override,
        reply_sender,
        room_screen_props.bound_bot_user_id.as_deref(),
        room_screen_props.resolved_parent_bot_user_id.as_deref(),
        &room_screen_props.known_bot_user_ids,
    ) {
        ResolvedTarget::ReplyBot(user_id) => Some(user_id),
        _ => None,
    }
}
```

The pure resolver decides whether reply-to-human falls back to `ExplicitOverride` / `RoomDefault`.

- [ ] **Step 4: Update both send paths**

In both the location send path and text send path:
- compute `ResolvedTarget` once
- derive `target_user_id` from `resolved_target_user_id(&resolved_target)`
- keep Matrix `Reply` relation unchanged
- never set `target_user_id` to a human sender merely because the user clicked reply

The current raw pattern:

```rust
let reply_target_user_id = self.replying_to.as_ref().map(|(item, _)| item.sender().to_owned());
```

should disappear from both send paths.

- [ ] **Step 5: Keep reply and explicit target ownership separate**

On cancel reply:
- `replying_to` becomes `None`
- `ReplyBot` disappears because it is derived
- `ExplicitOverride` remains unchanged

On restore:
- `ExplicitOverride` comes from `RoomInputBarState`
- `ReplyBot` is re-derived if `replying_to` restores

- [ ] **Step 6: Re-run the integration tests**

Run:

```bash
cargo test test_explicit_bot_with_reply_to_human
cargo test test_cancel_reply_clears_reply_bot
cargo test test_explicit_override_persists_navigation
cargo test test_reply_bot_restores_with_replying_to
```

Expected:
- PASS for send-path and restore semantics.

- [ ] **Step 7: Build after send-path changes**

Run:

```bash
cargo build
```

Expected:
- PASS

---

## Task 4: Add `TargetIndicator` UI, Menu Interaction, and i18n

**Files:**
- Modify: `src/room/room_input_bar.rs` (DSL root near lines 175-220 and surrounding widget tree, event handling in `handle_actions()`, helper functions)
- Modify: `src/room/reply_preview.rs` only if vertical spacing becomes cramped
- Modify: `resources/i18n/en.json`
- Modify: `resources/i18n/zh-CN.json`
- Test: `src/room/room_input_bar.rs`

- [ ] **Step 1: Add failing presentation tests**

Add these spec-bound tests:

```rust
#[test]
fn test_target_chip_room_default() { /* ... */ }

#[test]
fn test_target_chip_hidden_no_bot() { /* ... */ }

#[test]
fn test_explicit_bot_via_chip_menu() { /* ... */ }

#[test]
fn test_explicit_room_via_chip_menu() { /* ... */ }
```

Do not make them depend on full Makepad rendering. Add a pure presentation helper so the tests can assert:
- visibility
- label text
- subdued-vs-normal style flag
- whether dismiss is shown

- [ ] **Step 2: Run the presentation tests and confirm they fail**

Run:

```bash
cargo test test_target_chip_
cargo test test_explicit_bot_via_chip_menu
cargo test test_explicit_room_via_chip_menu
```

Expected:
- FAIL because there is no target-chip presentation layer yet.

- [ ] **Step 3: Add a pure presentation formatter**

Introduce a helper such as:

```rust
struct TargetChipPresentation {
    visible: bool,
    label: String,
    subdued: bool,
    dismissible: bool,
}

fn format_target_chip_presentation(
    app_language: AppLanguage,
    resolved_target: &ResolvedTarget,
    bot_display_name: Option<&str>,
) -> TargetChipPresentation
```

Formatting rules must match the spec exactly:
- `RoomDefault` → `"Default: {display_name}"` in subdued style
- `ExplicitBot` → `"To {display_name}"`
- `ExplicitRoom` → `"To room"`
- `ReplyBot` → `"Reply → {display_name}"`
- display name falls back to localpart if no room-member display name is available

- [ ] **Step 4: Add `TargetIndicator` DSL to `RoomInputBar`**

Insert the new UI above `replying_preview` in the widget tree so the target chip is the top-most context row:

```rust
target_indicator := View {
    visible: false
    width: Fill
    height: Fit
    flow: Down

    target_chip_row := View {
        flow: Right
        align: Align{y: 0.5}
        // chip label + dismiss + menu anchor
    }

    target_menu_popup := RoundedView {
        visible: false
        // room option + bound bot option
    }
}
```

Reuse the inline popup pattern already used by `emoji_picker_popup` / `translation_lang_wrapper`. Do not introduce a new global popup framework for this task.

- [ ] **Step 5: Wire chip/menu actions in `handle_actions()`**

Required interactions:
- clicking the chip toggles the target menu
- selecting the room option sets `ExplicitOverride::Room`
- selecting the bound bot option sets `ExplicitOverride::Bot(bound_bot_user_id.clone())`
- clicking `×` resets `ExplicitOverride::None`
- reply-to-bot does **not** mutate `ExplicitOverride`; it only changes runtime `ResolvedTarget`

If the room has no bound bot, do not invent a multi-bot menu. Hide the bot option and fall back to the `NoTarget` / `To room` behavior from the resolver.

- [ ] **Step 6: Add i18n keys**

Add keys under the existing `room_input_bar.*` namespace, for example:

```json
"room_input_bar.target.default": "Default: {display_name}",
"room_input_bar.target.to_bot": "To {display_name}",
"room_input_bar.target.to_room": "To room",
"room_input_bar.target.reply_bot": "Reply → {display_name}",
"room_input_bar.target.menu.bound_bot": "{display_name}",
"room_input_bar.target.menu.room": "To room"
```

Mirror them in `zh-CN.json`.

- [ ] **Step 7: Adjust `reply_preview.rs` only if the stacked layout looks wrong**

Allowed adjustment:
- padding / margin / spacing between the new `TargetIndicator` row and the existing `ReplyingPreview`

Not allowed:
- changing the `ReplyingPreview` state machine
- moving reply state ownership into `reply_preview.rs`

- [ ] **Step 8: Re-run the target-chip tests and build**

Run:

```bash
cargo test test_target_chip_
cargo test test_explicit_bot_via_chip_menu
cargo test test_explicit_room_via_chip_menu
cargo build
```

Expected:
- PASS

---

## Task 5: Full Verification and Manual Smoke Test

**Files:**
- Modify: none expected unless verification uncovers defects in the files above
- Verify: `src/home/room_screen.rs`, `src/room/room_input_bar.rs`, `src/room/reply_preview.rs`, `resources/i18n/en.json`, `resources/i18n/zh-CN.json`

- [ ] **Step 1: Run focused cargo test batches**

Run:

```bash
cargo test test_bot_detection_
cargo test test_target_chip_
cargo test test_reply_
cargo test test_explicit_
cargo test test_chip_dismiss_
```

Expected:
- PASS across all new spec-bound unit tests.

- [ ] **Step 2: Run a full build**

Run:

```bash
cargo build
```

Expected:
- PASS

- [ ] **Step 3: Run full contract verification**

Run:

```bash
agent-spec lifecycle specs/task-tg-bot-explicit-targeting.spec.md --code . --format json
```

Expected:
- every scenario verdict is `pass`
- no boundary violations
- no spec quality regressions

- [ ] **Step 4: Manual smoke test in the app**

Run:

```bash
cargo run
```

Manual checklist:
1. Open a room with a bound bot and confirm the chip shows `Default: <display name>`.
2. Click the chip and switch to `To room`; send a message and confirm `target_user_id` is absent.
3. Switch back to the bound bot; send a message and confirm `target_user_id` is the bot.
4. Reply to a human message and confirm the message is a Matrix reply but not targeted to that human.
5. Reply to a bot message and confirm the chip changes to `Reply → <display name>`.
6. Cancel the reply and confirm the chip falls back to the underlying explicit/default state.
7. Navigate away and back; confirm `ExplicitOverride` restores and `ReplyBot` only restores if `replying_to` restores.

- [ ] **Step 5: Stop for user testing**

Run:

```bash
git status --short
git diff --stat
```

Expected:
- only the planned files changed
- no commit has been created yet

Do not commit until the user tests the feature and explicitly approves the commit step.

---

## Final Exit Criteria

Before calling this implementation complete, all of the following must be true:

- `cargo build` passes
- all spec-bound `cargo test` filters above pass
- `agent-spec lifecycle specs/task-tg-bot-explicit-targeting.spec.md --code . --format json` shows all scenarios as `pass`
- manual smoke test covers target chip visibility, chip menu switching, reply-to-human fallback, reply-to-bot temporary override, and navigation restore
- the worktree is ready for user testing, but **not yet committed**
