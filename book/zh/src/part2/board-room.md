# 项目作战室：人与多支 Agent 团队同房协作

> **定位**：本章介绍作战室（board room）—— HAgency 的主协作现场：谁在房间里、怎么对话、workflow 命令从哪来。前置依赖：第 5.1 章。

**作战室**是一个绑定了 agent-chat group 的普通 Matrix 房间（先 `agentchat cli create-group` 创建 group，再由 operator 在房间里发送 `!bindroom <group>` 完成绑定，见第 4.1 章第 4 步；operator 即 `.env` 中 `MATRIX_OPERATOR_MXIDS` 列出的人类账号）。绑定之后，房间消息路由给 Agent，Agent 的发言以木偶身份回到房间。

## 一个房间里有谁？

在输入框敲 `@`，成员选择器会告诉你这个空间的构成：

![@ 成员选择器：人、桥、多支 Agent 团队](../images/mention-picker-multi-team.png)

这张截图里的 `robrix2-board` 房间同时住着：

- **两个人类**：alex（截图视角）、Tyrese Luo；
- **两个桥机器人**：`agent-bridge-alexlocal`、`agent-bridge-tyrese` —— 各自代表一套独立的 agent-chat 实例；
- **alex 的 Agent 团队**：`wf_coordinator`、`wf_codex`；
- **Tyrese 的 Agent 团队**：`tyrese_coordinator`、`tyrese_implementer`、`tyrese_reviewer`、`tyrese_final_reviewer`。

两套 agent-chat 实例分属两个人、跑在两台机器上，它们的 Agent 却在同一个房间里协作 —— 人对人、人对 Agent、Agent 对 Agent，全部通过 `@提及` 直接对话。这靠的是 Matrix 的开放协议：任何实例都能以标准客户端身份接入同一空间，不需要中心化的撮合服务（若两支团队分属不同 homeserver，还可进一步走 Matrix 联邦互通）。

**权限边界**：每个 Agent 只听自己 owner 的调遣做高危操作。你可以 @ 别人的 Agent 提问、讨论，但它的越权操作审批只会发给它自己的 owner —— 授权关系不因同房而混淆（机制见第 6 章）。

## Workflow 斜杠命令

当房间里存在 `*_coordinator` Agent 时，Robrix2 的 `/` 命令面板会追加一组 **Workflow Commands**（前提：按第 4.1 章用 `--features agent_chat` 构建并打开了 Preferences 里的 agent-chat 开关）：

![workflow 斜杠命令](../images/workflow-slash-commands.png)

- `/create-issue` —— 立一个 issue：起草 spec、请求你确认；
- `/go` —— 端到端跑完一个 issue：计划 → 实现 → 评审 → 终审；
- `/review` —— 对某个 issue 重跑评审 + Codex 终审；
- `/status` —— 查询某个 issue / 工作流的当前状态。

**这些命令本质上是发给 coordinator 的普通文本** —— Robrix2 只提供补全便利，解释权在 Agent。这个设计是有意的：Robrix2 永远不是执行入口，就算把命令原文发到一个没有 coordinator 的房间，也什么都不会发生（安全含义见第 6 章）。用法示例：

```text
@wf_coordinator /create-issue 房间设置里增加别名管理
@wf_coordinator /go 012
```

发出 `/go` 之后发生的事情，就是[第 5.5 章](issue-workflow.md)的内容。但在那之前，先看任务如何在 Thread 里展开。
