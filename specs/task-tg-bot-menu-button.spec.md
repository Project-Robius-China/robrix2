spec: task
name: "Telegram Bot UI Alignment — Phase 4a: Menu Button & Pure Command Send-on-Select"
inherits: project
tags: [bot, ui, telegram-parity, menu-button, slash-command]
depends: [task-tg-bot-mention-reply-first]
estimate: 1d
---

## Intent

对齐 Telegram 的 bot 命令入口交互：输入框旁引入一个显式的 "bot menu
button"，以及修正当前"选中命令 → 插入文本"的行为，让**纯命令**（不需要参
数的命令，比如 `/listbots`、`/bothelp`）在用户选中后立即发送，只有**参数
命令**（比如 `/createbot name desc`）才继续沿用"选中即插入文本"的行为。

当前的 `/bot` 文本快捷方式是 power user 级别的入口，不符合普通用户"点击
bot 图标看它能做什么"的心智模型；而选中纯命令后仍然要手动按 Enter 发送
也多了一步无谓操作。本任务消除这两个摩擦点。

## Decisions

- **Menu button 位置**：在 `room_input_bar.rs` 的 `input_row` 中，紧邻
  `mentionable_text_input`（emoji/translate 按钮那一排），新增一个
  `bot_menu_button`。
- **Menu button 显示条件（BotFather-only）**：menu button 只在**当前房间
  绑定的 bot 是 parent/management bot（即 BotFather）**时显示。具体条件：
  - `app_service_enabled = true` 且
  - `bound_bot_user_id.is_some()` 且
  - 当前 `bound_bot_user_id` 就是 parent/management bot——实现上通过
    `bound_bot_user_id == resolved_parent_bot_user_id`（当 parent 解析返
    回自身时）或等价的"is_management_bot_room"判断来决定。
  其他情况——包括绑定的是 child bot（如 `alexbot`、`bobbot`）的房间、未
  绑定 bot 的房间、AppService 未启用的房间——menu button 一律隐藏。
- **为什么 BotFather-only**：Phase 4a 的命令集合（`/listbots`、`/bothelp`、
  `/createbot`、`/deletebot`）全部是 **BotFather 的管理命令**，child bot
  不识别这些命令。如果在 child bot 房间里显示 menu button，会导致管理命
  令被发给错的 bot，是错误行为。用户要管理 bot，需要导航到 BotFather DM
  去操作。Phase 4a+ 的增量任务可以在 child bot 房间加 child-bot-specific
  命令菜单（需要一个动态命令注册协议，不在本 spec 范围）。
- **Menu button 图标**：使用 "slash command" 图标（如 `/` 或 bot 头像）——
  具体图标由实现时从现有 `resources/icons/` 中选择，避免引入新资源。
- **Menu button 行为**：点击时打开与输入框键入 `/` 相同的 slash command
  popup（复用 `CommandTextInput` 的现有机制），并自动把 `/` 插入到输入
  框，触发 popup 显示。不引入独立的命令面板 UI。
- **纯命令 vs 参数命令**：扩展 `SlashCommand` struct 添加 `needs_args:
  bool` 字段。初始分类如下（均为 BotFather 管理命令）：
  - **纯命令**（`needs_args: false`，选中即发送）：`/listbots`、`/bothelp`
  - **参数命令**（`needs_args: true`，选中插入文本等用户补充）：`/createbot`、`/deletebot`
- **与 Phase 4b 的 precedence（关键）**：Phase 4b 定义了"multi-bot room
  里裸 `/command` 不 auto-resolve，继续 room-first"的规则。**Phase 4a 分
  类的 BotFather 管理命令是这条规则的显式例外**：无论房间是 DM、single
  bot、multi-bot，也无论用户是通过 menu button 选中还是手打命令按 Enter
  提交，只要命令被 `SLASH_COMMANDS` 分类为已知的 BotFather 管理命令，就
  **总是**以 `target_user_id = parent_bot_user_id` + `explicit_room =
  false` 显式路由到 parent/management bot。Phase 4b 的 bare-command 规
  则只对**不在 `SLASH_COMMANDS` 里的未知命令**生效。
- **纯命令发送行为**：当用户从 popup 选中一个纯命令时，`MentionableTextInput`
  直接调用与 Cmd+Enter 相同的"提交消息"路径，把命令作为 `TextInputAction::Returned`
  派发出去。popup 关闭，输入框清空，无需用户再按 Enter。
- **纯命令路由**：纯命令是**管理意图消息**，必须显式路由到 bot，不能走
  mention-first 的 room-first 默认。发送前，send-path 显式设置
  `target_user_id = bound_bot_user_id`（或 `resolved_parent_bot_user_id`）
  并设置 `explicit_room = false`，让 Octos 侧的 `route_by_explicit_target`
  捕获它。这跟 Phase 3 普通消息的 room-first 行为是**per-message override**
  关系，不修改 `ExplicitOverride` 持久化状态。
- **参数命令插入行为**：保持现有行为——命令文本 + 一个尾随空格被插入到
  输入框，popup 关闭，光标停在空格后等用户补充参数。
- **`/bot` 文本快捷方式**：保留给 power user（现在的实现不动），不作为主
  入口。
- **DM 行为**：menu button 在 DM 里也显示，交互与多人房间一致。
- **多 bot room**：menu button 不做 bot 选择——如果房间有多个 bot，用户
  通过 `/command@bot` 语法（见 Phase 4b）消歧义。本 spec 不涉及多 bot
  选择 UI。
- **i18n**：按钮 tooltip/accessibility label 新增 i18n key
  `room_input_bar.bot_menu_button.tooltip`，中英文都要。

## Boundaries

### Allowed Changes
- src/room/room_input_bar.rs
- src/shared/mentionable_text_input.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json

Minor icon references are allowed if an existing icon fits; do NOT add new
SVG assets to `resources/icons/` as part of this task.

### Forbidden
- 不要修改 `CommandTextInput` 上游 Makepad widget
- 不要修改 Octos 后端
- 不要移除 `/bot` 文本快捷方式（保留给 power user）
- 不要引入 "reply keyboard"（参数命令的 keyboard replacement UI）——那是
  TG 的另一个概念，不在本 spec 范围
- 不要在本 spec 中实现 `/command@bot` 语法——交给 Phase 4b
- 不要添加新的 cargo 依赖
- 不要在 menu button 的 popup 里添加除 slash commands 之外的任何其他
  条目（比如"最近使用"、"收藏"）

## Out of Scope

- `/command@bot` 显式寻址（Phase 4b 独立 spec）
- Bot message action buttons / inline keyboard（Phase 4c 独立 spec）
- 动态命令注册协议（长期方向，需要 Matrix-side 设计）
- 多 bot room 选择器 UI

## Completion Criteria

Scenario: Bot menu button visible in BotFather-bound room with AppService enabled
  Test: test_bot_menu_button_visible_in_botfather_room
  Given a room with `bound_bot_user_id = "@octosbot:127.0.0.1:8128"`
  And `@octosbot:127.0.0.1:8128` is the parent/management bot (i.e.,
    `resolved_parent_bot_user_id == bound_bot_user_id`)
  And `app_service_enabled = true`
  When the user enters the room
  Then the `bot_menu_button` is visible in the input row
  And its tooltip reads "Bot commands" in English locale

Scenario: Bot menu button hidden in child bot room
  Test: test_bot_menu_button_hidden_in_child_bot_room
  Given a room with `bound_bot_user_id = "@octosbot_weather:127.0.0.1:8128"`
  And `@octosbot_weather` is a child bot managed by BotFather (i.e.,
    `resolved_parent_bot_user_id = "@octosbot:127.0.0.1:8128"`,
    different from `bound_bot_user_id`)
  And `app_service_enabled = true`
  When the user enters the room
  Then the `bot_menu_button` is not visible
  And no popup error is shown
  And the user can still chat with `@octosbot_weather` normally

Scenario: Bot menu button hidden without bound bot
  Test: test_bot_menu_button_hidden_without_bound_bot
  Given a room with no `bound_bot_user_id`
  And `app_service_enabled = true`
  When the user enters the room
  Then the `bot_menu_button` is not visible

Scenario: Bot menu button hidden when AppService disabled
  Test: test_bot_menu_button_hidden_without_app_service
  Given a room with `bound_bot_user_id` set to the parent/management bot
  And `app_service_enabled = false`
  When the user enters the room
  Then the `bot_menu_button` is not visible

Scenario: Clicking bot menu button opens slash command popup
  Test: test_bot_menu_button_click_opens_slash_popup
  Given the `bot_menu_button` is visible and the input is empty
  When the user clicks the `bot_menu_button`
  Then the slash command popup is visible
  And the input text becomes "/"
  And the cursor is positioned after the slash

Scenario: Pure command sends immediately on select
  Test: test_pure_command_sends_on_select
  Given the slash command popup is showing "/listbots" and "/bothelp"
  And the user's input contains only "/"
  When the user selects "/listbots" from the popup
  Then a message with body "/listbots" is submitted immediately
  And the input is cleared
  And the slash command popup is closed
  And no `Enter` key press was required

Scenario: Parameterized command inserts text and waits
  Test: test_parameterized_command_inserts_text
  Given the slash command popup is showing "/createbot"
  And the user's input contains only "/"
  When the user selects "/createbot" from the popup
  Then the input text becomes "/createbot " (with trailing space)
  And the cursor is positioned at the end
  And the slash command popup is closed
  And no message is submitted
  And the user can type arguments after the command

Scenario: SlashCommand needs_args classification covers all known commands
  Test: test_slash_command_classification_is_complete
  Given the `SLASH_COMMANDS` constant
  When each command is inspected
  Then `/listbots` has `needs_args = false`
  And `/bothelp` has `needs_args = false`
  And `/createbot` has `needs_args = true`
  And `/deletebot` has `needs_args = true`

Scenario: Existing /bot text shortcut still works
  Test: test_bot_text_shortcut_still_opens_panel
  Given the user types "/bot" into the input
  And `app_service_enabled = true`
  When the user presses Enter
  Then a `MessageAction::ToggleAppServiceActions` action is emitted
  And the input is cleared

Scenario: Typing slash directly still opens the popup with classification
  Test: test_typing_slash_still_uses_classification
  Given the input is empty
  When the user types "/"
  Then the slash command popup is shown
  And selecting "/listbots" submits immediately (not just inserts)

Scenario: Pure command submission explicitly targets the parent/management bot
  Test: test_pure_command_submission_targets_parent_bot
  Given a BotFather-bound multi-member room (non-DM)
  And `bound_bot_user_id = "@octosbot:127.0.0.1:8128"` (the parent/management bot)
  And `resolved_parent_bot_user_id = "@octosbot:127.0.0.1:8128"`
  And no `ExplicitOverride` is set
  When the user submits "/listbots" via send-on-select
  Then the outgoing message has `target_user_id = "@octosbot:127.0.0.1:8128"`
  And the outgoing message has `explicit_room = false`
  And the message body is "/listbots"
  And the `ExplicitOverride` persistent state remains `None`

Scenario: Pure command in DM still targets the parent/management bot
  Test: test_pure_command_in_dm_targets_parent_bot
  Given a DM with `bound_bot_user_id = "@octosbot:127.0.0.1:8128"` (the parent bot)
  And `resolved_parent_bot_user_id = "@octosbot:127.0.0.1:8128"`
  When the user submits "/bothelp" via send-on-select
  Then the outgoing message has `target_user_id = "@octosbot:127.0.0.1:8128"`
  And the outgoing message has `explicit_room = false`
  And the message body is "/bothelp"

Scenario: Typed slash command (not via menu) also targets the parent bot when classified as pure
  Test: test_typed_pure_command_also_targets_parent_bot
  Given a BotFather-bound multi-member room
  And `bound_bot_user_id = "@octosbot:127.0.0.1:8128"` (the parent/management bot)
  When the user types "/listbots" into the input and presses Enter
  Then the outgoing message has `target_user_id = "@octosbot:127.0.0.1:8128"`
  And the outgoing message has `explicit_room = false`
  And the behavior matches the menu-selected path (no drift between menu and keyboard entry)
  And this precedence holds even in multi-bot rooms (Phase 4b bare-command
    "no auto-resolve" rule does NOT apply to Phase 4a classified commands)

Scenario: Bot menu button click in thread returns error popup
  Test: test_bot_menu_button_click_in_thread_rejected
  Given the user is in a thread view (not the main timeline)
  And the `bot_menu_button` would otherwise be visible
  When the user clicks the `bot_menu_button`
  Then the click is rejected with an error message
  And no slash command popup is opened
  And a popup notification is shown with text "Bot commands are only supported in the main room timeline"
  And the popup kind is `PopupKind::Warning`

Scenario: Unknown slash command is not classified and falls through
  Test: test_unknown_slash_command_falls_through
  Given the user types "/unknowncmd" into the input
  When the user presses Enter
  Then the message is sent as a plain text message with body "/unknowncmd"
  And no classification lookup short-circuits the send
  And no send-on-select behavior is triggered (because the command is not in `SLASH_COMMANDS`)

Scenario: Parameterized command insert leaves trailing space for user input
  Test: test_parameterized_command_cursor_after_space
  Given the slash command popup is showing "/createbot"
  And the user has typed "/"
  When the user selects "/createbot" from the popup
  Then the input text ends with a single trailing space
  And no extra whitespace is added
  And the input does NOT contain "/createbot/createbot" (no duplication)
