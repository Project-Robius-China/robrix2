spec: task
name: "Telegram Bot UI Alignment — Phase 5a: Bot-aware Command Discovery"
inherits: project
tags: [bot, ui, telegram-parity, slash-command, discovery]
depends: [task-tg-bot-menu-button, task-tg-bot-command-at-addressing]
estimate: 1d
---

## Intent

当前 `robrix2` 已经能把 `/command@bot` 路由到正确的目标 bot，但命令发现 UI
仍然是“只知道 slash command，不知道这条命令对哪个 bot 合理”。这会导致用户
在 child bot 房间里仍然看到 BotFather 管理命令，或者需要靠试错才能知道
`/listbots` 应该发给 BotFather、`/help` 才是 child bot 更合理的命令。

本任务把现有 slash/menu command 入口升级成 **bot-aware command discovery**：
BotFather 房间只显示管理/编排命令，child bot 房间只显示 child-bot 命令，
没有 bot 上下文的房间不显示 bot command popup。这样命令发现层就能和
现有 `mention/reply-first`、`/command@bot` 路由层保持一致。

## Decisions

- **命令发现上下文**：v1 只区分四种上下文：
  - `ManagementDm`：当前房间是 BotFather DM
  - `ManagementRoom`：当前房间显式绑定了 BotFather
  - `ChildBotRoom`：当前房间绑定的是 child bot（不是 parent/management bot）
  - `None`：没有 bot 上下文
- **BotFather 命令集（DM）**：`ManagementDm` 上下文显示：
  - `/listbots`
  - `/bothelp`
  - `/createbot`
  - `/deletebot`
  - `/schedule`
  - `/schedules`
  - `/unschedule`
- **BotFather 命令集（房间）**：`ManagementRoom` 上下文显示：
  - `ManagementDm` 的全部命令
  - 额外新增 `/allbots`
- **Child bot 命令集**：`ChildBotRoom` 上下文显示静态 child-bot 会话命令：
  - `/new`
  - `/s <name>`
  - `/sessions`
  - `/back`
  - `/delete`
  - `/soul`
  - `/status`
  - `/adaptive`
  - `/reset`
  - `/help`
- **无 bot 上下文**：`None` 上下文不显示 bot command popup，也不显示 BotFather
  menu button。
- **menu button 继承 4a gating**：输入框旁的 BotFather menu button 继续只在
  `ManagementDm` 和 `ManagementRoom` 上下文显示；`ChildBotRoom` 和 `None`
  不显示该按钮。
- **slash popup 过滤时机**：popup 先按命令发现上下文过滤命令集，再按用户当前
  输入前缀过滤，不做先全量再隐藏。
- **静态命令表**：v1 不做动态命令注册协议；命令目录由 `robrix2` 本地静态表维护。
- **显示与发送分离**：bot-aware discovery 只决定“显示哪些命令”，不改变现有
  send-path：
  - Phase 4a 的已分类 BotFather 命令继续按 parent bot 路由
  - Phase 4b 的 `/command@bot` 继续按显式目标 bot 路由
- **显式 `@bot` 优先**：如果用户手工输入 `/command@bot`，发送路径仍以显式
  `@bot` 为准；popup 不需要因为输入了 `@bot` suffix 就切换到别的 bot 的目录。
- **`/allbots` 只在管理房间显示**：因为 `/allbots` 的广播目标依赖当前房间的
  显式 bot bindings，BotFather DM 没有这层上下文，所以 `ManagementDm`
  不显示 `/allbots`。
- **`/schedule` 系列在 DM 和管理房都显示**：它们面向 BotFather，自然语言调度
  目标是当前聊天上下文，所以在 `ManagementDm` 和 `ManagementRoom`
  都可见。
- **命令描述 i18n**：新增命令说明必须补齐中英文 i18n，尤其是：
  - `/allbots`
  - `/schedule`
  - `/schedules`
  - `/unschedule`

## Boundaries

### Allowed Changes
- specs/task-tg-bot-bot-aware-command-discovery.spec.md
- src/shared/mentionable_text_input.rs
- src/room/room_input_bar.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json

### Forbidden
- 不要新增动态命令注册协议
- 不要修改 Octos 后端的命令解析逻辑
- 不要让 child bot 房间显示 BotFather 管理命令
- 不要让无 bot 上下文的普通房间出现 bot command popup
- 不要改变 4a menu button 的位置或视觉样式
- 不要新增 cargo 依赖

## Out of Scope

- `/allbots` 的后端广播执行
- `/schedule`、`/schedules`、`/unschedule` 的后端语义实现
- 根据 `/command@bot` 动态切换 popup 到其他 bot 的命令集
- 动态命令注册协议
- 图形化 cron editor

## Completion Criteria

Scenario: BotFather DM shows management and scheduling commands but not /allbots
  Test: test_management_dm_command_catalog
  Given the command discovery context is `ManagementDm`
  When the user types "/"
  Then the slash popup shows `/listbots`, `/bothelp`, `/createbot`, `/deletebot`
  And the slash popup shows `/schedule`, `/schedules`, `/unschedule`
  And the slash popup does NOT show `/allbots`
  And the slash popup does NOT show `/new`

Scenario: BotFather-bound room shows /allbots in addition to management commands
  Test: test_management_room_command_catalog
  Given the command discovery context is `ManagementRoom`
  When the user types "/"
  Then the slash popup shows `/listbots`, `/bothelp`, `/createbot`, `/deletebot`
  And the slash popup shows `/schedule`, `/schedules`, `/unschedule`
  And the slash popup shows `/allbots`
  And the slash popup does NOT show `/new`

Scenario: Child bot room shows only child-bot commands
  Test: test_child_bot_room_command_catalog
  Given the command discovery context is `ChildBotRoom`
  When the user types "/"
  Then the slash popup shows `/new`, `/sessions`, `/help`, `/status`
  And the slash popup does NOT show `/listbots`
  And the slash popup does NOT show `/createbot`
  And the slash popup does NOT show `/allbots`
  And the slash popup does NOT show `/schedule`

Scenario: Room without bot context shows no bot command popup
  Test: test_no_bot_context_hides_bot_command_popup
  Given the command discovery context is `None`
  When the user types "/"
  Then no bot command popup is shown

Scenario: Menu button remains visible only in BotFather contexts
  Test: test_menu_button_visibility_matches_management_contexts
  Given a `ManagementDm` room
  Then the BotFather menu button is visible
  When the context changes to `ManagementRoom`
  Then the BotFather menu button is visible
  When the context changes to `ChildBotRoom`
  Then the BotFather menu button is hidden
  When the context changes to `None`
  Then the BotFather menu button is hidden

Scenario: Prefix filtering runs after bot-aware context filtering
  Test: test_management_room_prefix_filtering_after_context_filter
  Given the command discovery context is `ManagementRoom`
  And the user types "/sch"
  When the slash popup filters commands
  Then the results contain `/schedule` and `/schedules`
  And the results do NOT contain `/status`
  And the results do NOT contain `/sessions`

Scenario: Explicit /command@bot routing remains independent from popup catalog
  Test: test_explicit_command_at_bot_still_routes_without_popup_support
  Given the command discovery context is `ChildBotRoom`
  And the slash popup does NOT list `/listbots`
  When the user manually submits `/listbots@octosbot`
  Then the send path still treats it as an explicit `@bot` command
  And the popup catalog restrictions do not block submission

Scenario: /allbots discovery is room-only, not DM
  Test: test_allbots_visible_only_in_management_room
  Given the command discovery context is `ManagementDm`
  When the slash popup opens
  Then `/allbots` is absent
  When the context is `ManagementRoom`
  Then `/allbots` is present

Scenario: Command descriptions for new orchestration commands are localized
  Test: test_orchestration_command_descriptions_have_i18n_entries
  Given the slash command metadata table
  When the app resolves descriptions in English and Chinese locales
  Then `/allbots`, `/schedule`, `/schedules`, and `/unschedule` each have non-empty localized descriptions
