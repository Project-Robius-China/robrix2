# 附录：术语与能力状态

> **定位**：本附录统一 Matrix、agent-chat 与 workflow 中容易混淆的术语，并给出当前能力成熟度速查。

## 术语

| 术语 | 含义 |
|------|------|
| MXID | 完整 Matrix 用户 ID，如 `@alex:matrix.example.com`；授权校验不使用 display name |
| bridge bot | agent-chat 实例的 Matrix companion 账号，负责命令、桥接与审批加密发送 |
| puppet / 木偶 | 每个 Agent 的 `@ac_<name>` Matrix 账号 |
| trusted inviter | bridge 允许其邀请自己进入房间的完整 MXID |
| operator | 可运行 `!bindroom` 等管理命令的 MXID；不因此获得任意 Agent 的审批权 |
| owner | 在特定项目房邀请特定 Agent 的真实 `event.sender`；关系是 `(room, agent)→MXID` |
| group | agent-chat backend 的成员/消息分组 |
| project room | 绑定 group 的 Matrix 作战室；当前 Agent 出站要求非加密 |
| ordinary DM | 人与 Agent 的一对一消息房，按需创建 |
| approval room | 按 `(agent, owner)` 创建/复用的 E2EE 房，只承载结构化审批 |
| request / verdict | 审批请求事件与一次性决定事件 |
| digest | 对服务端规范化请求字段的 SHA-256 绑定 |
| TTL | 审批有效窗口，默认 5 分钟 |
| Olm / Megolm / OTK | Matrix 设备会话、房间加密与一次性密钥机制 |
| managed project | `agentchat project add` 暴露给 Agent 的 copy/symlink 项目 |
| worktree | Git 原生独立检出；不会由 `project add` 自动创建 |
| workflow binding | Project Board 的 group→project/workflow 只读配置；当前不是角色授权 API |
| capability | `strong` / `medium` / `lightweight` 调度层级 |
| dispatch lease | backend 为一次 pool dispatch 建立的 owner-bound、可续租占用 |

## 当前能力状态

| 能力 | 状态 |
|------|------|
| Matrix group mention routing | 当前实现；共享房默认 explicit mention |
| owner approval、TTL、single-use、server validation | 协议强制；前提是受管 runtime 与唯一 owner |
| E2EE approval room | 当前实现；可能受设备/key delivery 延迟影响，失败时拒绝 |
| 非加密项目房 Thread reply continuity | 当前实现；需要可信 `reply_to` |
| workflow 自动主动汇报 | 工作流约定，不是 transport 保证 |
| 四角色 issue-workflow | 实验性共享 skill；名字决定角色 |
| 持久化 role binding / workflow engine | 规划中 |
| Project Board | `feat/project-board` 预览；只读，不自动读取 demo state |
| GitHub + AtomGit 工件观察 | Project Board 预览已实现 |
| role×capability pool 与 backend dispatch | 当前 backend 基础；队列非重启持久 |
| Robrix 自然语言按任务选模型并确认调度 | 规划中 |
| 加密项目房的 Agent Thread 出站 | 尚未支持 |

## 实现证据索引

阅读代码或做安全复核时，优先从这些权威工件开始：

- agent-chat `bridge-matrix.js`：Matrix 邀请 provenance、mention routing、Thread relation、审批房与 E2EE；
- agent-chat `lib/approval-store.js`：owner 选择、digest、TTL、single consume 与 verdict 校验；
- agent-chat `lib/agent-launch-policy.js`、Codex permission hook：受管运行时策略；
- agent-chat `specs/task-matrix-thread-continuity.spec.md`：Thread 正常、降级、跨房间与重启窗口；
- agent-chat `specs/task-project-board.spec.md`：Project Board v1 隐私、provider 与 out-of-scope；
- agent-chat `lib/matrix-agent.js`：role×capability pool；
- Robrix2 `src/sliding_sync.rs` 与 `src/home/room_screen.rs`：verdict 发送、设备刷新和审批卡片 UI；
- Robrix2 `roadmap/agentchat-demo/issue-workflow/SKILL.md`：当前实验性 workflow 的名字分支与汇报约定。

代码与书发生冲突时，以固定 commit 的实现和 spec 为准，并把差异作为文档 bug 修复；不要用截图覆盖代码事实。
