# Splash Host Evolution — Implementation Plan

**Date:** 2026-04-21
**Status audit (2026-04-24):**
- **Robrix side** — Slice 1 Steps 1.1-1.9 DONE; template-runtime Slices A / B / C (as consumed by this plan) DONE; Slice 2 Steps 2.1-2.5 DONE (news consumer + templates + registration on Robrix); post-hoc P0 / P1a / P2 safety hardening DONE (see §"2026-04-24 Safety Hardening" below).
- **OctOS side** — Slice 2 Steps 2.6-2.10 (news_guidance capability + resolver prompts + fixtures) **status unknown**, not verified from this worktree. Inherited from producer-routing plan scope; treat as OPEN until separately audited.
- **E2E** — user-facing Matrix flow for news has **NOT** been verified; prior observation suggests resolver provider-error fallback (`task-agent-to-app-producer-routing.spec.md` 2026-04-22 addendum) may still be sending plain text instead of app envelope.
**Owner:** Robrix (primary) + OctOS (capability-side only; not inventoried here)

## Inputs

Generated from one approved task spec + its binding design document:

| Source | Reference |
|---|---|
| Spec (approved) | [`task-agent-to-app-splash-host-evolution`](../../specs/task-agent-to-app-splash-host-evolution.spec.md) |
| Design contract | [`docs/design/agent-to-app-design.md`](../design/agent-to-app-design.md) §4.3a, §5 Layer 2, §6 W5/W7, §6.1 attribution |
| Prior plan (reference shape) | [`2026-04-17-agent-to-app-implementation-plan`](./2026-04-17-agent-to-app-implementation-plan.md) |

## Scope Summary

Turn the current "`weather_guidance::render_guidance_weather` pushes 200 lines
of Splash-DSL string from Rust" into "Splash Host reads a `.splash` template
file, binds `state` via declarative paths, injects attribution chrome, emits
the same Splash DSL string — with a widget trust whitelist, a local-function
whitelist, and an attribution-override guard enforced at template load time."

Prove the new shape works by adding `news_guidance` as a **second capability
that ships with zero Rust UI code** (`≤ 120 lines` Rust, 2 `.splash` templates,
at most 2 news-specific widgets).

**No `org.octos.app` protocol change.** **No `room_screen.rs` change.**
**No `mod.rs::render_app_envelope_to_splash` control-flow change.**
`RenderedApp::render() -> String` stays the per-app render exit point;
the Splash Host is called *inside* each `RenderedApp::render` impl
(weather.rs, news.rs, etc.), not from the dispatcher. Existing
`content.splash_card.set_text(...)` seam preserved.

## Cross-Repo Boundary

| Repo | What changes |
|---|---|
| **robrix2** (this) | Splash Host trait + widget manifest + local function registry + capability descriptor table; `templates/` + `widgets/` assets under `src/home/app_registry/`; `WeatherRenderedApp::render` / `NewsRenderedApp::render` become the per-app host call sites (mod.rs dispatcher unchanged); `news` consumer registration; docs updates (§7.1 / §7.3, `AGENTS.md`, `MAKEPAD.md`) |
| **octos** (`/Users/zhangalex/Work/Projects/FW/octos`) | Delete UI code from `weather_guidance.rs`; add `news_guidance.rs` (≤ 120 lines); extend resolver prompt + fixtures; register `news_guidance` in `CapabilityRegistry` |

## Design Decision — v1 chrome source (implementing the spec addendum)

The spec's `§Attribution Chrome → 数据来源（v1 + future path）` subsection
(`task-agent-to-app-splash-host-evolution.spec.md` at the Attribution Chrome
block) **explicitly prescribes v1 chrome to come from a Robrix-side static
`CapabilityDescriptor` table**, not from envelope metadata. This plan
implements that approved addendum directly; there is no deviation.

**V1 implementation**: Robrix-side static `CapabilityDescriptor` table
(Step 1.3a) keyed by `app_type`. Each `RenderedApp::render` looks up chrome
from this table. OctOS dispatcher does **not** write chrome into envelope
metadata in v1 — the spec addendum explicitly defers the envelope chrome
channel to a future amendment.

**Future path** (NOT in this plan, captured verbatim in the spec addendum
and design doc §6.1): when the first dynamic / third-party /
orchestrator-gated capability ships, open a spec amendment to formalize an
envelope chrome channel (most likely as an optional sibling field under the
existing "optional extensions don't bump version" rule established by
`language` / `focus`). Do **not** preempt that design here.

This section exists so implementers reading the plan know to follow the
descriptor table path and do not revert to an envelope-metadata
interpretation of earlier spec drafts. If an implementer wonders "why not
put chrome in the envelope?", the answer is in the spec addendum, not
in a verbal override.

## Phase Plan

Two sequential vertical slices plus a docs-sync tail. Each slice must fully
pass its scenarios before the next starts — this is the lesson from
producer-routing, where premature forward progress caused re-work.

### Slice 1 — Weather template extraction (≈3d, the load-bearing vertical slice)

Goal: prove the Splash Host + W5 + W7 + attribution guard end-to-end on the
**one capability that already works**, so Slice 2 only has to demonstrate
"zero-Rust-UI" shape, not also validate host correctness.

Order matters — 1.1-1.3 build the host primitives; 1.4-1.5 prepare the
weather-specific assets; 1.6-1.7 are superseded by the template-runtime
plan's Slice A/B/C; 1.8-1.9 rewire Robrix; 1.10 deletes OctOS UI code;
1.11 is the regression gate. Per user direction on 2026-04-23, do not
touch OctOS agent code until explicitly resumed.

| Step | New/Edited | Scenarios unlocked / bound |
|---|---|---|
| 1.1 | **DONE** — `robrix2/src/home/app_registry/splash_host.rs` defines the `SplashHost` trait, `TemplateHandle`, `HostError`, `AttributionChrome`, and `ActionOutcome`. | — (primitives that template-runtime closes against) |
| 1.2 | **DONE** — `robrix2/src/home/app_registry/widget_manifest.rs` defines the v1 W5 manifest for public template-reachable builtins. | W5 enforcement |
| 1.3 | **DONE** — `robrix2/src/home/app_registry/local_functions.rs` defines the v1 W7 closed registry. | W7 enforcement |
| 1.3a | **DONE** — `robrix2/src/home/app_registry/capability_descriptors.rs` defines v1 `CapabilityDescriptor`, `HOST_VERSION`, version fields, and weather chrome. | `AttributionChrome` construction and cache compatibility fields |
| 1.4 | **DONE / no-op** — weather v1 uses registered Makepad builtins only; no weather-specific widget extraction. | plan/reality reconciliation |
| 1.5 | **DONE** — `robrix2/src/home/app_registry/templates/weather_guidance/card_standard.splash` is the static weather template. | template loads in host |
| 1.6 | **DONE via template-runtime Slice A** — `load_template` preflight covers parse, W5, W7, attribution override, and binding-path schema checks. | `test_preflight_rejects_*`, `test_all_templates_pass_preflight_at_build_time` |
| 1.7 | **DONE via template-runtime Slice B/C + follow-up** — render binding, cache/compatibility, fallback hierarchy, error shape, and v1 JSON Pointer `replace` / `remove` state updates are implemented. `append` / `splice` remain L2 follow-up. | cache/fallback/error-shape tests; render binding tests; `test_host_applies_state_update_replace`; `test_host_rejects_append_op_in_v1` |
| 1.8 | **DONE** — `app_registry::splash_host()` exposes the process-wide host without changing dispatcher control flow. | `SplashHost` reachable from per-app renderers |
| 1.9 | **DONE** — `weather.rs::render_guidance_weather` calls `SplashHost::load_template → render_to_splash` and uses descriptor-derived chrome. **P0 safety hardening (2026-04-24)**: the legacy string-binding fallback (`bind_guidance_template`) has been **removed from the production render path** and gated `#[cfg(test)]`; host preflight/bind failures now return `Err(RenderFailure)` → dispatcher returns `None` → Matrix `body` plain-text fallback. Same treatment applied to `news.rs::bind_news_template`. See §"2026-04-24 Safety Hardening". | Robrix weather card renders through template host; unsafe template content cannot leak past the host |
| 1.10 | **DEFERRED** — OctOS-side `weather_guidance.rs` cleanup is intentionally not started per user instruction. | pending |
| 1.11 | **DEFERRED** — producer-routing regression gate waits for OctOS work to resume. | pending |

**Gate → Slice 2:**
- All Slice-1 scenarios above pass.
- Robrix weather rendering goes through `SplashHost` and static template.
- OctOS `weather_guidance.rs` no longer contains `render_guidance_weather` / `out.push_str` after OctOS work resumes — enforceable by grep in CI.
- E2E smoke: user sends "今天北京穿什么" → weather card renders via new path, visually indistinguishable from pre-migration.
- `cargo check` clean; template linter passes on `card_standard.splash`.

### Slice 2 — news_guidance zero-Rust-UI (≈2d, shape-proof)

Goal: demonstrate that a **new** capability needs only schema + state + 2
templates + ≤ 120 lines Rust, and that resolver + dispatcher + host machinery
from Slice 1 absorbs it without code changes.

| Step | New/Edited | Scenarios unlocked / bound |
|---|---|---|
**Robrix-side (DONE 2026-04-24 audit):**

| Step | New/Edited | Scenarios unlocked / bound |
|---|---|---|
| 2.1 | **DONE / no-op in practice** — `news.rs` consumer also uses registered Makepad builtins only (no `widgets/news_card.rs` file); same v1 addendum as weather. | Template loads without bespoke widgets |
| 2.2 | **DONE** — `robrix2/src/home/app_registry/templates/news_guidance/headlines_card.splash` present. | Template bundled via `templates::NEWS_HEADLINES_CARD` |
| 2.3 | **DONE** — `robrix2/src/home/app_registry/templates/news_guidance/digest_card.splash` present. | Template bundled via `templates::NEWS_DIGEST_CARD` |
| 2.4 | **DONE** — `robrix2/src/home/app_registry/news.rs` (573 lines): `NewsFactory` + `NewsCapabilitySchema` + `NewsTemplateViewModel`; `NewsRenderedApp::render` returns `Result<String, RenderFailure>` via `SplashHost::load_template → render_to_splash` with `news_host_error_to_render_failure` classifier. Post-P0: `bind_news_template` is `#[cfg(test)]` only. | `news` app type rendered end-to-end through host |
| 2.5 | **DONE** — `news::FACTORY` registered in `mod.rs::registry()` alongside weather; `capability_descriptors` carries `news` entry with chrome + version fields. | news envelopes route through same dispatcher path |

**OctOS-side (STATUS UNKNOWN — not audited from this worktree):**

| Step | New/Edited | Status |
|---|---|---|
| 2.6 | `octos/crates/octos-agent/src/capabilities/news_guidance.rs` — Rust `Capability` impl | **UNKNOWN** — verify in OctOS repo |
| 2.7 | `octos/crates/octos-agent/src/capabilities/mod.rs` registers `news_guidance` | **UNKNOWN** |
| 2.8 | `octos/crates/octos-cli/src/prompts/resolver_default.txt` news_guidance block | **UNKNOWN** |
| 2.9 | `octos/crates/octos-cli/tests/resolver_fixtures/news_guidance/` ≥ 40 positives + ≥ 10 negatives | **UNKNOWN** |
| 2.10 | `news_guidance.rs` ≤ 120 lines + no Makepad refs test | **UNKNOWN** |
| 2.11 | E2E "今天有什么科技新闻" envelope → Robrix render | **NOT VERIFIED** — prior run showed resolver provider-error silent fallback (see §"Open Items" below) |

**Gate → Slice 3:**
- All Slice-2 scenarios above pass.
- `news_guidance.rs` ≤ 120 lines verified in CI.
- E2E smoke: user sends "今天有什么科技新闻" → news headline card renders.
- Resolver robustness gate on full corpus (weather + news): top-1 ≥ 90%, FPR = 0%.

### Slice 3 — Documentation sync (≈0.5d, nothing ships until this lands)

| Step | File | Change |
|---|---|---|
| 3.1 | `docs/design/agent-to-app-design.md` §7.1 Shipped | Add: `SplashHost trait + static template loader`, `WidgetManifest + W5 enforcement`, `LocalFunctionRegistry + W7 enforcement`, `AttributionChrome host-owned`, `weather template extraction`, `news_guidance first zero-Rust-UI capability`. |
| 3.2 | `docs/design/agent-to-app-design.md` §7.3 Not yet shipped | Remove items now shipped. Keep L2 actions, L3 stateful, generated templates, dynamic capability discovery. |
| 3.3 | `AGENTS.md` | Add "Splash template authoring" section: pointer to `templates/` + `widgets/` locations, W5/W7/attribution guard rules, how to add a new widget to manifest. |
| 3.4 | `MAKEPAD.md` | Add "Template authoring" row to routing table. |
| 3.5 | This plan | Mark Status: **Shipped** with commit hashes. |

**Gate → merge:** Slice 3 lands in the same PR stack as Slice 2. No
documentation drift allowed — doc updates are part of the shippable unit.

## 2026-04-24 Safety Hardening (post-Slice-1 audit)

After Slice 1 + Slice 2 Robrix-side code landed, a bounded audit surfaced
one **High** spec violation and two consolidation opportunities. All three
were fixed in 2026-04-24 without touching runtime behavior beyond what
safety required. These are recorded here as DONE work, not new steps:

### P0 — Remove non-spec bypass fallback (safety boundary)

**Finding**: `weather.rs::render_guidance_weather` and
`news.rs::render_news` had a production fallback (`bind_guidance_template`
/ `bind_news_template`) that ran `str::replace` binding on the raw
template source when `SplashHost::load_template` or `render_to_splash`
returned `Err`. This **bypassed** W5 / W7 / attribution preflight guards
— if the host rejected a template for a whitelist violation, the
fallback reconstructed it anyway.

**Fix**:
- Added `RenderFailure` enum to `mod.rs` (`HostRejected` / `HostError` /
  `TemplateMissing` / `Internal`)
- Changed `RenderedApp::render` signature from `-> String` to
  `-> Result<String, RenderFailure>`
- `mod.rs::render_app_envelope_to_splash` projects `Err(_)` → `None` →
  existing plain-text body fallback path
- `weather::host_error_to_render_failure` +
  `news::news_host_error_to_render_failure` classifier fns map HostError
  variants to RenderFailure categories (preflight-guard variants →
  `HostRejected`; runtime-binding variants → `HostError`;
  `TemplateNotFound` → `TemplateMissing`)
- `bind_guidance_template`, `bind_news_template`,
  `guidance_template_source`, `news_template_source`, and the
  `GuidanceTemplateViewModel::bindings()` helper are all gated
  `#[cfg(test)]`. **Release builds fail to compile** if any production
  code references them.

### P1a — Consolidate `include_str!` template sources

**Finding**: 4 separate `include_str!("templates/...")` sites (`weather.rs`,
`splash_host.rs`, `template_preflight_audit.rs`, `templates.rs`) could
drift — edit the template path in one site, stale in others.

**Fix**:
- `templates.rs` is now the **single source of truth**: holds all
  `include_str!` calls, exposes `ALL_TEMPLATES` table + `source_for(cap,
  tid) -> Option<&'static str>`
- `splash_host.rs::load_template_source` delegates to `source_for`
- `template_preflight_audit.rs` iterates `ALL_TEMPLATES`
- `weather.rs` / `news.rs` `#[cfg(test)]` bypass helpers source through
  `templates::WEATHER_CARD_STANDARD` / `templates::NEWS_HEADLINES_CARD`
- `include_str!("templates/...")` site count: **4 → 1**

### P2 — End-to-end host-rejection regression tests

**Finding**: classifier unit tests existed, but nothing locked the
end-to-end contract "host rejects → dispatcher returns `None` → no unsafe
content leaks".

**Fix**:
- Added `#[cfg(test)] DefaultSplashHost::load_template_from_source` —
  test-only backdoor that feeds arbitrary source through the real
  preflight validation
- Added `#[cfg(test)]` `UnsafeTemplateFactory` + `render_with_factory`
  dispatcher-equivalent helper in `mod.rs` tests
- 6 new tests in `mod.rs::tests`:
  `end_to_end_unsafe_widget_rejected_returns_none`,
  `end_to_end_attribution_override_rejected_returns_none`,
  `end_to_end_schema_binding_path_rejected_returns_none`,
  `host_load_template_from_source_rejects_unsafe_widget`,
  `host_rejection_produces_no_splash_leak` (contains leak-probe token),
  `version_mismatch_path_still_returns_none`
- `weather::host_error_to_render_failure` exposed as `pub(crate)` so
  dispatcher tests reuse production classifier

### Post-hardening test state

- `cargo test --lib`: 377 passed / 0 failed (371 after P1a + 6 P2)
- `cargo check --lib --release`: **warning-clean** — confirms no
  production code references any `#[cfg(test)]`-gated bypass helper

## Open Items

Not implemented in this plan's scope; tracked so they are not forgotten:

1. **OctOS-side news producer path** — Steps 2.6-2.10 state unknown;
   requires separate OctOS-repo audit.
2. **Resolver provider-error fallback (HIGH priority)** — Prior E2E
   observation: when the LLM provider returns a network/HTTP/timeout
   error, `session_actor`'s dispatcher silently falls through to legacy
   `deep_search` tool-calling. The addendum in
   `task-agent-to-app-producer-routing.spec.md` (2026-04-22) classifies
   this as `ResolverProviderError` and requires retry-once + explicit
   short error, NOT legacy fall-through. Implementation is in OctOS;
   status unknown. Likely cause of "text bubble that looks like a card"
   in user-facing observations.
3. **news `org.octos.app` envelope generation** — Even if 2.6-2.10 land,
   verifying that OctOS actually emits `type=news, version=1` envelopes
   from real LLM resolver traffic is a separate E2E task.
4. **Doc sync §7.1/§7.3** — `docs/design/agent-to-app-design.md` section
   updates (Slice 3 Steps 3.1-3.2) are open; design doc may still list
   shipped items under "Not yet shipped".
5. **OctOS `weather_guidance.rs::render_guidance_weather` deletion** —
   Plan Step 1.10 said "delete octos-side UI code from weather_guidance";
   Robrix side no longer needs it since Robrix does the rendering
   client-side. Deletion in OctOS repo has not been verified.
6. **CI test count baseline** — `cargo test --lib` is now 377 tests;
   baseline should be pinned in CI to catch silent test removal.

## Scenarios Coverage Map

Spec scenario → plan step (for traceability):

| Scenario | Step |
|---|---|
| `test_splash_host_loads_weather_card_and_binds_state` | 1.7 + 1.9 (chrome source via 1.3a) |
| `test_splash_host_rejects_unlisted_widget` | 1.6 |
| `test_splash_host_rejects_unlisted_local_function` | 1.6 |
| `test_splash_host_rejects_attribution_override` | 1.6 |
| `test_weather_card_template_structural_parity` | 1.10 + 1.11 |
| `test_news_guidance_has_no_rust_ui` | 2.6 + 2.10 |
| `test_news_guidance_end_to_end` | 2.11 |
| `test_resolver_fixtures_news_guidance` | 2.9 |
| `test_host_applies_state_update_replace` | 1.7 |
| `test_host_rejects_append_op_in_v1` | 1.7 |
| `test_host_rejects_generated_template_slot` | 1.6 |
| `test_weather_guidance_regression_after_template_extraction` | 1.11 |
| `test_design_doc_phase_tracking` | 3.1 + 3.2 |

All 13 scenarios bound to a concrete step. No orphan scenarios.

## Risks & Mitigations

- **R1 — Splash DSL binding syntax unknown.** Design doc §12 open question: `$state.path` vs point-notation. **Mitigation:** pick dot-notation in 1.7 (aligns with JSON Pointer + Makepad DSL feel); document in `AGENTS.md` (3.3). Escalate if hot-reload conflicts observed.
- **R2 — `TemplateHandle` intermediate form over-engineered.** Temptation to build a full AST. **Mitigation:** v1 `TemplateHandle` = parsed-but-unexpanded Splash string with AST metadata for `$state.path` and `${fn(...)}` sites. No IR, no typechecker beyond what W5/W7 need.
- **R3 — News API data source not specified in spec.** **Mitigation:** v1 use a fixture data source (hard-coded items) inside `news_guidance::fetch_data` to keep the slice scope on the architecture shape, not on news API integration. Real API source is a follow-up task.
- **R4 — 120-line hard cap on `news_guidance.rs` may force skipping needed code.** **Mitigation:** if 120 is genuinely tight after honest implementation, the cap is a **signal** that the host/manifest contract needs to absorb more — not a signal to inflate the file. Revisit cap in spec if measured limit is ≥ 150 on honest code.
- **R5 — `.splash` hot-reload with `include_str!` conflict.** Dev wants hot reload; prod wants static. **Mitigation:** step 1.1 declares both modes via `cfg`; resolve exact mechanism when 1.5 lands the first real template.

## Forward References (future specs identified mid-plan)

Codex review (2026-04-22) surfaced runtime-governance requirements (version
compatibility, render receipt, Splash preflight as a first-class concept,
TemplateHandle cache, fallback hierarchy) that extend beyond this plan's
scope. Rather than balloon this plan, those land in a focused follow-up
spec **after** Slice 1 + Slice 2 ship:

- **`task-agent-to-app-template-runtime.spec.md`** (approved and
  implemented through Slice C in Robrix). Scope strictly constrained to **host runtime contract** —
  not a governance platform, not a provenance system:
  1. **Template preflight validation** — parse + W5 + W7 +
     attribution-override + binding-path schema check at build time.
  2. **TemplateHandle cache + version compatibility** treated as one
     problem (cache reusability = version compatibility). Cache key
     composed from `(app_version, template_id, template_hash,
     manifest_version, host_version)`; mismatch forces invalidation.
  3. **Fallback hierarchy** — template → app version → plain text,
     each level explicit and logged.
  4. **Error shape (fail-explainable only)** — `fallback_reason`,
     `version_mismatch_reason`, template validation error shape; a full
     render-receipt/provenance store is explicitly **not** in scope.

  Explicitly **out of scope** for this future spec: full render-provenance
  storage, governance workflow, policy engine, trust/identity beyond what
  AttributionChrome already carries.

- **`task-agent-to-app-ui-packaging.spec.md`** (later, after
  `template-runtime` lands): skill-with-UI packaging, third-party
  capability/template distribution, capability market.

**This plan's preemptive accommodations** (avoid retrofits):
- Step 1.3a reserves `manifest_version` + `template_set_version` fields on
  `CapabilityDescriptor` and exports `HOST_VERSION: u32`. These do nothing
  in Slice 1 beyond being set to `1`; the runtime spec later wires them
  into compatibility checks and render receipt.
- Step 1.6 already runs W5/W7/attribution validation at `load_template`
  time (this is the preflight foundation the runtime spec will formalize).

## What This Plan Does NOT Cover

- L2 interactive actions (`org.octos.actions` button semantics) — `task-agent-to-app-l2-actions`
- L3 stateful mini-app tick scheduling — `task-agent-to-app-l3-stateful`
- Generated templates (Layer 5b Template-Author LLM actual implementation) — defer until a capability genuinely needs visual variety
- Dynamic capability discovery / plugin loading — design doc §10 Non-goals
- Performance benchmarks — out of scope, functional correctness only
- Real news API integration — v1 uses fixture data
- `org.octos.actions` wire format changes — host v1 only passes payload through
- **Template runtime governance** (cache / version matrix / fallback hierarchy / render receipt) — `task-agent-to-app-template-runtime.spec.md`, deferred per Forward References above
- **Skill-with-UI packaging** (skill as logic + UI contract + tests bundle) — `task-agent-to-app-ui-packaging.spec.md`, deferred

## Commit & PR Strategy

- **One PR per slice.** Slice 1 PR (~10-15 files touched) lands first, gets user E2E on weather. Slice 2 PR adds news. Slice 3 PR rolls docs + marks plan Shipped.
- **Do not mix cross-repo changes in one PR.** Robrix PR and OctOS PR go in parallel; both reference this plan + the spec.
- **Each PR must cite**: spec scenarios closed + regression baseline (weather fixtures continue to pass).

## Timeline

| Slice | Estimate | Dependencies |
|---|---|---|
| Slice 1 | 3d | Spec approved ✅; weather fixtures from producer-routing ✅ |
| Slice 2 | 2d | Slice 1 gate passed |
| Slice 3 | 0.5d | Slice 2 gate passed |

**Total:** 5.5d (matches spec `estimate: 5d` + 0.5d doc overhead).
