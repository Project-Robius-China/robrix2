spec: task
name: "Octos Native Approval Policy Configuration"
inherits: project
tags: [octos, approval, policy, config, audit]
depends: [task-tg-bot-approval-request]
estimate: 1.5d
---

## Intent

把当前 “通过 `before_tool_call` hook 返回 approval JSON” 的审批触发方式，
升级成 Octos 的**原生配置驱动**能力。目标不是替换已有的 approval-request
协议，而是为它提供一个一眼可见、可审计、可复用的权限来源：Octos 通过
`approval_policy` 配置决定某个 tool call 是否需要人工审批，再复用已经存在
的 pending approval / Matrix inline approval 按钮协议完成整条链路。Robrix
不需要知道审批规则细节，只消费 Octos 发出的审批请求。

## Decisions

- **权限分层**：
  1. `tool_policy.deny` 先执行，命中即硬拒绝
  2. `approval_policy` 决定“是否需要审批”
  3. 未命中审批规则的工具调用直接执行
  4. `before_tool_call` hooks 仍可保留，但退居高级/动态例外，不是 v1 的主审批路径
- **配置入口**：在 Octos `Config` 中新增顶层 `approval_policy` 字段，与
  `tool_policy`、`sandbox` 同层。
- **v1 匹配粒度**：`approval_policy.rules` 只按 **tool name** 匹配，不支持
  参数正则、路径前缀、provider-specific 细分，也不支持 per-room / per-sender
  条件表达式。
- **最小配置形态**：
  ```json
  {
    "approval_policy": {
      "default": "allow",
      "rules": [
        {
          "tools": ["shell"],
          "require_approval": true,
          "risk_level": "critical",
          "authorized_approvers": ["@alex:127.0.0.1:8128"],
          "expires_in_secs": 300,
          "on_timeout": "notify"
        }
      ]
    }
  }
  ```
- **`default` 语义**：v1 只允许 `"allow"`。未命中任何 rule 的 tool call 默认直接执行。
  不在本任务里引入 `"deny"` 或 `"approve"` 作为全局默认值。
- **规则顺序**：`rules` 按数组顺序匹配，**first match wins**。
- **规则字段**：
  - `tools`: 非空数组，元素为 tool name 字符串
  - `require_approval`: v1 只允许 `true`
  - `risk_level`: `"normal"` 或 `"critical"`
  - `authorized_approvers`: 非空完整 Matrix user ID 列表
  - `expires_in_secs`: 正整数，v1 必须显式配置
  - `on_timeout`: v1 只允许 `"notify"`
- **空 approver 列表无效**：如果规则配置了空 `authorized_approvers`，Octos 配置加载必须失败；
  运行时不得退回“没人能批但照样发 approval request”。
- **协议复用**：命中 `approval_policy` 的 tool call 必须复用
  `task-tg-bot-approval-request` 已定义的 pending approval / Matrix message protocol。
  不得再发明第二套 approval request 消息格式。
- **摘要生成**：`tool_args_digest` 继续由 Octos 在运行时根据真实 tool args 计算。
  `approval_policy` 配置中不出现 digest 字段。
- **标题/摘要来源**：v1 的 `title` 和 `summary` 在运行时由 Octos 根据 tool name +
  tool arguments 自动生成；不要求在配置里手填消息模板。
- **审计真相**：审批请求创建、审批终态、超时终态的审计仍由 Octos 记录；
  `approval_policy` 只是新增了判定来源，不改变审计归属。
- **Hook 兼容性**：如果当前会话同时配置了 `approval_policy` 和 `before_tool_call` hook：
  - `tool_policy.deny` 仍然最高优先级
  - `approval_policy` 命中后，直接进入审批路径
  - v1 不要求 hook 再次把同一次调用转成 approval-request
  - hooks 可以继续用于 deny / modify / logging
- **适用范围**：v1 先覆盖 `chat` / `gateway` / Matrix room bot 这条主路径。
  不单独为 HTTP API、dashboard shell、admin endpoints 做额外审批 UI。

## Boundaries

### Allowed Changes
- specs/task-octos-approval-policy-config.spec.md
- docs/superpowers/plans/2026-04-14-octos-approval-policy-config-plan.md
- ../octos/crates/octos-cli/src/config.rs
- ../octos/crates/octos-cli/src/commands/chat.rs
- ../octos/crates/octos-cli/src/commands/gateway/**
- ../octos/crates/octos-agent/**
- ../octos/book/src/advanced.md
- ../octos/book/src/configuration.md

### Forbidden
- 不要把审批规则判断挪到 Robrix
- 不要让 `approval_policy` 依赖外部 Python / shell hook 才能工作
- 不要在 v1 引入参数正则匹配、房间表达式、sender 表达式
- 不要新增 cargo 依赖
- 不要修改已经存在的 `org.octos.approval_request` / `org.octos.approval_response` 协议字段名

## Out of Scope

- 参数级审批规则（如 `shell.command` 正则）
- per-room / per-sender / per-channel 审批规则
- 审批模板国际化
- 批准时填写 reason
- 多级审批 / 多人批准
- 将 `tool_policy` 合并重写成全新权限 DSL

## Completion Criteria

Scenario: Config with approval_policy rule requiring approval for shell loads successfully
  Test: test_config_deserializes_approval_policy
  Given an Octos config JSON contains top-level `approval_policy`
  And the rule lists `tools = ["shell"]`
  And the rule defines non-empty `authorized_approvers`
  When Octos loads the config
  Then config parsing succeeds
  And `approval_policy.rules[0].tools` contains `"shell"`
  And `approval_policy.rules[0].risk_level` is preserved

Scenario: Empty authorized_approvers in approval_policy fails closed at config load
  Test: test_config_rejects_approval_policy_with_empty_authorized_approvers
  Given an Octos config JSON contains `approval_policy.rules[0].authorized_approvers = []`
  When Octos loads the config
  Then config parsing or validation fails
  And the error mentions `authorized_approvers`

Scenario: First matching approval rule wins
  Test: test_approval_policy_first_match_wins
  Given `approval_policy.rules` contains two rules matching the `shell` tool
  And the first rule authorizes `@alice:example.org`
  And the second rule authorizes `@bob:example.org`
  When Octos evaluates a `shell` tool call
  Then the resulting approval request uses only the first rule
  And `authorized_approvers = ["@alice:example.org"]`

Scenario: Tool policy deny still overrides approval policy
  Test: test_tool_policy_deny_overrides_approval_policy
  Given `tool_policy.deny` blocks the `shell` tool
  And `approval_policy` also contains a `shell` approval rule
  When the agent attempts to call `shell`
  Then Octos does not create a pending approval request
  And Octos does not execute the tool
  And the tool call is rejected as denied

Scenario: Approval policy converts matching tool call into pending approval
  Test: test_approval_policy_shell_call_emits_pending_approval
  Given `approval_policy.rules` contains a rule for `shell`
  When the agent attempts a `shell` tool call
  Then Octos does not execute `shell` immediately
  And Octos creates a pending approval request
  And the request `tool_name = "shell"`
  And the request `authorized_approvers` come from the matched rule
  And the request `risk_level` matches the rule
  And the request `on_timeout = "notify"`

Scenario: Non-matching tool call bypasses approval policy
  Test: test_approval_policy_non_matching_tool_executes_normally
  Given `approval_policy.rules` only match `shell`
  When the agent calls `read_file`
  Then Octos does not create a pending approval request
  And Octos executes `read_file` normally

Scenario: Approval policy generates expires_at from expires_in_secs
  Test: test_approval_policy_generates_relative_expiry
  Targets: approval-policy runtime, relative expiry calculation
  Given a matching approval rule has `expires_in_secs = 300`
  And the approval request is created at fixed UTC time `2026-04-14T12:00:00Z`
  When Octos creates the approval request
  Then the request `expires_at = "2026-04-14T12:05:00Z"`

Scenario: Gateway runtime emits Matrix approval request using policy-driven approvers
  Test: test_gateway_runtime_emits_matrix_approval_request_from_policy
  Level: integration
  Targets: config loading, approval policy evaluation, Matrix approval message protocol
  Given BotFather profile config contains an `approval_policy` rule for `shell`
  When a Matrix message causes the agent to attempt `shell`
  Then Octos emits a Matrix message containing `org.octos.approval_request`
  And the embedded `authorized_approvers` equal the rule's configured approvers
  And the embedded `tool_name = "shell"`

Scenario: before_tool_call hook remains optional and does not need to emit approval JSON
  Test: test_approval_policy_does_not_require_hook_exit_3
  Level: integration
  Targets: approval policy evaluation without external hook process
  Given `approval_policy` contains a rule for `shell`
  And no `before_tool_call` hook is configured
  When the agent attempts `shell`
  Then Octos still creates a pending approval request
  And no external hook process is required for the approval path
