spec: task
name: "Telegram Bot Orchestration — Natural-language Scheduling Commands"
inherits: project
tags: [bot, orchestration, cron, schedule, octos]
depends: [task-tg-bot-bot-aware-command-discovery]
estimate: 2d
---

## Intent

`octos` 已经有成熟的 `cron` tool 和持久化调度能力，但当前对用户暴露的仍然是
底层 `cron` 概念，不像 Telegram bot 常见的“自然语言创建提醒/定时任务”体验。
如果直接把 `cron` 暴露给 `robrix2` 用户，命令会显得过于底层，也不符合当前
BotFather 作为管理/编排入口的产品方向。

本任务把定时任务能力产品化成一组 **BotFather 自然语言命令**：
`/schedule <task>`、`/schedules`、`/unschedule <job-id>`。对用户来说这是
自然语言调度；对 Octos 内部来说，仍然复用现有 `cron` tool 和 `cron.json`
持久化，不另起第二套调度系统。

## Decisions

- **命令集合**：v1 只支持：
  - `/schedule <natural-language task>`
  - `/schedules`
  - `/unschedule <job-id>`
- **BotFather-only**：这三条命令都由 BotFather 解释；`robrix2` 发送时按
  parent/management bot 路由，不发给 child bots。
- **可用上下文**：`/schedule` 系列在以下上下文可用：
  - `ManagementDm`
  - `ManagementRoom`
  它们在 `ChildBotRoom` 和 `None` 上下文不可见。
- **底层实现**：BotFather 把自然语言调度请求解析后，调用现有 `cron` tool
  或等价的 cron service 接口；v1 不新增第二套调度存储。
- **当前聊天上下文继承**：新建的 schedule 必须继承当前对话的 delivery context：
  - 当前 channel
  - 当前 chat_id / room_id
  这样定时任务触发后会把消息送回创建它的同一聊天上下文。
- **`/schedule` 参数必填**：`/schedule` 没有正文时，BotFather 返回友好错误，
  不创建任何 cron job。
- **自然语言时间解释**：BotFather 负责把 `<natural-language task>` 解析成：
  - 要执行的 message
  - 时间/周期
  - 时区
  若信息不足，BotFather 返回澄清/错误，不创建 cron job。
- **相对延时任务**：v1 支持一次性相对延时表达，例如：
  - `20秒之后提醒我看天气`
  - `in 20 seconds remind me to check weather`
  这类请求创建 `CronSchedule::At` 一次性任务，而不是循环任务。
- **`/schedules` 列表范围**：v1 只列出“当前聊天上下文创建的 schedule”，而不是
  全 profile 的所有 cron jobs。
- **`/unschedule` 的 selector**：v1 只支持按 `job-id` 删除。`/schedules`
  的返回结果必须包含可复制/可引用的 `job-id`。
- **状态变更闭环**：`/unschedule <job-id>` 只能删除当前聊天上下文可见的 job；
  如果 `job-id` 不存在或属于别的聊天上下文，BotFather 返回友好错误。
- **v1 不做 pause/resume**：`/pause_schedule`、`/resume_schedule` 暂不支持。
- **v1 不调度 /allbots**：`/schedule` 创建的任务只针对当前聊天上下文，不把
  `/allbots` 编排语义纳入第一版。
- **命令发现配合**：
  - `ManagementDm` 和 `ManagementRoom` 都显示 `/schedule`、`/schedules`、
    `/unschedule`
  - `/schedule`、`/unschedule` 是参数命令
  - `/schedules` 是纯命令
- **结果展示**：v1 允许 schedule 创建/删除结果先用普通 bot 文本消息返回，
  不要求专门的结构化 cron result card。

## Boundaries

### Allowed Changes
- specs/task-tg-bot-natural-language-schedule.spec.md
- src/shared/mentionable_text_input.rs
- src/room/room_input_bar.rs
- ../octos/crates/octos-cli/src/session_actor.rs
- ../octos/crates/octos-cli/src/cron_tool.rs
- ../octos/crates/octos-bus/src/cron_service.rs
- ../octos/book/src/advanced.md
- ../octos/book/src/cli-reference.md

### Forbidden
- 不要在 `robrix2` 里直接暴露底层 `cron` 术语作为用户主入口
- 不要新增图形化 cron editor
- 不要让 child bot 自己解释 `/schedule`
- 不要让 `/schedules` 默认列出所有 profile 的任务
- 不要在 v1 里支持 `/pause_schedule` 或 `/resume_schedule`
- 不要新增 cargo 依赖

## Out of Scope

- `/schedule` 编排 `/allbots`
- 图形化 schedule 管理面板
- schedule 编辑功能
- pause/resume
- 按自然语言描述删除任务

## Completion Criteria

Scenario: Management DM shows schedule commands
  Test: test_management_dm_shows_schedule_commands
  Given the command discovery context is `ManagementDm`
  When the slash popup opens
  Then the popup shows `/schedule`, `/schedules`, and `/unschedule`
  And the popup does NOT show `/allbots`

Scenario: Child bot room does not show schedule commands
  Test: test_child_bot_room_hides_schedule_commands
  Given the command discovery context is `ChildBotRoom`
  When the slash popup opens
  Then the popup does NOT show `/schedule`
  And the popup does NOT show `/schedules`
  And the popup does NOT show `/unschedule`

Scenario: /schedule routes to BotFather and preserves user-visible natural language
  Test: test_schedule_command_targets_parent_bot
  Given a `ManagementDm` context bound to `@octosbot:127.0.0.1:8128`
  When the user submits `/schedule 每天早上 9 点提醒我看天气`
  Then the outgoing message `target_user_id` is `@octosbot:127.0.0.1:8128`
  And the outgoing message body is `/schedule 每天早上 9 点提醒我看天气`
  And `explicit_room` is false

Scenario: /schedule without task text is rejected
  Test: test_schedule_without_body_is_rejected
  Given the user submits `/schedule`
  When BotFather evaluates the command
  Then no cron job is created
  And BotFather replies with a user-visible error explaining that schedule text is required

Scenario: Successful /schedule creates a cron job for the current chat context
  Test: test_schedule_creates_cron_job_for_current_chat_context
  Given the user is in Matrix room `!room:127.0.0.1:8128`
  And the user submits `/schedule 每天早上 9 点提醒我看天气`
  When BotFather parses the request successfully
  Then Octos creates a cron job in `cron.json`
  And the cron payload records the current channel and current chat context
  And the scheduled delivery target is the same room `!room:127.0.0.1:8128`

Scenario: /schedules lists only jobs for the current chat context
  Test: test_schedules_lists_only_current_chat_jobs
  Given there are cron jobs for multiple chat contexts
  And only two jobs belong to the current Matrix room
  When the user submits `/schedules`
  Then BotFather replies with those two jobs only
  And each listed item contains a `job-id`
  And jobs from other chats are not shown

Scenario: /unschedule removes a visible job by job-id
  Test: test_unschedule_removes_job_by_id
  Given `/schedules` has listed a job with id `cron_ab12cd34`
  And that job belongs to the current chat context
  When the user submits `/unschedule cron_ab12cd34`
  Then Octos removes the cron job
  And BotFather replies with a user-visible success message

Scenario: /unschedule rejects unknown or foreign job-id
  Test: test_unschedule_rejects_unknown_or_foreign_job
  Given the user submits `/unschedule cron_deadbeef`
  And that `job-id` does not belong to the current chat context
  When BotFather evaluates the command
  Then no cron job is removed
  And BotFather replies with a user-visible error

Scenario: Ambiguous natural-language time does not create a schedule
  Test: test_schedule_ambiguous_time_returns_clarification
  Given the user submits `/schedule 下次提醒我看天气`
  When BotFather cannot resolve a concrete schedule from the text
  Then no cron job is created
  And BotFather replies with a clarification or validation message

Scenario: Relative delay creates a one-shot schedule
  Test: test_schedule_relative_delay_creates_one_shot_job
  Given the user submits `/schedule 20秒之后提醒我看天气`
  When BotFather parses the request successfully
  Then Octos creates a `CronSchedule::At` job for the current chat context
  And the scheduled payload message is `提醒我看天气`
  And the user-visible success reply indicates a one-shot delayed schedule

Scenario: /schedules remains a pure command in discovery
  Test: test_schedules_is_pure_command_in_discovery
  Given the slash popup command metadata table
  When `/schedules` is inspected
  Then it is classified as a pure command
  And `/schedule` and `/unschedule` are classified as parameterized commands
