# Telegram Bot Mention/Reply-First Targeting Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the always-visible target chip/popup and make bot-bound rooms default to room-first message routing, with bot interaction driven by `@mention` and reply-to-bot.

**Architecture:** Keep the existing mention routing, reply-to-bot targeting, and `explicit_room` marker pipeline. Change the input bar’s default resolved target from `RoomDefault` to `ExplicitRoom` in bot-bound rooms, clear any persisted explicit target overrides from older builds, and hide the target chip/popup UI entirely so the main interaction path is text-first rather than mode-switch-first.

**Tech Stack:** Rust, Makepad 2.0 `script_mod!`, `cargo test`, `cargo build`, `agent-spec`.

---

## File Map

- `src/room/room_input_bar.rs`
  - Change default target resolution to room-first in bot-bound rooms.
  - Remove visible target-chip behavior from the input bar.
  - Stop restoring persisted explicit overrides from previous builds.
  - Update unit tests from chip/menu semantics to mention/reply-first semantics.

- `src/home/room_screen.rs`
  - Remove or neutralize the now-unused target popup trigger path if needed for compile cleanliness.

- `specs/task-tg-bot-mention-reply-first.spec.md`
  - Verification contract for the new direction.

---

### Task 1: Lock the New Routing Semantics with Failing Tests

**Files:**
- Modify: `src/room/room_input_bar.rs`
- Test: `src/room/room_input_bar.rs`

- [ ] **Step 1: Add failing tests for the new default behavior**

Add tests for:
- `test_bot_bound_room_defaults_to_explicit_room`
- `test_reply_to_human_in_bot_bound_room_stays_explicit_room`
- `test_reply_to_bot_still_targets_bot`
- `test_persisted_explicit_override_is_ignored_on_restore`

- [ ] **Step 2: Run the new tests and confirm they fail**

Run:

```bash
cargo test test_bot_bound_room_defaults_to_explicit_room
cargo test test_reply_to_human_in_bot_bound_room_stays_explicit_room
cargo test test_reply_to_bot_still_targets_bot
cargo test test_persisted_explicit_override_is_ignored_on_restore
```

- [ ] **Step 3: Update the target resolution helpers**

Implement the minimal logic so that:
- `ExplicitOverride::None + bound_bot_user_id` resolves to `ExplicitRoom`
- `reply-to-bot` still resolves to `ReplyBot`
- persisted explicit overrides restore as `None`

- [ ] **Step 4: Re-run the routing tests**

Run the same four tests and confirm they pass.

---

### Task 2: Remove the Visible Target UI from the Input Bar

**Files:**
- Modify: `src/room/room_input_bar.rs`
- Modify: `src/home/room_screen.rs`

- [ ] **Step 1: Add a failing UI-state test**

Add:
- `test_target_chip_hidden_in_bot_bound_room`

- [ ] **Step 2: Run the test and confirm it fails**

Run:

```bash
cargo test test_target_chip_hidden_in_bot_bound_room
```

- [ ] **Step 3: Hide the target indicator and stop exposing the popup**

Make the minimal code changes so:
- `sync_target_indicator()` always hides the target chip
- the target chip no longer opens a popup in normal use
- any stale popup path in `RoomScreen` is neutralized or left dormant but unreachable

- [ ] **Step 4: Re-run the UI-state test**

Run:

```bash
cargo test test_target_chip_hidden_in_bot_bound_room
```

---

### Task 3: Verify Mention/Reply Routing Still Works End-to-End

**Files:**
- Modify: `src/room/room_input_bar.rs`

- [ ] **Step 1: Keep and adapt the mention/reply regression tests**

Ensure these tests still cover the final behavior:
- `test_message_bot_mention_keeps_explicit_room_marker`
- `test_text_mentions_known_bot_matches_localpart`
- `test_message_mentions_room_member_bot_with_empty_known_bot_list`
- `test_message_mentions_known_bot_prefers_structured_mentions`

- [ ] **Step 2: Run the focused regression suite**

Run:

```bash
cargo test test_message_bot_mention_keeps_explicit_room_marker
cargo test test_text_mentions_known_bot_matches_localpart
cargo test test_message_mentions_room_member_bot_with_empty_known_bot_list
cargo test test_message_mentions_known_bot_prefers_structured_mentions
```

- [ ] **Step 3: Run the final verification gates**

Run:

```bash
cargo build
agent-spec parse specs/task-tg-bot-mention-reply-first.spec.md
agent-spec lint specs/task-tg-bot-mention-reply-first.spec.md --min-score 0.7
```

