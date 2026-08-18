spec: task
name: "CI Test And UX Gate — Run All Test Targets And The Static UX Audit With A Ratchet Policy On Every PR"
inherits: project
tags: [ci, infra, i18n, a11y, ux-harness]
estimate: 0.5d
---

## Intent

Fix GitHub issue #311: a green CI run does not currently prove that the ~674
`#[test]`s compile and pass, that doctests build, that design tokens meet
WCAG AA, or that locale catalogs are complete. PR #324 introduced the
`spec_gate` job with `cargo test --lib`; this task finishes the issue:

- every Cargo test target (lib, bins, doctests) compiles and runs on PRs;
- the five known doctest failures are fixed, not quarantined;
- the `tools/ux-harness` static audit runs on PRs and **fails the job**
  according to an explicit, versioned ratchet policy (`tools/ux-harness/gate.json`);
- the required checks and the policy are documented for contributors.

## Constraints

- Do not run `cargo fmt` / `rustfmt`
- Do not add dependencies to the root crate; `tools/ux-harness` may not add dependencies either
- Do not weaken or delete any existing test to make CI green
- Do not change harness scoring semantics; the gate is a separate pass/fail layer over findings
- Doctests must be fixed by correcting the examples (imports, syntax, current API); `ignore` is not an accepted fix
- The gate policy is a ratchet: thresholds may only be lowered in future PRs unless a PR explicitly justifies raising one

## Decisions

- `Finding` gains a `count: usize` field (default 1) so aggregate rules
  (`i18n.untranslated-screen`, `i18n.untranslated-value`, `i18n.missing-key`,
  `visual.hardcoded-color`, `legibility.token-contrast`) expose the number of
  offending files/keys/literals/pairs; the gate compares counts, not severities
- New harness flag `--gate <policy.json>` for `static` and `run`; the policy is
  `{ "rules": { "<rule-id>": { "max": N, "note": "..." } }, "unlisted": "fail" | "warn" }`;
  the harness exits with code 3 when any listed rule's total count exceeds
  `max`, or when an unlisted rule produced findings and `unlisted == "fail"`; it
  prints a `rule | count | max | status` table before exiting
- `tools/ux-harness/gate.json` ships with: `i18n.missing-key` 0,
  `visual.hardcoded-color` 0, `legibility.token-contrast` 0,
  `i18n.untranslated-screen` 37 (current baseline), `i18n.untranslated-value` 4
  (current baseline), `unlisted: "fail"`
- CI job `spec_gate` runs `cargo test --workspace` (all targets of both crates, replacing `cargo test --lib`)
  and then `cargo run -q -p ux-harness -- static --repo . --out target/ux-audit --gate tools/ux-harness/gate.json`
- Policy conformance is itself tested: `tests/ci_policy.rs` (root crate
  integration test, no new deps) checks the workflow file and `gate.json`
  structure; the harness gets unit tests for the gate evaluation
- The five doctests are fixed in place: `logout_state_machine.rs` (state-flow
  diagram fenced as `text`; usage wrapped in a hidden async fn with `use`),
  `room_display_filter.rs` (example updated to the current
  `set_filter_criteria` / `FilterableRoom::room_name()` API), `confirmation_modal.rs`
  (missing commas + `use`), `utils.rs` (`use` line)
- Contributor documentation lives in `CLAUDE.md` (Build & Test) and
  `tools/ux-harness/README.md` (gate section)
- `tools/ux-harness` becomes a member of a root `[workspace]` (no new
  dependencies; its `png`/`serde_json` versions already match the root lockfile),
  so `cargo test --workspace` runs its unit tests and agent-spec `Package: ux-harness`
  selectors resolve; its private `Cargo.lock` and `[profile.release]` are removed

## Boundaries

### Allowed Changes
- `.github/workflows/main.yml`
- `tools/ux-harness/src/main.rs`
- `tools/ux-harness/src/findings.rs`
- `tools/ux-harness/src/rules_static.rs`
- `tools/ux-harness/src/rules_runtime.rs`
- `tools/ux-harness/gate.json`
- `tools/ux-harness/README.md`
- `tests/ci_policy.rs`
- `src/logout/logout_state_machine.rs`
- `src/room/room_display_filter.rs`
- `src/shared/confirmation_modal.rs`
- `src/utils.rs`
- ./CLAUDE.md
- ./Cargo.toml
- ./Cargo.lock
- `tools/ux-harness/Cargo.toml`
- `tools/ux-harness/Cargo.lock`
- `tools/ux-harness/src/gate.rs`
- `specs/task-ci-test-and-ux-gate.spec.md`

### Forbidden
- Do not modify `scripts/spec-guard.sh` semantics (this task adds CI steps beside it, not inside it)
- Do not modify `src/shared/design_tokens.rs` or `resources/i18n/**` (baselines are recorded, not "fixed", here)
- Do not add `#[ignore]` or ```` ```ignore ```` to make tests or doctests pass
- Do not run `cargo fmt`

## Acceptance Criteria

<!--
Invariants:
  ci-1  ∀ PR: cargo test (lib ∧ bins ∧ doctests) runs and must pass  — "green main" ⇒ tests compiled and passed
  ci-2  ∀ PR: harness static audit runs with gate.json; ∀ rule r listed: count(r) ≤ max(r); unlisted rule with findings ⇒ fail
  ci-3  gate.json is a ratchet: max values are the current baseline (37 / 4) or 0
  ci-4  doctests are executable examples of the current API (no `ignore`)
-->

### Rule: ci-1 — Every test target runs on PRs

Scenario: Workflow runs all Cargo test targets
  Tags: critical
  Test: ci_workflow_runs_all_cargo_test_targets
  Given `.github/workflows/main.yml`
  When the `spec_gate` job steps are inspected
  Then a step runs `cargo test --workspace` without restricting to `--lib`
  And the job is not gated behind draft-only or manual triggers

Scenario: Design-token WCAG contrast is enforced by a unit test
  Test: semantic_token_pairs_meet_wcag_aa
  Given the semantic `RBX_*` foreground/background pairs
  When contrast ratios are computed
  Then every pair meets WCAG AA

### Rule: ci-2 — Static UX audit gates PRs by an explicit policy

Scenario: Workflow runs the static UX audit with the gate policy
  Tags: critical
  Test: ci_workflow_runs_ux_harness_static_gate
  Given `.github/workflows/main.yml`
  When the `spec_gate` job steps are inspected
  Then a step runs `ux-harness static` with `--gate tools/ux-harness/gate.json`

Scenario: Gate fails when a listed rule exceeds its maximum
  Tags: critical
  Test:
    Package: ux-harness
    Filter: gate_fails_when_listed_rule_exceeds_max
  Given a policy `{ "i18n.missing-key": max 0 }` and a finding for `i18n.missing-key` with count 2
  When the gate is evaluated
  Then the result is a failure naming `i18n.missing-key` with `2 > 0`

Scenario: Gate passes when every listed rule is within its maximum
  Test:
    Package: ux-harness
    Filter: gate_passes_when_counts_within_max
  Given a policy `{ "i18n.untranslated-screen": max 37 }` and a finding with count 37
  When the gate is evaluated
  Then the result is a pass

Scenario: Unlisted rules fail the gate when the policy says so
  Test:
    Package: ux-harness
    Filter: gate_unlisted_rule_respects_policy
  Given a finding for rule `visual.something-new` not present in the policy
  When the gate is evaluated with `unlisted: "fail"`
  Then the result is a failure naming the unlisted rule
  And with `unlisted: "warn"` the result is a pass

Scenario: Aggregate findings carry their counts
  Test:
    Package: ux-harness
    Filter: aggregate_findings_expose_counts
  Given a source scan with 3 DSL files lacking translation calls out of 10
  And a locale dictionary missing 2 keys and repeating 1 English value
  When the static rules run
  Then the `i18n.untranslated-screen` finding has count 3
  And the `i18n.missing-key` finding has count 2
  And the `i18n.untranslated-value` finding has count 1

### Rule: ci-3 — The policy is a ratchet baseline

Scenario: The shipped policy matches the recorded baseline
  Test:
    Package: ux-harness
    Filter: shipped_gate_policy_is_a_ratchet_baseline
  Given `tools/ux-harness/gate.json`
  When it is parsed
  Then `unlisted` is `fail`
  And `i18n.missing-key`, `visual.hardcoded-color`, `legibility.token-contrast` have max 0
  And `i18n.untranslated-screen` has max 37 and `i18n.untranslated-value` has max 4

Scenario: The current tree passes the shipped policy
  Test: manual_test_static_audit_passes_shipped_gate
  Given the repository at this commit
  When `cargo run -q -p ux-harness -- static --repo . --out target/ux-audit --gate tools/ux-harness/gate.json` runs
  Then it exits 0 and prints every listed rule as `ok`

### Rule: ci-4 — Doctests are live examples

Scenario: All doctests compile and pass
  Tags: critical
  Test: manual_test_cargo_test_doc_passes
  Given the crate at this commit
  When `cargo test --doc` runs
  Then it reports 0 failed and 0 ignored beyond the pre-existing single ignored example
  And no doctest in the five previously failing files is marked `ignore`

Scenario: Fixed doctests do not use `ignore`
  Test: fixed_doctests_do_not_use_ignore
  Given `src/logout/logout_state_machine.rs`, `src/room/room_display_filter.rs`, `src/shared/confirmation_modal.rs`, `src/utils.rs`
  When their doc comments are scanned
  Then no code fence uses the `ignore` attribute

## Out Of Scope

- Running the full headless UX audit (`ux-harness run`) on every PR (scheduled/opt-in later)
- Actually translating the 37 untranslated DSL files or the 4 identical values (separate i18n work; the ratchet records them)
- Extending clippy/test compilation to mobile targets
- ux-harness dependency licensing (#312)
