spec: task
name: "Telegram Bot UI Alignment — Phase 2: Explicit Target Model (SUPERSEDED)"
inherits: project
tags: [bot, ui, telegram-parity, target-model, superseded]
depends: [task-tg-bot-ui-alignment]
estimate: 3d
---

> **⚠️ SUPERSEDED (2026-04-13).** This spec represented the Phase 2 design based
> on a persistent target chip UI for explicit bot targeting. The product
> direction was revised on 2026-04-12 to a **mention/reply-first** model which
> hides the target chip entirely and treats bot-bound multi-member rooms as
> room-first by default.
>
> The authoritative active spec is
> [`task-tg-bot-mention-reply-first.spec.md`](./task-tg-bot-mention-reply-first.spec.md)
> (Phase 3). This Phase 2 document is kept for historical context only —
> do NOT use it as the basis for new implementation work. Sections about the
> target chip, the chip switching menu, `RoomDefault` vs `ExplicitBot` visual
> distinction, and persisted `ExplicitOverride` restore are all obsolete.
>
> What remains valid from Phase 2 and was carried forward into Phase 3:
> - The `ExplicitOverride` / `ResolvedTarget` two-layer type model
> - The `is_known_or_likely_bot()` helper and context fields on `RoomScreenProps`
> - The reply-to-human-must-not-trigger-bot-targeting fix
>
> Everything else in this document is historical.

## Intent

将 Robrix2 的 bot 消息路由从"implicit room-bound routing"转向"explicit bot targeting"，对标 Telegram 的显式 bot 交互体验。当前用户无法看到消息会发给谁——路由由隐藏的 `target_user_id` fallback 静默决定（`room_input_bar.rs:861`）。本任务引入两层 target 状态模型（持久化用户意图 + 运行时推导），在输入框上方添加 target chip 显示当前消息目标，并修复 reply-to-human 误触发 bot targeting 的 bug。

架构原则：Telegram-style bot UX on top of Matrix semantics。底层保留 Matrix 的 room/user/message 模型，只在客户端 UX 层补上显式、低摩擦的 bot 交互。

## Decisions

- 状态模型: 两层分离——持久化层 `ExplicitOverride { None, Bot(OwnedUserId), Room }` 存入 `RoomInputBarState`，运行时层 `ResolvedTarget { NoTarget, RoomDefault(OwnedUserId), ExplicitBot(OwnedUserId), ExplicitRoom, ReplyBot(OwnedUserId) }` 实时推导
- 持久化策略: 只持久化 `ExplicitOverride`。`RoomDefault` 从 `bound_bot_user_id` 推导，`ReplyBot` 从 `replying_to` + bot 判定推导，均不独立持久化
- Resolve 优先级: (1) `replying_to` 且被回复者是 bot → `ReplyBot`；(2) `ExplicitOverride::Bot` → `ExplicitBot`，`::Room` → `ExplicitRoom`；(3) `ExplicitOverride::None` + 有 `bound_bot_user_id` → `RoomDefault`；(4) 否则 → `NoTarget`
- Bot 判定: 新建 `is_known_or_likely_bot(user_id: &UserId, resolved_parent_bot_user_id: Option<&UserId>, known_bot_user_ids: &[OwnedUserId]) -> bool`，三条检测路径：(1) `known_bot_user_ids` 精确匹配；(2) `resolved_parent_bot_user_id` 精确匹配；(3) `is_likely_bot_user_id()` 启发式。函数只吃预计算后的最小上下文，不依赖 `BotSettingsState` 或 `current_user_id`
- Target chip UI: 参考 `ReplyingPreview`（`reply_preview.rs:77-123`）的模式，在输入框上方添加 `TargetIndicator` widget
- Chip × 行为: 清除 `ExplicitOverride` 回到 `None`，resolve 自动回退到 `RoomDefault`（有绑定 bot 时）或 `NoTarget`（无 bot 时）
- Reply/Target 所有权: 取消 reply 清掉 `ReplyBot`（真相来源是 `replying_to`）；清掉 target chip 保留 Matrix reply（reply 是协议层，target 是 UX 层）
- 混合场景: `ExplicitOverride::Bot` + reply-to-human → resolve 为 `ExplicitBot`，同时保留 Matrix reply 关系，两者独立
- 切换入口: 点 chip 弹出切换菜单（可选 bot 或 room），reply bot 自动 resolve 为 `ReplyBot`，chip × 清除 override
- Chip 文案规则: chip 中 bot 标识统一使用 display name（如 "BotFather"），不使用 MXID 或 localpart。具体格式：`RoomDefault` → "Default: {display_name}"（subdued style）；`ExplicitBot` → "To {display_name}"；`ExplicitRoom` → "To room"；`ReplyBot` → "Reply → {display_name}"。display name 取自 room member 的 `display_name()`，如果为空则 fallback 到 localpart

## Boundaries

### Allowed Changes
- src/room/room_input_bar.rs
- src/home/room_screen.rs
- src/room/reply_preview.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json

What to change in each file: room_input_bar.rs gets ExplicitOverride enum, ResolvedTarget enum, refactored resolve_target_user_id(), TargetIndicator widget DSL, save/restore logic. room_screen.rs gets is_known_or_likely_bot() new function, expanded RoomScreenProps with resolved_parent_bot_user_id and known_bot_user_ids fields, bot detection context passing. reply_preview.rs may need layout adjustments for TargetIndicator. en.json and zh-CN.json get target chip display strings.

### Forbidden
- 不要修改 `detected_bot_binding_for_members()` 的职责边界——它是房间级绑定发现，不是通用 bot 判定
- 不要修改 `ReplyingPreview` 的现有显示/取消/恢复状态机逻辑（`show_replying_to()`、`clear_replying_to()`、`on_editing_pane_hidden()`）
- 不要修改 `DEFAULT_BOTFATHER_LOCALPART` 的全局默认值
- 不要添加新的 cargo 依赖
- 不要实现动态命令注册（独立未来方向，需 Matrix-side 协议设计）
- 不要实现 `/command@bot` 语法解析（P2 范围）
- 不要实现多 bot room 切换菜单（P2 范围，当前只需支持单个绑定 bot）

## Out of Scope

- Menu button 替代 `/bot`（P2）
- 命令行为分类（send-on-select vs insert）（P2）
- `/command@bot` 显式寻址语法（P2）
- 多 bot room 场景下的 target 切换（P2）
- 动态命令注册协议设计

## Completion Criteria

Scenario: Target chip shows RoomDefault in bot-bound room
  Test: test_target_chip_room_default
  Given a room with bound bot "@octosbot:127.0.0.1:8128"
  And the user has no `ExplicitOverride` set for this room
  When the user enters the room
  Then the target chip displays "Default: {display_name}" in subdued style where display_name is the bot's room member display name
  And the `ResolvedTarget` is `RoomDefault` with the bound bot's user ID

Scenario: Target chip hidden in normal room
  Test: test_target_chip_hidden_no_bot
  Given a room with no bound bot
  When the user enters the room
  Then no target chip is displayed
  And the `ResolvedTarget` is `NoTarget`

Scenario: User switches to ExplicitBot via chip menu
  Test: test_explicit_bot_via_chip_menu
  Given a room with bound bot "@octosbot:127.0.0.1:8128"
  And the target chip shows "Default: {display_name}" where display_name is the bot's room member display name
  When the user clicks the chip and selects the bound bot from the menu
  Then the target chip displays "To {display_name}" in normal style
  And the `ExplicitOverride` is `Bot` with the bound bot's user ID
  And a message sent from this state has `target_user_id` set to the bot

Scenario: User switches to ExplicitRoom via chip menu
  Test: test_explicit_room_via_chip_menu
  Given a room with bound bot "@octosbot:127.0.0.1:8128"
  And the target chip shows "Default: {display_name}" where display_name is the bot's room member display name
  When the user clicks the chip and selects "To room" from the menu
  Then the target chip displays "To room"
  And the `ExplicitOverride` is `Room`
  And a message sent from this state has `target_user_id` set to `None`

Scenario: Chip dismiss clears ExplicitOverride to RoomDefault
  Test: test_chip_dismiss_returns_to_room_default
  Given a room with bound bot "@octosbot:127.0.0.1:8128"
  And the `ExplicitOverride` is `Bot` with the bound bot's user ID
  When the user clicks the × button on the target chip
  Then the `ExplicitOverride` resets to `None`
  And the target chip displays "Default: {display_name}" in subdued style where display_name is the bot's room member display name

Scenario: Reply-to-bot triggers ReplyBot target
  Test: test_reply_to_bot_triggers_reply_bot
  Given a room with a message from bot "@octosbot:127.0.0.1:8128"
  When the user clicks reply on the bot's message
  Then the `ResolvedTarget` is `ReplyBot` with the bot's user ID
  And the target chip displays "Reply → {display_name}"
  And the reply preview shows the bot's message content

Scenario: Reply-to-human does NOT trigger bot targeting
  Test: test_reply_to_human_no_bot_targeting
  Given a room with bound bot "@octosbot:127.0.0.1:8128"
  And no `ExplicitOverride` set (defaults to `None`)
  And a message from regular user "@alice:127.0.0.1:8128"
  When the user clicks reply on alice's message
  Then the `ResolvedTarget` is `RoomDefault` with the bound bot's user ID (NOT `ReplyBot`)
  And the message is sent with Matrix reply relation to alice's event
  And `target_user_id` is set to the bound bot (via RoomDefault fallback)
  But `target_user_id` is NOT set to alice's user ID

Scenario: Cancel reply clears ReplyBot but preserves ExplicitOverride
  Test: test_cancel_reply_clears_reply_bot
  Given a room where the user has `ExplicitOverride::Bot` set to "@octosbot:127.0.0.1:8128"
  And the user is replying to a bot message (ResolvedTarget is ReplyBot)
  When the user cancels the reply via the reply preview's cancel button
  Then the `ResolvedTarget` changes from `ReplyBot` to `ExplicitBot`
  And the target chip changes from "Reply → {display_name}" to "To {display_name}"
  And the reply preview is hidden

Scenario: ExplicitBot persists with reply-to-human
  Test: test_explicit_bot_with_reply_to_human
  Given a room with `ExplicitOverride::Bot` set to "@octosbot:127.0.0.1:8128"
  And the user replies to a message from regular user "@alice:127.0.0.1:8128"
  When the user sends the message
  Then the message has `target_user_id` set to the bot's user ID
  And the message has Matrix reply relation to alice's event
  And the target chip continues to show "To {display_name}"

Scenario: ExplicitOverride persists across room navigation
  Test: test_explicit_override_persists_navigation
  Given a room with `ExplicitOverride::Room` set
  When the user navigates away from the room and returns
  Then the `ExplicitOverride` is still `Room`
  And the target chip displays "To room"

Scenario: ReplyBot restores when replying_to restores
  Test: test_reply_bot_restores_with_replying_to
  Given a room where the user is replying to a bot message
  When the user navigates away and returns
  Then `replying_to` is restored from `RoomInputBarState`
  And the `ResolvedTarget` re-resolves to `ReplyBot` with the bot's user ID
  And the target chip displays "Reply → {display_name}"

Scenario: ReplyBot overrides ExplicitRoom when replying to bot
  Test: test_reply_bot_overrides_explicit_room
  Given a room with bound bot "@octosbot:127.0.0.1:8128"
  And the user has `ExplicitOverride::Room` set
  And the target chip displays "To room"
  When the user clicks reply on the bot's message
  Then the `ResolvedTarget` is `ReplyBot` with the bot's user ID
  And the target chip displays "Reply → {display_name}"
  But the `ExplicitOverride` remains `Room`
  And when the user cancels the reply, the target reverts to `ExplicitRoom`

Scenario: Chip dismiss clears ExplicitRoom back to RoomDefault
  Test: test_chip_dismiss_explicit_room_to_room_default
  Given a room with bound bot "@octosbot:127.0.0.1:8128"
  And the `ExplicitOverride` is `Room`
  And the target chip displays "To room"
  When the user clicks the × button on the target chip
  Then the `ExplicitOverride` resets to `None`
  And the `ResolvedTarget` is `RoomDefault` with the bound bot's user ID
  And the target chip displays "Default: {display_name}" in subdued style

Scenario: Chip dismiss in room without bound bot returns NoTarget
  Test: test_chip_dismiss_no_bound_bot
  Given a room with no bound bot
  And the user has somehow set `ExplicitOverride::Bot` with a user ID
  When the user clicks the × button on the target chip
  Then the `ExplicitOverride` resets to `None`
  And the `ResolvedTarget` is `NoTarget`
  And no target chip is displayed

Scenario: is_known_or_likely_bot detects configured parent bot via resolved_parent_bot_user_id
  Test: test_bot_detection_configured_parent
  Given `resolved_parent_bot_user_id` is "@octosbot:127.0.0.1:8128"
  And `known_bot_user_ids` is empty
  When `is_known_or_likely_bot` is called with user ID "@octosbot:127.0.0.1:8128"
  Then the function returns `true` via `resolved_parent_bot_user_id` matching

Scenario: is_known_or_likely_bot detects bot by heuristic when not in known list
  Test: test_bot_detection_heuristic_fallback
  Given `resolved_parent_bot_user_id` is `None`
  And `known_bot_user_ids` is empty
  When `is_known_or_likely_bot` is called with user ID "@myservice_bot:other.server"
  Then the function returns `true` via localpart heuristic (ends with "_bot")

Scenario: is_known_or_likely_bot detects child bots via known list
  Test: test_bot_detection_child_bot
  Given `known_bot_user_ids` containing "@octosbot_weather:127.0.0.1:8128"
  And `resolved_parent_bot_user_id` is `None`
  When `is_known_or_likely_bot` is called with user ID "@octosbot_weather:127.0.0.1:8128"
  Then the function returns `true` via `known_bot_user_ids` matching

Scenario: is_known_or_likely_bot rejects normal users
  Test: test_bot_detection_rejects_normal_user
  Given `resolved_parent_bot_user_id` is `None`
  And `known_bot_user_ids` is empty
  When `is_known_or_likely_bot` is called with user ID "@alice:127.0.0.1:8128"
  Then the function returns `false`
