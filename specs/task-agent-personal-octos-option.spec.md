spec: task
name: "Agent Lab OctosDirect(user-account octos,direct 方式)注册选项"
inherits: project
tags: [agent, settings, labs, registry, octos, user-account]
estimate: 1d
---

## Intent

在 Agent Lab(`task-agent-config-labs`)的 framework 选择器里新增一个「OctosDirect」选项(以 direct 方式
加入的 user-account 模式 octos),让用户把一个 user-account 模式的 octos(如 `@myagent`,**可部署在任意
机器,不必与 robrix 或 homeserver 同机**——robrix 只经 Matrix 通过其 MXID 与它交互)按普通 Matrix ID
绑定路径注册进 `AgentRegistry`,而**不**走现有 `Octos` 选项的 App Service / BotFather 配置路径。同时修复既有缺陷:
现有 `register_agent_with_modal_settings` 对任意 `Octos` 注册都无条件把它设为 BotFather,导致注册一个
child / 非父 bot 会覆盖掉已配置的 BotFather——本任务加一道守卫使其不再误覆盖。这是 Phase 2 的第一步,
为后续「从 registry 里选 bot 邀请进房」(`/invitebot`,2b)提供一个诚实、正确标注的数据来源。

## Decisions

- 新增枚举变体 `OctosDirect`(全名 `AgentFramework::OctosDirect`),语义为「个人 / user-account
  模式的 octos」,与既有 `Octos`(App Service 后端)并列而非替换
- `OctosDirect` 走与 `Hermes` / `OpenClaw` 相同的普通 Matrix ID 绑定路径(搜索 / 输入完整 MXID →
  写入 `AgentRegistry`),**不**进入 App Service / BotFather 配置分支
- `register_agent_with_modal_settings` 中「设置 BotFather」的分支条件保持只对 `AgentFramework::Octos`
  生效;`OctosDirect` 注册**不得**写 `bot_settings.botfather_user_id`、不得置 `bot_settings.enabled = true`、
  不得调用 `record_known_bot_user_ids`
- 修复 child-bot 覆盖缺陷:`register_agent_with_modal_settings` 对 `Octos`(appservice)注册,仅当
  `bot_settings.botfather_user_id` 当前等于默认 localpart(`DEFAULT_BOTFATHER_LOCALPART` = "bot")或已等于
  传入 `user_id` 时,才写入 `botfather_user_id`;若已配置了不同的 BotFather 则**不覆盖**,该 bot 仍记入
  `known_bot_user_ids` 与 registry。显式更改 BotFather 仍走 App Service 卡的 BotFather MXID 字段(不受此守卫影响)
- `framework_options()` 的返回集合新增 `OctosDirect`
- `framework_label(OctosDirect)` 返回 "Octos (Direct)"(zh-CN 同为 "Octos (Direct)"),与 `Octos` 的 "Octos"
  不同;`framework_mono(OctosDirect)` 返回不同缩写(如 "OD")
- agent 数量汇总中 `OctosDirect` 计入「direct」类(与 `Hermes` / `OpenClaw` 同类),不计入 App Service 类
- `agent_row_shows_recheck(OctosDirect)` 返回 `false`——OctosDirect 非 appservice,robrix 对它没有可健康复查的 admin 端点(且其部署位置对 robrix 不可见),故不显示 App Service 健康复查行

## Constraints

- 注册 `OctosDirect` agent 不得修改或清空 `known_bot_user_ids` 与 `room_bindings`(App Service 绑定状态)
- 首次(`botfather_user_id` 为默认 localpart 时)注册 `AgentFramework::Octos` 仍写 `botfather_user_id` 并置 `enabled = true`,现有主流程不回退
- 已配置了不同 BotFather 时,再注册一个 `Octos` bot 不得覆盖 `botfather_user_id`(child-bot 守卫)
- 已配置了不同 BotFather 且已有自定义 `octos_service_url` 时,再注册一个 `Octos` child bot 不得把服务地址重置为默认值或传入值
- `register_agent_from_search` 对 `OctosDirect` 保持幂等:重复注册同一 `agent_mxid` 不产生重复条目,不覆盖已存在条目
- `OctosDirect` 选择卡新增的用户可见文案必须通过 i18n key 提供 en / zh-CN 文案
- Agent Lab 注册的 `OctosDirect` 是 agent 身份来源:bot/agent 识别与 bot picker 必须通过 `AgentRegistry ∪ known_bot_user_ids` 看到它,但不得把它写入 App Service raw `known_bot_user_ids`
- 步骤切换(`sync_steps`)时必须把共享滚动体 `body_scroll` 复位到顶部;否则 step1 的 4 张 framework 卡溢出固定高度滚动区后,残留滚动偏移会裁切 step2 顶部的 header 与 Matrix ID 输入框
- Agent Lab 界面内禁止裸 hex 颜色字面量,使用 `RBX_*` 或 `styles.rs` token

## Boundaries

### Allowed Changes
- src/app.rs
- src/home/bot_binding_modal.rs
- src/home/room_screen.rs
- src/settings/agent_settings.rs
- src/settings/agent_add_modal.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json
- specs/task-agent-personal-octos-option.spec.md

### Forbidden
- 不要改动或回退 `AgentFramework::Octos`(App Service / BotFather)的注册与配置流
- 不要为 `OctosDirect` 触发任何 App Service / BotFather 设置(`botfather_user_id` / `enabled` / `known_bot_user_ids`)
- 不要构建 `/invitebot` 命令或任何把 agent 邀请进房的逻辑
- 不要为 OctosDirect 增加 admin API 健康检查 / 管理 UI(robrix 只经 Matrix 与其交互,不关心其部署位置)
- 不要改动 `create_bot_modal` / BotFather 业务流;`bot_binding_modal` 只允许把 `OctosDirect` registry agent 纳入现有 bot picker 集合
- 不要新增 cargo 依赖
- 不要运行 `cargo fmt`

## Out of Scope

- `/invitebot` 客户端动作命令(2b)
- 把 agent 邀请进房间 / 房间绑定
- OctosDirect 的 admin API / 健康检查 / 远程管理(robrix 只经 Matrix 与其交互,其运行机器与位置对 robrix 不可见)
- 运行 / 停止 / 配置 octos 进程本身
- timeline / 房间列表的 badge 渲染改造
- 加密房间(user-account octos 不支持 E2EE)

## Completion Criteria

<!-- lint-ack: bdd-rule-grouping — 单一能力(新增 OctosDirect 选项),扁平结构清晰,无需 Rule 分组 -->

场景: 注册OctosDirect 写入 registry 且 framework 正确
  测试: test_register_agent_octos_direct
  假设 `AppState` 的 agent_registry 不含 "@myagent:example.org"
  当 以 framework `OctosDirect`、display_name "MyAgent" 注册 "@myagent:example.org"
  那么 agent_registry 含 "@myagent:example.org"
  并且 该条目的 framework 为 `OctosDirect`
  并且 该条目的 display_name 为 "MyAgent"

场景: framework 选项集合包含 OctosDirect
  测试: test_framework_options_include_octos_direct
  假设 调用 `framework_options()`
  当 检查返回的可选 framework 列表
  那么 列表包含 `OctosDirect`
  并且 列表仍包含 `Octos`

场景: OctosDirect 的标签与 Octos 不同
  测试: test_framework_label_octos_direct_distinct
  假设 分别对 `Octos` 与 `OctosDirect` 调用 `framework_label`
  当 比较两个标签
  那么 两个标签互不相同
  并且 `OctosDirect` 的标签非空

场景: 注册OctosDirect 不触发 BotFather 设置
  测试: test_register_octos_direct_does_not_touch_botfather
  假设 `BotSettingsState` 的 `botfather_user_id` 为默认 localpart 且 `enabled` 为 false
  当 通过 `register_agent_with_modal_settings` 以 framework `OctosDirect` 注册 "@myagent:example.org"
  那么 `bot_settings.botfather_user_id` 不等于 "@myagent:example.org"
  并且 `bot_settings.enabled` 仍为 false
  并且 `known_bot_user_ids` 不含 "@myagent:example.org"

场景: 首次注册 App Service Octos 仍设置 BotFather(不回退)
  测试: test_register_octos_appservice_first_time_sets_botfather
  假设 `bot_settings.botfather_user_id` 为默认 localpart "bot"
  当 以 framework `Octos` 注册 "@octos:example.org"
  那么 `bot_settings.botfather_user_id` 等于 "@octos:example.org"
  并且 `bot_settings.enabled` 为 true

场景: 注册 child Octos 不覆盖已配置的 BotFather
  测试: test_register_octos_child_does_not_clobber_botfather
  假设 `bot_settings.botfather_user_id` 已配置为 "@octos:example.org"
  当 以 framework `Octos` 注册 child bot "@octos_weather:example.org"
  那么 `bot_settings.botfather_user_id` 仍等于 "@octos:example.org"
  但是 `known_bot_user_ids` 含 "@octos_weather:example.org"
  并且 agent_registry 含 "@octos_weather:example.org"

<!-- lint-ack: verification-metadata-suggestion — 单元测试只验证 AppState 字段保留,不执行网络 I/O -->
场景: 注册 child Octos 不覆盖已配置的 AppService URL
  测试: test_register_octos_child_preserves_existing_appservice_url
  Level: unit
  假设 `bot_settings.botfather_user_id` 已配置为 "@octos:example.org"
  并且 `bot_settings.octos_service_url` 已配置为 "http://10.0.0.5:8010"
  当 以 framework `Octos` 注册 child bot "@octos_weather:example.org" 且传入默认服务地址
  那么 `bot_settings.octos_service_url` 仍等于 "http://10.0.0.5:8010"
  并且 agent_registry 含 "@octos_weather:example.org"

场景: OctosDirect 在汇总中计入 direct 类
  测试: test_octos_direct_counts_as_direct
  假设 agent_registry 含一个 framework 为 `OctosDirect` 的条目
  当 计算 framework 汇总
  那么 direct 类计数包含该 `OctosDirect` 条目
  并且 App Service(octos)类计数不包含该条目

场景: OctosDirect agent 出现在房间 bot 选择器
  测试: room_bot_picker_includes_registered_octos_direct_agents
  假设 agent_registry 含一个 framework 为 `OctosDirect` 的 "@myagent:matrix.palpo.im"
  当 计算房间 bot picker 的可选 bot 集合
  那么 可选集合含 "@myagent:matrix.palpo.im"
  但是 raw `bot_settings.known_bot_user_ids` 不含 "@myagent:matrix.palpo.im"

场景: DM bot binding 检测使用 registry 合并后的 known-bots
  测试: test_detected_bot_binding_uses_registry_augmented_known_bots
  假设 agent_registry 含一个 framework 为 `OctosDirect` 的 agent
  当 DM 成员检测需要判断候选 bot
  那么 检测逻辑使用 `timeline_known_bot_user_ids(app_state)`
  但是 不只读取 raw `app_state.bot_settings.known_bot_user_ids()`

场景: OctosDirect 不显示 App Service 健康复查行
  测试: test_octos_direct_no_appservice_recheck
  假设 一个 framework 为 `OctosDirect` 的 agent 行
  当 调用 `agent_row_shows_recheck(OctosDirect)`
  那么 返回值为 false
  但是 `agent_row_shows_recheck(Octos)` 返回 true

场景: 重复注册同一 OctosDirect mxid 保持幂等
  测试: test_register_octos_direct_idempotent
  假设 agent_registry 已含以 framework `OctosDirect` 注册的 "@myagent:example.org"
  当 再次以 framework `OctosDirect` 注册同一 "@myagent:example.org"
  那么 agent_registry 中该 mxid 仅有 1 个条目
  并且 该条目的 framework 仍为 `OctosDirect`

场景: OctosDirect 绑定拒绝 localpart 输入
  测试: test_octos_direct_binding_rejects_localpart
  假设 为 `OctosDirect` 绑定提供的标识是 localpart "myagent" 而非完整 Matrix ID
  当 调用 `parse_agent_user_id`
  那么 返回错误,提示需要完整 Matrix user ID
  但是 agent_registry 不新增任何条目

场景: OctosDirect 选择卡文案使用 i18n key
  测试: test_octos_direct_card_text_uses_i18n_keys
  假设 app language 为 English 或 zh-CN
  当 读取 OctosDirect framework 展示文案
  那么 卡片名称 / tag / blurb 均从 `settings.labs.agents.framework.octos_direct.*` key 读取
  并且 en / zh-CN 资源文件均包含这些 key

场景: 步骤切换时复位滚动到顶部
  测试: test_step_switch_resets_body_scroll_to_top
  假设 读取 `agent_add_modal.rs` 的 `sync_steps` 源码
  当 检查步骤切换逻辑
  那么 `sync_steps` 函数体同时出现 `body_scroll` 与 `set_scroll_pos`
