# Telegram Bot UI Alignment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add bot identity badges on message timeline and `/` slash command autocomplete menu to align Robrix bot UX with Telegram.

**Architecture:** Two independent features that share no code: (1) a `bot_badge` Label widget added to the `Message` DSL template, controlled by `set_visible()` in `populate_message_view()`; (2) slash command autocomplete reusing the existing `CommandTextInput` trigger mechanism with a hardcoded command list. Both features are purely UI — no Matrix SDK or backend changes needed.

**Tech Stack:** Makepad 2.0 `script_mod!` DSL, Rust, existing `CommandTextInput` widget, existing `is_likely_bot_user_id()` detection.

**Spec:** `specs/task-tg-bot-ui-alignment.spec.md`

---

## File Structure

| File | Action | Responsibility |
|------|--------|----------------|
| `src/home/room_screen.rs` | Modify | Add `bot_badge` Label to `Message` DSL template; set visibility in `populate_message_view()` |
| `src/shared/mentionable_text_input.rs` | Modify | Add slash command detection, hardcoded command list, command item template |
| `resources/i18n/en.json` | Modify | Add slash command description strings |
| `resources/i18n/zh-CN.json` | Modify | Add slash command description strings (Chinese) |

---

### Task 1: Add Bot Badge Widget to Message DSL Template

**Files:**
- Modify: `src/home/room_screen.rs:665-682` (username_view in Message template)

This task adds a hidden-by-default `bot_badge` Label inside the `username_view` of the `Message` template. The badge is a small rounded label with blue background and white "bot" text.

- [ ] **Step 1: Add bot_badge to the Message template DSL**

In `src/home/room_screen.rs`, inside the `username_view` (line ~665), add a `bot_badge` Label after the `username` Label. The badge should be hidden by default (`visible: false`).

Find the existing `username_view` block:
```
username_view := View {
    flow: Right,
    width: Fill,
    height: Fit,
    username := Label {
        width: Fill,
        flow: Right, // do not wrap
        ...
        text: ""
    }
}
```

Change `username` width from `Fill` to `Fit` (so the badge can sit next to it), and add the badge:
```
username_view := View {
    flow: Right,
    width: Fill,
    height: Fit,
    align: Align{y: 0.5}
    username := Label {
        width: Fit,
        flow: Right,
        padding: 0,
        margin: Inset{bottom: 9.0, top: 20.0, right: 4.0,}
        max_lines: 1
        text_overflow: Ellipsis
        draw_text +: {
            text_style: USERNAME_TEXT_STYLE {},
            color: (USERNAME_TEXT_COLOR)
        }
        text: ""
    }
    bot_badge := RoundedView {
        visible: false
        width: Fit
        height: 16
        margin: Inset{top: 18.0, right: 6.0}
        padding: Inset{left: 5, right: 5, top: 1, bottom: 1}
        show_bg: true
        draw_bg +: {
            color: (COLOR_ACTIVE_PRIMARY)
            border_radius: 3.0
        }
        bot_badge_label := Label {
            width: Fit
            height: Fit
            draw_text +: {
                text_style: REGULAR_TEXT {font_size: 8.0}
                color: #fff
            }
            text: "bot"
        }
    }
}
```

- [ ] **Step 2: Build to verify DSL compiles**

Run: `cargo build 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 3: Commit**

```bash
git add src/home/room_screen.rs
git commit -m "ui: add bot_badge widget to Message template (hidden by default)"
```

---

### Task 2: Show/Hide Bot Badge in populate_message_view()

**Files:**
- Modify: `src/home/room_screen.rs:7360-7394` (username setting block in populate_message_view)

This task adds bot detection logic that calls `set_visible()` on the badge after setting the username.

- [ ] **Step 1: Add bot detection and badge visibility logic**

In `src/home/room_screen.rs`, after the username is set (line ~7380 `username_label.set_text(cx, &username);`), add bot badge visibility logic. The detection uses the existing `is_likely_bot_user_id()` function (already defined in this file around line 383).

Find the block:
```rust
username_label.set_text(cx, &username);
new_drawn_status.profile_drawn = profile_drawn;
```

Add after it:
```rust
// Show/hide the bot badge based on sender's user ID
let sender_is_bot = is_likely_bot_user_id(event_tl_item.sender());
item.view(cx, ids!(content.username_view.bot_badge)).set_visible(cx, sender_is_bot);
```

Also in the `else` branch (server notice, line ~7383), ensure the badge is hidden:
```rust
item.view(cx, ids!(content.username_view.bot_badge)).set_visible(cx, false);
```

- [ ] **Step 2: Build to verify**

Run: `cargo build 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 3: Manual test**

Run: `cargo run`
- Open a room with a bot (e.g., `@octosbot:127.0.0.1:8128`)
- Bot messages should show a blue "bot" badge next to the username
- Your own messages should NOT show the badge
- Condensed messages (consecutive from same sender) should NOT show the badge (username_view is hidden in CondensedMessage)

- [ ] **Step 4: Commit**

```bash
git add src/home/room_screen.rs
git commit -m "ui: show bot badge on messages from bot users"
```

---

### Task 3: Add i18n Keys for Slash Commands

**Files:**
- Modify: `resources/i18n/en.json`
- Modify: `resources/i18n/zh-CN.json`

- [ ] **Step 1: Add English slash command descriptions**

In `resources/i18n/en.json`, add these keys (in the appropriate alphabetical position):

```json
"slash_command.createbot.description": "Create a new child bot",
"slash_command.deletebot.description": "Delete an existing bot",
"slash_command.listbots.description": "List all available bots",
"slash_command.bothelp.description": "Show bot management help",
"slash_command.header": "Bot Commands",
```

- [ ] **Step 2: Add Chinese slash command descriptions**

In `resources/i18n/zh-CN.json`, add:

```json
"slash_command.createbot.description": "创建一个新的子 Bot",
"slash_command.deletebot.description": "删除一个已有的 Bot",
"slash_command.listbots.description": "列出所有可用的 Bot",
"slash_command.bothelp.description": "显示 Bot 管理帮助",
"slash_command.header": "Bot 命令",
```

- [ ] **Step 3: Build to verify JSON is valid**

Run: `cargo build 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 4: Commit**

```bash
git add resources/i18n/en.json resources/i18n/zh-CN.json
git commit -m "i18n: add slash command description strings"
```

---

### Task 4: Add Slash Command Detection to MentionableTextInput

**Files:**
- Modify: `src/shared/mentionable_text_input.rs`

The existing `MentionableTextInput` uses `CommandTextInput` with trigger `"@"`. We need to detect when the user types `/` at the **start of the input** (position 0 or after a newline) and show a hardcoded list of bot commands.

The approach: instead of modifying `CommandTextInput`'s single-trigger mechanism, we handle `/` detection in `MentionableTextInput`'s own `handle_event` / `handle_actions` by checking the text content. When `/` is detected at position 0, we populate the popup with command items instead of user items.

- [ ] **Step 1: Add slash command data struct and constant list**

In `src/shared/mentionable_text_input.rs`, add a struct and constant list after the imports:

```rust
/// A bot slash command entry for the command autocomplete popup.
pub struct SlashCommand {
    pub command: &'static str,
    pub description_key: &'static str,
}

/// Hardcoded BotFather slash commands.
const SLASH_COMMANDS: &[SlashCommand] = &[
    SlashCommand { command: "/createbot", description_key: "slash_command.createbot.description" },
    SlashCommand { command: "/deletebot", description_key: "slash_command.deletebot.description" },
    SlashCommand { command: "/listbots", description_key: "slash_command.listbots.description" },
    SlashCommand { command: "/bothelp", description_key: "slash_command.bothelp.description" },
];
```

- [ ] **Step 2: Add a slash command list item DSL template**

In the `script_mod!` block, add a template for slash command items (similar to `UserListItem` but simpler — command name + description):

```
mod.widgets.SlashCommandListItem = {
    ..mod.widgets.View
    width: Fill
    height: Fit
    padding: Inset{left: 12, right: 12, top: 8, bottom: 8}
    spacing: 4
    flow: Down
    show_bg: true
    draw_bg +: {
        color: (COLOR_PRIMARY)
    }

    command_label := Label {
        width: Fit
        height: Fit
        draw_text +: {
            text_style: REGULAR_TEXT {font_size: 11.0}
            color: (COLOR_ACTIVE_PRIMARY)
        }
    }
    description_label := Label {
        width: Fill
        height: Fit
        draw_text +: {
            text_style: REGULAR_TEXT {font_size: 9.5}
            color: #888
        }
    }
}
```

Also add a `#[live]` field to `MentionableTextInput` for this template:
```rust
#[live]
slash_command_list_item: Option<LivePtr>,
```

- [ ] **Step 3: Add slash command state tracking**

Add fields to `MentionableTextInput`:
```rust
/// Whether slash command popup is currently active (instead of @mention)
#[rust(false)]
slash_command_active: bool,
```

- [ ] **Step 4: Implement slash command detection and popup**

In the `handle_event` or `handle_actions` method of `MentionableTextInput`, add detection logic:

When the text input content changes:
1. Check if the text starts with `/`
2. If yes and no `@mention` search is active, extract the prefix after `/`
3. Filter `SLASH_COMMANDS` by prefix match
4. Populate the popup list with matching commands
5. Show the popup

When a command item is selected:
1. Replace the input text with the selected command
2. Close the popup

When the text no longer starts with `/`:
1. If `slash_command_active`, hide the popup and reset

This is the most complex step. The implementation should hook into the existing `changed()` action handler where text changes are detected.

- [ ] **Step 5: Build to verify**

Run: `cargo build 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 6: Manual test**

Run: `cargo run`
- Type `/` at the start of the message input
- A popup should appear with 4 commands: `/createbot`, `/deletebot`, `/listbots`, `/bothelp`
- Type `/list` — popup should filter to show only `/listbots`
- Type `/zzz` — popup should show empty or close
- Select a command — it should be inserted into the input
- Type a normal message (no `/`) — no popup should appear

- [ ] **Step 7: Commit**

```bash
git add src/shared/mentionable_text_input.rs
git commit -m "feat: add slash command autocomplete for bot commands"
```

---

### Task 5: Final Integration Test and Cleanup

**Files:**
- All modified files

- [ ] **Step 1: Full build**

Run: `cargo build 2>&1 | tail -5`
Expected: `Finished` with no errors

- [ ] **Step 2: End-to-end manual test**

Run: `cargo run`

Verify all scenarios from the spec:
1. Bot messages show blue "bot" badge next to username
2. User messages do NOT show bot badge
3. Consecutive bot messages (condensed view) do NOT show badge
4. Typing `/` shows command popup with 4 commands
5. Filtering works (type `/list` to narrow)
6. Selecting a command inserts it
7. Non-matching prefix (`/zzz`) shows empty/closes
8. Normal typing (no `/`) does not trigger popup

- [ ] **Step 3: Verify against spec**

Run: `agent-spec parse specs/task-tg-bot-ui-alignment.spec.md`
Review each scenario and confirm it passes manually.

- [ ] **Step 4: Final commit if any cleanup needed**

```bash
git add -A
git commit -m "feat: telegram bot UI alignment — bot badge and slash commands"
```
