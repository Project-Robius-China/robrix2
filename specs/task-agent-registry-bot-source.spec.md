spec: task
name: "AgentRegistry 数据层 + 全局 bot 识别源"
inherits: project
tags: [agent, registry, bot, persistence, timeline, data-layer]
estimate: 1d
---

## Intent

为 Robrix2 的 agent 体系打地基:在 `AppState` 中新增并持久化 `AgentRegistry`，
作为全局 agent 身份的真相源，并把现有房间级 `known_bot_user_ids` 提升为全局识别来源——
timeline 的 bot 识别从"只认房间级 known_bot_user_ids"改为"认 AgentRegistry ∪ known_bot_user_ids"。
这是 P0-A Agent 配置中心的依赖根(清单第 2、5、6 条);它不含任何列表 / 表单 / CRUD 界面,
也不动 Settings,只交付数据模型、迁移、并集识别与 per-account 持久化。

## Constraints

- `AgentRegistry` 必须作为 `AppState` 的新字段持久化，复用既有 `latest_app_state.json` per-account 机制(`persistent_state_dir(user_id)`)，不得新建独立存储文件或目录
- 新字段必须标注 `#[serde(default)]`，使缺少该字段的旧 `AppState` JSON 反序列化为空 registry 而不报错
- 不得删除、重命名或停用现有 `known_bot_user_ids` 字段及其读写路径——`bot_binding_modal.rs` / BotFather 流程仍依赖它
- timeline bot 识别必须取 `AgentRegistry ∪ known_bot_user_ids` 的并集，确保现有房间级识别不回退

## Decisions

- 存放位置:`AgentRegistry` 作为 `AppState` 新增字段 `#[serde(default)] pub agent_registry: AgentRegistry`，随既有 `save_app_state` / `load_app_state` 自动持久化
- 数据模型:`AgentRegistry` 内部为 `BTreeMap<OwnedUserId, AgentEntry>`，key = `agent_mxid`;`BTreeMap` 保证去重与确定性 serde 顺序(便于 round-trip 单测)
- `AgentEntry` 字段:`display_name: Option<String>`、`framework: AgentFramework`、`avatar: Option<OwnedMxcUri>`、`capabilities: Vec<AgentCapability>`、`trust_tier: TrustTier`
- `AgentFramework` 枚举含 `Unknown`(默认)、`Octos`、`Hermes`、`OpenClaw` 变体;`TrustTier` 默认取最低信任档;`capabilities` 默认为空 `Vec`
- 迁移:加载后若 `agent_registry` 为空，则用现有 `known_bot_user_ids` 播种——每个 mxid 生成 `framework: Unknown`、其余字段默认的 `AgentEntry`;已存在的 mxid 条目不被覆盖
- 刀4 注入点:`room_screen.rs` 中现有以 `known_bot_user_ids: &[OwnedUserId]` 计算 `is_bot_sender` 的 sender 匹配 helper，改为同时查询 `AgentRegistry`，返回两者并集的判定
- 损坏回退:沿用 `load_app_state` 既有行为——反序列化失败时回退 `AppState::default()` 并备份旧文件，不 panic

## Boundaries

### Allowed Changes
- src/app.rs
- src/home/room_screen.rs
- src/persistence/app_state.rs
- specs/task-agent-registry-bot-source.spec.md

### Forbidden
- 不要新增 agent 列表 / 表单 / 增删改的 UI 组件
- 不要修改 `src/home/settings_screen.rs`
- 不要删除 / 重命名 `known_bot_user_ids` 或其读写方法
- 不要改动 `bot_binding_modal.rs` / `create_bot_modal.rs` / BotFather 业务流
- 不要新建独立持久化文件或目录
- 不要新增 cargo 依赖
- 不要运行 `cargo fmt`

## Out of Scope

- 清单第 1 条:Lab / Settings 中的 Agent 配置中心 UI(列表 / 详情表单 / 增删改入口)
- 清单第 3 条:绑定已有 Matrix 账号为 agent
- 清单第 4 条:接入 BotFather 创建 bot 流程
- CapabilityChip 等渲染 framework / trust_tier / capabilities 的可视组件
- avatar 的实际拉取与渲染、capabilities 与 trust_tier 的界面展示

## Completion Criteria

场景: AgentRegistry serde round-trip 保持相等
  测试: test_agent_registry_serde_roundtrip
  假设 一个含两个 AgentEntry 的 AgentRegistry
  当 将其序列化为 JSON 再反序列化
  那么 反序列化结果与原 registry 相等
  并且 两个条目的 framework 与 trust_tier 字段值保持不变

场景: 迁移把 known_bot_user_ids 播种为 Unknown framework 条目
  测试: test_migrate_known_bots_into_registry_as_unknown_framework
  假设 AppState 的 agent_registry 为空且 known_bot_user_ids 含以下 mxid:
    | mxid                  |
    | @botA:example.org     |
    | @botB:example.org     |
  当 执行 registry 迁移播种
  那么 agent_registry 含 "@botA:example.org" 且其 framework 为 Unknown
  并且 agent_registry 含 "@botB:example.org" 且其 framework 为 Unknown

场景: registry 中的 agent 在 timeline 被识别为 bot sender
  测试: test_registry_agent_detected_as_bot_sender
  假设 known_bot_user_ids 为空
  并且 agent_registry 含 "@agent:example.org"
  当 对 sender "@agent:example.org" 计算 is_bot_sender
  那么 is_bot_sender 为 true

场景: AgentRegistry 按账号持久化且不跨账号串号
  测试: test_agent_registry_persists_per_account_no_cross_leak
  假设 账号 "@alice:example.org" 的 `AgentRegistry` 含 "@agent:example.org"
  并且 账号 "@bob:example.org" 的 `AgentRegistry` 为空
  当 分别把两个账号的 `AppState` 持久化到各自的 `persistent_state_dir(user_id)` 再各自加载
  那么 "@alice:example.org" 加载后的 `AgentRegistry` 含 "@agent:example.org"
  并且 "@bob:example.org" 加载后的 `AgentRegistry` 为空

场景: 旧 AppState JSON 缺少 agent_registry 字段反序列化为空 registry
  测试: test_load_legacy_app_state_without_registry_field_defaults_empty
  假设 一段不含 "agent_registry" 键的旧 AppState JSON
  当 反序列化为 AppState
  那么 反序列化成功且不 panic
  并且 agent_registry 条目数量为 0

场景: 重复 mxid 注册保持幂等
  测试: test_register_duplicate_mxid_is_idempotent
  假设 agent_registry 已含 "@agent:example.org"
  当 再次以相同 mxid 注册一个 AgentEntry
  那么 agent_registry 中该 mxid 仅有 1 个条目

场景: 普通用户不被误判为 bot sender
  测试: test_non_agent_user_not_detected_as_bot
  假设 known_bot_user_ids 为空且 agent_registry 为空
  当 对 sender "@human:example.org" 计算 is_bot_sender
  那么 is_bot_sender 为 false

场景: 空 registry 且无 known bot 时 timeline 不显示 bot 卡
  测试: test_empty_registry_and_no_known_bots_shows_no_bot_card
  假设 agent_registry 为空且 known_bot_user_ids 为空
  当 对任一 sender 计算 timeline bot 渲染状态
  那么 show_card 为 false

场景: 迁移后 known_bot_user_ids 字段仍保留不被清空
  测试: test_migration_preserves_known_bot_user_ids
  假设 `known_bot_user_ids` 含 "@botA:example.org"
  当 执行 registry 迁移播种后读取 `known_bot_user_ids`
  那么 `known_bot_user_ids` 仍含 "@botA:example.org"
  并且 `known_bot_user_ids` 条目数量不为 0

场景: 损坏的 agent_registry JSON 回退到默认 AppState
  测试: test_corrupt_registry_json_falls_back_to_default_state
  假设 持久化文件中的 agent_registry 字段为结构损坏的 JSON
  当 调用 load_app_state 加载该账号状态
  那么 返回默认 AppState 且不 panic
  并且 旧的损坏文件被备份保留
