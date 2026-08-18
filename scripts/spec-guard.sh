#!/usr/bin/env bash
# Spec regression gate for robrix2 (pre-commit + CI).
#
# What it enforces (and what it deliberately does not):
#   1. `agent-spec lint --min-score 0.7` on specs changed in this change set.
#      Legacy specs are not re-linted, so old lint debt does not block PRs.
#   2. Structural guards from specs/structure-guards.txt (`agent-spec check-structure`).
#   3. Capability specs (specs/capabilities/*.spec.md) must fully pass
#      (`agent-spec lifecycle`, no skips), then every ADR they satisfy must be
#      `honored` (`agent-spec trace --gate`).
#   4. Task specs. Robrix2 task specs bind UI / homeserver scenarios to
#      `manual_test_*` selectors, which agent-spec reports as `skip`, and the
#      boundary layer applies the whole change set to every spec, so a naive
#      `agent-spec guard` fails on every PR. Instead:
#        - specs changed in this change set are the active contracts: run
#          `lifecycle` WITH the change set (boundaries + tests), fail on failed>0;
#        - all other specs are regression checks: run `verify` WITHOUT a change
#          set (tests only), fail on failed>0. Skips are tolerated.
#
# Usage:
#   scripts/spec-guard.sh [--change-scope staged|worktree] [--base <git-ref>] [--fast]
#     --change-scope  local mode: git staged (default) or worktree changes
#     --base <ref>    CI mode: change set = `git diff --name-only <ref>...HEAD`
#     --fast          steps 1-2 only (cheap pre-commit path)
set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

SCOPE="staged"; BASE=""; FAST=0
while [ $# -gt 0 ]; do
  case "$1" in
    --change-scope) SCOPE="$2"; shift 2;;
    --base) BASE="$2"; shift 2;;
    --fast) FAST=1; shift;;
    *) echo "unknown arg: $1" >&2; exit 64;;
  esac
done

if ! command -v agent-spec >/dev/null 2>&1; then
  echo "spec-guard: agent-spec CLI not found (cargo install agent-spec --locked)" >&2
  exit 127
fi

# ---- change set ---------------------------------------------------------------
if [ -n "$BASE" ]; then
  mapfile -t CHANGED < <(git diff --name-only "$BASE...HEAD" --diff-filter=ACMR)
elif [ "$SCOPE" = "worktree" ]; then
  mapfile -t CHANGED < <({ git diff --name-only --diff-filter=ACMR; git diff --name-only --cached --diff-filter=ACMR; git ls-files --others --exclude-standard; } | sort -u)
else
  mapfile -t CHANGED < <(git diff --name-only --cached --diff-filter=ACMR)
fi
CHANGED_SPECS=()
for f in "${CHANGED[@]:-}"; do
  case "$f" in specs/*.spec.md|specs/*/*.spec.md) [ -f "$f" ] && CHANGED_SPECS+=("$f");; esac
done

fail=0
say() { printf '\n== %s\n' "$*"; }

# ---- 1. lint changed specs -----------------------------------------------------
say "1/4 lint changed specs (${#CHANGED_SPECS[@]})"
for s in "${CHANGED_SPECS[@]:-}"; do
  [ -n "$s" ] || continue
  if ! agent-spec lint "$s" --min-score 0.7; then echo "spec-guard: lint failed: $s"; fail=1; fi
done

# ---- 2. structural guards ------------------------------------------------------
say "2/4 structural guards (specs/structure-guards.txt)"
if [ -f specs/structure-guards.txt ]; then
  while IFS='|' read -r forbid glob; do
    forbid="$(echo "$forbid" | sed 's/^ *//; s/ *$//')"; glob="$(echo "$glob" | sed 's/^ *//; s/ *$//')"
    [ -z "$forbid" ] && continue; case "$forbid" in \#*) continue;; esac
    if ! agent-spec check-structure --code . --forbid "$forbid" --in "$glob"; then fail=1; fi
  done < specs/structure-guards.txt
fi

if [ "$FAST" = "1" ]; then
  [ "$fail" = "0" ] && echo "spec-guard (fast): OK" || echo "spec-guard (fast): FAILED"
  exit "$fail"
fi

# ---- 3. capability specs + ADR liveness ---------------------------------------
say "3/4 capability specs must fully pass; satisfied ADRs must be honored"
mkdir -p .agent-spec/runs
for cap in specs/capabilities/*.spec.md; do
  [ -f "$cap" ] || continue
  if ! agent-spec lifecycle "$cap" --code . --run-log-dir .agent-spec/runs >/dev/null 2>&1; then
    echo "spec-guard: capability spec not fully passing: $cap"
    agent-spec lifecycle "$cap" --code . --run-log-dir .agent-spec/runs 2>&1 | grep -E '"verdict": "(fail|skip)"' -B6 | grep -E 'scenario_name|reason' | head -20 || true
    fail=1
  else
    echo "ok: $cap"
  fi
  # ADR ids from `satisfies:` frontmatter
  for adr in $(sed -n 's/^satisfies:[[:space:]]*\[\(.*\)\]/\1/p' "$cap" | tr ',' ' '); do
    if ! agent-spec trace "$adr" --gate; then echo "spec-guard: liveness gate failed for $adr"; fail=1; fi
  done
done

# ---- 4. task specs ---------------------------------------------------------------
# 4a. Changed specs are the active contracts of this change: verify them WITH the
#     change set (boundaries + bound tests). Manual `skip`s are tolerated; any
#     `failed > 0` (test or boundary violation) fails the gate.
# 4b. Every other spec is a regression check: verify WITHOUT a change set (no
#     boundary layer, since foreign files would trivially violate them) and fail
#     only on `failed > 0`.
say "4/4 task specs: changed = contract check (with change set), others = regression (tests only)"
CHANGE_ARGS=()
for f in "${CHANGED[@]:-}"; do [ -n "$f" ] && [ -e "$f" ] && CHANGE_ARGS+=(--change "$f"); done
summary_failed() { grep -Eo '"failed": [0-9]+' | head -1 | grep -Eo '[0-9]+'; }
is_changed_spec() { local x; for x in "${CHANGED_SPECS[@]:-}"; do [ "$x" = "$1" ] && return 0; done; return 1; }
for spec in specs/*.spec.md; do
  case "$spec" in specs/project.spec.md) continue;; esac
  if is_changed_spec "$spec"; then
    out="$(agent-spec lifecycle "$spec" --code . --run-log-dir .agent-spec/runs "${CHANGE_ARGS[@]}" --format json 2>&1 || true)"
    n="$(echo "$out" | summary_failed || echo "?")"
    if [ "$n" != "0" ]; then
      echo "FAIL (contract): $spec — failed=$n"
      echo "$out" | grep -E '"reason": "not covered|"verdict": "fail"' -B3 | grep -E 'reason|step_text|scenario_name' | head -12
      fail=1
    else
      echo "ok (contract): $spec"
    fi
  else
    out="$(agent-spec verify "$spec" --code . --format json 2>&1 || true)"
    n="$(echo "$out" | summary_failed || echo "?")"
    if [ "$n" != "0" ]; then
      echo "FAIL (regression): $spec — failed=$n"
      echo "$out" | grep -E '"verdict": "fail"' -B8 | grep -E 'scenario_name|reason' | head -8
      fail=1
    fi
  fi
done

if [ "$fail" = "0" ]; then echo; echo "spec-guard: OK"; else echo; echo "spec-guard: FAILED"; fi
exit "$fail"
