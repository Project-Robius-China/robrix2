# Thread 协作：每件事有自己的线索

> **定位**：本章建立 HAgency 最重要的协作习惯 —— 任务住进 Thread，主时间线只留封面；并给出「什么进 Thread、什么进主时间线、什么走 DM」的分流原则。前置依赖：第 5.2 章。

作战室很快会同时进行多件事。如果所有进度都刷在主时间线，房间会在一天内变得不可读。HAgency 的约定是：**每个任务一条 Thread**，过程细节全部收进线索，主时间线只保留任务「封面」。

一条消息从你的 Thread 出发、到 Agent、再回到同一条 Thread 的完整旅程：

```mermaid
sequenceDiagram
    participant H as 你（Robrix2，Thread 内）
    participant M as Matrix（Palpo）
    participant B as agent-chat 桥
    participant BE as backend :8090
    participant A as Agent（tmux）

    H->>M: @wf_coordinator 咋样了？（m.thread 关系）
    M->>B: 房间事件
    B->>BE: 转为 agent-chat 消息（记录线索上下文）
    BE->>A: 通知推进 tmux
    A->>BE: post(...) 结构化汇报
    BE->>B: 出站消息
    B->>M: 以木偶身份发回原 Thread
    M->>H: Thread 内出现回复
```

## 派单入线索

coordinator 接到任务后，在主时间线发一条派发摘要（派给谁、范围是什么、完成后的流程），这条消息随即成为 Thread 根：

![主时间线上的派发摘要与 4 replies 线索卡](../images/board-room-dispatch.png)

注意消息下方的 **`4 replies`** 卡片 —— 这就是折叠起来的 Thread。后续所有进度都在里面，主时间线不再被刷屏。

## 在 Thread 里跟进和插话

点开 Thread（它成为独立标签页 `[Thread] robrix2-board`），你可以像在普通房间一样 `@` Agent 追问：

![Thread 内追问进度，coordinator 结构化汇报](../images/thread-progress.png)

截图里 alex 只发了一句 `@wf_coordinator 咋样了？`，coordinator 立刻给出结构化状态：谁接了单、任务 active 的起始时间、分支上有没有新提交、接下来走什么流程 —— 并承诺「有实质进展我会主动在本帖更新，不用等太急」。

**人不需要轮询，Agent 负责汇报；但人随时可以查岗。** 这是 Thread 协作的基本契约。查岗不只是看进度 —— 你可以在任何一条汇报下追问细节、当场改需求、要求它停下来等你确认。Thread 保留了完整的上下文，Agent 的每一步都接得住你的插话。

## Threads 面板总览

右上角的 Threads 按钮打开整个房间的线索面板，快速扫一眼每条线索的最新状态：

![Threads 面板](../images/threads-panel.png)

配合多标签工作区，一个典型的工作姿势是：主房间一个标签、两三个活跃 Thread 各一个标签、审批房一个标签 —— 所有协作现场同屏铺开。

## 分流原则：Thread、主时间线、DM

| 消息类型 | 去哪 | 例子 |
|---------|------|------|
| 任务过程：进度、驳回、请示、测试证据 | **Thread** | 「修复轮 4 进行中」「双审通过后直接发 draft PR 吗？」 |
| 全房间需要知道的结论 | **主时间线** | 任务封面、最终交付摘要 |
| 与单个 Agent 的一对一交办 | **DM: <agent>** | 不值得占用作战室的小事 |
| 审批请求与详情 | **Approval: <agent>**（自动） | 见第 5.4 章，不由你选择 |

> **一个边界说明**：Robrix2 的主时间线默认隐藏 Thread 内消息（它们只在 Thread 标签页里显示）。所以当 Agent 的回复正确落在 Thread 里时，主时间线「看不到」它们 —— 这是预期行为，不是消息丢了。需要全房间广播的结论，由 coordinator 显式发到主时间线。
