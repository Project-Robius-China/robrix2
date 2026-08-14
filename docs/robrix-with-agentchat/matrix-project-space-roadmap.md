# Robrix2 Matrix Project Space Roadmap

> - **状态：** Proposed roadmap，尚不是 Accepted spec，也不代表交付日期承诺
> - **日期：** 2026-08-06
> - **Robrix2 代码基线：** `9f70cd66392ba51e81e0cfcc77770649d2fb5c9a`
> - **产品依据：** [PRD — HAFleet as a digital-labour resource plane](prd-hafleet-pdu-v0.2.md)
> - **范围：** Robrix2 的 Matrix Space 生命周期、Project Space 导航与项目绑定；不包含 HAFleet 的资源调度实现

## 1. 路线图结论

Robrix2 当前的 Matrix Space 能力是“读侧成熟，写侧残缺”：用户可以发现、浏览、搜索、加入和离开 Space，
可以邀请成员，也可以在 Space 内新建房间；但还不能创建 Space、把已有房间加入 Space、从 Space 移除房间，
顶层 Space 邀请也不会进入现有可见邀请列表。

HAgency 的目标不是把 Space 继续当作一个只读房间目录，而是把它作为客户项目平面的稳定入口：

- **Matrix Space = Project**：面向客户的项目容器和导航根。
- **Child room = Team/Process Surface**：PRD、SE、模块 Spec、测试、Release、MO、PDT、销售、客服、HR、CFO 等协作面。
- **Thread 或持久化任务 = Work Item**：具体问题、需求、决策或交付单元。
- **HAFleet = Resource Plane**：提供数字员工、能力、成本、健康和派工状态，不拥有客户的项目流程。

因此，Space 邀请是第一个必须修复的用户阻断点，但不是 Project Space 产品闭环。创建、挂载、移除、
稳定层级和 `ProjectRef` 绑定都应进入同一条路线图。

## 2. 已确认的当前基线

| 能力 | 当前状态 | 代码证据或说明 |
|---|---|---|
| 已加入 Space 的发现和增量同步 | 已实现 | [`space_service_sync.rs`](../../src/space_service_sync.rs) 订阅 SDK 的 joined-space service |
| 嵌套 Space/room 层级浏览 | 已实现 | Space Lobby、桌面侧栏和移动端 Spaces 标签已经接入 |
| Space 内搜索和导航 | 已实现 | Lobby 搜索会保留匹配项的祖先路径 |
| 加入子房间、离开 Space | 已实现 | 离开 Space 可级联离开其已加入的房间 |
| 邀请成员加入当前 Space | 已实现 | [`space_lobby.rs`](../../src/home/space_lobby.rs) 可打开成员邀请入口 |
| 在当前 Space 内新建房间 | 部分实现 | 新房间创建后调用 `attach_room_to_space()` 写入 Space 关系 |
| 顶层 Space 邀请可见 | 缺失，P0 | 普通房间列表过滤 Space；Space service 只订阅已加入 Space |
| 创建 Space | 缺失，P0 | 创建房间请求没有设置 `m.space` room type/creation content |
| 把已有房间加入 Space | 缺失，P0 | 当前关系写入只出现在“新建房间后挂载”路径 |
| 从 Space 移除房间 | 缺失，P0 | 没有撤销 `m.space.child` 关系的请求和 UI |
| 规范子房间顺序 | 有缺陷，P1 | Lobby 丢弃 SDK 顺序，重新按“Space 优先、名称排序”排列 |
| 侧边栏稳定顺序 | 有缺陷，P1 | 部分重建路径来自 `HashMap`，顺序可能漂移 |
| 上次选中 Space 持久化 | 缺失，P1 | `selected_tab` 使用 `#[serde(skip)]`，启动后回到 Home |
| Space 深链接 | 部分实现，P1 | Add/Explore 可解析；时间线内点击 Space room ID 的导航尚未贯通 |
| Space 未读角标 | 缺失，P2 | 当前 Space 未读数量固定为 0 |
| Space 专项 spec/自动化测试 | 缺失，P0 | 只有其他任务附带的 SpaceLobby 场景，没有完整能力合同 |

这张表描述的是上述代码基线，不用于推断团队路线图意图，也不把未来目标登记成当前回归 bug。

## 3. 产品与安全边界

### 3.1 Space 是项目导航与绑定载体，不是权限证明

`m.space.child` 和 `m.space.parent` 表达层级关系，不自动授予以下权限：

- 项目成员身份或房间成员身份；
- room power level；
- HAFleet 派工、预算、成本或审批权限；
- Agent Operations 的 owner-DM 权限；
- E2EE 密钥访问权限。

Robrix2 可以展示关系和提交 Matrix 操作，但最终权限由 homeserver、房间状态和对应服务端的认证主体判定。

### 3.2 Project 身份不能只从名称或层级推导

目标 `ProjectRef` 至少需要：

- 稳定 `project_id`；
- 可选但规范化的 `space_room_id`；
- binding version；
- 带类型的 child-room bindings；
- classification、revocation 和 audit 信息。

不得用 Space 名称、房间名称或当前树位置临时推断规范项目身份。共享房间、嵌套 Space、关系撤销和迁移
都必须有明确语义。

### 3.3 Matrix 写操作必须走既有异步边界

未来的创建 Space、挂载和移除操作必须通过 `submit_async_request(MatrixRequest::*)` 进入 Matrix worker，
由后台执行 SDK/API 调用并把结构化结果送回 UI。UI 不直接启动 Tokio 任务，也不把本地乐观状态当作服务端终态。

## 4. 分阶段路线

阶段编号表示依赖顺序，不表示自然月或合同交付期。每个阶段开始实现前，应拆成独立 task spec，继承
[`specs/project.spec.md`](../../specs/project.spec.md)，并通过 `agent-spec parse` 与
`agent-spec lint --min-score 0.7`。

### Phase 0 — 合同、模型与测试基线（P0）

**目标：** 在增加写能力前，先确定 Project Space 生命周期和安全语义。

交付物：

1. `task-space-invitations.spec.md`：Space 邀请发现、接受、拒绝和实时更新。
2. `task-space-lifecycle.spec.md`：创建 Space、挂载已有房间、移除房间和失败恢复。
3. `task-space-hierarchy-integrity.spec.md`：循环、重复边、规范排序和稳定导航。
4. Project Space/`ProjectRef` ADR/REQ：修订或取代 room-as-project 的旧假设。
5. 明确桌面、移动端、i18n、权限不足、网络失败、重启恢复和无障碍验收矩阵。

退出条件：

- Space 邀请和 Space lifecycle 的状态模型、动作结果及错误语义无歧义。
- 明确 `m.space.child` 与 `m.space.parent` 的写入顺序、幂等策略和部分失败补偿。
- 明确关系变化不会自动改变成员、power level、E2EE 或 HAFleet 权限。
- 核心模型和图算法可以脱离 Makepad Widget 做单元测试。

### Phase 1 — 让 Project Space 可到达（P0）

**目标：** 用户收到邀请或链接后，能够在 Robrix2 中发现并进入 Project Space。

范围：

- 为邀请模型增加明确 room type/`is_space` 分类，不再让 Space 邀请依赖被过滤的普通房间列表。
- 桌面和移动端统一展示 Space 邀请，支持接受、拒绝、处理中和失败状态。
- 支持同步期间实时到达、应用重启后恢复、重复事件去重和邀请撤销。
- 打通时间线中的 `matrix.to`/Matrix URI 到 Space Lobby 的导航。
- 未加入 Space 的预览不得泄漏无权读取的成员或子房间信息。

关键验收：

- 用户无需重启即可看到新 Space 邀请。
- 接受后邀请条目消失且 Space 只出现一次；拒绝或邀请撤销后不残留幽灵条目。
- 无权限、网络失败和服务端拒绝都有可理解、可翻译的错误状态。
- Add/Explore、时间线链接和邀请卡最终进入同一个 Space 导航结果。

### Phase 2 — 完成 Space 生命周期（P0）

**目标：** 客户可以直接在 Robrix2 中建立和维护 Project Space 结构。

范围：

- 创建 `RoomType::Space` 的 Space，配置名称、topic、avatar、可见性和初始权限。
- 将已有房间或子 Space 加入当前 Space。
- 从 Space 中移除房间或子 Space；默认只解除关系，不离开、不删除目标房间。
- 保留“在 Space 中新建房间”，但统一复用同一套关系写入状态机。
- 在 UI 中根据 power level 禁用无权操作，并由服务端做最终授权。
- 对双向关系写入提供明确结果：成功、部分成功、权限不足、关系已存在、目标不可见和重试后终态。

关系一致性要求：

1. 父 Space 的 `m.space.child` 是层级展示的必要写入。
2. 子房间的 `m.space.parent` 仅在有权限且产品合同要求时写入。
3. 第二次写入失败不得把第一次已经成功的写入伪装成“完全失败”。
4. 重试必须幂等；UI 必须重新读取服务端状态，而不是只修改本地树。
5. 移除关系不得隐式退房、删除房间、删除历史消息或撤销成员权限。

关键验收：

- 创建的 Space 能被 Robrix2 和另一标准 Matrix 客户端识别为 Space。
- 已有房间可挂载、重复挂载不产生错误重复项、移除后可重新挂载。
- 对父有权限但对子无权限，以及对子有权限但对父无权限的情况都有确定结果。
- 写入成功后，侧边栏、Lobby 和房间过滤视图最终收敛到同一服务端状态。

### Phase 3 — 层级完整性与日常可用性（P1）

**目标：** Space 树在复杂项目、重启和长期使用下保持稳定。

范围：

- Robrix2 自己的递归遍历增加 visited set、最大深度或等价的循环保护。
- 保留 Matrix SDK 给出的规范 child order；没有 order 时使用确定性 fallback。
- 侧边栏使用稳定、有定义的排序，正确处理 Insert/Set/Remove/Reset 等增量变化。
- 持久化上次选中的 Space，并在 Space 已退出或不可见时安全回退到 Home。
- 为 Space 增加聚合未读和 mention 语义；明确是否递归包含子 Space。
- 清理 stale relation、孤立 parent、重复边和关系撤销后的缓存状态。

关键验收：

- 人工构造 A → B → A、自环和重复边时，不出现无限递归、栈溢出或重复渲染。
- 同一服务端状态跨启动、搜索清空和增量刷新后保持相同顺序。
- selected Space、其 Dock 布局和当前房间恢复一致；目标失效时不会打开错误项目。
- 未读计数与既定聚合规则一致，不因环或共享房间重复计数。

### Phase 4 — Project Space 产品绑定（P1，跨 Robrix2/HAFleet）

**目标：** Space 从通用 Matrix 容器升级为 HAgency 的规范项目入口，同时保持项目平面与资源平面解耦。

范围：

- 建立 versioned `ProjectRef`，绑定 `project_id` 与 `space_room_id`。
- 支持 typed room bindings，例如 `prd`、`architecture`、`module_spec`、`testing`、`release`、
  `marketing`、`pdt`、`sales`、`support`、`hr` 和 `cfo`。
- 明确一个房间被多个 Space 引用、嵌套 Space、房间迁移和撤销的处理方式。
- Robrix2 在 Project Space 中展示 HAFleet 的 Agent、assignment、cost 和 health 投影。
- HAFleet 的请求必须携带稳定 `project_id`；Space/room/thread 只作为受验证的绑定和追踪上下文。
- Project Space 中的阶段、验收和项目决策仍归客户 Matrix 流程所有，HAFleet 不成为项目工作流引擎。

退出条件：

- `ProjectRef`/typed bindings 的 ADR、REQ、schema 和 canonical fixtures 已 Accepted。
- room-as-project 兼容模式有明确迁移和退役条件。
- 解除或变更 Space 关系不会静默改变项目身份、成本归属或授权。
- Robrix2 与 HAFleet 对同一绑定版本、撤销状态和错误语义达成一致。

## 5. 建议交付切片

| 切片 | 包含阶段 | 用户价值 | 是否可独立发布 |
|---|---|---|---|
| **A. Space Access** | Phase 0 + Phase 1 | 能看到邀请，并从邀请或链接进入现有 Project Space | 可以；解决最直接阻断点 |
| **B. Space Management** | Phase 2 + Phase 3 核心项 | 能创建和维护真实项目结构，层级在长期使用中稳定 | 可以；是 Robrix2 Project Space MVP |
| **C. HAgency Project Binding** | Phase 4 | 项目身份、团队房间与 HAFleet 派工/成本投影形成规范关联 | 需跨仓合同后发布 |

切片 A 不能被描述为“Project Space 已完成”；只有切片 B 通过后，Robrix2 才具备客户自主管理 Project Space
的完整基础。切片 C 不应反向阻塞纯 Matrix Space 管理能力，但其身份和授权合同必须在接入 HAFleet 前完成。

## 6. 测试与验收策略

### 自动化测试

- **纯模型测试：** 邀请分类、动作状态、错误映射、排序 fallback、图遍历和循环保护。
- **Matrix 集成测试：** 创建 Space、挂载/移除、双向关系、power level、邀请到达、重启和幂等重试。
- **持久化测试：** selected Space、失效回退、Dock 状态和账户隔离。
- **契约 fixtures：** ProjectRef、typed bindings、binding revocation 和未知版本拒绝。

### 手工验收

- 桌面和移动端分别覆盖邀请、创建、挂载、移除、搜索、深链接和错误恢复。
- 至少使用两个不同权限的 Matrix 账号验证授权边界。
- 至少与一个标准 Matrix 客户端交叉验证所创建 Space 和关系事件。
- 检查中文和英文文案、长 Space 名称、空状态、加载状态和窄屏布局。

## 7. 明确非目标

本路线图不包含：

- 通过 Space 层级自动授予 HAFleet、审批或 Agent Operations 权限；
- 让 HAFleet 接管客户的 PRD → Spec → Testing → Acceptance 流程；
- 根据房间名称自动认定房间类型或项目身份；
- 移除 Space 关系时自动删除、退房或清理工作成果；
- 在没有独立租户与数据隔离合同前实现跨客户 Project Space 资源市场；
- 把 Space 管理与 Agent Operations 的独立客户端认证问题合并成同一协议。

## 8. 维护规则

- 每个阶段必须链接到对应 task spec、实现 PR、自动化测试和用户验收记录。
- “当前缺陷”“产品目标”“治理依赖”使用不同状态，不得互相替代。
- 路线图基线变化时更新代码 SHA、状态表和已完成证据，不根据提交沉默推断产品意图。
- 任何新增 UI 文案必须同步检查 `resources/i18n/en.json` 和 `resources/i18n/zh-CN.json`。
- 未经用户测试，不提交代码、不创建 PR。
