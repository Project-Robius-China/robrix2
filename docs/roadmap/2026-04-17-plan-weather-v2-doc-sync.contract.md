=== Contract ===

# Task Contract: Agent-to-App L1 Weather — v2 Schema Documentation Sync

## Intent
weather L1 spec（[`task-agent-to-app-l1-weather-card`](task-agent-to-app-l1-weather-card.spec.md)）
当前只声明 v1 字段集合；但 Robrix consumer 代码已经实现并发布了 v2 schema
（`WeatherFactory::supported_version() = 2`），并且 producer-routing spec
（[`task-agent-to-app-producer-routing`](task-agent-to-app-producer-routing.spec.md)）
把 "v2 是当前正式版本" 作为事实依赖。

本 spec 是一次**纯文档同步**——把已经在
`src/home/app_registry/weather.rs` 实施的 v2 新增字段正式承认进 weather L1
spec，让合同与代码对齐、消除 L1 spec §Out of Scope 里遗留的"v2 字段级
schema 暂未声明"缺口。

**严格边界**：
- **不修改任何代码**（包括不修改已有测试）。代码就是事实来源，本 spec 的
  工作是描述事实，不是重定义事实
- 不新增字段（只承认代码里已有的 5 个 v2 字段）
- 不 bump `version`（已经是 2，这个事实在 L1 spec 已承认）

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
- **`high_c: number`** — 当日最高温摄氏度
- 范围校验复用 `MIN_TEMP_C..=MAX_TEMP_C`（与 `temp_c` 相同）
- 解析逻辑：`parse_optional_temperature(obj, "high_c")`（`weather.rs:57`）
- 超范围 fail closed（与 `temp_c` 行为一致）
- **`low_c: number`** — 当日最低温摄氏度
- 校验与 `high_c` 相同（`weather.rs:58`）
- 不强制 `low_c <= high_c`（consumer 当前未交叉校验，忠实记录为不校验）
- **`uv_index_max: number`** — 最大 UV 指数
- `>= 0`（`parse_optional_nonnegative_number`，`weather.rs:59, 342-362`）
- 负值 fail closed
- **`precipitation_probability_max: number`** — 当日最大降水概率百分比
- 范围 `0..=100`（`parse_optional_percentage_number`，`weather.rs:60-62, 364-373`）
- 超范围 fail closed
- **`periods: array`** — 日内周期天气数组，最多 3 项
- 超过 3 项时代码 `.take(3)` 截断（`weather.rs:475`）——本 spec 忠实记录：
- 每项 object 结构：
- `slot: enum{"morning","noon","night"}` — 必填；未知值 fail closed
- `temp_c: number` — 必填，范围 `MIN_TEMP_C..=MAX_TEMP_C`；超范围 fail closed
- `condition: string` — optional；未知值 fall back 到 `"sunny"`（与
- `precipitation_probability: number` — optional；代码同时接受整数和
- `location`、`temp_c`、`condition`、`feels_like_c`、`humidity`、
- v1 里 `condition` 未知值 **warning log** 的行为保留；v2 新增的 `periods[].condition`
- `raw_matrix_weather_v2_event_renders_guidance_card`（v2 payload 完整
- `v2_guidance_payload_renders_periods_and_blue_cloudy_theme`（periods
- `rendered_weather_splash_eval_parses_in_makepad_vm`（生成的 Splash DSL

## Boundaries
Allowed changes:
- specs/task-agent-to-app-l1-weather-card.spec.md
- specs/task-agent-to-app-l1-weather-v2-doc-sync.spec.md
Forbidden:
- **不要**修改 `src/home/app_registry/weather.rs` 或任何 Rust 代码
- **不要**修改 `cargo test` 能跑到的任何测试名——本 spec 以代码为事实来源
- **不要**在 L1 spec 里引入未在代码中实现的 v2 字段（例如 `air_quality_index`、
- **不要**bump `app_version`（已经是 2；本 spec 不改版本）
- **不要**删除 L1 spec 的 v1 字段定义（`periods` 之类是**新增**，不是
- **不要**试图"统一" `condition` 未知值 warning log 行为的差异——代码
- **不要**把此处承认的 v2 字段放回 L1 §Out of Scope
- **不要**新增 cargo 依赖
Out of scope:
- v2 之后的下一版 schema 演进（归新的 weather L1 v3 子 spec）
- 渲染层变更（布局、颜色、字号等——归 L1 v2 渲染细化 spec，本 spec 不
- `periods` 超过 3 项时补全 warning log 的代码修复（属于行为变更，归新任务）
- `periods[].condition` 未知值补 warning log 对齐 v1 行为的代码修复
- `low_c > high_c` 的交叉校验（代码当前不校验，本 spec 不改）
- weather producer 侧 resolver / capability 的改动（归 producer-routing spec）
- 非 weather 类型的其他 app schema
- L1 spec 的重构（改章节组织、换语言等）

## Completion Criteria
Scenario: L1 spec optional fields list includes all 5 v2 fields
  Test:
    Filter: test_l1_spec_declares_all_v2_optional_fields
  Given the edited `specs/task-agent-to-app-l1-weather-card.spec.md`
  When the §Weather type JSON schema section is parsed
  Then the optional fields list contains entries for `high_c`, `low_c`, `uv_index_max`, `precipitation_probability_max`, and `periods`
  And each entry declares the field's JSON type, validation range, and failure mode (fail closed vs silent fallback)
  And the `periods` entry declares the 3-item truncation behavior as "silent truncate, no warning"

Scenario: L1 spec Out of Scope no longer lists v2 field-level schema as a gap
  Test:
    Filter: test_l1_spec_out_of_scope_removes_v2_field_gap
  Given the edited `specs/task-agent-to-app-l1-weather-card.spec.md`
  When the §Out of Scope section is parsed
  Then the section does NOT contain the phrase "v2 已在代码实现但本 spec 暂未承认"
  And the section does NOT mention `high_c`, `low_c`, `morning`, `noon`, `night`, `uv_index_max`, or `precipitation_probability_max` as gaps
  And the section still lists genuinely out-of-scope items (refresh button, multi-location, hourly forecast, AQI, etc.)

Scenario: L1 spec Schema 扩展 addendum's "未对齐声明" is updated or removed
  Test:
    Filter: test_l1_addendum_no_longer_disclaims_v2_alignment
  Given the edited `specs/task-agent-to-app-l1-weather-card.spec.md`
  When the §Schema 扩展 addendum section is parsed
  Then the section either does NOT contain a "未对齐声明" paragraph at all
  And the section still clearly records that `language` and `focus` are acknowledged here

Scenario: L1 spec periods entry declares the per-entry sub-schema
  Test:
    Filter: test_l1_spec_periods_entry_defines_subschema
  Given the edited `specs/task-agent-to-app-l1-weather-card.spec.md`
  When the `periods` field entry in §Weather type JSON schema is parsed
  Then the entry declares the array element as an object with keys `slot`, `temp_c`, optional `condition`, optional `precipitation_probability`
  And the `slot` key is typed as enum `{"morning","noon","night"}` with fail-closed on unknown
  And the `temp_c` key is required and constrained to the same range as root `temp_c`
  And the optional `condition` key documents the silent-sunny fallback AND the behavioral difference (no warning log, unlike root `condition`)
  And the optional `precipitation_probability` key is documented as number `0..=100` where floats are rounded to integer before the range check (per `weather.rs:507-515`), with fail-closed on out-of-range or non-numeric types

Scenario: L1 spec references already-implemented tests, no new tests invented
  Test:
    Filter: test_l1_spec_test_selectors_match_existing_rust_tests
  Given the edited `specs/task-agent-to-app-l1-weather-card.spec.md`
  When every `Test:` selector in a v2-related scenario is extracted
  Then each selector matches an existing Rust test function name in the repo
  And no new test names are introduced that are not already present in the codebase
  And running `cargo test <selector_name>` succeeds for each referenced test

Scenario: No Rust source files are modified by this doc-sync task
  Test:
    Filter: test_doc_sync_does_not_touch_rust_sources
  Given this task's Allowed Changes list contains only the two spec files under `specs/`
  When the set of files changed by this task's commits is computed (scoped to this task's change set, not the entire branch — this supports stacked branches where dependency tasks contribute their own commits)
  Then every path in that task-scoped change set is a member of Allowed Changes
  And no path under `src/`, under any `tests/` directory, or any `Cargo.toml` / `Cargo.lock` appears in that set
  And the BoundariesVerifier returns no out-of-scope modifications for this task
  And this check does NOT use `git diff main...HEAD`, which conflates dependency commits with this task's own changes

Scenario: Lint quality stays at 100% after the edit
  Test:
    Filter: test_lint_quality_unchanged_after_doc_sync
  Given the edited `specs/task-agent-to-app-l1-weather-card.spec.md`
  When `agent-spec lint specs/task-agent-to-app-l1-weather-card.spec.md --min-score 0.7` is run
  Then the reported `Quality:` line is `100%`
  And no new `determinism` or `testability` warnings are introduced relative to the pre-edit state
  And the exit code is 0

Scenario: Rejecting a fabricated field that is not present in the Rust code
  Test:
    Filter: test_reject_fabricated_v2_field
  Given a reviewer proposes adding `sunrise: string` as a new optional field to the L1 spec under the v2 acknowledgement
  And `grep "sunrise" src/home/app_registry/weather.rs` returns no results
  When this spec's Forbidden list is consulted
  Then the proposal is rejected because it introduces a field not present in the code
  And the reviewer is directed to either a separate "v3 schema" task or to first land the field in code before documenting it

Scenario: Cross-spec invariant — producer-routing continues to compile against L1 v2 after edit
  Test:
    Filter: test_producer_routing_references_stay_valid
  Given producer-routing spec declares `app_version = 2` as the value derived from weather L1
  When L1 spec is edited by this task
  Then the L1 spec still declares `supported_version() = 2` and compatibility with `1 | 2`
  And producer-routing spec's references to the L1 version remain valid without edit
  And the `depends:` frontmatter on producer-routing spec does not need updating

=== Codebase Context ===

(no matching files found)

=== Task Sketch ===

Group 1 (order 1):
  Scenarios:
    - L1 spec optional fields list includes all 5 v2 fields
    - L1 spec Out of Scope no longer lists v2 field-level schema as a gap
    - L1 spec Schema 扩展 addendum's "未对齐声明" is updated or removed
    - L1 spec periods entry declares the per-entry sub-schema
    - L1 spec references already-implemented tests, no new tests invented
    - No Rust source files are modified by this doc-sync task
    - Lint quality stays at 100% after the edit
    - Rejecting a fabricated field that is not present in the Rust code
    - Cross-spec invariant — producer-routing continues to compile against L1 v2 after edit
  Boundary paths:
    - specs/task-agent-to-app-l1-weather-card.spec.md
  Test selectors:
    - test_l1_spec_declares_all_v2_optional_fields
    - test_l1_spec_out_of_scope_removes_v2_field_gap
    - test_l1_addendum_no_longer_disclaims_v2_alignment
    - test_l1_spec_periods_entry_defines_subschema
    - test_l1_spec_test_selectors_match_existing_rust_tests
    - test_doc_sync_does_not_touch_rust_sources
    - test_lint_quality_unchanged_after_doc_sync
    - test_reject_fabricated_v2_field
    - test_producer_routing_references_stay_valid
