spec: task
name: "Agent-to-App L1: Weather Card (first reference mini-app)"
inherits: project
tags: [bot, agent-to-app, mini-app, L1-static, weather, splash]
depends: [task-agent-to-app-system]
estimate: 2d
---

## Intent

实现 **agent-to-app 系统合同下的第一个真实 mini-app**：天气卡片。这是 L1
层（纯静态、无状态、无交互）的 reference implementation——它的存在既是
为了给终端用户提供一个可用的功能，也是为了**验证 master spec 的合同在真实
渲染路径上是否自洽**：type registry routing、Canvas Splash eval 语法、
immutable envelope、与 `org.octos.actions` 的组合边界、生产环境 raw
splash_card 禁用，全部在一个最小可用 app 上走通一遍。

天气卡片本身是 pure presentational 组件（按 master spec design anchor 2）——
无本地状态、无 tick、无 `on_action`。**卡片外**的 refresh / forecast 日切换
按钮走 `org.octos.actions`（独立于本 spec，归 L2a 子 spec 管）。本 spec
只交付：weather type 的 JSON schema、registry 条目、Splash DSL 生成函数、
对应的解析/渲染路径接线、输入校验与转义、以及 i18n 文案。

**不在本 spec 范围**（按 master spec §Out of Scope）：L2a 新闻阅读器、
L2b 卡内按钮、L3 host runtime、具体注册表模块的最终命名（本 spec 会提议
一个名字，但允许实施者合理改名）。

## Decisions

### Weather type JSON schema

- **`type` 注册表 key**：`"weather"`；**当前正式版本 `2`**，consumer 同时
  兼容 legacy `1` payload（见 §当前实际版本）。下方字段定义是 v1 的
  原始集合，v2 在此基础上扩展——扩展字段分两部分：§Schema 扩展 addendum
  承认的（`language`/`focus`），和 §Out of Scope 中暂未声明的 v2 字段级
  schema
- **`org.octos.app.initial_state` 必填字段**：
  - `location: string` — 地点名称，长度 1..=64 字符（字符数，不是字节数），超长按 §校验
    规则截断 + warning
  - `temp_c: number` — 当前温度摄氏度，合法范围 `-80..=80`，超范围 fail closed
  - `condition: string` — 天气状况枚举：`"sunny" | "cloudy" | "rainy" | "snowy" | "stormy" | "foggy"`；
    未知值 fall back 到 `"sunny"`（见 §校验规则）
- **`org.octos.app.initial_state` 可选字段（v1 原始集合）**：
  - `feels_like_c: number` — 体感温度，范围和校验同 `temp_c`
  - `humidity: integer` — 相对湿度百分比，`0..=100`
  - `wind_kph: number` — 风速，`>= 0`
  - `updated_at: string` — 数据时间戳，必须是 RFC 3339 UTC 字符串；解析失败时字段被忽略但不 fail
    close 整张卡片
  - `forecast: array` — 未来若干天预报，每项是 `{day: string, high_c: number, low_c: number, condition: string}`；
    长度 0..=7；超过 7 条按 §校验规则截断 + warning
- **`org.octos.app.initial_state` 可选字段（v2 新增，producer-routing 与 v2
  guidance card 引入）**（对应
  [`task-agent-to-app-l1-weather-v2-doc-sync`](task-agent-to-app-l1-weather-v2-doc-sync.spec.md) 同步进来的字段）：
  - `high_c: number` — 当日最高温摄氏度；范围和校验同 `temp_c`（超范围
    fail closed）。代码见 `src/home/app_registry/weather.rs:57`
  - `low_c: number` — 当日最低温摄氏度；范围和校验同 `temp_c`（超范围
    fail closed）。**不强制** `low_c <= high_c`——consumer 当前不做交叉
    校验，本 spec 忠实记录为不校验
  - `uv_index_max: number` — 最大 UV 指数；`>= 0`；负值 fail closed
    （`weather.rs:59`）
  - `precipitation_probability_max: number` — 当日最大降水概率百分比；
    `0..=100`；超范围 fail closed（`weather.rs:60-62`）
  - `periods: array` — 日内周期天气数组；最多 3 项，**超出静默截断**（见
    §校验规则）。每项 object 结构：
    - `slot: enum{"morning","noon","night"}` — 必填；未知值 fail closed
      （`weather.rs:483-485`）
    - `temp_c: number` — 必填，范围和校验同根级 `temp_c`；超范围 fail
      closed
    - `condition: string` — optional，枚举同根级 `condition`；未知值
      fall back 到 `"sunny"`，**但不 warning log**（与根级 `condition`
      的"未知值 warning"行为不同，见 §校验规则）
    - `precipitation_probability: number` — optional，`0..=100`；代码
      同时接受整数和浮点（浮点先 `round()` 再范围校验，最终存储为 `u32`，
      `weather.rs:507-515`）；超范围 fail closed
- **未来 schema 演进**：新增字段必须设置默认值向后兼容；删除或改语义的字段必须 bump
  `version`，registry 对不支持的 `version` 直接 fall back 到 plain text + warning。
- **v1 不支持的字段**：天气图标 URL（只用 `condition` 枚举映射文本 emoji）、
  multi-location、小时级预报、空气质量。这些在 v2 再议。

### 当前实际版本（事实声明，不 bump）

- **`WeatherFactory::supported_version()` 返回 `2`**，`supports_version(v)`
  同时接受 `1 | 2`（代码见 `src/home/app_registry/weather.rs:36-42`）。本
  spec 在此**显式承认** v2 是当前正式版本；producer 合同（
  [task-agent-to-app-producer-routing](task-agent-to-app-producer-routing.spec.md)）
  依赖此事实作为 `org.octos.app.version` 的默认发送值。
- v2 引入的所有字段现已在本 spec 内完整承认：
  - `language`、`focus` 在下方 §Schema 扩展 addendum
  - `high_c`、`low_c`、`uv_index_max`、`precipitation_probability_max`、
    `periods`（含子结构）在 §Weather type JSON schema 的 "v2 新增" 段
  - 对应行为规则在 §校验规则
  - 同步来源：[`task-agent-to-app-l1-weather-v2-doc-sync`](task-agent-to-app-l1-weather-v2-doc-sync.spec.md)（doc-sync 任务）

### Schema 扩展（producer-routing 接续 addendum）

以下 optional 字段由 [`task-agent-to-app-producer-routing`](task-agent-to-app-producer-routing.spec.md) 引入并已在
consumer 代码实现，本 spec 在此承认它们为合法的 `initial_state` optional
扩展，**不 bump `version`**：

- `language: enum{"en","zh-CN"}` — payload 级语言覆盖。存在时 render 必须用
  此字段而非全局 `AppLanguage`；当前实现在 `src/home/app_registry/weather.rs:47`
  和 `:292`。Producer 侧规则（什么时候发、何时省略）由 producer-routing
  spec 管。
- `focus: enum{"overview","clothing","umbrella","outdoor"}` — 指导卡焦点。
  consumer 收到未知值时 fall back 到 `"overview"` 并 warning。Producer 侧
  分派规则由 producer-routing spec 管。

**其余 v2 字段位置**：本 addendum 只承认 `language` 与 `focus`；v2 的数据
结构字段（`high_c` / `low_c` / `uv_index_max` / `precipitation_probability_max` /
`periods`）见 §Weather type JSON schema 的 "v2 新增" 段与 §校验规则的
periods 相关条目。

### Registry 条目

- **模块命名（建议）**：新增模块 `src/home/app_registry/mod.rs` 作为 type registry
  入口；`weather` 类型在 `src/home/app_registry/weather.rs` 子模块中实现。
  实施者**可以合理改名**，但必须遵守以下结构约束：
  - registry 是一个 `type -> factory` 的 `HashMap<&'static str, Box<dyn AppFactory>>`
  - factory trait 只要求：
    - `init(initial_state: &JsonValue) -> Result<RenderedApp, ValidationError>`
    - `render(state: &RenderedApp, app_language: AppLanguage) -> String`
  - `render` **必须**接受 `app_language` 参数；i18n 标签通过显式传入的
    `AppLanguage` 解析，**不得**使用任何线程局部或全局语言状态
  - 调用方负责在调用 `render` 前计算 **effective language**：若
    `initial_state.language` 存在且合法，effective = payload language；
    否则 effective = `AppState::app_language`。`render` 看到的就是这个
    effective language，函数本身不再做 fallback 判断（语言来源由 §Schema
    扩展 addendum 钉死，本条与之兼容）
  - L1 类型**只**实现这两个方法——不提供 `on_tick` / `on_action` / `teardown`
- **注册时机**：registry 在 `RoomScreen::init` 或等价的 room-level 初始化
  路径上构造一次，此后只读。不动态注册。
- **`weather` 条目的 `init` 职责**：
  - 校验所有必填字段存在且类型正确
  - 校验数值范围（temp_c、feels_like_c、humidity、wind_kph）
  - 校验 condition 枚举；未知值不拒绝整条消息，**但映射为 `"sunny"` 并记录 warning**
  - 截断 `location` 和 `forecast`
  - 返回内部 `RenderedWeather` 结构（所有 Option 字段已经解析或 None）
- **`weather` 条目的 `render` 职责**：
  - 纯函数，输入 `(&RenderedWeather, AppLanguage)`，输出 Splash DSL 字符串
  - 所有字符串字段在插入 Splash DSL 前必须经过 **Splash-safe 转义**（见 §校验规则）
  - 输出符合 Canvas eval-path 语法约束（见 §Splash 输出约束）
  - i18n 标签按传入的 `AppLanguage` 解析——不得引用任何外部语言状态

### Splash 输出约束（Canvas eval path）

这是本 spec 最容易出错的部分，必须严格遵守，否则 Splash eval 会静默渲染
失败或样式错乱。

- **语法风格**：使用 **Canvas Splash 语法**，不是 `script_mod!` 编译期语法。
  具体差异：
  - **dot-path 内联属性**：`draw_bg.color: #x1a1a2e`，**不要**用嵌套块
    `draw_bg: { color: ... }`
  - **`draw_bg.radius`**，不是 `draw_bg.border_radius`
  - **显式 `Inset{}` 类型** + 尾随点浮点：`padding: Inset{left: 20. right: 20. top: 16. bottom: 16.}`
  - **显式 `Align{}` 类型**：`align: Align{y: 0.5}` 或 `align: Center`
  - **整数部分的浮点用尾随点形式**：`8.`、`16.`（不是 `8.0`、`16.0`）。
    小数浮点（`0.5`、`1.25`、`3.14` 等）照常写，不需要也不能加尾随点——
    它们本身已经有嵌入小数点
  - **`SolidView` / `RoundedView` 不需要** `show_bg: true` 或 `new_batch: true`——
    它们开箱就渲染背景
- **禁用 widget**：
  - **不要使用 `ScrollYView`**——它在 Splash eval 路径下渲染空白（Canvas
    会把整个 Splash panel 包在自己的 scroll container 里）
  - **不要使用需要 `on_after_apply` 的 widget**（Markdown 嵌套 CodeView 这类），
    它们在 eval 路径下类型默认不继承
- **布局骨架**：
  - 根容器：`SolidView { width: Fill, height: Fit, flow: Down, draw_bg.color: ... }`
  - forecast 是 `View { flow: Right, spacing: 8., ... }` 内横排几个 `RoundedView`
    day chip，**不是** `ScrollYView` 或列表 widget
- **颜色 keyed on condition**：六种 condition 映射到六组色板（background +
  text color），硬编码在 render 函数里。具体色值由实施者选，但必须在 i18n /
  accessibility 可读性范围内（contrast ratio ≥ 4.5:1）。
- **字体大小使用 `draw_text.text_style.font_size`**：避免 CSS-like 别名。
  temp 主数字用大号（40-48），location 中号（16-20），其它小号（10-12）。

### 解析与接线

- **事件路径**：在 `src/home/room_screen.rs` 现有的 `org.octos.splash_card`
  解析点旁边新增 `org.octos.app` 解析分支。顺序：
  1. 先看事件 content 是否有 `org.octos.app` + valid `type` + version 匹配的
     registry 条目 → 走 app registry 路径
  2. 否则 fall through 到现有的 `org.octos.splash_card` 路径（仅开发构建启用）
  3. 否则 fall through 到普通 text / formatted body 渲染
- **Splash widget 复用**：继续使用 `Message` 模板里现有的 `splash_card`
  Splash widget 槽位（`src/home/room_screen.rs:1968`）。registry 产出
  的 Splash DSL 通过 `splash_widget.set_text(cx, splash_code)` 注入。**不要**新增
  第二个 Splash widget 槽位。
- **immutability**：解析时必须读**原始事件 content**，不得读 `m.new_content`。
  这条从 master spec 继承，本 spec 不重新讨论，但实施者必须对着 `m.replace`
  测试确认。
- **与 `org.octos.actions` 的组合**：如果同一事件 content 同时有
  `org.octos.app.type = "weather"` 和 `org.octos.actions` 列表，两者独立
  渲染——weather card 通过 registry 渲染在 `splash_card` 槽位，action row
  通过 Phase 4c 渲染在 action-row 槽位。**本 spec 不处理按钮**。

### 校验规则

- **必填字段缺失**：fail closed——整个 `org.octos.app` 被忽略，消息 fall back
  到普通 body 文本渲染，warning log 包含 `type = "weather"` 和缺失字段名。
- **数值范围超限**：fail closed，同上。
- **未知 `condition` 枚举值**：**不 fail close**，映射为 `"sunny"` 默认值，
  warning log 记录原值。
- **`location` 超长（> 64 字符）**：截断到 64 字符，**尾部追加 U+2026 HORIZONTAL
  ELLIPSIS**（`…`），warning log 记录原长度。截断逻辑基于 Unicode grapheme
  clusters，不是 `str::len()`。
- **`forecast` 超长（> 7 条）**：取前 7 条，warning log 记录丢弃数量。
- **`updated_at` 解析失败**：字段设为 `None`，卡片正常渲染但不显示更新时间戳。
  不 fail close 整张卡片。
- **`periods` 超长（> 3 条）**：**静默截断**取前 3 条——**不** warning log
  （`weather.rs:475` 的 `.take(3)` 行为）。与 `forecast` 的"截断 + warning"
  行为有意不同，本 spec 忠实记录为静默。
- **`periods[].condition` 未知值**：fall back 到 `"sunny"`，**不** warning
  log（`weather.rs:499-503`）。与根级 `condition` 的"未知值 + warning"行为
  有意不同——差异来自代码现状，本 spec 忠实记录，对齐属于行为变更，归独立
  任务。
- **`periods[].precipitation_probability` 浮点输入**：代码先 `as_i64()`，
  失败时回退 `as_f64().map(|raw| raw.round() as i64)`；再做 `0..=100` 范围
  检查（`weather.rs:507-515`）。非数字类型 fail closed；超范围 fail closed。
- **Splash-safe 字符串转义**：所有从 `initial_state` 来的字符串字段在插入
  Splash DSL 前必须转义以下字符：
  - `"` → `\"`
  - `\` → `\\`
  - 换行 `\n` → 空格
  - 控制字符 (U+0000 to U+001F 除 `\t`) → 空格
  转义函数是纯函数，测试独立可覆盖。

### i18n

- 本 spec 不翻译任何 `initial_state` 内容（那是 agent 的责任）
- 但新增 i18n key：
  - `agent_to_app.weather.feels_like` → EN: "Feels like" / ZH: "体感"
  - `agent_to_app.weather.humidity` → EN: "Humidity" / ZH: "湿度"
  - `agent_to_app.weather.wind` → EN: "Wind" / ZH: "风速"
  - `agent_to_app.weather.forecast` → EN: "Forecast" / ZH: "预报"
  - `agent_to_app.weather.updated_at_prefix` → EN: "Updated" / ZH: "更新于"
- Label 文案在 render 时已经按 **effective language** 解析成字符串
  （`initial_state.language` 优先于 `AppState::app_language`，见 §Registry
  条目），不在 Splash DSL 里保留 i18n key。

## Boundaries

### Allowed Changes

- src/home/mod.rs (add `pub mod app_registry;` line; no other changes)
- src/home/room_screen.rs
- src/home/app_registry/mod.rs (new)
- src/home/app_registry/weather.rs (new)
- resources/i18n/en.json
- resources/i18n/zh-CN.json
- specs/task-agent-to-app-l1-weather-card.spec.md

### Forbidden

- 不要扩展 `org.octos.splash_card` 原始字符串路径让它接 JSON 对象或 app
  envelope——master spec 明确禁止。具体地：
  - 可以在**同一个 content 解析点**加一个**并列的** `org.octos.app` 解析
    分支（新代码路径），只要和现有 `org.octos.splash_card` 分支互相独立、
    条件互斥
  - 不可以把 `org.octos.splash_card` 本身的字符串解析或 eval 逻辑改成
    "如果看起来是 JSON 就当 app envelope 处理"
  - 不可以删除或禁用现有 `splash_card` 分支（生产禁用由 master spec 的
    build-mode gating 统一处理，不在本 spec 范围）
- 不要在 `Message` widget 的 `#[rust]` 字段上存天气数据——L1 无状态，render
  输出的 Splash DSL 即是全部
- 不要在 weather render 函数里调用 `ScrollYView` 或任何需要 `on_after_apply`
  的 widget
- 不要使用 `script_mod!` 编译期语法风格写 Splash DSL（嵌套 `draw_bg: { ... }`、
  裸浮点 `8.0`）——必须用 Canvas eval 语法
- 不要为 weather 类型新增 `on_tick` / `on_action` / `teardown`——L1 不需要
- 不要新增 cargo 依赖
- 不要为 weather 卡片渲染路径单独开 tokio 任务
- 不要让未通过 `init` 校验的消息触发 Splash eval
- 不要硬编码中英文以外的 i18n——v1 只支持 EN 和 zh-CN

## Out of Scope

- 天气 refresh 按钮（归 L2a 新闻阅读器 / 通用外部 action row 子 spec）
- 卡内点击图标放大（归 L2b in-card control 子 spec）
- 天气图标的真实图片资源（v1 只用 emoji 或文本符号）
- 多地点同屏对比
- 小时级预报
- 空气质量 / AQI
- **v3+ schema 演进**：超出 v2 已承认字段集（`language` / `focus` +
  `high_c` / `low_c` / `uv_index_max` / `precipitation_probability_max` /
  `periods`）之外的新字段，归独立的 weather v3 spec；本 spec 不前瞻声明
- 跨 restart 持久化上一次渲染的 weather payload
- 自动从外部 API 拉取天气数据（那是 agent 侧的事，不是 Robrix 的事）
- i18n 扩展到 EN/zh-CN 之外

## Completion Criteria

Scenario: Valid weather payload renders via app registry and appears in splash_card slot
  Test: test_valid_weather_payload_renders_via_registry
  Level: integration
  Targets: app registry routing, weather `init + render`, splash_card slot reuse
  Given a Matrix event with content containing `org.octos.app` with:
    | key           | value                   |
    | type          | "weather"               |
    | version       | 1                       |
    | initial_state | see payload below       |
  And the `initial_state` is:
    ```json
    {
      "location": "北京",
      "temp_c": 22,
      "condition": "sunny",
      "humidity": 65,
      "forecast": [
        {"day": "Mon", "high_c": 24, "low_c": 18, "condition": "sunny"},
        {"day": "Tue", "high_c": 25, "low_c": 19, "condition": "cloudy"}
      ]
    }
    ```
  When Robrix renders the message
  Then the weather type factory's `init` is called with the parsed `initial_state`
  And the factory's `render` produces a Splash DSL string
  And the Splash DSL string is injected into the message's existing `splash_card` slot via `set_text`
  And the rendered card visually shows "北京", "22", "sunny", "Mon", "Tue"
  And no raw `org.octos.splash_card` parsing path is invoked for this event

Scenario: Weather payload without registered type fall-back is ignored correctly
  Test: test_unregistered_type_ignored
  Given a Matrix event with `org.octos.app.type = "weird_weather_v2"`
  And the registry does NOT contain an entry for `"weird_weather_v2"`
  When Robrix renders the message
  Then no Splash DSL is injected into the `splash_card` slot
  And the message renders its body as plain text (per master spec §协议 envelope unknown-type fallback)
  And a warning is logged containing the unrecognized `type`

Scenario: Missing required field fails closed with warning
  Test: test_missing_required_field_fails_closed
  Given a Matrix event with `org.octos.app.type = "weather"` and `initial_state = {"temp_c": 22, "condition": "sunny"}`
  And the required `location` field is missing
  When the weather factory's `init` is called
  Then `init` returns a validation error naming `location`
  And Robrix falls back to plain text body rendering
  And a warning is logged containing `type = "weather"` and the missing field name
  And the `splash_card` slot is not populated

Scenario: Temperature outside plausible range fails closed
  Test: test_temperature_out_of_range_fails_closed
  Given a weather payload with `temp_c = -100`
  When the factory's `init` validates the payload
  Then `init` returns a validation error naming `temp_c`
  And the message falls back to plain text
  And a warning is logged naming the field and the out-of-range value

Scenario: Unknown condition value does NOT fail closed — falls back to "sunny"
  Test: test_unknown_condition_falls_back_to_sunny
  Given a weather payload with `condition = "alien_storm"`
  And all other required fields are valid
  When the factory's `init` validates the payload
  Then `init` succeeds and normalizes `condition` to `"sunny"`
  And a warning is logged naming the unknown condition value
  And the card renders successfully using the sunny color palette

Scenario: Missing optional fields renders card without them
  Test: test_optional_fields_absent_renders_minimum_card
  Given a weather payload with only `location = "Beijing"`, `temp_c = 22`, `condition = "cloudy"`
  And no `feels_like_c`, `humidity`, `wind_kph`, `updated_at`, or `forecast`
  When Robrix renders the message
  Then the card renders successfully
  And the card visually shows the location, temperature, and condition
  And the card does NOT visually show the "Feels like", "Humidity", "Wind", "Forecast", or "Updated" labels

Scenario: Long location is truncated with ellipsis based on grapheme clusters
  Test: test_long_location_truncated_with_ellipsis
  Given a weather payload with `location` equal to a 200-character string (mixed Latin + CJK)
  When the factory's `init` processes the payload
  Then the resulting `RenderedWeather.location` is at most 65 grapheme clusters long
  And the last grapheme cluster is U+2026 `…`
  And the truncation is done on grapheme clusters, not byte indices
  And a warning is logged naming the original length

Scenario: Forecast longer than 7 entries is truncated with warning
  Test: test_forecast_over_seven_truncated
  Given a weather payload with a 10-entry `forecast` array
  When the factory's `init` processes the payload
  Then the resulting `RenderedWeather.forecast` contains exactly 7 entries
  And those 7 entries are the first 7 of the input
  And a warning is logged naming the dropped count

Scenario: String field is escaped before insertion into Splash DSL
  Test: test_location_string_is_splash_escaped
  Given a weather payload with `location = "Beijing\"; rm -rf /\""`
  When the factory's `render` produces the Splash DSL string
  Then the output contains the literal sequence `Beijing\\\"; rm -rf /\\\"`
  And the output does NOT contain unescaped `"` inside the location string literal
  And the rendered card displays the text without executing any injected Splash code

Scenario: Generated Splash DSL follows Canvas eval-path syntax requirements
  Test: test_render_output_uses_canvas_eval_syntax
  Given a minimum valid weather payload
  When the factory's `render` produces the Splash DSL string
  Then the output uses `draw_bg.radius:` (NOT `draw_bg.border_radius:`)
  And the output uses dot-path property access (NOT nested `draw_bg: { ... }`)
  And any whole-number float literal in the output uses the trailing-dot form (e.g. `8.`, `16.`) rather than the explicit-zero form (`8.0`, `16.0`)
  And fractional float literals (`0.5`, `1.25`) are left unchanged — they already contain a decimal point
  And the output uses explicit `Inset{}` type for padding values
  And the output does NOT contain the substring `ScrollYView`
  And the output does NOT contain `show_bg: true` on `SolidView` or `RoundedView` containers

Scenario: Weather card and org.octos.actions coexist independently in the same event
  Test: test_weather_card_coexists_with_actions_row
  Level: integration
  Targets: app registry routing, Phase 4c action-button path, field independence invariant
  Given a Matrix event with both `org.octos.app` (valid weather payload) and `org.octos.actions = [{"id": "refresh", "label": "Refresh"}]`
  When Robrix renders the message
  Then the weather card is rendered via the app registry into the `splash_card` slot
  And the refresh button is rendered via the existing Phase 4c action-button row
  And clicking the refresh button sends an `org.octos.action_response` (per Phase 4c), NOT an app-envelope response
  And removing the `org.octos.actions` field still renders the weather card successfully
  And removing the `org.octos.app` field still renders the action row successfully

Scenario: m.replace edit targeting a weather event is ignored at render time
  Test: test_m_replace_edit_to_weather_event_ignored
  Given an original Matrix event with `org.octos.app.type = "weather"` and `location = "Beijing"`
  And a later `m.replace` edit whose `m.new_content` sets `initial_state.location = "Shenzhen"`
  When Robrix renders the message in the timeline
  Then the rendered weather card still shows `location = "Beijing"` (from the original event)
  And the `m.replace` edit has no effect on the rendered app envelope
  And this enforces the master spec §消息不可变性 rule

Scenario: Raw org.octos.splash_card path is NOT used when org.octos.app is present
  Test: test_app_envelope_takes_priority_over_raw_splash_card
  Given a Matrix event that contains BOTH `org.octos.app` (valid weather payload) and `org.octos.splash_card` (a raw Splash string)
  When Robrix renders the message
  Then only the app registry path produces the Splash DSL for the `splash_card` slot
  And the raw `splash_card` string is ignored
  And a debug log notes that app envelope took priority

Scenario: i18n labels resolve via the current app language when payload language is absent
  Test: test_weather_labels_resolve_via_i18n
  Given a valid weather payload with `humidity = 65` and no `initial_state.language`
  And the app language is `zh-CN`
  When the factory's `render` produces the Splash DSL string
  Then the output contains the literal text `湿度`
  And the output does NOT contain the literal text `Humidity`
  And switching app language to `en` produces output containing `Humidity` instead

Scenario: payload language overrides app UI language for i18n labels
  Test: test_payload_language_overrides_app_language_for_guidance_card
  Given a valid weather payload with `initial_state.language = "zh-CN"` and `humidity = 65`
  And the app's current `AppLanguage` is `en`
  When the factory's `render` is called with the effective language resolved per §Registry 条目
  Then the output contains the literal text `湿度`
  And the output does NOT contain the literal text `Humidity`
  And switching `AppLanguage` to `zh-CN` (matching the payload) leaves the output unchanged
  And switching `AppLanguage` back to `en` does NOT revert the output to English while `initial_state.language = "zh-CN"` is present
