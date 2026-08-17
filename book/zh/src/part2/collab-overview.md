# 团队协作实战

> **定位**：本章是六个协作场景的导览图，并标注哪些是系统能力、哪些是工作流约定。前置依赖：第 4 章（基础链路已跑通）。

部署完成后，你的 HAgency 空间大致是这样一幅图景：

- 一个（或多个）**项目作战室**（如 `robrix2-board`）：人 + 一支或多支 Agent 团队同房；
- 每个任务可以在作战室里展开自己的 **Thread**；带可信 `reply_to` 的回复会延续线索；
- 普通 **DM 私聊**在首次一对一发送时按需创建；
- **`Approval:` 加密审批房**按 `(Agent, owner)` 创建或复用，高危操作在这里等 owner 拍板。

Robrix2 的多标签工作区正是为这种形态设计的 —— 下面这排标签就是一次真实协作会话的现场：

```text
robrix2-board │ [Thread] robrix2-board │ DM: wf_coordinator │ Approval: wf_coordinator │ Approval: wf_codex
```

## 六个场景

| 章节 | 场景 | 你将看到 |
|------|------|---------|
| [5.1 把 Agent 请进你的空间](onboarding-agents.md) | 接入 | Agent Access 设置、框架选择、接受桥邀请 |
| [5.2 项目作战室](board-room.md) | 同房协作 | 人与多支 Agent 团队混合成员、workflow 斜杠命令 |
| [5.3 Thread 协作](threads.md) | 任务跟进 | 派单入线索、进度跟帖、Threads 面板 |
| [5.4 Owner 审批](approvals.md) | 授权 | 加密审批卡片、Approve once / Deny、fail-closed |
| [5.5 issue-workflow](issue-workflow.md) | 完整工作流 | 四角色团队端到端交付一个功能 |
| [5.6 项目看板](project-board.md) | 全局审计 | dashboard 上的团队状态、spec 与 issue 总览 |

## 一天的典型节奏

把六个场景串起来，一个真实工作日大概是这样的：

**早上**，你在作战室 `@wf_coordinator /go 012` 交办一个 issue。Robrix2 只发送普通文本；是否理解 `/go`、创建 spec、主动更新 Thread，取决于 coordinator 已安装的 workflow skill。

**中途**，coordinator 可以在 Thread 里请示方向；`Approval: wf_final_reviewer` 房间可能出现一次受保护操作请求。前者是工作流约定，后者在受管运行时、唯一 owner 绑定和健康 E2EE 通道的前提下由协议强制。

**下班前**，你打开 Threads 面板和 Project Board 检查公开消息、backend task 与工件状态。当前 demo workflow 未必把内部阶段写进 durable task，所以看板和 Thread 都不是单一真相源；最终仍要检查 Git 状态、测试证据与目标平台。

建议把人的检查点固定在三类事件：拍板、授权、验收。系统能强制的是授权边界；拍板与主动汇报需要 workflow 配置和运行健康来维持。
