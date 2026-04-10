spec: task
name: "Scrollable @mention User List"
inherits: project
tags: [feature, mention, ui, scrolling]
estimate: 1d
---

## Intent

Make the @mention autocomplete popup's user list scrollable so that users can browse all matching members instead of being limited to a hardcoded maximum (10 on desktop, 5 on mobile). In rooms with many members, the current truncation silently hides relevant results. A scrollable list with a fixed maximum height improves discoverability while keeping the popup compact.

## Context

The current `List` widget (in `command_text_input.rs`) is a plain `View` with `flow: Down` — it has no scroll capability. The popup uses `height: Fit` which means it grows unbounded. To prevent this, `DESKTOP_MAX_VISIBLE_ITEMS = 10` and `MOBILE_MAX_VISIBLE_ITEMS = 5` artificially cap the displayed items. The search buffer fetches `max_visible_items * 2` results but only the first half are shown.

Keyboard navigation (ArrowUp/Down) is managed via `keyboard_focus_index` in `CommandTextInput`. The highlight uses Animator states (`highlight.on/off`) on dynamically-created widgets.

## Acceptance Criteria

### Scenario: Popup shows scrollable list when results exceed visible area
- **Given** a room with 50+ members
- **When** the user types `@` (no filter text)
- **Then** the popup shows up to ~10 visible items with a scrollbar
- **And** the user can scroll down to see more results

### Scenario: Keyboard navigation scrolls the list
- **Given** the mention popup is open with 20+ results
- **When** the user presses ArrowDown past the visible area
- **Then** the list scrolls to keep the focused item visible
- **When** the user presses ArrowUp past the top of the visible area
- **Then** the list scrolls back up to show the focused item

### Scenario: Mouse wheel scrolling works
- **Given** the mention popup is open with results exceeding the visible area
- **When** the user scrolls with the mouse wheel/trackpad over the list
- **Then** the list scrolls smoothly

### Scenario: Search results are no longer artificially truncated
- **Given** a room where 30 members match the search query
- **When** the popup displays results
- **Then** all 30 results are accessible via scrolling (not capped at 10)

### Scenario: Popup height is bounded
- **Given** a room with 500+ members
- **When** the popup shows unfiltered results
- **Then** the popup height does not exceed a reasonable maximum (~360px desktop, ~216px mobile)
- **And** the popup does not overflow the screen

## Decisions

- Wrap the `List` widget in a `ScrollYView` named `list_scroll` in the CommandTextInput DSL — do NOT rewrite List's draw logic
- ScrollYView requires a fixed height viewport (NOT `Fit` or `Fit{max}`). DSL default is `height: 200`; Rust dynamically sets the height via `walk.height = Size::Fixed(...)` based on item count and platform
- `DESKTOP_MAX_DISPLAY_ITEMS = 30` / `MOBILE_MAX_DISPLAY_ITEMS = 15` — display limits for the scrollable list. Kept at 30/15 (not 50) for scroll performance since List is non-virtualized
- `DESKTOP_MAX_SCROLL_HEIGHT = 360.0` / `MOBILE_MAX_SCROLL_HEIGHT = 216.0` — platform-specific viewport height caps
- Backend search limit (`max_results`) uses `max_display_items * SEARCH_BUFFER_MULTIPLIER` to ensure enough results
- Old `MAX_VISIBLE_ITEMS` constants removed — no longer needed
- Keyboard auto-scroll derives current scroll offset from `List.area` rendered position (not a tracked field), so it works correctly after manual wheel/trackpad scrolling
- Must access `List.area` via `borrow()`, NOT via `.as_view().area()` — List stores its drawn area in its own `area` field, not in the deref View's area
- `reset_list_scroll()` and `reset_list_scroll_height()` called from `clear_popup()` — NOT from `clear_items()` (which runs on every streaming refresh)
- `reset_list_scroll()` also called at new-search-start in `start_background_search()`
- `clip_y: true` on `list_scroll` to prevent content leaking past rounded corners
- Hot-path `log!` calls removed from KeyDown, Actions, clear_items, add_item for scroll performance

## Boundaries

### Allowed Changes
- `src/shared/command_text_input.rs` — modify List widget to support scrolling, adjust DSL
- `src/shared/mentionable_text_input.rs` — adjust constants, popup DSL overrides, scroll-on-keyboard logic
- No new cargo dependencies

### Forbidden
- Do NOT change the trigger mechanism or search logic
- Do NOT change the mention insertion or tracking behavior
- Do NOT change the highlight/Animator system
- Do NOT use `PortalList` (virtual scrolling) — would require rewriting item instantiation; deferred as future optimization if 30 items is still too slow
- Do NOT run `cargo fmt`

## Completion Criteria

- [ ] User list scrolls vertically when items exceed visible area
- [ ] Keyboard ArrowUp/Down auto-scrolls to keep focused item visible
- [ ] Mouse wheel/trackpad scrolling works on the list
- [ ] Popup height is bounded (does not grow unbounded)
- [ ] More results are accessible than before (not capped at 10)
- [ ] No visual regression: rounded corners, shadow, header still look correct
- [ ] `cargo build` passes
