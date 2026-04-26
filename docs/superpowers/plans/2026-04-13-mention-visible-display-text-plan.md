# Visible Mention Display Text Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make selected `@mention` items render as friendly visible text in the composer, while still generating correct Matrix mention links and `Mentions` metadata when the message is sent.

**Architecture:** Replace the current “possible mentions + final string scan” model with explicit tracked visible-mention spans owned by `MentionableTextInput`. Selection inserts visible text like `@Alice `, text edits reconcile or invalidate only the affected tracked spans, and send-time transformation resolves those spans into Matrix mention links for markdown and `/html` while leaving `/plain` untouched.

**Tech Stack:** Rust, Makepad 2.0 `script_mod!`, `matrix-sdk`/`ruma` `Mentions`, unit tests in `src/shared/mentionable_text_input.rs`, `cargo test`, `cargo build`, `agent-spec`.

---

## File Map

- `src/shared/mentionable_text_input.rs`
  - Replace `possible_mentions: BTreeMap<OwnedUserId, String>` with a tracked visible-mention span model.
  - Change popup selection insertion from raw markdown link text to visible mention text.
  - Reconcile span state on `TextInputAction::Changed`.
  - Resolve tracked spans into outgoing markdown/HTML mention links and `Mentions` metadata at send time.
  - Host all new unit tests for span tracking, invalidation, and send transformation.

- `src/home/editing_pane.rs`
  - **No change expected for this task.**
  - Keep historical edit-message handling out of scope unless implementation reveals an unavoidable shared helper extraction.

- `specs/task-mention-visible-display-text.spec.md`
  - Source-of-truth contract for behavior; update only if implementation reveals a true wording bug.

## Key Implementation Decisions

- Store tracked mentions as explicit byte spans over the current composer text, not as a superset map.
- Each tracked mention should carry:
  - `user_id: OwnedUserId`
  - `visible_text: String` (for example `@Alice`)
  - `start: usize`
  - `end: usize`
- Span bookkeeping should use byte indices because the existing text input utilities and cursor indices are byte-based.
- Reconciliation should be conservative:
  - edits strictly before a span shift it
  - edits strictly after a span leave it unchanged
  - edits overlapping a span invalidate only that span
- `/plain` remains plain text and must not emit `Mentions`.
- Duplicate visible labels are valid because the stable key is `OwnedUserId + span`, not the label text.

---

### Task 1: Replace Raw Markdown Insertion with Visible Mention Tokens

**Files:**
- Modify: `src/shared/mentionable_text_input.rs`
- Test: `src/shared/mentionable_text_input.rs`

- [ ] **Step 1: Write the failing selection tests**

Add unit tests:
- `test_selecting_user_mention_inserts_visible_display_name`
- `test_selecting_user_mention_without_display_name_falls_back_to_localpart`

Each test should assert:
- inserted composer text is `@DisplayName ` or `@localpart `
- inserted composer text does not contain `matrix.to`
- cursor lands after the trailing space

- [ ] **Step 2: Run the new selection tests and confirm they fail**

Run:

```bash
cargo test test_selecting_user_mention_inserts_visible_display_name
cargo test test_selecting_user_mention_without_display_name_falls_back_to_localpart
```

Expected: FAIL because `on_user_selected()` still inserts markdown link text.

- [ ] **Step 3: Introduce the tracked visible-mention state model**

In `src/shared/mentionable_text_input.rs`:
- add a small `TrackedVisibleMention` struct near the widget state definitions
- replace `possible_mentions: BTreeMap<OwnedUserId, String>` with `tracked_visible_mentions: Vec<TrackedVisibleMention>`
- add a helper that derives visible mention text from popup selection:
  - prefer display name
  - fall back to MXID localpart
  - always prefix with `@`

- [ ] **Step 4: Change popup selection insertion to use visible mention text**

Update `on_user_selected()` so that:
- selecting `@room` keeps existing `@room ` behavior
- selecting a user inserts visible text rather than markdown link syntax
- the inserted mention span is registered immediately with `start/end/user_id/visible_text`
- the cursor is moved to the end of the visible mention plus trailing space

- [ ] **Step 5: Re-run the selection tests**

Run:

```bash
cargo test test_selecting_user_mention_inserts_visible_display_name
cargo test test_selecting_user_mention_without_display_name_falls_back_to_localpart
```

Expected: PASS.

---

### Task 2: Reconcile and Invalidate Mention Spans on Composer Edits

**Files:**
- Modify: `src/shared/mentionable_text_input.rs`
- Test: `src/shared/mentionable_text_input.rs`

- [ ] **Step 1: Write the failing span-reconciliation tests**

Add unit tests:
- `test_editing_inside_visible_mention_clears_tracking_for_that_mention`
- `test_duplicate_display_name_mentions_preserve_distinct_user_ids`

Also add one focused helper test for unchanged spans shifting correctly when text is inserted before them, for example:
- `test_visible_mention_spans_shift_when_edit_happens_before_them`

- [ ] **Step 2: Run the new reconciliation tests and confirm they fail**

Run:

```bash
cargo test test_editing_inside_visible_mention_clears_tracking_for_that_mention
cargo test test_duplicate_display_name_mentions_preserve_distinct_user_ids
cargo test test_visible_mention_spans_shift_when_edit_happens_before_them
```

Expected: FAIL because the widget currently has no span model or reconciliation helper.

- [ ] **Step 3: Add a pure reconciliation helper**

In `src/shared/mentionable_text_input.rs`:
- add a helper that compares `old_text` and `new_text`
- compute the changed byte window and net length delta
- return updated tracked spans by:
  - shifting spans fully after the edit
  - preserving spans fully before the edit
  - dropping only spans overlapped by the edit

Keep the helper pure so the tests can drive it directly.

- [ ] **Step 4: Wire reconciliation into `handle_text_change()`**

When text changes:
- reconcile `tracked_visible_mentions` against the new text before mention search logic continues
- clear all tracked mentions when the text becomes empty
- keep `possible_room_mention` semantics unchanged for `@room`

- [ ] **Step 5: Re-run the reconciliation tests**

Run:

```bash
cargo test test_editing_inside_visible_mention_clears_tracking_for_that_mention
cargo test test_duplicate_display_name_mentions_preserve_distinct_user_ids
cargo test test_visible_mention_spans_shift_when_edit_happens_before_them
```

Expected: PASS.

---

### Task 3: Resolve Visible Mentions into Outgoing Matrix Links and Metadata

**Files:**
- Modify: `src/shared/mentionable_text_input.rs`
- Test: `src/shared/mentionable_text_input.rs`

- [ ] **Step 1: Write the failing send-transformation tests**

Add unit tests:
- `test_create_message_with_visible_mentions_emits_matrix_links_and_mentions`
- `test_html_message_with_visible_mentions_emits_anchor_and_mentions`
- `test_plain_mode_visible_mentions_remain_plain_text_without_mentions`

These should assert:
- default markdown send path resolves visible mentions into Matrix user links
- `/html` send path emits `<a href="...">` using the visible label
- `/plain` keeps the visible label as plain text and emits no `Mentions`

- [ ] **Step 2: Run the new send tests and confirm they fail**

Run:

```bash
cargo test test_create_message_with_visible_mentions_emits_matrix_links_and_mentions
cargo test test_html_message_with_visible_mentions_emits_anchor_and_mentions
cargo test test_plain_mode_visible_mentions_remain_plain_text_without_mentions
```

Expected: FAIL because send-time logic still scans for pre-rendered markdown links.

- [ ] **Step 3: Add pure transformation helpers**

In `src/shared/mentionable_text_input.rs`:
- add a helper that resolves the current plain composer text plus tracked spans into:
  - transformed markdown text
  - transformed HTML text
  - `Mentions`
- make it preserve visible labels in the outgoing link text, for example `[@Alice](matrix.to...)`
- ensure duplicate visible labels resolve using span identity, not string matching

- [ ] **Step 4: Update `create_message_with_mentions()` to use the helpers**

Implement the minimal wiring:
- `/html`: build outgoing HTML from tracked spans, then attach `Mentions`
- default markdown: build outgoing markdown from tracked spans, then attach `Mentions`
- `/plain`: bypass mention resolution and return plain text without metadata
- remove or dead-code-eliminate the old `possible_mentions` scanning helpers

- [ ] **Step 5: Re-run the send-transformation tests**

Run:

```bash
cargo test test_create_message_with_visible_mentions_emits_matrix_links_and_mentions
cargo test test_html_message_with_visible_mentions_emits_anchor_and_mentions
cargo test test_plain_mode_visible_mentions_remain_plain_text_without_mentions
```

Expected: PASS.

---

### Task 4: Regression Verification and Manual Composer Checks

**Files:**
- Modify: `specs/task-mention-visible-display-text.spec.md` only if implementation forces a wording correction

- [ ] **Step 1: Run the focused mention test suite**

Run:

```bash
cargo test test_selecting_user_mention_inserts_visible_display_name
cargo test test_selecting_user_mention_without_display_name_falls_back_to_localpart
cargo test test_editing_inside_visible_mention_clears_tracking_for_that_mention
cargo test test_duplicate_display_name_mentions_preserve_distinct_user_ids
cargo test test_visible_mention_spans_shift_when_edit_happens_before_them
cargo test test_create_message_with_visible_mentions_emits_matrix_links_and_mentions
cargo test test_html_message_with_visible_mentions_emits_anchor_and_mentions
cargo test test_plain_mode_visible_mentions_remain_plain_text_without_mentions
```

Expected: PASS.

- [ ] **Step 2: Run full compile verification**

Run:

```bash
cargo build
agent-spec parse specs/task-mention-visible-display-text.spec.md
agent-spec lint specs/task-mention-visible-display-text.spec.md --min-score 0.7
```

Expected: PASS.

- [ ] **Step 3: Manual verification in the app**

Check in app:
- selecting a user from the `@mention` popup inserts `@DisplayName ` instead of raw markdown link syntax
- the input field never shows `matrix.to` after selection
- sending a default markdown message still produces a clickable Matrix mention in the timeline
- sending `/html` still produces a clickable Matrix mention in the timeline
- sending `/plain` leaves the visible mention as plain text with no structured mention behavior
- editing one selected mention token invalidates only that token, not neighboring mentions
- duplicate visible labels such as two `@Alex` mentions still notify both distinct users

