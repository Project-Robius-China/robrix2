spec: task
name: "Mobile Room Search Inline Flow (No Modal)"
inherits: project
tags: [bugfix, mobile, search, ui, room-filter]
estimate: 2d
---

## Intent

Fix the mobile search UX bug: desktop search is modal-driven, but mobile should not require opening a modal to search rooms/spaces. On mobile, users should type directly in the existing search input and get results in place. If there are no local results, mobile should present the same three remote search choices as desktop (`People`, `Rooms`, `Spaces`) and allow server-side search from that inline state.

Also clean up mobile search UI so it no longer shows placeholder/todo affordances such as `Search (TODO)` and redundant mobile search icon affordances.

## Constraints

- Keep desktop behavior unchanged:
  - desktop still opens `room_filter_modal` via the existing desktop entry
  - desktop modal layout and interaction remain intact
- Keep existing Matrix async request path for remote directory search:
  - `submit_async_request(MatrixRequest::SearchDirectory { query, kind, limit })`
- Keep existing remote search kinds and semantics:
  - `RemoteDirectorySearchKind::People`
  - `RemoteDirectorySearchKind::Rooms`
  - `RemoteDirectorySearchKind::Spaces`
- Keep existing room-open / join / DM-open behavior for selected results
- Do not add new dependencies
- Do not run `cargo fmt` / `rustfmt`

## Decisions

- Mobile search flow is inline-only:
  - typing in the mobile `RoomFilterInputBar` immediately drives search/filter behavior
  - mobile must not require opening `room_filter_modal`
- Mobile no-result state for non-empty query shows three remote search buttons inline (People/Rooms/Spaces), matching desktop modal logic and text keys
- Clicking inline remote search buttons on mobile triggers the same remote search request path and result handling semantics as desktop
- Remote search loading, empty, and failure messaging on mobile reuses existing room-filter i18n messages (`app.room_filter.*`) for consistency
- Stale remote responses must be ignored when input query changed before response arrives
- Mobile UI cleanup:
  - remove redundant mobile search icon affordance in the search area
  - remove/replace visible `Search (TODO)` wording; no TODO-labelled search text remains in the mobile rooms sidebar

## Boundaries

### Allowed Changes
- `src/app.rs`
- `src/home/rooms_sidebar.rs`
- `src/shared/room_filter_input_bar.rs`
- `src/home/search_messages.rs` (only if needed for TODO text cleanup/removal)
- `src/home/rooms_list_header.rs` (only if needed for mobile/desktop search entry separation)
- `resources/i18n/en.json`
- `resources/i18n/zh-CN.json`

### Forbidden
- Do not redesign desktop search modal UI
- Do not change unrelated navigation/tab behavior
- Do not change Matrix backend request payloads or protocol behavior
- Do not run formatting tools or reformat unrelated code

## Acceptance Criteria

Scenario: Desktop search entry and modal behavior remain unchanged
  Test: manual_test_desktop_search_modal_unchanged
  Given desktop layout is active
  When user clicks the search entry in rooms header
  Then `room_filter_modal` opens as before
  And desktop search interaction remains unchanged

Scenario: Mobile search works directly in search input without modal
  Test: manual_test_mobile_inline_search_no_modal
  Given mobile layout is active
  When user types a non-empty query in mobile room filter input
  Then local room/space results update inline
  And no `room_filter_modal` is opened

Scenario: Mobile no-local-results state shows remote options inline
  Test: manual_test_mobile_no_results_shows_remote_options
  Given mobile layout is active
  And local results for query "qwerty-no-hit" are empty
  When query is non-empty
  Then inline empty-state text is shown
  And three remote option buttons are visible: People, Rooms, Spaces

Scenario: Mobile remote people search can be triggered from inline state
  Test: manual_test_mobile_remote_people_search
  Given mobile layout inline no-result state is shown
  When user taps People remote option
  Then app submits `MatrixRequest::SearchDirectory` with kind `People`
  And loading state text is shown while request is in progress

Scenario: Mobile remote rooms/spaces search can be triggered from inline state
  Test: manual_test_mobile_remote_rooms_spaces_search
  Given mobile layout inline no-result state is shown
  When user taps Rooms or Spaces remote option
  Then app submits `MatrixRequest::SearchDirectory` with matching kind
  And returned results are shown inline and selectable

Scenario: Mobile remote option buttons map to exact directory search kinds
  Test: manual_test_mobile_remote_option_kind_mapping
  Given mobile layout inline no-result state is shown
  When user taps People, Rooms, and Spaces remote options
  Then People maps to `RemoteDirectorySearchKind::People`
  And Rooms maps to `RemoteDirectorySearchKind::Rooms`
  And Spaces maps to `RemoteDirectorySearchKind::Spaces`

Scenario: Mobile remote search selection keeps existing destination behavior
  Test: manual_test_mobile_remote_result_selection_behavior
  Given mobile inline remote search returns at least one result
  When user selects a remote user result
  Then app follows existing direct-message open/create behavior
  When user selects a remote room/space result
  Then app follows existing join/open flow

Scenario: Mobile remote search failure shows error and allows retry
  Test: manual_test_mobile_remote_search_failure_retry
  Given mobile inline remote search request fails
  Then inline error text is shown
  And remote option buttons remain available for retry

Scenario: Stale mobile remote results are discarded when query changed
  Test: manual_test_mobile_remote_search_stale_results_ignored
  Given user starts remote search for query "abc"
  And user updates input to query "abcd" and this becomes the active inline query
  When response for "abc" arrives after "abcd" is already active
  Then response for "abc" does not overwrite current inline results for "abcd"

Scenario: Mobile search UI no longer shows TODO copy or redundant icon affordance
  Test: manual_test_mobile_search_ui_cleanup
  Given mobile rooms sidebar is visible
  Then no visible text contains `Search (TODO)`
  And mobile search area no longer shows the redundant search icon affordance

## Out of Scope

- New global search features beyond current room/space/people directory search
- Redesign of desktop modal visuals/copy
- Changes to invite modal search behavior
- Changes to message timeline search feature scope
