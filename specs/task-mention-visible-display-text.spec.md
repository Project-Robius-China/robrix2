spec: task
name: "Visible @mention Display Text in Composer"
inherits: project
tags: [feature, mention, ui, matrix, composer]
---

## Intent

Make the mention composer show friendly visible mention text in the input field instead of raw markdown link syntax after the user selects an item from the `@mention` popup. The composer should display readable mention text such as `@octosbot` while preserving the correct Matrix mention link and `Mentions` metadata when the message is sent.

## Decisions

- Composer insertion format: selecting a user mention inserts visible text `@{display_name}` with a trailing space; if the display name is empty or unavailable, fall back to `@{localpart}`
- Internal terminology: the visible text in the input is a `visible mention`, while the generated outgoing link is a `resolved mention link`
- Mention tracking model: track inserted mentions as explicit mention spans/tokens keyed to `OwnedUserId`; do not rely on scanning the final composer text for markdown link patterns
- Send transformation: for markdown/default messages and `/html` messages, tracked visible mentions are converted to Matrix user links only when creating `RoomMessageEventContent`
- Generated link label: preserve the visible composer label in the outgoing link or HTML anchor text, e.g. `@Alice`
- Invalidating edited mentions: if the user edits inside a tracked visible mention token, only that token loses structured mention tracking; other tracked mentions remain intact
- Duplicate visible labels are supported: two tracked mentions may render as the same visible text (e.g. `@Alex`) while still resolving to different `OwnedUserId`s at send time
- `/plain` mode keeps its current plain-text semantics: visible mentions remain plain text and do not add Matrix mention links or `Mentions` metadata

## Boundaries

### Allowed Changes
- src/shared/mentionable_text_input.rs
- src/home/editing_pane.rs
- specs/task-mention-visible-display-text.spec.md

### Forbidden
- Do not change the mention popup search/ranking behavior
- Do not change received timeline `MatrixLinkPill` rendering
- Do not change bot routing behavior in `src/room/room_input_bar.rs`
- Do not add new cargo dependencies

## Out of Scope

- Parsing arbitrary hand-typed `@octos` / `@alice` text into structured mentions without popup selection
- Changing mention popup layout, styling, or keyboard navigation
- Retrofitting historical raw markdown mention links already present in edited messages into visible mention tokens
- Changing `/plain` mode to support structured mention metadata

## Completion Criteria

Scenario: Selecting a user mention inserts visible display text
  Test: test_selecting_user_mention_inserts_visible_display_name
  Given the mention popup contains a user with display name `"Alice"` and MXID `"@alice:example.com"`
  When the user selects that mention item
  Then the input text contains `"@Alice "`
  And the input text does not contain `"matrix.to"`
  And the cursor is positioned after the trailing space

Scenario: Sending a markdown message resolves visible mentions into Matrix links and metadata
  Test:
    Filter: test_create_message_with_visible_mentions_emits_matrix_links_and_mentions
    Level: unit
  Given the composer text contains a tracked visible mention `"@Alice "` for user `"@alice:example.com"`
  When `create_message_with_mentions()` is called for the default markdown path
  Then the outgoing message body contains a Matrix user link for `"@alice:example.com"`
  And the outgoing link label includes `"@Alice"`
  And `Mentions.user_ids` contains `"@alice:example.com"`

Scenario: Sending an HTML message resolves visible mentions into anchors and metadata
  Test:
    Filter: test_html_message_with_visible_mentions_emits_anchor_and_mentions
    Level: unit
  Given the composer text contains a tracked visible mention `"@Alice "` for user `"@alice:example.com"`
  And the message is sent via the `/html` path
  When `create_message_with_mentions()` is called
  Then the formatted HTML contains an `<a href="...">` anchor for `"@alice:example.com"`
  And the anchor text includes `"@Alice"`
  And `Mentions.user_ids` contains `"@alice:example.com"`

Scenario: Missing display name falls back to localpart in the composer
  Test: test_selecting_user_mention_without_display_name_falls_back_to_localpart
  Given the mention popup contains a user with MXID `"@octosbot:127.0.0.1:8128"` and no display name
  When the user selects that mention item
  Then the input text contains `"@octosbot "`
  And the input text does not contain `"matrix.to"`

Scenario: Editing inside one visible mention invalidates only that mention token
  Test: test_editing_inside_visible_mention_clears_tracking_for_that_mention
  Given the composer contains tracked visible mentions `"@Alice "` and `"@Bob "`
  When the user edits the `"@Alice"` token text directly
  Then `create_message_with_mentions()` does not include `"@alice:example.com"` in `Mentions.user_ids`
  And `create_message_with_mentions()` still includes `"@bob:example.com"` in `Mentions.user_ids`

Scenario: Duplicate display names remain distinct tracked mentions
  Test: test_duplicate_display_name_mentions_preserve_distinct_user_ids
  Given the composer contains two tracked visible mentions that both render as `"@Alex"`
  And the underlying users are `"@alex:one.example"` and `"@alex:two.example"`
  When `create_message_with_mentions()` is called
  Then `Mentions.user_ids` contains both `"@alex:one.example"` and `"@alex:two.example"`
  And the outgoing message contains distinct Matrix user links for both users

Scenario: Plain text mode keeps visible mentions as plain text without structured mention metadata
  Test:
    Filter: test_plain_mode_visible_mentions_remain_plain_text_without_mentions
    Level: unit
  Given the composer text contains a tracked visible mention `"@Alice "` for user `"@alice:example.com"`
  And the message is sent via the `/plain` path
  When `create_message_with_mentions()` is called
  Then the outgoing plain-text body contains `"@Alice"`
  And the outgoing message does not include `Mentions.user_ids`
