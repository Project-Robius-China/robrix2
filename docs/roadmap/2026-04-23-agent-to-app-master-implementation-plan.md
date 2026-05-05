# Agent-to-App Master Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `superpowers:subagent-driven-development` (recommended) or `superpowers:executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deliver the full Splash-first agent-to-app path from today's working weather card reference into a validated, cache-aware, reusable host runtime, then prove the shape with a second capability.

**Architecture:** This master plan does not replace the approved subordinate plans. It sequences them into one execution path. Weather remains the first vertical slice; `template-runtime` hardening is inserted before the second capability so that `news` lands on a validated host/runtime instead of a prototype host.

**Tech Stack:** Rust, Makepad 2.0, Splash templates, Matrix `org.octos.app`, Robrix host/runtime modules, OctOS resolver/dispatcher/capability pipeline.

---

## Plan Inputs

This plan orchestrates the already approved design/spec/plan set:

- Design:
  - [agent-to-app-design.md](../design/agent-to-app-design.md)
- Specs:
  - [task-agent-to-app-splash-host-evolution.spec.md](../../specs/task-agent-to-app-splash-host-evolution.spec.md)
  - [task-agent-to-app-template-runtime.spec.md](../../specs/task-agent-to-app-template-runtime.spec.md)
- Existing detailed plans:
  - [2026-04-21-splash-host-evolution-implementation-plan.md](./2026-04-21-splash-host-evolution-implementation-plan.md)
  - [2026-04-22-template-runtime-implementation-plan.md](./2026-04-22-template-runtime-implementation-plan.md)

Those documents remain the authoritative source for per-slice scenario
coverage. This master plan is the **delivery order and checkpoint plan**.

## Current State

### Already done (verified in Robrix worktree 2026-04-24)

- producer-routing baseline exists
- weather v2 guidance cards render in Robrix
- `SplashHost` trait and singleton accessor exist
- `widget_manifest.rs` exists and backs W5 template preflight
- `local_functions.rs` exists and backs W7 template preflight
- W7 formatter/check functions execute during template rendering
- `capability_descriptors.rs` exists and owns v1 chrome/version fields
- weather renders through a static Splash template path in Robrix
- template preflight audit exists
- `TemplateHandle` cache and compatibility key exist
- fallback/error-shape primitives exist
- v1 JSON Pointer `replace` / `remove` state updates exist
- runtime docs/spec cross-references are synchronized
- `news` renders through the same Robrix `SplashHost`
- **P0 safety hardening (2026-04-24)**: `bind_guidance_template` /
  `bind_news_template` moved to `#[cfg(test)]`; `RenderFailure` typed
  fallback; host preflight/bind errors project to `None` → plain-text.
  Release build fails if any production path references the bypass.
- **P1a template source consolidation (2026-04-24)**: 4 `include_str!`
  sites → 1 (`templates::source_for` / `ALL_TEMPLATES` is the single
  source of truth).
- **P2 end-to-end host-rejection tests (2026-04-24)**: 6 dispatcher-level
  tests exercising W5 / attribution / schema rejection with
  leak-probe assertion. `cargo test --lib` now 377 passed.

### Claimed done but not re-audited in 2026-04-24 (OctOS-side)

These items live in the OctOS repo (not this worktree) and were not
re-verified during the 2026-04-24 Robrix-side audit. Treat as OPEN
until separately confirmed:

- OctOS has a thin `news_guidance` capability registered in the dispatcher
- resolver prompt + structural fixtures include `news_guidance`

### Not done

- **E2E not verified**: Robrix weather + news Matrix-flow rendering after
  P0 / P1a / P2 hardening has not been user-exercised.
- **Resolver provider-error fallback (HIGH)**: prior E2E showed
  dispatcher silently falling through to legacy `deep_search` on LLM
  provider error. Addendum in `task-agent-to-app-producer-routing.spec.md`
  (2026-04-22) reclassifies as `ResolverProviderError` requiring
  retry-once + explicit short error. Implementation in OctOS, status
  unknown. Likely cause of "card-looking text" observations.
- **news org.octos.app envelope generation**: even if OctOS news_guidance
  is registered, whether real LLM resolver traffic actually produces
  `type=news, version=1` envelopes is not verified.
- OctOS weather UI-building cleanup is deferred by user instruction
- OctOS weather routing/regression gate is deferred by user instruction
- live LLM resolver gate has not been run for the expanded weather + news fixture set
- `news_guidance` uses deterministic sample data for the reuse proof; production news API/source integration is not implemented
- packaging / skills-with-UI

## Delivery Rules

- Do **not** re-open protocol questions already settled by approved specs.
- Do **not** skip from host prototype work straight to packaging.
- Do **not** start L2/L3 interactive app work before static-card runtime is stable.
- Keep the existing `mod.rs -> factory.init -> rendered.render(app_language)` seam.
- Weather remains the first load-bearing vertical slice; `news` is the first
  reuse proof, not the first experiment.

## File Structure Overview

### Robrix host/runtime files

- `src/home/app_registry/mod.rs`
  - app lookup and render dispatch seam
- `src/home/app_registry/splash_host.rs`
  - host trait + template loading/rendering runtime
- `src/home/app_registry/widget_manifest.rs`
  - W5 widget whitelist
- `src/home/app_registry/local_functions.rs`
  - W7 local function whitelist
- `src/home/app_registry/capability_descriptors.rs`
  - host-owned chrome / version fields
- `src/home/app_registry/template_cache.rs`
  - TemplateHandle cache + compatibility key
- `src/home/app_registry/template_preflight_audit.rs`
  - build-time template audit harness
- `src/home/app_registry/fallback.rs`
  - fallback reason + validation error shape (or merged into `splash_host.rs`)
- `src/home/app_registry/weather.rs`
  - weather consumer + render-side view-model
- `src/home/app_registry/news.rs`
  - second consumer proving zero-Rust-UI
- `src/home/app_registry/templates/**`
  - static Splash templates

### OctOS capability-side files

- `crates/octos-agent/src/capabilities/weather_guidance.rs`
  - weather capability; pending cleanup should remove UI-building code
- `crates/octos-agent/src/capabilities/news_guidance.rs`
  - second capability
- `crates/octos-cli/src/prompts/resolver_default.txt`
  - resolver support for `news_guidance`
- `crates/octos-cli/tests/resolver_fixtures/**`
  - capability routing fixtures

## Execution Order

### Task 1: Finish the weather Splash-first vertical slice

**Source plans:** splash-host-evolution Slice 1, but with runtime work
split out per template-runtime.

**Files:**
- Modify: `src/home/app_registry/weather.rs`
- Create/modify: `src/home/app_registry/templates/weather_guidance/card_standard.splash`
- Modify: `src/home/app_registry/mod.rs` only where the existing seam already permits
- Modify: `octos/crates/octos-agent/src/capabilities/weather_guidance.rs`

- [x] **Step 1: Complete weather static template extraction**

Use the consumer-side view-model path already chosen:
- keep language selection in Robrix
- keep raw state in the envelope
- make the template bind to a render-ready view-model, not raw optional-heavy JSON

Expected result:
- `WeatherRenderedApp::render(app_language)` becomes the only weather host call site
- no new weather-specific widget abstractions are introduced for v1

- [x] **Step 2: Verify the weather template path in Robrix tests**

Run the existing weather-focused tests plus the Makepad eval test and confirm:
- template renders with full data
- template remains renderable with missing optional metrics
- no `$state.*` placeholders leak into output

User-visible Matrix E2E remains a later checkpoint.

- [ ] **Step 3: Delete OctOS weather UI-building code** — deferred

Remove the old `render_guidance_weather` / `out.push_str` UI assembly path from
`weather_guidance.rs` so OctOS produces state/body only. Do not start this until
the user explicitly resumes OctOS agent work.

- [ ] **Step 4: Re-run weather routing/regression checks** — deferred

Re-run the producer-routing / fixture checks that prove weather behavior did not
regress while the rendering side moved to templates.

### Task 2: Implement template-runtime Slice A (preflight)

**Source plan:** template-runtime Slice A

**Files:**
- Modify: `src/home/app_registry/splash_host.rs`
- Create: `src/home/app_registry/template_preflight_audit.rs`
- Modify: `src/home/app_registry/weather.rs` (schema-path participation only if needed)

- [x] **Step 1: Add the missing host error and parser support**

Implement:
- parse error reporting
- binding-path-not-in-schema error
- minimal `SplashAst` needed for validation

- [x] **Step 2: Implement W5/W7/attribution checks**

`load_template` must reject:
- unlisted widget
- unlisted local function
- attribution override attempts

- [x] **Step 3: Implement binding-path schema checking**

Introduce the narrow schema-path contract needed for preflight.
For v1, weather can provide a static list of legal paths.

- [x] **Step 4: Add build-time preflight audit**

Create the library-internal audit that scans every `.splash` template with
`include_str!` and turns any template violation into a red `cargo test --lib`.

### Task 3: Implement template-runtime Slice B (cache + compatibility)

**Source plan:** template-runtime Slice B

**Files:**
- Create: `src/home/app_registry/template_cache.rs`
- Modify: `src/home/app_registry/splash_host.rs`
- Modify: `Cargo.toml` only if the approved spec allowance is needed for a direct `sha2` entry

- [x] **Step 1: Add `CacheKey` and `TemplateCache`**

The cache key must include all six approved compatibility dimensions:
- `app_type`
- `app_version`
- `template_id`
- `template_hash`
- `manifest_version`
- `host_version`

- [x] **Step 2: Implement `template_hash`**

Use the approved strong-hash route from the runtime spec/plan.

- [x] **Step 3: Integrate cache lookup into `load_template`**

Miss:
- parse
- preflight
- cache insert

Hit:
- skip parse/preflight
- reuse `TemplateHandle`

- [x] **Step 4: Add compatibility invalidation tests**

Prove misses happen when:
- template source changes
- manifest version changes
- host version changes

### Task 4: Implement template-runtime Slice C (fallback + fail-explainable errors)

**Source plan:** template-runtime Slice C

**Files:**
- Modify: `src/home/app_registry/splash_host.rs`
- Create/modify: `src/home/app_registry/fallback.rs`
- Modify: `src/home/app_registry/capability_descriptors.rs`
- Modify: `src/home/app_registry/mod.rs` (log-field-only seam adjustments)

- [x] **Step 1: Add `FallbackReason` and `ValidationError`**

Keep this at the "fail-explainable" level only.
Do **not** introduce a receipt/provenance store.

- [x] **Step 2: Add template-to-template fallback**

Support:
- preferred template
- optional fallback template
- structured logging on each fallback event

- [x] **Step 3: Preserve existing plain-text fallback behavior**

Unsupported app version remains:
- `AppLookup::VersionMismatch`
- plain text fallback

Do not introduce internal app-version downgrading.

- [x] **Step 4: Add the meta-guards**

Keep the runtime contract honest:
- no provenance storage modules
- no accidental dependency graph growth beyond the approved allowance

### Task 5: Documentation sync for the runtime phase

**Source plans:** template-runtime Slice D + splash-host-evolution doc sync

**Files:**
- Modify: `docs/design/agent-to-app-design.md`
- Modify: `docs/roadmap/2026-04-21-splash-host-evolution-implementation-plan.md`
- Modify: `docs/roadmap/2026-04-22-template-runtime-implementation-plan.md`
- Modify: `AGENTS.md`
- Modify: `MAKEPAD.md`

- [x] **Step 1: Mark shipped runtime capabilities accurately**
- [x] **Step 2: Move still-unshipped items to the right deferred sections**
- [x] **Step 3: Sync the routing/authoring guidance**

### Task 6: Add the second capability (`news`) to prove reuse

**Source plan:** splash-host-evolution Slice 2

**Files:**
- Create: `src/home/app_registry/news.rs`
- Create: `src/home/app_registry/templates/news_guidance/*.splash`
- Modify: `src/home/app_registry/mod.rs`
- Modify: `src/home/app_registry/capability_descriptors.rs`
- Create/modify in OctOS:
  - `crates/octos-agent/src/capabilities/news_guidance.rs`
  - `crates/octos-agent/src/capabilities/mod.rs`
  - `crates/octos-cli/src/prompts/resolver_default.txt`
  - `crates/octos-cli/tests/resolver_fixtures/news_guidance/**`

- [x] **Step 1: Add the Robrix `news` consumer**

It must follow the exact same pattern as weather after the host/runtime
is complete.

- [x] **Step 2: Add the `news_guidance` capability**

Keep the capability thin:
- fetch data
- build state
- build body
- choose `template_id`

- [x] **Step 3: Add resolver fixtures**

Include:
- positive headlines
- positive digest
- negatives
- bilingual phrasing coverage

- [x] **Step 4: Prove "zero-Rust-UI"**

Gate the new capability with:
- line-count / no-UI-building assertions
- end-to-end host render test

### Task 7: Real E2E stabilization and default-enable criteria

This is the point where the stack stops being "architecturally correct"
and starts being operationally trustworthy.

**Files:**
- Mostly existing test/harness files in OctOS and Robrix
- Potentially small config/gating touch points if enabled-by-default policy changes

- [ ] **Step 1: Run the full weather + news routing matrix**
- [ ] **Step 2: Fix misroutes and follow-up-context failures**
- [ ] **Step 3: Eliminate stray post-card text replies**
- [ ] **Step 4: Define and satisfy the enable-by-default gate**

### Task 8: Post-foundation follow-up specs

These are **not** implementation of the current stack. They are the next spec
wave after the static-card runtime is proven.

- [ ] **Step 1: Write UI packaging / skill-with-UI spec**
- [ ] **Step 2: Write L2 actions spec**
- [ ] **Step 3: Write L3 stateful mini-app spec**

## Checkpoints

### Checkpoint A — Robrix weather host migration complete

Must be true:
- weather renders via static Splash template
- weather visual parity is acceptable

### Checkpoint A2 — OctOS weather cleanup complete

Must be true after OctOS work resumes:
- OctOS no longer builds weather UI strings
- weather routing/regression checks still pass

### Checkpoint B — Template runtime complete

Must be true:
- build-time preflight works
- cache + compatibility work
- fallback hierarchy is implemented
- errors are fail-explainable
- v1 state updates support `replace` / `remove` and reject `append`

### Checkpoint C — Reuse proven

Must be true:
- `news` works through the same host/runtime
- no weather-only assumptions remain

### Checkpoint D — Operational confidence

Must be true:
- routing accuracy is stable enough
- follow-up context failures are understood/handled
- default-enable decision can be made from evidence

## Explicit Non-Goals of This Plan

This master plan does **not** include:

- HTML / WebView fallback
- A2UI-style JSON component graph rendering
- full provenance / receipt persistence
- marketplace / third-party distribution
- dynamic capability discovery
- generalized generated-template runtime as the default path

## Recommended Execution Order

1. User-E2E test weather on the hardened Robrix runtime.
2. Resume OctOS weather cleanup and routing/regression checks when allowed.
3. Add `news` as the first zero-Rust-UI reuse proof.
4. Only then write / execute packaging and interactive-app follow-up specs.

## Final Outcome

If this plan is completed, the project ends up with:

- one reusable native Splash host
- validated/static templates
- bounded local functions and widget surface
- cache + compatibility semantics
- explicit fallback behavior
- one reference capability (`weather`)
- one reuse proof (`news`)

That is the minimum complete foundation before skill-carried UI, packaging,
and interactive mini-app layers are worth pursuing.
