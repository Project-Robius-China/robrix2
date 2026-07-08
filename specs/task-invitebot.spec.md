spec: task
name: "/invitebot — 快速邀请已注册 agent 进房间"
inherits: project
tags: [bot, agent, slash-command, invite, ui]
estimate: 0.5d
---

## Intent

为消息输入框新增 `/invitebot` slash 命令：在任何拥有邀请权限的房间里，用户
输入 `/invitebot` 并选中后，弹出一个内联 bot 选择列表（数据来自全局
AgentRegistry，排除已在房间内的成员），点选某个 bot 即立刻发出 Matrix 邀请。
全程两次点击、无需输入 Matrix ID、无需按 Enter。与现有 BotFather 管理命令
不同，`/invitebot` 是纯客户端命令——它绝不作为消息文本发送，而是触发本地
`MatrixRequest::InviteUser` 请求。

## Decisions

- **命令定义**：`/invitebot` 独立于 `MANAGEMENT_DM_SLASH_COMMANDS` /
  `MANAGEMENT_ROOM_SLASH_COMMANDS` 目录（那些是发给 BotFather 的服务端命令，
  受 `is_management_bot_room` 门控）。`/invitebot` 定义为独立常量，门控条件
  只有一个：`RoomScreenProps.can_invite == true`（复用
  `UserPowerLevels::can_invite`）。AppService 开关、registry 是否为空均不影
  响命令的出现。
- **两段式选择器**：`MentionableTextInput` 的 `PopupMode` 新增 `BotInvite`
  变体。用户从 slash popup 选中 `/invitebot` 时，不走 `needs_args` 插入路径
  也不走 `emit_primary_submit_action` 发送路径，而是：清除输入框中的
  `/invitebot` 触发文本 → 切换到 `PopupMode::BotInvite` → 弹出 bot 列表。
- **数据源**：`room_screen` 在 draw 时计算
  `RoomScreenProps.invitable_agents: Vec<InvitableAgent>`（字段：`user_id`、
  `display_name`），取值 = `AgentRegistry.agent_user_ids()` 减去当前房间成员
  （含已被邀请者）。过滤逻辑抽成纯函数 `filter_invitable_agents`，可单元测试。
- **空列表提示**：registry 为空、或所有已知 bot 都已在房间时，选择器显示一条
  不可选提示项（复用现有 `add_unselectable_item` 机制），文案走 i18n key
  `slash_command.invitebot.empty_hint`（引导去 Settings → Agents 注册）与
  `slash_command.invitebot.all_present_hint`。
- **行渲染**：bot 列表行复用现有 `user_list_item` 模板（显示名 + MXID），
  不新建 DSL 模板。
- **邀请派发**：选中 bot 后 widget 发出
  `MentionableTextInputAction::InviteBotSelected { room_id, user_id }`，由
  `room_screen` 转换为 `submit_async_request(MatrixRequest::InviteUser)`。
- **结果反馈**：复用现有 `InviteResultAction::Sent / Failed` → popup
  notification 管线（`room_screen.rs` 已处理），本任务不新增通知代码，避免
  重复弹窗。
- **i18n**：新增 key `slash_command.invitebot.description`、
  `slash_command.invitebot.empty_hint`、
  `slash_command.invitebot.all_present_hint`，中英文（`resources/i18n/`
  下所有 locale 文件）都要。
- **命令识别语义（评审修订）**：`is_invitebot_command` 判定为「trim 后整串
  与 `/invitebot` 做 ASCII 大小写不敏感的精确相等」。裸命令（含大小写变体、
  首尾空白）在一切提交路径（Return、Cmd/Ctrl+Return、发送按钮）都转入选择
  器且不发送；带尾随文字的输入（如 "/invitebot is broken ..."）视为普通消
  息正常发送，绝不清空用户已编写的内容。
- **提交路径全覆盖**：除 widget 内 Return 拦截（新增 key-focus 门控，避免
  无焦点实例劫持他处 Enter）外，`room_input_bar` 的发送路径（发送按钮与
  Returned）对裸 `/invitebot` 同样转入选择器（经
  `MentionableTextInputRef::open_bot_invite_picker`）。
- **编辑面板豁免**：`MentionableTextInput` 新增 `#[live] disable_invite_commands`
  （默认 false），`editing_pane` 的实例置 true——消息编辑上下文不提供
  `/invitebot`，避免清空编辑中的内容。
- **成员未加载门控**：`/invitebot` 仅在 `room_members` 已加载
  （`is_some()`）时提供，避免把已在房的 bot 误列为可邀请。
- **选择器关闭途径（评审修订）**：选中、ESC、真实打字之外，新增
  「FingerDown 落在弹窗与输入框区域之外即关闭」（双端一致的点击外部关闭，
  不依赖焦点事件时序），以及「scope 房间切换时关闭任何活动弹窗」（防止
  pinned 选择器带旧房候选漂进新房间）。
- **幽灵 Changed 过滤上移（评审修订）**：新鲜度检查（action 携带文本 ==
  输入框当前文本）从 BotInvite 分支上移到 `TextInputAction::Changed` 的统
  一分发点，所有弹窗模式一体防护（Android 程序化 set_text 的迟到回声也会
  威胁 @mention 插入的追踪一致性）。
- **邀请派发定址（评审修订）**：`InviteBotSelected` 改用
  `cx.widget_action(room_screen_widget_uid, ...)` 定址派发，结构上保证同
  房多 RoomScreen（主时间线+线程）时恰好一次处理；处理时乐观写入
  `pending_invited_users`，杜绝网络往返窗口内的重复邀请。
- **候选计算惰性化（评审修订）**：agent 候选列表不再在 build_room_screen_props
  （每事件/每帧热路径）中计算；props 仅保留 `can_invite` 与轻量
  `pending_invited_users`，选择器打开时经 `AgentRegistry::agents()`（新增
  的 O(n) 迭代器）+ props 中的 `room_members` 现算。显示名回退与 Agent Lab
  对齐（空白名过滤后回退 localpart）；`InvitableAgent` 携带 avatar MXC，
  行渲染与 @mention 列表同用真实头像（文字缩写兜底）。
- **平台验收**：邀请端到端链路（选中 → 邀请落地 → 通知显示）由用户在
  macOS 与 Android 上手动测试确认；本 spec 的场景只绑定纯函数单元测试。
- **Android 触摸选中的焦点豁免（popup pin）**：Android 上点击弹窗项会让
  文本输入框失焦，且焦点舞蹈跨越多帧（tap 夺焦 → draw 还焦 → IME 再夺），
  产生多次 `KeyFocusLost`；`CommandTextInput` 的 `KeyFocusLost → hide_popup`
  会把选中后刚重开的 bot 选择器关闭（桌面鼠标点击不触发失焦，故 mac 正
  常）。修复：`CommandTextInput` 新增 `keep_popup_open_on_focus_loss` 开
  关，`open_bot_invite_picker` 在选择器存活期间置 true（`close_mention_popup`
  与 mention/slash 列表重建时置 false），失焦 hide 同时豁免
  `is_text_input_focus_pending` 窗口。选择器的关闭途径保持：选中、ESC、
  打字。常态（开关 false）行为不变；由 Android 手动测试验收（logcat 已
  验证第一版单帧豁免不足，多帧失焦为实测事实）。

## Boundaries

### Allowed Changes
- src/shared/mentionable_text_input.rs
- src/shared/command_text_input.rs
- src/home/room_screen.rs
- src/home/editing_pane.rs
- src/room/room_input_bar.rs
- src/app.rs
- src/i18n.rs
- resources/i18n/**
- specs/task-invitebot.spec.md

### Forbidden
- 不要新增 cargo 依赖
- 不要把 `/invitebot` 加入 `MANAGEMENT_DM_SLASH_COMMANDS` 或
  `MANAGEMENT_ROOM_SLASH_COMMANDS`
- 不要把 `/invitebot` 作为消息文本发送（不得经过
  `emit_primary_submit_action` 或消息发送路径）
- 不要修改 `invite_modal.rs` 或复用其 Modal UI
- 不要修改 `MatrixRequest::InviteUser` 在 `sliding_sync.rs` 中的处理逻辑

## Completion Criteria

场景: 有邀请权限时提供 /invitebot 命令
  测试: test_invitebot_offered_when_can_invite
  假设 用户在房间内拥有邀请权限 (can_invite = true)
  当 slash 命令候选按前缀 "inv" 过滤
  那么 候选列表包含 "/invitebot"

场景: 无邀请权限时隐藏 /invitebot 命令
  测试: test_invitebot_hidden_without_can_invite
  假设 用户在房间内没有邀请权限 (can_invite = false)
  当 slash 命令候选按前缀 "inv" 过滤
  那么 候选列表不包含 "/invitebot"

场景: 选择器只列出未入房的注册 agent
  测试: test_invitable_agents_excludes_room_members
  假设 AgentRegistry 包含以下 agent:
    | user_id           | display_name |
    | @octos:server     | Octos        |
    | @hermes:server    | Hermes       |
  并且 房间成员包含 "@octos:server"
  当 计算可邀请 agent 列表
  那么 列表恰好包含 "@hermes:server"
  但是 列表不包含 "@octos:server"

场景: registry 为空时返回空列表
  测试: test_invitable_agents_empty_registry
  假设 AgentRegistry 中没有任何 agent
  当 计算可邀请 agent 列表
  那么 列表为空

场景: 所有已知 bot 均已在房间时返回空列表
  测试: test_invitable_agents_all_in_room
  假设 AgentRegistry 包含 "@octos:server"
  并且 房间成员包含 "@octos:server"
  当 计算可邀请 agent 列表
  那么 列表为空

场景: /invitebot 不被归类为可提交的消息命令
  测试: test_invitebot_never_classified_for_message_submission
  假设 输入文本为 "/invitebot"
  当 调用 classify_known_slash_command_for_submission 归类
  那么 返回 None
  并且 is_invitebot_command("/invitebot") 返回 true
  并且 大小写变体 "/INVITEBOT" 与首尾空白变体 "  /invitebot  " 也返回 true

场景: /invitebot 前缀过滤大小写不敏感
  测试: test_invitebot_matches_case_insensitive_prefix
  假设 用户在房间内拥有邀请权限 (can_invite = true)
  当 slash 命令候选按前缀 "INV" 过滤
  那么 候选列表包含 "/invitebot"

场景: 相似或带尾随文字的输入不会误判为 /invitebot
  测试: test_is_invitebot_command_rejects_lookalikes
  假设 输入文本为 "/invitebotx"
  当 调用 is_invitebot_command 判断
  那么 返回 false
  并且 输入文本 "/invite" 与空字符串 "" 也返回 false
  并且 带尾随文字的 "/invitebot is broken" 也返回 false（作为普通消息发送，不清空用户内容）
  并且 "hello /invitebot" 也返回 false

场景: 两个 locale 文件都包含 invitebot i18n key
  测试: test_invitebot_i18n_keys_exist_in_all_locales
  假设 en 与 zh-CN 两套字典已加载
  当 用 tr_key 解析 "slash_command.invitebot.description"、"slash_command.invitebot.empty_hint" 和 "slash_command.invitebot.all_present_hint"
  那么 每个 key 在两种语言下都返回非 key 本身的翻译文案

## Out of Scope

- 邀请非 bot 的普通用户（现有 invite modal 已覆盖）
- `/invitebot @bot` 参数语法（本任务只做两段式选择器）
- registry 只有一个 bot 时跳过列表直接邀请
- BotFather 服务端管理命令（`/createbot` 等）的任何行为变更
- 房间内 bot pill / badge 的更新（已有机制自动生效）
- 邀请结果通知文案的调整（复用现有 InviteResultAction 管线）
