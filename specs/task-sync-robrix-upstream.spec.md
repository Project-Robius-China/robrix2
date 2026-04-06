spec: task
name: "Sync Upstream Robrix UI Improvements"
inherits: project
tags: [sync, upstream, ui, performance]
estimate: 2d
---

## Intent

Synchronize UI improvements and bug fixes from the upstream robrix repository (ZhangHanDong/robrix) into robrix2 (Project-Robius-China/robrix2). The two repositories diverged as independent forks; robrix2 added @mention, multi-account, bot, and i18n features while robrix continued receiving UI polish from kevinaboos. This task ports the robrix-only improvements into robrix2 to prevent further drift.

## Context

The Makepad dependency has already been unified to the `text_flow_ellipsis` branch (from `stack_nav_improvements`), and `smooth_scroll_to` API changes have been adapted. The remaining work is porting individual UI improvements.

## Scope

### Phase 1: Performance & Bug Fixes (no Makepad branch dependency)

| Commit | Description | Files |
|--------|-------------|-------|
| `1a00e5c5` | Fix N+1 query in typing notifications | sliding_sync.rs |
| `43cc02ed` | Parallelize fetching typing users' display names | sliding_sync.rs |
| `54a71bee` | Invalidate pending/failed cache on reconnect | media_cache.rs, user_profile_cache.rs |
| `aa1e85a0` | Password visibility toggle on login screen | login_screen.rs, eye_open.svg |

### Phase 2: UI Layout Improvements (depend on text_flow_ellipsis)

| Commit | Description | Files |
|--------|-------------|-------|
| `0d6084df` + `f730d57e` | SpaceLobby 2-line ellipsis layout | space_lobby.rs |
| `2c3bf4f5` | Ellipsis wrapping support | spaces_bar.rs, reply_preview.rs + others |
| `6711e977` + `5dc90c0f` + `ed447cfa` | EditingPane animation overhaul | editing_pane.rs, room_input_bar.rs |
| `0be39eb3` | Relative max height for TextInputs | room_screen.rs, mentionable_text_input.rs, styles.rs |

### Phase 3: Minor Fixes

| Commit | Description | Files |
|--------|-------------|-------|
| `32d68fdf` | Dock minor fixes | main_desktop_ui.rs, room_input_bar.rs |
| `3d6382e8` | Jump-to-bottom button visibility fix | jump_to_bottom_button.rs |
| `77f323ee` | PortalList smooth scroll improvements | rooms_list.rs, spaces_bar.rs |

## Decisions

- Port changes manually by reading the diff from robrix and applying equivalent changes to robrix2, NOT by git cherry-pick (no shared history)
- For files that diverged significantly (sliding_sync.rs, room_screen.rs, app.rs), identify the specific code change and apply it to robrix2's version
- Do NOT overwrite robrix2 files wholesale with robrix versions — robrix2 has newer features (@mention, bot, multi-account) that must be preserved
- The `mentionable_text_input.rs` from robrix (140-line stub) has no value — skip it entirely
- Skip commit `083870c7` ("update") and `aa3ab71b` ("fixed conflict") — these are housekeeping, not functional changes

## Boundaries

### Allowed Changes
- `src/sliding_sync.rs` — typing notification performance fix
- `src/media_cache.rs` — cache invalidation on reconnect
- `src/profile/user_profile_cache.rs` — cache invalidation on reconnect
- `src/login/login_screen.rs` — password visibility toggle
- `resources/icons/eye_open.svg` — new icon for password toggle
- `src/home/space_lobby.rs` — ellipsis layout
- `src/home/spaces_bar.rs` — ellipsis wrapping
- `src/home/editing_pane.rs` — animation overhaul
- `src/room/room_input_bar.rs` — animation + minor fixes
- `src/home/main_desktop_ui.rs` — dock fixes
- `src/shared/jump_to_bottom_button.rs` — visibility fix
- `src/shared/styles.rs` — style constants
- `src/home/rooms_list.rs` — smooth scroll
- `src/room/reply_preview.rs` — ellipsis wrapping

### Forbidden
- Do NOT overwrite robrix2's mentionable_text_input.rs, command_text_input.rs, or app.rs with robrix versions
- Do NOT change the @mention system, multi-account, bot, or i18n code
- Do NOT add new cargo dependencies without explicit approval (except SVG resource files)
- Do NOT run `cargo fmt`
- Do NOT downgrade the Makepad branch from `text_flow_ellipsis`

## Completion Criteria

- [ ] Phase 1: typing N+1 fix applied, cache invalidation ported, password toggle added
- [ ] Phase 2: ellipsis layout improvements applied, EditingPane animation ported
- [ ] Phase 3: dock fixes, jump-to-bottom fix, smooth scroll improvements applied
- [ ] `cargo build` passes after each phase
- [ ] User tests basic functionality after each phase before proceeding
