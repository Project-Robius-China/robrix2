warning: Allowed Changes path not found: crates/octos-agent/src/capabilities/** （new 模块） (resolved to ./crates/octos-agent/src/capabilities)
warning: Allowed Changes path not found: crates/octos-agent/src/capabilities/weather_guidance.rs （new） (resolved to ./crates/octos-agent/src/capabilities/weather_guidance.rs （new）)
warning: Allowed Changes path not found: crates/octos-agent/src/tools/send_app_card.rs (resolved to ./crates/octos-agent/src/tools/send_app_card.rs)
warning: Allowed Changes path not found: crates/octos-agent/src/tools/show_weather_card.rs （**单一路径**：改成 (resolved to ./crates/octos-agent/src/tools)
warning: Allowed Changes path not found: crates/octos-agent/src/resolver/** （new 模块，resolver + fixture loader） (resolved to ./crates/octos-agent/src/resolver)
warning: Allowed Changes path not found: crates/octos-cli/src/session_actor.rs (resolved to ./crates/octos-cli/src/session_actor.rs)
warning: Allowed Changes path not found: crates/octos-cli/src/prompts/gateway_default.txt (resolved to ./crates/octos-cli/src/prompts/gateway_default.txt)
warning: Allowed Changes path not found: crates/octos-cli/tests/resolver_fixtures/** （new，测试集 fixture） (resolved to ./crates/octos-cli/tests/resolver_fixtures)
warning: Allowed Changes path not found: specs/task-agent-to-app-l1-weather-card.spec.md （addendum：承认 optional (resolved to ./specs/task-agent-to-app-l1-weather-card.spec.md （addendum：承认 optional)
=== Contract ===

# Task Contract: Agent-to-App Producer Routing — OctOS-side Intent-to-Capability Contract

## Intent
Master spec (`task-agent-to-app-system`) 定义了 **consumer / envelope 合同**：Robrix
按 `type + version + initial_state` 渲染 app 卡片。本 spec 补齐缺失的另一半——
**OctOS 侧从自然语言到 app envelope 的 producer routing 合同**。

当前状态（2026-04-16 复盘）：Robrix 已经是通用的 dumb consumer，但 OctOS
仍然靠 **LLM tool-calling 直接猜要不要出卡**——"今天北京天气"能出天气卡，
"今天北京穿什么"就不会，尽管是同一领域能力。`show_weather_card` 既承担意图
识别又承担数据组装，耦合且 prompt-sensitive。

本 spec 把 producer 侧拆成 **Resolver + Capability Registry + Deterministic
State Builder + Fallback Body Builder** 四件事，并钉死：resolver 不允许纯 prompt
猜、每个 capability 必须同时产出 Matrix `body` 和 app `initial_state` 且两者
同源、`focus` 作为可选扩展字段的兼容规则、`weather_guidance` 作为第一个落地
capability 的最小覆盖集。

本 spec **不修改** `org.octos.app` envelope 协议本身、**不修改** Robrix 任何
consumer/渲染代码——所有改动都在 OctOS 仓库内。

## Must
- All code must compile with `cargo build` on the `feature/mention-user-migration` branch (or `main` after merge)
- All UI widgets must use Makepad 2.0 `script_mod!` DSL syntax — do NOT use Makepad 1.x `live_design!` syntax
- Named widget children must use `:=` operator, NOT `=`
- Property overrides on inherited widgets must use `+:` merge operator to preserve parent properties
- Do NOT use `cargo fmt` — the project does not enforce rustfmt and formatting changes create noisy diffs
- Do NOT add new cargo dependencies without explicit approval in the task spec
- Do NOT use `.unwrap()` on user-facing code paths — use proper error handling with `anyhow` or pattern matching
- Async Matrix operations must go through `submit_async_request(MatrixRequest::*)` — do NOT spawn raw tokio tasks for Matrix API calls
- Widget state changes on dynamically-created widgets (via `widget_ref_from_live_ptr()`) must use Animator + shader instance variables, NOT `script_apply_eval!` (which silently fails due to `ScriptObject::ZERO`)
- `script_apply_eval!` must NOT use DSL constants (`Right`, `Down`, `Fit`, `Fill`, `Align`, `Inset`, `MouseCursor`) — these are not available at runtime scope
- All `draw_bg` property modifications must use `+:` merge syntax, NOT `:` replace syntax, to avoid losing shader/border/animation properties

## Decisions
- UI Framework: Makepad 2.0 with `script_mod!` DSL (fork: `kevinaboos/makepad`, branch: `stack_nav_improvements`)
- Matrix SDK: `matrix-sdk` with sliding sync, E2E encryption, SQLite storage
- Async runtime: Tokio
- State persistence: JSON serialization via serde to `~/.local/share/org.robius.robrix/`
- Widget template instantiation: `crate::widget_ref_from_live_ptr(cx, Some(ptr))` for creating widgets from `#[live] Option<LivePtr>` fields
- Derive macros: `#[derive(Script, ScriptHook, Widget)]` for widget structs (NOT `Live`/`LiveHook`)
- DSL property syntax: whitespace-separated (no commas), `Inset{...}` for margins/padding, `Align{...}` for alignment
- Hex colors with letter 'e': use `#x` prefix (e.g., `#x1E90FF`)
- Background CPU work: `cpu_worker::spawn_cpu_job(cx, CpuJob::*)` via `cx.spawn_thread()`
- Dock state restoration: programmatic tab recreation via `close_all_tabs()` + `focus_or_create_tab()`, NOT `Dock.load_state()` (which corrupts DrawList references)
- **禁止纯 prompt 路由**：不允许让 LLM 通过"自由 tool-calling"直接决定
- **Hybrid 模式必选**：resolver 分两步：
- **confidence 阈值**：`confidence < 0.6` 的结果视为 "未命中 capability"，
- **失败模式**：resolver LLM 超时 / 返回无效 JSON / 所有 capability 都未命中
- **执行顺序（resolver vs 普通 tool-calling）**：每条用户消息进入回复 pipeline
- **`confidence` 语义（v1）**：采用 **LLM 自报 confidence**——LLM 在 JSON
- **可回放**：resolver 的输入（用户消息 + 对话上下文摘要）和输出（JSON）
- **注册表形状**：OctOS 侧新增 `CapabilityRegistry`，是一个
- **Capability trait 必选方法**：
- `id() -> &'static str` — e.g. `"weather_guidance"`
- `app_type() -> &'static str` — e.g. `"weather"`（与 Robrix consumer
- `app_version() -> u32` — 发出的 `org.octos.app.version`
- `supported_focuses() -> &'static [&'static str]` — finite enum，空数组
- `required_slots() -> SlotSchema` — 声明必填/可选 slot 及类型
- `min_confidence() -> f32` — 默认 0.6
- `build_state(slots, data) -> Result<JsonValue, CapabilityError>` —
- `build_body(slots, data, language) -> String` — 确定性函数，输出 Matrix
- **共同数据源**：`build_state` 与 `build_body` **必须共享同一个 `data`
- **一对一映射**：一个 `capability_id` 映射到**恰好一个** `app_type`；反之
- **优先级**：当 resolver 输出的 `capability_id` 存在于 registry 时直接
- **`focus` 是 `initial_state` 的可选字段**：不在 schema 里时等价于 capability
- **不 bump `app_version`**：向 `initial_state` 新增 optional 字段 **不** 触发
- **bump 触发条件（明确）**：以下任一情况必须 bump `app_version`：
- 删除 `initial_state` 中任何已有字段
- 修改已有字段的类型或值域语义
- 把可选字段改为必填
- 变更 focus 枚举中已有值的渲染语义
- 新增 focus 枚举值并**要求** consumer 必须识别（反之新增可 fall back 到
- **unknown focus fallback**：consumer 收到未知 `focus` 值时，必须渲染
- **每个 capability 必须同时产出 `initial_state` 和 `body`**：`send_app_card`
- **禁止"每通道一套逻辑"**：Telegram / CLI / 任何非 app-capable 通道显示的
- **内容不矛盾原则**：`build_state` 和 `build_body` 的输出在事实数据上必须
- 展现详略（`body` 更精简，`state` 可包含更多字段）
- 语言本地化（两者都按传入的 `language` 参数本地化）
- 格式化（`body` 是 Markdown/plain，`state` 是结构化 JSON）
- **fallback 体现**：不支持 app envelope 的通道只渲染 `body`；支持的通道
- **capability_id**: `"weather_guidance"`
- **app_type**: `"weather"`（映射到 `task-agent-to-app-l1-weather-card` 注册
- **app_version**: 与 weather L1 当前 **代码实现** 保持一致——consumer
- **supported_focuses**: `["overview", "clothing", "umbrella", "outdoor"]`
- **default_focus**: `"overview"`
- **required_slots**: `location: string`, `time_scope: enum{"today","tomorrow"}`（默认 `today`），
- **min_confidence**: `0.6`
- **数据源**：复用现有 `show_weather_card` 的天气数据提供者，本 spec 不改
- **focus → state diff**：`focus` 只影响 `initial_state` 里 `focus` 字段本身
- **跨 spec patch（必带）**：weather L1 spec 当前没有声明 `language` /
- **测试集规模**：为 `weather_guidance` 的 4 个 focus 各准备 ≥ 10 条中文
- **通过门槛**：resolver 在此测试集上的 top-1 accuracy ≥ 90%，并且
- **Regression 锁**：测试集 fixture 进仓库，CI 上任何 resolver prompt 或
- **不达标不上线**：fail 了就不能打开 `weather_guidance` capability，仍走

## Boundaries
Allowed changes:
- crates/octos-agent/src/capabilities/** （new 模块）
- crates/octos-agent/src/capabilities/weather_guidance.rs （new）
- crates/octos-agent/src/tools/send_app_card.rs
- crates/octos-agent/src/tools/show_weather_card.rs （**单一路径**：改成
- crates/octos-agent/src/resolver/** （new 模块，resolver + fixture loader）
- crates/octos-cli/src/session_actor.rs
- crates/octos-cli/src/prompts/gateway_default.txt
- crates/octos-cli/tests/resolver_fixtures/** （new，测试集 fixture）
- specs/task-agent-to-app-producer-routing.spec.md
- specs/task-agent-to-app-l1-weather-card.spec.md （addendum：承认 optional
Forbidden:
- 不要修改 `org.octos.app` envelope 协议（`type`/`version`/`initial_state` 三
- 不要修改 Robrix 任何 `src/home/app_registry/**` 代码——本 spec 是
- 不要把 "pure prompt tool-calling" 作为 capability 路由机制（即 LLM 直接
- 不要允许 capability 直接写 `initial_state` 而绕过 `build_state`——`body`
- 不要在 Matrix / Telegram / CLI 三个通道各写一套 body 生成逻辑——body 必须
- 不要为 `focus` 扩展字段 bump `org.octos.app.version`——只要消费者能安全
- 不要在 robustness gate 未达标前把 `weather_guidance` 当默认路径上线。
- 不要新增 cargo 依赖。
- 不要让 resolver 在 LLM 超时 / 无效 JSON 时 panic——必须降级到 plain text。
- 不要让 resolver 输出 multi-candidate（本 spec 只支持 top-1 分派）。
Out of scope:
- Capability 自动发现 / 动态注册（所有 capability 编译进 OctOS）
- Capability 组合（一次回复触发多个 capability，例如天气 + 日程）
- Resolver 模型训练 / 微调（本 spec 只规定接口契约与 regression 测试集）
- 非文本 slot 抽取（图片、位置共享等）
- 多轮对话中 capability 的状态继承（v1 每次回复独立 resolve）
- Telegram / CLI 通道具体 UI 行为（本 spec 只保证 `body` 同源，不规定渲染）
- Weather 以外任何 capability（`news_guidance` / `calendar_guidance` 等走
- 把 `show_weather_card` 从代码库中彻底删除——本 spec 只把它改成 thin
- `org.octos.actions` 按钮与 capability routing 的组合语义（按钮仍走 Phase

## Completion Criteria
Scenario: Resolver emits structured JSON, not free tool choice
  Test:
    Filter: test_resolver_output_is_structured_json
  Given a user message "今天北京穿什么"
  And the resolver is invoked with the shared conversation context
  When the resolver produces a result
  Then the result is a single JSON object with keys `capability_id`, `focus`, `slots`, `confidence`
  And `capability_id` equals `"weather_guidance"`
  And `focus` equals `"clothing"`
  And `slots.location` equals `"北京"`
  And `confidence` is a float in `[0.0, 1.0]`
  And the LLM call used a fixed-schema structured-output mode, NOT multi-tool free selection

Scenario: Hybrid dispatch routes to capability only when registry contains the id
  Test:
    Filter: test_dispatch_routes_only_known_capability
  Given the registry contains `"weather_guidance"` but not `"fitness_guidance"`
  When the resolver returns `capability_id = "fitness_guidance"` with `confidence = 0.95`
  Then the dispatcher does NOT invoke any capability
  And the reply falls back to a plain text response
  And a warning is logged naming the unknown `capability_id`

Scenario: Low-confidence result falls back to plain text, no card emitted
  Test:
    Filter: test_low_confidence_falls_back_to_text
  Given a user message "告诉我北京"
  And the resolver returns `capability_id = "weather_guidance"` with `confidence = 0.45`
  And `weather_guidance.min_confidence() = 0.6`
  When the dispatcher evaluates the result
  Then no `org.octos.app` envelope is emitted
  And the outbound message is a plain-text reply only
  And a debug log records the rejected low-confidence dispatch

Scenario: Resolver LLM timeout degrades to plain text reply
  Test:
    Filter: test_resolver_timeout_degrades_gracefully
  Given the resolver LLM call times out after the configured deadline
  When the dispatcher handles the timeout
  Then no panic is raised
  And the outbound reply is a plain-text response generated by the default pathway
  And a warning is logged indicating resolver timeout

Scenario: Resolver returns invalid JSON and the producer degrades gracefully
  Test:
    Filter: test_resolver_invalid_json_degrades
  Given the resolver LLM returns a malformed JSON string
  When the dispatcher attempts to parse the result
  Then parsing fails and the dispatcher does NOT retry indefinitely
  And the outbound reply is a plain-text response
  And a warning is logged naming the JSON parse error

Scenario: build_state and build_body are produced from the same data fetch
  Test:
    Filter: test_state_and_body_share_one_data_fetch
  Given the weather data provider is instrumented to count fetch calls
  And a valid user message triggers `weather_guidance`
  When the capability produces an outbound message
  Then the data provider is called exactly once for this dispatch
  And both `initial_state` and `body` are derived from that single fetch result
  And the `body` string reports the same temperature value that appears in `initial_state.temp_c`

Scenario: Single capability invocation produces body and initial_state from one shared data source
  Test:
    Filter: test_capability_invocation_produces_paired_body_and_state
  Given a `weather_guidance` capability instance with slots `(location = "Beijing", focus = "clothing")`
  And the weather data provider is instrumented to record each call
  When the capability constructs an outbound message
  Then `build_state` and `build_body` are both invoked exactly once with the same `data` argument
  And the produced outbound message carries `initial_state` from `build_state` and `body` from `build_body`
  And no `body` value is constructed by any other code path in the pipeline (no channel-specific text generation)
  And the location, temperature, and focus-specific guidance present in `initial_state` also appear in `body`

Scenario: focus is optional and omitted when using the capability default
  Test:
    Filter: test_focus_omitted_when_default
  Given a user message "巴黎今天天气如何"
  When the resolver returns `focus = "overview"` which equals `weather_guidance.default_focus`
  Then the dispatcher MAY omit the `focus` key from `initial_state` entirely
  And the resulting `org.octos.app.version` equals the existing weather L1 version
  And no version bump is emitted

Scenario: Unknown focus from producer is rejected before emit, not after render
  Test:
    Filter: test_unknown_focus_rejected_before_emit
  Given the capability registry declares `supported_focuses = ["overview", "clothing", "umbrella", "outdoor"]`
  And the resolver returns `focus = "gardening"` with high confidence
  When the dispatcher validates the result
  Then the dispatcher rejects `focus = "gardening"` before calling `build_state`
  And the dispatcher either falls back to `default_focus` (with warning) or degrades to plain text
  And no `org.octos.app` envelope is emitted with `focus = "gardening"`

Scenario: Adding optional focus slot does not bump app_version
  Test:
    Filter: test_optional_focus_does_not_bump_version
  Given the weather L1 consumer currently advertises `supported_version() = 2`
  And the consumer also accepts version `1` via `supports_version(v) = matches!(v, 1 | 2)`
  When `weather_guidance` starts emitting `initial_state.focus = "clothing"`
  Then the emitted `org.octos.app.version` remains `2`
  And a consumer build that does not recognize `focus` renders the default focus layout
  And a warning is logged by the consumer, not an error

Scenario: Required slot missing triggers resolver re-ask path, not fabrication
  Test:
    Filter: test_missing_slot_triggers_reask
  Given a user message "穿什么好" with no location in context
  And `weather_guidance.required_slots` marks `location` as required
  When the resolver attempts to extract slots
  Then `slots.location` is either missing or flagged as unresolved
  And the dispatcher does NOT invent a default location
  And the outbound reply asks the user to specify a location in plain text
  And no `org.octos.app` envelope is emitted

Scenario: Weather guidance regression fixture meets the robustness gate
  Test:
    Filter: test_weather_guidance_regression_fixture_passes
  Given the fixture contains at least 80 phrasings covering four focuses in English and Chinese
  And the fixture also contains negative examples outside the `weather_guidance` domain
  When the resolver runs over the full fixture
  Then top-1 accuracy on in-domain examples is at least 90%
  And the false-positive rate on negative examples is 0% (every negative is rejected)
  And the fixture and its expected outputs are committed under `crates/octos-cli/tests/resolver_fixtures/`

Scenario: Robustness gate not met keeps weather_guidance disabled
  Test:
    Filter: test_gate_failure_keeps_capability_disabled
  Given the resolver accuracy on the fixture drops below 90%
  When the OctOS binary starts
  Then the `weather_guidance` capability is registered but marked disabled
  And incoming weather-related messages fall back to the legacy `show_weather_card` tool path
  And a startup log warns that the capability is gated off

Scenario: Capability produces no org.octos.actions and no button bridging
  Test:
    Filter: test_capability_does_not_emit_actions
  Given `weather_guidance` dispatches a reply
  When the outbound message is constructed
  Then the message's Matrix content contains `org.octos.app` only
  And the message's Matrix content does NOT contain `org.octos.actions`
  And any future button integration goes through the existing Phase 4c path, not through the capability

Scenario: Recorded resolver fixture replays deterministically through the dispatcher
  Test:
    Filter: test_dispatcher_snapshot_replays_deterministically
  Given a recorded resolver fixture file containing `(user_message, context_summary)` and the recorded LLM JSON response
  When the test harness replays the recorded LLM response through the dispatcher (without calling the live LLM)
  Then both inputs and the recorded structured output round-trip through JSON without loss
  And the dispatcher's decision (resolved `capability_id`, `focus`, `slots`, dispatch outcome) is identical across runs
  And the live LLM is NOT invoked during this replay
  And LLM-side non-determinism is exercised separately by the regression fixture scenario, not asserted here

=== Codebase Context ===

(no matching files found)

=== Task Sketch ===

Group 1 (order 1):
  Scenarios:
    - Resolver emits structured JSON, not free tool choice
    - Hybrid dispatch routes to capability only when registry contains the id
    - Low-confidence result falls back to plain text, no card emitted
    - Resolver LLM timeout degrades to plain text reply
    - Resolver returns invalid JSON and the producer degrades gracefully
    - build_state and build_body are produced from the same data fetch
    - Single capability invocation produces body and initial_state from one shared data source
    - focus is optional and omitted when using the capability default
    - Unknown focus from producer is rejected before emit, not after render
    - Adding optional focus slot does not bump app_version
    - Required slot missing triggers resolver re-ask path, not fabrication
    - Weather guidance regression fixture meets the robustness gate
    - Robustness gate not met keeps weather_guidance disabled
    - Capability produces no org.octos.actions and no button bridging
    - Recorded resolver fixture replays deterministically through the dispatcher
  Boundary paths:
    - crates/octos-cli/tests/resolver_fixtures/** （new，测试集 fixture）
  Test selectors:
    - test_resolver_output_is_structured_json
    - test_dispatch_routes_only_known_capability
    - test_low_confidence_falls_back_to_text
    - test_resolver_timeout_degrades_gracefully
    - test_resolver_invalid_json_degrades
    - test_state_and_body_share_one_data_fetch
    - test_capability_invocation_produces_paired_body_and_state
    - test_focus_omitted_when_default
    - test_unknown_focus_rejected_before_emit
    - test_optional_focus_does_not_bump_version
    - test_missing_slot_triggers_reask
    - test_weather_guidance_regression_fixture_passes
    - test_gate_failure_keeps_capability_disabled
    - test_capability_does_not_emit_actions
    - test_dispatcher_snapshot_replays_deterministically

=== Warnings ===

  - Allowed Changes path not found: crates/octos-agent/src/capabilities/** （new 模块） (resolved to ./crates/octos-agent/src/capabilities)
  - Allowed Changes path not found: crates/octos-agent/src/capabilities/weather_guidance.rs （new） (resolved to ./crates/octos-agent/src/capabilities/weather_guidance.rs （new）)
  - Allowed Changes path not found: crates/octos-agent/src/tools/send_app_card.rs (resolved to ./crates/octos-agent/src/tools/send_app_card.rs)
  - Allowed Changes path not found: crates/octos-agent/src/tools/show_weather_card.rs （**单一路径**：改成 (resolved to ./crates/octos-agent/src/tools)
  - Allowed Changes path not found: crates/octos-agent/src/resolver/** （new 模块，resolver + fixture loader） (resolved to ./crates/octos-agent/src/resolver)
  - Allowed Changes path not found: crates/octos-cli/src/session_actor.rs (resolved to ./crates/octos-cli/src/session_actor.rs)
  - Allowed Changes path not found: crates/octos-cli/src/prompts/gateway_default.txt (resolved to ./crates/octos-cli/src/prompts/gateway_default.txt)
  - Allowed Changes path not found: crates/octos-cli/tests/resolver_fixtures/** （new，测试集 fixture） (resolved to ./crates/octos-cli/tests/resolver_fixtures)
  - Allowed Changes path not found: specs/task-agent-to-app-l1-weather-card.spec.md （addendum：承认 optional (resolved to ./specs/task-agent-to-app-l1-weather-card.spec.md （addendum：承认 optional)
