# TG Bot Action Buttons Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Render bot-supplied inline action buttons below timeline messages and send a one-shot action response back to the original bot when a user clicks one.

**Architecture:** Implement Phase 4c with native Makepad widgets, not Splash-generated buttons. `room_screen.rs` will parse `org.octos.actions`, populate a fixed six-slot button row inside each message item, and submit a direct `MatrixRequest::SendMessage` with `target_user_id = original_sender`, `explicit_room = false`, `m.in_reply_to`, and `org.octos.action_response` custom fields. The input bar state is not consulted.

**Tech Stack:** Makepad 2.0 `script_mod!` DSL, native `Button`/`Robrix*Button` widgets, Matrix `RoomMessageEventContent`, existing `submit_async_request(MatrixRequest::SendMessage)` send path, project i18n popup notifications.

---

## File Map

- Modify: `src/home/room_screen.rs`
  - Add action-button DSL nodes to `Message` and `CondensedMessage`
  - Parse `org.octos.actions` from original event JSON
  - Populate per-message button rows
  - Handle button clicks, local disable/re-enable, and one-shot action-response sending
- Modify: `resources/i18n/en.json`
  - Add error/fallback strings for action response failure and accessibility label text
- Modify: `resources/i18n/zh-CN.json`
  - Add Chinese translations for the same strings

## Implementation Notes

- Use a fixed maximum of 6 buttons in the DSL. Hide unused slots instead of creating dynamic widget trees.
- Keep action buttons as a sibling of `splash_card` / `message` / `link_preview_view`; do not change message body structure.
- Parse event JSON once per populate path. If malformed entries are present, skip them and log warnings instead of failing the whole message.
- For the click path, build the outgoing `RoomMessageEventContent` directly in `room_screen.rs`. Do not route through input-bar reply state or `ResolvedTarget`.
- Use client-local disabled state keyed by `(event_id, action_id)` so double-clicks are suppressed until send success/failure resolves.

### Task 1: Parse and Render Action Rows

**Files:**
- Modify: `src/home/room_screen.rs`
- Test: `src/home/room_screen.rs` (existing unit-test module)

- [ ] **Step 1: Write failing parsing/render-state tests**

Add tests for:
- `test_parse_octos_actions_skips_malformed_entries`
- `test_parse_octos_actions_truncates_after_six`
- `test_action_buttons_render_state_hidden_without_actions`
- `test_action_buttons_render_state_with_primary_secondary_danger`

Each test should operate on small JSON fixtures and assert a compact parsed/render-state struct, not a whole widget tree.

- [ ] **Step 2: Run the new tests to verify RED**

Run:
```bash
cargo test parse_octos_actions --quiet
```

Expected:
- The new tests fail because action parsing/render-state helpers do not exist yet.

- [ ] **Step 3: Add minimal action model + parser**

In `src/home/room_screen.rs`, add focused helpers like:
```rust
#[derive(Debug, Clone, PartialEq, Eq)]
struct OctosActionButton {
    id: String,
    label: String,
    style: OctosActionStyle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OctosActionStyle {
    Primary,
    Secondary,
    Danger,
}
```

And a parser:
```rust
fn parse_octos_actions(
    original_json: &Raw<AnySyncTimelineEvent>,
) -> Vec<OctosActionButton>
```

Requirements:
- Read `content.org.octos.actions`
- Accept only entries with non-empty `id` and `label`
- Default unknown/missing style to `Secondary`
- Truncate after 6 entries
- Emit warnings for malformed/truncated entries

- [ ] **Step 4: Add DSL slots for six action buttons**

In both `Message` and `CondensedMessage` templates, add:
- `action_buttons := View { visible: false ... }`
- six named child buttons, e.g. `action_button_0` .. `action_button_5`

Use existing project button widgets:
- `primary`: `RobrixPositiveIconButton`
- `secondary`: `Button`
- `danger`: `RobrixNegativeIconButton`

Keep the container below `message` / `splash_card` and above `link_preview_view`.

- [ ] **Step 5: Populate buttons during timeline rendering**

Extend the existing message population path so text/notice messages also call a helper like:
```rust
populate_octos_action_buttons(cx, &item, event_tl_item.original_json(), event_id);
```

Requirements:
- Hide the entire container when no valid actions exist
- Show only the used button slots
- Set each visible button's text label and style
- Keep unused slots hidden

- [ ] **Step 6: Run focused tests to verify GREEN**

Run:
```bash
cargo test parse_octos_actions --quiet
```

Expected:
- All new parser/render-state tests pass.

### Task 2: Send Action Responses with One-Shot Bot Targeting

**Files:**
- Modify: `src/home/room_screen.rs`
- Test: `src/home/room_screen.rs` (existing unit-test module)

- [ ] **Step 1: Write failing click/send contract tests**

Add tests for:
- `test_build_action_response_targets_original_sender`
- `test_build_action_response_preserves_reply_relation_to_source_event`
- `test_click_action_button_disables_all_buttons_locally`
- `test_action_response_failure_reenables_buttons`

Model these around a pure helper plus a small local-state map, not around live Makepad events.

- [ ] **Step 2: Run the tests to verify RED**

Run:
```bash
cargo test action_response --quiet
```

Expected:
- Tests fail because the helper/state does not exist yet.

- [ ] **Step 3: Add action-response builder helper**

Add a helper in `src/home/room_screen.rs`:
```rust
fn build_octos_action_response_message(
    label: &str,
    action_id: &str,
    source_event_id: &EventId,
) -> RoomMessageEventContent
```

Requirements:
- Body fallback: `[Action: {label}]`
- Custom field:
```json
{
  "org.octos.action_response": {
    "action_id": "...",
    "source_event_id": "$event"
  }
}
```
- Reply relation: `m.in_reply_to` to the source event

- [ ] **Step 4: Add local disabled-state tracking**

Add a widget-local rust field in `RoomScreen` for disabled action keys, keyed by:
```rust
(OwnedEventId, String)
```

Add helpers to:
- mark all actions for one source event disabled on click
- clear them on send failure
- leave them disabled on send success

- [ ] **Step 5: Wire action-button clicks to MatrixRequest::SendMessage**

In the `RoomScreen` action handler:
- detect which action button was clicked and recover:
  - source event id
  - original sender user id
  - selected `action_id`
  - selected label
- immediately disable all buttons for that source event
- submit:
```rust
submit_async_request(MatrixRequest::SendMessage {
    room_id,
    timeline_kind,
    message,
    reply_to: None,
    target_user_id: Some(original_sender.to_owned()),
    explicit_room: false,
});
```

Do not consult input-bar reply state or mention parsing.

- [ ] **Step 6: Handle send failure popup + local re-enable**

Hook the existing Matrix send error path for this action-response request so failures:
- show popup `Failed to send action response`
- re-enable buttons for the original event

Keep success path silent.

- [ ] **Step 7: Run focused tests to verify GREEN**

Run:
```bash
cargo test action_response --quiet
```

Expected:
- All action-response tests pass.

### Task 3: Integrate with Existing Message Types and Verify End-to-End Behavior

**Files:**
- Modify: `src/home/room_screen.rs`
- Modify: `resources/i18n/en.json`
- Modify: `resources/i18n/zh-CN.json`
- Test: `src/home/room_screen.rs`

- [ ] **Step 1: Add failing integration tests**

Add tests for:
- `test_plain_message_without_actions_keeps_action_row_hidden`
- `test_splash_card_and_actions_coexist`
- `test_action_button_label_escaped`
- `test_unknown_style_falls_back_to_secondary`

- [ ] **Step 2: Run the tests to verify RED**

Run:
```bash
cargo test action_buttons --quiet
```

Expected:
- New integration tests fail until the final wiring is complete.

- [ ] **Step 3: Finish mixed rendering cases**

Ensure:
- plain messages without `org.octos.actions` render exactly as before
- `org.octos.splash_card` and `org.octos.actions` can both render on one message
- action labels are escaped before setting button text / fallback body text

- [ ] **Step 4: Add i18n strings**

Add:
- `room_screen.action_response_failed`
- `room_screen.action_button_prefix`

English and Chinese only; no new locales.

- [ ] **Step 5: Run targeted tests**

Run:
```bash
cargo test action_buttons --quiet
```

Expected:
- All 4c-specific tests pass.

- [ ] **Step 6: Run broader regression checks**

Run:
```bash
cargo test room_screen --quiet
cargo build
agent-spec parse specs/task-tg-bot-action-buttons.spec.md
agent-spec lint specs/task-tg-bot-action-buttons.spec.md --min-score 0.7
```

Expected:
- `cargo build` passes
- no existing bot timeline regressions
- spec parse/lint passes

- [ ] **Step 7: Manual verification**

Test in app:
1. Open a room with a bot message that includes `org.octos.actions`.
2. Confirm buttons render below the message body.
3. Click a button and confirm:
   - buttons disable immediately
   - one reply message is sent
   - it routes to the original bot
4. Force a send failure and confirm:
   - error popup appears
   - buttons re-enable
5. Confirm a normal message without actions does not show any button row.
