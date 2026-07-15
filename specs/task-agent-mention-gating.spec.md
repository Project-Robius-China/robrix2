spec: task
name: "Agent Mention Gating — 多 agent 房间仅 @ 响应"
inherits: project
tags: [bot, octos, routing, mention-gating, multi-agent]
depends: [task-tg-bot-explicit-room-no-fallback]
estimate: 1d
---

## Intent

支持多用户多 agent 在同一房间共存：房间里的 agent 只有被显式寻址（@ 提及、回复 bot 消息、显式 target）时才响应；唯一豁免是"agent 私聊房"——1 个人类 + 该 appservice 在此房间仅管理 1 个 bot。当前有两个缺陷破坏该规则：(1) Palpo 的 `joined_members` 不返回 appservice 虚拟用户，octos 门控按成员表数 bot 时永远得 0，多 bot 房被误判为私聊；(2) Robrix2 在 DM 房间对绑定 bot 做隐式定向（每条消息自动携带 `org.octos.target_user_id`），绕过门控。本任务修复这两处。

## Decisions

- "agent 私聊房"判定仅用于 agent 响应门控：`humans <= 1` 且 `managed_bots <= 1`；该定义不改变 Robrix 中基于 Matrix 协议的房间隐私 / `m.direct` / `is_dm_room` 语义
- octos 门控的 bot 计数来源改为 `BotRouter` 的房间到 bot 映射（room map）与 `joined_members` 派生计数二者的最大值 `max(room_map_bots, joined_members_bots)`——Palpo 隐藏 appservice 虚拟用户时以 room map 为准
- 人类计数来源保持 `joined_members`（真实用户始终在列）
- 成员信息无法获取时保持 fail-closed：按非私聊处理，要求显式寻址（由 `gate_fails_closed_when_membership_unknown` 验证）
<!-- lint-ack: decision-coverage — fail-closed 决策已由场景"成员信息无法获取时 fail-closed 按非私聊处理"覆盖并绑定测试,linter 中文关键词匹配未识别 -->
- `mention_only` channel 级配置保留且默认 `true`；`org.octos.explicit_room` 标记的消息即使 `mention_only = false` 也走同一门控
- Robrix2 `resolve_target`：删除 `is_dm_room && bound_bot` 时返回 `ResolvedTarget::ExplicitBot` 的隐式定向分支；房间绑定（`RoomBotBindingState`）降级为 UI 偏好（known bots 列表与默认选中），不再自动成为消息 target
- 显式寻址渠道保持不变：Matrix `@mention`、回复 bot 消息、`ExplicitOverride::Bot`（UI 显式选择 To @bot）
- 1:1 私聊免 @ 体验由 octos 端私聊豁免提供，不依赖客户端隐式定向

## Boundaries

### Allowed Changes
- specs/task-agent-mention-gating.spec.md
- src/room/room_input_bar.rs
- ../octos/crates/octos-bus/src/matrix_channel.rs

### Forbidden
- 不要修改 Robrix 中 `is_dm_room` 判定、`m.direct` account-data 处理、房间 join rules / 可见性相关代码
- 不要修改 `ExplicitOverride` / `ResolvedTarget` 的枚举形态与 target chip UI
- 不要修改 bot 绑定的存储结构 `RoomBotBindingState`
- 不要改变 `@mention` 与显式 target 的既有路由优先级
- 不要添加新的 cargo 依赖

## Out of Scope

- 房间级 / bot 级"仅 @ 响应"开关的 UI 配置
- octos user-account 模式（`matrix_user_channel`）的 `require_mention` 行为
- agent-chat bridge 侧的 mention 门控
- 私聊豁免本身的开关配置（如 `dm_mention_required`）
- 多 bot 房间的显式 bot 选择菜单

## Completion Criteria

<!-- 注: octos-bus 场景位于 ../octos 仓库且需要 `cargo test -p octos-bus --lib --features matrix` 运行;
     agent-spec verify 无法跨仓库传递 cargo feature 标志,这 5 个场景以手动运行验证
     (2026-07-12: 351 passed, 0 failed)。robrix 侧 4 个场景由 agent-spec verify --code . 机械验证通过。 -->

### Rule: gate-bot-count-from-room-map — octos 门控以 room map 计数 bot

场景: 多 bot 映射房间拦截未寻址消息（critical）
  测试:
    包: octos-bus
    过滤: gate_blocks_unaddressed_when_room_map_has_multiple_bots
  假设 BotRouter room map 中该房间映射了 2 个 bot
  并且 homeserver `joined_members` 仅返回 1 个人类成员
  当 收到一条无 @ 提及、无 `org.octos.target_user_id` 的普通消息且 `mention_only = true`
  那么 该消息不产生 `InboundMessage`

场景: joined_members 隐藏 bot 时以 room map 计数为准
  测试:
    包: octos-bus
    过滤: room_member_counts_uses_room_map_when_homeserver_hides_bots
  假设 homeserver `joined_members` 仅返回 1 个人类（不含任何 appservice 虚拟用户）
  并且 BotRouter room map 中该房间映射了 2 个 bot
  当 门控计算房间成员构成
  那么 bot 计数为 2
  并且 该房间不满足私聊豁免条件

场景: room map 单 bot 私聊豁免放行
  测试:
    包: octos-bus
    过滤: gate_allows_unaddressed_dm_with_single_mapped_bot
  假设 BotRouter room map 中该房间仅映射 1 个 bot
  并且 homeserver `joined_members` 仅返回 1 个人类成员
  当 收到一条无 @ 提及的普通消息且 `mention_only = true`
  那么 该消息产生 `InboundMessage` 并派发给该 bot

场景: 多 bot 房间中 @ 提及照常派发
  测试:
    包: octos-bus
    过滤: gate_dispatches_when_addressed
  假设 房间不满足私聊豁免条件
  当 收到一条 @ 提及某 bot 的消息
  那么 门控放行该消息

场景: 成员信息无法获取时 fail-closed 按非私聊处理
  测试:
    包: octos-bus
    过滤: gate_fails_closed_when_membership_unknown
  假设 homeserver 成员查询失败且缓存无记录
  当 收到一条无 @ 提及的普通消息且 `mention_only = true`
  那么 门控按非私聊处理，要求显式寻址
  并且 该消息不产生 `InboundMessage`

### Rule: no-implicit-bot-targeting — Robrix 绑定 bot 不再隐式定向

场景: DM 房间绑定 bot 的普通消息不携带 target（critical）
  测试: test_resolve_target_dm_with_bound_bot_returns_no_target
  假设 房间 `is_dm_room = true` 且已绑定 bot
  并且 无显式 override、非回复场景
  当 解析发送目标
  那么 解析结果不是 `ResolvedTarget::ExplicitBot`
  并且 发出的消息内容不包含 `org.octos.target_user_id`

场景: 回复 bot 消息仍显式定向该 bot
  测试: test_resolve_target_reply_to_bot_targets_bot
  假设 用户正在回复一条 bot 发送的消息
  当 解析发送目标
  那么 解析结果为该 bot 的显式 target

场景: UI 显式选择 To @bot 仍定向
  测试: test_resolve_target_explicit_bot_override_targets_bot
  假设 输入栏处于 `ExplicitOverride::Bot` 状态
  当 解析发送目标
  那么 解析结果为该 bot 的显式 target

场景: 群房间绑定 bot 的普通消息不携带 target
  测试: test_resolve_target_group_room_with_bound_bot_is_room
  假设 房间 `is_dm_room = false` 且已绑定 bot
  并且 无显式 override、非回复场景
  当 解析发送目标
  那么 解析结果不携带 bot target
