# Agent-to-App Producer Routing — Implementation Plan

**Date:** 2026-04-17
**Status:** Planning (specs approved, no code yet)
**Owner:** cross-repo (Robrix + OctOS)

## Inputs

This plan is generated from three approved task specs. Raw `agent-spec plan`
outputs (Contract + Task Sketch per spec) are saved alongside this document:

| Spec | Raw plan file | Estimate |
|---|---|---|
| [`task-agent-to-app-producer-routing`](../../specs/task-agent-to-app-producer-routing.spec.md) | [plan-producer-routing](./2026-04-17-plan-producer-routing.contract.md) | 3d |
| [`task-agent-to-app-l1-weather-card`](../../specs/task-agent-to-app-l1-weather-card.spec.md) (with addendum) | [plan-weather-l1](./2026-04-17-plan-weather-l1.contract.md) | 2d |
| [`task-agent-to-app-l1-weather-v2-doc-sync`](../../specs/task-agent-to-app-l1-weather-v2-doc-sync.spec.md) | [plan-weather-v2-doc-sync](./2026-04-17-plan-weather-v2-doc-sync.contract.md) | 0.5d |

Dependency graph: [`2026-04-17-agent-to-app-deps.svg`](./2026-04-17-agent-to-app-deps.svg)
(source: [`2026-04-17-agent-to-app-deps.dot`](./2026-04-17-agent-to-app-deps.dot)).

Generate commands used (verified against `agent-spec 0.2.7`; if a local
`agent-spec-tool-first` reference doc doesn't list these subcommands, the
reference is out of date — run `agent-spec --help` to confirm):

```bash
agent-spec --version                                   # expected: agent-spec 0.2.7 or newer
agent-spec plan specs/<spec>.spec.md --format text     # produces Contract + Task Sketch
agent-spec graph --spec-dir specs --format svg         # produces dependency SVG
agent-spec graph --spec-dir specs --format dot         # produces dependency DOT
```

**Note on raw `plan-*.contract.md` files:** Each begins with a block of
`warning: Allowed Changes path not found: crates/octos-agent/...`. These are
**expected** cross-repo side effects: producer-routing's Allowed Changes list
OctOS paths (`/Users/zhangalex/Work/Projects/FW/octos/...`), but `agent-spec
plan` resolves paths against the cwd (this repo). The warnings are not
configuration errors; they reflect the documented cross-repo boundary in
[Cross-Repo Boundary](#cross-repo-boundary) below. Leaving the warnings in
the raw files preserves the verbatim tool output.

## Scope Summary

Turn the agent→app pipeline from "LLM tool-calls `show_weather_card` when it
guesses right" into "resolver extracts `(capability_id, focus, slots)` from
structured output, deterministic dispatcher routes to capability registry,
capability produces paired `body` + `initial_state`".

User-visible improvement: prompts like "今天北京穿什么 / 带伞吗 / 户外
适合吗" will all dispatch to the `weather_guidance` capability with the right
`focus`, not only direct "今天天气如何" queries.

**No `org.octos.app` protocol change.** **No Robrix consumer code change.**
All code work is in the `octos` sibling repo. Spec edits are in this repo.

## Cross-Repo Boundary

| Repo | What changes |
|---|---|
| **robrix2** (this) | 3 spec files (`producer-routing`, `weather-card` addendum, `weather-v2-doc-sync`) and this roadmap. **No Rust changes.** |
| **octos** (`/Users/zhangalex/Work/Projects/FW/octos`) | New `capabilities/` + `resolver/` modules; `show_weather_card` becomes a thin adapter; `session_actor` + `gateway_default.txt` wired; new fixture under `tests/resolver_fixtures/`. |

## Phase Plan

### Phase 0 — Weather L1 v2 doc-sync (0.5d, Robrix-only, unblocks Phase 1 references)

Owner: `task-agent-to-app-l1-weather-v2-doc-sync` (9 scenarios).

| Step | Action | Acceptance |
|---|---|---|
| 0.1 | Add v2 optional fields to L1 §Weather type JSON schema: `high_c`, `low_c`, `uv_index_max`, `precipitation_probability_max`, `periods` (with sub-schema) | `test_l1_spec_declares_all_v2_optional_fields`, `test_l1_spec_periods_entry_defines_subschema` |
| 0.2 | Update §校验规则 to record `periods` silent-truncate-to-3 and `periods[].condition` silent-fallback-to-sunny (no warning log) | `test_l1_spec_periods_entry_defines_subschema` |
| 0.3 | Remove the "v2 已在代码实现但本 spec 暂未承认的字段级 schema" block from L1 §Out of Scope | `test_l1_spec_out_of_scope_removes_v2_field_gap` |
| 0.4 | Collapse or update §Schema 扩展 addendum's "未对齐声明" paragraph | `test_l1_addendum_no_longer_disclaims_v2_alignment` |
| 0.5 | Re-lint L1 spec, confirm 100% | `test_lint_quality_unchanged_after_doc_sync` |

**Gate → Phase 1:** L1 spec text reflects code; `agent-spec lint` stays at
100%; `cargo test` unchanged (no code touched).

### Phase 1 — OctOS scaffold (≈1d, no user-visible behavior yet)

Owner: `task-agent-to-app-producer-routing` scenarios grouped by structure
(lines in `plan-producer-routing` Task Sketch).

Order matters — 1.1→1.4 build the abstractions, 1.5 fills the first concrete
capability, 1.6 preserves backward compat, 1.7–1.8 wire into the message
pipeline.

| Step | New/Edited | Scenarios unlocked / bound |
|---|---|---|
| 1.1 | `crates/octos-agent/src/capabilities/mod.rs` — `Capability` trait + `CapabilityRegistry` + `SlotSchema` + default `min_confidence()=0.6`. **Infrastructure only**; no spec scenario closes here. Locally verified by inline `#[cfg(test)]` unit tests (`should_return_capability_by_id_when_registered`, `should_default_min_confidence_to_zero_point_six`, etc.) | — (primitives that 1.4 will use to close `test_dispatch_routes_only_known_capability`) |
| 1.2 | `crates/octos-agent/src/resolver/mod.rs` — result struct types + JSON schema + fixture loader | unlocks `test_resolver_output_is_structured_json` (after 1.3 wires the LLM call) and `test_dispatcher_snapshot_replays_deterministically` (after 1.4 wires the dispatcher) |
| 1.3 | resolver LLM call (structured-output mode; no multi-tool free pick) | closes `test_resolver_output_is_structured_json`, `test_resolver_timeout_degrades_gracefully`, `test_resolver_invalid_json_degrades` |
| 1.4 | `crates/octos-agent/src/dispatcher.rs` — resolver→registry→capability pipeline + all four fallback paths | closes `test_dispatch_routes_only_known_capability`, `test_low_confidence_falls_back_to_text`, `test_unknown_focus_rejected_before_emit`, `test_missing_slot_triggers_reask`, `test_focus_omitted_when_default`, `test_dispatcher_snapshot_replays_deterministically` |
| 1.5 | `crates/octos-agent/src/capabilities/weather_guidance.rs` — `build_state` + `build_body` sharing one `data` fetch; 4 focuses | `test_state_and_body_share_one_data_fetch`, `test_capability_invocation_produces_paired_body_and_state`, `test_capability_does_not_emit_actions`, `test_optional_focus_does_not_bump_version` |
| 1.6 | `crates/octos-agent/src/tools/show_weather_card.rs` — thin adapter forwarding to `weather_guidance` | backward-compat smoke: existing call sites unchanged |
| 1.7 | `crates/octos-cli/src/prompts/gateway_default.txt` — describe resolver's fixed JSON schema; remove any free tool-choice steering for weather | manual prompt review |
| 1.8 | `crates/octos-cli/src/session_actor.rs` — resolver-first pipeline; on miss/low-confidence fall through to legacy tool-calling. **Runtime consumption of `DispatchDecision` lands here**: translate `AppReply { initial_state, body }` into the Matrix `OutboundMessage` envelope, and translate `PlainTextReply(text)` into a plain-text reply without envelope. Until this step lands, dispatcher-level scenarios (`test_missing_slot_triggers_reask` etc.) are only unit-closed — no end-to-end user-visible behavior. | integration: sentence in → correct path out (envelope, plain-text re-ask, or LLM fall-through) |

**Gate → Phase 2:** all Phase-1 scenarios pass at unit level. The two
fixture-dependent scenarios (`test_weather_guidance_regression_fixture_passes`,
`test_gate_failure_keeps_capability_disabled`) are still red here — that's
Phase 2's job.

### Phase 2 — Robustness fixture + gate (≈1d)

| Step | Action | Acceptance |
|---|---|---|
| 2.1 | Build fixture: ≥ 80 in-domain phrasings (4 focuses × ≥ 10 EN × ≥ 10 zh) under `crates/octos-cli/tests/resolver_fixtures/weather_guidance/` | count check in harness |
| 2.2 | Build negative examples (non-weather queries) under `.../negatives/` | count check |
| 2.3 | Harness runs fixture against the live resolver LLM, computes top-1 accuracy and false-positive rate | `test_weather_guidance_regression_fixture_passes` passes: top-1 ≥ 90%, FPR = 0% |
| 2.4 | Wire startup gating: if fixture run fails, `weather_guidance` is registered but `disabled=true`; dispatcher falls through to `show_weather_card` legacy tool | `test_gate_failure_keeps_capability_disabled` |

**Gate → Phase 3:** the two fixture scenarios green. If gate fails, **do
not** ship Phase 1 as default — land it behind the `disabled` flag only.

### Phase 3 — End-to-end verification (time-boxed, with user)

Real-environment smoke on the bot:

1. "今天北京天气" → card with `focus=overview`
2. "今天北京穿什么" → card with `focus=clothing`
3. "今天北京要带伞吗" → card with `focus=umbrella`
4. "北京适合户外吗" → card with `focus=outdoor`
5. "北京" (ambiguous, one word) → plain text (low confidence)
6. "今天是几号" (negative) → plain text (capability miss)
7. (If a non-app channel like Telegram/CLI is configured) verify `body`
   matches card state for all of the above — exercising the same-source
   invariant.

**Sign-off:** user confirms all seven expected outcomes. Only then → commit /
PR.

## Verification Checkpoints (per spec)

- Producer-routing: 15 scenarios → 13 unit-level (Phase 1) + 2 fixture-level
  (Phase 2). Run `agent-spec verify` against OctOS test output when
  cross-repo adapter is ready.
- Weather L1: after Phase 0 edit + Phase 1.5 (`focus` gets emitted in
  practice), the existing `test_payload_language_overrides_app_language_for_guidance_card`
  scenario gets real coverage — currently it passes because consumer code
  supports payload `language`; after Phase 1.5 it also gets exercised
  end-to-end via producer emitting `language`.
- Weather v2 doc-sync: 9 scenarios, all Phase 0.

## Out of Scope (explicit, per specs)

These are documented exclusions; do **not** absorb into this plan:

- Deleting `show_weather_card` altogether — separate `show_weather_card
  removal` task after Phase 2 gate passes.
- Additional capabilities (`news_guidance`, `calendar_guidance`, …) — each
  gets its own task spec.
- Weather L1 v3 schema — separate v3 spec after v2 settles.
- `org.octos.actions` integration with capabilities — capabilities remain
  action-free; buttons stay on the Phase 4c path.
- Multi-turn capability state — v1 resolves each turn independently.
- Capability auto-discovery / dynamic registration — all compiled in.

## Risks & Open Questions

| Risk | Mitigation |
|---|---|
| Fixture accuracy < 90% after careful prompt engineering | Phase 2 gate keeps legacy `show_weather_card` live; Phase 1 code sits behind the flag until fixture passes. No regression. |
| LLM provider rate limits during fixture runs | Cache recorded LLM responses in snapshot files; `test_dispatcher_snapshot_replays_deterministically` verifies dispatcher determinism independent of live LLM. |
| OctOS PRs land before Robrix spec PRs | Robrix specs are advisory during OctOS implementation; merge specs first, then OctOS adapters, to avoid "code exists but spec doesn't justify it". |
| `focus` semantics drift between producer and L1 consumer | Producer-routing spec's `Unknown focus from producer is rejected before emit` scenario and L1's `Unknown focus` fallback rule (in addendum) together form the invariant — both checked by their respective suites. |

## Recommended Starting Point

Phase 0 first. It's Robrix-only, purely documentation, and immediately
closes the "v2 schema undeclared" gap Codex flagged across rounds. After
user sign-off on Phase 0, open the OctOS work on a fresh branch.
