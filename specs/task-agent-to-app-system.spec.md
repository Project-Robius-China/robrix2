spec: task
name: "Agent-to-App — Mini-Apps System Contract"
inherits: project
tags: [bot, agent-to-app, mini-app, protocol, lifecycle, octos]
depends: [task-tg-bot-action-buttons]
estimate: 5d
---

## Intent

定义 Robrix 的 **agent-to-app mini-app 系统合同**。这是一个 master spec，
不直接落地任何 app；它规定了所有下游子任务（L1 天气、L2a 新闻、L2b 卡内控件、
L3 番茄钟 / host runtime）必须共同遵守的**协议 envelope、宿主身份、消息
不可变性、分层契约、以及生命周期集成边界**。

本 spec 要解决的是"系统级不变量"——任何一个具体 mini-app 实现都不许重新
讨论这些决策，它们一旦写入这份合同，就是下游所有工作的硬边界。

当前背景：
- Phase 3 Splash card 原型已经证明 `org.octos.splash_card` 路径可以把
  raw Splash DSL 字符串注入到 timeline 渲染成原生 GPU 卡片。
- Phase 4c `org.octos.actions` / `org.octos.action_response` 实装并在
  生产中使用（`src/home/room_screen.rs:1969` 渲染, `:6877` 捕获 click, 
  `src/sliding_sync.rs:3006` 回发）。
- Phase 5 approval request 引入了 m.replace 免疫原则，防止 edit 改写安全
  关键字段。
- Makepad 上游有 `test_widget_ref_helper_tracks_dynamic_nested_child_like_child_by_path`
  （`widgets/src/widget_tree.rs:3740`）专门测试动态子节点的 path 查找。

本 spec 在这些既成事实上定义 mini-app 系统，**复用**现有协议而不重造。

## Decisions

### 协议 envelope

- **分层、不吸收**：mini-app 系统引入一个新字段 `org.octos.app`，承载 app
  数据 + 生命周期元数据；交互按钮继续使用**已有**的 `org.octos.actions`
  / `org.octos.action_response`，**不**被吸收进 `org.octos.app`。一条
  Matrix event 可以同时携带两个字段。`org.octos.app` 负责 app，
  `org.octos.actions` 负责按钮，两者独立解析、独立处理。
- **`org.octos.app` 必填字段**：
  - `type: string` — 客户端 app 类型注册表的查找 key。Robrix 的注册表是
    **白名单**：未在注册表里的 type 被忽略，不执行 raw Splash eval。
  - `version: integer` — 该 type 的 schema 版本号。registry 里的每个
    type 可以声明它支持的 version 范围。
  - `initial_state: object` — agent 发过来的初始状态，客户端可在本地修改
    但不回写 Matrix。
- **`org.octos.app` 可选字段**：
  - `app_semantic_id: string` — app 自己用的语义 ID（例如区分同房间内的
    两个独立 pomodoro 实例）。**这不是客户端的主键**，客户端不得依赖它
    做身份存储（见下一条）。
  - `client_tick: boolean` — 是否需要客户端驱动连续 tick（L3 场景）。
    默认 `false`。
- **`org.octos.app` 不包含 `actions`**：如果 app 需要交互按钮，必须在同一
  事件的 `org.octos.actions` 字段里定义。禁止在 `org.octos.app` 内嵌按钮
  列表。
- **不复用 `org.octos.splash_card`**：新的 app 渲染路径走
  `org.octos.app` + type registry；`splash_card` 原始字符串路径保留作为
  开发期后门，生产白名单外不得使用。

### 宿主身份

- **主键 `(room_id, event_id)`**：客户端对每个 mini-app 实例的存储主键
  必须是 `(room_id, event_id)`。这两个字段来自 Matrix 事件本身，由
  homeserver 签名、不可变、不可伪造。
- **`app_semantic_id` 不是主键**：`org.octos.app.app_semantic_id` 是 agent
  发过来的自由字符串。客户端**不得**把它作为存储 key 使用，否则两个不同
  event 的同 `app_semantic_id` 会撞车，并且恶意 agent 可伪造。
- **Robrix app host storage**：host state 的存储位置是 `RoomScreen` /
  `TimelineUiState`（或等价的 room-level state 容器），按
  `(room_id, event_id)` 索引。Message widget 本身**不**拥有 host state。

### 消息不可变性

- **`org.octos.app` 和 `org.octos.actions` 对 `m.replace` 免疫**：Robrix
  渲染一条携带 mini-app 元数据的消息时，必须只读取**原始事件的 content**，
  不得采纳任何 `m.replace` / `m.new_content` 对这两个字段的修改。
- **更新机制是"发新消息"**：bot 要更新一个运行中的 app，必须发一条新的
  Matrix 事件，旧 event 的 host 实例在新 event 渲染时被**强制 teardown**
  （见 §生命周期）。
- **一致性**：这条规则与 Phase 5 `org.octos.approval_request` 的
  immutability 以及 Phase 4c `org.octos.actions` 的 client-side
  immutability 完全对齐。三者规则相同：客户端只信任原始事件。

### 分层契约

- **L1 Static**：无状态卡片（例：天气快照）。纯函数 `type + initial_state →
  splash_code`。不需要 host，不需要 tick，不需要按钮桥接。可独立于 L2 / L3
  实施。
- **L2a External action row**：卡片外部的按钮行（例：天气 refresh、新闻
  next/open）。按钮通过同一事件的 `org.octos.actions` 定义，复用 Phase 4c
  的渲染 + 点击捕获 + `org.octos.action_response` 回发路径。**不依赖
  Splash 内部按钮桥接**，可立即实施。
- **L2b In-card control**：卡片内部的按钮（例：卡内点击图标放大）。按钮
  由 Splash DSL 声明在 card body 里，点击后 `ButtonAction::Clicked` 必须
  能冒出到外层 `RoomScreen` 捕获。实装前需要一次 **micro-PoC** 验证
  `splash_ref.button(cx, ids!(<dynamic_child>))` path resolve，不是独立
  spike 任务。
- **L3 Stateful host**：需要 client-driven tick + 本地持久状态的 app
  （例：pomodoro 倒计时）。需要完整的 host runtime（trait + registry +
  tick 调度 + §生命周期集成）。
- **共享注册表**：所有四层都通过同一个 `type` 注册表路由——registry 是
  一个 `HashMap<&'static str, Box<dyn AppFactory>>`，每个 entry 提供
  `init`、`render`、可选 `on_tick` / `on_action`、必选 `teardown`。
- **L1/L2a 不必实现 trait 的所有方法**：注册表允许 entry 只提供
  `init + render` 这两项，其余回 `None` / no-op。
- **渐进实施顺序**：master spec 落地后，L1 与 L2a **同时可开始**；L2b
  在 micro-PoC 通过后开始；L3 在 §生命周期 spec 落地后开始。

### 生命周期集成（L3 关键前置）

- **存储位置**：app host state 存活在 `RoomScreen` / `TimelineUiState`，
  不能存在 `Message` widget 的 `#[rust]` 字段上——因为 `PortalList` 会
  recycle 同一个 `Message` widget 给不同事件。
- **Message widget 的角色**：只拿到一个**非拥有**的 host handle（通过
  `(room_id, event_id)` 查表），绘制时使用，不存储。
- **Scroll-out eviction**：当 timeline window 把某条事件挤出内存时，对应
  host 必须调用 `teardown()` 并从 storage 中移除。
- **Scroll-back re-init**：事件重新进入 window 时，host 重新用原事件的
  `initial_state` 初始化（**v1 不恢复 pre-eviction 的本地变化**）。
- **Room switch teardown**：切换房间时，离开房间的所有 host 必须 teardown。
- **客户端 tick 调度**：`client_tick = true` 的 app 在 host 存活期间每秒
  被 `on_tick(now)` 调用一次；`on_tick` 返回 `Some(splash_code)` 时触发
  对应 Splash widget 的 `set_text` 重绘。tick 调度由 `RoomScreen::handle_event`
  的 `NextFrame` 路径驱动，不开额外 tokio 任务。
- **v1 简化**：不跨 restart 持久化、不跨 eviction 恢复本地状态、不跨设备
  同步。所有简化可在有真实用户需求后迭代。

### 安全与校验

- **类型白名单**：`type` 不在注册表中的消息**不触发** raw Splash eval，
  也不渲染成 app 卡片——退化成 body 文本渲染。这避免 agent 注入任意 widget 树。
- **`initial_state` 输入校验**：每个 type 的 `init` 负责校验自己的
  `initial_state`（例如 `duration_seconds` 合理范围、`started_at` 是合法
  RFC 3339）。非法输入退化成 body 文本渲染 + warning log，不 panic。
- **label 转义**：如果 app 的 render 函数把 `initial_state` 里的字符串
  插进 Splash DSL 字符串，必须先做 Splash-safe 转义，防止注入攻击。
- **raw splash_card 后门**：`org.octos.splash_card` 原始字符串路径保留
  作为开发工具，但不得在生产发布版本中默认启用。

## Boundaries

### Allowed Changes

- specs/task-agent-to-app-system.spec.md
- roadmap/2026-04-13-agent-to-app-mini-apps.md

### Forbidden

- 不要在本 spec 里实现任何 app（本 spec 只是合同）。天气、新闻、pomodoro
  都走独立子 spec。
- 不要修改 `org.octos.actions` / `org.octos.action_response` 协议——复用
  不重造。
- 不要把 `actions` 或按钮列表塞进 `org.octos.app`。
- 不要把 `app_semantic_id` 当宿主存储主键。
- 不要让 `m.replace` 修改已渲染的 `org.octos.app` 或 `org.octos.actions`
  字段。
- 不要在 `Message` widget 的 `#[rust]` 字段上存 app host state。
- 不要为 host 新开 tokio 任务调度 tick（使用 `NextFrame`）。
- 不要让未注册的 type 触发任意 Splash 代码执行。
- 不要新增 cargo 依赖。

## Out of Scope

- 具体 app 实现（天气 / 新闻 / pomodoro 各自子 spec）
- 跨 restart 的 host state 持久化
- 跨设备的 host state 同步
- `m.replace` 对 app envelope 的更新语义（明确禁止）
- 动态 type 注册（所有 type 编译进 Robrix）
- Timeline 外的 app 容器（独立 tab / 通知弹窗）
- L3 host 的并发模型细节（留给 host 子 spec）
- 具体的 card_registry / miniapp_host 模块名称和代码组织（留给子 spec）

## Completion Criteria

Scenario: Message with org.octos.app envelope uses the type registry, not raw splash
  Test: test_app_envelope_routes_through_type_registry
  Given a Matrix event with `org.octos.app.type = "weather"` and `org.octos.app.initial_state` valid
  And the local type registry contains an entry for "weather"
  When Robrix renders the message
  Then the weather type's render function is invoked with `initial_state`
  And the rendered Splash code comes from the registry, not from a raw `org.octos.splash_card` string
  And no raw Splash eval happens for a field other than what the registry produced

Scenario: Unknown type falls back to plain body rendering with a warning
  Test: test_unknown_app_type_falls_back_to_text
  Given a Matrix event with `org.octos.app.type = "weird_custom_app"`
  And the local type registry does NOT contain an entry for "weird_custom_app"
  When Robrix renders the message
  Then the app envelope is ignored
  And the message body is rendered as plain text
  And a warning is logged containing the unrecognized `type`
  And no Splash eval is attempted for that envelope

Scenario: App envelope and actions field coexist in the same event, parsed independently
  Test: test_app_envelope_and_actions_field_coexist
  Level: integration
  Targets: type registry routing, action-button renderer, field independence invariant
  Given a Matrix event that contains both `org.octos.app` (valid weather payload) and `org.octos.actions` (valid action list with a `refresh` button)
  When Robrix renders the message
  Then the weather card is rendered via the type registry
  And the `refresh` action button is rendered via the existing Phase 4c action-button path
  And the `org.octos.app` object does NOT contain an `actions` key
  And removing either of the two fields leaves the other working independently

Scenario: Host storage key is (room_id, event_id), not app_semantic_id
  Test: test_host_storage_keyed_on_matrix_identity
  Given a Matrix event in room `!room1:example.org` with `event_id = "$evt_a"` and `org.octos.app.app_semantic_id = "shared_id"`
  And another Matrix event in the same room with `event_id = "$evt_b"` and the same `app_semantic_id = "shared_id"`
  When Robrix creates host state for both messages
  Then the two host instances are stored under distinct keys `(!room1:..., $evt_a)` and `(!room1:..., $evt_b)`
  And mutating one host does not affect the other
  And `app_semantic_id` is not used as any part of the host storage key

Scenario: Forged app_semantic_id from a malicious agent does not collide with existing hosts
  Test: test_forged_semantic_id_does_not_collide
  Given an existing rendered app instance at `(room_id, event_id)` with `app_semantic_id = "pom_abc"`
  When a new event arrives with a different `event_id` but `app_semantic_id = "pom_abc"`
  Then the new event is stored under its own `(room_id, event_id)` key
  And the original host is not reused, rebound, or mutated by the new event

Scenario: m.replace edit to an app envelope is ignored at render time
  Test: test_m_replace_edit_to_app_envelope_ignored
  Given an original Matrix event with `org.octos.app.type = "weather"` and `initial_state = {"city": "Beijing"}`
  And a later `m.replace` edit targeting the same event whose `m.new_content` sets `initial_state = {"city": "Shenzhen"}`
  When Robrix renders the message
  Then the rendered weather card uses `city = "Beijing"` (from the original event)
  And the `m.replace` edit is ignored for app envelope purposes
  And no host state is mutated based on the edit

Scenario: m.replace edit to an actions list is ignored at render time
  Test: test_m_replace_edit_to_actions_ignored
  Given an original Matrix event with `org.octos.actions = [{"id": "approve", "label": "Approve"}]`
  And a later `m.replace` edit whose `m.new_content` sets `org.octos.actions = [{"id": "auto_approve", "label": "Auto"}]`
  When Robrix renders the message
  Then the rendered action row still shows the original `approve` button
  And the edit is ignored for action-button purposes

Scenario: L1 static card ships without depending on L2b or L3
  Test: test_l1_static_card_independent_of_l2b_and_l3
  Given a Matrix event with `org.octos.app.type = "weather"` and no `org.octos.actions` field
  And no `client_tick` flag
  When Robrix renders the message
  Then the weather card is rendered successfully
  And no button bridge, no tick scheduler, and no host runtime is required
  And the type registry entry for "weather" only needs to implement `init` and `render`

Scenario: L2a external action row reuses Phase 4c click path without a bridge
  Test: test_l2a_external_actions_reuse_phase4c_click_path
  Level: integration
  Targets: Phase 4c action-button click path, org.octos.action_response send path, L2a reuse invariant
  Given a Matrix event with `org.octos.app.type = "news"` and `org.octos.actions = [{"id": "next"}, {"id": "open"}]`
  When the user clicks the "next" button
  Then the click is handled by the existing Phase 4c action-button code path
  And an `org.octos.action_response` with `action_id = "next"` is sent back to the original sender
  And the click handler does NOT require resolving any widget inside the Splash card body

Scenario: L2b in-card control requires micro-PoC evidence before implementation starts
  Test: test_l2b_is_gated_on_splash_button_micro_poc
  Given the master spec is in effect
  And no micro-PoC has yet demonstrated `splash_ref.button(cx, ids!(<dynamic_child>))` resolving after `set_text`
  When a sub-spec for an L2b in-card control is proposed
  Then the sub-spec must reference a passing micro-PoC result
  And implementation work on L2b must not start before the PoC has been recorded

Scenario: Host is owned by RoomScreen and not by the recyclable Message widget
  Test: test_host_state_lives_outside_message_widget
  Given a room with multiple app-carrying events in its timeline
  And the local Portal List recycles `Message` widgets as the user scrolls
  When a `Message` widget instance is reused for a different event
  Then it retrieves the corresponding host via `(room_id, event_id)` lookup
  And it does NOT carry over any host state from the previous event it rendered
  And host storage lives on `RoomScreen` / `TimelineUiState`, not on the `Message` widget

Scenario: Scroll-out evicts the host, scroll-back re-inits from initial_state
  Test: test_scroll_out_evicts_host_scroll_back_reinits
  Given an app with `client_tick = true` that has been running and has mutated local state
  When the hosting event scrolls far enough out of the timeline window to be evicted
  Then the host's `teardown` method is called
  And the host state is removed from storage
  When the user scrolls back and the event re-enters the window
  Then a new host is initialized from the original event's `initial_state`
  And no state from before the eviction is restored
  And this behavior is acceptable under v1 "no pre-eviction state restoration"

Scenario: Leaving a room tears down all hosts owned by that room
  Test: test_room_switch_tears_down_all_hosts
  Given the user is in room `!room1:example.org` with N running app hosts
  When the user navigates to a different room
  Then each of the N hosts has `teardown` called
  And the host storage for `!room1:...` is cleared
  And returning to `!room1:...` re-initializes hosts from their `initial_state`

Scenario: Host tick schedule runs on NextFrame, not a dedicated tokio task
  Test: test_host_tick_driven_by_next_frame
  Given an app with `client_tick = true`
  When the host is running and visible in the timeline
  Then tick dispatch happens during `RoomScreen::handle_event` on `NextFrame`
  And no additional tokio task is spawned for the tick schedule
  And if `on_tick` returns `Some(splash_code)`, the underlying Splash widget receives `set_text`

Scenario: Invalid initial_state from a registered type falls back to text with a warning
  Test: test_invalid_initial_state_falls_back
  Given the "pomodoro" type is registered
  And a Matrix event provides `initial_state = {"duration_seconds": -1}`
  When Robrix renders the message
  Then the pomodoro type's `init` rejects the invalid input
  And the message is rendered as plain text body
  And a warning is logged naming `type = "pomodoro"` and the validation failure
  And no host is created

Scenario: String values in initial_state are escaped before being interpolated into Splash code
  Test: test_initial_state_strings_escaped_in_render
  Given an event with `initial_state.city = "<script>alert(1)</script>"`
  When the weather type's render function builds the Splash code
  Then the displayed card shows the literal text `<script>alert(1)</script>`
  And no Splash widget is created from the injected substring
  And the escape logic is implemented in the render function, not left to the type registry caller

Scenario: Raw org.octos.splash_card path remains available for development but not for production builds
  Test: test_raw_splash_card_path_disabled_in_production
  Level: integration
  Targets: build-mode gating, raw Splash eval backdoor, production safety invariant
  Given a Matrix event with only an `org.octos.splash_card` field and no `org.octos.app`
  And the build is a production release build
  When Robrix renders the message
  Then the raw splash_card path is NOT invoked
  And the message is rendered as plain text body
  And a warning is logged noting that raw splash_card is disabled in production
