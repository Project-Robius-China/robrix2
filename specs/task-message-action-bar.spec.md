spec: task
name: "Message Action Bar — Copy Button (Phase 1)"
inherits: project
tags: [feature, timeline, ui, message-actions]
estimate: 1d
---

## Intent

Add a Cherry Studio-style message action bar that is always visible below every text message in the timeline, on both desktop (macOS) and Android. Phase 1 contains a single Copy button that copies the message's plaintext body to the clipboard and shows a success toast. The bar reuses the existing `MessageAction::CopyText` pipeline (already used by the right-click/long-press context menu), and is designed as the extension point for future per-message actions (reply, translate, etc.) in later PRs.

## Decisions

- Placement: a new `message_action_bar` child View inserted into the `Message` template's `content` view in `src/home/room_screen.rs`, directly after `download_section` and before the reactions/read-receipts row, so the bar sits tight under the message content (placing it after the read-receipts row leaves a visually unacceptable vertical gap — user-tested 2026-07-13); left-aligned with the message body text
- Visibility: `visible: false` by default in the DSL template; `populate_message_view` sets it visible only for copyable messages: Text and Emote always, Notice only when the sender is a bot per `is_timeline_sender_bot` (agents replying via `m.notice` keep the copy button; client-generated management notices such as the "[App Service] …" feedback, which are sent as `m.notice` by the current user, do not show it)
- `CondensedMessage` re-declares its `body`/`content` subtree with `:=` (full replacement, not merge), so the same `message_action_bar` block is added to both the `Message` and `CondensedMessage` templates
- Button style: ghost icon button derived from `RobrixIconButton` — transparent background, 16px `ICON_COPY` icon, idle icon color `RBX_FG_TERTIARY`, hover background `RBX_HIT_HOVER` with icon color `RBX_FG_SECONDARY`, pressed background `RBX_HIT_DOWN`, corner radius `RBX_RADIUS_XS`; all colors from existing tokens, no new hex values in `room_screen.rs`
- One DSL definition for both platforms — no `AdaptiveView` desktop/mobile split for the bar
- Click dispatch: the button emits the existing `MessageAction::CopyText(details)` widget action targeted at `room_screen_widget_uid`, identical to the context menu's `copy_text_button` dispatch in `src/home/new_message_context_menu.rs`
- Success confirmation: add `enqueue_popup_notification` with `PopupKind::Success` and 2.0s auto-dismissal in the existing `MessageAction::CopyText` handler in `src/home/room_screen.rs`, so both the bar and the context menu paths gain the toast
- Empty-copy guard: when the stripped body is empty (scaffolding-only bot updates, e.g. progress/metrics messages), the handler leaves the clipboard untouched and shows a `PopupKind::Info` popup with the new i18n key `room_screen.popup.message.copy_empty` instead of claiming success
- Toast text: new i18n key `room_screen.popup.message.copied` added to `resources/i18n/en.json` and `resources/i18n/zh-CN.json`, resolved via the existing `tr_key` mechanism, with a key-existence unit test in `src/i18n.rs`
- Clipboard: existing `cx.copy_to_clipboard` with `plaintext_body_of_timeline_item` — no new cargo dependencies
- Copied text mirrors what the bubble displays: for bot-sent messages the handler passes the raw body through a pure helper `clipboard_text_for_message_body(body, sender_is_bot)` that returns `compute_bot_timeline_render_state(body, true).body`, stripping the embedded status / provider / `_metadata_` scaffolding lines; bot detection reuses `compute_timeline_bot_context` + `is_timeline_sender_bot` (identical to timeline rendering); human-sent messages copy the raw plaintext body verbatim

## Boundaries

### Allowed Changes
- src/home/room_screen.rs
- src/shared/icon_button.rs
- src/shared/styles.rs
- src/shared/design_tokens.rs
- src/i18n.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json
- specs/task-message-action-bar.spec.md
- docs/superpowers/plans/2026-07-10-message-action-bar.md

### Forbidden
- Do not modify the right-click / long-press context menu opening logic (`Hit::FingerDown` secondary / `Hit::FingerLongPress` handling)
- Do not modify `NewMessageContextMenu` buttons or layout in `src/home/new_message_context_menu.rs`
- Do not reorder or remove existing children of the `Message` template's `content` view — the new bar is inserted between existing children without changing their relative order
- Do not add a hover-reveal show/hide state machine — the bar is always visible in this phase
- Do not modify the `PortalList` timeline structure or item template registration beyond the `Message` template body

## Out of Scope

- Additional action buttons (reply, quote, translate, resend, delete, "more" menu)
- Hover-reveal or tap-to-toggle visibility modes
- Copy-as-HTML / formatted copy entry in the bar
- Action bar on image, video, audio, sticker, or file messages
- Multi-message selection and bulk copy
- New i18n framework work (toast text follows the existing string convention used by other copy confirmations)
- The right-click / long-press context menu's Copy Text stays available on ALL messages (including management notices) — the inline button's visibility gate deliberately does not restrict the menu
- Copy-as-HTML keeps the raw formatted body including bot scaffolding — cleaning the HTML copy path is a later phase

## Completion Criteria

Scenario: Action bar visible under a text message on desktop
  Test: manual_test_action_bar_visible_desktop
  Given a room timeline containing a text message from any sender
  When the timeline renders on macOS
  Then a `message_action_bar` row appears below the message body, left-aligned with the message text
  And the bar contains a single copy button with a gray `ICON_COPY` icon

Scenario: Clicking copy copies plaintext and shows a success toast
  Test: manual_test_copy_click_desktop
  Given a text message with body "hello robrix" is displayed on macOS
  When the user clicks the copy button in the message action bar
  Then the system clipboard contains "hello robrix"
  And a `PopupKind::Success` toast appears confirming the copy
  And the toast auto-dismisses within "2" seconds

Scenario: Tapping copy works on Android
  Test: manual_test_copy_tap_android
  Given a text message with body "hello robrix" is displayed on Android
  When the user taps the copy button in the message action bar
  Then the system clipboard contains "hello robrix"
  And a `PopupKind::Success` toast appears confirming the copy

Scenario: Condensed messages also show the action bar
  Test: manual_test_condensed_message_bar
  Given two consecutive text messages from the same sender so the second renders as `CondensedMessage`
  When the timeline renders
  Then the condensed message also shows the `message_action_bar` below its body

Scenario: Multi-line message copies the full plaintext body
  Test: manual_test_multiline_copy
  Given a text message whose body contains "3" lines separated by newlines
  When the user activates the copy button
  Then the clipboard contains all "3" lines with newlines preserved

Scenario: Image messages show no action bar
  Test: manual_test_image_message_no_bar
  Given a room timeline containing an image message
  When the timeline renders
  Then no `message_action_bar` is visible below the image message

Scenario: App Service management notices show no copy button
  Test: manual_test_app_service_notice_no_copy
  Given a room containing an "[App Service] Added bot ..." feedback message (an m.notice sent by the current user)
  When the timeline renders
  Then no copy button is visible on that message

Scenario: Bot replies sent as m.notice keep the copy button
  Test: manual_test_bot_notice_keeps_copy
  Given a message of type Notice from a sender identified as a bot
  When the timeline renders
  Then the copy button is visible on that message

Scenario: Redacted messages show no action bar
  Test: manual_test_redacted_message_no_bar
  Given a room timeline containing a redacted (deleted) message
  When the timeline renders
  Then no `message_action_bar` is visible below the redacted message

Scenario: Scroll drag starting on the copy button does not copy on Android
  Test: manual_test_android_scroll_no_copy
  Given a scrollable room timeline is displayed on Android
  When the user starts a vertical drag gesture with the finger initially on a copy button
  Then the timeline scrolls
  And the clipboard content is unchanged
  And no toast appears

Scenario: Copying a bot message excludes the metadata scaffolding
  Test: test_clipboard_text_strips_bot_scaffolding
  Given a bot message whose raw body embeds a footer line "_deepseek@api/deepseek-chat · 13.3K in · 162 out · 3s_"
  When the clipboard text is computed with sender_is_bot true
  Then the result contains the reply body text
  And the result does not contain "deepseek@api/deepseek-chat"

Scenario: Copying a human message preserves the body verbatim
  Test: test_clipboard_text_verbatim_for_human
  Given a human message whose body coincidentally contains a "via someone@api (model)" line
  When the clipboard text is computed with sender_is_bot false
  Then the result equals the raw body unchanged

Scenario: Copying a scaffolding-only bot update copies nothing and says so
  Test: manual_test_copy_empty_bot_progress
  Given a bot progress message whose body contains only status/provider/metrics scaffolding lines
  When the user activates the copy button
  Then the clipboard content is unchanged
  And a `PopupKind::Info` popup reports there is nothing to copy
  And no `PopupKind::Success` toast appears

Scenario: Copy fails gracefully when the event is missing from the timeline
  Test: manual_test_copy_event_not_found
  Given a `MessageAction::CopyText` is dispatched whose event is no longer present in the timeline items
  When the copy handler runs
  Then a `PopupKind::Error` popup appears reporting the failure
  And the clipboard content is unchanged
  And no `PopupKind::Success` toast appears

Scenario: Context menu Copy Text still works and gains the toast
  Test: manual_test_context_menu_copy_regression
  Given a text message with body "regression check" is displayed
  When the user opens the message context menu via right-click (macOS) or long-press (Android) and selects Copy Text
  Then the system clipboard contains "regression check"
  And a `PopupKind::Success` toast appears confirming the copy

Scenario: Build passes without formatting churn
  Test: manual_test_cargo_build
  Given the implementation is complete
  When `cargo build` runs on the feature branch
  Then the build succeeds with no new warnings introduced by the change
  And no files are reformatted by `cargo fmt`
