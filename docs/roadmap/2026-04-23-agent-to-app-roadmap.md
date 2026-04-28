# Roadmap: Agent-to-App Runtime & Splash-First Delivery

**Date:** 2026-04-23
**Status:** Consolidated roadmap after the A2UI / AOSF / harness-engineering discussion
**Owner:** Robrix + OctOS

## Purpose

This roadmap turns the current agent-to-app discussion into one execution
sequence:

- what is already shipped
- what is currently in flight
- what must land next
- what is explicitly deferred

It does **not** replace the approved task specs or per-spec implementation
plans. It is the high-level delivery map that keeps them aligned.

## Current Baseline

### Already landed

- `resolver -> dispatcher -> capability -> org.octos.app` exists in OctOS
  for the weather reference capability.
- Robrix can render weather cards from `org.octos.app`.
- Weather v2 guidance cards exist and are user-visible.
- The architectural root document is approved:
  [agent-to-app-design.md](../design/agent-to-app-design.md)
- The two load-bearing implementation specs are approved:
  - [task-agent-to-app-splash-host-evolution.spec.md](../../specs/task-agent-to-app-splash-host-evolution.spec.md)
  - [task-agent-to-app-template-runtime.spec.md](../../specs/task-agent-to-app-template-runtime.spec.md)

### Landed in the current Robrix branch (updated 2026-04-24)

- Splash-first host/runtime foundation:
  - `SplashHost` trait and process-wide host accessor
  - widget manifest and W5 preflight checks
  - local function registry and W7 preflight checks
  - capability descriptor table and host-owned chrome
  - weather static template extraction
  - template preflight audit
  - `TemplateHandle` cache + compatibility key
  - fallback/error-shape primitives
- **P0 safety hardening (2026-04-24)**: removed non-spec bypass paths
  (`bind_guidance_template`, `bind_news_template`) from production;
  `RenderedApp::render` returns `Result<String, RenderFailure>`; host
  preflight/bind failures now project to `None` → Matrix body plain-text.
  Release build fails if production references any test-only bypass.
- **P1a template source consolidation (2026-04-24)**: all production
  `include_str!` lookups go through `templates::source_for` /
  `templates::ALL_TEMPLATES`. 4 sites → 1 site.
- **P2 end-to-end host-rejection tests (2026-04-24)**: 6 tests in
  `mod.rs::tests` exercising unsafe-widget / attribution-override /
  schema-binding rejection via the real dispatcher path; none of them
  reach a bypass. Leak-probe assertion included.
- Second capability (`news`): Robrix-side consumer + 2 templates + chrome
  all present; `news::FACTORY` registered.

### Not yet complete

- **E2E not verified**: Robrix weather + news rendering after P0/P1a/P2
  hardening has not been user-exercised against real Matrix traffic.
- **OctOS-side audit pending**: `weather_guidance.rs` legacy UI code,
  `news_guidance` capability, resolver prompts + fixtures for news —
  status inherited from prior plan; **unknown** in this worktree.
- **Resolver provider-error fallback (HIGH)**: prior observation showed
  dispatcher silently falling through to legacy `deep_search` when the
  LLM provider errors. Addendum in `task-agent-to-app-producer-routing.spec.md`
  (2026-04-22) reclassified this as `ResolverProviderError` requiring
  retry-once + explicit short error; OctOS-side implementation state
  unknown. This is the most likely cause of "card-looking text" a user
  sees when agent2app appears to fail.
- **Expanded live LLM resolver gate** for weather + news has not been run.
- **Packaging / skills-with-UI / marketplace**: remain deferred.

## Architectural Direction

The roadmap keeps the already approved direction:

- **Splash-first**
- **Rust host written once**
- **Capabilities produce state, not UI code**
- **A2UI guardrails adopted, A2UI JSON component graph rejected**

In practical terms:

- borrow from A2UI:
  - whitelist thinking
  - validation loop
  - local-vs-remote action split
  - incremental state semantics
  - explicit fallback behavior
- do **not** adopt:
  - adjacency-list component graphs as the primary render format
  - a second transport protocol beside Matrix timeline events
  - free-form LLM-generated UI code without host validation

## Delivery Phases

### Phase 0 — Weather reference baseline

**Status:** Mostly done

Goal:
- keep the current weather capability as the reference vertical slice
- finish stabilizing visible regressions before broadening scope

Exit condition:
- weather card renders correctly
- no stray assistant text after successful card replies
- follow-up weather phrasing is understood well enough to support testing

### Phase 1 — Splash Host Evolution (weather first)

**Status:** Robrix path implemented; OctOS cleanup deferred
**Primary source of truth:** [2026-04-21-splash-host-evolution-implementation-plan.md](./2026-04-21-splash-host-evolution-implementation-plan.md)

Goal:
- move weather from Rust-heavy string assembly to a real Splash-first host path
- prove the host contract on the one capability that already exists

Key outcomes:
- `SplashHost`
- `WidgetManifest` (W5)
- `LocalFunctionRegistry` (W7)
- `CapabilityDescriptor` / host-owned chrome
- `card_standard.splash` for weather
- `WeatherRenderedApp::render()` becomes a host call site instead of a UI builder

Exit condition:
- weather card is rendered through the host/template path
- no `render_guidance_weather` style UI code remains in OctOS after OctOS
  work resumes
- existing `mod.rs -> rendered.render(app_language)` seam stays intact

### Phase 2 — Template Runtime Contract

**Status:** Implemented in current Robrix branch; not user-E2E verified
**Primary source of truth:** [2026-04-22-template-runtime-implementation-plan.md](./2026-04-22-template-runtime-implementation-plan.md)

Goal:
- turn the host from "can render a template" into
  "can validate, cache, version, and fall back safely"

Scope:
- preflight validation
- TemplateHandle cache
- cache/compatibility unified design
- fallback hierarchy
- fail-explainable error shape

Explicitly not in this phase:
- provenance store
- render receipt persistence
- skills carrying UI
- marketplace / distribution

Exit condition:
- every production `.splash` template passes build-time preflight
- cache invalidates on real compatibility boundaries
- template failures degrade predictably to plain text

### Phase 3 — Zero-Rust-UI Proof (`news_guidance`)

**Status:** Implemented as a deterministic reuse proof; not user-E2E verified
**Primary source of truth:** splash-host-evolution Slice 2

Goal:
- prove the host/template/runtime stack is reusable
- show that a new capability can ship with minimal Rust and no bespoke Rust UI

Scope:
- `news` consumer registration in Robrix
- `news_guidance` capability in OctOS
- 2 Splash templates
- resolver fixtures for news

Current caveat:
- `news_guidance` uses deterministic sample data, not a production news API/source.

Exit condition:
- a second app type renders through the same host
- no weather-only assumptions remain in the host/runtime path

### Phase 4 — Runtime Hardening for Release Enablement

**Status:** Not started

Goal:
- close the gap between "architecture works" and "safe to enable by default"

Candidate work:
- broader E2E matrix across phrasing families
- regression baselines
- startup gating / feature rollout policy
- render-path observability strong enough to debug fallback behavior

Exit condition:
- agent-to-app can be turned on by default without relying on manual babysitting

### Phase 5 — UI Packaging & Capability Distribution

**Status:** Deferred on purpose

This is where the earlier AOSF-inspired concerns belong, but **not before**
Phases 1-4 are stable:

- skills carrying UI artifacts
- capability packaging format
- third-party template/capability distribution
- marketplace / catalog governance
- richer provenance and trust workflows

This phase needs its own spec family, not ad hoc additions to runtime work.

### Phase 6 — L2 / L3 Interactive Apps

**Status:** Deferred

After the static-card runtime is stable:

- L2 remote actions
- local function actions with bounded semantics
- incremental state updates
- stateful mini-app host lifecycle
- tick-driven apps

These remain downstream of the static-card foundation.

## Workstreams

### Workstream A — User-visible correctness

Priority now:
- weather reply correctness
- card/no-card routing correctness
- suppressing stray post-card text replies
- making follow-up weather prompts route consistently

### Workstream B — Host/runtime foundation

Implementation in current Robrix branch:
- preflight
- cache
- compatibility
- fallback

Priority now:
- user-E2E the hardened weather path
- keep fallback logs explainable during real Matrix testing

This is the load-bearing engineering work from the A2UI/harness discussion.

### Workstream C — Capability scaling proof

Priority after A/B:
- `news_guidance`
- prove reusable host
- prove "new app does not need a custom Rust UI layer"

### Workstream D — Packaging / ecosystem

Priority later:
- skills-with-UI
- capability packs
- third-party distribution

## What Is Explicitly Deferred

To avoid plan creep, the following are **not** part of the current active
delivery scope:

- full provenance / render receipt persistence
- UI marketplace
- dynamic capability discovery
- HTML / WebView fallback
- A2UI-style JSON component graph as the main render representation
- generalized LLM-generated templates as the default path

## Recommended Near-Term Sequence

1. User-test weather on the hardened Robrix host/runtime path.
2. If user E2E is acceptable, resume OctOS cleanup and routing regression
   gates.
3. Add `news_guidance` as the zero-Rust-UI proof.
4. Only after weather + news are stable, write packaging / skills-with-UI
   specs.

## Roadmap Summary

The current roadmap is:

- **now:** E2E-test the hardened weather path
- **next:** resume OctOS cleanup/routing gates when allowed
- **then:** prove the shape with `news`
- **later:** package UI with skills and grow into L2/L3 apps

This keeps the project focused on the real bottleneck:

**turning agent-to-app from a working demo into a governable native runtime.**
