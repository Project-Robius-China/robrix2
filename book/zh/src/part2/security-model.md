# 安全模型

> **定位**：本章把散落在前几章的安全机制收拢成五条原则，并给出「威胁 → 防线」对照。前置依赖：第 3 章、第 5.4 章。评估者可与前言路径 B 配合阅读。

HAgency 给了 Agent 很大的行动自由，这份自由必须配上同样坚固的边界。

## 五条原则

**1. Robrix2 永远不是授权来源。** Robrix2 只做两件事：展示（审批卡片、工作流状态）与发起（把你的点击变成结构化 Matrix 事件）。所有授权判定发生在 agent-chat 服务端：verdict 的真实发送者（`event.sender`）必须等于绑定的 owner 账号 —— 不信任 display name，不信任 payload 里自称的身份；房间、agent、project、request_id、input_digest 逐项吻合；审批绑定字段只从**原始事件**读取，`m.replace` 编辑无法篡改一张已发出的卡。即使客户端被替换或伪造，服务端校验依然成立。

**2. 审批一次性、有时效、防重放。** `Approve once` 每张卡只放行一次；服务端先消费审批再通知运行时，allow 不可重放。默认 5 分钟过期。`input_digest` 对 agent、runtime、project、project room、owner、approval room、request ID、upstream request、工具描述和最多 8KB 输入预览等规范字段做 SHA-256，把 verdict 钉在服务端保存的这一条请求记录上。

**3. Fail-closed：一切异常等于拒绝。** 从 Codex hook 到 Claude channel，链路异常不会变成 allow。Codex hook 绑定脚本 SHA-256，首次启用或 hash 变化时需在本地 TTY 输入 `TRUST`；hook timeout 由 approval TTL 加缓冲推导。Claude 依赖受管 `auto` 模式和显式 Ask 规则把受保护命令送入 channel。

**4. 加密通道与密钥卫生。** 审批房正文使用端到端加密（Megolm），正常密钥假设下 homeserver 不能读取内容，但仍可见成员、时间和流量元数据。Robrix2 在 verdict 前刷新 bridge 设备密钥并轮换出站 session，降低设备轮换导致的 UTD；bridge 对暂时解不开的 verdict 有界持久化等待 room key。任何一步失败都不会放行。

**5. 受管的运行时与最小项目范围。** Claude Code 使用 `--permission-mode auto` + channel；Codex 使用 `workspace-write` + `on-request` hook。launcher 会拒绝接管同名但没有 managed marker 的 tmux，会过滤权限策略覆盖参数；它不能阻止用户在另一个名字/终端手工启动野生 CLI。因此所有承诺都以“任务由 agent-chat launcher 启动”为前提。通过 `agentchat project add` 只暴露指定仓库或 worktree，`copy` 与 `symlink` 的写回边界必须由 owner 明确选择。

## 威胁 → 防线对照

| 威胁 | 防线 | 出处 |
|------|------|------|
| 有人在群里冒充 owner 说「同意」 | 文字回复不是审批；verdict 的 `event.sender` 服务端校验 | 原则 1 |
| 重放一张旧的批准 | 单次消费 + TTL + request_id 绑定 | 原则 2 |
| 批准 A 命令、实际执行 B 命令 | `input_digest` 内容级绑定 | 原则 2 |
| 审批链路故障导致「默认放行」 | 全链路 fail-closed，异常一律 deny | 原则 3 |
| homeserver 或网络窥探审批正文 | 审批房正文 E2EE；服务器仍见成员/时间等元数据 | 原则 4 |
| 篡改审批 hook / 绕过受管启动 | hook SHA-256 自校验 + TRUST 信任确认 + 受管 PID 标记 | 原则 3 / 5 |
| 编辑（m.replace）已发出的审批卡 | 绑定字段只读原始事件 | 原则 1 |
| 在公共项目房执行 `!ctl` / `!agentctl` 绕过 | 项目房和审批房显式拒绝这些控制命令 | 原则 1 |
| 没有配置 owner 时让管理员代批 | owner binding 为空/歧义时直接拒绝，不回退管理员 | 原则 1 / 3 |
| 普通房间消息唤醒所有 Agent | `MATRIX_DEFAULT_WAKE=off`，显式 @ 目标路由 | 原则 5 |

## 边界与残余风险

- owner 设备被攻陷后，攻击者可用真实 MXID 发送 verdict；
- backend/bridge 主机或 root 级攻击者超出应用层威胁模型；
- 8KB preview 之后的输入不会完整显示，owner 应拒绝不可读、动态拼接或来源不明的命令；
- 项目作战室当前是非加密房；不要发送不应由 homeserver/联邦成员保存的秘密；
- E2EE 隐藏正文，不隐藏成员、时间、事件大小等元数据；
- approval 保护被 launcher/Ask/hook 捕获的操作，不证明任意第三方工具都已接入审批；
- workflow 角色、主动汇报、双审顺序目前是 skill 约定，不享有审批协议同等级别的强制力。

发布验收应覆盖：唯一 owner、空 owner 拒绝、错误 sender/room/digest 拒绝、过期与重放拒绝、Claude/Codex 两条运行时路径、公共房脱敏通知、控制命令旁路被禁，以及 bridge E2EE 暂时失败时不放行。
