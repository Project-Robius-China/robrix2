# 项目作战室：人与多支 Agent 团队同房协作

> **定位**：本章介绍作战室（board room）—— HAgency 的主协作现场：谁在房间里、怎么对话、workflow 命令从哪来。前置依赖：第 5.1 章。

**作战室**是一个绑定了 agent-chat group 的**非加密** Matrix 房间。`!bindroom` 建立 room→group；人类再亲自邀请每个 Agent 建立 room+agent→owner。两种绑定都完成后，显式路由给 Agent 的消息进入 backend，Agent 的公开回复以木偶身份回到房间。

## 一个房间里有谁？

在输入框敲 `@`，成员选择器会告诉你这个空间的构成：

![@ 成员选择器：人、桥、多支 Agent 团队](../images/mention-picker-multi-team.png)

这张截图里的 `robrix2-board` 房间同时住着：

- **两个人类**：alex（截图视角）、Tyrese Luo；
- **两个桥机器人**：`agent-bridge-alexlocal`、`agent-bridge-tyrese` —— 各自代表一套独立的 agent-chat 实例；
- **alex 的 Agent 团队**：`wf_coordinator`、`wf_codex`；
- **Tyrese 的 Agent 团队**：`tyrese_coordinator`、`tyrese_implementer`、`tyrese_reviewer`、`tyrese_final_reviewer`。

两套 agent-chat 实例分属两个人、跑在两台机器上，它们的 Agent 可以在同一个房间里公开发言。**人→Agent** 用 `@具体 Agent` 路由；同一 backend 内的 Agent→Agent 派单使用 MCP/backend 消息。当前 bridge 会忽略 `@ac_*` 发送者以防循环，所以不能把跨实例 Agent 在 Matrix 里互相 @ 描述成可靠的执行通道；跨团队消息目前应由人转交，或只作为公开状态阅读。

**权限边界**：你可以 @ 别人的 Agent 讨论，但该 Agent 是否接受任务由对方实例策略决定；受保护操作只发给它自己的 owner。每个 backend 只能调度自己的 Agent、项目路径、token 与模型池，不能借共享房间取得队友机器的权限。

## @ 是执行路由，不只是提醒

默认 `MATRIX_DEFAULT_WAKE=off`。共享房间中的顶层消息如果没有显式 @，bridge 可以记录它，但不会唤醒任何 Agent。rich reply 目前可能依据被回复的木偶推断目标；如果团队要求“每次都必须显式 @”，不要把这种推断当作安全边界，应在验收时单独验证运行版本的行为。

为了避免抢答和请求放大，作战室按以下规则使用：

| 输入 | 预期 |
|------|------|
| 顶层消息，无 @ | 不唤醒 Agent |
| `@wf_coordinator ...` | 只唤醒对应 Agent |
| 同时 @ 两个 Agent | 两个目标各自收到任务 |
| Agent 在房间公开发言 | 供人阅读；不会自动成为另一实例 Agent 的任务 |

## Workflow 斜杠命令

当房间里存在 `*_coordinator` Agent 时，Robrix2 的 `/` 命令面板会追加一组 **Workflow Commands**（前提：按第 4.1 章用 `--features agent_chat` 构建并打开了 Preferences 里的 agent-chat 开关）：

![workflow 斜杠命令](../images/workflow-slash-commands.png)

- `/create-issue` —— 立一个 issue：起草 spec、请求你确认；
- `/go` —— 端到端跑完一个 issue：计划 → 实现 → 评审 → 终审；
- `/review` —— 对某个 issue 重跑评审 + Codex 终审；
- `/status` —— 查询某个 issue / 工作流的当前状态。

**这些命令本质上是发给 coordinator 的普通文本**。Robrix2 只提供补全；只有安装了兼容 workflow skill 的 coordinator 才会解释它们。没有 skill、Agent 离线或未 @ 时，命令不会自动创建 backend workflow run。

```text
@wf_coordinator /create-issue 房间设置里增加别名管理
@wf_coordinator /go 012
```

发出 `/go` 之后发生的事情，就是[第 5.5 章](issue-workflow.md)的内容。但在那之前，先看任务如何在 Thread 里展开。
