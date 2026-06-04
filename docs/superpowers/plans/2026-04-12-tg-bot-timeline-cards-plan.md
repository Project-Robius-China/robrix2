# Telegram Bot Timeline Cards Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Restyle bot-authored text messages in the room timeline into Telegram-inspired reply cards with a clear body card, lightweight status strip, and subdued metadata footer.

**Architecture:** Keep the current mention/reply-first routing and current Matrix payloads untouched. Add a small bot-message parsing layer in `room_screen.rs` that recognizes Octos' existing status/provider/footer text format, then render bot text messages through a dedicated timeline card sub-structure while preserving `HtmlOrPlaintext` for the extracted main body.

**Tech Stack:** Rust, Makepad 2.0 `script_mod!`, existing `HtmlOrPlaintext`, `cargo test`, `cargo build`, `agent-spec`.

---

## File Map

- `src/home/room_screen.rs`
  - Add pure helpers for parsing bot timeline layers from existing Octos text output.
  - Add bot card widgets / styling to `Message` and `CondensedMessage`.
  - Wire parsed bot layers into timeline population for text and notice messages.

- `src/shared/html_or_plaintext.rs`
  - Only touch if the bot card body needs a small style or spacing hook that cannot live entirely in `room_screen.rs`.

- `src/home/edited_indicator.rs`
  - Only touch if edited indicator placement or visual weight needs a small adjustment to fit the new footer hierarchy.

- `specs/task-tg-bot-timeline-cards.spec.md`
  - Verification contract for the work.

---

### Task 1: Lock Bot Timeline Parsing with Pure Failing Tests

**Files:**
- Modify: `src/home/room_screen.rs`
- Test: `src/home/room_screen.rs`

- [ ] **Step 1: Add parsing tests for the happy path and fallback cases**

Add tests for:
- `test_parse_bot_timeline_layers_extracts_status_provider_body_and_footer`
- `test_parse_bot_timeline_layers_falls_back_for_unmatched_bot_text`
- `test_parse_bot_timeline_layers_ignores_regular_user_messages`
- `test_parse_bot_timeline_layers_prefers_safe_fallback_for_malformed_metadata`
- `test_parse_bot_timeline_layers_invalid_metadata_does_not_panic`

- [ ] **Step 2: Run the focused parsing tests and confirm they fail**

Run:

```bash
cargo test test_parse_bot_timeline_layers_extracts_status_provider_body_and_footer
cargo test test_parse_bot_timeline_layers_falls_back_for_unmatched_bot_text
cargo test test_parse_bot_timeline_layers_ignores_regular_user_messages
cargo test test_parse_bot_timeline_layers_prefers_safe_fallback_for_malformed_metadata
cargo test test_parse_bot_timeline_layers_invalid_metadata_does_not_panic
```

- [ ] **Step 3: Add a minimal bot timeline layer parser**

Implement small pure helpers in `src/home/room_screen.rs`:
- one type to hold parsed bot layers
- one function that only activates for bot senders
- conservative parsing for:
  - optional top status line
  - optional `via provider (model)` line
  - optional trailing `_model · X in · Y out · Zs_` footer
- safe fallback to full-body rendering when the format is unmatched or malformed

- [ ] **Step 4: Re-run the parsing tests**

Run the same five tests and confirm they pass.

---

### Task 2: Add the Bot Reply Card Structure to Timeline Widgets

**Files:**
- Modify: `src/home/room_screen.rs`

- [ ] **Step 1: Add rendering-state tests for card visibility and hierarchy**

Add tests for:
- `test_bot_timeline_card_visible_for_bot_text_message`
- `test_bot_timeline_card_hidden_for_regular_user_message`
- `test_bot_status_strip_renders_above_body_and_not_inside_body`
- `test_bot_metadata_footer_renders_below_body`

- [ ] **Step 2: Run the rendering-state tests and confirm they fail**

Run:

```bash
cargo test test_bot_timeline_card_visible_for_bot_text_message
cargo test test_bot_timeline_card_hidden_for_regular_user_message
cargo test test_bot_status_strip_renders_above_body_and_not_inside_body
cargo test test_bot_metadata_footer_renders_below_body
```

- [ ] **Step 3: Add bot-specific card widgets to the message templates**

In `src/home/room_screen.rs`:
- extend `Message` with a bot-only card container around the message body
- add optional `status strip` and `metadata footer` regions
- keep the existing username row and `bot` badge
- keep the main reply body rendered through `HtmlOrPlaintext`
- make sure ordinary user messages still use the plain timeline path

- [ ] **Step 4: Populate the new bot card subviews**

Update the timeline population path so that:
- bot-authored text/notice messages use the parsed layers
- main body text is sent to `HtmlOrPlaintext`
- provider/footer text is routed to the lighter metadata views
- unmatched bot messages fall back cleanly without partial junk UI

- [ ] **Step 5: Re-run the rendering-state tests**

Run the same four tests and confirm they pass.

---

### Task 3: Preserve Reply Preview, Condensed Layout, and Final Rendering Semantics

**Files:**
- Modify: `src/home/room_screen.rs`
- Modify if needed: `src/shared/html_or_plaintext.rs`
- Modify if needed: `src/home/edited_indicator.rs`

- [ ] **Step 1: Add regression tests for shared timeline behavior**

Add tests for:
- `test_bot_timeline_card_body_uses_html_or_plaintext_rendering`
- `test_bot_timeline_card_preserves_reply_preview_and_condensed_layout`

- [ ] **Step 2: Run the regression tests and confirm they fail**

Run:

```bash
cargo test test_bot_timeline_card_body_uses_html_or_plaintext_rendering
cargo test test_bot_timeline_card_preserves_reply_preview_and_condensed_layout
```

- [ ] **Step 3: Adjust spacing and supporting widget hooks only where needed**

Make the minimal changes needed so that:
- reply preview still sits correctly above the bot card
- condensed bot messages still render a readable card body without restoring a full profile row
- edited indicator and footer do not visually compete
- `HtmlOrPlaintext` behavior remains unchanged for links, emphasis, and line breaks

- [ ] **Step 4: Run the targeted regression suite**

Run:

```bash
cargo test test_bot_timeline_card_body_uses_html_or_plaintext_rendering
cargo test test_bot_timeline_card_preserves_reply_preview_and_condensed_layout
```

- [ ] **Step 5: Run the final verification gates**

Run:

```bash
cargo build
agent-spec parse specs/task-tg-bot-timeline-cards.spec.md
agent-spec lint specs/task-tg-bot-timeline-cards.spec.md --min-score 0.7
```

- [ ] **Step 6: Manual GUI validation**

Run:

```bash
cargo run
```

Verify:
- bot replies read as distinct cards in mixed human/bot rooms
- status text is visually above the main reply, not inside it
- provider/model and token/latency text are visibly weaker than the answer
- long bot replies stay readable
- reply previews and condensed messages still align correctly
