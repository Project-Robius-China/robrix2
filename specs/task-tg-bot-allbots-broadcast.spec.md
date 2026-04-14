spec: task
name: "Telegram Bot Orchestration — /allbots Broadcast to Bound Bots"
inherits: project
tags: [bot, orchestration, broadcast, matrix, octos]
depends: [task-tg-bot-bot-aware-command-discovery, task-tg-bot-command-at-addressing]
estimate: 2d
---

## Intent

当前 `robrix2` 已经支持把消息显式发给某个 bot，但还不支持“把同一条任务同时发给这个房间绑定的所有 bot”。对多 bot 协作来说，这会迫使用户手动逐个 `@bot` 或重复发送同一条消息，不符合 Telegram 风格的 bot orchestration 心智。

本任务新增一个 **BotFather 编排命令**：`/allbots <message>`。用户在显式绑定了
BotFather 的房间里提交该命令后，Robrix 仍然只把命令发给 BotFather；
Octos/BotFather 再根据当前房间的显式 bot bindings，把 `<message>` fan-out
给所有 child bots。每个 bot 各自回复，不做聚合。

## Decisions

- **命令语法**：`/allbots <message>`。`<message>` 是必填参数；空消息无效。
- **可用上下文**：`/allbots` 只允许在 `ManagementRoom` 上下文使用，也就是：
  - 当前房间显式绑定了 BotFather
  - 当前房间不是单纯的 BotFather DM
- **BotFather-only 路由**：Robrix 发送 `/allbots` 时仍按普通 BotFather 管理命令
  规则，`target_user_id = parent_bot_user_id`，`explicit_room = false`。
- **广播目标来源**：BotFather 在 Octos 内部按“当前 room 的显式 bot bindings”
  解析目标集合；只包含 child bots，排除 BotFather 自己。
- **广播目标快照**：广播时使用提交瞬间的 bound-bot 快照；后续房间绑定变化不影响
  已发出的 `/allbots` 请求。
- **无目标时拒绝**：如果当前 room 没有任何 child bot bindings，BotFather 返回
  友好错误，不做空广播。
- **human-only**：只有人类发起的 `/allbots` 有效。bot-originated `/allbots`
  必须被拒绝，防止 bot 自己触发新的广播。
- **内部 fan-out，不走 Matrix bot-to-bot message**：BotFather 对 child bots 的
  fan-out 在 Octos 内部完成，不能依赖“BotFather 先向 Matrix 发消息，再让总线
  把 bot 消息再路由给 child bot”的方式。原因是当前 `octos-bus` 会忽略
  appservice 管理的 bot sender 消息，这层保护不能被 `/allbots` 绕开。
- **fan-out 后的 child 消息 body**：每个 child bot 收到的内容是裸 `<message>`，
  不包含 `/allbots` 前缀。
- **原始请求者保留**：child bot 的 fan-out inbound message 必须保留原始人类
  requester 身份，用于审计和权限判断；不能把 BotFather 自己伪装成 requester。
- **回复模型**：每个 child bot 独立回复，房间里会出现多条独立消息；v1 不做
  聚合结果卡片。
- **loop/storm guard**：`/allbots` fan-out 必须受保护：
  - 默认最大目标数 `8`
  - 每个 `/allbots` 请求只 fan-out 一层，不允许 child bot 回复再次触发 fan-out
  - Octos 必须为 `/allbots` 记录一次 broadcast request id，用于审计和去重
- **审计记录**：Octos/BotFather 至少记录：
  - requester Matrix user id
  - room id
  - broadcast request id
  - 目标 child bot 列表
  - 原始 `<message>`
- **命令发现配合**：`/allbots` 只在 `ManagementRoom` 的 slash/menu discovery 中显示；
  `ManagementDm` 和 `ChildBotRoom` 都不显示。

## Boundaries

### Allowed Changes
- specs/task-tg-bot-allbots-broadcast.spec.md
- src/shared/mentionable_text_input.rs
- src/room/room_input_bar.rs
- ../octos/crates/octos-cli/src/session_actor.rs
- ../octos/crates/octos-bus/src/matrix_channel.rs
- ../octos/book/src/advanced.md
- ../octos/book/src/channels.md

### Forbidden
- 不要修改 `@room` 的原生 Matrix 语义
- 不要让 `/allbots` 在 BotFather DM 里生效
- 不要依赖 Matrix bot-to-bot 外发消息做广播
- 不要把 BotFather 自己包含进 fan-out 目标
- 不要做结果聚合 UI
- 不要新增 cargo 依赖

## Out of Scope

- `/allbots@subset`
- 按 bot 标签/角色筛选
- 广播结果聚合卡片
- 定时 `/allbots`
- bot 主动触发 bot 广播

## Completion Criteria

Scenario: /allbots is routed to BotFather from a management room
  Test: test_allbots_command_targets_parent_bot
  Given the room context is `ManagementRoom`
  And the user submits `/allbots summarize this issue`
  When Robrix builds the outgoing message
  Then the outgoing message `target_user_id` is the parent/management bot
  And the outgoing message body is `/allbots summarize this issue`
  And `explicit_room` is false

Scenario: /allbots is hidden outside management rooms
  Test: test_allbots_not_discoverable_outside_management_room
  Given the command discovery context is `ManagementDm`
  Then `/allbots` is absent from the popup
  When the context is `ChildBotRoom`
  Then `/allbots` is absent from the popup

Scenario: BotFather rejects /allbots when no child bot bindings exist
  Test: test_allbots_rejects_when_room_has_no_child_bindings
  Given a management room whose explicit bot bindings contain only the parent bot
  When BotFather receives `/allbots summarize this issue`
  Then no child bot fan-out occurs
  And BotFather sends a user-visible failure message
  And the failure explains that no bound child bots were found

Scenario: BotFather fans out /allbots to all bound child bots
  Test: test_allbots_fans_out_to_bound_child_bots
  Given a management room bound to `@octosbot_alexbot` and `@octosbot_bob`
  And the room is also bound to the parent bot `@octosbot`
  When BotFather receives `/allbots summarize this issue`
  Then Octos creates one internal fan-out delivery for `@octosbot_alexbot`
  And Octos creates one internal fan-out delivery for `@octosbot_bob`
  And neither delivery targets the parent bot
  And each child bot receives the body `summarize this issue`

Scenario: /allbots preserves the original human requester
  Test: test_allbots_preserves_original_requester_identity
  Given a human requester `@alex:127.0.0.1:8128`
  And BotFather receives `/allbots summarize this issue`
  When Octos creates the child-bot fan-out messages
  Then each child-bot delivery records `@alex:127.0.0.1:8128` as the requester identity
  And BotFather is not treated as the requester

Scenario: /allbots is rejected when triggered by a bot sender
  Test: test_allbots_rejects_bot_originated_broadcast
  Given an appservice-managed bot sender issues `/allbots summarize this issue`
  When BotFather evaluates the command
  Then the command is rejected
  And no fan-out delivery occurs

Scenario: /allbots fan-out uses internal dispatch instead of Matrix bot-to-bot messages
  Test: test_allbots_uses_internal_fanout_dispatch
  Given the current Matrix bus ignores appservice-managed bot sender messages
  When BotFather handles `/allbots summarize this issue`
  Then the fan-out path does not rely on Matrix outbound bot messages being re-consumed
  And child-bot deliveries are created through Octos internal dispatch

Scenario: /allbots enforces maximum fan-out target count
  Test: test_allbots_rejects_more_than_max_targets
  Given a management room with `9` bound child bots
  And the configured maximum fan-out target count is `8`
  When the user submits `/allbots summarize this issue`
  Then the command is rejected before fan-out
  And BotFather sends a user-visible error mentioning the target-count limit

Scenario: /allbots replies remain unaggregated
  Test: test_allbots_child_replies_are_independent
  Given `@octosbot_alexbot` and `@octosbot_bob` both receive the fan-out request
  When each child bot replies
  Then the room timeline shows separate replies from each child bot
  And no aggregate summary card is created by Robrix or BotFather
