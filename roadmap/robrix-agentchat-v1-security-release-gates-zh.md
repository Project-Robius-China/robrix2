# Robrix2 × agent-chat v1 安全发布门禁

> 状态：规划中，默认未通过
> 适用版本：Robrix2 × agent-chat 首个完整版本
> 优先级：P0，任一必测条件失败均阻断发布

## 1. 目标

首个完整版本必须确保：公开项目房间可以安全地编排本地 coding agent，但房间成员不能借此读取其他开发者的审批内容、替其他开发者审批，或让 agent 越过绑定项目访问本机其他资源。

本文只定义发布门禁与可验证结果，不表示这些能力已经实现。Demo 流程跑通不能替代本文的安全验收。

## 2. 已确定的安全边界

### 2.1 职责边界

- Robrix2 只负责展示公开状态、私有审批 UI 和用户操作结果，不能作为授权来源。
- agent-chat 是 owner、项目绑定、审批请求、审批状态和沙箱策略的权威来源。
- Palpo / Matrix 提供经过认证的完整 Matrix User ID、房间成员关系、私有房间和加密传输能力。
- UI 隐藏不是授权。所有 approve / reject 操作都必须在 agent-chat 服务端再次校验。

### 2.2 Agent owner 规则

owner 是房间级关系：

```text
(project_room_id, agent_mxid) -> owner_mxid
```

- 当房间成员邀请 agent 时，以该 `m.room.member` 邀请事件的完整 `event.sender` 作为 owner。
- 以邀请事件的 `state_key` 作为被邀请的 agent MXID，不能从显示名或 MXID 命名约定推断。
- 必须在 `membership=invite` 时持久化邀请者；agent 加入后的 join 事件不能用于反推 owner。
- v1 采用“首次有效邀请者生效”。踢出并重新邀请不能静默抢走 owner。
- owner 转移必须走单独、显式、可审计的流程；不在普通房间消息中隐式完成。
- agent 自行加入公开房间、缺少邀请事件、邀请者不是有效人类用户，或 owner 数据不完整时，审批能力必须 fail closed。

### 2.3 审批可见性与授权

- 详细审批请求只能发送到 owner 的专属私有审批房间；线上 Palpo profile 的详细审批房间必须启用 Matrix E2EE。
- 私有审批房间成员必须收敛到审批 bot / bridge 与对应 owner；检测到额外成员时暂停发送详情并告警。
- 公共项目房间只能显示脱敏状态，例如“agent A 正在等待所属开发者审批”。
- 公共状态不得包含命令、工具参数、文件内容、diff、绝对本机路径、凭据、审批 token 或私有审批房间 ID。
- agent-chat 必须根据审批事件的完整 `event.sender` MXID 校验身份，不接受显示名、本地备注或房间昵称。
- 审批请求必须绑定 `agent + project + request_id`，同时保存 `owner_mxid`、创建时间、过期时间和最终决策。
- request 只能决策一次。重复、过期、跨 agent、跨 project 或被篡改的审批请求必须拒绝。
- owner/approver 为空时必须拒绝，不得退化为“任意房间成员、operator 或 admin 均可审批”。
- v1 不提供 Matrix 房间内的隐式 admin 兜底审批。紧急 break-glass 如后续需要，必须作为独立、显式、强审计流程设计。

### 2.4 禁止通用终端控制绕过审批

- 公共项目房间必须禁用 `!ctl key`、`!ctl send`、`!ctl status` 及等价的 pane 读取、按键或文本注入能力。
- 不能通过通用 tmux 控制向 agent 输入 `Enter`、`y`、`approve` 等内容绕过结构化审批。
- 正常审批只能经过审批 broker；broker 在验证 owner、请求绑定、有效期和一次性状态后，才把结构化决策返回对应 runtime。

## 3. agent-chat 默认沙箱策略

所有 coding agent 启动时必须默认进入沙箱，包括：首次启动、恢复会话、primary agent、supervisor agent、本机 launcher 和 remote launcher。

### 3.1 默认权限

- 默认仅允许访问 agent home、运行所需的最小状态目录和显式绑定的 managed project。
- managed project 路径必须 canonicalize；不能通过符号链接、`..`、mount 或子进程逃逸到允许根目录之外。
- 允许根目录之外的读、写、删除和执行测试必须失败。
- 出站网络默认关闭；确有需要时只能由项目策略显式放行目标和用途。
- 子进程必须继承相同或更严格的文件系统、网络和凭据边界。
- agent 只能获得自身运行所需的最小凭据，不能继承 Matrix bridge secret、全局 backend token 或其他 agent token。

### 3.2 Runtime 基线

- Codex 不得默认使用 `--yolo` 或 `--dangerously-bypass-approvals-and-sandbox`；v1 基线为 `workspace-write + on-request`，审批交给私有审批 broker。
- Claude v1 基线为 `--permission-mode auto`，不得默认使用 `--dangerously-skip-permissions`。Claude 自身 permission mode 不能替代 OS / 容器级路径隔离。
- supervisor 和 resume 路径必须使用与 primary 首次启动相同的沙箱与审批策略。
- `extraArgs`、runtime profile、环境变量和已保存 session 不得静默重新开启全权限。
- 如保留 unsafe escape hatch，必须默认关闭、显式命名、记录审计事件，并且不能用于公开项目房间。

## 4. v1 P0 测试条件

以下测试必须同时覆盖自动测试和线上 Palpo 联调。`S-*` 为安全测试编号，发布报告必须附对应日志或事件证据。

### S-01：邀请者成为房间级 owner

```gherkin
Given 开发者 Alice 在项目房间 P 中邀请 agent A
When agent-chat 收到 membership=invite 事件
Then 持久化 (P, A) -> Alice 的完整 MXID
And 保存 invite event ID 与时间
And agent A 加入房间后该映射不被 join 事件覆盖
```

反例：Bob 踢出并重新邀请 A，不能静默把 owner 改成 Bob。

### S-02：详细审批只对 owner 可见

```gherkin
Given Alice 是项目 P 中 agent A 的 owner
And Bob 是同一公共项目房间的其他开发者
When agent A 请求高风险操作审批
Then 详细请求只发送到 Alice 的专属私有加密审批房间
And Bob 在公共房间、自己的私聊和同步事件中均看不到审批详情
```

### S-03：公共项目房间只显示脱敏状态

```gherkin
When agent A 创建审批请求
Then 公共房间最多显示 agent、等待状态和通用说明
And 不包含命令、参数、diff、文件内容、绝对路径、凭据、token 或私有房间 ID
```

### S-04：服务端只接受 owner 的完整 MXID

```gherkin
Given Bob 把显示名改成与 Alice 相同
When Bob 对 agent A 的 request_id 执行 approve 或 reject
Then agent-chat 根据 event.sender 的完整 MXID 拒绝请求
And 不改变审批状态
And 写入拒绝审计事件
```

### S-05：请求绑定、过期和防重放

对同一个审批 request 分别验证：

- 正确 owner 对正确 `agent + project + request_id` 首次决策成功。
- 同一 request 第二次决策失败。
- request 过期后决策失败。
- 把 request_id 用于另一个 agent 或 project 时失败。
- 修改 payload、owner 或 project 后失败。
- approve 与 reject 都会终结 request，不能互相覆盖。

### S-06：公共房间不能通过 `!ctl` 绕过

```gherkin
Given agent A 正在等待审批
When 任意公共房间成员或全局 Matrix admin 发送 !ctl key Enter、!ctl send approve 或 !ctl status
Then agent-chat 拒绝执行 pane 读取、按键和文本注入
And agent A 仍保持等待审批
And 公共房间看不到终端内容
```

### S-07：approver 为空时 fail closed

owner 缺失、owner 已离开、owner 绑定损坏、审批私房不可用或 approver 列表为空时：

- 不创建可被其他人接管的审批。
- 不回退到 operator/admin/任意房间成员。
- agent 保持沙箱内等待或安全失败。
- 公共房间只显示脱敏错误；详细原因进入安全日志。

### S-08：所有启动路径默认启用沙箱

分别从以下入口启动 agent，并验证启动参数和实际隔离结果：

- Claude primary 首次启动与 resume。
- Codex primary 首次启动与 resume。
- Claude/Codex supervisor。
- 本机 `agent-up` / `up-v1`。
- remote launcher。

每个入口都必须证明未使用危险默认参数，并且 sandbox policy 一致。

### S-09：只能访问绑定项目

在沙箱测试机创建：绑定项目、相邻未绑定项目、用户凭据目录和符号链接逃逸路径。让 agent 尝试读取、写入、删除和执行：

- 绑定项目内允许的操作成功。
- 相邻项目和凭据目录的所有操作失败。
- 指向允许根目录外的符号链接操作失败。
- agent 创建的子进程执行同样操作仍失败。
- 未经项目策略允许的外网访问失败。

### S-10：启动参数不能静默关闭沙箱

通过 `extraArgs`、runtime profile、环境变量和恢复旧 session 分别注入危险参数：

- 默认启动必须拒绝并给出明确错误。
- 不能只打印 warning 后继续以 full access 启动。
- 失败事件包含 agent、project、触发来源和被拒绝参数类别，但不得记录密钥。

### S-11：审批 broker 不扩大 agent 权限

批准单次操作时：

- 决策只返回原始 request 对应的 runtime/tool call。
- 不把整个 session 永久切换到 full access。
- 不增加新的文件系统根目录、网络目标或凭据。
- agent 不能伪造 request_id 或自行调用 broker 形成自我批准。

### S-12：Robrix2 不是授权旁路

- 修改 Robrix2 本地 AppState、owner 展示或按钮状态不能改变 agent-chat 的授权结果。
- 非 owner 即使构造 Matrix action event，也会被 agent-chat 拒绝。
- Robrix2 重装、换设备或离线重连后，授权结果仍由 agent-chat 的权威记录决定。

## 5. 发布证据

每次候选版本必须保存：

- S-01 至 S-12 的测试结果和版本号。
- robrix2、agent-chat、Palpo 的 commit SHA。
- 测试所用 agent runtime 与 CLI 版本。
- sandbox 启动策略的脱敏快照。
- Matrix owner 邀请事件、私有审批事件和拒绝事件的脱敏记录。
- request 过期、重放、跨 project、错误 MXID 和 `!ctl` 绕过测试日志。
- 线上 Palpo 端到端测试使用的临时项目与清理记录。

测试证据不得包含 access token、LLM key、bridge secret、终端敏感输出或审批详情正文。

## 6. 发布退出条件

- [ ] S-01 至 S-12 全部通过。
- [ ] agent-chat 默认启动不再使用危险跳过权限参数。
- [ ] 公开项目房间不存在 pane 读取或输入注入审批旁路。
- [ ] 无 owner、无 approver、私有房间异常和加密异常均 fail closed。
- [ ] 至少完成一次线上 Palpo 的双开发者、双 agent 联调：交叉审批全部失败，owner 审批成功。
- [ ] Robrix2 只展示服务端授权结果，没有本地授权分支。
- [ ] 安全测试证据经过人工复核。

以上条件全部满足后，Robrix2 × agent-chat 才能标记为首个完整版本候选。
