spec: task
name: "Telegram Bot UI Alignment — Phase 4: Bot Timeline Cards"
inherits: project
tags: [bot, ui, telegram-parity, timeline]
depends: [task-tg-bot-ui-alignment, task-tg-bot-mention-reply-first]
estimate: 1d
---

## Intent

将 Robrix2 的 bot 时间线消息继续向 Telegram bot chat 的阅读体验对齐。当前 mention/reply-first 交互已经成立，但 bot 回复在视觉上仍然过于扁平：生成状态、provider/model 行、token/耗时统计和真正的回复正文混在一起，用户很难一眼抓住 bot 的答案主体。

本任务要求把 bot-authored 文本消息重构成更清楚的时间线层次：正文作为主卡片，状态行作为轻量顶部条，provider/model 与 token/耗时作为弱化 footer，同时不改变当前输入区、路由模型和后端协议。

## Decisions

- 作用范围: 只重构房间时间线中的 bot-authored `Text` / `Notice` 消息，不改普通用户消息样式
- 呈现模型: bot 时间线消息拆成 `status strip`、`body card`、`metadata footer` 三层；正文始终是视觉中心
- 数据来源: 前端仅对 Octos 现有文本格式做结构化提取，不要求后端新增字段
- 提取规则: 仅对 bot sender 启用结构化提取；无法匹配的内容必须安全回退为普通正文渲染
- provider 行: 识别 `via provider (model)` 这类前导元信息行，并移出正文主体
- footer 行: 识别 Octos 现有尾注格式 `_model · X in · Y out · Zs_`，并渲染为弱化 footer
- 状态行: 当 bot 消息顶部存在短状态行且其后紧跟 provider 行时，该状态行渲染为 `status strip`，不得继续作为正文第一段
- 正文渲染: 结构化提取后的正文仍通过现有 `HtmlOrPlaintext` 渲染，不改 Markdown / HTML 支持模型
- 兼容性: 普通用户消息、服务器 notice、图片/文件消息、输入区、slash command、replying preview 均不在本任务内重设计

## Boundaries

### Allowed Changes
- src/home/room_screen.rs
- src/shared/html_or_plaintext.rs
- src/home/edited_indicator.rs
- specs/task-tg-bot-timeline-cards.spec.md
- docs/superpowers/specs/2026-04-12-bot-timeline-card-design.md
- docs/superpowers/plans/2026-04-12-tg-bot-timeline-cards-plan.md

### Forbidden
- 不要修改 Octos 后端输出格式
- 不要重新设计输入区
- 不要修改 mention/reply-first 路由行为
- 不要新增 cargo 依赖
- 不要把普通用户消息一起改成新卡片样式

## Out of Scope

- inline keyboard / bot action buttons
- `/` 命令菜单改版
- BotFather 面板或 bot profile 页面
- 显示名式 `@octos` mention 容错
- 图片、文件、语音等非文本 bot 消息样式重设计

## Completion Criteria

Scenario: Bot timeline parser extracts status provider body and footer
  Test: test_parse_bot_timeline_layers_extracts_status_provider_body_and_footer
  Given a bot-authored text message whose status line is "施法中"
  And the message content has lines:
    | line |
    | 施法中 |
    | via moonshot@api (kimi-k2.5) |
    |  |
    | 你好！我是 **Alex** |
    |  |
    | _moonshot@api/kimi-k2.5 · 5.3K in · 330 out · 6s_ |
  When the timeline parses bot message layers
  Then the status strip is "施法中"
  And the provider line is "via moonshot@api (kimi-k2.5)"
  And the body is "你好！我是 **Alex**"
  And the footer line is "_moonshot@api/kimi-k2.5 · 5.3K in · 330 out · 6s_"

Scenario: Parser safely falls back for unmatched bot text
  Test: test_parse_bot_timeline_layers_falls_back_for_unmatched_bot_text
  Given a bot-authored text message whose content does not match the known Octos metadata layout
  When the timeline parses bot message layers
  Then no status strip is extracted
  And no provider line is extracted
  And no footer line is extracted
  And the full original content remains in the body

Scenario: Regular user text is not structurally re-parsed as a bot card
  Test: test_parse_bot_timeline_layers_ignores_regular_user_messages
  Given a regular user message containing lines that resemble "via moonshot@api (kimi-k2.5)"
  When the timeline parses message layers
  Then the message is treated as ordinary user content
  And no bot-specific status strip or footer is produced

Scenario: Malformed bot metadata prefers safe body fallback over partial extraction
  Test: test_parse_bot_timeline_layers_prefers_safe_fallback_for_malformed_metadata
  Given a bot-authored text message whose first line looks like a status line
  And the following lines do not form a valid provider-plus-footer layout
  When the timeline parses bot message layers
  Then the parser does not emit a partial structured card
  And the unmatched metadata lines remain in the body text
  And the message still renders as readable bot content

Scenario: Invalid bot metadata input does not panic and falls back to plain body
  Test: test_parse_bot_timeline_layers_invalid_metadata_does_not_panic
  Given a bot-authored text message with corrupted or partially truncated metadata lines
  When the timeline parses bot message layers
  Then parsing completes without panic
  And no invalid status strip or footer is emitted
  And the original message content remains renderable as body text

Scenario: Bot text message renders a distinct body card
  Test: test_bot_timeline_card_visible_for_bot_text_message
  Given a bot-authored text message in the room timeline
  When the message item is populated
  Then the bot body card is visible
  And the main reply body is rendered inside the card surface

Scenario: Regular user text message does not render the bot card surface
  Test: test_bot_timeline_card_hidden_for_regular_user_message
  Given a regular user-authored text message in the room timeline
  When the message item is populated
  Then the bot body card is not shown
  And the message continues using the ordinary timeline text layout

Scenario: Bot status strip is shown above the main reply body
  Test: test_bot_status_strip_renders_above_body_and_not_inside_body
  Given a bot-authored text message whose top metadata contains a short status line followed by a provider line
  When the message item is populated
  Then the status strip is rendered above the body card
  And the body text no longer starts with that status line

Scenario: Provider and usage metadata render as subdued footer below the body
  Test: test_bot_metadata_footer_renders_below_body
  Given a bot-authored text message with a provider line and an annotation footer
  When the message item is populated
  Then the provider/model line is shown below the body card
  And the token/latency annotation is shown in the footer
  And the footer text style is weaker than the main body text style

Scenario: Structured bot body still renders through HtmlOrPlaintext
  Test: test_bot_timeline_card_body_uses_html_or_plaintext_rendering
  Given a bot-authored rich-text message whose structured body contains Markdown or Matrix HTML formatting
  When the message item is populated
  Then the extracted main body is rendered through `HtmlOrPlaintext`
  And links, emphasis, and line breaks still render the same way as before card styling

Scenario: Bot card layout preserves reply preview and condensed timeline behavior
  Test: test_bot_timeline_card_preserves_reply_preview_and_condensed_layout
  Given a bot-authored reply message and a later condensed bot-authored message
  When the timeline populates both items
  Then the reply preview remains visible above the bot card on the reply item
  And the condensed message still renders its bot body card without restoring a full profile row
