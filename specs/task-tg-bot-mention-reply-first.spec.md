spec: task
name: "Telegram Bot UI Alignment — Phase 3: Mention/Reply-First Targeting"
inherits: project
tags: [bot, ui, telegram-parity, mention, reply]
depends: [task-tg-bot-explicit-targeting, task-tg-bot-explicit-room-no-fallback]
estimate: 1d
---

## Intent

将 Robrix2 的群聊 bot 交互进一步收敛到 Telegram 风格：默认普通消息发给房间，bot 交互优先通过 `@bot` mention 和 reply-to-bot 完成，而不是依赖常驻 target chip 或手工切换菜单。当前 Phase 2/3 方向把“显式 target”暴露成了主 UI，导致多人房间里心智偏重、界面噪音大，而且 popup 复杂度明显高于它带来的价值。

本任务要求移除输入框上的常驻 target chip / target popup 作为主交互入口，并把 bot-bound 房间的默认输入行为改成 room-first：普通消息默认携带 `explicit_room` 语义来抑制 Octos fallback，只有 reply-to-bot 才继续走显式 target。

## Decisions

- 默认输入模型: `RoomInputBar` 不再暴露手工 target 切换 UI；普通输入默认是 room-first
- 默认解析: 当 `ExplicitOverride::None` 且存在 `bound_bot_user_id` 时，输入栏运行时目标解析为 `ResolvedTarget::ExplicitRoom`，不再回落到 `RoomDefault`
- 普通房间保持不变: 当 `ExplicitOverride::None` 且不存在 `bound_bot_user_id` 时，解析结果仍为 `ResolvedTarget::NoTarget`
- Reply-to-bot 解析: 当默认结果原本会是 room-first 时，reply-to-bot 仍解析为 `ReplyBot(bot_user_id)`
- Reply-to-human 保持 room-first: reply-to-human 不得触发 bot target
- Mention 规则保持: 文本或结构化 `@mention` 命中 bot 时，不得继续附带 `target_user_id`；如果当前消息同时具备 room-first 语义，仍可保留 `explicit_room`
- 迁移策略: 旧版本持久化下来的 `ExplicitOverride::Bot` / `::Room` 在 restore 时一律丢弃，避免隐藏 target 状态在 UI 被移除后继续生效
- UI 策略: `target_indicator` 和 target popup 不再作为可见主 UI；reply 状态仍由现有 `ReplyingPreview` 承担

## Boundaries

### Allowed Changes
- src/room/room_input_bar.rs
- src/home/room_screen.rs
- specs/task-tg-bot-mention-reply-first.spec.md
- docs/superpowers/plans/2026-04-12-tg-bot-mention-reply-first-plan.md

### Forbidden
- 不要修改 Octos 后端
- 不要重新设计 `ReplyingPreview`
- 不要实现新的 bot menu button
- 不要实现 `/command@bot`
- 不要新增 cargo 依赖

## Out of Scope

- Telegram 风格 bot command menu
- BotFather 面板改版
- 显示名式 `@octos` mention 解析
- 单 bot 私聊的特殊自动路由策略

## Completion Criteria

Scenario: Bot-bound room defaults to room-first routing
  Test: test_bot_bound_room_defaults_to_explicit_room
  Given a room with `bound_bot_user_id = "@octosbot:127.0.0.1:8128"`
  And the user has no `ExplicitOverride`
  When the input bar resolves the current target without replying
  Then the `ResolvedTarget` is `ExplicitRoom`
  And a plain message sent from this state has `target_user_id = None`
  And a plain message sent from this state has `explicit_room = true`

Scenario: Replying to a human in a bot-bound room stays room-first
  Test: test_reply_to_human_in_bot_bound_room_stays_explicit_room
  Given a room with `bound_bot_user_id = "@octosbot:127.0.0.1:8128"`
  And no `ExplicitOverride`
  And the user is replying to a message from "@alice:127.0.0.1:8128"
  When the input bar resolves the current target
  Then the `ResolvedTarget` is `ExplicitRoom`
  And the outgoing message does not set `target_user_id`
  And the outgoing message keeps `explicit_room = true`

Scenario: Replying to a bot still targets that bot
  Test: test_reply_to_bot_still_targets_bot
  Given a room with `bound_bot_user_id = "@octosbot:127.0.0.1:8128"`
  And the user is replying to a message from "@octosbot:127.0.0.1:8128"
  When the input bar resolves the current target
  Then the `ResolvedTarget` is `ReplyBot("@octosbot:127.0.0.1:8128")`
  And the outgoing message sets `target_user_id = "@octosbot:127.0.0.1:8128"`
  And the outgoing message does not set `explicit_room`

Scenario: Reply-to-bot overrides the room-first default
  Test: test_reply_to_bot_overrides_room_first_default
  Given a room with `bound_bot_user_id = "@octosbot:127.0.0.1:8128"`
  And without replying the input bar would resolve to `ExplicitRoom`
  When the user replies to a message from "@octosbot:127.0.0.1:8128"
  Then the resolved target is `ReplyBot("@octosbot:127.0.0.1:8128")`
  And the resolved target is not `ExplicitRoom`

Scenario: Mentioning a bot in a bot-bound room does not attach target_user_id
  Test: test_message_bot_mention_keeps_explicit_room_marker
  Given a bot-bound room whose default resolved target is `ExplicitRoom`
  When the user sends the message "@octosbot_alexbot 你好"
  Then the outgoing message does not set `target_user_id`
  And the outgoing message keeps `explicit_room = true`
  And the message body still contains "@octosbot_alexbot"

Scenario: Target chip is hidden in bot-bound room
  Test: test_target_chip_hidden_in_bot_bound_room
  Given a room with `bound_bot_user_id = "@octosbot:127.0.0.1:8128"`
  When the room input bar syncs its target UI
  Then no target chip is shown

Scenario: Persisted explicit override is ignored on restore
  Test: test_persisted_explicit_override_is_ignored_on_restore
  Given a saved `RoomInputBarState` containing `ExplicitOverride::Bot("@octosbot:127.0.0.1:8128")`
  When the room input bar restores state
  Then the restored explicit override is `None`

Scenario: Invalid stale explicit bot selection is ignored after removing the target menu
  Test: test_stale_explicit_bot_selection_is_ignored_without_available_bots
  Given the current explicit override is `None`
  And there are no available bot targets in the current room
  When invalid stale code tries to apply `TargetMenuSelection::Bot("@octosbot:127.0.0.1:8128")`
  Then the explicit override remains `None`
