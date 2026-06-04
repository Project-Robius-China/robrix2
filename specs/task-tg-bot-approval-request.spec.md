spec: task
name: "Telegram Bot UI Alignment — Approval Requests via Inline Buttons"
inherits: project
tags: [bot, ui, telegram-parity, approval, audit, octos]
depends: [task-tg-bot-action-buttons]
estimate: 2d
---

## Intent

为高风险 bot 操作建立一条**Octos 判权、Robrix 展示审批 UI、Octos 审计并执行**
的完整协议。当前 `tool_policy`、`sandbox`、`before_tool_call` hook 等权限真相
都在 Octos 侧，但 Robrix 还没有一套正式的“审批请求”展示和回传协议；这会
导致需要人工确认的动作只能退回文字说明或临时命令，既不像 Telegram 的
inline keyboard，也缺少稳定审计面。

本任务定义一个**最小可用**的审批请求协议：Octos 在需要人工确认时发送带
`org.octos.approval_request` 和 `org.octos.actions` 的 Matrix 消息；Robrix
把它渲染成消息下方的审批按钮；用户点击后 Robrix 发回结构化审批响应；
Octos 重新校验权限、做 replay/timeout 防护，并落审计日志。Robrix 不负责
决定“什么时候需要审批”，它只消费 Octos 发出的审批请求。

## Decisions

- **权限真相**：是否需要审批、谁有权审批、审批是否过期、批准后是否真正执行，
  全部由 Octos 决定。Robrix 不得根据本地规则自行放行或拒绝需要审批的操作。
- **审批请求消息**：当 Octos 判定某次操作需要人工确认时，发送一条普通
  `m.room.message`，并在 content 中附带：
  ```json
  {
    "msgtype": "m.text",
    "body": "Approval required: Execute shell command",
    "org.octos.approval_request": {
      "request_id": "req_abc123",
      "tool_name": "shell",
      "tool_args_digest": "sha256:4bf5...",
      "title": "Execute shell command",
      "summary": "rm -rf ~/tmp/cache",
      "risk_level": "critical",
      "authorized_approvers": ["@alice:example.org"],
      "expires_at": "2026-04-14T14:30:00Z",
      "on_timeout": "notify"
    },
    "org.octos.actions": [
      { "id": "approve", "label": "Approve", "style": "primary" },
      { "id": "deny",    "label": "Deny",    "style": "danger" }
    ]
  }
  ```
- **字段定义**：`org.octos.approval_request` 的必填字段是 `request_id`,
  `tool_name`, `tool_args_digest`, `title`, `summary`, `risk_level`,
  `authorized_approvers`, `expires_at`, `on_timeout`。其中
  `tool_args_digest` 的格式固定为 `sha256:<hex>`，`risk_level` 在 v1 只允
  许 `"normal"` 或 `"critical"`，`authorized_approvers` 必须是完整
  Matrix user ID 列表，`on_timeout` 在 v1 只允许 `"notify"`。
- **批准人来源**：`authorized_approvers` 由 Octos 根据当前 profile / room /
  sender policy 推导。Robrix 只用于 UI 呈现，Octos 在收到响应时必须重新按
  当前策略校验，不能把消息里的列表当最终真相。
- **空批准人列表无效**：`authorized_approvers` 必须至少包含 1 个完整 Matrix
  user ID。若当前策略判定“无人可批”，Octos 必须直接拒绝该工具调用，不得发出
  approval request。Robrix 收到空数组时按 malformed approval 处理。
- **审批请求不可变**：Robrix 渲染审批请求时必须只读取原始事件 content，不得
  采纳 `m.replace` / `m.new_content` 对 `org.octos.approval_request` 的修改。
  如果 Octos 要取消、更新或替换一条审批请求，必须发送一条新的 Matrix 消息。
- **按钮协议**：审批 UI 复用 Phase 4c 的 `org.octos.actions` 按钮行。v1
  只要求 `approve` 和 `deny` 两个 action id；其他 action id 不属于本任务。
- **Robrix 展示规则**：
  - 只有当消息同时包含**有效**的 `org.octos.approval_request` 和
    `org.octos.actions` 时，才渲染“审批卡片”
  - 当前用户在 `authorized_approvers` 内时，按钮可点击
  - 当前用户不在列表内时，审批卡片仍显示，但按钮为 disabled 状态
  - 如果 `org.octos.approval_request` 存在但结构无效，Robrix 必须 fail
    closed：不渲染审批按钮，并记录 warning
- **审批响应消息**：用户点击审批按钮后，Robrix 发送一条新的 Matrix 消息，
  回到同一房间，并 one-shot target 到原审批消息的 sender。content 必须包含：
  ```json
  {
    "msgtype": "m.text",
    "body": "[Approval: approve] Execute shell command",
    "org.octos.approval_response": {
      "request_id": "req_abc123",
      "decision": "approve",
      "source_event_id": "$original_event_id",
      "tool_args_digest": "sha256:4bf5..."
    },
    "m.relates_to": {
      "event_id": "$original_event_id",
      "rel_type": "m.in_reply_to"
    }
  }
  ```
  `decision` 取值仅为 `"approve"` 或 `"deny"`。
- **响应身份来源**：Octos 处理 `org.octos.approval_response` 时，approver 身份
  必须取自承载该 payload 的 Matrix 事件 `sender`，不得信任 payload 中任何
  自报身份字段。
- **Digest 归属**：`tool_args_digest` 对 Robrix 是不透明字符串。Robrix 只负责
  从 approval request 原样复制到 approval response，不解析、不校验、不重算。
- **发送路径**：审批响应不经过输入框当前 reply / mention / target 状态。
  它必须直接以审批消息的 `sender` 作为 one-shot `target_user_id` 构造，
  避免被用户当下输入框状态污染。
- **Replay/timeout 保护**：Octos 必须维护 pending approval store。一个
  `request_id` 只能消费一次；过期请求或已消费请求的响应不得执行工具调用。
- **房间绑定**：Octos 的 pending approval store 必须记录 `(request_id, room_id)`。
  收到审批响应时，响应消息所在的 Matrix `room_id` 必须与原审批请求的 `room_id`
  一致；不同房间的同 `request_id` 响应必须被拒绝。
- **超时行为**：当 `expires_at` 到期且尚未消费时，Octos 将 pending request
  标记为 expired，不执行该操作，并向房间发送一条 follow-up 通知消息
  （`on_timeout = "notify"`）。
- **审计落盘**：Octos 必须记录两类审计事件：
  - 审批请求创建：`request_id`, `tool_name`, `tool_args_digest`,
    requester, room_id, expires_at
  - 审批终态：`request_id`, decision, approver, decided_at, execution_outcome
- **兼容性**：没有 `org.octos.approval_request` 的普通 `org.octos.actions`
  消息继续按 Phase 4c 的 generic action-buttons 语义工作；本任务不改变它们。

## Boundaries

### Allowed Changes
- specs/task-tg-bot-approval-request.spec.md
- src/home/room_screen.rs
- src/sliding_sync.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json
- ../octos/crates/octos-agent/**
- ../octos/crates/octos-bus/**
- ../octos/book/src/**

### Forbidden
- 不要让 Robrix 根据本地 heuristics 决定某次操作“需要审批”
- 不要添加 `/approve 123`、`/deny 123` 这类 slash 命令式审批入口
- 不要让 Robrix 的 Approve 按钮绕过 Octos 的二次权限校验
- 不要新增 cargo 依赖
- 不要引入多步表单（填写 reason / confirm phrase）
- 不要让审批状态依赖本地-only 持久化才能保证安全

## Out of Scope

- 多级审批 / 两人批准
- 批准时填写 reason
- 批准按钮上的图标/emoji 设计
- 审批请求的 URL 按钮
- 审批历史浏览器 / 独立审计页面
- critical 风险的二次确认弹窗

## Completion Criteria

Scenario: Octos emits approval request message when policy requires human approval
  Test: test_matrix_approval_request_event_contains_protocol_fields
  Level: integration
  Targets: Matrix approval message protocol, pending-approval store
  Given Octos policy marks a `shell` tool call as approval-required
  When the agent attempts the tool call
  Then Octos does not execute the tool immediately
  And Octos emits a Matrix `m.room.message` event to the room
  And the event content contains a valid `org.octos.approval_request`
  And the event content contains `org.octos.actions` with `approve` and `deny`
  And `org.octos.approval_request.risk_level` is either `normal` or `critical`
  And `org.octos.approval_request.on_timeout = "notify"`

Scenario: Authorized approver sees enabled approval buttons inline
  Test: test_approval_request_renders_enabled_buttons_for_authorized_user
  Given a room timeline message contains a valid `org.octos.approval_request`
  And the local Matrix user is listed in `authorized_approvers`
  When Robrix renders the message
  Then the approval title and summary are visible
  And an "Approve" button is visible below the message body
  And a "Deny" button is visible below the message body
  And both buttons are enabled

Scenario: Unauthorized user sees approval card but cannot act
  Test: test_approval_request_disables_buttons_for_unauthorized_user
  Given a room timeline message contains a valid `org.octos.approval_request`
  And the local Matrix user is NOT listed in `authorized_approvers`
  When Robrix renders the message
  Then the approval title and summary are visible
  And the "Approve" button is visible but disabled
  And the "Deny" button is visible but disabled
  And clicking either button sends no Matrix message

Scenario: Approval request ignores m.replace edits to approver metadata
  Test: test_approval_request_ignores_m_replace_edits
  Given an approval-request message originally lists `authorized_approvers = ["@alice:example.org"]`
  And a later `m.replace` edit attempts to change `authorized_approvers` to `["@mallory:example.org"]`
  When Robrix renders the approval card
  Then the rendered approval card still uses the original `authorized_approvers = ["@alice:example.org"]`
  And "@mallory:example.org" cannot interact with the approval buttons

Scenario: Clicking Approve sends structured approval response to original bot
  Test: test_click_approve_sends_targeted_approval_response
  Given a rendered approval-request message from sender "@octosbot:127.0.0.1:8128"
  And the approval request has `request_id = "req_abc123"`
  And the approval request has `tool_args_digest = "sha256:4bf5"`
  And the approval request title is "Execute shell command"
  And the approval message defines an action button labeled "Approve"
  When the authorized user clicks the "Approve" button
  Then Robrix sends a new Matrix message to the same room
  And the outgoing message `target_user_id = "@octosbot:127.0.0.1:8128"`
  And the outgoing message content has `org.octos.approval_response.request_id = "req_abc123"`
  And the outgoing message content has `org.octos.approval_response.decision = "approve"`
  And the outgoing message content has `org.octos.approval_response.tool_args_digest = "sha256:4bf5"`
  And the outgoing message replies to the original event via `m.in_reply_to`

Scenario: Clicking Deny sends structured deny response to original bot
  Test: test_click_deny_sends_targeted_approval_response
  Given a rendered approval-request message from sender "@octosbot:127.0.0.1:8128"
  And the approval request has `request_id = "req_abc123"`
  And the approval message defines an action button labeled "Deny"
  When the authorized user clicks the "Deny" button
  Then Robrix sends a new Matrix message to the same room
  And the outgoing message `target_user_id = "@octosbot:127.0.0.1:8128"`
  And the outgoing message content has `org.octos.approval_response.decision = "deny"`
  And the input bar's current reply/mention state is not consulted

Scenario: Malformed approval request fails closed in Robrix
  Test: test_malformed_approval_request_does_not_render_buttons
  Given a Matrix message contains `org.octos.approval_request`
  But the object is missing `request_id` or `authorized_approvers`
  Or `authorized_approvers` is an empty array
  When Robrix renders the message
  Then no approval buttons are rendered
  And a warning is logged with text containing `org.octos.approval_request`

Scenario: Expired approval response does not execute the tool and emits timeout notification
  Test: test_expired_approval_request_notifies_and_does_not_execute
  Level: integration
  Targets: timeout handling, notification send path, execution guard
  Given Octos has a pending approval request with `request_id = "req_abc123"`
  And its `expires_at` is in the past
  When no valid approval response has consumed the request
  Then Octos marks the request as expired
  And Octos does not execute the pending tool call
  And Octos emits a follow-up Matrix message notifying that the approval request expired

Scenario: Duplicate approval response is rejected after the first consume
  Test: test_duplicate_approval_response_is_rejected
  Given Octos has already consumed approval request `req_abc123`
  When a second `org.octos.approval_response` arrives for `req_abc123`
  Then Octos does not execute the tool a second time
  And Octos records the response as rejected because the request is no longer pending

Scenario: Octos revalidates approver authority against current policy
  Test: test_approval_response_revalidated_against_current_policy
  Given the original approval request listed "@alice:example.org" in `authorized_approvers`
  But current Octos policy no longer authorizes "@alice:example.org" to approve this tool call
  When Octos receives an `org.octos.approval_response` from "@alice:example.org"
  Then Octos rejects the response
  And Octos does not execute the tool call

Scenario: Approval authority is derived from Matrix event sender, not response payload
  Test: test_approval_response_uses_matrix_sender_identity
  Given the original approval request listed "@alice:example.org" in `authorized_approvers`
  When a Matrix event from "@mallory:example.org" carries an `org.octos.approval_response` payload for that request
  Then Octos rejects the response regardless of any payload-level approver field
  And Octos does not execute the tool call

Scenario: Approval response from the wrong room is rejected
  Test: test_approval_response_wrong_room_rejected
  Given Octos created approval request `req_abc123` in room "!original:example.org"
  When an `org.octos.approval_response` for `req_abc123` arrives from room "!attacker:example.org"
  Then Octos rejects the response
  And Octos does not execute the tool call

Scenario: Approval request and decision are both written to audit log
  Test: test_approval_request_and_decision_are_audited
  Given Octos emits approval request `req_abc123`
  And an authorized user later approves it
  When the request reaches terminal state
  Then the audit log contains an entry for request creation with `request_id`, `tool_name`, and `tool_args_digest`
  And the audit log contains an entry for the terminal decision with `request_id`, `decision`, `approver`, and `execution_outcome`

Scenario: Generic action-buttons message without approval metadata keeps existing behavior
  Test: test_generic_actions_without_approval_request_remain_supported
  Given a Matrix message contains `org.octos.actions`
  And the message does NOT contain `org.octos.approval_request`
  When the user clicks one of the generic action buttons
  Then Robrix sends the existing generic `org.octos.action_response`
  And no `org.octos.approval_response` field is added
