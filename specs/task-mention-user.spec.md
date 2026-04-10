spec: task
name: "@mention User Autocomplete"
inherits: project
tags: [feature, mention, ui, matrix]
estimate: 3d
---

## Intent

Provide @mention autocomplete functionality in the message input bar. When a user types `@` followed by text, a popup appears showing matching room members with avatars and display names. Selecting a member inserts a markdown mention link that includes Matrix mention metadata when the message is sent. The system supports background search for responsiveness in large rooms (1000+ members) and renders received mentions as colored pills in the timeline.

## Decisions

- Popup infrastructure: reuse `CommandTextInput` as the base widget via `#[deref]` composition
- Trigger character: `@` preceded by whitespace or start of text
- State machine: `Idle` → `WaitingForMembers` → `Searching` → `Idle` (also `JustCancelled` on ESC)
- Search execution: background thread via `cpu_worker::spawn_cpu_job()` with MPSC channel streaming results in batches of 10
- Search cancellation: `Arc<AtomicBool>` token for graceful abort
- Mention insertion format: `[{username}](matrixUri)` markdown link with trailing space
- Mention tracking: `possible_mentions: BTreeMap<OwnedUserId, String>` for users, `possible_room_mention: bool` for @room
- Mention extraction: scan final message text for tracked mention patterns before sending via `Mentions` struct
- Highlight rendering: Animator states with `selected: instance(0.0)` shader variable + `animator_cut(cx, ids!(highlight.on/off))` — NOT `script_apply_eval!` (fails on dynamic widgets)
- Timeline pill rendering: `MatrixLinkPill` widget with avatar + display name, current-user mentions in red `#d91b38`
- @room availability: controlled by user's room power levels via `MentionableTextInputAction::PowerLevelsUpdated`
- Popup styling: rounded corners (6px), border (#ddd), shadow, compact items (36px height)
- Username display: bold text, user_id right-aligned in lighter gray

## Boundaries

### Allowed Changes
- src/shared/mentionable_text_input.rs
- src/shared/command_text_input.rs
- src/shared/html_or_plaintext.rs
- src/room/member_search.rs
- src/cpu_worker.rs
- src/room/mod.rs
- src/lib.rs
- src/home/room_screen.rs

### Forbidden
- Do not modify the message sending pipeline in `sliding_sync.rs`
- Do not modify `RoomInputBar` DSL layout — the `mentionable_text_input` widget slot already exists

## Out of Scope

- Vertical alignment of inline MatrixLinkPill with surrounding text (known Makepad limitation)
- Mention extraction during message editing (editing_pane.rs has a TODO)
- Custom pill colors per user
- Desktop vs mobile adaptive popup layout (single desktop layout used)

## Completion Criteria

Scenario: Typing @ triggers mention popup
  Test: manual_test_at_trigger_popup
  Given a room with at least 3 members
  When the user types "@" in the message input
  Then a popup appears with header "Users in this Room"
  And the popup lists room members with avatars and display names

Scenario: Search filters members by typed text
  Test: manual_test_mention_search_filter
  Given a room with members "Alice", "Bob", "Alex"
  When the user types "@al"
  Then the popup shows "Alice" and "Alex"
  And the popup does not show "Bob"

Scenario: Keyboard arrow keys highlight items
  Test: manual_test_arrow_key_highlight
  Given the mention popup is showing results
  When the user presses ArrowDown
  Then the next item is visually highlighted with a blue background
  And the previous item returns to default background

Scenario: Selecting a mention inserts markdown link
  Test: manual_test_mention_insert_markdown
  Given the mention popup is showing results
  When the user selects "Alice" via Enter or mouse click
  Then the input text contains "[Alice](matrix:u/alice:example.com) "
  And the popup closes
  And the cursor is positioned after the trailing space

Scenario: ESC dismisses mention popup
  Test: manual_test_esc_dismisses_popup
  Given the mention popup is open
  When the user presses ESC
  Then the popup closes
  And typing another "@" immediately does not re-open the popup (JustCancelled state)

Scenario: Sent message includes Mentions metadata
  Test: manual_test_mentions_metadata
  Given the user inserted a mention for "@alice:example.com"
  When the user sends the message
  Then the RoomMessageEventContent includes Mentions with user_ids containing "alice:example.com"

Scenario: @room mention requires power level
  Test: manual_test_at_room_power_level
  Given the user has room notification power level
  When the user types "@room" and selects the @room item
  Then the message includes Mentions with room: true

Scenario: @room hidden without power level
  Test: manual_test_at_room_hidden
  Given the user does not have room notification power level
  When the user types "@"
  Then the popup does not show the @room option

Scenario: Background search keeps UI responsive
  Test: manual_test_background_search
  Given a room with 1000+ members
  When the user types "@a"
  Then the UI remains responsive during search
  And results appear progressively

Scenario: MatrixLinkPill renders for received mentions
  Test: manual_test_pill_renders
  Given a message containing an HTML mention link
  When the message is displayed in the timeline
  Then a pill-style widget renders with avatar and display name

Scenario: Current user mention renders with red background
  Test: manual_test_current_user_pill
  Given a message mentions the current logged-in user
  When the message is displayed in the timeline
  Then the mention pill renders with red background color

Scenario: No matches shows indicator
  Test: manual_test_no_matches
  Given a room with members "Alice", "Bob"
  When the user types "@zzzzzzz"
  Then the popup shows "No matching users found" indicator

Scenario: Popup shows loading while members sync
  Test: manual_test_loading_during_sync
  Given a room where members have not been synced yet
  When the user types "@"
  Then the popup shows a loading indicator
  And after sync completes the member list appears

Scenario: Stale search results are discarded
  Test: manual_test_stale_search_discarded
  Given a background search is in progress for "@al"
  When the user presses ESC and then types "@bo"
  Then results from "@al" search are discarded
  And only "@bo" results are displayed

Scenario: Unicode member names are searchable
  Test: manual_test_unicode_search
  Given a room with a member named "Zhang San"
  When the user types the first character of their name
  Then the popup shows the member in results
