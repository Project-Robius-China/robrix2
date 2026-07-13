spec: task
name: "Agent Message Response Cards — Timeline UI"
inherits: project
tags: [bot, agent, ui, timeline, octos, design-tokens]
depends: [task-tg-bot-timeline-cards, task-tg-bot-action-buttons, task-tg-bot-approval-request]
estimate: 3d
---

## Intent

为 Robrix2 房间时间线中的 Octos/agent 回复设计并落地四种消息返回 UI 样式：流式指示器、
AgentMessageCard(§4.5)、ApprovalCard(§4.6)、CodeOutputCard(§4.7)。全部只消费 Octos 现有输出，
不改路由、输入区与后端协议。设计详见
`docs/superpowers/specs/2026-07-10-agent-message-response-cards.md`。

## Decisions

- 作用范围: 只重构房间时间线中 bot/agent-authored 消息的呈现，不改普通用户消息样式
- 协调模型: 引入 `agent_render_state`（`AgentCardKind` { PlainMessage / BotTextCard / AgentCard }）
  统一决定单条消息渲染成哪种卡，避免可见性互相打架（visual spec §5.4）
- Token: 新样式一律用 `RBX_*` 设计 token；`COLOR_BOT_*` 随各卡按面迁移，不做一次性大改
- CodeOutputCard: 只把「围栏代码块 widget」（highlighted 路径）改深底；行内 `code` 与
  CJK-in-fence 的 plain 回退本轮保持浅底
- ApprovalCard: 就地把 `approval_request_view` recipe 改成琥珀 §4.6 版（visual spec §4.1
  recipe；此 fork 无法可靠向派生模板追加子节点，新建自定义 widget 风险更高、无收益——
  Approve/Reject 按钮本就在兄弟 `action_button_row`）。Message / CondensedMessage 两处同改
- AgentMessageCard: v1 不新建单独 `agent_message_card`（Octos 无法发出触发信号 → 永不显示、不可测），
  改为**就地把 bot 文本卡升级成 agent 卡**：把解析出的 status 文案渲成「单个 active StepChip」
  （§4.10 teal 药丸）、body 卡迁 `RBX_*` 并加 4px `RBX_ACCENT` 左边条、`bot_badge` 迁 `RBX_ACCENT` 且文案改 `APP`。
  多步链条依赖后端 `org.octos.steps`；6×6 绿点头像（动 profile 头像列）留 ②.2
- 流式指示器: 复用 makepad 内置自转 `LoadingSpinner`（teal，置于 `bot_message_card` 顶部的
  accent-soft 药丸内），由 `is_live` 驱动可见；流式帧循环扩展为 `is_live` 时持续调度帧 + 重绘
  （不重填），使 spinner 在 full-snapshot markdown 模式也平滑转动；移除正文尾部 `●` glyph
- 动态 widget 状态: 一律 Animator + shader 实例变量，禁用 `script_apply_eval!`（Pitfall #40）

## Boundaries

### Allowed Changes
- src/shared/design_tokens.rs
- src/home/room_screen.rs
- src/home/streaming_animation.rs
- docs/superpowers/specs/2026-07-10-agent-message-response-cards.md
- specs/task-agent-message-response-cards.spec.md

### Forbidden
- 不改 Octos 后端输出格式 / 协议
- 不重设计输入区、mention/slash 路由、`bot_menu_button`
- 不把普通用户消息改成新卡样式
- 不新增 cargo 依赖
- 不硬编码 hex（先在 `design_tokens.rs` 加 token 再引用）
- 不用 `cargo fmt`

## Out of Scope

- inline keyboard 之外的新交互
- Poll 卡 / 位置图缩略 / 图片下载进度
- 翻译 footer 的翻译逻辑本身（依赖 realtime-translation 特性）
- 后端 `org.octos.steps` / 结构化 schema 迁移

## Completion Criteria

Scenario: Render-state classifier maps senders to card kinds
  Test: test_compute_agent_render_state_maps_senders
  Given a message render-state classifier
  When a non-bot sender is classified
  Then the card kind is PlainMessage
  And a bot sender without a structured agent signal is BotTextCard
  And a bot sender with a structured agent signal is AgentCard

Scenario: Fenced bot code renders on the dark CodeOutputCard panel
  Test: manual
  Given a bot-authored reply containing a fenced code block
  When the message item is populated
  Then the code block renders on the dark RBX_CODE_BG panel
  And keyword/string/comment syntax colors are readable on the dark surface
  And inline code and body text remain readable

Scenario: Dark code tokens exist in the design-token layer
  Test: build
  Given the CodeOutputCard uses dark syntax colors
  Then RBX_CODE_BORDER/NUMBER/FUNCTION/TYPE/ERROR/WARNING/PUNCT are defined in design_tokens.rs
  And room_screen.rs references them (no new hardcoded syntax hex in the widget)

Scenario: Approval request renders as an amber ApprovalCard with a Pending badge
  Test: manual
  Given an org.octos.approval_request event
  When the message item is populated
  Then the card container is amber (RBX_WARNING_BG + RBX_WARNING_FG border)
  And a Pending badge is shown
  And Approve(success)/Reject(danger) buttons send the existing responses
  And an unauthorized user sees disabled buttons

Scenario: Streaming reply shows animated dots and no stray cursor glyph
  Test: manual
  Given a bot reply that is still streaming
  When the message updates
  Then an animated indicator is shown in the status strip
  And no trailing ● glyph appears in the body
  And the done state hides the indicator and reveals the footer

Scenario: Ordinary user and plain bot messages are unaffected
  Test: manual
  Given a regular user message and a plain (non-structured) bot message
  When both are populated
  Then the user message uses the ordinary timeline layout
  And the plain bot message keeps the existing 3-layer bot card
