spec: task
name: "Agent-to-App L2a: Booking Capability (chat-driven Airbnb-style flow)"
inherits: project
tags: [bot, agent-to-app, mini-app, L1-static, L2a-actions, booking, splash, octos]
depends: [task-agent-to-app-system, task-agent-to-app-composite-response, task-agent-to-app-producer-routing, task-agent-to-app-template-runtime, task-tg-bot-action-buttons]
estimate: 4d
---

## Intent

实现 agent2app 系统下的**第一个非信息型 capability**：一个聊天驱动的
Airbnb 式订房流程。目的是验证以下两件事，而不是实现 native booking
体验：

1. **现有 L1 + L2a 合同能否承载多步交互场景**——所有"思考与状态机"
   留在 OctOS 一侧，Robrix 端只是渲染器 + 按钮回路。
2. **同一 type 注册项支持多模板（per-step template selection）**是否
   工作良好——为后续 L2b 卡内控件 / L3 host state 的 spec 工作产出
   真实使用 evidence。

UX 形态：用户用自然语言或按钮表达搜索/筛选/确认意图；OctOS 维护一个
`booking_session`，每一步发一条新 Matrix event，事件里 `org.octos.app`
携带当前步骤的 `initial_state`，`org.octos.actions` 携带本步可走的
按钮。任何"非按钮覆盖"的输入（如自定义日期、留言）都退化到 Matrix
正文 + NLU 解析。**支付一律跳外链**（Stripe Checkout via `open_url`），
不在卡内做。

本 spec 严格遵循 master spec 的所有不变量：编译期闭合 type 白名单、
`(room_id, event_id)` 主键、`m.replace` 免疫、scroll-out 即 teardown、
不跨 restart 持久化。**多步状态机的所有持久化都在 OctOS 侧**，Robrix
端不需要任何"booking session"概念。

可行性背景与决策来源见
[docs/roadmap/2026-04-28-agent-to-app-airbnb-feasibility.md](../docs/roadmap/2026-04-28-agent-to-app-airbnb-feasibility.md)
（Option A）。

## Decisions

### Booking type JSON schema

- **`type` 注册表 key**：`"booking"`；初版 `version = 1`。
- **必填字段**：
  - `step: enum{"search","results","detail","confirm","booked"}` —
    当前步骤。**不同 step 走不同 Splash 模板**；其余字段是否必填随
    step 变化（见下"per-step required fields"）。
  - `session_id: string` — OctOS 侧 booking_session 标识，**仅供观测/
    日志**；Robrix 端不得以此为存储 key（master spec §宿主身份）。
    长度 1..=64，超长 fail closed。
  - `language: enum{"en","zh-CN"}` — payload 级语言覆盖。规则与
    `weather` 一致（详见 task-agent-to-app-l1-weather-card.spec.md
    §Schema 扩展 addendum）。
- **per-step required fields**：
  - `search`：可选 `query: string`（用户输入回显）。无其它必填。
  - `results`：必填 `criteria` (object) + `listings` (array, 1..=8 项)。
  - `detail`：必填 `listing` (object), 可选 `priced_quote` (object)。
  - `confirm`：必填 `listing` + `priced_quote` + `checkout_url: string`。
  - `booked`：必填 `confirmation`（object: `code`/`listing`/`dates`/`total`）。
- **共享子结构**（出现在多 step）：
  - `criteria: { city: string, check_in: date, check_out: date,
    guests: integer, max_price_usd?: number, room_type?: string }`。
    日期 RFC 3339 date（YYYY-MM-DD），缺日期 fail closed。
  - `listing: { listing_id: string, title: string, city: string,
    neighborhood: string, room_type: string, accommodates: integer,
    bedrooms: integer, beds: integer, bathrooms: number, rating: number,
    review_count: integer, price: { amount: number, currency: string,
    amount_usd: number, per: "night" }, host: { name: string,
    is_superhost: bool }, photos: [{url: string, caption?: string}] (1..=10),
    amenities?: [string] }`。
  - `priced_quote: { nights: integer, nightly: number, subtotal: number,
    cleaning_fee: number, service_fee: number, taxes: number,
    total: number, currency: string }`。所有金额取两位小数；币种与
    `listing.price.currency` 一致。
  - `confirmation: { code: string, listing_id: string, check_in: date,
    check_out: date, guests: integer, total: number, currency: string }`。
- **未来扩展**：新增 step 必须 bump `version` 或在 `step` 枚举末尾追加；
  新增字段必须默认值兼容，否则同上。
- **不在 v1 范围**：取消/退款流程、多人共付、订单列表、消息收发与房东
  对话——这些归未来 sub-spec。

### Splash 模板矩阵

- 模板目录：`src/home/app_registry/templates/booking/`
- 文件：
  - `search.splash` — 首屏，渲染欢迎语 + 用户已输入的 criteria 摘要 +
    建议按钮（"使用我的当前位置"等）。
  - `results.splash` — 列表页，最多 8 张 `RoundedView` 行，每行图 +
    标题 + 价格 + 评分 + "查看详情"按钮目标 `action_id`。
  - `detail.splash` — 详情页，图集（最多 5 张图竖排）+ 房型 / 容量 /
    设施 / 评分 / 价格 / "立即预订"按钮。
  - `confirm.splash` — 订单确认，账单拆分（priced_quote 各项）+
    "确认并支付"按钮（带 `checkout_url` 通过 `${open_url}` 跳外链）。
  - `booked.splash` — 完成页，订单号 / 入住日期 / 总价 / "查看订单"
    按钮。
- **widget 调色板限制**：所有模板**只能**用 `widget_manifest.rs` 当前
  允许的 widget（`View` / `Label` / `Icon` / `Image` / `Button` /
  `RoundedView`）。本 spec **不**新增 widget。
- **local function 限制**：只能用 `local_functions.rs` 当前允许的函数
  （`open_url` / `format_date` / `format_number` / `required` /
  `regex_match`）。本 spec **不**新增本地函数。
- **图片数量上限**：`detail.splash` 最多渲染 5 张图（`photos[0..5]`），
  `results.splash` 每行只渲染 `photos[0]`。**渲染层显式截断**，不
  依赖 producer 端控制数量。
- **i18n**：所有文案（"立即预订"、"查看详情"、"清洁费"等）通过
  `app_language` 传入 render 函数，模板内不得硬编码语种。

### 交互按钮（L2a，复用 Phase 4c）

按钮**继续**通过同事件的 `org.octos.actions` 字段定义；**不**在
`org.octos.app` 内嵌按钮列表（master spec §协议 envelope）。

- 各 step 的标准按钮集（OctOS 必须在事件里同时下发）：
  - `search`：`{id: "use_current_city", label: "Use current city"}`,
    `{id: "browse_popular", label: "Browse popular cities"}`。
  - `results`：每条 listing 一个 `{id: "open_<listing_id>",
    label: "View"}`（最多 8 个）；外加 `{id: "refine", label: "Refine"}`。
  - `detail`：`{id: "book_<listing_id>", label: "Book this place"}`,
    `{id: "back_to_results", label: "Back"}`。
  - `confirm`：`{id: "pay", label: "Confirm & pay"}`（点击触发
    Robrix 端 `open_url(<checkout_url>)`），`{id: "modify", label: "Modify"}`。
  - `booked`：`{id: "view_order", label: "View order"}`。
- **action_response 回路**：所有按钮点击经 Phase 4c
  `org.octos.action_response` 路径回发到原 sender；OctOS 端
  dispatcher 接收并基于 `(room_id, session_id, action_id)` 推进
  状态机，输出下一 step 的 event。
- **`pay` 按钮的特殊处理**：
  - Robrix 端在收到 `pay` action 时，**先**调用 `open_url(checkout_url)`
    打开外部浏览器；**再**回发 `org.octos.action_response`。这样即使
    用户没在浏览器完成支付，OctOS 也能记录"已点击 pay"事件用于异步
    webhook 对账。
  - `checkout_url` 必须是 `https://` URL，否则 fail closed（payload
    校验阶段拒绝）。
- **不允许**通过 action 携带表单字段。一切结构化输入要么是按钮
  `action_id`、要么走 Matrix `body` 自由文本 + OctOS NLU。

### Composite response（卡 + body 双面）

复用
[task-agent-to-app-composite-response.spec.md](task-agent-to-app-composite-response.spec.md)：
每条 booking event 的 Matrix `body` 包含同步骤的纯文本摘要（"在
Tokyo Asakusa 找到 5 套，价格 8000-22000 JPY/晚"），从同一次
capability 数据 fetch 派生，**不**为 body 单独发起 LLM 调用。Robrix
渲染卡片下方继续显示 detail bubble（不隐藏）。

### 校验规则

- **必填字段缺失 / 类型错误 / 范围越界**：fail closed，整个
  `org.octos.app` 被忽略，消息退化到 plain body 渲染，warning log
  记录 `type = "booking"` + 缺失字段名 + step。
- **未知 `step` 枚举值**：fail closed，warning log。
- **`listings` / `photos` / `amenities` 数组超长**：渲染层截断
  （listings ≤ 8, photos[detail] ≤ 5, photos[results] = 1, amenities
  ≤ 8 项），warning log 记录丢弃数量。
- **金额币种不一致**（`priced_quote.currency != listing.price.currency`）：
  fail closed，warning log。
- **日期顺序错误**（`check_out <= check_in`）：fail closed，warning
  log。
- **`checkout_url` 非 HTTPS**：fail closed，warning log。
- **未知 `condition` / `room_type` / amenities 字符串**：**不** fail
  closed，原样渲染（这些是展示文本，不影响交互正确性）。
- **Splash-safe 字符串转义**：所有从 `initial_state` 来的字符串字段
  在插入 Splash DSL 前必须经过 `weather.rs` 同款转义（防注入）。
  适用范围：`title`, `description`, `caption`, `host.name`,
  `confirmation.code`, `priced_quote.*` 标签等。

### 测试数据

- Demo / 集成测试用 fixture：`fixtures/airbnb-mock/listings.json`
  （500 条 mock listing，详见
  [`fixtures/airbnb-mock/README.md`](../fixtures/airbnb-mock/README.md)）。
- OctOS booking capability 在开发与回归阶段**直接读取此 fixture**
  作为 listing 池；不连真实 Airbnb 后台。
- Production 切换：未来若接入真实订房 backend，capability 内部数据
  源换成 backend client；本 spec 不涉及 backend 选型。

### Producer 侧契约（OctOS）

- 新 capability `booking` 实现现有 `Capability` trait（参考
  `weather_guidance`）。
- `app_type() -> "booking"`, `app_version() -> 1`。
- `supported_focuses()`：本 capability **不**用 focus 字段（step 已
  完成同等表达力）；返回空切片。
- Resolver 必须能识别"订房 / 找住的 / 看看 Tokyo 的房源 / book"等
  典型短语，slot schema：`{location: string?, check_in: date?,
  check_out: date?, guests: integer?, max_price_usd: number?,
  action: enum{"search","filter","open","book","cancel","status"}?}`。
  缺失的字段由 dispatcher 状态机决定补问还是用默认值。
- Robustness gate：**至少 50 条** EN + ZH 短语回归 fixture，
  ≥ 90% capability 命中率（与 producer-routing spec §Robustness gate
  一致）；反例（"今天天气如何"等）100% 拒绝命中 booking。
- Booking session state store：进程内 `HashMap<session_id,
  BookingSession>` 即可（本 spec v1 不要求跨重启持久化）。

## Boundaries

### Allowed Changes

- `specs/task-agent-to-app-l2a-booking-capability.spec.md`（本 spec）
- `src/home/app_registry/mod.rs`（registry 加一行）
- `src/home/app_registry/booking.rs`（新建，参考 `weather.rs`）
- `src/home/app_registry/templates/booking/*.splash`（新建）
- `src/home/app_registry/capability_descriptors.rs`（加 `booking`
  descriptor + chrome metadata）
- `resources/i18n/en.json` / `resources/i18n/zh-CN.json`（加按钮 / 标签
  文案）
- `fixtures/airbnb-mock/`（新建数据源）
- OctOS 仓库的 capability 实现 + resolver 训练 fixture

### Forbidden

- 不修改 `widget_manifest.rs`（不新增 widget）
- 不修改 `local_functions.rs`（不新增本地函数）
- 不在 `org.octos.app` 内嵌按钮列表（master spec §协议 envelope）
- 不通过 action 携带表单字段（master spec §协议 envelope；本 spec
  §交互按钮）
- 不在客户端实现 booking session 状态机
- 不在客户端做支付输入（一律 `open_url` 跳外链）
- 不发起第二次独立 LLM 调用生成 body（composite-response 约束）
- 不让 `m.replace` 修改已渲染的 booking event
- 不新增 cargo / npm 依赖
- 不跑 `cargo fmt`

### Out of Scope

- L2 form widgets（DatePicker / NumericInput / 等）—— 归独立 sub-spec
- L3 host state（卡内积累选择）—— 归独立 sub-spec
- Sensitive trust 级 + 卡内支付控件 —— 全新议题
- Image carousel / Map widget —— 归独立 widget sub-spec
- 真实订房后端集成（stripe 实账户、酒店 PMS API）
- 取消 / 退款 / 改期流程
- 房东直聊 / 在房间内消息线程
- 多人共付 / 拆账
- 跨设备 booking session 同步

## Acceptance Criteria

Scenario: Booking event with step=results renders the results template
  Test: test_booking_results_template_renders_listing_rows
  Given a Matrix event with `org.octos.app.type = "booking"`,
    `version = 1`, `initial_state.step = "results"`,
    `initial_state.criteria = {city: "Tokyo", check_in: "2026-05-10",
     check_out: "2026-05-12", guests: 2}`,
    and `initial_state.listings` (3 valid listing objects)
  When Robrix renders the message
  Then the booking type's render is invoked with the results template
  And each of the 3 listings renders one row with photo + title + price + rating
  And the rendered Splash code uses only the existing widget palette
  And no Splash widget outside `widget_manifest.rs` is referenced

Scenario: Booking event with step=detail truncates photos to 5
  Test: test_booking_detail_truncates_photos_to_5
  Given a booking event at step "detail" with a listing whose `photos` array has 9 entries
  When Robrix renders the card
  Then the detail card displays exactly 5 photos
  And a warning log is emitted noting 4 photos were truncated

Scenario: Booking and actions coexist independently
  Test: test_booking_event_with_actions_coexists
  Given a booking event at step "results" with valid `org.octos.app`
    and `org.octos.actions = [{id: "open_19632763"}, {id: "refine"}]`
  When Robrix renders the message
  Then the booking card is rendered via the type registry
  And the action button row is rendered via the existing Phase 4c path
  And `org.octos.app` does NOT contain an `actions` key

Scenario: Pay action opens checkout URL before sending action_response
  Test: test_booking_pay_action_opens_url_before_response
  Given a booking event at step "confirm" with `checkout_url =
    "https://checkout.example.com/cs_test_123"`
  And the user clicks the `pay` button
  When Robrix handles the click
  Then `open_url("https://checkout.example.com/cs_test_123")` is invoked
  And THEN `org.octos.action_response` with `action_id = "pay"` is sent
  And the sequence is observable in the test (open_url completes before send)

Scenario: Non-HTTPS checkout URL fails closed
  Test: test_booking_non_https_checkout_url_fails_closed
  Given a booking event at step "confirm" with `checkout_url =
    "http://insecure.example.com"`
  When Robrix attempts to validate `initial_state`
  Then the event falls back to plain body rendering
  And a warning log records the validation failure

Scenario: Date order error fails closed
  Test: test_booking_dates_in_wrong_order_fail_closed
  Given a booking event at step "results" with `criteria.check_in = "2026-05-12"`
    and `criteria.check_out = "2026-05-10"`
  When Robrix validates `initial_state`
  Then the event falls back to plain body rendering
  And a warning log records the date-order failure

Scenario: m.replace edit to booking envelope is ignored
  Test: test_booking_m_replace_edit_ignored
  Given an original booking event at step "results" listing 3 properties
  And a later `m.replace` edit changes `initial_state.listings` to a
    different set of 3 properties
  When Robrix renders the message
  Then the rendered card shows the original listings
  And the m.replace edit is ignored for the booking envelope

Scenario: Currency mismatch between priced_quote and listing fails closed
  Test: test_booking_currency_mismatch_fails_closed
  Given a booking event at step "confirm" with
    `listing.price.currency = "USD"` and `priced_quote.currency = "EUR"`
  When Robrix validates `initial_state`
  Then the event falls back to plain body rendering
  And a warning log records the currency mismatch

Scenario: Composite-response — booking card and body coexist
  Test: test_booking_card_and_body_coexist
  Level: integration
  Targets: composite-response invariant, booking type registry, body bubble path
  Given a booking event at step "results" with both `org.octos.app` and
    a Matrix `body` summarizing the same listings
  When Robrix renders the timeline item
  Then the booking card is visible above
  And the body detail bubble is visible below the card
  And the body is not hidden as accessibility-only fallback

Scenario: Resolver picks the booking capability for a typical EN booking phrase
  Test: should_route_book_a_room_in_tokyo_to_booking_capability
  Given the OctOS resolver receives "book a room in Tokyo for two next weekend"
  When dispatcher classifies the user turn
  Then it resolves to capability `booking` with confidence ≥ 0.6
  And it extracts slots `{location: "Tokyo", guests: 2}` and a parseable
    `check_in` / `check_out` pair

Scenario: Resolver does NOT route weather queries to booking
  Test: should_not_route_weather_queries_to_booking_capability
  Given the OctOS resolver receives "what's the weather in Tokyo today"
  When dispatcher classifies the user turn
  Then it does NOT resolve to capability `booking`

Scenario: Booking capability sources its listing pool from the airbnb-mock fixture
  Test: should_load_listings_from_airbnb_mock_fixture
  Given the OctOS booking capability is configured with the in-tree fixture
  When the capability handles a search request
  Then candidate listings are read from `fixtures/airbnb-mock/listings.json`
  And no external HTTP call to airbnb.com is made
  And no real-money payment provider is contacted in test mode

Scenario: Schema rejects unknown step value
  Test: test_booking_unknown_step_fails_closed
  Given a booking event with `initial_state.step = "checkout_v2"`
  When Robrix validates `initial_state`
  Then the event falls back to plain body rendering
  And a warning log records the unknown step value
