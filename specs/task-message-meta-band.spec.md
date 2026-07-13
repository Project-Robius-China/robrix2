spec: task
name: "Message Meta Band — V2 Layout (Phase 2)"
inherits: project
tags: [feature, timeline, ui, message-actions, layout]
estimate: 1d
---

## Intent

Restructure the per-message meta area into the approved V2 "single meta band" layout (design: `docs/superpowers/design/message-layout-variants.html`): one 24px row below the message content holding the copy button (left), bot model metadata (middle), and read-receipt avatars (right). The timestamp moves from the avatar column into the username row, and the bot badge switches from legacy blue to the accent pill style. This removes the current three scattered rows (metadata / copy / receipts) that make the message block feel loose. Builds directly on Phase 1 (`specs/task-message-action-bar.spec.md`).

## Decisions

- The Phase 1 `message_action_bar` View becomes the meta band, declared ONCE as `mod.widgets.MessageMetaBand` and instantiated by both templates: `flow: Right`, `align y 0.5`, all spacing/sizing/type from tokens (`SPACE_XS`, `RBX_ICON_SM`, `RBX_TEXT_META`, `RBX_FG_TERTIARY`), children in order: `copy_button` (ghost style), `metadata_label` (Fill-width Label, `max_lines: 1`, `text_overflow: Ellipsis`, hidden when there is no metadata), then `avatar_row` (moved into the band)
- The meta band is ALWAYS visible on `Message`/`CondensedMessage`-family items; per-populate visibility now controls children individually: `copy_button` visible only for copyable messages (Text/Emote always, Notice only from bot senders — Phase 1 logic, `set_data` param renamed `show_copy_button`), `metadata_label` visible only when the bot render state carries provider/footer text
- `metadata_label` text = provider and footer display text joined with " · " (both from the existing `compute_bot_timeline_render_state` result; `display_bot_footer_text` reused); returned by `populate_bot_text_message_content` and applied at the single `set_data`/`set_band_metadata` choke point in `populate_message_view`
- Per-populate band updates are previous-value-gated via `#[rust]` fields on the `Message` widget (`show_copy_button`, `band_metadata`): widgets are only touched when the value flips, the copy button gets `reset_hover` when newly shown (recycling hygiene, mirroring the download buttons), and `handle_event` skips the per-Actions `clicked()` lookup while the button is hidden
- The in-card `bot_metadata_footer` (with `bot_provider_label` / `bot_footer_label`) is REMOVED from both the `Message` and `CondensedMessage` templates and its populate block deleted — the band's `metadata_label` is its single replacement; the existing unit test `test_bot_metadata_footer_renders_below_body` stays unchanged (it asserts `compute_bot_timeline_render_state` provider/footer extraction, which still feeds the band)
- `avatar_row` moves out of the reactions row into the band's right end; `reaction_list` keeps its own dedicated row (chips can wrap); existing name-only lookups (`ids!(avatar_row)`) keep working unchanged
- Timestamp: in the `Message` template, `timestamp` moves from the `profile` column into `username_view` (after the bot badge, color `RBX_FG_TERTIARY`); `CondensedMessage` keeps its gutter timestamp unchanged (it has no username row); the populate call switches from `ids!(profile.timestamp)` to the name-only `ids!(timestamp)` so one call resolves both templates
- `edited_indicator` and `tsp_sign_indicator` stay in the `profile` column, unchanged
- `bot_badge` restyle: background `RBX_ACCENT_SOFT`, label color `RBX_ACCENT` (replaces legacy `COLOR_ACTIVE_PRIMARY` blue background with white text)
- `compute_bot_timeline_render_state` logic and its unit tests stay untouched; no new cargo dependencies; all colors from existing tokens

## Boundaries

### Allowed Changes
- src/home/room_screen.rs
- specs/task-message-meta-band.spec.md
- docs/superpowers/design/message-layout-variants.html
- docs/superpowers/plans/2026-07-13-message-meta-band.md

### Forbidden
- Do not modify the `MessageAction::CopyText` dispatch or handler (Phase 1 pipeline stays as-is)
- Do not modify the right-click / long-press context menu logic or `src/home/new_message_context_menu.rs`
- Do not change `compute_bot_timeline_render_state` or its existing unit tests
- Do not change the `CondensedMessage` gutter timestamp or the `SmallStateEvent` templates
- Do not remove or rename `reaction_list` or change its own-row placement

## Out of Scope

- Additional action buttons in the band (reply, translate — later phases)
- Hover-reveal behavior
- Redesign of the bot status strip, approval buttons, thread summary, or reply preview
- Dark theme

## Completion Criteria

Scenario: Bot message shows a single meta band
  Test: manual_test_meta_band_bot_desktop
  Given a bot text message with provider metadata is displayed on macOS
  When the timeline renders
  Then one band row below the bubble shows the copy button on the left, the model metadata text in the middle, and read-receipt avatars on the right
  And no separate metadata or receipts rows exist below the band

Scenario: Human text message shows copy and receipts only
  Test: manual_test_meta_band_human
  Given a text message from a human sender with at least one read receipt
  When the timeline renders
  Then the band shows the copy button on the left and receipt avatars on the right
  And no `metadata_label` text is visible

Scenario: Timestamp renders in the username row
  Test: manual_test_timestamp_in_name_row
  Given a full (non-condensed) message is displayed
  When the timeline renders
  Then the timestamp appears in the username row after the name and badge in `RBX_FG_TERTIARY` color
  And the avatar column contains only the avatar

Scenario: Bot badge uses the accent pill style
  Test: manual_test_bot_badge_accent
  Given a message from a sender identified as a bot
  When the username row renders
  Then the bot badge shows `RBX_ACCENT` colored text on an `RBX_ACCENT_SOFT` background

Scenario: Condensed message keeps its gutter timestamp
  Test: manual_test_condensed_gutter_timestamp
  Given two consecutive messages from the same sender so the second renders as `CondensedMessage`
  When the timeline renders
  Then the condensed message shows its timestamp in the left gutter column
  And the condensed message's meta band renders the copy button

Scenario: Image message keeps read receipts despite having no copy button
  Test: manual_test_image_receipts_preserved
  Given an image message with at least one read receipt
  When the timeline renders
  Then the band shows the receipt avatars on the right
  And no copy button and no metadata text are visible in the band

Scenario: Reactions render on their own row above the band
  Test: manual_test_reactions_own_row
  Given a text message with at least one emoji reaction
  When the timeline renders
  Then the reaction chips render on their own row between the message content and the meta band
  And the reaction chips do not share a row with the receipt avatars

Scenario: Bot metadata is not duplicated
  Test: manual_test_no_duplicate_metadata
  Given a bot text message with provider metadata
  When the timeline renders
  Then the model metadata text appears exactly once (in the meta band)
  And no metadata footer renders inside the bot card

Scenario: Unit tests pass with the relocated footer assertion
  Test: manual_test_cargo_test_footer_update
  Given the implementation is complete
  When `cargo test` runs
  Then all tests pass including the updated bot-footer placement test
  And the `compute_bot_timeline_render_state` tests are unchanged and passing

Scenario: Phase 1 copy flow still works
  Test: manual_test_phase1_copy_regression
  Given a text message with body "regression check" is displayed
  When the user activates the copy button in the meta band
  Then the system clipboard contains "regression check"
  And a `PopupKind::Success` toast appears

Scenario: Android band fits narrow width without overflow
  Test: manual_test_android_band_truncation
  Given a bot message with long provider metadata is displayed on a 390dp-wide Android screen
  When the timeline renders
  Then the metadata text truncates with an ellipsis
  And the copy button and receipt avatars remain fully visible without horizontal overflow
