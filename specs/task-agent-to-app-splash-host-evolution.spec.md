spec: task
name: "Agent-to-App Splash Host Evolution — Layer 2 host + template extraction"
inherits: project
tags: [bot, agent-to-app, splash, host, widget-manifest, template, layer-2, w5, w7]
depends: [task-agent-to-app-system, task-agent-to-app-producer-routing, task-agent-to-app-l1-weather-card]
estimate: 5d
---

## Intent

`task-agent-to-app-producer-routing` 落地了 **Resolver + Dispatcher + Capability
trait + `weather_guidance`**，证明了 `org.octos.app` envelope 路径端到端可跑。
但当前 `weather_guidance::render_guidance_weather` 仍在 Rust 里用 `out.push_str`
拼 Splash 字符串（`octos/crates/octos-agent/src/capabilities/weather_guidance.rs`
约 200 行），**与设计文档 §3 "one Rust UI codebase; every app is a
declarative four-part artifact" 直接冲突**。

设计文档（`docs/design/agent-to-app-design.md` §7.2）明确承认：当前 weather 是
**保守版 reference implementation，不是终局形态**。

本 spec 把 Rust UI 代码从 capability 里抽出，替换为 **Layer 2 Splash Host +
静态模板文件 + widget manifest + 受控本地函数白名单**，并用 **`news_guidance`
作为首个"零 Rust UI"capability** 证明新形态可走通。

同时吸收 A2UI guardrails（设计文档 §6 W5-W7、§9 repair loop、§6.1 attribution）
作为 Host 契约的一部分。**不采用 A2UI 的 adjacency-list 组件图作为主视图
表达**（设计文档 §10 Non-goals 已锁定 Splash-first）。

本 spec **不涉及**：
- L2 交互式 action（`org.octos.actions` 按钮语义，归 `task-agent-to-app-l2-actions`）
- L3 stateful mini-app tick 调度（归 `task-agent-to-app-l3-stateful`）
- 生成式模板（Template-Author LLM 的 Layer 5b 落地，待首个需要生成式的
  capability 出现时再开独立 spec）
- 动态 capability 加载 / 插件机制（设计文档 §10 Non-goals）

## Decisions

### Runtime contract cross-reference（2026-04-23）

本 spec 定义 Layer 2 Splash Host 的结构契约和 weather/news 迁移目标；
`specs/task-agent-to-app-template-runtime.spec.md` 定义并补充其运行时契约：
preflight 五重检查、`TemplateHandle` cache、版本兼容矩阵、fallback 层级、
`FallbackReason` / `ValidationError` 错误形状。凡涉及 preflight/cache/fallback
的实现细节，以 template-runtime spec 为准；本 spec 保持结构边界不变。

Dependency boundary 同步：本 spec 仍禁止为 splash-host-evolution 本体引入
新的 transitive cargo crate。template-runtime spec 明确允许把已存在于
`Cargo.lock` 的 `sha2` 提升为 direct dependency 来实现 sha256 截断哈希；
这不扩大 build graph。

### Layer 2 Splash Host trait（新增）

- **trait 位置**：`robrix2/src/home/app_registry/splash_host.rs`（新建）。
- **trait 形状**（最小集，v1）：
  - `fn load_template(&self, capability_id: &str, template_id: &str) -> Result<TemplateHandle, HostError>`
    —— 从 manifest 查找 `(capability_id, template_id)` 对应的 `*.splash` 文件，
    解析并校验为 `TemplateHandle`（预处理后的可绑定中间形态）。dev build
    支持 `include_str!` + Makepad live reload，prod build 用 `include_str!`
    静态编入。
  - `fn render_to_splash(&self, handle: &TemplateHandle, state: &JsonValue, chrome: &AttributionChrome) -> Result<String, HostError>`
    —— 绑定 state、注入 attribution chrome（在 chrome 层外侧 wrap），输出
    **Splash DSL 字符串**。这是 v1 与现有 seam 兼容的关键契约：host 输出
    字符串，直接喂给现有 timeline `Splash` widget `set_text()`；不改
    `RenderedApp::render() -> String` 契约、不改 timeline 挂载方式。
  - `fn apply_state_update(&self, handle: &TemplateHandle, state: &mut JsonValue, path: &str, value: &JsonValue) -> Result<bool, HostError>`
    —— path-scoped upsert 作用在 state JSON 上。v1 实现 `replace + remove`；
    `append/splice` 留给 L2/L3 spec。返回 `bool` 指示 state 是否被改动
    （triggers re-render）。
  - `fn route_action(&self, capability_id: &str, action_id: &str, payload: &JsonValue) -> Result<ActionOutcome, HostError>`
    —— 触发 capability 的远端 action 或本地 W7 函数。v1 只需实现透传到
    capability action handler + 本地 W7 函数查表 + openUrl 透传（`org.octos.actions`
    远端语义已有，不在本 spec 新增）。

- **与现有 seam 的接线约束**：
  - `RenderedApp::render(envelope) -> String`（`src/home/app_registry/mod.rs:51`）
    契约不变。本 spec 只改 **实现**：从 `render_app_envelope_to_splash()` 的
    "Rust 里手写字符串" 换成 "SplashHost::render_to_splash() 读模板 + 绑 state"。
  - timeline 挂载（`src/home/room_screen.rs` 的 `content.splash_card.set_text(...)`
    路径）不动；本 spec **不** 修改 `room_screen.rs`。
  - attribution chrome 由 host 在 Splash 字符串的最外层容器注入，而不是独立
    widget —— 在 Splash DSL 层面实现"模板内容区 + chrome 容器"的结构。

- **host 负责、template 不负责**的边界：
  - 模板文件中**禁止**出现 `capability_id` / `display_name` / `icon` /
    `trust_badge` 字段的 override —— 这些由 host chrome 从 envelope metadata
    注入（设计文档 §6.1）。
  - 模板文件中**禁止**使用 `script_mod!` 内 `script:` 块 / `rust {}` 等任何
    会执行任意代码的 Makepad 构造。Host 在加载时静态扫描并拒绝。

- **lifecycle**：
  - widget 随 timeline item 滚出视口时销毁（沿用 Robrix 现有 timeline 复用
    机制）；host 需保证 `apply_state_update` 对已销毁 widget 是 no-op，不 panic。

### Widget Manifest 与 W5 trust whitelist

- **manifest 位置**：`robrix2/src/home/app_registry/widget_manifest.rs`
  —— 静态 `LazyLock<HashMap<&'static str, WidgetDescriptor>>`，v1 手写维护。
- **WidgetDescriptor 字段**：
  - `name: &'static str` —— 模板里引用的名字（如 `"WeatherCard"`、`"Label"`）
  - `trust_level: TrustLevel { Public, Internal, Sensitive }`
  - `prop_schema: &'static [PropSpec]` —— 每个 prop 的名字、类型、是否必填
  - `module: &'static str` —— `script_mod!` 所在模块路径（用于生成 Splash
    `import` 语句）
- **W5 强制点**：`load_template` 扫描模板 AST，遇到任何不在 manifest 中或
  `trust_level != Public` 的 widget 名字时，**拒绝编译该模板**并返回
  `HostError::WidgetNotAllowed { name, trust_level }`。
- **v1 manifest 覆盖范围**：
  - Makepad builtins（`View`, `Label`, `Icon`, `Image`, `Button`, `RoundedView`）
    标 `Public`
  - **v1 addendum（2026-04-23）**：weather v1 模板**不** 抽出
    weather-specific widgets。现实核对结果：`weather.rs` 目前只是用
    `out.push_str` 拼 primitives，没有任何 `script_mod!` weather widget
    可"抽取"；起草 plan 时假设的 `TemperatureBar` / `FocusTile` / `UvChip`
    命名只是理想化 naming，不是已存在实体。v1 weather 模板直接依赖上面
    6 个 Makepad builtins；跨 capability 的通用 widget 抽象**延后到**
    `news` 也落下来后再决定（避免单-capability 过早抽象）。
  - 任何 auth / payment / 敏感表单类 widget 默认 `Sensitive`（本 spec 不引入
    实例，只固定分类约定）

### W7 Local Function Registry

- **注册位置**：`robrix2/src/home/app_registry/local_functions.rs` 新增
  `LocalFunctionRegistry` —— 静态表，Host 启动时装载。
- **v1 允许的本地函数（闭集）**：
  - `open_url(url: string)` —— 平台 open；Matrix timeline 消息链接的现有
    `OpenLink` 行为复用
  - `format_date(value: string, pattern: string) -> string`
  - `format_number(value: number, precision: u8) -> string`
  - `required(value) -> bool` —— 非空校验
  - `regex_match(value: string, pattern: string) -> bool`
- **W7 强制点**：模板里引用的任何 `${fn_name(...)}` 或
  `action: { functionCall: { call: "...", args: {...} } }` 必须名字在表中且
  参数类型匹配；否则 `load_template` 拒绝。
- **禁止项**：
  - 模板内禁止定义新函数（无 let / fn 语法）
  - Host 不提供通用表达式求值器；只允许 `${state.path}` 绑定 + 已注册函数调用
  - `LocalFunctionRegistry` **非** extensible-from-template；新增需改 Host Rust
    代码并通过设计审查

### Attribution Chrome（host-owned identity）

- **AttributionChrome struct**：`{ capability_id: String, display_name:
  String, icon_url: Option<String>, trust_badge: TrustBadge }`
- **注入点**：Host `render()` 方法接收并强制渲染 chrome 区域（显示 capability
  名与图标），**模板无法覆盖**（host 把 chrome 放在模板内容区外侧容器）。
- **数据来源（v1 + future path）**：本 spec 覆盖的 v1 capabilities 全部
  是**编译期内建**（weather + news），其 `AttributionChrome` 字段
  （`display_name` / `icon_url`）可由客户端静态表推导，因此 v1 实装走:
  - **Robrix 侧静态 `CapabilityDescriptor` 表**（keyed by `app_type`），
    每个条目定义该 capability 的 chrome 字段。
  - OctOS dispatcher v1 **不** 在 envelope 写 chrome metadata——这避免了
    与 master spec / producer-routing spec 锁定的 envelope 协议
    （`type + version + initial_state` 三字段）冲突。
  - 每个 app 的 `RenderedApp::render` 从该表查表得到 chrome，再传给 host。

  **Future path**（本 spec 不打开）：当第一个 **动态 / 第三方 /
  orchestrator-gated** capability ship 时，开一条独立 spec amendment 形式化
  envelope chrome channel（最可能的形态：作为 `initial_state` 的 optional
  扩展字段，沿用 `language` / `focus` 已有的"optional 不 bump version"
  规则；或在 envelope 顶层加一个 optional sibling，届时再做协议决策）。
  设计文档 §6.1 的 "orchestrator must verify and stamp these fields" 语言
  已为此预留入口。

  这条 v1 偏离是**显式的**：任何看到 spec 中 "envelope metadata" 提法的
  实装者，务必读到这一节才动手；不得私自引入 envelope 协议变更。
- **trust_badge 语义（v1）**：编译期 capability 全部标 `TrustBadge::Builtin`；
  为将来第三方 / 沙箱 capability 预留 `Verified / Unverified` 值，本 spec 不
  使用它们。

### Template File 格式与位置

- **资产归属（v1 硬约束）**：**模板文件属于 Robrix 客户端仓的静态资产**，
  随 Robrix release 一起发布；**不** 由 OctOS 下发、**不** 跨越 envelope
  协议边界。理由：
  - envelope 协议由 master spec 锁定，不许改
  - Splash DSL 是客户端渲染语言，模板与 widget library 同代码库演化最一致
  - 模板升级走 Robrix release，版本兼容由 envelope `version` + consumer
    `supports_version()` 已有机制承担
  - 未来若需要服务端下发模板，那是 Layer 5b 生成式模板的路径（`§4.3b`），
    要过 validation repair loop；本 spec 不打开这条路径
- **位置约定**：`robrix2/src/home/app_registry/templates/<capability_id>/<template_id>.splash`
- **wire 规定**：纯 Makepad 2.0 `script_mod!` DSL 片段（`live_design!` 已于
  Makepad 2.0 弃用，项目 CLAUDE.md 已规定），项目内 `AGENTS.md` +
  `makepad-2.0-dsl` skill 定义语法。
- **binding 语法**：`$state.path.to.field`（host 在 render 阶段替换为具体值）。
  路径语法对齐 JSON Pointer 语义（`/user/name` 或点表示 `state.user.name`
  任选其一，v1 统一用点表示）。
- **编译契约**：模板变更必须通过 `cargo check` 的 linter（本 spec 新增）：
  - 静态扫描模板 AST，W5 + W7 + attribution-lock 三重校验
  - 失败 = `cargo check` 失败；dev 模式下给出具体行号 + 违规名

### Weather Template 抽取（Migration Scenario）

- **delete**：`octos/crates/octos-agent/src/capabilities/weather_guidance.rs`
  中 `render_guidance_weather` 函数及其 200 行 `out.push_str` 代码。
- **delete**：相关 Splash-string-building helper（`render_focus_tile` 等）。
- **create**：`robrix2/src/home/app_registry/templates/weather_guidance/card_standard.splash`
  —— 客户端 Robrix 仓内的静态模板资产；与现有字符串 output 在输出的 Splash
  DSL 结构上等价，渲染出的视觉与现有 weather 卡 **视觉像素级一致**（可用
  当前 weather 卡截图做回归基准）。
- **v1 addendum（2026-04-23）**：**不** 新建
  `robrix2/src/home/app_registry/widgets/weather_card.rs`。起草时假设
  的 `TemperatureBar` / `FocusTile` / `UvChip` 在现实代码中不存在，
  v1 weather 模板直接用已登记的 Makepad builtins（`View` / `RoundedView` /
  `Label` / `Icon` / `Image` / `Button`）组合出视觉效果。跨 capability
  通用 widget 抽象延后到 `news` 出现后再决定。
- **rewire**：`robrix2/src/home/app_registry/mod.rs` 里现有的
  `render_app_envelope_to_splash()` 路径改为：读 envelope → 找
  `(capability_id, template_id)` → 调用 `SplashHost::render_to_splash()` →
  返回 String 给原有 consumer 接口。`RenderedApp::render() -> String`
  契约保持。
- **keep**：`weather_guidance.rs` 里 `fetch_data` / `build_state` /
  `build_body` 三函数保留；**仅删除 UI 代码**。
- **keep**：`Capability::template_ids()` 返回 `&["card_standard"]`（v1 单模板）。

### news_guidance 作为"零 Rust UI"终验

- **capability_id**: `"news_guidance"`
- **app_type**: `"news"`（Robrix consumer 侧需在 `src/home/app_registry/mod.rs`
  注册；本 spec 包含该注册）
- **app_version**: 1（新 app type，从 v1 起步）
- **supported_focuses**: `["headlines", "digest"]`（v1 两个最小集；`deep_dive`
  留到 L2 spec）
- **required_slots**: `topic: string, time_range: enum{"today","this_week"}`（默认
  `today`）
- **min_confidence**: 0.6
- **Rust 代码量约束**：整个 `news_guidance.rs` **必须 ≤ 120 行**（含空行与
  `use` 声明；含 `fetch_data` + `build_state` + `build_body` + trait impl）。
  超过 120 行视为 spec 验收失败。
- **Splash 文件数**：`headlines_card.splash` + `digest_card.splash` 两个。
- **新 widget 数**：最多 2 个 news-specific widgets（如 `NewsTile`、
  `SourceChip`）登记到 `WidgetManifest`。

### Resolver / Dispatcher 侧变更（最小）

- **Resolver prompt 增量**：`news_guidance` 的 2 focus × EN/zh 共 ≥ 40 条
  fixture；整体 robustness gate 门槛不变（top-1 ≥ 90%，FPR = 0%）。
- **Dispatcher**：无结构变化；`news_guidance` 直接注册到现有
  `CapabilityRegistry`。
- **`template_id` 选择**：v1 单模板 capability 直接 `supported_focuses()` 决定，
  Resolver 不输出 `template_id` 字段；`template_id` 选择属 capability 内部
  逻辑（focus → template_id 映射由 capability 声明）。

### Generated-template mode（Layer 5b / §4.3b）——**本 spec 不落地**

- 接口预留：`Capability::template_ids()` 可返回 `TemplateSlot { id, kind:
  Static | Generated }`，本 spec 只实现 `Static` 分支。
- **Generated 分支留到下一个 spec**：触发时机是第一个真正需要视觉变体的
  capability（设计文档举例：news deep_dive 卡面按 story type 自适应）。
- 本 spec 只确认：host 的 `load_template` 路径对 `Generated` slot 返回
  `HostError::GeneratedTemplateNotYetSupported`，测试锁定该行为，防止以后
  绕过 repair loop 偷跑。

### Incremental state update（§4.2）——**v1 实装 replace/remove，不实装完整语义**

- trait 接口已声明（`apply_state_update`），v1 实装 `replace + remove`。
- `append / splice / array-index` 归 `task-agent-to-app-l2-actions`（L2 交互
  产生增量时一并落地）。
- 本 spec 测试锁定 v1 行为：给 `append` 或 `splice` 语义的 update 必须返回
  `HostError::UpdateOpNotYetSupported`，不得静默成 noop 或错误应用。

### Documentation sync

- **更新 `AGENTS.md`**：添加 "Splash template authoring" 小节，引用本 spec 与
  widget manifest 位置。
- **更新 `MAKEPAD.md`**：在 routing 表新增 "Template authoring" 场景，引用
  W5/W7/attribution 三道 guardrail。
- **更新 `docs/design/agent-to-app-design.md` §7.1 Shipped**：添加本 spec
  产出的 Splash Host trait + weather template + news_guidance 三项。
- **更新 `docs/design/agent-to-app-design.md` §7.3 Not yet shipped**：删除已
  落地项，保留 L2/L3/Generated/动态发现等仍未落地项。

## Boundaries

### Allowed Changes

Robrix 仓（`/Users/zhangalex/Work/Projects/FW/robius/robrix2`）：

- `src/home/app_registry/splash_host.rs`（new）
- `src/home/app_registry/widget_manifest.rs`（new）
- `src/home/app_registry/local_functions.rs`（new）
- ~~`src/home/app_registry/widgets/weather_card.rs`~~（**v1 不建**，见
  上节 v1 addendum；weather 模板直接用已登记 Makepad builtins）
- `src/home/app_registry/widgets/news_card.rs`（optional，news widgets —— news 侧是否
  抽出 news-specific widgets 基于真实代码判断；若 news 模板也能直接用
  primitives，则 no-op）
- `src/home/app_registry/templates/weather_guidance/card_standard.splash`（new）
- `src/home/app_registry/templates/news_guidance/headlines_card.splash`（new）
- `src/home/app_registry/templates/news_guidance/digest_card.splash`（new）
- `src/home/app_registry/news.rs`（new，news consumer：与 weather 对称的
  thin consumer，走 SplashHost；仅注册、不含 UI 代码）
- `src/home/app_registry/mod.rs`（register news consumer、wire host）
- `src/home/app_registry/weather.rs`（rewire：现有
  `render_app_envelope_to_splash` 改为调用 SplashHost + 模板；契约不变）
- `docs/design/agent-to-app-design.md`（§7.1 / §7.3 更新）
- `AGENTS.md`, `MAKEPAD.md`（补 template authoring 小节）
- `specs/task-agent-to-app-splash-host-evolution.spec.md`（本文件）

**显式不动的文件**：
- `src/home/room_screen.rs` —— timeline 挂载与 `Splash` widget `set_text()`
  路径保持不变。本 spec 的输出形态是 Splash DSL String，与现有 seam 兼容。

OctOS 仓（`/Users/zhangalex/Work/Projects/FW/octos`）：

- `crates/octos-agent/src/capabilities/weather_guidance.rs`（**删除 UI 代码**
  `render_guidance_weather` 与相关 helper；保留 data / state / body 三函数）
- `crates/octos-agent/src/capabilities/news_guidance.rs`（new，≤ 120 行）
- `crates/octos-agent/src/capabilities/mod.rs`（注册 news）
- `crates/octos-agent/src/resolver/mod.rs`（news fixtures + prompt）
- `crates/octos-cli/src/prompts/resolver_default.txt`（news_guidance 条目）
- `crates/octos-cli/tests/resolver_fixtures/news_guidance/**`（new fixture bundle）

### Forbidden

- **不得**在本 spec 范围内引入 Layer 5b Template-Author LLM 实装；生成式
  模板仅作为 trait 层面的预留 + 测试锁定。
- **不得**修改 `org.octos.app` envelope 协议（master spec 锁定）。
- **不得**让 capability 代码继续持有 UI 字符串生成逻辑 —— weather 迁移必须
  彻底抽干净，否则 spec 验收失败。
- **不得**在模板文件里使用 `script:` / `rust {}` / 任何会执行 Makepad 脚本
  侧代码的构造；template linter 必须检测并拒绝。
- **不得**让 template 覆盖 host chrome（`capability_id` / `display_name` /
  `icon` / `trust_badge` 字段在模板里不可写）。
- **不得**为本 spec 主体引入新的 transitive cargo crate。template-runtime
  spec 对 `sha2` direct dependency 的例外以其更具体的运行时契约为准。
- **不得**让 news_guidance 引入任何 Rust UI 代码（全部 UI 必须在 `.splash`
  文件或 widget manifest 里）。
- **不得**在未达 Robustness gate 门槛前把 `news_guidance` 置为默认路径。
- **不得**为 `news` app type 跳过 Robrix consumer 注册 —— consumer 必须在
  `app_registry/mod.rs` 登记，否则 Robrix 会 fallback 到 plain text 渲染。

## Out of Scope

- L2 交互动作（按钮、表单提交、远端 action roundtrip）—— `task-agent-to-app-l2-actions`
- L3 stateful mini-app tick 调度 / 后台运行 —— `task-agent-to-app-l3-stateful`
- Template-Author LLM 的实际落地（Layer 5b 生成式模板 + repair loop 实装）
  —— 待首个生成式 capability 独立 spec
- 动态 capability 加载 / 插件 / 热装载
- Template migration tool（旧的 Rust-heavy 形态到新形态的批量转换）
  —— weather 一次性手工迁移即可，news 是全新形态
- WebView / HTML 兜底渲染 —— 设计文档 §10 Non-goals 明确拒绝
- 第三方 widget 包分发 —— W5 manifest v1 手写维护，包分发归未来独立 spec
- Splash 语法扩展（`$state.path` 绑定语义如需扩展，归独立 Makepad skill spec）
- Performance benchmarks（模板加载 / render 耗时基线）—— 本 spec 只要求
  功能正确，latency 门槛归 performance spec
- Cross-language（zh/EN 之外语言）支持 —— weather v2 已有 language slot，
  本 spec 延续不扩展

## Completion Criteria

Scenario: SplashHost loads a whitelisted template and binds state to Splash DSL output
  Test: test_splash_host_loads_weather_card_and_binds_state
  Given the WidgetManifest registers `View`, `RoundedView`, `Label`, `Icon`, `Image`, `Button` as `TrustLevel::Public` (Makepad builtins; weather v1 uses primitives, no weather-specific widget per the v1 addendum)
  And `src/home/app_registry/templates/weather_guidance/card_standard.splash` references only whitelisted widgets
  When the host calls `load_template("weather_guidance", "card_standard")`
  And then `render_to_splash(handle, state, chrome)` with a sample weather state
  Then the returned Splash DSL string is non-empty
  And the string contains a `RoundedView {` top-level container (the weather card root)
  And the string contains the temperature reading derived from `state.temperature_c` at the bound position
  And the string contains the AttributionChrome `display_name = "Weather"` in the host-owned chrome wrapper, separated from the template's content region

Scenario: SplashHost rejects a template that references a non-whitelisted widget
  Test: test_splash_host_rejects_unlisted_widget
  Given the WidgetManifest does NOT contain `EvilWidget`
  And a template references `EvilWidget`
  When the host calls `load_template(capability, template_id)`
  Then the call returns `HostError::WidgetNotAllowed { name: "EvilWidget", trust_level: None }`
  And no Splash DSL string is produced

Scenario: SplashHost rejects a template that references a non-whitelisted local function
  Test: test_splash_host_rejects_unlisted_local_function
  Given the LocalFunctionRegistry does NOT contain `exec_shell`
  And a template contains `action: { functionCall: { call: "exec_shell", args: {} } }`
  When the host calls `load_template(capability, template_id)`
  Then the call returns `HostError::LocalFunctionNotAllowed { name: "exec_shell" }`
  And no Splash DSL string is produced

Scenario: SplashHost rejects a template that attempts to override host-owned attribution
  Test: test_splash_host_rejects_attribution_override
  Given a template sets `capability_id: "impersonator"` in its content region
  When the host calls `load_template(...)`
  Then the call returns `HostError::AttributionFieldInTemplate { field: "capability_id" }`
  And no Splash DSL string is produced

Scenario: Weather template preserves Splash DSL structural parity after extraction
  Test: test_weather_card_template_structural_parity
  Level: integration
  Given a known weather state fixture (location=Beijing, clothing focus, v2 schema)
  And the recorded baseline is the Splash DSL string that `render_guidance_weather` produced for that fixture pre-migration
  When the new SplashHost + static template renders the same state
  Then the emitted Splash DSL string matches the baseline modulo declared normalization (whitespace, key ordering within each node, attribution chrome wrapper added/removed)
  And `weather_guidance.rs` no longer contains `render_guidance_weather` or any `out.push_str` UI-building calls
  And the existing `content.splash_card.set_text(...)` timeline path renders both the baseline and the new output without visual regression

Scenario: news_guidance capability ships with zero Rust UI code
  Test: test_news_guidance_has_no_rust_ui
  Given `octos/crates/octos-agent/src/capabilities/news_guidance.rs`
  When counted for non-blank non-comment lines
  Then the file contains ≤ 120 lines total
  And the file does NOT contain any `push_str` / `write!` / `format!` calls producing Splash DSL
  And the file does NOT reference any Widget / View / script_mod types from Makepad
  And all news UI lives in `src/home/app_registry/templates/news_guidance/*.splash` plus optional registered widgets

Scenario: news_guidance routes end-to-end from user query to rendered card
  Test: test_news_guidance_end_to_end
  Level: integration
  Given `enable_capability_dispatcher = true` and `capability_dispatcher_gate_passed = true`
  And the user sends "今天有什么科技新闻"
  When the resolver emits `capability_id = "news_guidance"` with `focus = "headlines"`, `slots.topic = "科技"`, `confidence ≥ 0.6`
  And the dispatcher routes to NewsGuidance
  And `fetch_data → build_state → build_body` completes
  Then the outbound message contains an `org.octos.app` envelope with `type = "news"`, `version = 1`, `initial_state.focus = "headlines"`
  And the Matrix `body` contains the same top headline text as `initial_state.items[0].title`
  And Robrix consumer renders via SplashHost using `headlines_card.splash`

Scenario: Resolver robustness gate passes for news_guidance
  Test: test_resolver_fixtures_news_guidance
  Given ≥ 40 news_guidance fixtures (2 focuses × 2 languages × ≥ 10 phrasings)
  And ≥ 10 news negatives (queries that should NOT route to news_guidance)
  When the resolver runs over the fixture set
  Then top-1 accuracy on positives ≥ 90%
  And rejection rate on negatives = 100% (either `capability_id = null` or `confidence < 0.6`)

Scenario: Host applies path-scoped state update (v1 replace/remove only)
  Test: test_host_applies_state_update_replace
  Given state `{ user: { name: "Alice" } }` bound to a loaded template handle
  When the host calls `apply_state_update(handle, &mut state, "/user/name", "Bob")`
  Then the call returns `Ok(true)`
  And the state becomes `{ user: { name: "Bob" } }`
  And the subsequent `render_to_splash` output differs only in the bound field

Scenario: Host rejects unsupported update operations in v1
  Test: test_host_rejects_append_op_in_v1
  Given state `{ items: [] }` bound to a loaded template handle
  When the host is asked to apply an `append`-semantics update to `/items`
  Then the call returns `HostError::UpdateOpNotYetSupported { op: "append" }`
  And the state is NOT mutated

Scenario: Host rejects generated-template slot in v1
  Test: test_host_rejects_generated_template_slot
  Given a capability declares `template_ids` returning `TemplateSlot { id, kind: Generated }`
  When the host calls `load_template(capability, "the_generated_id")`
  Then the call returns `HostError::GeneratedTemplateNotYetSupported`
  And no Splash DSL string is produced

Scenario: Weather capability still passes producer-routing robustness gate after migration
  Test: test_weather_guidance_regression_after_template_extraction
  Given the fixture set from `task-agent-to-app-producer-routing` (≥ 80 phrasings, 4 focuses × 2 languages)
  When the resolver + dispatcher + (migrated) WeatherGuidance capability runs end-to-end
  Then top-1 accuracy ≥ 90% and negative rejection = 100%
  And every emitted `initial_state` JSON is byte-equal to the pre-migration baseline (data layer unchanged)
  And every Matrix `body` is byte-equal to the pre-migration baseline

Scenario: Design document reflects shipped state
  Test: test_design_doc_phase_tracking
  Given `docs/design/agent-to-app-design.md` after this spec lands
  Then §7.1 Shipped lists "SplashHost trait + static template loader", "WidgetManifest + W5 enforcement", "LocalFunctionRegistry + W7 enforcement", "AttributionChrome host-owned", "weather template extraction", "news_guidance first zero-Rust-UI capability"
  And §7.3 Not yet shipped no longer lists the above items
  And §7.3 still lists L2 actions, L3 stateful, generated templates, dynamic capability discovery
