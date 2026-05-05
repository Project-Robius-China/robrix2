spec: task
name: "Agent-to-App Template Runtime — host runtime contract (preflight, cache, compatibility, fallback, error shape)"
inherits: project
tags: [bot, agent-to-app, splash, host, runtime, preflight, cache, version, fallback, w5, w7]
depends: [task-agent-to-app-system, task-agent-to-app-splash-host-evolution]
estimate: 3d
---

## Intent

`task-agent-to-app-splash-host-evolution` 建立了 Layer 2 Splash Host 的
**结构契约**：SplashHost trait、widget manifest（W5）、local function
registry（W7）、capability descriptor、静态 `.splash` 模板。它回答的是
**"模板怎么进宿主、怎么渲染"**。

本 spec 补齐 **运行时契约**：**"模板进宿主之后，宿主怎么保证可验证、
可缓存、可回退、可兼容"**。

触发这份 spec 的是 AOSF 治理视角 + A2UI 工程手法的综合要求（2026-04-22
讨论收敛版）。不做大而全的 governance / marketplace / policy engine /
provenance store —— 只解决 Splash Host 近期落地马上会撞到的 4 件事：

1. **Splash preflight validation**：模板进入宿主前的五重静态检查
2. **TemplateHandle cache + version compatibility**（一体化）：何时 cache
   可复用 = 何时版本兼容
3. **Fallback / rollback hierarchy**：template → app version → plain text
4. **Error shape（fail-explainable only）**：失败必须可解释；不做完整
   provenance receipt

**显式不涉及**（留给未来独立 spec）：
- 完整 render receipt / provenance store（审计与存储系统）
- skill-with-UI packaging（capability 携带 UI + schema + fixtures 的打包
  格式）→ `task-agent-to-app-ui-packaging.spec.md`
- 第三方 capability / 模板分发机制
- capability market / marketplace governance
- policy engine / trust workflow beyond `TrustBadge::Builtin`
- dynamic capability hot reload / plugin 加载
- L2 交互 action 的增量 state 语义（归 `task-agent-to-app-l2-actions`）
- L3 stateful mini-app tick 调度（归 `task-agent-to-app-l3-stateful`）

本 spec **不修改** `org.octos.app` envelope 协议；**不修改** `mod.rs::render_app_envelope_to_splash`
的控制流（与 splash-host-evolution 相同的接线承诺）；**不新增** cargo
依赖。

## Decisions

### Splash preflight validation（硬约束）

模板进入 `SplashHost::load_template` 必须通过**五重静态检查**，失败即拒绝
返回 `HostError`；拒绝理由必须结构化到错误变体本身，不允许只在日志里记
文本。

- **Check 1 — Parse**：`.splash` 源文件能被 Makepad 2.0 `script_mod!`
  parser 解析为 AST。语法错误 → `HostError::ParseError { message, line }`。
- **Check 2 — W5 Widget whitelist**：AST 中每个 widget 引用的 type name
  必须在 `widget_manifest::is_template_reachable(name) == true`。违反
  → `HostError::WidgetNotAllowed { name, trust_level }`。
- **Check 3 — W7 Local function whitelist**：AST 中每个 `${fn(...)}`
  字符串插值站点或 `action: { functionCall: { call: "..." } }` 的函数
  名必须在 `local_functions::is_registered(name) == true`。违反 →
  `HostError::LocalFunctionNotAllowed { name }`。
- **Check 4 — Attribution override guard**：模板不得在自身内容区声明
  `capability_id` / `display_name` / `icon` / `trust_badge` 任何一个
  字段 —— 这些由 host chrome 容器注入。违反 →
  `HostError::AttributionFieldInTemplate { field }`。
- **Check 5 — Binding path schema check**：AST 中每个 `$state.path` 绑定
  站点必须对应 capability 的 `(app_type, app_version)` schema 的一个
  合法 JSON Pointer 路径。违反 → `HostError::BindingPathNotInSchema
  { path, app_type, app_version }`（**本 spec 新增** HostError 变体）。

**构建期执行**：本 spec 要求在 **lib 内** 新增一个 `#[cfg(test)]` 模块
（建议位置 `src/home/app_registry/template_preflight_audit.rs`，或并入
`splash_host.rs` 的 `#[cfg(test)] mod tests_preflight_audit`）。该模块
在编译期借助 `include_str!` 汇聚所有模板字节，测试
`test_all_templates_pass_preflight_at_build_time` 遍历每个
`(capability_id, template_id)` 对跑一次 `SplashHost::load_template` +
schema 对齐校验；失败视为 build break（`cargo test --lib` 必须亮红）。

**位置选择理由**：Cargo 语义里 `cargo test --lib` 不跑 `tests/**`
integration test。为让 "preflight 是构建期硬约束" 的承诺真正可执行，
测试必须落在库内 `#[cfg(test)]`；同时 `include_str!` 在编译期绑定所有
模板字节，与 production build 的静态资产装载路径一致，避免 v1
dev vs prod 的哈希来源分叉。

这是 splash-host-evolution spec "cargo check 的 linter" 承诺的可执行
形式。

**禁止**：
- 运行时绕过检查（即允许发现某个 widget 不在 manifest 还继续渲染）
- 静默失败（必须返回具体错误变体而不是 `None`）
- 动态 widget / 函数注册（v1 manifest + registry 是编译期闭集）

### TemplateHandle cache + version compatibility（一体化设计）

**核心原则**：Template cache 的可复用性 = 版本兼容性；两者不可分开
设计。一个 `TemplateHandle` 可复用当且仅当其 cache key 的每个字段都未
改变。

- **Cache key 六元组**：`(app_type, app_version, template_id,
  template_hash, manifest_version, host_version)`
  - `app_type`：envelope `type` 字段
  - `app_version`：envelope `version` 字段
  - `template_id`：capability 选定的模板 id
  - `template_hash`：`.splash` 源文件的内容哈希（v1 用 sha256 前 16 字节
    hex，`u64`-sized）
  - `manifest_version`：capability descriptor 中的 `manifest_version`
  - `host_version`：常量 `HOST_VERSION`
- **Cache 结构**：`OnceLock<DashMap<CacheKey, Arc<TemplateHandle>>>`
  或等价 `RwLock<HashMap>` —— v1 允许任一实现，只要保证并发读安全
  （timeline 同时渲染多个卡）
- **Cache 命中行为**：直接返回 `Arc<TemplateHandle>` 副本，跳过 parse
  + preflight。渲染仍然要做（state binding 每次不同）。
- **Cache miss 行为**：跑完整 preflight 五重检查 → 若通过则 insert 到
  cache → 返回。失败则 error 冒泡给调用方（不 insert error 到 cache
  防止 poison）。
- **Dev 热重载**：`cfg(debug_assertions)` 下，`template_hash` 每次从
  磁盘重算；prod build 用 `include_str!` 编入源码，哈希固定。这对齐
  splash-host-evolution spec 的 dev hot-reload 承诺。
- **Cache 容量**：v1 无上限（built-in capability 数量可数；模板总数 <
  20），future 如需 LRU 另开 spec。

**版本兼容矩阵（non-breaking vs breaking）**：

| 变更类型 | 属性 | 影响字段 | Cache 行为 |
|---|---|---|---|
| 新增 optional state 字段 | Non-breaking | — | 保留 |
| 新增 template_id | Non-breaking | — | 保留（旧 key 不受影响） |
| 新增 W7 函数 | Non-breaking | — | 保留 |
| 新增 widget 到 manifest | Non-breaking | `manifest_version +1` | 保留（但旧 cache key 的 `manifest_version` 不匹配 → 自然 miss + 重建）|
| 新增 required state 字段 | **Breaking** | `app_version +1` | 旧 key 失效 |
| 删除/重命名 widget | **Breaking** | `manifest_version +1` | 旧 key 失效 |
| 删除/重命名 W7 函数 | **Breaking** | `manifest_version +1` | 旧 key 失效 |
| 修改 host chrome 结构语义 | **Breaking** | `HOST_VERSION +1` | 所有 key 失效 |
| 修改 `$state.path` 绑定语义 | **Breaking** | `app_version +1` | 旧 key 失效 |
| `.splash` 源文件内容改 | 视内容 | `template_hash` 自动变 | 单个 key 失效 |

**禁止**：
- 跨版本静默 cache 复用（即改了 host 但不 bump `HOST_VERSION`）
- Template preflight 成功后跳过 cache 存储（浪费）
- Template hash 使用可能碰撞的弱哈希（v1 要求 sha256-truncate，不是 FNV
  / CRC32）

### Fallback / rollback hierarchy（两级 + 强制日志）

所有渲染路径失败必须按下述两级顺序降级，**每一级降级必须记录结构化
日志**，不允许 silent degrade：

1. **Template fallback**（L1）：当 capability 的 preferred template_id
   preflight 或 cache 查询失败时 —— 尝试 capability 声明的 fallback
   template_id（若有）；failure reason 用 `FallbackReason::TemplateFailed
   { capability_id, preferred_template_id, underlying }` 记录。
2. **Plain-text fallback**（L2）：L1 不成功（或该 capability 未声明
   fallback template_id）—— 返回 `None` 给
   `render_app_envelope_to_splash`，timeline 渲染 envelope 同消息的
   Matrix `body` plain text，记录 `FallbackReason::AllTemplatesFailed
   { final_error }`。

**App version mismatch 不属于本 spec 的 fallback 范围**。当 envelope
`version` 当前 consumer 不支持（`supports_version(v) == false`）时，
既有主路径在 `app_registry/mod.rs` 遇到 `AppLookup::VersionMismatch`
直接回 plain text（weather L1 spec 已合同化）。本 spec **不** 引入
"consumer 内部多版本降级" 机制，因为：

- 本 spec 自己锁定"不改 `mod.rs::render_app_envelope_to_splash` 控制流"
- weather L1 spec 已明确将 version mismatch 视为 plain-text 回退而非
  再尝试其他版本
- 加入中间层 app-version 降级需要 `AppFactory::init` 知晓 version 并
  尝试多路径，这是架构级变更，归独立 spec

未来若需要"capability 内部支持多 schema 版本兼容渲染"，归一份新的
`task-agent-to-app-multi-version-rendering.spec.md`（TBD），届时可能
引入 `FallbackReason::VersionDowngraded`；本 spec 不预留该变体。

**强制约束**：
- 每次 fallback 必须产出**一条** `makepad_widgets::log!` 事件（`tracing`
  crate 引入归单独 task），字段包含 `capability_id` / `app_type` /
  `app_version` / `template_id` / `fallback_level` / `fallback_reason`。
- 任何一级 fallback 成功后，**不得再回退到其他级别**（避免振荡）。
- L2 fallback 禁止返回空字符串或占位符 Splash；必须让 timeline 走已有
  plain text 路径。

### Error shape（fail-explainable only）

所有 host-side 错误必须可解释到足以让下游（timeline 日志、将来的审计
系统、或 template-author LLM 的 repair loop）定位问题。

- **FallbackReason enum**：新增于本 spec 定义文件（建议位置
  `splash_host.rs` 或独立 `fallback.rs`）。变体至少覆盖：
  - `TemplateFailed { capability_id, preferred_template_id, underlying: Box<HostError> }`
  - `AllTemplatesFailed { final_error: Box<HostError> }`
  - `HostVersionMismatch { expected_host_version, got }`（诊断性；出现在
    日志里标记 cache key 失效，不用于 fallback 路径分支）

  **不** 引入 `VersionDowngraded` —— app-version 降级不属于本 spec
  范围（见上节 Fallback hierarchy）。
- **Template validation error 结构**：A2UI 风格三元组
  `{ code: &'static str, path: String, message: String }`，从现有
  `HostError::ParseError` / `WidgetNotAllowed` / `LocalFunctionNotAllowed` /
  `AttributionFieldInTemplate` / `BindingPathNotInSchema` 变体派生。
  新增 method `HostError::to_validation_error() -> ValidationError`。
- **No provenance storage**：v1 **不** 定义 `render_receipt` 持久化
  schema；只要求失败侧可解释。成功渲染的 trace 通过现有 `log!` /
  `tracing` 日志路径承担，不引入专门的 receipt store。

**禁止**：
- 使用 `anyhow::Error` / `Box<dyn Error>` 作为 host API 错误类型 —— 必须
  是结构化 enum 以支持 `to_validation_error()`。
- 把失败原因塞进 `Option<String>` 而不是结构化变体。
- 把成功路径的 trace 写入持久化 store —— 只用 log 级别。

## Boundaries

### Allowed Changes

Robrix 仓（`/Users/zhangalex/Work/Projects/FW/robius/robrix2`）：

- `src/home/app_registry/splash_host.rs`（扩展）：
  - 新增 `HostError::BindingPathNotInSchema { path, app_type, app_version }`
  - 新增 `FallbackReason` enum
  - 新增 `HostError::to_validation_error() -> ValidationError`
  - `DefaultSplashHost::load_template` 实装五重 preflight
  - `DefaultSplashHost::render_to_splash` 实装 cache 查询 + 命中复用
- `src/home/app_registry/template_cache.rs`（new）：
  - `CacheKey` struct + `TemplateCache` 结构 + cache miss/hit 逻辑
- `src/home/app_registry/fallback.rs`（new，或合并入 splash_host.rs）：
  - `FallbackReason` enum（如未在 splash_host.rs）
  - `ValidationError` struct
  - Fallback hierarchy 路由辅助函数
- `src/home/app_registry/capability_descriptors.rs`（扩展）：
  - 新增 per-capability `fallback_template_id: Option<&'static str>` 字段
- `src/home/app_registry/mod.rs`（微调）：
  - `render_app_envelope_to_splash` 的 `None` 路径日志格式对齐新
    `FallbackReason` 结构（行为不变）
- Tests:
  - `src/home/app_registry/template_preflight_audit.rs`（new，**库内
    `#[cfg(test)]` 模块**；扫描所有 `.splash` 文件跑 preflight，
    `cargo test --lib` 亮红即 build break。**不** 放 `tests/**`
    integration test 以保持 `cargo test --lib` 承诺有效）
  - 各模块内部 `#[cfg(test)]` 单测扩展

### Forbidden

- **不得**修改 `org.octos.app` envelope 协议（`type + version +
  initial_state` 三字段）—— master spec 锁定
- **不得**修改 `mod.rs::render_app_envelope_to_splash` 的控制流
  （`factory.init → rendered.render(lang)` 路径不变，与 splash-host-evolution
  同承诺）
- **不得**修改 `room_screen.rs` / timeline 挂载
- **不得**引入新的 **transitive** cargo crate（`Cargo.lock` 中不得出现
  新的 crate 名条目）。**允许**在 `Cargo.toml` 中把一个已经存在于
  `Cargo.lock` 的 crate（例如 `sha2`，通过 matrix-sdk 传递引入）提升为
  direct dependency —— 这只是 import 路径变得干净，不会让 build graph
  增长。实装前必须用 `cargo tree --duplicates` 对比 baseline 验证无新
  transitive crate
- **不得**把任何 v1 preflight 检查降级为 runtime-only
- **不得**引入 provenance / render receipt 持久化存储
- **不得**在本 spec 引入 L2 action 的增量 state 语义
- **不得**绕过 fallback hierarchy 直接返回 `None`（每级降级都要记录）

## Out of Scope

- Skill-with-UI packaging（capability 携带 UI + schema + fixtures 打包
  格式）—— `task-agent-to-app-ui-packaging.spec.md`
- 完整 render receipt / provenance storage —— 未来独立 spec
- Third-party / dynamic capability 加载 —— 设计文档 §10 Non-goals
- Capability marketplace / distribution
- Policy engine / fine-grained trust workflow（超出 `TrustBadge::Builtin`
  的范围）
- Template author LLM（Layer 5b 生成式模板）—— 待首个生成式 capability
  独立 spec
- L2 interactive actions + 增量 state 的 `append`/`splice` 操作
- Performance benchmarks（渲染 latency 基线）
- Cache eviction / LRU 策略（v1 无上限，future spec 再考虑）
- Cross-language template variant 选择策略（现有 language slot 机制
  不变）
- `tracing` crate 引入（v1 继续用 `makepad_widgets::log!`；如果要升级到
  `tracing` 归独立 observability spec）

## Completion Criteria

Scenario: Preflight catches template referencing non-whitelisted widget
  Test: test_preflight_rejects_unlisted_widget
  Given the WidgetManifest does NOT contain `EvilWidget`
  And a template `templates/test_capability/broken.splash` references `EvilWidget`
  When `SplashHost::load_template("test_capability", "broken")` is invoked
  Then the call returns `HostError::WidgetNotAllowed { name: "EvilWidget", trust_level: None }`
  And no cache entry is inserted for this key

Scenario: Preflight catches template referencing non-whitelisted local function
  Test: test_preflight_rejects_unlisted_local_function
  Given the LocalFunctionRegistry does NOT contain `exec_shell`
  And a template contains `action: { functionCall: { call: "exec_shell", args: {} } }`
  When `load_template` is invoked
  Then the call returns `HostError::LocalFunctionNotAllowed { name: "exec_shell" }`

Scenario: Preflight catches template attempting attribution override
  Test: test_preflight_rejects_attribution_override
  Given a template sets `capability_id: "impersonator"` inside its content region
  When `load_template` is invoked
  Then the call returns `HostError::AttributionFieldInTemplate { field: "capability_id" }`

Scenario: Preflight catches state binding path not in capability schema
  Test: test_preflight_rejects_binding_path_not_in_schema
  Given a template contains `$state.nonexistent_field`
  And the capability's `(app_type, app_version)` schema does NOT declare that path
  When `load_template` is invoked
  Then the call returns `HostError::BindingPathNotInSchema { path: "/nonexistent_field", app_type, app_version }`

Scenario: All production templates pass preflight at build time
  Test: test_all_templates_pass_preflight_at_build_time
  Level: unit (lib-internal `#[cfg(test)]`, not `tests/**`)
  Given every `.splash` file under `src/home/app_registry/templates/` bundled via `include_str!`
  When the test iterates and calls `SplashHost::load_template` for each `(capability, template_id)` pair
  Then every call returns `Ok(TemplateHandle)`
  And the test fails `cargo test --lib` if any single template fails preflight (build break)

Scenario: Cache hit reuses TemplateHandle without re-parsing
  Test: test_cache_hit_skips_parse
  Given a template has been loaded once, producing a `TemplateHandle` in cache
  When the same `(app_type, app_version, template_id, template_hash, manifest_version, host_version)` is queried again
  Then the second call returns in sub-millisecond time
  And no parse-side instrumentation counter increases

Scenario: Cache miss on template_hash change (source file edited)
  Test: test_cache_miss_on_template_hash_change
  Given a template has been loaded once and cached
  When the `.splash` source content changes (new `template_hash`)
  And `load_template` is called with the new hash
  Then the cache miss triggers a fresh parse + preflight
  And the old entry remains at its key but is unreachable via the new key

Scenario: Cache miss on manifest_version bump (breaking widget change)
  Test: test_cache_miss_on_manifest_version_change
  Given a template `T` was cached at `manifest_version = 1`
  When `CapabilityDescriptor.manifest_version` bumps to `2`
  And `load_template` is called
  Then the previous cache entry is NOT reused
  And a fresh preflight runs

Scenario: Cache miss on HOST_VERSION bump (breaking host contract change)
  Test: test_cache_miss_on_host_version_change
  Given templates cached at `HOST_VERSION = 1`
  When the constant changes to `2` (simulated via test hook)
  Then every cached entry becomes unreachable
  And next `load_template` re-runs preflight

Scenario: Breaking change table is self-consistent
  Test: test_breaking_change_table_self_consistent
  Given the Decisions table listing Non-breaking vs Breaking changes
  When each row is exercised by a synthetic fixture that triggers the change
  Then a Non-breaking change MUST NOT bump `app_version` / `manifest_version` / `HOST_VERSION` at the same time it claims Non-breaking
  And a Breaking change MUST bump exactly one of those three
  (This scenario is a compliance harness for reviewers, not a runtime behavior test)

Scenario: Template-to-template fallback — preferred fails, fallback template_id succeeds
  Test: test_fallback_template_id_succeeds
  Given a capability declares `template_ids = [preferred, fallback_template]`
  And `preferred` preflight fails
  When the host attempts rendering
  Then it uses `fallback_template` and logs `FallbackReason::TemplateFailed { preferred_template_id: "preferred", ... }`
  And the rendered Splash string is produced

Scenario: Plain-text fallback — all templates fail, Matrix body renders
  Test: test_fallback_plain_text
  Given every template path fails (preflight + capability's declared fallback template)
  When `render_app_envelope_to_splash` is invoked
  Then it returns `None`
  And `FallbackReason::AllTemplatesFailed { final_error }` is logged
  And the timeline renders the envelope's Matrix `body` plain text

Scenario: Envelope version mismatch goes straight to plain text, bypasses L1/L2
  Test: test_unsupported_version_bypasses_template_fallback
  Given envelope `version = 99`
  And every registered consumer returns `supports_version(99) = false`
  When `render_app_envelope_to_splash` is invoked
  Then the existing `AppLookup::VersionMismatch` path fires (no template load attempted)
  And the result is plain text — not a `FallbackReason::TemplateFailed` path
  (Version-mismatch is explicitly NOT a template fallback case in this spec)

Scenario: Fallback hierarchy never oscillates
  Test: test_fallback_does_not_oscillate
  Given a synthetic failure pattern where the template-to-template fallback succeeds
  When rendering is invoked
  Then the plain-text fallback path is NEVER entered for that render
  (Invariant check: once a fallback level succeeds, no subsequent level runs)

Scenario: HostError converts to structured ValidationError
  Test: test_host_error_to_validation_error
  Given any variant of `HostError` related to template validation (not cache / fallback infrastructure)
  When `to_validation_error()` is called
  Then it returns a `ValidationError { code, path, message }` triple
  And `code` is a stable `'static str` matching the variant

Scenario: No provenance store is introduced
  Test: test_no_provenance_storage_new_modules
  Level: meta (reviewer check)
  Given the code diff that implements this spec
  When examined for new modules/files related to receipt/provenance
  Then NO new module named like `render_receipt.rs` / `provenance.rs` / `audit.rs` is introduced
  And NO new file persists a per-render record to disk or database
  (Positive trace via log! is acceptable; persistent store is not)

Scenario: No new transitive crate introduced
  Test: test_no_new_cargo_dependencies
  Level: meta
  Given the `Cargo.lock` state before and after this spec's implementation
  When the two lock files are compared
  Then no new crate name appears in the post-state `Cargo.lock`
  And any hashing added by this spec uses a workspace-already-present crypto crate
  (Promoting an already-transitive crate like `sha2` to a direct `Cargo.toml` entry is explicitly allowed — the safety invariant is no growth in the build graph, not "Cargo.toml has zero new lines")
