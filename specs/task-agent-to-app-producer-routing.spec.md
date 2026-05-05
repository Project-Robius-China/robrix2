spec: task
name: "Agent-to-App Producer Routing — OctOS-side Intent-to-Capability Contract"
inherits: project
tags: [bot, agent-to-app, producer, octos, capability, resolver]
depends: [task-agent-to-app-system, task-agent-to-app-l1-weather-card]
estimate: 3d
---

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

## Decisions

### Resolver 实现模式（硬约束）

- **禁止纯 prompt 路由**：不允许让 LLM 通过"自由 tool-calling"直接决定
  `show_weather_card` vs 普通文本回复。判断点必须显式化、可测试、可回放。
- **Hybrid 模式必选**：resolver 分两步：
  1. **Intent classification + slot extraction**（LLM，structured output）：
     LLM 只输出 `{capability_id, focus, slots: {...}, confidence}` 这种 JSON
     schema，**不**直接选 tool。使用 OpenAI/Anthropic 的 structured-output /
     tool-use "单一固定 schema" 模式，而不是多工具自由选择。
  2. **Deterministic dispatch**（Rust 代码）：拿到 JSON 结果后，由 Rust 代码
     查 capability registry、校验 focus 合法、校验 slots 齐全、拒绝
     `confidence` 低于阈值的结果，最后调用 capability 的 producer。
- **confidence 阈值**：`confidence < 0.6` 的结果视为 "未命中 capability"，
  走 plain text 回复路径。阈值作为 capability-level 配置项，允许每个
  capability 单独调整。
- **失败模式（分类，addendum 2026-04-24，revised）**：resolver 失败原因
  **必须分类**，不同类别走不同降级路径。技术错误必须在日志中显式可观测，
  但不应直接阻断用户获得普通 LLM/tool 回复。分三类：

  | 失败类别 | 触发条件 | 允许的降级路径 |
  |---|---|---|
  | **SemanticNoMatch** | resolver 明确返回 `capability_id = null` / 低 confidence / 所有 capability 都未命中 / 明确判定 TextOnly / UnsupportedCapability | 走 legacy tool-calling + plain text（现有行为保留） |
  | **ResolverProviderError** | LLM provider 网络错误 / HTTP 非 2xx / SSL / 连接超时 / TLS 失败 / rate limit | retry resolver 一次（幂等）；仍失败则记录 `fallback_reason=resolver_provider_error` + `app_dispatch_failed=true`，然后 fall through 到 legacy LLM/tool pipeline。只有 legacy pipeline 也失败时，才返回用户可见的短错误 |
  | **ResolverSchemaError** | LLM 返回了但 JSON 无效 / 违反 schema / 超时但无网络错误 | 与 `ResolverProviderError` 同处理：retry 一次 + 结构化日志 + legacy fallthrough |

  **为什么允许 fall through 到 legacy**：resolver 是 app-dispatch 的前置判定器，
  不是整个 agent 的可用性 gate。resolver 技术失败说明"本轮无法可靠产 app
  envelope"，不等于"无法回复用户"。必须用结构化日志暴露降级原因，避免开发侧
  误判；但用户侧应继续得到普通 LLM/tool 回复。禁止的是"把 legacy 文本伪装成
  已成功产出的 app 卡片"，不是禁止 legacy 本身。

- **日志分类（强制 tracing 字段）**：dispatcher 在每条 fallback 路径产出一条
  结构化日志，`fallback_reason` 字段必须是以下之一：
  `semantic_no_match` / `low_confidence` / `resolver_provider_error` /
  `resolver_schema_error` / `app_dispatch_failed` / `unknown_capability` /
  `missing_slot`。**禁止**把所有 fallback 都记成泛化"degraded to plain text"。
- **执行顺序（resolver vs 普通 tool-calling）**：每条用户消息进入回复 pipeline
  时，**先**调用 resolver。当 resolver 输出 `capability_id != null` 且通过
  全部校验（registry 命中、focus 合法、slots 齐全、confidence ≥ 阈值）时，
  dispatch 给 capability，**不再**进入普通 LLM tool-calling 路径；否则
  （未命中 / 低 confidence / resolver 失败）走现有的普通 tool-calling 路径，
  LLM 可自由选择搜索、计算、提醒等非 app 类工具。**Resolver 不替代普通
  tool-calling，只占据 capability/app 路由这一段**。
- **`confidence` 语义（v1）**：采用 **LLM 自报 confidence**——LLM 在 JSON
  schema 中作为字段返回。已知不可靠，所以 `min_confidence` 阈值的角色是
  **粗筛工具**，不是统计置信度。logprobs / 模型校准 / 独立 hallucination
  detector 等更精细方法属于 Out of Scope，留给后续迭代。
- **可回放**：resolver 的输入（用户消息 + 对话上下文摘要）和输出（JSON）
  必须可序列化为 fixture。**回放只断言 dispatcher 行为的确定性**——给定
  同一条录制好的 LLM 响应，dispatcher 决策必须可复现；LLM 自身的非确定性
  通过 regression fixture 的统计门槛覆盖（见 Robustness gate），**不**作为
  bit-exact 断言。

### Capability Registry

- **注册表形状**：OctOS 侧新增 `CapabilityRegistry`，是一个
  `HashMap<&'static str, Arc<dyn Capability>>`，key 是 `capability_id`。
- **Capability trait 必选方法**：
  - `id() -> &'static str` — e.g. `"weather_guidance"`
  - `app_type() -> &'static str` — e.g. `"weather"`（与 Robrix consumer
    注册表的 type key 一一对应）
  - `app_version() -> u32` — 发出的 `org.octos.app.version`
  - `supported_focuses() -> &'static [&'static str]` — finite enum，空数组
    表示不支持 focus（只有 default rendering）
  - `required_slots() -> SlotSchema` — 声明必填/可选 slot 及类型
  - `min_confidence() -> f32` — 默认 0.6
  - `build_state(slots, data) -> Result<JsonValue, CapabilityError>` —
    确定性函数，输出 `initial_state`
  - `build_body(slots, data, language) -> String` — 确定性函数，输出 Matrix
    `body` 字段的 plain-text 版本
- **共同数据源**：`build_state` 与 `build_body` **必须共享同一个 `data`
  输入**（同一次数据拉取的结果）。不得出现 body 用一套数据、state 用另一套。
- **一对一映射**：一个 `capability_id` 映射到**恰好一个** `app_type`；反之
  一个 `app_type` 可以被多个 capability 共享（例如未来 `commute_guidance`
  与 `weather_guidance` 都可能写入 `weather` 卡）。
- **优先级**：当 resolver 输出的 `capability_id` 存在于 registry 时直接
  分派；LLM **不允许**一次返回多个 candidate。并列命中需要在 resolver 侧
  用 tie-breaker（在 schema 中只返回 top-1）解决。

### focus 字段兼容规则（schema 扩展，不 bump version）

- **`focus` 是 `initial_state` 的可选字段**：不在 schema 里时等价于 capability
  的 `default_focus`（每个 capability 自行声明）。
- **不 bump `app_version`**：向 `initial_state` 新增 optional 字段 **不** 触发
  version 增。旧 consumer 遇到未知字段必须静默忽略，这条已经在 master spec
  §协议 envelope 的不变量里（本 spec 不重写，只要求 consumer 条目在 PR 检查
  中确认行为）。
- **bump 触发条件（明确）**：以下任一情况必须 bump `app_version`：
  - 删除 `initial_state` 中任何已有字段
  - 修改已有字段的类型或值域语义
  - 把可选字段改为必填
  - 变更 focus 枚举中已有值的渲染语义
  - 新增 focus 枚举值并**要求** consumer 必须识别（反之新增可 fall back 到
    default 的 focus 值，不 bump）
- **unknown focus fallback**：consumer 收到未知 `focus` 值时，必须渲染
  `default_focus` 的 layout，并记录 warning。Producer 不得发送当前 version
  consumer 无法 fall back 的 focus 值。

### Fallback Body 同源性（跨通道一致性）

- **每个 capability 必须同时产出 `initial_state` 和 `body`**：`send_app_card`
  tool 的 wrapper 必须要求 `body` 和 `initial_state` 来自同一个 capability
  实例的同一次 `(build_state, build_body)` 调用。
- **禁止"每通道一套逻辑"**：Telegram / CLI / 任何非 app-capable 通道显示的
  文本（`body`）不得单独由 LLM 现场生成，必须来自 capability 的
  `build_body`。
- **内容不矛盾原则**：`build_state` 和 `build_body` 的输出在事实数据上必须
  一致。允许的差异仅限：
  - 展现详略（`body` 更精简，`state` 可包含更多字段）
  - 语言本地化（两者都按传入的 `language` 参数本地化）
  - 格式化（`body` 是 Markdown/plain，`state` 是结构化 JSON）
- **fallback 体现**：不支持 app envelope 的通道只渲染 `body`；支持的通道
  两者都收到，渲染 card，`body` 作为无障碍 / accessibility 备选文本不显示
  在 timeline 可见区域（具体由 consumer spec 约束）。

### weather_guidance 作为最小落地集合

- **capability_id**: `"weather_guidance"`
- **app_type**: `"weather"`（映射到 `task-agent-to-app-l1-weather-card` 注册
  的类型）
- **app_version**: 与 weather L1 当前 **代码实现** 保持一致——consumer
  目前 `supported_version() = 2` 且 `supports_version(v) = matches!(v, 1 | 2)`
  （`src/home/app_registry/weather.rs:36-42`）。Producer 默认发 `version = 2`；
  `focus` 作为 optional 字段新增不 bump version
- **supported_focuses**: `["overview", "clothing", "umbrella", "outdoor"]`
- **default_focus**: `"overview"`
- **required_slots**: `location: string`, `time_scope: enum{"today","tomorrow"}`（默认 `today`），
  `language: enum{"en","zh-CN"}`（从会话语言默认，用户显式 override 生效）
- **min_confidence**: `0.6`
- **数据源**：复用现有 `show_weather_card` 的天气数据提供者，本 spec 不改
  数据拉取逻辑，只把它从 tool 实现里抽出来给 capability 调用。
- **focus → state diff**：`focus` 只影响 `initial_state` 里 `focus` 字段本身
  和 `body` 的文案重点（例如 clothing focus 的 body 以穿衣建议为首句）；
  **不**新增必填数据字段，保持向后兼容。
- **跨 spec patch（必带）**：weather L1 spec 当前没有声明 `language` /
  `focus` 这两个 optional 字段，但 Robrix consumer 已经实现了
  `initial_state.language` 覆盖（`src/home/app_registry/weather.rs:47, 292`）。
  本 spec 同步给 weather L1 spec 加 addendum，承认 `language` 与 `focus`
  为 optional schema 扩展，不 bump version。weather v2 引入的其他结构化
  字段（`high_c/low_c`、`morning/noon/night`、`uv_index_max` 等）的 spec
  同步**不**在本 spec 范围，归独立的 weather L1 v2 doc-sync 任务。

### Robustness gate（上线前必过）

- **测试集规模**：为 `weather_guidance` 的 4 个 focus 各准备 ≥ 10 条中文
  + ≥ 10 条英文 diverse phrasings（共 ≥ 80 条），存为 fixture。
- **通过门槛**：resolver 在此测试集上的 top-1 accuracy ≥ 90%，并且
  "focus 反例"（如 "今天是几号" 这种不属于 weather_guidance 的 query）
  必须 100% 拒绝（confidence < 0.6 或输出 `capability_id = null`）。
- **Regression 锁**：测试集 fixture 进仓库，CI 上任何 resolver prompt 或
  model 变更都必须跑这套门槛。
- **不达标不上线**：fail 了就不能打开 `weather_guidance` capability，仍走
  旧的 `show_weather_card` tool 直到达标。

## Boundaries

### Allowed Changes

OctOS 仓（`/Users/zhangalex/Work/Projects/FW/octos`）内：

- crates/octos-agent/src/capabilities/** （new 模块）
- crates/octos-agent/src/capabilities/weather_guidance.rs （new）
- crates/octos-agent/src/tools/send_app_card.rs
- crates/octos-agent/src/tools/show_weather_card.rs （**单一路径**：改成
  thin adapter，把所有调用直接转发给 `weather_guidance` capability。tool
  入口本身保留至 robustness gate 达标，**达标后**的整体删除归独立的
  "show_weather_card removal" 清理任务，**不**在本 spec 范围）
- crates/octos-agent/src/resolver/** （new 模块，resolver + fixture loader）
- crates/octos-cli/src/session_actor.rs
- crates/octos-cli/src/prompts/gateway_default.txt
- crates/octos-cli/tests/resolver_fixtures/** （new，测试集 fixture）

Robrix 仓（本仓）内：

- specs/task-agent-to-app-producer-routing.spec.md
- specs/task-agent-to-app-l1-weather-card.spec.md （addendum：承认 optional
  `language` / `focus` 字段；其余 v2 schema 同步留给独立任务）

### Forbidden

- 不要修改 `org.octos.app` envelope 协议（`type`/`version`/`initial_state` 三
  字段的语义和必填性）——master spec 明确禁止。
- 不要修改 Robrix 任何 `src/home/app_registry/**` 代码——本 spec 是
  producer-only。
- 不要把 "pure prompt tool-calling" 作为 capability 路由机制（即 LLM 直接
  从多个 tool 里挑）——必须先经过 structured-output intent classifier。
- 不要允许 capability 直接写 `initial_state` 而绕过 `build_state`——`body`
  和 `state` 必须来自同一 capability 调用。
- 不要在 Matrix / Telegram / CLI 三个通道各写一套 body 生成逻辑——body 必须
  来自 capability 的 `build_body`。
- 不要为 `focus` 扩展字段 bump `org.octos.app.version`——只要消费者能安全
  忽略该字段（已经由 master spec 的向前兼容规则保证）。
- 不要在 robustness gate 未达标前把 `weather_guidance` 当默认路径上线。
- 不要新增 cargo 依赖。
- 不要让 resolver 在 LLM 超时 / 无效 JSON 时 panic——必须 retry 一次并记录
  分类后的 `fallback_reason`。
- **不要在 `ResolverProviderError` / `ResolverSchemaError` 发生时直接向用户
  暴露 resolver 实现错误**。provider / schema 技术失败必须 retry 一次，仍
  失败则记录 `fallback_reason` + `app_dispatch_failed=true`，再 fall through 到
  legacy LLM/tool pipeline。只有 legacy pipeline 也失败时，才返回用户可见的
  短错误 plain-text。
- 不要把不同类的 fallback 都记成泛化"degraded to plain text" 日志 —— 必须
  按 `fallback_reason` 字段分类（见 §Resolver 实现模式 失败模式）。
- 不要让 resolver 输出 multi-candidate（本 spec 只支持 top-1 分派）。

## Out of Scope

- Capability 自动发现 / 动态注册（所有 capability 编译进 OctOS）
- Capability 组合（一次回复触发多个 capability，例如天气 + 日程）
- Resolver 模型训练 / 微调（本 spec 只规定接口契约与 regression 测试集）
- 非文本 slot 抽取（图片、位置共享等）
- 多轮对话中 capability 的状态继承（v1 每次回复独立 resolve）
- Telegram / CLI 通道具体 UI 行为（本 spec 只保证 `body` 同源，不规定渲染）
- Weather 以外任何 capability（`news_guidance` / `calendar_guidance` 等走
  独立子 spec）
- 把 `show_weather_card` 从代码库中彻底删除——本 spec 只把它改成 thin
  adapter；达标后的整体删除归独立的 "show_weather_card removal" 清理任务
- `org.octos.actions` 按钮与 capability routing 的组合语义（按钮仍走 Phase
  4c 路径，本 spec 不涉及）

## Completion Criteria

Scenario: Resolver emits structured JSON, not free tool choice
  Test: test_resolver_output_is_structured_json
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
  Test: test_dispatch_routes_only_known_capability
  Given the registry contains `"weather_guidance"` but not `"fitness_guidance"`
  When the resolver returns `capability_id = "fitness_guidance"` with `confidence = 0.95`
  Then the dispatcher does NOT invoke any capability
  And the reply falls back to a plain text response
  And a warning is logged naming the unknown `capability_id`

Scenario: Low-confidence result falls back to plain text, no card emitted
  Test: test_low_confidence_falls_back_to_text
  Given a user message "告诉我北京"
  And the resolver returns `capability_id = "weather_guidance"` with `confidence = 0.45`
  And `weather_guidance.min_confidence() = 0.6`
  When the dispatcher evaluates the result
  Then no `org.octos.app` envelope is emitted
  And the outbound message is a plain-text reply only
  And a debug log records the rejected low-confidence dispatch

Scenario: Resolver LLM timeout retries once then falls through to legacy
  Test: test_resolver_timeout_retries_then_falls_through_to_legacy
  Level: integration
  Targets: ResolverProviderError classification, retry-once invariant, legacy-fallthrough
  Given the resolver LLM call times out after the configured deadline
  When the dispatcher handles the timeout
  Then no panic is raised
  And the dispatcher retries the resolver exactly ONCE (idempotent)
  And if the retry also times out, dispatcher returns `FallThrough`
  And the legacy LLM/tool pipeline is allowed to produce the user-visible reply
  And a structured log records `fallback_reason = "resolver_provider_error"` AND `app_dispatch_failed = true`

Scenario: Resolver returns invalid JSON and the producer falls through to legacy
  Test: test_resolver_invalid_json_falls_through_to_legacy
  Given the resolver LLM returns a malformed JSON string
  When the dispatcher attempts to parse the result
  Then parsing fails and the dispatcher retries the resolver exactly ONCE
  And if the retry also returns invalid JSON, dispatcher returns `FallThrough`
  And the legacy LLM/tool pipeline is allowed to produce the user-visible reply
  And a structured log records `fallback_reason = "resolver_schema_error"` AND `app_dispatch_failed = true`

Scenario: Resolver provider network error falls through with structured observability
  Test: should_retry_once_on_resolver_provider_error_then_fall_through_to_legacy
  Level: integration
  Targets: app-dispatch degradation observability, user-facing continuity
  Given the resolver LLM provider returns a network / connection / HTTP non-2xx / TLS error
  And the dispatcher is configured with `enable_capability_dispatcher = true` (app-first policy active)
  When the dispatcher handles the provider error
  Then the dispatcher retries the resolver exactly ONCE
  And if retry also errors, dispatcher returns `FallThrough`
  And the legacy LLM/tool pipeline is allowed to produce the user-visible reply
  And a structured log records `fallback_reason = "resolver_provider_error"` AND `app_dispatch_failed = true`

Scenario: Semantic no-match still allows legacy tool-calling fallback (intentional)
  Test: test_semantic_no_match_allows_legacy_fallback
  Level: integration
  Targets: semantic vs technical failure classification boundary
  Given the resolver LLM returns a valid JSON with `capability_id = null` (explicit no-match)
  When the dispatcher handles the result
  Then the legacy tool-calling pathway IS invoked (this is NOT a technical failure)
  And the outbound reply may use `deep_search` or other legacy tools as usual
  And a structured log records `fallback_reason = "semantic_no_match"` (distinct from `resolver_provider_error`)

Scenario: Every fallback path produces a classified `fallback_reason` log field
  Test: test_fallback_reason_log_classification
  Level: unit
  Targets: observability guardrail
  Given the set of all possible fallback paths (semantic_no_match / low_confidence / resolver_provider_error / resolver_schema_error / app_dispatch_failed / unknown_capability / missing_slot)
  When each fallback path is triggered by a synthetic fixture
  Then the emitted structured log contains a `fallback_reason` field matching one of those exact enum strings
  And no fallback path emits a log that omits `fallback_reason` or uses a generic string like "degraded to plain text"

Scenario: build_state and build_body are produced from the same data fetch
  Test: test_state_and_body_share_one_data_fetch
  Given the weather data provider is instrumented to count fetch calls
  And a valid user message triggers `weather_guidance`
  When the capability produces an outbound message
  Then the data provider is called exactly once for this dispatch
  And both `initial_state` and `body` are derived from that single fetch result
  And the `body` string reports the same temperature value that appears in `initial_state.temp_c`

Scenario: Single capability invocation produces body and initial_state from one shared data source
  Test: test_capability_invocation_produces_paired_body_and_state
  Level: unit
  Targets: build_state and build_body invariant, paired-output guarantee
  Given a `weather_guidance` capability instance with slots `(location = "Beijing", focus = "clothing")`
  And the weather data provider is instrumented to record each call
  When the capability constructs an outbound message
  Then `build_state` and `build_body` are both invoked exactly once with the same `data` argument
  And the produced outbound message carries `initial_state` from `build_state` and `body` from `build_body`
  And no `body` value is constructed by any other code path in the pipeline (no channel-specific text generation)
  And the location, temperature, and focus-specific guidance present in `initial_state` also appear in `body`

Scenario: focus is optional and omitted when using the capability default
  Test: test_focus_omitted_when_default
  Given a user message "巴黎今天天气如何"
  When the resolver returns `focus = "overview"` which equals `weather_guidance.default_focus`
  Then the dispatcher MAY omit the `focus` key from `initial_state` entirely
  And the resulting `org.octos.app.version` equals the existing weather L1 version
  And no version bump is emitted

Scenario: Unknown focus from producer is rejected before emit, not after render
  Test: test_unknown_focus_rejected_before_emit
  Given the capability registry declares `supported_focuses = ["overview", "clothing", "umbrella", "outdoor"]`
  And the resolver returns `focus = "gardening"` with high confidence
  When the dispatcher validates the result
  Then the dispatcher rejects `focus = "gardening"` before calling `build_state`
  And the dispatcher either falls back to `default_focus` (with warning) or degrades to plain text
  And no `org.octos.app` envelope is emitted with `focus = "gardening"`

Scenario: Adding optional focus slot does not bump app_version
  Test: test_optional_focus_does_not_bump_version
  Given the weather L1 consumer currently advertises `supported_version() = 2`
  And the consumer also accepts version `1` via `supports_version(v) = matches!(v, 1 | 2)`
  When `weather_guidance` starts emitting `initial_state.focus = "clothing"`
  Then the emitted `org.octos.app.version` remains `2`
  And a consumer build that does not recognize `focus` renders the default focus layout
  And a warning is logged by the consumer, not an error

Scenario: Required slot missing triggers resolver re-ask path, not fabrication
  Test: test_missing_slot_triggers_reask
  Level: integration
  Targets: slot extraction, required-slot validation, no-fabrication invariant
  Given a user message "穿什么好" with no location in context
  And `weather_guidance.required_slots` marks `location` as required
  When the resolver attempts to extract slots
  Then `slots.location` is either missing or flagged as unresolved
  And the dispatcher does NOT invent a default location
  And the outbound reply asks the user to specify a location in plain text
  And no `org.octos.app` envelope is emitted

Scenario: Weather guidance regression fixture meets the robustness gate
  Test: test_weather_guidance_regression_fixture_passes
  Level: integration
  Targets: resolver accuracy, regression fixture, robustness gate invariant
  Given the fixture contains at least 80 phrasings covering four focuses in English and Chinese
  And the fixture also contains negative examples outside the `weather_guidance` domain
  When the resolver runs over the full fixture
  Then top-1 accuracy on in-domain examples is at least 90%
  And the false-positive rate on negative examples is 0% (every negative is rejected)
  And the fixture and its expected outputs are committed under `crates/octos-cli/tests/resolver_fixtures/`

Scenario: Robustness gate not met keeps weather_guidance disabled
  Test: test_gate_failure_keeps_capability_disabled
  Level: integration
  Targets: startup gating, legacy fallback path, gate-enforcement invariant
  Given the resolver accuracy on the fixture drops below 90%
  When the OctOS binary starts
  Then the `weather_guidance` capability is registered but marked disabled
  And incoming weather-related messages fall back to the legacy `show_weather_card` tool path
  And a startup log warns that the capability is gated off

Scenario: Capability produces no org.octos.actions and no button bridging
  Test: test_capability_does_not_emit_actions
  Level: integration
  Targets: outbound envelope composition, protocol independence (app vs actions), Phase 4c boundary
  Given `weather_guidance` dispatches a reply
  When the outbound message is constructed
  Then the message's Matrix content contains `org.octos.app` only
  And the message's Matrix content does NOT contain `org.octos.actions`
  And any future button integration goes through the existing Phase 4c path, not through the capability

Scenario: Recorded resolver fixture replays deterministically through the dispatcher
  Test: test_dispatcher_snapshot_replays_deterministically
  Level: integration
  Targets: dispatcher determinism, fixture replay harness, no-live-LLM invariant
  Given a recorded resolver fixture file containing `(user_message, context_summary)` and the recorded LLM JSON response
  When the test harness replays the recorded LLM response through the dispatcher (without calling the live LLM)
  Then both inputs and the recorded structured output round-trip through JSON without loss
  And the dispatcher's decision (resolved `capability_id`, `focus`, `slots`, dispatch outcome) is identical across runs
  And the live LLM is NOT invoked during this replay
  And LLM-side non-determinism is exercised separately by the regression fixture scenario, not asserted here
