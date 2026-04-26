spec: task
name: "Telegram Bot UI Alignment — Phase 2b: ExplicitRoom Suppresses Room Fallback"
inherits: project
tags: [bot, ui, telegram-parity, explicit-room, octos]
depends: [task-tg-bot-explicit-targeting]
estimate: 1d
---

## Intent

修复 `To room` 的误导性行为。当前 Robrix2 在 `ExplicitOverride::Room` 状态下虽然不再附带 `target_user_id`，但 Octos 仍会对单 bot 房间执行 `route_by_room()` fallback，导致普通消息仍被 bot 接收并回复。这与用户对 `To room` 的直觉和 Telegram 风格显式 target 语义不符。

本任务要求 `To room` 成为真正的“发送给房间，不走默认 bot fallback”语义，同时保持现有显式 target、`@mention` 和 reply-to-bot 行为不变。

## Decisions

- 新增消息内容标记: `org.octos.explicit_room: true`
- Robrix2 只在 `ExplicitOverride::Room` 解析为发送路径时写入该标记；`RoomDefault`、`ExplicitBot`、`ReplyBot`、无 target 的普通房间消息均不写入
- `org.octos.explicit_room` 在 Matrix appservice 路径上表示“这条消息明确发给房间”；当消息同时不包含 `org.octos.target_user_id` 且不包含 bot `@mention` 时，Octos 不得把它 fallback 到主 profile，也不得继续生成 `InboundMessage`
- `org.octos.explicit_room` 不禁止显式 `org.octos.target_user_id`，也不禁止 Matrix `@mention` 路由
- Octos 路由优先级调整为: `(1) explicit target` → `(2) matrix @mention` → `(3) explicit_room guard` → `(4) room fallback`
- Robrix2 的普通发送路径和 reply 发送路径都必须支持 `explicit_room` 标记，避免 “To room” 在 reply 场景下退化
- `To room` 的 UI 文案不变；修复只改变其后端语义，使之与现有文案一致

## Boundaries

### Allowed Changes
- specs/task-tg-bot-explicit-room-no-fallback.spec.md
- src/room/room_input_bar.rs
- src/sliding_sync.rs
- ../octos/crates/octos-bus/src/matrix_channel.rs

### Forbidden
- 不要修改 `ExplicitOverride` / `ResolvedTarget` 的现有状态模型
- 不要修改 target chip 的文案或布局
- 不要实现多 bot room 切换菜单
- 不要实现 `/command@bot` 语法
- 不要改变 `@mention` 的既有路由优先级
- 不要添加新的 cargo 依赖

## Out of Scope

- 多 bot 房间的显式 bot 选择 UI
- Telegram 风格 menu button
- 动态命令注册
- 重新设计 Octos 的整体 room routing 模型

## Completion Criteria

Scenario: ExplicitRoom plain message carries explicit_room marker
  Test: test_send_message_explicit_room_sets_octos_explicit_room_marker
  Given a room whose input bar is in `ExplicitOverride::Room`
  When the user sends a plain text message
  Then the outgoing message content includes `org.octos.explicit_room = true`
  And the outgoing message content does not include `org.octos.target_user_id`

Scenario: ExplicitRoom reply message carries explicit_room marker
  Test: test_send_reply_explicit_room_sets_octos_explicit_room_marker
  Given a room whose input bar is in `ExplicitOverride::Room`
  And the user is replying to an existing event
  When the user sends the reply
  Then the outgoing reply content includes `org.octos.explicit_room = true`
  And the outgoing reply content does not include `org.octos.target_user_id`

Scenario: RoomDefault message does not carry explicit_room marker
  Test: test_send_message_room_default_does_not_set_octos_explicit_room_marker
  Given a bot-bound room with no explicit override
  When the user sends a plain text message
  Then the outgoing message content does not include `org.octos.explicit_room`

Scenario: Octos drops unaddressed explicit_room messages instead of dispatching to a bot
  Test: test_handle_transaction_explicit_room_skips_room_fallback
  Given Octos knows exactly one bot profile for room `"!room:localhost"`
  And an incoming `m.room.message` event contains `org.octos.explicit_room = true`
  And the event contains no `org.octos.target_user_id`
  And the event body contains no bot mention
  When Octos handles the transaction
  Then Octos does not emit any `InboundMessage` for that event

Scenario: Octos still routes by mention when explicit_room marker is present
  Test: test_handle_transaction_explicit_room_preserves_mention_routing
  Given Octos knows exactly one bot profile for room `"!room:localhost"`
  And `"@bot_weather:localhost"` maps to profile `"profile-weather"`
  And an incoming `m.room.message` event contains `org.octos.explicit_room = true`
  And the event body explicitly mentions `"@bot_weather:localhost"`
  When Octos handles the transaction
  Then inbound message metadata contains `target_profile_id = "profile-weather"`

Scenario: Octos still routes by explicit target when explicit_room marker is present
  Test: test_handle_transaction_explicit_room_preserves_explicit_target_priority
  Given `"@bot_weather:localhost"` maps to profile `"profile-weather"`
  And an incoming `m.room.message` event contains both `org.octos.explicit_room = true` and `org.octos.target_user_id = "@bot_weather:localhost"`
  When Octos handles the transaction
  Then inbound message metadata contains `target_profile_id = "profile-weather"`

Scenario: Invalid explicit_room marker does not suppress room fallback
  Test: test_handle_transaction_invalid_explicit_room_marker_ignored
  Given Octos knows exactly one bot profile for room `"!room:localhost"`
  And an incoming `m.room.message` event contains `org.octos.explicit_room = "true"` as a string value
  And the event contains no `org.octos.target_user_id`
  And the event body contains no bot mention
  When Octos handles the transaction
  Then inbound message metadata contains `target_profile_id`
