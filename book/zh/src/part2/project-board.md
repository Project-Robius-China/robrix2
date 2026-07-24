# 项目看板：任务与工件的全局视图

> **定位**：本章介绍 agent-chat Project Board 的预览实现：backend 状态与项目工件的只读投影。基线是 `feat/project-board` 提交 `3102a5f`，尚不是本书核验时的 agent-chat 主线功能。

聊天房间是公开协作**发生**的地方，但它是时间线视角。Project Board (`/projects`) 汇总 backend durable tasks / task graphs / heartbeat 和受绑定项目的本地工件扫描。顶部导航与 Monitor / Tasks / Pool / Alerts / Config 保持一致；Agent 卡片可跳到对应 Monitor 视图。

它不会读取 demo workflow 的 `.agentchat-demo/state.json`。如果 workflow 没创建 backend task/task graph，`/go` 的内部阶段不会自动出现在看板。发布本章前应先把 Project Board 分支合入目标 release，并提供受支持的 group→project binding 写入流程；当前绑定数据需要预先准备。

## 团队总览

![Project Board：项目组、统计与成员卡片](../images/project-board.png)

看板顶部选择一个 **project group**（截图为 `robrix2-board`，绑定 `robrix2` 项目与 `issue-workflow@1` 工作流），下方一排统计瓦片直接回答最常问的问题：

- **Members / Online**：项目组成员数与在线数；
- **Working / Blocked / Open Tasks**：几个在干活、几个被卡住（`waiting` / `stale` 状态单独提示，如截图中 coordinator 已等待 wf_codex 终审 7 小时 —— 这类「静默停滞」正是看板要暴露的）；
- **Worktrees**：Agent managed project/worktree 数量与 Git dirty 状态。`0 dirty` 只表示 `git status --porcelain` 没有未提交改动，不表示任务已完成、提交、推送或合并；
- **Specs / Changes**：spec 与本地/远端 issue 的数量（下一节展开）。

成员卡片展示 runtime、backend 已知任务和心跳。**UNREGISTERED** 表示 Matrix 房间成员不属于当前 backend，例如队友的 Agent 木偶或人类账号；它是只读观察，不授予调度或审批权。一个 Agent 同属多个 group 时，v1 task 没有 project ID，任务可能投影到多个项目，这是当前限制。

## Specs & Changes：spec 驱动的工件面板

![Specs 与 Issues 双栏](../images/board-specs-issues.png)

看板下半部分把项目的两类核心工件放在一起：

**左栏 Specifications** —— 扫描项目里的 [agent-spec](https://github.com/ZhangHanDong/agent-spec) 合约文件（`specs/*.spec.md`），显示声明的 Scenario / `Test:` 映射数量和提供这份 worktree 检查结果的 Agent。它不运行测试，也不表示 coverage/pass；“Agent”也不是正式 spec owner。

**右栏 Changes** —— provider-neutral 聚合：

![本地 issue 与 GitHub issue 聚合](../images/board-specs-github.png)

- **LOCAL**：`issues/` 目录的本地 issue 文档及 `publish target` 元数据。Board 只展示目标，不执行发布；
- **GitHub**：远端 issues 与 pull requests；
- **AtomGit**：远端 issues 与 merge/pull requests，通过 [AtomGit OpenAPI](https://docs.openatom.tech/en/category/api/) 读取；私有仓库 token 只留在 backend 的 `ATOMGIT_TOKEN`；
- 不支持或暂时不可用的 provider 仍保留为 unsynced，不把 token、绝对路径、上游错误正文送到浏览器。

统一使用 **change request** 指 GitHub PR / AtomGit MR。创建远端 issue、发布本地 issue、创建 change request 都不属于 Board v1；这些写操作仍由 Agent 工具与 owner approval 完成。

## 看板在 HAgency 里的位置

Project Board 是**只读投影**：它不发消息、不派任务、不审批，也不是授权来源。它展示的项目必须来自明确的 group→project binding，并只包含该 group 的成员、tasks/graphs 与项目工件；DM、审批详情、完整消息正文、API key 和绝对路径不得进入响应。

它回答的是“backend 记录了什么、工作树里声明了什么、远端 provider 当前可观察到什么”，而不是“workflow 一定执行到了哪一步”。要判断交付是否完成，仍需结合 Thread、backend task、Git commit、测试执行结果和 PR/MR 状态。
