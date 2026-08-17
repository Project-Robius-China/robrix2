# Robrix2 × agent-chat：当前已知问题与后续路线

> **状态日期：** 2026-07-24  
> **范围：** Robrix2、agent-chat backend/Matrix bridge/runtime，以及共同组成的 coding-agent workflow。  
> **代码基线：** Robrix2 `origin/main`（`08d92357`）；agent-chat `github/master`（`ad45f67`）；Project Board 本地预览提交 `3102a5f`。  
> **发布口径：** 首个完整 HAgency 版本包含严格 `@agent` 唤醒、owner 专属审批、安全 onboarding、四角色 workflow 状态和带 `agent_chat` feature 的 Robrix2 产物；Room aliases、iOS、加密项目房和自然语言模型调度不默认纳入首版。

## 状态总览

| 分类 | 优先级 | 仓库 | 是否阻塞首版 | 当前证据 | 下一步 |
|---|---:|---|---|---|---|
| rich reply 可绕过严格 `@agent` | P0 | agent-chat | 是 | bridge 会从 `reply_to` 推断 Agent | strict mention PR |
| Robrix2 本机时钟提前禁用审批 | P1 | Robrix2 | 否 | UI 用墙钟硬拦截 | approval clock PR |
| 审批失败对 owner 静默 | P0 | agent-chat | 是 | 4xx 只写日志 | approval status PR |
| Owner onboarding 仍依赖人工步骤 | P0 | 两侧 | 是 | 自动建房不能证明人类 owner | onboarding + invite UI |
| 生产 trust/command ACL 默认不 fail-closed | P0 | agent-chat | 是 | trust 默认 audit；ACL 全空会允许命令 | startup preflight/doctor |
| workflow 状态依赖 skill 自律 | P0 | agent-chat | 是 | demo 状态不进入 backend task/run | binding + durable run |
| Project Board 未发 PR | P1 | agent-chat | 否 | 本地实现和定向测试完成 | 独立推送 PR |
| dispatch queue/lease 不持久 | P1 | agent-chat | 否 | 进程内状态 | scheduler persistence |
| HAgency 发行产物未显式启用 feature | P0 | Robrix2 | 是 | Cargo 默认 feature 为空 | release/CI PR |

P0/P1/P2 只表示优先级；下文用四个互斥状态区分“真 bug、已实现能力的缺口、未来能力、已修复事项”。

## A. 已确认的当前 bug 或验收缺口

### A1. 无显式 `@agent` 的 rich reply 仍会唤醒 Agent

**优先级：P0；归属：agent-chat。**

普通项目房消息在没有明确 `@agent` 时不会唤醒 Agent；但用户用 Matrix rich reply 回复 Agent 消息时，
[`inferReplyMention()`](https://github.com/ZhangHanDong/agent-chat/blob/ad45f67/bridge-matrix.js#L2362-L2404)
会从被回复消息的 `from`、唯一 mention 或 `to` 推断 Agent，并加入 `effectiveMentions`。

该逻辑只在同 group、可信 `reply_to` 且目标唯一时生效，但仍违反当前验收条件：

> 公共项目房只有明确 `@` 具体 Agent 才允许其执行命令和发言。

修复要求：

- 默认关闭 group reply inference；兼容模式只能显式 opt-in。
- Agent/人类 DM 不受此规则限制。
- 覆盖顶层消息、Thread reply、rich reply 和歧义目标测试。

[mention-only PR #7](https://github.com/ZhangHanDong/agent-chat/pull/7) 已修复普通顶层消息，但没有移除这一旧逻辑。

### A2. Robrix2 使用本机墙钟硬性禁用审批

**优先级：P1；归属：Robrix2。**

Robrix2 使用本机时间与 `expires_at` 比较，并在点击时硬性拦截过期请求，见
[`src/home/room_screen.rs`](../../src/home/room_screen.rs)。本机时钟偏快时，backend 仍认为有效的请求可能被 UI 提前拒绝。

修复应使用 Matrix event/server timestamp 估算偏移，或让客户端过期只作提示并把 verdict 交给 backend 最终裁决。服务端 TTL、单次消费和完整字段校验不能放宽。

## B. 已实现能力的可靠性或产品化缺口

### B1. 审批终态与失败原因对 owner 不可见

**优先级：P0；归属：agent-chat。**

Claude 与 Codex 的成功链路已经跑通；但 verdict 被 backend 以 400/401/403/404/409/410 拒绝时，
[`onApprovalVerdict()`](https://github.com/ZhangHanDong/agent-chat/blob/ad45f67/bridge-matrix.js#L3091-L3112)
只记录事件和日志，不会在 owner approval room 显示 expired、sender/room/digest mismatch 或 not pending。

应从 backend 持久化 approval record 构造幂等的 `com.agentchat.approval.status.v1`：

- 私密 approval room 显示详细终态；公共项目房只显示脱敏终态。
- 路由不得信任 verdict payload 自带的 agent/project/room。
- 404 或记录缺失时不得向 payload 指定房间发送状态。
- Robrix2 只把可信 bridge sender 的结构化事件渲染为系统状态。
- 状态发送失败不得改变服务端 fail-closed 结果。

### B2. Owner onboarding 尚未成为安全的默认产品路径

**优先级：P0；归属：两侧。**

权威关系是：

```text
(project room, exact agent)
    → 邀请该 Agent 的 event.sender 完整 MXID
    → 该 Agent 在该项目房的 owner
```

`!bindroom` 只建立 room → group 映射。由 bridge 自动建房或邀请 Agent，不能证明某个人类开发者是 owner。生产验收还要求邀请者属于 `MATRIX_TRUSTED_INVITER_MXIDS`；否则 enforce 模式拒绝加入，audit 模式即使加入也不会建立可信 owner binding。

下一步：

- agent-chat 提供 backend-only group 或 `--no-auto-room`。
- Robrix2 显示准确 Agent MXID、目标房间和预期 owner，由登录的人类账号确认并发送邀请。
- doctor 校验 `(room, agent) → owner`、Agent membership 和 approval room ready。
- 禁止通过任意请求 payload 直接写 owner。

### B3. 生产 trust 与管理命令 ACL 仍需 fail-closed 启动门

**优先级：P0；归属：agent-chat。**

当前 `MATRIX_TRUST_MODE` 默认是 `audit`；`MATRIX_OPERATOR_MXIDS` 与 `MATRIX_ADMIN_MXIDS` 同时为空时，命令 ACL 为兼容旧部署而允许命令。项目房和审批房虽然禁止 `!ctl`/`!agentctl`，普通 Agent DM 仍可能暴露终端控制路径。

首版生产模式必须：

- `MATRIX_TRUST_MODE=enforce`。
- `MATRIX_TRUSTED_INVITER_MXIDS` 非空。
- `MATRIX_OPERATOR_MXIDS`/`MATRIX_ADMIN_MXIDS` 按职责配置，不能同时为空。
- startup preflight 或 doctor 在配置缺失时拒绝生产启动。
- 保留本地开发模式时必须双重显式开启并显示醒目警告。

### B4. 现有 workflow 的状态上报不可靠

**优先级：P0；归属：agent-chat。**

四角色 demo 可以运行，但 Robrix2 的 `/workflow-*` 只是文本补全；角色主要由 Agent 名字和 skill 约定决定；状态写入 `.agentchat-demo/state.json`，Project Board 不读取它。Agent 离线、会话中断或 relay 异常时，“主动汇报”没有系统保证。

首版需要版本化 workflow binding 和持久化 run：

```text
definition/version + project/thread
    → run/phase/role/assignee/worktree
    → blocked/approval/review verdict
    → durable transition
    → Project Board + 原 Thread 状态
```

写权限必须明确：

- 启动/取消由 owner/operator 的完整 MXID 授权。
- phase transition 只接受绑定角色的 Agent token。
- 使用合法迁移表、幂等键、版本/CAS 和 append-only audit。
- backend 是权威状态源；Robrix2 只展示并提交经确认的操作。

### B5. Project Board 本地实现完成，尚未交付

**优先级：P1；归属：agent-chat。**

本地 `feat/project-board` 比 `github/master` 领先提交 `3102a5f`，尚未推送同名远端分支，也没有 PR。GitHub/AtomGit、worktree、spec、本地/远程 issue、PR/CR、tasks/graphs/activity 等定向测试为 16/16，完整测试套件仍待在干净环境运行。

v1 边界：

- `workflow_bindings.json` 需手工准备，没有 operator ACL CRUD/UI。
- 不读取 `.agentchat-demo/state.json`。
- task 缺少明确 project ID，多 group Agent 可能产生重复投影。
- 看板只读，不负责发布 issue 或创建 PR/CR。
- spec tests 是声明映射数量，不是实际执行或覆盖率。

现有提交可以先独立推送和发 PR；workflow binding 与 durable run 分开提交。

### B6. Agent Pool 的 queue/lease 不持久

**优先级：P1；归属：agent-chat。**

`/api/pool`、`/api/dispatch`、role/capability、lease 和 provision plan 已存在，但 queue/lease 是进程内状态，backend 重启后不能可靠恢复。

持久化时，owner 必须从认证主体派生，不能接受调用方自由填写或生产环境 default owner；随后再补 restart recovery 和 planning API。

### B7. 完整 HAgency 发行产物尚无明确构建门

**优先级：P0；归属：Robrix2 Release/CI。**

Cargo `agent_chat` feature 默认关闭，见 [`Cargo.toml`](../../Cargo.toml)；当前
[release workflow](../../.github/workflows/release.yml) 没有明确为 HAgency artifact 启用它。

应新增独立 release/CI PR，构建带 `agent_chat` 的目标产物，并对实际 artifact 验证 Preferences、workflow 命令、approval card 和 Thread，而不只验证默认 feature 的开发构建。

## C. 后续能力与一般 backlog

以下未纳入当前首版，不应登记为现有能力回归：

### C1. Robrix2 自然语言模型调度

目标流程是把“medium 实现、strong Claude 复审、Codex 终审”转换为结构化计划，展示 runtime/model/project/worktree/permission，用户确认后再调用持久化 scheduler。当前尚未接入 Robrix2。

单个 agent-chat backend 只能调度注册在自身实例中的 Agent，不能管理同房其他开发者的本地 Agent。

### C2. 加密项目房的 Agent Thread

[Thread continuity PR #9](https://github.com/ZhangHanDong/agent-chat/pull/9) 已覆盖非加密 group 房间的 thread context、Matrix relation、delivery journal、重启恢复和多 Agent 多跳。

普通加密项目房的 Agent 出站尚不走 crypto client；approval DM 使用独立 E2EE 路径。旧消息缺少 delivery metadata 时降级为顶层回复属于兼容策略。

### C3. Room aliases

[Issue #266](https://github.com/Project-Robius-China/robrix2/issues/266) 仍 OPEN。`feat/room-aliases` 已推送但无 PR，独立 worktree 还有未提交、未贯通的 generation/reconcile 竞态修复。它不属于当前 HAgency 首版，完成接线、编译、alias gate 测试和用户验收后应独立发 PR。

### C4. Robrix2 常规 backlog

| 条目 | 分类/归因 | 状态与后续 |
|---|---|---|
| [跨 homeserver 401 storm](https://github.com/Project-Robius-China/robrix2/issues/157) | 调查中，尚未归因 | 记录准确 URL、homeserver、errcode、token 所属账号、频率和停止条件；区分客户端 token 路由与 Palpo/federation |
| [SSO provider 硬编码](../../issues/006-sso-provider-list-is-hardcoded.md) | 已确认客户端缺口 | 动态读取 Matrix login capabilities 和真实 provider ID |
| [typing subscription 生命周期竞态](../../issues/007-room-timeline-typing-subscription-race.md) | 已确认客户端缺口 | 对 room registry 延迟排队/重试，收紧 timeline attach/detach |
| [idle sliding-sync 循环](https://github.com/Project-Robius-China/robrix2/issues/30) | 调查中 | 定位空闲请求来源与停止条件 |
| [4 个 ignored bot 测试](../../issues/011-upstream-bot-test-failures.md) | 未裁决 | 分别判断测试过时或用户行为回归 |
| [Settings dropdown 箭头](../../issues/002-settings-dropdown-arrow-visual-artifact.md) | 已确认低优先级 UI bug | 后续修复 |
| Dock splitter/多窗格恢复 | 未来增强 | 崩坏路径已绕过；不再调用 `Dock.load_state()` |
| 历史已离房 tab 数据清理 | 产品化清理 | 新离房行为已修；增加一次性迁移 |

最近 v1.1 release 的 Android、Windows、macOS 和 Linux job 成功，iOS packaging 失败：
[GitHub Actions run 29935273011](https://github.com/Project-Robius-China/robrix2/actions/runs/29935273011)。
当前 run 没有足够日志证明具体根因。iOS 不属于本次首版；若未来承诺 iOS artifact，再单独完成签名、IPA/TestFlight 和真机持久化验收。

## D. 已修复，不再作为当前 bug

- Robrix2 邀请实时出现，无需重启。
- 普通顶层消息无 `@agent` 不再唤醒；仅剩 A1 的 rich reply 例外。
- Owner 专属审批、完整 `event.sender` MXID、agent/project/request/digest/TTL/单次消费校验。
- 审批者为空或绑定歧义时 fail-closed，不回退给管理员。
- 公共房只显示脱敏 waiting；详细请求进入 owner approval room。
- 项目房/审批房的 `!ctl`、`!agentctl` 不能绕过审批。
- Claude auto-mode channel；Codex PermissionRequest hook、显式 TRUST 和 TTL timeout 联动。
- Claude/Codex 默认 coding-agent 沙箱。
- OTK 对账、crypto store/device identity 和延迟 room key 恢复。
- Claude 与 Codex 的加密 owner approval 成功闭环。
- 非加密 group Thread continuity、delivery journal 和 bridge 重启恢复。
- Dashboard `/msg/:id?view=...` capability link。
- [Owner approval PR #8](https://github.com/ZhangHanDong/agent-chat/pull/8) 与 [Thread continuity PR #9](https://github.com/ZhangHanDong/agent-chat/pull/9) 已合并。

2026-07-24 重新执行的 agent-chat 定向验证覆盖 7 个测试文件，131/131 通过；Project Board 4 个定向测试文件 16/16 通过。

## 建议的 PR 顺序

1. **agent-chat Project Board**：先交付现有独立提交并跑完整测试。
2. **agent-chat strict mention**：默认关闭无显式 `@agent` 的 reply inference。
3. **agent-chat approval status**：私密详细终态、公共脱敏终态。
4. **agent-chat production preflight**：enforce trust、trusted inviter、operator/admin ACL。
5. **agent-chat owner-safe onboarding**：backend-only group/no-auto-room 与 doctor。
6. **Robrix2 owner invite UI**：完整 MXID 预览与人类确认。
7. **Robrix2 approval clock**：消除本机墙钟硬拦截。
8. **agent-chat versioned workflow binding**：operator ACL 和角色/能力映射。
9. **agent-chat durable workflow run**：状态机、CAS、audit、Thread anchor。
10. **workflow skill/status integration**：写入 backend 并自动投影到 Thread/Board。
11. **Robrix2 HAgency release/CI**：显式启用并验收 `agent_chat` artifact。
12. **agent-chat dispatch persistence**：认证 owner、queue/lease 和重启恢复。
13. **Robrix2 Room aliases**：完成竞态修复后独立提交。
14. **Robrix2 model dispatch preview**：scheduler 稳定后再接入。

## 维护规则

每个条目必须记录状态分类、优先级、权威来源和关闭证据。安全或恢复类问题需要 fix commit/PR、自动化测试和必要的 Palpo 在环验证；未实现的产品目标不能登记为回归 bug。Robrix2 始终只负责展示和提交操作，owner、approval、workflow 与 dispatch 权限必须由服务端基于认证身份、完整 MXID 和持久化记录判定。
