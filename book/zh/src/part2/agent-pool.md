# Agent 池、模型与多用户边界

> **定位**：本章说明模型在何处选择、agent-chat 已有的 role×capability 调度基础，以及多人共用一个 Matrix 房间时哪些能力不会跨实例共享。前置依赖：第 5.2、5.5 章。

## 当前如何选择模型

Claude/Codex 的模型属于**运行时进程配置**。当前可在启动时指定：

```bash
bin/agentchat up wf_implementer /path/to/worktree claude --model <model>
bin/agentchat up wf_final_reviewer /path/to/review-worktree codex --model <model>
```

dashboard 的 runtime profile 也可保存启动配置；已运行的 tmux 不会因为 Robrix2 里一句自然语言自动换模型，变更后需要受管重启。不要让 coordinator 声称“已切到某模型”，却没有核对实际 runtime/profile。

同一 Agent 同一时刻只有一个工作目录与模型进程。需要并行实现、复审和终审时，推荐维护多个 Agent，每个绑定自己的项目路径或 Git worktree，而不是反复重启同一个 Agent。

## 已有的 Agent 池

agent-chat backend 已有 role×capability pool 和 `/api/dispatch` 基础：

| role | 默认 capability |
|------|--------------------|
| architect / review | `strong` |
| coding / testing / integration | `medium` |
| documentation | `lightweight` |

调度器优先选择“满足要求的最便宜空闲 Agent”，没有候选时返回 provision plan 或进入队列。dispatch 使用 owner-bound、可续租 lease；owner/lease/agent 不匹配时不能 renew/release。当前队列与 in-flight lease 仍是进程内状态，backend 重启不是完整的 durable scheduler。

这个池是当前 backend 的资源池，不是 Matrix 房间所有成员的公共池。队友实例的 `UNREGISTERED` Agent 即使出现在同一房间，也不能被本 backend 分配本地路径、token 或任务 lease。

## 任务级模型调度的目标形态

本 session 讨论的目标交互是：

```text
“medium 实现，strong Claude 复审，strong Codex 终审”
                         ↓
Robrix2 展示结构化调度预览（Agent / runtime / model / project / worktree）
                         ↓
用户确认
                         ↓
agent-chat 从自己的模型池选择并建立 dispatch lease
```

这是合理的设计，但 **Robrix2 自然语言 → 结构化预览 → 用户确认 → `/api/dispatch`** 尚未接通，应标记为规划中。现在可复现的做法是 owner 预先创建多个 profile/Agent，由 coordinator 在 workflow 约定中选择明确的 Agent 名称。

## 多用户安全模型

多人把各自 Agent 邀请进同一个公开项目房时：

- 每个 Agent 的 owner 仍来自“谁邀请这个 Agent”的完整 MXID；
- 每个 backend 只管理自己的 Agent、项目路径、runtime profile、API token 和 dispatch lease；
- 公共房间只共享被显式发布的消息和脱敏审批状态；
- 详细审批进入各自 `(agent, owner)` 的 E2EE approval room；
- Robrix2 可以展示跨实例成员，但不能据此授予、转移或推断权限；
- 自动邀请自己的 Agent 可以作为 UI 辅助，但必须由当前登录 owner 明确确认目标 Agent 与房间，并由服务端从真实 invite event 建立 provenance，不能让 coordinator 或普通成员替 owner 静默完成。

因此，“邀请 Agent 入群”既是成员操作，也是安全操作。批量邀请 UI 如果未来实现，应先展示完整 MXID、Agent 所属 backend、目标项目房和将建立的 owner 关系，再让用户确认。
