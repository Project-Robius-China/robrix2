spec: task
name: "Telegram Bot UI Alignment — Phase 1: Bot Badge & Message Identity"
inherits: project
tags: [bot, ui, telegram-parity]
depends: []
estimate: 1d
---

## Intent

在 Robrix 的消息流中对齐 Telegram 的 bot 对话体验。当前 Robrix 的 bot 消息与普通用户消息在视觉上完全相同，用户无法快速识别哪些消息来自 bot。Telegram 在 bot 消息的用户名旁显示一个蓝色 "bot" 标签，这是 bot 对话体验的基础视觉元素。

本 spec 覆盖 Phase 1（Bot 身份标识）和 Phase 2（`/` 命令菜单），这两项是让 bot 对话"感觉像 Telegram"的最小可行改动。

## Decisions

- Bot 徽章样式: 在消息 username Label 右侧添加一个小号 RoundedView，内含 "bot" 文本，背景色使用 `COLOR_ACTIVE_PRIMARY`，文字白色，圆角 3px，高度 16px
- Bot 检测逻辑: 复用现有 `is_likely_bot_user_id()` 和 `is_likely_bot_member()` 函数（`room_screen.rs`），不引入新的检测机制
- Bot 徽章数据传递: 在 `populate_message_view()` 中检测发送者是否为 bot，通过 `set_visible()` 控制徽章显示/隐藏
- `/` 命令菜单: 复用现有 `CommandTextInput` 组件（已用于 `@mention`），添加 `/` 作为第二个 trigger character
- 命令来源: 硬编码一组 BotFather 基础命令（`/createbot`, `/deletebot`, `/listbots`, `/bothelp`），不实现动态命令注册
- 命令列表 UI: 复用 `@mention` 弹出列表的样式，每项显示命令名和描述

## Boundaries

### Allowed Changes
- src/home/room_screen.rs — 消息模板添加 bot 徽章 widget，`populate_message_view()` 添加 bot 检测逻辑
- src/shared/mentionable_text_input.rs — 添加 `/` trigger 和命令列表数据源
- src/room/room_input_bar.rs — 必要时调整输入栏与命令补全的集成
- resources/i18n/en.json — 添加命令描述的 i18n 键
- resources/i18n/zh-CN.json — 添加命令描述的 i18n 键

### Forbidden
- 不要修改 bot 检测逻辑（`is_likely_bot_user_id` / `is_likely_bot_member`）的判断规则
- 不要添加新的 cargo 依赖
- 不要修改消息的整体布局结构（avatar 位置、消息气泡宽度等）
- 不要实现 Inline Keyboard 按钮（Phase 3 范围）
- 不要实现 Reply Keyboard（Phase 3 范围）
- 不要实现动态命令注册协议（超出当前范围）

## Out of Scope

- Inline Keyboard 按钮（消息下方的可点击按钮）
- Reply Keyboard（替换用户键盘的预设选项）
- Bot Profile 页面（专属的 bot 信息/描述页面）
- Menu Button（输入框旁的 bot 菜单按钮）
- Bot 欢迎消息 / "What can this bot do?" 描述框
- 动态命令注册（bot 向客户端声明自己支持的命令）
- Bot-to-Bot 通信

## Completion Criteria

Scenario: Bot badge visible on bot messages
  Test: test_bot_badge_visible
  Given a room with user "alice" and bot "@octosbot:127.0.0.1:8128"
  When the bot sends a message to the room
  Then the bot's message displays username "BotFather" with a "bot" badge label visible next to it
  And the badge has a distinct background color (not the same as username text)

Scenario: Bot badge hidden on regular user messages
  Test: test_bot_badge_hidden_for_users
  Given a room with user "alice" and bot "@octosbot:127.0.0.1:8128"
  When user "alice" sends a message
  Then "alice"'s message does NOT display a "bot" badge label

Scenario: Bot badge visible on condensed messages
  Test: test_bot_badge_condensed
  Given a bot sends multiple consecutive messages
  When the messages are rendered as CondensedMessage (no avatar/profile)
  Then the condensed messages do NOT show the bot badge (since username row is hidden)

Scenario: Bot badge detects known bot patterns
  Test: test_bot_badge_detection
  Given a user with ID "@mybot:server" or "@octosbot_translator:server"
  When their message is rendered
  Then the bot badge is visible because `is_likely_bot_user_id()` returns true

Scenario: Slash command menu appears on "/" input
  Test: test_slash_command_trigger
  Given the user is in a room with bot features enabled
  When the user types "/" in the message input field
  Then a popup list appears showing available bot commands with descriptions

Scenario: Slash command menu absent without "/"
  Test: test_slash_command_no_trigger
  Given the user is in a room
  When the user types a regular message without "/"
  Then no command popup list appears

Scenario: Selecting a command from the menu inserts it
  Test: test_slash_command_selection
  Given the slash command popup is visible
  When the user selects "/listbots" from the list
  Then "/listbots" is inserted into the message input field
  And the popup closes

Scenario: Slash command menu shows BotFather commands with descriptions
  Test: test_slash_command_list_content
  Given the slash command popup is visible
  Then the list contains at least: "/createbot", "/deletebot", "/listbots", "/bothelp"
  And each command entry displays both a command name and a description label in the popup list style matching the @mention popup

Scenario: Slash command menu handles empty prefix gracefully
  Test: test_slash_command_empty_prefix
  Given the user is in a room
  When the user types "/" followed by a non-matching string "zzzznotacommand"
  Then the command popup shows an empty list or closes
  And no error is displayed

Scenario: Bot badge not shown for user with "bot" in display name but normal user ID
  Test: test_bot_badge_false_positive
  Given a regular user with ID "@roberto:server" and display name "Roberto"
  When their message is rendered
  Then the bot badge is NOT visible because `is_likely_bot_user_id()` returns false for "roberto"
