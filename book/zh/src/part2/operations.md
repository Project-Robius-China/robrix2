# 运行验收与故障排查

> **定位**：本章给出从 Matrix 入站、经 thread-session 路由与一次性 runner、再到审批回传的分层检查表。适用于“Agent 不回复、Thread 跑偏、卡片不出现、点击后仍过期”等问题。

## 发布前验收清单

先记录以下事实，避免用显示名或截图猜状态。建议把这张表随每次测试保存成短报告；当 homeserver、bridge 设备或运行时版本变化时，新旧结果才可对账：

| 项目 | 需要记录 |
|------|---------|
| 版本 | Robrix2 commit、HAFleet commit、homeserver 版本与日期 |
| 账号 | human/bridge/每个 `@ac_*` 的完整 MXID |
| 绑定 | room→group；每个 `(room, agent)→owner`；可选 group→project |
| 运行时 | agent 名称、Claude/Codex、model、project path/mode、thread-session 开关状态 |
| 房间 | 非加密项目房；每个 `(agent, owner)` 的 E2EE approval room |
| workflow | skill 版本、四个角色命名、worktree/commit SHA |

最小端到端验收：

1. 顶层无 @ 消息不唤醒 Agent；显式 @ 只唤醒目标 Agent；
2. 在 Thread 中 @，直接回复仍在同一 Thread，主时间线不重复显示；
3. bridge 重启后做第二跳 reply，delivery journal 能延续 Thread；
4. Claude 触发一次受保护命令：公共房只见脱敏 waiting，owner approval room 有 pending 卡片；
5. `Approve once` 后命令执行一次；重放同一 verdict 被拒绝；
6. Codex 首次完成 `TRUST`，沙箱内普通读写不弹审批，越界操作出现卡片；
7. 错误 owner、错误房间、过期卡片、空 owner binding 全部 fail-closed；
8. `!ctl` / `!agentctl` 在项目房和审批房不能旁路审批；
9. dashboard 的 Agent/Tasks/Pool 与 Git/worktree 实际状态对账；
10. workflow 最终结论同时核对 commit、测试命令结果与 PR/MR，而不是只看 Agent 文本。

## Agent 不回复

按链路逐层检查，不要直接重启所有服务：

```text
Matrix event
  → explicit mention / trusted room
  → bridge ingestion
  → backend 消息 → thread-session 路由(每 (Agent, Thread) 一个会话)
  → dispatch 入队(queued → leased → started → completed)
  → 孵化一次性 runner
  → runner 回复 → reply outbox
  → bridge 领取出站 → Matrix puppet send
```

- 房间里是否真的 @ 了完整目标；`MATRIX_DEFAULT_WAKE` 是否为 `off`；
- Agent 和 companion bridge 是否都 joined；邀请轮询默认可能约 60 秒；
- `hafleet ls` 是否已注册；backend `/health` 的 agents 数是否非零（为 0 通常是 `HAFLEET_RUNTIME_DIR` 指错了数据目录）；
- dispatch 一直停在 `queued`：看 backend 日志的 `[router-runner]` 行。`agent token is unavailable` = `HAFLEET_HOMEDIR` 与 Agent 家目录不符；模型 400 = `/thread model` 指定了与框架不兼容的模型；
- dispatch 落入 `outcome_unknown`：runner 在可能已开始工作后异常终止（如模型报错、进程被杀），该会话 fail-closed 锁定，需要 operator 走 `POST /api/router/dispatches/:id/outcome-inspection` → `resolve-outcome`（action `continue` 附恢复说明）裁决后自动重跑；
- Agent 回复说自己在 plan mode：预期行为——聊天回合只读，写权限走任务流或 `/thread mode auto`。

## Thread 回复掉到主时间线

检查入站消息的 `matrixContext`、被引用消息的 `matrixDelivery.primaryEventId` 和本地 delivery journal。三类结果要区分：

- reply target 属于另一房间：安全错误，拒绝发送；
- 旧消息/回写失败导致 delivery 缺失：降级顶层发送并记录 warning；
- Agent 主动 `post(group=...)` 没有 `reply_to`：这是 workflow 调用缺上下文，不是 Matrix 客户端丢 Thread。

项目房如果开了 E2EE，当前 Agent group 出站路径不支持，应迁移到非加密作战室；审批 E2EE 不受此限制。

## 审批卡片没有出现

从入口向下确认：

1. backend 是否实际出现 pending approval 是唯一判据（runner 的审批经 MCP 审批通道进 backend，不再有本地 tmux 弹框）；
2. Claude runner 是否带审批 channel 启动（backend 固定注入）；Codex hook 是否 trusted、hash 是否匹配；
3. approval store 是否找到唯一 `(room, agent)→owner`，否则是 `owner_binding_missing/ambiguous`；
4. owner 是否已加入 approval room，否则是 `owner_invite_pending`；
5. bridge 是否发送 `com.hafleet.approval.request.v1`；E2EE 是否有 queued UTD；
6. Robrix2 是否已同步、解密并渲染 custom event。

不要用聊天文字“批准”代替卡片，也不要以任何本地方式绕过 Matrix 验收。

## 点击批准但运行时仍显示过期/拒绝

对账 `request_id`、`expires_at`、Matrix `event.sender`、approval room ID、agent/project/project room 与 `input_digest`。默认 TTL 是 5 分钟；Agent 重试会生成新 request，旧卡即使仍在 UI 中也不能批准新请求。

查看 backend audit 的最终 rejection code 和 bridge 的 verdict 日志。E2EE key 延迟可能让 verdict 到达时已过期；设备刷新/session 轮换降低概率，但不是“必定解密”。不要自动重试外部写操作，先确认旧请求已终态且没有产生副作用。

## 运维状态看哪里

| 想知道什么 | 权威/主要证据 |
|------------|---------------|
| Agent 是否可用 | `hafleet ls` 注册状态 + backend `/health`（thread-session 模式下空闲无进程是常态） |
| 消息是否送达 backend | backend message/inbox |
| workflow 到哪一步 | workflow state + durable task + Thread；当前没有单一权威源 |
| 某轮对话的执行状态 | router 的 dispatch 记录（queued/started/completed/outcome_unknown） |
| 谁能审批 | bridge owner binding + 原始 Matrix invite sender |
| 审批结果 | backend approval store/audit |
| 代码是否完成 | Git commit/worktree + 实际测试结果 |
| 远端是否发布 | GitHub/AtomGit issue 或 change request 状态 |

Project Board 是聚合视图，不替代这些源记录。状态冲突时按表中的权威证据逐层定位。
