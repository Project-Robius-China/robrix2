spec: task
name: "Agent 配置中心 MVP — Labs 绑定方式 + App Service 共存"
inherits: project
tags: [agent, settings, labs, registry, search, makepad, ui]
estimate: 2d
---

## Intent

在 Settings ▸ Labs 新增一个 Agents 配置中心,作为已合并的 `AgentRegistry`(PR #196)的写入入口:
用户在该界面通过 framework 弹出选择器选择 Octos / Hermes / OpenClaw。Octos 当前走已有
App Service / BotFather 配置路径,可在同一 Labs 页面启用 App Service、输入 BotFather MXID、
检查 local Octos service。Hermes / OpenClaw 等普通 agent 走 Matrix ID 绑定路径:在配置区内
搜索或直接输入用户 ID,完成 Add friend 与本地 agent 标记,不要求用户另去搜索好友。

## Constraints

- `register_agent_from_search` 必须幂等:对已存在的 `agent_mxid` 重复注册不产生重复条目,且不覆盖已存在的 `AgentEntry`
- 注册 agent 不得修改或清空 `known_bot_user_ids` 与 `room_bindings`(App Service 绑定状态)
- 注册后必须调用 `save_app_state` 持久化,使配置在重启后保留
- 普通 agent 绑定必须在 Agents 配置界面内完成「搜索/输入 Matrix ID → Add friend → 标记 framework → 写入 AgentRegistry」,不要求用户另去 Add Room / 搜索好友页操作
- App Service 的 `BotSettings` widget 必须仍嵌入 Labs 的 desktop 与 mobile 两个变体,enable / BotFather / Octos URL 行为不得回退
- 用户搜索必须复用 `MatrixRequest::SearchDirectory`(`kind: People`),不得新建搜索后端
- framework 选择必须使用弹出选择控件,至少提供 `Octos`、`Hermes`、`OpenClaw` 三个选项;普通绑定写入 `AgentEntry.framework`
- Agents 界面内禁止裸 hex 颜色,使用 `RBX_*` 或 `styles.rs` token

## Decisions

- 新 widget:`src/settings/agent_settings.rs` 中的 `AgentSettings`,镜像 `src/settings/bot_settings.rs` 的 `#[derive(Script, ScriptHook, Widget)]` + `handle_actions` + sync-from-scope + persist 模式
- 嵌入点:在 `settings_screen.rs` 的 Labs 页 desktop 与 mobile 两个变体中都加 `agent_settings := AgentSettings {}`,置于 `bot_settings`(App Service)卡之上;两变体使用相同 widget id(`PageFlip` 要求)
- App Service 卡:`BotSettings` 原样保留,位于 Agents 卡之下,作为 Octos App Service / BotFather / local Octos health check 的真实配置入口;Agents 卡在选择 Octos 时展示说明并指向该卡,不复制 BotSettings 的状态机
- 搜索:`submit_async_request(MatrixRequest::SearchDirectory { query, kind: RemoteDirectorySearchKind::People, limit: 30 })`,结果经 `RoomFilterRemoteSearchAction::Results` 返回,结果行复用 `invite_modal.rs` 的展示结构(头像 + display_name + mxid)
- framework 选项:`Octos` / `Hermes` / `OpenClaw` 使用 `DropDownFlat`,映射到 `AgentFramework`
- 注册函数:`pub fn register_agent_from_search(app_state: &mut AppState, user_id: OwnedUserId, display_name: Option<String>, framework: AgentFramework) -> bool` 构造 `AgentEntry { display_name, framework, ..Default::default() }` 调 `app_state.agent_registry.register(...)`,UI 侧随后调 `persistence::save_app_state(app_state.clone(), user_id)`
- 普通绑定函数:`pub fn parse_agent_user_id(raw: &str) -> Result<OwnedUserId, String>` 只接受完整 Matrix user ID;UI 成功注册后触发现有 add-friend 后端请求并把按钮/文案表现为 `Add friend & bind`
- 列表:读 `app_state.agent_registry.agent_user_ids()` 渲染每个 agent(mxid + framework 徽章),徽章用 `RBX_*` token
- 测试:数据逻辑用 Rust 单测;UI 接线用「读源码断言」契约测试(本仓库既有约定,见 `task-login-mobile-redesign.spec.md`)

## Boundaries

### Allowed Changes
- src/settings/agent_settings.rs
- src/settings/settings_screen.rs
- src/settings/mod.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json
- specs/task-agent-config-labs.spec.md

### Forbidden
- 不要删除或改动 `BotSettings`(App Service)/ `known_bot_user_ids` / `bot_binding_modal` / `create_bot_modal` 的业务流
- 不要构建 agent 详情 / 编辑 / 删除页,或 capabilities / trust_tier / avatar 的编辑 UI
- 不要改动移动端底部导航或 Settings 的外层 AdaptiveView 结构
- 不要新建用户搜索后端
- 不要新增 cargo 依赖
- 不要在 Agents 界面写裸 hex 颜色字面量
- 不要运行 `cargo fmt`

## Out of Scope

- Agent 详情 / 编辑 / 删除页面
- capabilities / trust_tier / avatar 的编辑界面
- 消息专属渲染 / timeline badge / 房间列表 badge(P0-B、P0-C)
- BotFather 创建流程的改造
- `/createbot` 命令表单化或发送逻辑改造
- 新建 Matrix 好友协议或联系人协议;若底层仍复用当前 Robrix 的 direct invite/add-friend 实现,不得在 Agent 配置 UI 中暴露为独立 DM 创建流程

## Completion Criteria

场景: 搜索结果注册为 Octos agent
  测试: test_register_searched_agent_octos
  假设 `AppState` 的 agent_registry 不含 "@svc:example.org"
  当 以 framework `Octos`、display_name "Svc" 注册 "@svc:example.org"
  那么 agent_registry 含 "@svc:example.org"
  并且 该条目的 framework 为 `Octos`
  并且 该条目的 display_name 为 "Svc"

场景: 普通账号注册为 Hermes agent
  测试: test_register_searched_agent_hermes
  假设 `AppState` 的 agent_registry 不含 "@helper:example.org"
  当 以 framework `Hermes` 注册 "@helper:example.org"
  那么 agent_registry 含 "@helper:example.org"
  并且 该条目的 framework 为 `Hermes`

场景: 直接输入完整 Matrix ID 可以注册并用于 Add friend 绑定
  测试: test_parse_agent_user_id_accepts_full_mxid
  假设 用户在 Agents 配置区输入 "@helper:example.org"
  当 调用 `parse_agent_user_id`
  那么 返回的 user_id 为 "@helper:example.org"

场景: 直接输入 localpart 被拒绝
  测试: test_parse_agent_user_id_rejects_localpart
  假设 用户在 Agents 配置区输入 "helper"
  当 调用 `parse_agent_user_id`
  那么 返回错误,提示需要完整 Matrix user ID

场景: Labs 的两个变体都嵌入 AgentSettings
  测试: test_labs_embeds_agent_settings_both_variants
  假设 读取 `settings_screen.rs` 源码
  当 统计 "agent_settings" 的嵌入次数
  那么 源码包含 "AgentSettings"
  并且 desktop 与 mobile 两个 Labs 变体各自包含 "agent_settings"

场景: App Service 仍嵌入 Labs 不回退
  测试: test_labs_still_embeds_app_service
  假设 读取 `settings_screen.rs` 源码
  当 检查 App Service 入口
  那么 源码仍包含 "bot_settings"
  并且 源码仍包含 "BotSettings"

场景: 重复注册同一 mxid 保持幂等
  测试: test_register_searched_agent_idempotent
  假设 agent_registry 已含以 framework `Octos` 经 `register_agent_from_search` 注册的 "@svc:example.org"
  当 再次以 framework `Hermes` 调 `register_agent_from_search` 注册同一 `agent_mxid` "@svc:example.org"
  那么 agent_registry 中该 mxid 仅有 1 个条目
  并且 该条目的 framework 仍为 `Octos`

场景: 注册不破坏 App Service 绑定状态
  测试: test_register_agent_preserves_app_service_state
  假设 `BotSettingsState` 的 `known_bot_user_ids` 含 "@botA:example.org"
  当 注册 "@svc:example.org" 为 agent
  那么 `known_bot_user_ids` 仍含 "@botA:example.org"
  并且 `known_bot_user_ids` 条目数量不变

场景: framework 选项提供 Octos / Hermes / OpenClaw
  测试: test_framework_options_include_octos_hermes_openclaw
  假设 调用 `framework_options()`
  当 检查返回的可选 framework 列表
  那么 列表包含 `Octos`
  并且 列表包含 `Hermes`
  并且 列表包含 `OpenClaw`

场景: framework 选择使用弹出控件而不是分段按钮
  测试: test_framework_selector_uses_dropdown
  假设 读取 `agent_settings.rs` 源码
  当 检查 framework selector 的 DSL
  那么 源码包含 "framework_dropdown := DropDownFlat"
  并且 源码不包含 "framework_octos_button"

场景: 普通 agent 注册后触发 Add friend 绑定入口
  测试: test_agent_settings_exposes_add_friend_binding
  假设 读取 `agent_settings.rs` 源码
  当 检查注册按钮和绑定动作
  那么 源码包含 "Add friend & bind"
  并且 源码包含 "register_selected"

场景: framework 标签对每个变体唯一可读
  测试: test_framework_label_maps_each_variant
  假设 对每个 `AgentFramework` 变体调用 `framework_label`
  当 比较 `Octos` / `Hermes` / `OpenClaw` / `Unknown` 的标签
  那么 四个标签两两不同
  并且 `Unknown` 的标签非空
