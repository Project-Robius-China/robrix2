# Robrix × agent-chat 软件工厂 MVP

> 本文是《软件工厂需求文档》的可验收 MVP 切片，目标是在一个中午完成一次可观察的长任务演示：
> 用户从 Robrix 提交任务，agent 团队按固定软件工厂流程交付，过程中包含 spec 版本锁定、
> 人类审批、实现、对抗 review、Codex 终审和复盘产物。

## 1. MVP 目标

中午验收不是验证“完整软件工厂产品”，而是验证最关键的一条长任务链路是否已经成立：

```text
Robrix issue intake
  -> Factory Work Item
  -> spec v1 draft
  -> spec approval + immutable pin
  -> plan
  -> implement
  -> adversarial review
  -> Codex final review
  -> retrospective
  -> board / artifact inspection
```

验收时必须能回答四个问题：

- 任务从哪里来，是否被标准化成 work item。
- agent 到底按哪个 spec 版本执行，后续是否被偷偷改过。
- 长任务每一步由谁完成，证据和产物在哪里。
- reviewer / final reviewer 是否真的能打回，而不是只做形式确认。

## 2. MVP 范围

### 2.1 必须包含

- Robrix IM 作为唯一必需 issue 来源。
- 一个默认长任务 workflow：`intake -> spec -> approval -> plan -> implement -> review -> final_review -> retrospective -> done`。
- 一个 workflow run ledger，记录 stage、agent、产物、gate、时间和结论。
- 一个 spec version ledger，记录 task spec 的版本、状态、checksum 和 approval。
- Workflow Board 能展示当前 issue、spec、plan、run 状态、review 结论和 retrospective。
- 对抗审查 loop 至少支持一次 reject -> fix -> re-review。
- Codex final reviewer 必须独立重跑关键验证或明确记录无法验证原因。
- Learning loop 只生成 retrospective 和 improvement proposal，不自动修改 skill 或 workflow。

### 2.2 明确不做

- GitHub / GitCode webhook 自动导入。
- 云平台 sync-back。
- 任意 workflow definition UI。
- 多项目权限模型。
- 自动激活 agent skill / prompt / workflow 的自我升级。
- Robrix 原生 factory board 页面。

### 2.3 接口预留

MVP 内部数据结构保留 `source`、`external_refs`、`workflow_id`、`workflow_version` 字段，
这样后续接 GitHub / GitCode 时不需要推翻 run 和 spec 记录。

## 3. 演示成功标准

一次午间验收通过的最低标准：

- 用户在 Robrix 房间发起一个长任务，coordinator 生成 work item 和 task spec v1。
- 人类批准的是 spec v1 的 checksum；run 启动后 pin 住该版本。
- Implementer 修改真实目标仓库代码或文档，不只是回消息。
- Reviewer 独立检查产物，至少覆盖 acceptance criteria、diff、验证命令和风险。
- 如果 reviewer reject，implementer 必须逐条回应 findings 后再进入 re-review。
- Final reviewer 使用 Codex runtime 独立终审，并记录自己的验证命令或拒绝理由。
- Workflow Board 或产物目录能看到完整 timeline 和最终 verdict。
- 完成后生成 retrospective，列出耗时、返工、未验证事项和改进建议。

## 4. MVP 数据模型

### 4.1 Factory Work Item

MVP 可以先落到目标仓库文件，而不要求进入 agent-chat 后端数据库。

建议文件：

```text
.agentchat-factory/work-items/WI-0001.json
```

最小字段：

```json
{
  "id": "WI-0001",
  "source": "robrix_im",
  "source_room": "demoboard",
  "source_message_id": "...",
  "title": "为 demo app 增加持久化计数器",
  "body": "...",
  "author": "@alex:127.0.0.1:8128",
  "external_refs": [],
  "status": "ready_for_spec",
  "created_at": "2026-06-19T12:10:00+08:00"
}
```

### 4.2 Workflow Run

建议文件：

```text
.agentchat-factory/runs/RUN-0001.json
```

最小字段：

```json
{
  "id": "RUN-0001",
  "work_item_id": "WI-0001",
  "workflow_id": "software-factory-mvp",
  "workflow_version": 1,
  "status": "in_progress",
  "current_stage": "implement",
  "pinned_spec": {
    "spec_id": "task-WI-0001",
    "version": 1,
    "checksum": "sha256:..."
  },
  "roles": {
    "coordinator": "wf_coordinator",
    "implementer": "wf_implementer",
    "reviewer": "wf_reviewer",
    "final_reviewer": "wf_final_reviewer"
  },
  "transitions": [],
  "artifacts": [],
  "gate_results": []
}
```

### 4.3 Event Log

MVP 可以用 append-only JSONL：

```text
.agentchat-factory/events.jsonl
```

每行记录一个事实事件：

```json
{"ts":"...","run_id":"RUN-0001","type":"stage_started","stage":"review","actor":"wf_reviewer"}
{"ts":"...","run_id":"RUN-0001","type":"review_verdict","stage":"review","verdict":"reject","findings":["..."]}
```

Board 可以扫描 artifact，也可以从 event log 投影；事实来源应以后者为准。

## 5. Spec 版本管理机制

### 5.1 原则

Spec 是任务执行契约，不是普通草稿文档。MVP 必须遵守：

- 已批准 spec 不允许原地修改。
- 每次实质变更创建新版本。
- Workflow run 必须 pin 到一个具体 spec version 和 checksum。
- Plan、implementation、review、final review 都必须引用同一个 pinned spec。
- Reviewer 如果发现实现对照了错误 spec version，必须 reject。

### 5.2 文件命名

建议 task spec 文件按版本命名：

```text
specs/task-WI-0001-counter-persistence.v1.spec.md
specs/task-WI-0001-counter-persistence.v2.spec.md
```

`specs/project.spec.md` 仍然是项目级约束，不参与每个 task 的版本递增；task spec 继承它。

### 5.3 Spec Version Ledger

建议文件：

```text
.agentchat-factory/specs.json
```

示例：

```json
{
  "specs": [
    {
      "spec_id": "task-WI-0001-counter-persistence",
      "version": 1,
      "path": "specs/task-WI-0001-counter-persistence.v1.spec.md",
      "status": "approved",
      "checksum": "sha256:...",
      "created_by": "wf_coordinator",
      "created_at": "2026-06-19T12:15:00+08:00",
      "approved_by": "@alex:127.0.0.1:8128",
      "approved_at": "2026-06-19T12:19:00+08:00",
      "supersedes": null,
      "superseded_by": null
    }
  ]
}
```

状态枚举：

- `draft`: coordinator 起草中，不能启动 implement。
- `approved`: 人类已批准，可以被 run pin。
- `superseded`: 已被新版本替代，不能被新的 run 使用。
- `retired`: 明确废弃。

### 5.4 修改规则

- `draft` 可以被 coordinator 更新，但每次提交审批前必须重新计算 checksum。
- `approved` 不可原地改。需要变更时创建 `v2`，并把 `v1.status` 设为 `superseded`。
- 已启动 run 继续使用 pinned spec，除非人类显式取消当前 run 并以新 spec version 重开。
- Review findings 触发的需求变更，如果超出当前 spec，应进入 `spec_change_requested`，不能让 implementer 自行扩大范围。

### 5.5 Approval Gate

Robrix 中的人类 approval 必须包含或对应以下信息：

```text
approve spec task-WI-0001-counter-persistence v1 sha256:...
```

如果为了演示仍保留简写 `approve`，coordinator 必须在回帖中明确写出：

- 被批准的 spec path。
- version。
- checksum。
- 之后 run 将 pin 此版本。

## 6. MVP Workflow

### 6.1 Stage 定义

| Stage | Owner | 输入 | 输出 | Gate |
|---|---|---|---|---|
| `intake` | coordinator | Robrix 消息 | work item | 无 |
| `spec` | coordinator | work item + project spec | task spec draft + spec ledger | `agent_spec_lint` |
| `approval` | human + coordinator | task spec draft | approved spec version | `human_approval` |
| `plan` | coordinator | pinned spec | plan artifact | 无 |
| `implement` | implementer | pinned spec + plan | diff + implementation report | verification command |
| `review` | reviewer | pinned spec + diff | approve/reject findings | adversarial review |
| `final_review` | final reviewer | pinned spec + reviewer verdict | final approve/reject | independent runtime |
| `retrospective` | coordinator | complete run log | retrospective + proposals | 无 |

### 6.2 Review Loop

MVP loop 规则：

- reviewer 可以 `approve`、`reject` 或 `block`。
- `reject` 回到 `implement`，round +1。
- Implementer 下一轮必须逐条回应 previous findings。
- 最多 2 轮 reject；超过后进入 `needs_human_resolution`。
- final reviewer reject 也回到 `implement`，并记录 final gate finding。

### 6.3 Agent Result Schema

每个 agent 阶段回帖可以是自然语言，但必须附结构化摘要：

```json
{
  "run_id": "RUN-0001",
  "stage": "review",
  "actor": "wf_reviewer",
  "status": "complete",
  "verdict": "reject",
  "evidence": {
    "commands_run": ["cargo check"],
    "artifacts_read": [
      "specs/task-WI-0001-counter-persistence.v1.spec.md",
      "docs/plans/WI-0001-plan.md"
    ]
  },
  "findings": [
    {
      "severity": "major",
      "message": "Acceptance criterion 2 is not implemented"
    }
  ],
  "next_stage": "implement"
}
```

## 7. 中午验收脚本

### 7.1 验收前准备

- Palpo 已启动并可从 Robrix 登录。
- agent-chat backend、bridge、push-relay、dashboard 已启动。
- `wf_coordinator`、`wf_implementer`、`wf_reviewer`、`wf_final_reviewer` 已在线。
- Workflow Board 已打开。
- 目标仓库为一次性 sandbox 或明确允许 agent 修改的 demo repo。
- 目标仓库已初始化 `.agentchat-factory/` 目录。

建议窗口：

```text
12:00-12:10  preflight + 打开 Robrix / Agent Monitor / Workflow Board
12:10-12:20  创建 work item + spec v1 + approval
12:20-12:45  plan + implement + first review
12:45-13:00  reject/fix/re-review 或 final review
13:00-13:10  retrospective + artifact inspection
```

### 7.2 演示任务建议

任务需要足够长，能体现多阶段协作，但不能大到午间不可控。建议用 demo sandbox：

```text
@wf_coordinator /create-issue 计数器持久化长任务 |
在现有 Makepad 2.0 计数器 demo 上增加本地持久化：
1. 点击 +1 后计数增加；
2. 重启 app 后恢复上次计数；
3. UI 显示一个 reset 按钮；
4. cargo check 必须通过；
5. reviewer 必须确认实现没有绕过 project.spec.md 的 Makepad 2.0 约束。
```

这个任务天然适合验收：

- 需要 spec 明确验收标准。
- 需要真实代码修改。
- 需要 reviewer 检查持久化和 reset。
- 容易制造一次 review reject，例如漏掉 reset 或未记录重启恢复验证。

### 7.3 操作步骤

1. 在 Robrix `demoboard` 房间发送 `/create-issue`。
2. 等 coordinator 回帖，确认已生成 work item、task spec v1、lint 结果和 checksum。
3. 打开 Workflow Board，确认 work item 和 spec draft 可见。
4. 在 Robrix 发送 approval。
5. 确认 run ledger 中出现 `pinned_spec.version = 1` 和对应 checksum。
6. 等 coordinator 生成 plan 并派发给 implementer。
7. 观察 implementer 修改目标仓库并回报 verification command。
8. 观察 reviewer 独立检查，不接受纯口头报告。
9. 如果 reviewer reject，确认 run 回到 implement，round +1，implementer 逐条回应 findings。
10. reviewer approve 后，确认 Codex final reviewer 介入。
11. final reviewer approve 后，确认 coordinator 生成 retrospective。
12. 在 Board 上打开 run 详情，检查 timeline、artifacts、gate results、review findings。

## 8. 验收 Checklist

### 8.1 功能链路

- [ ] Robrix 消息创建了 `Factory Work Item`。
- [ ] Work item 保留 source room、author、原始需求文本。
- [ ] Coordinator 生成了 task spec v1。
- [ ] `agent-spec parse` 和 `agent-spec lint --min-score 0.7` 已执行并记录。
- [ ] 人类 approval 绑定 spec version + checksum。
- [ ] Run ledger pin 住 approved spec。
- [ ] Plan 引用 pinned spec。
- [ ] Implementer 产生真实 diff。
- [ ] Reviewer 独立读取 spec、plan、diff 和验证结果。
- [ ] Review reject 能回到 implement。
- [ ] Final reviewer 在 reviewer approve 后才介入。
- [ ] Retrospective 已生成。

### 8.2 Spec 版本

- [ ] `specs/*.v1.spec.md` 存在。
- [ ] `.agentchat-factory/specs.json` 记录 v1 path、status、checksum、approved_by。
- [ ] 已 approved 的 spec 没有被原地修改。
- [ ] `RUN-*.json` 中 pinned checksum 与 ledger 一致。
- [ ] Review / final review 明确引用 pinned spec。
- [ ] 如果出现需求变更，系统创建 v2 或进入 `spec_change_requested`，没有静默扩大范围。

### 8.3 可观察性

- [ ] Workflow Board 能看到 work item、spec、plan、run、review、retrospective。
- [ ] `.agentchat-factory/events.jsonl` 能解释每一次 stage transition。
- [ ] 每个 agent result 都能回溯到 Matrix 回帖或产物文件。
- [ ] 失败、block、manual override 不会消失在聊天记录里。

### 8.4 安全边界

- [ ] Robrix 输入只作为 user request，不覆盖 agent/system 指令。
- [ ] Learning proposal 不会自动改 active skill。
- [ ] Agent 不能在未 approval 的 spec 上开始 implementation。
- [ ] Reviewer 不接受“我已经完成了”这种无证据报告。

## 9. MVP 交付物清单

文档和配置：

- `docs/robrix-with-agentchat/software-factory-mvp.md`
- `docs/robrix-with-agentchat/software-factory-requirements.md`
- `roadmap/agentchat-demo/CHECKLIST.md`

目标仓库运行产物：

- `.agentchat-factory/work-items/WI-0001.json`
- `.agentchat-factory/runs/RUN-0001.json`
- `.agentchat-factory/specs.json`
- `.agentchat-factory/events.jsonl`
- `issues/WI-0001-*.md`
- `specs/task-WI-0001-*.v1.spec.md`
- `docs/plans/WI-0001-*.md`
- `docs/reviews/WI-0001-review.md`
- `docs/reviews/WI-0001-final-review.md`
- `docs/retrospectives/WI-0001-retrospective.md`

## 10. 分阶段实现计划

### Step 1: 文件协议先行

- 增加 `.agentchat-factory/` 文件协议。
- Coordinator 写 work item、spec ledger、run ledger、event log。
- Board 先扫描这些文件，不急着改 agent-chat 后端数据模型。

完成标准：

- 不启动完整 workflow，也能从文件看懂一个 run 的状态。

### Step 2: Spec Version Gate

- 修改 coordinator 行为：生成 versioned task spec。
- Approval 时记录 checksum。
- Run 启动时 pin spec version。
- Reviewer 校验 pinned spec。

完成标准：

- 人为修改 approved spec 后，reviewer 或 preflight 能报出 checksum mismatch。

### Step 3: Long-task Run Ledger

- 每个 stage transition 写入 events.jsonl。
- 每个 agent result 写入 run ledger artifact refs。
- Board 显示 timeline。

完成标准：

- 断开聊天上下文后，仍能从 ledger 恢复当前 stage。

### Step 4: Review Loop

- Reviewer reject 进入 implement round 2。
- Implementer 必须引用 previous findings。
- 超过 max rounds 进入 human resolution。

完成标准：

- 午间演示能看到至少一次 reject/fix/re-review，或用预置任务稳定触发。

### Step 5: Retrospective

- Run 完成后生成 retrospective。
- Improvement proposal 只进入 pending queue。

完成标准：

- 验收结束时能看到这次长任务的返工原因、耗时和下一步改进建议。

## 11. 后续从 MVP 到完整软件工厂

MVP 通过后再推进：

- 把 `.agentchat-factory/` 升级为 agent-chat 后端 Work Item / Run Store。
- 把固定 workflow 抽成 versioned workflow definition。
- 接入 GitHub / GitCode issue intake。
- 增加 sync-back。
- 把 learning proposal 接入独立 review workflow。
- 在 Robrix 中提供更原生的 factory 状态入口。
