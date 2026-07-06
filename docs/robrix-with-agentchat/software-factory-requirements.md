# Robrix × agent-chat 软件工厂需求文档

> 本文定义在现有 Robrix × agent-chat workflow demo 基础上继续产品化的软件工厂需求。
> 当前 demo 已经跑通固定流程 `issue -> spec -> plan -> implement -> review -> final-review`；
> 本文关注下一步：多来源 issue intake、可定制 workflow、可审计 agent 交付、
> agent 自我提升循环，以及 agent 间对抗审查循环。

## 1. 背景与现状

现有 demo 已经证明以下能力可行：

- Robrix 作为 Matrix IM 操作入口，用户在房间中发起 workflow 命令。
- agent-chat 负责 agent 注册、消息路由、tmux agent 生命周期、Dashboard 和 MCP 工具。
- `roadmap/agentchat-demo/issue-workflow/SKILL.md` 通过 agent 名称分支出 coordinator、
  implementer、reviewer、final_reviewer 等角色。
- `roadmap/agentchat-demo/workflow-board.mjs` 作为独立 Workflow Board，读取目标项目中的
  `issues/`、`specs/`、`docs/plans/` 和 agent notes，展示 workflow 产物。
- 当前固定流程已经包含 spec gate、人类 approval、实现、对抗 review、Codex final review。

当前不足：

- Workflow 被写死在 shared skill 和 demo board 里，不能按项目或任务类型定制。
- Issue 来源主要是 Robrix IM 命令，尚未统一支持 GitHub、GitCode 等云平台 issue。
- Workflow Board 主要扫描文件产物，缺少统一的 workflow run 状态模型。
- Agent 之间的审查、驳回、返工、终审虽然已在流程中体现，但还不是可配置协议。
- 自我提升目前依赖人工总结和记忆沉淀，还没有正式 learning loop。

## 2. 产品目标

构建一个本地优先、可接云平台、可审计的软件工厂系统，使用户可以从多个入口提交需求，
由 agent 团队按可定制 workflow 完成交付，并在每次交付后沉淀可审查的改进建议。

目标能力：

1. 从 Robrix IM、GitHub、GitCode 等来源统一导入 issue。
2. 将外部 issue 标准化为内部 Factory Work Item。
3. 根据 work item 类型、标签、项目、来源或人工选择启动可定制 workflow。
4. 通过 agent-chat 调度多 agent 执行 spec、plan、implement、review、release 等阶段。
5. 通过 Workflow Board 展示 intake、run 状态、产物、审查结论和学习建议。
6. 通过 bounded loop 支持 agent 间反复审查、驳回、修复、终审和人工升级。
7. 通过 learning loop 让 agent 的技能、提示词和 workflow 随时间改进，但不允许静默自我升级。

非目标：

- 不把 Robrix 变成 workflow 权限源；Robrix 是人机交互界面。
- 不让 GitHub/GitCode webhook 直接触发高权限代码执行。
- 不在第一阶段支持任意 SaaS 项目管理系统；GitHub/GitCode 之外的来源留给 adapter 扩展。
- 不让 agent 自动修改并激活自己的 skill；所有持久化能力提升必须可审查、可回滚。

## 3. 核心概念

### 3.1 Issue Source

外部需求来源。首批支持：

- `robrix_im`: Robrix 房间命令、IM 消息、人工输入。
- `github`: GitHub issue、PR、comment、label、webhook。
- `gitcode`: GitCode issue、MR/PR、comment、label、webhook。

Source adapter 只负责接收、拉取、校验、标准化，不负责调度 agent。

### 3.2 Issue Envelope

所有来源进入系统后的标准化 envelope。

```json
{
  "source": "robrix_im | github | gitcode",
  "source_id": "github:owner/repo#123",
  "external_id": "123",
  "external_url": "https://...",
  "project": "robrix2",
  "repo": "owner/repo",
  "title": "...",
  "body": "...",
  "author": "...",
  "labels": ["bug", "p1"],
  "priority": "p0 | p1 | p2 | p3",
  "attachments": [],
  "created_at": "...",
  "updated_at": "...",
  "raw": {}
}
```

### 3.3 Factory Work Item

软件工厂内部工作项。一个 work item 可以来自一个或多个 external refs：

- Robrix 讨论房间。
- GitHub issue。
- GitCode issue。
- 后续生成的 PR/MR。
- Workflow run 产物。

Work item 是 workflow 的输入，不等同于某个平台的 issue。

### 3.4 Workflow Definition

可版本化的 workflow 定义，描述阶段、角色、产物、gate、转移条件和循环规则。

示例：

```yaml
id: software-factory-default
version: 1
entry_stage: intake

roles:
  coordinator: {}
  implementer: {}
  reviewer: {}
  final_reviewer: {}

stages:
  - id: intake
    role: coordinator
    output_artifacts: [issue]
    next: spec

  - id: spec
    role: coordinator
    gates: [agent_spec_lint, human_approval]
    output_artifacts: [task_spec]
    next: plan

  - id: implement
    role: implementer
    output_artifacts: [implementation_report, diff]
    next: review

  - id: review
    role: reviewer
    gates: [adversarial_review]
    on_approve: final_review
    on_reject: implement
    max_rounds: 3

  - id: final_review
    role: final_reviewer
    on_approve: done
    on_reject: implement
```

### 3.5 Workflow Run

某个 work item 按某个 workflow definition 执行的一次实例。Run 必须记录：

- run id、workflow id、workflow version。
- 当前 stage、历史 transitions。
- agent 角色绑定。
- 输入 envelope 和 work item id。
- 产物引用。
- gate 结果。
- review findings。
- 人工操作。
- 状态回写记录。

### 3.6 Gate

阻止 workflow 自动进入下一阶段的检查点。首批 gate：

- `human_approval`: 人类批准。
- `agent_spec_lint`: agent-spec parse/lint 通过。
- `test_pass`: 指定命令成功。
- `review_approve`: reviewer verdict 为 approve。
- `final_review_approve`: final reviewer verdict 为 approve。
- `security_review_approve`: 可选安全审查通过。
- `manual_override`: operator 强制推进或终止。

### 3.7 Learning Loop

Workflow 完成后生成 retrospective 和 improvement proposal。Proposal 必须进入审查队列，
不能自动修改并激活 agent skill、system prompt 或 workflow definition。

## 4. 总体架构

```text
Robrix IM ----\
GitHub --------> Issue Source Adapters -> Intake Queue -> Work Item Store
GitCode ------/                                      |
                                                     v
                                      Workflow Definition Registry
                                                     |
                                                     v
                                      Workflow Run Engine / Task Graph
                                                     |
                                                     v
                     agent-chat MCP / messaging / tmux agents / supervisors
                                                     |
                                                     v
               Artifacts + Review Findings + Event Log + Learning Proposals
                                                     |
                                                     v
                         Workflow Board / Robrix status / cloud sync-back
```

架构原则：

- Source adapter 不直接执行 workflow。
- Workflow run engine 不关心 issue 来自哪个平台，只消费 work item。
- Robrix 和云平台都是入口/展示/回写 surface，不是执行权限的最终来源。
- agent-chat 是 control plane，负责 agent 调度、消息路由和状态持久化。
- Workflow definition 是执行协议，必须版本化。
- Event log 是审计事实来源。

## 5. 功能需求

### 5.1 Issue Intake

需求：

- 支持从 Robrix IM 创建 issue envelope。
- 支持从 GitHub webhook 导入 issue opened、edited、labeled、commented。
- 支持从 GitCode webhook 导入 issue/MR 相关事件。
- 支持 polling fallback，用于 webhook 不可用的私有部署。
- 所有事件进入 intake queue，不直接启动高权限 agent 执行。
- Intake 页面显示来源、标题、作者、标签、优先级、外部链接、信任状态。

验收：

- 同一个 GitHub issue 的重复 webhook 不会创建重复 work item。
- Robrix 手动创建的 work item 可以绑定到后续 GitHub/GitCode issue。
- 无签名或签名错误的 webhook 被拒绝或标为 untrusted，不进入自动 workflow。

### 5.2 Work Item Store

需求：

- 存储标准化 work item。
- 支持 external refs 数组。
- 支持 source metadata 和 raw payload 保留。
- 支持 manual triage 字段：type、priority、workflow override、assignee team。
- 支持 work item 与多个 workflow runs 关联。

验收：

- 一个 work item 可以先跑 research workflow，再跑 implementation workflow。
- Work item 的外部引用可以回查到原平台链接。
- Work item 修改记录进入 event log。

### 5.3 Workflow Definition Registry

需求：

- 支持按项目、仓库或全局注册 workflow definition。
- 支持 workflow version。
- 支持启用/禁用某个 workflow。
- 支持默认 workflow 和 label/type 映射。
- 支持 definition lint，禁止无出口阶段、循环无上限、未知角色、未知 gate。

验收：

- 新增 research workflow 不需要修改 Workflow Board 主逻辑。
- 新增 security review stage 不需要修改 runner skill 主流程。
- 无效 workflow definition 在启动 run 前被拒绝。

### 5.4 Workflow Matching

需求：

- 根据 source、repo、project、label、issue type、priority 选择 workflow。
- 支持人工 override。
- 支持 dry-run 解释：为什么某个 work item 匹配到某个 workflow。

示例：

```yaml
source_rules:
  - source: github
    repo: project-robius/robrix2
    default_workflow: software-factory-default
    label_map:
      bug: bugfix-flow
      security: security-review-flow
      research: research-flow

  - source: robrix_im
    room: demoboard
    default_workflow: software-factory-default
```

验收：

- `label=bug` 的 GitHub issue 自动建议 `bugfix-flow`。
- Robrix 房间创建的临时需求默认进入 `software-factory-default`。
- 人工选择 workflow 后，自动匹配结果不再覆盖人工选择。

### 5.5 Workflow Run Engine

需求：

- 基于 workflow definition 创建 workflow run。
- 维护 run 当前 stage 和 transition history。
- 调用 agent-chat 发送 stage assignment。
- 收集 agent result，并根据 gate 决定下一阶段。
- 支持 bounded loop，例如 review reject -> implement，最多 3 轮后升级人工。
- 支持取消、暂停、恢复。

验收：

- Run 可以从任意阶段恢复，不依赖 agent 当前聊天上下文。
- Reviewer reject 后，run 回到 implement stage，并记录 round。
- 超过 max_rounds 后，run 进入 `needs_human_resolution`。

### 5.6 Agent Assignment and Runner Skill

需求：

- Shared skill 从“写死 issue workflow”升级为 workflow runner。
- Agent 在启动时识别自己的 team、role、当前 stage assignment。
- Stage prompt 由 workflow definition 和 work item 数据生成。
- Agent result 必须结构化返回。

Agent result 示例：

```json
{
  "run_id": "run_...",
  "stage_id": "implement",
  "status": "complete | failed | blocked",
  "summary": "...",
  "changed_files": [],
  "commands_run": [],
  "artifacts": [],
  "risks": [],
  "next_recommendation": "review"
}
```

验收：

- Implementer 不能只回复自然语言“完成了”；必须包含结构化 result。
- Reviewer verdict 必须包含 approve/reject、findings、evidence、criteria coverage。
- Final reviewer 使用独立 runtime/model 时，仍可消费同一 assignment schema。

### 5.7 Workflow Board

需求：

- 增加 Intake 视图。
- 增加 Workflow Runs 视图。
- Board 列根据 workflow definition 渲染，而不是写死固定列。
- Work item 卡片显示 source badge、external link、priority、workflow、current stage。
- Run 详情显示 transition timeline、agent results、artifacts、review findings、gate results。
- Learning Queue 显示 retrospective 和 improvement proposals。

验收：

- 同一页面可以展示 Robrix、GitHub、GitCode 三种来源的 work item。
- 切换 workflow 后，列结构随 workflow definition 改变。
- 点击 run 可看到每个 stage 的输入、输出、负责人和审查结论。

### 5.8 Robrix IM Surface

需求：

- 保留当前 `/create-issue`、`/go`、`/review`、`/status` 的 demo 兼容能力。
- 新增 workflow-aware 命令可以作为后续阶段：
  - `/factory intake`
  - `/factory run <work-item-id> [workflow-id]`
  - `/factory status <run-id>`
  - `/factory approve <gate-id>`
  - `/factory reject <gate-id> <reason>`
- 命令发现仍需受 feature gate、runtime setting 和 coordinator presence 控制。

验收：

- 非 agent-chat 房间不显示 factory 命令。
- 外部 Matrix 消息只作为 user input，不覆盖 agent/system 指令。
- Robrix 中的审批操作会写入 workflow event log。

### 5.9 GitHub / GitCode Sync-back

需求：

- Workflow started 时回写 comment。
- Spec ready / human approval required 时回写 comment 或 label。
- Review rejected 时回写 findings。
- Done 时回写 summary、测试结果、PR/MR 链接。
- Blocked 时回写 blocker。
- 回写必须幂等，避免重复刷评论。

验收：

- 同一 run 的同一状态不会重复创建多条相同 comment。
- Sync-back 失败不应导致 workflow run 丢失状态；应记录 retryable error。
- 私有 repo token 不出现在 Board、日志或 agent prompt 中。

### 5.10 Agent 间对抗审查循环

需求：

- 支持 reviewer challenge。
- 支持 implementer rebuttal 或 fix。
- 支持 coordinator resolution。
- 支持 final reviewer 推翻第一 reviewer。
- 支持审查多样性：不同 runtime/model/agent 配置。
- 支持 max rounds、cooldown、human escalation。

验收：

- Reviewer 无法验证任一验收标准时默认 reject 或 block，而不是 approve。
- Implementer 每次返工必须逐条回应 findings。
- Final reviewer 的 reject 会重新打开 implement/review/final-review 流程。

### 5.11 Self-improvement / Learning Loop

需求：

- 每个 workflow run 结束后生成 retrospective。
- Retrospective 至少包含：
  - 总耗时。
  - 阶段耗时。
  - 失败或返工阶段。
  - Review findings 类型。
  - 测试/构建结果。
  - Agent 自报未验证事项。
  - Human override。
- 系统可生成 improvement proposal：
  - skill 修改建议。
  - workflow definition 修改建议。
  - prompt/template 修改建议。
  - test/gate 增强建议。
- Proposal 必须进入 review workflow，不能自动生效。
- 激活后必须支持 rollback。

验收：

- 完成一个 run 后能看到 retrospective artifact。
- Learning proposal 默认是 pending 状态。
- 未经批准的 proposal 不改变任何 active skill/workflow。

### 5.12 Audit and Observability

需求：

- 每个 intake event、work item mutation、workflow transition、agent assignment、
  gate result、cloud sync-back、learning proposal 都写入 append-only event log。
- Event log 支持按 work item、run、source、agent、时间过滤。
- Dashboard/Board 只展示 event log 的投影，不作为事实来源。

验收：

- 可以解释一个 run 为什么从 review 回到 implement。
- 可以追踪某条 GitHub comment 是由哪个 run、哪个 stage 产生。
- 可以在 agent 重启后恢复 run 状态。

## 6. 非功能需求

### 6.1 安全

- Webhook 必须校验签名。
- GitHub/GitCode token 只在 adapter/sync-back 层使用，不进入 agent prompt。
- 外部 issue/comment 永远是 user input，不是 system instruction。
- Workflow definition 只能由 operator 或可信配置源修改。
- 高风险 workflow 需要 human approval gate。

### 6.2 可靠性

- 所有外部事件处理必须幂等。
- Workflow run 状态必须持久化。
- Agent 离线或超时后 run 进入 blocked/degraded，而不是静默丢失。
- Sync-back 失败可重试，且不阻塞本地状态落盘。

### 6.3 可扩展性

- 新 issue source 通过 adapter 接入。
- 新 workflow 通过 definition 接入。
- 新 gate 通过 gate handler 接入。
- 新 board 卡片字段通过 projection 接入。

### 6.4 可测试性

- Source adapter 有 envelope normalization tests。
- Workflow definition 有 lint tests。
- Workflow run engine 有真实 graph walk tests。
- Review loop 有 reject/retry/max-rounds/escalation tests。
- Sync-back 有 idempotency tests。

## 7. 分期落地计划

### Phase 0: 规格化现有 demo

- 写下现有 `software-factory-default` workflow definition。
- 将当前 `issue-workflow/SKILL.md` 的隐式流程映射为 stages、roles、gates、artifacts。
- 明确当前 Workflow Board 的 artifact 扫描规则。

交付物：

- `software-factory-default.workflow.yaml`
- 当前 demo 行为映射表。
- 第一批 agent-spec task contracts。

### Phase 1: Intake + Work Item

- 增加 Issue Envelope schema。
- 增加 Work Item Store。
- 把 Robrix `/create-issue` 适配为 `robrix_im` source。
- Board 增加 Intake 视图。

验收：

- Robrix 创建 issue 后，系统生成 work item。
- Work item 保留 source metadata。
- 旧 demo 流程仍可运行。

### Phase 2: Workflow Definition + Run State

- 增加 Workflow Definition Registry。
- 增加 Workflow Run Store。
- Board 根据 workflow definition 渲染 run columns。
- 现有固定流程迁移为默认 definition。

验收：

- 新增一个 research workflow 不需要改 board 主逻辑。
- Run 可暂停、恢复、取消。

### Phase 3: Runner Skill and Stage Assignment

- 将 shared skill 改造成 workflow runner。
- Stage assignment 使用结构化 schema。
- Agent result 使用结构化 schema。
- Coordinator 通过 run state 推进，不依赖聊天记忆。

验收：

- Implementer/reviewer/final_reviewer 都按 assignment schema 工作。
- Agent 重启后可以从当前 stage 恢复。

### Phase 4: GitHub / GitCode Intake

- 增加 GitHub webhook adapter。
- 增加 GitCode webhook adapter。
- 增加 source mapping rules。
- 增加 sync-back MVP。

验收：

- GitHub/GitCode issue 可以进入 intake queue。
- Label/type 可自动建议 workflow。
- Workflow 状态可以回写到原 issue。

### Phase 5: Review Loop and Learning Loop

- 抽象 adversarial review loop。
- 增加 retrospective artifact。
- 增加 learning proposal queue。
- 增加 proposal review/activate/rollback 协议。

验收：

- Reviewer reject 触发 bounded loop。
- 完成 run 后生成 retrospective。
- Proposal 未批准前不改变 active workflow/skill。

### Phase 6: Robrix 产品化入口

- Robrix 展示 factory/intake/run 状态。
- 增加 workflow-aware slash commands。
- 保持 agent-chat feature gate 和 runtime setting gate。

验收：

- 用户可以在 Robrix 中查看 work item/run 状态并审批 gate。
- 非启用房间不暴露 experimental factory 命令。

## 8. 建议拆分的 Agent Specs

第一批建议写成独立 task specs：

1. `task-software-factory-intake.spec.md`
   - Issue Envelope、Robrix source adapter、Work Item Store。
2. `task-software-factory-workflow-definition.spec.md`
   - Workflow Definition Registry、definition lint、default workflow。
3. `task-software-factory-run-state.spec.md`
   - Workflow Run Store、transition log、pause/resume/cancel。
4. `task-software-factory-board.spec.md`
   - Intake view、run view、definition-driven columns。
5. `task-software-factory-runner-skill.spec.md`
   - Stage assignment schema、agent result schema、role runner behavior。
6. `task-software-factory-github-gitcode-intake.spec.md`
   - GitHub/GitCode webhook normalization、dedup、trust policy。
7. `task-software-factory-review-loop.spec.md`
   - Reviewer challenge、implementer rebuttal/fix、max rounds、final-review loop。
8. `task-software-factory-learning-loop.spec.md`
   - Retrospective、improvement proposal、approval、activation、rollback。

## 9. Open Questions

1. Workflow definition 存放位置：agent-chat runtime data、目标 repo `.agentchat/`，还是 Robrix demo 目录？
2. Work item store 应放在 agent-chat backend 内，还是先作为 demo 外层服务？
3. GitHub/GitCode webhook 是否需要第一阶段支持公网部署，还是只支持本地 tunneling/dev token？
4. Sync-back 的最小权限模型如何定义：只 comment，还是也允许 label/assign/close？
5. Learning proposal 的 reviewer 是否必须是不同 runtime/model？
6. Robrix 是否只做命令入口，还是也嵌入 factory board 的原生 UI？

## 10. 第一阶段成功标准

第一阶段不追求全量云平台和自我学习。最小成功标准：

- 当前 Robrix IM demo 行为不回退。
- Robrix `/create-issue` 产生标准 Work Item。
- Work Item 可以手动启动 `software-factory-default` workflow run。
- Workflow Board 能显示 Intake 和 Run 状态。
- Run transition、agent result、review verdict 都进入 event log。
- Review reject 能回到 implement，且超过 max rounds 会升级人工。

达到以上标准后，再接 GitHub/GitCode intake 和 learning loop。
