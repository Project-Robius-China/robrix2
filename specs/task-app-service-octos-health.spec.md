spec: task
name: "App Service Settings — Octos Service Health Status"
inherits: project
tags: [app-service, octos, settings, health]
estimate: 0.5d
---

## Intent

在当前 Robrix2 里，`App Service` 只知道“功能是否启用”和“房间是否绑定 bot”，却不知道本机 `octos serve` 是否真的活着。这会导致用户在 bot 不响应时无法快速区分是 Matrix 路由问题、BotFather 绑定问题，还是本机 Octos 后端根本没起来。

本任务要求在 `Settings > App Service` 卡片中增加一个 **Octos Service** 地址输入与健康状态。默认地址为 `http://127.0.0.1:8010`，用户可改成其他远程地址，但必须通过 URI 规范校验。该状态只用于健康检查提示，不得影响 Matrix 登录、homeserver、消息发送或 bot 路由。

## Decisions

- 默认 Octos Service 地址为 `http://127.0.0.1:8010`
- UI 必须显示可编辑的 Octos Service 地址输入框
- 用户输入的 Octos Service URL 必须通过 URI 校验后才能保存或用于探测
- 探测顺序固定为: `GET /health`，若失败再尝试 `GET /api/status`
- 当任一探测返回 HTTP 200 时，状态视为 `Reachable`
- 当两个探测都失败，或连接被拒绝、超时、DNS 失败时，状态视为 `Unreachable`
- 初始状态为 `Unknown`
- 点击 `Check Now` 后，探测进行中的中间状态为 `Checking`
- 本任务只支持手动检查，不做自动轮询
- 健康状态是 Robrix 本地 UI 状态，不写入 Matrix，不影响 `bot_settings.enabled`、`room_bindings`、`target_user_id` 或 `explicit_room`
- 实现必须复用现有 HTTP 能力，不新增 cargo 依赖

## Boundaries

### Allowed Changes
- src/settings/bot_settings.rs
- src/settings/settings_screen.rs
- src/app.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json
- specs/task-app-service-octos-health.spec.md

### Forbidden
- 不要修改 Matrix homeserver 配置逻辑
- 不要修改消息发送、bot routing、BotFather 命令或 `/bot` 面板行为
- 不要添加自动轮询、后台定时探测或开机自检
- 不要新增 cargo 依赖

## Out of Scope

- 远程 Octos 服务地址配置
- 房间内的 appservice 健康提示
- 通过 Matrix 往返验证 bot 可达性
- 自动重试、取消、指数退避
- `/health` 或 `/api/status` 的后端协议改造

## Completion Criteria

Scenario: Settings card shows editable Octos service address with local default and unknown status
  Test: test_app_service_health_defaults_to_unknown_with_editable_local_url
  Level: widget
  Targets: BotSettings, App Service settings card
  Given the user opens `Settings > App Service`
  When no health check has been run in the current app session
  Then the Octos Service input shows `http://127.0.0.1:8010`
  And the displayed health status is `Unknown`
  And a `Check Now` action is visible
  And the Octos Service field is editable

Scenario: Custom Octos service URL is accepted when valid
  Test: test_app_service_health_uses_custom_octos_service_url_when_configured
  Given the user enters `https://octos.example.com:9443`
  When the address is saved
  Then the stored Octos Service URL becomes `https://octos.example.com:9443`
  And subsequent health checks use that base URL

Scenario: Invalid Octos service URL is rejected before probing
  Test: test_app_service_health_validates_octos_service_url
  Given the user enters an invalid Octos Service URL
  When the user presses `Save` or `Check Now`
  Then the invalid value is rejected
  And no health probe request is started

Scenario: Successful /health probe marks Octos as reachable
  Test: test_app_service_health_check_uses_health_endpoint_first
  Level: integration
  Test Double: local HTTP probe stub
  Targets: `/health`
  Given the app service settings card is visible
  And `GET {configured Octos Service URL}/health` returns HTTP 200
  When the user presses `Check Now`
  Then the health status changes to `Checking`
  And then the health status changes to `Reachable`

Scenario: Fallback to /api/status when /health is unavailable
  Test: test_app_service_health_check_falls_back_to_api_status
  Level: integration
  Test Double: local HTTP probe stub
  Targets: `/health`, `/api/status`
  Given the app service settings card is visible
  And `GET {configured Octos Service URL}/health` fails
  And `GET {configured Octos Service URL}/api/status` returns HTTP 200
  When the user presses `Check Now`
  Then the health status changes to `Reachable`

Scenario: Failed probe marks Octos as unreachable
  Test: test_app_service_health_check_sets_unreachable_when_both_probes_fail
  Level: integration
  Test Double: local HTTP probe stub
  Targets: `/health`, `/api/status`
  Given the app service settings card is visible
  And `GET {configured Octos Service URL}/health` fails
  And `GET {configured Octos Service URL}/api/status` fails
  When the user presses `Check Now`
  Then the health status changes to `Checking`
  And then the health status changes to `Unreachable`

Scenario: Repeated clicks do not start overlapping checks
  Test: test_app_service_health_check_disables_check_now_while_checking
  Given a health check is already in progress
  When the user presses `Check Now` again
  Then no second probe sequence is started
  And the UI continues to show `Checking`

Scenario: Opening settings does not trigger an automatic Octos probe
  Test: test_app_service_health_does_not_auto_probe_on_open
  Level: widget
  Targets: BotSettings, manual Check Now flow
  Given the user opens `Settings > App Service`
  When the card is first rendered
  Then the health status remains `Unknown`
  And no health check runs until the user presses `Check Now`
  And no `/health` request is started
  And no `/api/status` request is started

Scenario: Health checks do not affect app service enablement or bot routing state
  Test: test_app_service_health_check_is_ui_only
  Given any combination of `bot_settings.enabled` and room bot bindings
  When the user runs a health check
  Then `bot_settings.enabled` is unchanged
  And `room_bindings` are unchanged
  And no Matrix message or appservice command is sent
