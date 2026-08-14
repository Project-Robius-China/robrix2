# Robrix2 Agent Operations 展示模型实验 R3（非 V1 合同）

状态：**Historical Design Experiment / 非 canonical / 不可用于生产接线**  
实验 schema：`io.robrix.agent_ops.proposal.r3`  
日期：2026-08-05

agent-chat 拥有 canonical 合同命名空间 `io.agentchat.agent_ops.v1`。本文档只保留
Robrix2 早期展示与安全设计思考，不得定义、覆盖或伪装成该 canonical wire
contract。对应 Rust 类型只在 `cfg(test)` 下编译。

## 文档语义

| 类别 | 含义 |
|---|---|
| Current Fact | 两个仓库中已能核实的当前事实，不因本提案而改变 |
| Current Gate Requirement | Robrix2 当前必须执行的安全门控，已经落地并可测试 |
| Proposed Future Contract | 未来跨仓库协议草案；只有进入 agent-chat Accepted ADR/REQ、canonical artifacts 和实现后才有约束力 |

除明确标为 Current Fact 或 Current Gate Requirement 的内容外，第 3–8 节中的“必须/不得”
都表示 **Proposed Future Contract**，不能被解释为 agent-chat 已支持这些字段或端点。

## 1. Current Fact：为什么不能直接接 Dashboard router

agent-chat 当前接受的线程会话需求与 ADR 规定：V1 router 读端点留在本地 Dashboard 的
认证边界内；Robrix2 若要访问，必须先有单独获批的客户端认证合同。当前
`/api/router/*` 使用 backend-wide `API_TOKEN`，把它交给桌面客户端等于交出 operator
权限。

当前两个仓库还存在实际线格式和状态机差异：

- Robrix2 proposal 使用 `schema + scope/projection/stream + seq +
  attention/tasks/queue/worktrees` 的 UI 投影；
- agent-chat Dashboard 当前返回 `schemaVersion + lowWatermark/highWatermark +
  sessions/tasks/dispatches/resources/attention`；
- 当前 Dashboard event endpoint 返回带 watermarks、gap 与 events 数组的 EventPage；旧原型
  曾使用不同的失效 envelope。当前 proposal fixture 使用 `schema + projection/epoch + seq`，
  两者都不是已接受的客户端合同；
- 清理 dirty 的真实对象是 `resource_id`，不是 session/worktree 标识；
- `outcome_unknown` 不能靠再次 cancel 解决，而要进入结果检查与显式消歧流程。

## 2. Current Gate Requirement：Robrix2 保持 fail closed

在 agent-chat 的 canonical 合同被发布、绑定并由 Robrix2 校验前，Agent Operations 面板：

- 只显示“合同未就绪”，不展示真实 operational rows；
- 不发网络请求、不轮询、不注册 command transport；
- 不采集、保存或迁移 Dashboard token、router bearer、bridge secret 或其他后台凭据；
- 不提供 mutation、approve 或 deny 按钮；
- proposal model 和 fixtures 只在 `cfg(test)` 下编译，不导出为生产 API，
  也不被运行时 panel 引用。

## 3. 拟议的安全、认证与传输边界

### 3.1 权威、部署与数据面

1. Robrix2 是展示与请求客户端，不是 session、task、dispatch、lease 或审批的授权源。
2. 审批仍只在既有 owner-DM 审批链中完成。Agent Operations 不构造 approve/deny；最多
   提供跳转入口。
3. V1 operational client 仅支持 Robrix2 与 agent-chat **同机桌面部署**。远程桌面和移动端
   数据面不在 V1 范围内，必须另定传输合同。
4. Matrix 作为认证控制面，只承载 bootstrap、低体积失效/撤销通知和 audit correlation；
   snapshot、inspection 和 command 走 scoped loopback HTTP 数据面。
5. loopback 只允许明确的 `127.0.0.1` 或 `[::1]` endpoint，禁止重定向，并冻结 Host、
   Origin/CORS、audience 与服务端身份校验规则。含 capability 的响应必须
   `Cache-Control: no-store`，不得进入访问日志、telemetry、代理缓存或崩溃报告。
6. 任何失败都不能降级到 Dashboard `API_TOKEN`、未加密 Matrix 消息、公共房间或在 DM
   timeline 中永久保存数据面 capability。

### 3.2 精确授权 scope

一个 session capability 只授权一个 scope：

```text
scope = owner_mxid + owner_dm_room_id + project_room_id + stable_agent_id
```

snapshot 顶层必须携带相同的 `scope_id` 和展开后的 scope。全局面板只能在客户端安全拼接
多个独立 scope 的快照，不能把一个 scope 的 capability 用于另一个 scope。若未来需要后台
聚合，必须另设带显式成员列表的 aggregate scope，不能隐式扩大权限。

“DM”不是安全谓词。正式 ADR 必须冻结：owner MXID 权威来源、exact room ID、允许成员集合、
完整 `event.sender` 比较、设备 ID/key、cross-sign verification、`was_encrypted` 与 crypto
verification state。当前 bridge ingestion 没有提供完整设备/加密证明，因此这是 agent-chat
的前置实现变更。成员变化、关闭加密、binding 重配、设备 blocked/deleted/unverified、logout、
project/agent 删除或 backend key rotation 时都必须提升 client-auth revocation epoch 并拒绝旧请求。

### 3.3 Bootstrap 与 proof-of-possession

正式 ADR 必须把 bootstrap 写成可测试 wire protocol，而不只是流程叙述：

1. Robrix2 在绑定的 encrypted owner DM 发送专用 control event（建议
   `io.agentchat.agent_ops.client_session.request.v1`），包含合同版本、请求 scope、
   `client_nonce`、一次性 ephemeral public key 和客户端支持的 PoP profile；不能复用普通
   chat message ingestion。
2. agent-chat 在验证 sender/room/scope/device/E2EE 后签发 server-owned
   `client_session_id`，不得信任客户端自报 session id。响应 grant 绑定 owner、room、
   project、agent、Matrix device、client key、audience、scope、expiry、唯一 jti、
   server challenge 与 client-auth revocation epoch。
3. grant 同时提供精确 loopback exchange endpoint 和可验证的服务端 key/certificate
   fingerprint。敏感 grant payload 必须使用经安全评审的标准 profile 密封给 ephemeral key；
   不得自创加密格式。
4. exchange 请求必须以 ephemeral private key 对 `grant_jti + client_nonce + server_challenge +
   HTTP method/path + canonical body digest + audience` 做 sender-constrained/DPoP 类证明。
   仅持有 bearer grant、能读取 DM 或能抢占本地端口都不足以兑换。
5. grant 原子消费一次。换得的短期 session capability 只驻留内存；每次后续 HTTP 请求都
   携带新 nonce 的 sender-constrained proof。服务端在每次请求中重查 scope、expiry 和
   client-auth revocation epoch，而不是把 Matrix revoke 通知当作授权判断。

具体算法、canonicalization、key confirmation、nonce/replay cache、时钟偏差、endpoint
discovery 和 key rotation 必须经 threat model 与安全评审后进入 Accepted ADR；本提案不选择
自定义密码算法。

### 3.4 重放、幂等与撤销

- 每个 mutation 携带全局唯一 `request_id`。agent-chat 对**解析后的规范字段**计算 semantic
  digest，不依赖 JSON 字段顺序；同 id 同 digest 返回原结果，同 id 不同 digest 返回
  `idempotency_conflict`。
- backend-issued opaque action capability 绑定 scope、projection、stream epoch、client
  session、auth fence、action kind、target entity/version、resource generation、allowed
  resolutions、expiry 和 jti。它不必是自包含签名 token，也可由后台散列存储，但验证语义相同。
- capability 验证、实体 CAS、业务 mutation、idempotency result 写入和 capability 消费必须
  在同一事务中提交。result retention 必须长于重试窗口，以覆盖“已提交但响应丢失”。
- Matrix `event_id` 持久去重；撤销状态由服务端持久 fence 强制执行，不能只依赖客户端删 token。

## 4. 拟议的只读投影与失效流

每份 snapshot 只对应一个授权 scope，并至少包含：

- `schema: "io.robrix.agent_ops.proposal.r3"`；
- `scope` 与 `scope_id`；
- `projection_id`：稳定标识该 projection；
- `stream_epoch`：持久 UUID，数据库恢复/替换或无法保证序列连续时必须更换；
- `auth_fence_generation`：客户端认证撤销代次，区别于现有 dispatch/runner fence；
- `seq`：仅在同 `scope_id + projection_id + stream_epoch + auth_fence_generation` 内单调递增；
- `attention/tasks/queue/worktrees` 和 backend-issued `available_actions`。

`seq`、version、generation 与 Unix 时间戳在线上均为 JSON safe integer
（`0..=2^53-1`）；`expires_at_unix_ms` 明确使用 Unix epoch 毫秒。scope、epoch 或 fence
变化时 Robrix2 清空全部可操作数据并重新 bootstrap，不能把低序号猜成回滚。

投影字段语义：

- `attention` 只含后台明确标记需要人处理的条目；
- `tasks` 携带标题、stable agent id、后台状态、运行时间和完整 Matrix thread reference；
- `queue` 是后台生成的阻塞链，不允许客户端从 lease/event 自行推导；
- `worktrees` 使用 `resource_id`、entity version、`dirty_generation`、安全 label 与 branch，
  不含真实路径；
- 每个 `available_action` 使用统一 `target { entity_kind, entity_id, entity_version }`、可选
  `resource_precondition { resource_id, dirty_generation }`、expiry 和 opaque capability；
- `resolve_outcome` 还必须显式携带 `allowed_resolutions`。客户端只显示该集合，不从 task
  状态猜测；non-task dispatch 只能收到 `continue`。

Matrix event 只发失效通知，携带同一 scope/projection/epoch/fence 和新 `seq`。Robrix2 收到
更高序列后重新请求完整 snapshot，不在本地合并 router 状态。schema、scope、epoch、fence、
序列或解析任一不匹配时停止展示可操作数据并重新同步。

`snapshot_seq` 只是视图基线，不是 mutation 授权。dispatch/resource/task 或 action-relevant
aggregate 必须有持久 entity version；任何影响展示字段、动作资格或 CAS 的事务都必须 bump。
资源 dirty 状态变化必须 bump `dirty_generation`。当前 agent-chat 没有完整的 entity-version
投影，这是 Accepted 前的后台实现工作。

## 5. 拟议的隐私合同

### 5.1 后台规范性过滤

agent-chat 必须在数据离开后台前，从所有可展示字符串和结构化字段删除：绝对路径、
home-relative 路径、worktree 实际目录、bearer/secret、环境变量值、命令输出凭据、审批正文、
可用于构造审批副作用的字段，以及其他 owner scope 的任务/资源/房间信息。

### 5.2 客户端纵深防御

Robrix2 对完整 projection 做路径启发式检查；失败时拒绝整份 projection。该检查不能可靠识别
所有编码、Unicode 绕过或 secret，因此不替代后台过滤，也不声称是通用 secret detector。

## 6. 拟议的命令 envelope 与事务语义

每个后台 mutation 使用同一公共 envelope：

```text
schema
request_id
client_session_id                 # backend 签发
scope_id
projection_id
stream_epoch
auth_fence_generation
snapshot_seq
action_capability
target { entity_kind, entity_id, entity_version }
resource_precondition? { resource_id, dirty_generation }
```

capability 已绑定的字段仍在请求中显式回传，便于审计和 semantic digest。服务端必须在同一事务
比较 capability binding、请求字段和当前数据库状态，并重查 session/resource/task 级业务条件；
任一不匹配返回稳定错误并要求刷新，不允许客户端自动改变参数重试。

- `cancel_dispatch`：只在 snapshot 明确提供时请求；`outcome_unknown` 不再提供 cancel。
- `mark_resource_inspected`：只清相同 generation 的非 quarantine dirty。若资源由
  outcome_unknown dispatch 隔离，返回 `inspection_required`。
- `begin_outcome_inspection`：本身是幂等 mutation，必须使用公共 envelope；响应丢失后同一
  request 可安全取得同一结果。
- `open_thread`：纯客户端导航，不是 mutation。

删除 worktree/branch、启动或恢复 runner、approve 与 deny 不在 V1。

## 7. 拟议的 `outcome_unknown` 恢复闭环

1. snapshot 为 dispatch 提供 `begin_outcome_inspection` action，并绑定 dispatch version 与
   `resource_id + dirty_generation`。
2. 成功 response 返回 scope/projection/epoch/fence、server session id、`inspection_id`、
   一次性 `inspection_token`、`expires_at_unix_ms`、dispatch target、`task_id`（可空）、
   `terminal_reason`、隐私过滤后的 resource context，以及新的 `resolve_outcome` action。
3. resolution action 显式列出 `allowed_resolutions`。task dispatch 可按后台状态授权下列集合；
   non-task dispatch 只授权 `continue`。

| resolution | 必填字段 | 后台语义 |
|---|---|---|
| `continue` | inspection id/token、operator note、recovery instruction | 消费 inspection，清同 generation quarantine，创建或复用经验证的 queued replacement dispatch；绝不重放原 started dispatch |
| `accept_completed` | inspection id/token、operator note | 消费 inspection，清同 generation quarantine，将关联 task 接受为完成；non-task 不允许 |
| `keep_blocked` | inspection id/token、operator note | 消费 inspection，清同 generation quarantine，使关联 task 保持 blocked；non-task 不允许 |

4. resolution request 使用公共 envelope，并显式回传 inspection id/token、resource
   precondition 和选择。`continue` 的 recovery instruction 必须非空；另两种携带该字段必须
   被严格反序列化拒绝；三种 operator note 都必须非空。
5. agent-chat 原子验证 token 未过期/未消费、所选 resolution 在 capability 的允许集合中、
   dispatch 仍为 outcome_unknown、workspace generation 未变、同 session 无其他 active runner，
   再提交 resolution。客户端不能只靠 dispatch version 替代这些事务内检查。
6. 成功 response 字段冻结为 `resolution`、`task_id`、可选 `replacement_dispatch_id` 和
   `idempotent_request_replay`。这里的 replay 只表示同 request digest 返回既存结果，绝不表示
   重放原 started dispatch。状态仍以新 snapshot 为准。

`inspection_token` 和 action capability 只驻留内存，不进日志、AppState 或 Matrix timeline。
三个本地 resolution fixtures 是互斥案例，各自使用独立的一次性 token/capability。

## 8. 拟议合同的 canonical artifacts 与发布门槛

Robrix2 仓库中的 `specs/fixtures/agent-ops-client-v1-proposal/` 是
**Non-canonical proposal fixtures**，只示例字段并做本地类型回归，不构成已接受线格式。
`specs/fixtures/agent-ops-client-v1/` 专门保留给 agent-chat 发布的 canonical manifest
与未来逐字节 vendored artifacts，两类材料不得混放。

Accepted 合同必须由 agent-chat 发布以下 canonical artifacts：

| Artifact | 最低内容 |
|---|---|
| `manifest.json` | 合同版本、agent-chat source commit、digest algorithm、每个 artifact 的 SHA-256 |
| control-plane fixtures | bootstrap request、grant、exchange、session、invalidation、revoke 的正反案例 |
| projection fixtures | snapshot、scope/epoch/fence mismatch、隐私拒绝案例 |
| mutation fixtures | begin inspection、cancel、mark inspected、resolve 的 request/success/error |
| security/error fixtures | expired/consumed capability、idempotent replay/conflict、wrong target/version/generation、非法 resolution |
| machine schema | 所有 envelope、safe integer、长度限制、enum 与 stable error code 定义 |

agent-chat 合入前还必须：

1. 接受正式 requirement/ADR 和 threat model；
2. 实现上述认证 bridge 证据、projection/version、命令事务闭环与稳定错误码；
3. capability 存储/散列、key rotation、revocation、canonical digest 和 retention 通过安全评审；
4. 测试证明无路径/secret 泄露、无第二审批入口，并提供完整 thread reference。

Robrix2 接线前必须：

1. 按 agent-chat canonical manifest 同步或生成模型并验证 digest；
2. 通过既有 Matrix async worker 通道和独立 scoped data-plane worker 接线，不从 UI 线程
   spawn 原始 tokio task；
3. 先把 wire DTO 转换成经全部不变量验证的 trusted model；未知 action 不进入 UI；
4. schema/scope/epoch/fence/gap/auth/privacy 任一失败时 fail closed；
5. feature on/off、i18n、导航、端到端和安全回归全部通过。

## 9. 真实开发时间

当前 gate task（状态页、旧凭据清理、proposal 与测试）的预算仍是约 **3 个工程日**。它不等于
未来完整 operational client 的工期。

未来完整合同包含新的设备信任、PoP、服务端身份、撤销、projection/CAS 与 inspection 闭环，
建议先安排 **2–3 个工程日的 Matrix device/PoP 技术 spike**，再按风险预算：

- agent-chat requirement/ADR、threat model 与安全评审：4–6 工程日；
- agent-chat bootstrap/session、projection、命令闭环与 canonical tests：9–14 工程日；
- Robrix2 四视图、worker 接线、导航、状态与测试：5–8 工程日；
- 跨仓库集成、安全回归与验收：4–7 工程日。

总量约 **22–35 个工程日**。两端并行且首轮冻结时，真实日历周期约 **4–6 周**；单人串行或
合同反复时应更长。合同交付周期必须从“Accepted ADR/REQ + threat model + canonical manifest
冻结”开始计算，不能从当前 Dashboard 原型开始计算。
