spec: task
name: "Telegram Bot UI Alignment — Phase 4c: Bot Message Action Buttons"
inherits: project
tags: [bot, ui, telegram-parity, action-buttons, inline-keyboard]
depends: [task-tg-bot-mention-reply-first]
estimate: 2d
---

## Intent

让 bot 的消息可以附带**可点击的 action 按钮**，呈现在消息下方（Telegram
inline keyboard 风格），用户点击按钮后直接触发相应动作，不需要手动打字。
这是真实场景驱动的功能：当前 PPT 生成失败、文件重发、命令确认等场景下，
bot 只能用文字说 "please try again"，用户必须再手动 @mention 重新说
"regenerate"，摩擦很大。

本任务定义一个**最小可用**的 action button 协议 + UI：Matrix 事件里加
一个自定义 field 承载按钮结构，Robrix2 在消息下方渲染对应按钮，用户点击
后把选择作为一条新的 Matrix 消息发回给 bot（带自定义 field 标识"这是一
个 action response"，而非普通消息）。底层复用现有 `Splash` 渲染框架，不
引入独立的 widget 树。

**故意不做**：reply keyboard（替换输入区的按钮组）、键盘行列嵌套布局、
inline 按钮发起 URL 跳转等 TG 高级特性——这些是后续增量。

## Decisions

- **传输协议**：bot 发送消息时，在 `m.room.message` 事件的 content 里加
  一个自定义 field：
  ```json
  {
    "msgtype": "m.text",
    "body": "...",
    "org.octos.actions": [
      { "id": "retry_pptx", "label": "Regenerate PPT", "style": "primary" },
      { "id": "cancel",      "label": "Cancel",         "style": "secondary" }
    ]
  }
  ```
- **字段定义**：`org.octos.actions` 是一个 JSON 数组。每个元素是一个
  object，字段：
  - `id: string` — 必填。机器可读的 action 标识，最长 64 字节。
  - `label: string` — 必填。用户看到的按钮文字，最长 32 字符。
  - `style: "primary" | "secondary" | "danger"` — 可选，默认 `secondary`。
  影响按钮视觉样式。
- **最多按钮数**：单条消息最多 6 个按钮。超出的按钮丢弃并在日志里记录
  warning，不影响消息主体显示。
- **渲染位置**：按钮渲染在 `Message` 模板的 `content.message`（文本/html
  部分）之后，`link_preview_view` 之前。这是新增 DSL 节点
  `action_buttons := View { ... }`，与现有 `splash_card` 并列，不嵌套。
- **渲染策略（开放）**：action_buttons view 根据事件中的 `org.octos.actions`
  field 动态生成按钮。实现方可以选择：
  - **方案 A：Splash eval** — 生成 Splash code 字符串，通过
    `Splash::set_text()` 注入。复用现有 card 渲染基础设施，最少新代码。
  - **方案 B：原生 Makepad button row** — 在 `action_buttons` View 里
    预定义一个 PortalList/动态子节点模板，Rust 侧直接操作
    `WidgetRef`。比 Splash 更可控，click 事件通过标准 `ButtonAction`
    回流。
  实现前需要做 spike 验证哪一个路径的 click-to-Rust 桥接更可靠。**不
  强制绑定任何一个方案**——spec 只定义协议和 scenario，允许实现方根据
  试水结果选择。
- **按钮样式**：primary 使用 `RobrixPositiveIconButton`，secondary
  使用默认 `Button`，danger 使用 `RobrixNegativeIconButton`。均使用项目
  里已有的 button widget，不引入新样式。
- **点击响应**：用户点击按钮后，Robrix 发送一条新的 Matrix 消息回到同
  一房间：
  ```json
  {
    "msgtype": "m.text",
    "body": "[Action: Regenerate PPT]",
    "org.octos.action_response": {
      "action_id": "retry_pptx",
      "source_event_id": "$original_event_id"
    },
    "m.relates_to": {
      "event_id": "$original_event_id",
      "rel_type": "m.in_reply_to"
    }
  }
  ```
  body 是用户可读的 fallback（在不识别 `org.octos.action_response` 的
  Matrix 客户端也能看到一个 reply 消息）。
- **路由（修正）**：action response 消息**不走** input bar 当前的
  reply-to / mention 状态解析路径——那个路径依赖用户"正在回复某条消息"
  的 UI 状态，而 action button 的点击是独立于 input bar 的触发点，用户
  此刻并不一定在 reply 模式。正确的实现是：
  - action button 的 click handler **直接以 `original_sender`（即挂
    button 的那条事件的 sender）作为 one-shot `target_user_id`** 构造
    outgoing message
  - **复用** input bar 现有的消息构造 helper（比如
    `build_outgoing_message_content()` 或等价物）来生成 body/custom
    fields/`m.relates_to` 等字段
  - **bypass** input bar 的 `ResolvedTarget`/`ExplicitOverride` 状态——
    action response 的 routing 信号完全由 button 的上下文决定，不受用
    户当前输入框状态影响
  - 结果：outgoing message 的 `target_user_id = original_sender`，
    `explicit_room = false`，`m.relates_to.rel_type = "m.in_reply_to"`
    指向 original event，并带上 `org.octos.action_response` 字段。
  这条 decision 让 action button 点击与 input bar 解耦，避免用户
  "一边 reply 另一条消息一边点 button"时的状态冲突。
- **按钮禁用状态**：action button 默认在点击后**立即禁用**所有按钮，避免
  重复提交。禁用状态通过本地 UI state 维护，不修改原事件。
- **本地选择回显**：用户点击某个 action button 后，Robrix 必须在原消息
  的 action 区立即回显“已点击的是哪个按钮”。v1 的回显规则是：
  - 只保留被点击的那个按钮
  - 该按钮文案前加 `✓`
  - 该按钮保持禁用状态
  - 其它按钮本地隐藏
  如果 action response 发送失败，Robrix 必须恢复原始按钮组，允许用户重试。
- **原事件不可变**：action buttons 是 client-side 附加的交互层，**不**通过
  Matrix edit (`m.replace`) 机制更新原消息。如果 bot 想更新按钮状态，需
  要发送一条新消息。
- **与 Splash card 的关系**：Phase 4c 的 action buttons 与 Phase 3 的
  Splash card 是**正交且可组合**的。一条消息可以同时有 `org.octos.splash_card`
  和 `org.octos.actions`——splash card 渲染主要视觉内容，action buttons
  在下方渲染交互按钮。两个 field 独立解析，互不干扰。
- **i18n**：按钮文字由 bot 提供（`label` 字段），Robrix 侧不负责翻译。
  但 accessibility fallback（比如 "Action button:" prefix）走 Robrix i18n。

## Boundaries

### Allowed Changes
- src/home/room_screen.rs
- src/shared/mentionable_text_input.rs (only if the existing message-
  construction helper needs a new entry point for action-button callers;
  must NOT change the input bar's reply/mention state logic)
- src/sliding_sync.rs (only if a new send-path variant is needed to carry
  the `target_user_id` + `org.octos.action_response` custom fields without
  routing through input-bar state)
- resources/i18n/en.json
- resources/i18n/zh-CN.json

### Forbidden
- 不要修改 `Message` 模板的主体布局（avatar 位置、消息气泡宽度等）
- 不要引入 reply keyboard（替换输入区的按钮组）
- 不要解析或执行按钮 URL（所有 action 都通过 `action_id` 回传给 bot）
- 不要添加新的 cargo 依赖
- 不要修改 Octos 后端（按钮协议的消费端由 Octos 的 agent tool 层处理，
  本 spec 只定义 Matrix 事件协议和 Robrix 侧渲染/响应）
- 不要通过 `m.replace` 更新原事件的按钮（禁用状态是纯客户端的）
- 不要支持嵌套/分行布局——所有按钮横排，超出宽度自动 wrap

## Out of Scope

- Reply keyboard（替换输入区的键盘）
- 按钮组分行/分组布局控制
- URL 跳转按钮
- Bot 通过 edit 更新按钮状态
- 按钮上的图标/emoji 渲染（只支持纯文字 label）
- Server-side 按钮状态持久化

## Completion Criteria

Scenario: Message with action buttons renders buttons below message body
  Test: test_action_buttons_render_below_message
  Given a Matrix event with `msgtype: "m.text"` and body "PPT generation failed"
  And the event content contains `org.octos.actions` with two entries:
    | id          | label           | style     |
    | retry_pptx  | Regenerate PPT  | primary   |
    | cancel      | Cancel          | secondary |
  When the message is rendered in the room timeline
  Then the message body "PPT generation failed" is visible
  And a button labeled "Regenerate PPT" is visible below the body
  And a button labeled "Cancel" is visible to the right of "Regenerate PPT"
  And the "Regenerate PPT" button uses the primary style
  And the "Cancel" button uses the secondary style

Scenario: Plain text message without actions renders normally
  Test: test_plain_message_without_actions
  Given a Matrix event with `msgtype: "m.text"` and body "hello"
  And the event content does NOT contain `org.octos.actions`
  When the message is rendered in the room timeline
  Then no action buttons widget is visible
  And the message renders exactly as before Phase 4c

Scenario: Clicking an action button sends action response message
  Test: test_click_action_button_sends_response
  Given a rendered bot message with event_id "$orig123" and an action button "retry_pptx" / "Regenerate PPT"
  When the user clicks the "Regenerate PPT" button
  Then a new Matrix message is sent to the same room
  And the outgoing message body is "[Action: Regenerate PPT]"
  And the outgoing message content has `org.octos.action_response.action_id = "retry_pptx"`
  And the outgoing message content has `org.octos.action_response.source_event_id = "$orig123"`
  And the outgoing message has `m.relates_to.rel_type = "m.in_reply_to"` pointing to "$orig123"

Scenario: Clicked button is disabled locally to prevent double submission
  Test: test_clicked_button_disabled_locally
  Given a rendered bot message with two action buttons "retry_pptx" and "cancel"
  When the user clicks "Regenerate PPT"
  Then both the "Regenerate PPT" and "Cancel" buttons become disabled
  And clicking them again does not send additional messages
  And the original event's `org.octos.actions` field is unchanged (no m.replace emitted)

Scenario: Clicked button remains visible as the selected local acknowledgement
  Test: test_clicked_action_button_collapses_to_selected_acknowledgement
  Given a rendered bot message with two action buttons "retry_pptx" and "cancel"
  When the user clicks "Regenerate PPT"
  Then 只保留被点击的那个按钮
  And the remaining button label is "✓ Regenerate PPT"
  And the remaining button uses the original clicked button style
  And 该按钮保持禁用状态
  And 其它按钮本地隐藏

Scenario: Action response routes to original sender via one-shot target (bypassing input bar state)
  Test: test_action_response_routes_to_original_sender
  Given the original bot message sender is "@octosbot_weather:127.0.0.1:8128"
  And the user is currently replying to a DIFFERENT message from "@alice:127.0.0.1:8128" in the input bar
  And the user clicks an action button on the bot message from `@octosbot_weather`
  When the click handler builds the outgoing action response
  Then the outgoing message `target_user_id = "@octosbot_weather:127.0.0.1:8128"` (the bot, not Alice)
  And `explicit_room` is false
  And the outgoing message has `m.relates_to.rel_type = "m.in_reply_to"` pointing to the bot event
  And the input bar's current `ResolvedTarget`/reply state is NOT consulted
  And the input bar's reply state to Alice is preserved (the click does not clear it)

Scenario: Message with more than 6 buttons drops extras and logs warning
  Test: test_too_many_buttons_truncated
  Given a Matrix event with `org.octos.actions` containing 8 action entries
  When the message is rendered
  Then only the first 6 buttons are visible
  And a warning is logged with text "org.octos.actions: truncated 2 extra buttons"
  And the message body renders normally

Scenario: Malformed action entry is skipped with warning, others render
  Test: test_malformed_action_entry_skipped
  Given a Matrix event with `org.octos.actions` containing three entries:
    | id       | label  | style     | notes                            |
    | good_1   | Ok     | primary   |                                  |
    | (missing id field)          | invalid: no id                   |
    | good_2   | Cancel | secondary |                                  |
  When the message is rendered
  Then buttons "Ok" and "Cancel" are visible
  And the malformed entry in the middle is skipped
  And a warning is logged with text "org.octos.actions: skipping malformed entry at index 1"

Scenario: Danger-style button renders with negative visual style
  Test: test_danger_style_button_visual
  Given a Matrix event with an action button `{ id: "delete", label: "Delete", style: "danger" }`
  When the message is rendered
  Then the button uses `RobrixNegativeIconButton` style (red/destructive)

Scenario: Unknown style falls back to secondary
  Test: test_unknown_style_falls_back
  Given an action button with `style: "weird_style"`
  When the message is rendered
  Then the button uses the secondary style
  And no error is raised

Scenario: Splash card and action buttons coexist
  Test: test_splash_card_and_actions_coexist
  Given a Matrix event with both `org.octos.splash_card` and `org.octos.actions`
  When the message is rendered
  Then the Splash card widget renders the card content
  And the action buttons render below the card (regardless of whether the
    action button implementation uses Splash eval or a native Makepad button row)
  And both are visible simultaneously

Scenario: Action button label is escaped to prevent injection
  Test: test_action_button_label_escaped
  Given an action button with `label: "<script>alert(1)</script>"`
  When the message is rendered
  Then the button displays the literal text "<script>alert(1)</script>"
  And no `<script>` tag is interpreted or executed by the chosen renderer
  And if Splash eval is the chosen renderer, the label string is escaped before being
    interpolated into the Splash code string

Scenario: Action response failure shows error popup and re-enables buttons
  Test: test_action_response_send_failure_reenables
  Given the user has clicked "Regenerate PPT" and the buttons are disabled
  When the outgoing Matrix send request fails with an error
  Then a popup notification is shown with text "Failed to send action response"
  And the action buttons become enabled again
  And the user can retry by clicking a button

Scenario: Button label longer than 32 chars is truncated with ellipsis
  Test: test_long_label_truncated
  Given an action button with `label: "A very long label that exceeds the thirty two character limit by a lot"`
  When the message is rendered
  Then the displayed label is truncated to 32 characters with a trailing "…"
  And the full `label` value is still sent back in no response field (not used)
