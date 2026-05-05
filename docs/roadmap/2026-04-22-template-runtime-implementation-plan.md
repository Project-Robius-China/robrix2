# Template Runtime — Implementation Plan

**Date:** 2026-04-22
**Status audit (2026-04-24):** Slice A / B / C / D all DONE in the current Robrix worktree. Subsequently hardened by out-of-band P0 / P1a / P2 safety fixes (see `2026-04-21-splash-host-evolution-implementation-plan.md` §"2026-04-24 Safety Hardening"): bypass paths removed + `RenderFailure` typed fallback + template source consolidated to `templates::ALL_TEMPLATES` + end-to-end host-rejection regression tests. **Not user-E2E verified**. Not committed.
**Owner:** Robrix (single-repo; no OctOS changes)

## Inputs

| Source | Reference |
|---|---|
| Spec (approved) | [`task-agent-to-app-template-runtime`](../../specs/task-agent-to-app-template-runtime.spec.md) |
| Upstream spec (also approved) | [`task-agent-to-app-splash-host-evolution`](../../specs/task-agent-to-app-splash-host-evolution.spec.md) |
| Upstream plan (in progress) | [`2026-04-21-splash-host-evolution-implementation-plan`](./2026-04-21-splash-host-evolution-implementation-plan.md) |

## Relationship to the splash-host-evolution plan

This plan **replaces Steps 1.6 and 1.7** of the splash-host-evolution plan
with a finer-grained breakdown that honors the runtime contract (preflight,
cache, fallback, error shape). Steps 1.1–1.5 and 1.8–1.11 of that plan are
unchanged and execute in their original order.

Resulting combined execution order:

```
splash-host-evolution Step 1.1   → splash_host.rs skeleton           (DONE)
splash-host-evolution Step 1.2   → widget_manifest.rs                (DONE)
splash-host-evolution Step 1.3   → local_functions.rs                (DONE)
splash-host-evolution Step 1.3a  → capability_descriptors.rs         (DONE)
splash-host-evolution Step 1.4   → widgets/weather_card.rs / no-op     (DONE)
splash-host-evolution Step 1.5   → templates/weather_guidance/card_standard.splash (DONE)
template-runtime      Slice A    → preflight validation + audit test     (DONE)
template-runtime      Slice B    → TemplateHandle cache + compat matrix  (DONE)
template-runtime      Slice C    → fallback hierarchy + FallbackReason    (DONE)
template-runtime      Slice D    → doc sync                              (DONE)
splash-host-evolution Step 1.8   → splash_host() singleton accessor      (DONE)
splash-host-evolution Step 1.9   → weather.rs rewire                     (DONE)
splash-host-evolution Step 1.10  → delete octos-side render_guidance_weather (DEFERRED)
splash-host-evolution Step 1.11  → regression gate (producer-routing fixtures + baseline byte-equality) (DEFERRED)
splash-host-evolution Slice 2    → news_guidance (2.1-2.11)
splash-host-evolution Slice 3    → docs sync
```

## Scope Summary

Implement the 4-block runtime contract declared by `task-agent-to-app-template-runtime.spec.md`:

1. **Preflight validation** (five checks run at `load_template` time)
2. **TemplateHandle cache + version compatibility** (six-tuple cache key)
3. **Fallback hierarchy** (template-to-template → plain text)
4. **Error shape** (`FallbackReason` + `HostError::to_validation_error()`)

**Hard constraints preserved**:
- No `org.octos.app` envelope protocol change
- No `mod.rs::render_app_envelope_to_splash` control-flow change
- No `room_screen.rs` change
- No new **transitive** cargo crate. `sha2` may be promoted to a direct
  dependency because it already exists in `Cargo.lock`; this is verified
  by dependency-diff review rather than by a blanket `Cargo.toml` freeze.
- No provenance / receipt persistent store
- No app-version fallback (envelope version mismatch goes straight to plain text via existing `AppLookup::VersionMismatch` path)

## Phase Plan

Four slices. Each slice is a PR-sized unit. Gates between slices demand
all scenarios in that slice pass `cargo test --lib`.

### Slice A — Preflight validation (DONE)

Goal: fill in `DefaultSplashHost::load_template` with the five-check
validation pipeline. After this slice, templates that violate W5 / W7 /
attribution / binding-path rules fail at `load_template` call time.

| Step | New/Edited | Scenarios unlocked / bound |
|---|---|---|
| A.1 | `splash_host.rs` — extend `HostError` with `BindingPathNotInSchema { path: String, app_type: String, app_version: u32 }`. Unit test: locks variant shape + `Display` impl. | foundation for A.4 |
| A.2 | `splash_host.rs` — introduce `SplashAst` internal type (module-private) that wraps the parsed-but-unexpanded template. Fields: flat list of widget references, `$state.path` binding sites, `${fn(...)}` call sites, attribution-region markers. v1 parser strategy: recursive-descent over the raw `.splash` text using Makepad 2.0 DSL tokens (no new dep — reuse existing `makepad-widgets` tokenizer). | foundation for A.3-A.6 |
| A.3 | `splash_host.rs` — implement Check 1 (Parse): `parse_to_ast(source) -> Result<SplashAst, HostError::ParseError>`. Inline `#[cfg(test)]` lock: malformed input produces `ParseError { line }`. | closes `test_splash_host_rejects_parse_error` (implicit; covered by template-runtime Scenario X) |
| A.4 | `splash_host.rs` — implement Checks 2-4 on `SplashAst`: iterate widget references → W5 manifest lookup; iterate function call sites → W7 registry lookup; iterate attribution-region markers → reject if template content touches `capability_id`/`display_name`/`icon`/`trust_badge`. Each violation returns its specific `HostError` variant. | closes `test_preflight_rejects_unlisted_widget`, `test_preflight_rejects_unlisted_local_function`, `test_preflight_rejects_attribution_override` |
| A.5 | `splash_host.rs` — implement Check 5 (Binding path schema check). Introduce `CapabilitySchema` trait (method `contains_path(path: &str) -> bool`) implemented by each consumer (v1 just `WeatherFactory`). At `load_template` time, walk AST binding sites and call `contains_path` for each. Violation → `BindingPathNotInSchema`. | closes `test_preflight_rejects_binding_path_not_in_schema` |
| A.6 | `src/home/app_registry/template_preflight_audit.rs` — **lib-internal** `#[cfg(test)]` module. Use `include_str!` to bundle every `.splash` file under `templates/**`, parse into `(capability_id, template_id, source)` tuples, run each through `SplashHost::load_template`. Any failure = `cargo test --lib` red. This is the build-time-preflight-harness the spec requires. | closes `test_all_templates_pass_preflight_at_build_time` |

**Gate → Slice B:**
- Scenarios above all green under `cargo test --lib`.
- Running `cargo test --lib home::app_registry::` shows the 22 existing tests + new preflight audit test + new rejection tests.
- `weather/card_standard.splash` (Step 1.5, authored in splash-host-evolution) passes the audit — this is the first real template the host validates.

### Slice B — TemplateHandle cache + compatibility (DONE)

Goal: make `load_template` cache-aware so repeat calls with the same
six-tuple key skip parse + preflight. Add version-change invalidation.

| Step | New/Edited | Scenarios unlocked / bound |
|---|---|---|
| B.1 | `src/home/app_registry/template_cache.rs` (new) — `CacheKey { app_type: String, app_version: u32, template_id: String, template_hash: u64, manifest_version: u32, host_version: u32 }` + `TemplateCache` struct wrapping `RwLock<HashMap<CacheKey, Arc<TemplateHandle>>>`. Inline unit tests: key equality, key-by-field hash stability. | foundation for B.2-B.3 |
| B.2 | `template_cache.rs` — `fn template_hash(source: &str) -> u64` per spec: use `sha2::Sha256` over source bytes, take the first 16 hex chars and interpret as `u64` (u64-sized truncation of the full digest). `sha2` is **already transitively present** in `Cargo.lock` via matrix-sdk; this step adds a direct `sha2` entry to `Cargo.toml` to make the import clean but introduces **no new transitive crate** (verify before commit via `cargo tree --duplicates` vs baseline). This matches the spec's "sha256 前 16 字节 hex, u64-sized" requirement and the spec's explicit "优先复用 workspace 已有的 crypto crate" allowance. | foundation for B.3 |
| B.3 | `splash_host.rs` — rewire `DefaultSplashHost::load_template` to: (a) compute `template_hash` from source; (b) assemble six-tuple `CacheKey`; (c) attempt `template_cache.get(&key)`; (d) on hit, return cached `Arc<TemplateHandle>` clone; (e) on miss, run Slice A preflight; (f) on success, insert into cache and return. | closes `test_cache_hit_skips_parse`, `test_cache_miss_on_template_hash_change`, `test_cache_miss_on_manifest_version_change`, `test_cache_miss_on_host_version_change` |
| B.4 | `template_cache.rs` — breaking-change table self-consistency test (scenario: `test_breaking_change_table_self_consistent`). Iterate a fixture list of `(change_kind, bumps_app_version, bumps_manifest_version, bumps_host_version)` tuples and assert Non-breaking rows bump nothing, Breaking rows bump exactly one. Keeps the spec's compatibility matrix auditable from code. | closes `test_breaking_change_table_self_consistent` |

**Gate → Slice C:**
- Cache hit path demonstrably skips `parse_to_ast` (verified via an instrumentation counter in debug builds).
- Bumping any of the three version constants in a test harness correctly invalidates prior cache entries.
- `cargo check --lib` clean; no new cargo dependency.

### Slice C — Fallback hierarchy + error shape (DONE)

Goal: introduce `FallbackReason` + `ValidationError`, wire them into the
template-to-template fallback and into `mod.rs`'s existing plain-text path.

| Step | New/Edited | Scenarios unlocked / bound |
|---|---|---|
| C.1 | `splash_host.rs` — define `FallbackReason` enum with 3 variants per spec (`TemplateFailed { capability_id, preferred_template_id, underlying: Box<HostError> }`, `AllTemplatesFailed { final_error: Box<HostError> }`, `HostVersionMismatch { expected_host_version, got }`). Inline unit test: variant shape. | foundation for C.2-C.3 |
| C.2 | `splash_host.rs` — define `ValidationError { code: &'static str, path: String, message: String }` + `impl HostError { fn to_validation_error(&self) -> ValidationError }` with stable `code` strings (`"WIDGET_NOT_ALLOWED"`, `"LOCAL_FUNCTION_NOT_ALLOWED"`, `"ATTRIBUTION_OVERRIDE"`, `"BINDING_PATH_NOT_IN_SCHEMA"`, `"PARSE_ERROR"`). | closes `test_host_error_to_validation_error` |
| C.3 | `capability_descriptors.rs` — extend `CapabilityDescriptor` with `fallback_template_id: Option<&'static str>` (v1 `None` for `weather` — single template). Update constructor + unit test to lock new field. | unlocks C.4 |
| C.4 | `splash_host.rs` — introduce `fn render_with_fallback(handle_or_err, ...) -> Result<String, FallbackReason>` that tries preferred template; on failure looks up `fallback_template_id` and retries; on second failure returns `FallbackReason::AllTemplatesFailed`. Attach structured log via `makepad_widgets::log!` on every fallback event. | closes `test_fallback_template_id_succeeds`, `test_fallback_plain_text`, `test_fallback_does_not_oscillate` |
| C.5 | `src/home/app_registry/mod.rs::render_app_envelope_to_splash` — **no control-flow change**. Only update the existing `None`-fall-back log to include the `FallbackReason::AllTemplatesFailed` context when available (log-field-only change). `AppLookup::VersionMismatch` path stays exactly as today; new scenario `test_unsupported_version_bypasses_template_fallback` verifies this. | closes `test_unsupported_version_bypasses_template_fallback` |
| C.6 | Meta-check scenarios — `test_no_provenance_storage_new_modules` + `test_no_new_cargo_dependencies`. `test_no_provenance_storage_new_modules`: assert no module path `home::app_registry::render_receipt` / `home::app_registry::provenance` / `home::app_registry::audit` resolves. `test_no_new_cargo_dependencies`: the **transitive crate count** (from `Cargo.lock`) at the end of this plan must equal the baseline recorded at plan start (adding `sha2` as a **direct** dep in Cargo.toml does NOT grow the transitive set — verified by `cargo tree --duplicates` showing zero new entries). Pin the baseline in a comment inside this test so future dep additions are reviewed. | closes `test_no_provenance_storage_new_modules`, `test_no_new_cargo_dependencies` |

**Gate → Slice D:**
- All 17 template-runtime spec scenarios pass under `cargo test --lib`.
- Fallback hierarchy correctly logs structured reasons; no silent degrade.
- `mod.rs::render_app_envelope_to_splash` control flow textually identical to today (diff contains only log-field additions in the existing `None` branch).

### Slice D — Documentation sync (DONE)

| Step | File | Change |
|---|---|---|
| D.1 | `docs/design/agent-to-app-design.md` §7.1 Shipped | Add: "Template preflight (W5/W7/attribution/binding-path)", "TemplateHandle cache + compatibility matrix", "Fallback hierarchy (template → plain text)", "FallbackReason + ValidationError error shape". |
| D.2 | `docs/design/agent-to-app-design.md` §7.3 Not yet shipped | Remove items shipped in this plan. Keep L2 actions, L3 stateful, generated templates, skill-ui packaging, multi-version rendering. |
| D.3 | `specs/task-agent-to-app-splash-host-evolution.spec.md` | Add cross-reference note to the template-runtime spec in Decisions sections that mention preflight / cache / fallback. No decision change, just pointer. |
| D.4 | `2026-04-21-splash-host-evolution-implementation-plan.md` | Update Step 1.6 + 1.7 rows to point at this plan's Slice A/B/C rather than restating the logic. |
| D.5 | This plan | Mark Status as implemented in branch; add commit hashes only after user testing and an actual commit. |

**Gate → merge:** Slice D has landed in the current branch alongside Slice C.
No doc drift allowed before commit/PR.

## Scenarios Coverage Map

Spec scenario → plan step (for traceability). All 17 template-runtime scenarios (4 preflight + 1 build-audit + 4 cache + 1 compat-table + 4 fallback + 1 error-shape + 2 meta):

| Scenario | Slice.Step |
|---|---|
| `test_preflight_rejects_unlisted_widget` | A.4 |
| `test_preflight_rejects_unlisted_local_function` | A.4 |
| `test_preflight_rejects_attribution_override` | A.4 |
| `test_preflight_rejects_binding_path_not_in_schema` | A.5 |
| `test_all_templates_pass_preflight_at_build_time` | A.6 |
| `test_cache_hit_skips_parse` | B.3 |
| `test_cache_miss_on_template_hash_change` | B.3 |
| `test_cache_miss_on_manifest_version_change` | B.3 |
| `test_cache_miss_on_host_version_change` | B.3 |
| `test_breaking_change_table_self_consistent` | B.4 |
| `test_fallback_template_id_succeeds` | C.4 |
| `test_fallback_plain_text` | C.4 |
| `test_unsupported_version_bypasses_template_fallback` | C.5 |
| `test_fallback_does_not_oscillate` | C.4 |
| `test_host_error_to_validation_error` | C.2 |
| `test_no_provenance_storage_new_modules` | C.6 |
| `test_no_new_cargo_dependencies` | C.6 |

All 17 spec scenarios bound to concrete steps. No orphan scenarios.

## Risks & Mitigations

- **R1 — Splash DSL parser in A.3.** v1 reuses Makepad 2.0 tokenizer;
  if the tokenizer's API surface is too deep to use out-of-context,
  fall back to a hand-rolled recursive-descent over the subset of DSL
  tokens the manifest / registry care about (widget names, field names,
  binding sites, function-call sites, attribution-region markers). **Do
  not** introduce a full Splash parser clone; only parse what preflight
  needs.
- **R2 — sha256 via workspace-transitive `sha2`.** Spec mandates
  sha256-truncated hashing. `sha2` is already a transitive dep in
  Cargo.lock via matrix-sdk; step B.2 adds a **direct** `sha2` entry in
  Cargo.toml but introduces **no new transitive crate**. Verify with
  `cargo tree --duplicates` before/after to prove dep tree growth is
  zero. If review deems even a new direct-dep line unacceptable, pivot
  to invoking `sha2` through an existing robrix-side reexport (if one
  emerges during review). Do not substitute `DefaultHasher` — that
  violates the spec's "weak hash forbidden" clause.
- **R3 — `BindingPathNotInSchema` requires each `Capability` to know
  its schema paths.** v1 introduces `CapabilitySchema` trait; implementing
  it for `WeatherFactory` is trivial (static list of paths). For dynamic
  schemas (future), this trait becomes the natural extension point.
- **R4 — Meta-check scenarios C.6.** Testing "no new modules added" is
  awkward from inside the test runner. v1 encodes invariants
  structurally (assert no such module path resolves); the richer
  diff-based check lands with CI tooling in a later task.
- **R5 — Interaction with in-progress splash-host-evolution Slice 1.**
  This plan's Slice A/B/C *replace* the old Steps 1.6-1.7. Writers of
  those two steps in the splash-host-evolution plan must now defer to
  this plan's slices. D.4 formalizes that redirect.

## What This Plan Does NOT Cover

- Full render receipt / provenance storage — future spec
- Skill-with-UI packaging — `task-agent-to-app-ui-packaging.spec.md`
- Multi-version rendering (`AppFactory::init` knowing version to try multiple paths) — `task-agent-to-app-multi-version-rendering.spec.md` (TBD)
- Template-Author LLM repair loop — opens with first generative capability
- `tracing` crate migration — separate observability task
- Cache eviction / LRU — future perf task
- Cross-language template variant selection — existing language slot mechanism unchanged

## Commit & PR Strategy

- **One PR per slice.** Slice A PR (~6-8 files touched), Slice B PR (~3-4), Slice C PR (~4-6), Slice D PR (~4 doc files).
- **All changes single-repo (robrix2).** No OctOS-side changes.
- **Each PR must cite**: template-runtime spec scenarios closed + cross-reference to splash-host-evolution plan (to confirm Step 1.6/1.7 redirect).

## Timeline

| Slice | Estimate | Dependencies |
|---|---|---|
| Slice A | 1d | splash-host-evolution Steps 1.1-1.5 complete (1.1-1.3a DONE; 1.4-1.5 ≈0.5d first) |
| Slice B | 1d | Slice A gate passed |
| Slice C | 0.5d | Slice B gate passed |
| Slice D | 0.5d | Slice C gate passed |

**Total:** 3d (matches spec `estimate: 3d`).

Combined with splash-host-evolution remaining work (Steps 1.4-1.5 + 1.8-1.11 + Slice 2 + Slice 3) still yields the original 5-6d total for the full agent-to-app Splash-first delivery.
