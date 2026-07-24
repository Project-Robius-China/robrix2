# 团队协作实战

> **定位**：本章是五个协作场景的导览图，并给出一天的典型使用节奏。前置依赖：第 4 章（系统已跑通）。后续五章全部配真实截图，按使用顺序展开。

部署完成后，你的 HAgency 空间大致是这样一幅图景：

- 一个（或多个）**项目作战室**（如 `robrix2-board`）：人 + 一支或多支 Agent 团队同房；
- 每个任务在作战室里展开自己的 **Thread**，Agent 在里面持续汇报进度；
- 每个 Agent 一条 **DM 私聊**，用于一对一交办；
- 每个 Agent 一个 **`Approval:` 加密审批房**，高危操作在这里等你拍板。

Robrix2 的多标签工作区正是为这种形态设计的 —— 下面这排标签就是一次真实协作会话的现场：

```text
robrix2-board │ [Thread] robrix2-board │ DM: wf_coordinator │ Approval: wf_coordinator │ Approval: wf_codex
```

## 五个场景

| 章节 | 场景 | 你将看到 |
|------|------|---------|
| [5.1 把 Agent 请进你的空间](onboarding-agents.md) | 接入 | Agent Access 设置、框架选择、接受桥邀请 |
| [5.2 项目作战室](board-room.md) | 同房协作 | 人与多支 Agent 团队混合成员、workflow 斜杠命令 |
| [5.3 Thread 协作](threads.md) | 任务跟进 | 派单入线索、进度跟帖、Threads 面板 |
| [5.4 Owner 审批](approvals.md) | 授权 | 加密审批卡片、Approve once / Deny、fail-closed |
| [5.5 issue-workflow](issue-workflow.md) | 完整工作流 | 四角色团队端到端交付一个功能 |

## 一天的典型节奏

把五个场景串起来，一个真实工作日大概是这样的：

**早上**，你在作战室 `@wf_coordinator /go 012` 交办一个 issue，然后去做自己的事 —— 不需要盯着。coordinator 派单的封面消息出现在主时间线，过程收进 Thread。

**中途**，Robrix2 的通知把你拉回来两三次：一次是 coordinator 在 Thread 里请示方向（你回一句话拍板）；一次是 `Approval: wf_codex` 房间亮了 —— Codex 终审想跑一条越沙箱的命令，你扫一眼命令预览，点 `Approve once`。

**下班前**，你打开 Threads 面板扫一遍各条线索的最新状态，对完成的任务做真机验证，让 coordinator 发 draft PR（这一步又是一次审批）。

人的投入集中在**三类高价值瞬间**：拍板、授权、验收。其余时间，Agent 团队自己转，而你随时可以点开任何一条 Thread 看到全部过程 —— 这就是「透明的自治」。
