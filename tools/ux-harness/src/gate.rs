//! Pass/fail gate over harness findings.
//!
//! Scoring stays informative (a 9.2 is not a verdict); the gate is the
//! separate, explicit policy CI enforces: a JSON file listing, per rule id,
//! the maximum total `count` that is tolerated. Thresholds are a ratchet —
//! they record today's baseline and are only meant to go down.
//!
//! ```json
//! {
//!   "unlisted": "fail",
//!   "rules": {
//!     "i18n.missing-key":        { "max": 0,  "note": "every en key must exist in every locale" },
//!     "i18n.untranslated-screen": { "max": 37, "note": "baseline 2026-08-19; lower as screens are translated" }
//!   }
//! }
//! ```

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::findings::Finding;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Unlisted {
    /// A rule with findings that the policy does not mention fails the gate.
    Fail,
    /// Such a rule is reported but does not fail the gate.
    Warn,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RulePolicy {
    pub max: usize,
    #[serde(default)]
    pub note: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GatePolicy {
    pub unlisted: Unlisted,
    pub rules: BTreeMap<String, RulePolicy>,
}

impl GatePolicy {
    pub fn parse(json: &str) -> Result<GatePolicy, String> {
        serde_json::from_str(json).map_err(|e| format!("gate policy: {e}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowStatus {
    Ok,
    Exceeded,
    Unlisted,
}

#[derive(Debug, Clone)]
pub struct GateRow {
    pub rule: String,
    pub count: usize,
    pub max: Option<usize>,
    pub status: RowStatus,
}

#[derive(Debug, Clone)]
pub struct GateOutcome {
    pub rows: Vec<GateRow>,
    pub failures: Vec<String>,
}

impl GateOutcome {
    pub fn passed(&self) -> bool {
        self.failures.is_empty()
    }
}

/// Sum `count` per rule id and compare against the policy.
pub fn evaluate(policy: &GatePolicy, findings: &[Finding]) -> GateOutcome {
    let mut totals: BTreeMap<String, usize> = BTreeMap::new();
    for f in findings {
        *totals.entry(f.rule.clone()).or_insert(0) += f.count;
    }

    let mut rows = Vec::new();
    let mut failures = Vec::new();

    // Every listed rule gets a row, even with zero findings, so the table
    // shows what the gate is watching.
    for (rule, rp) in &policy.rules {
        let count = totals.get(rule).copied().unwrap_or(0);
        let status = if count > rp.max { RowStatus::Exceeded } else { RowStatus::Ok };
        if status == RowStatus::Exceeded {
            let note = if rp.note.trim().is_empty() { String::new() } else { format!(" — {}", rp.note.trim()) };
            failures.push(format!("{rule}: {count} > {}{note}", rp.max));
        }
        rows.push(GateRow { rule: rule.clone(), count, max: Some(rp.max), status });
    }
    for (rule, count) in &totals {
        if policy.rules.contains_key(rule) {
            continue;
        }
        if policy.unlisted == Unlisted::Fail {
            failures.push(format!("{rule}: {count} finding(s) for a rule not covered by the gate policy"));
        }
        rows.push(GateRow { rule: rule.clone(), count: *count, max: None, status: RowStatus::Unlisted });
    }

    GateOutcome { rows, failures }
}

pub fn render(outcome: &GateOutcome, unlisted: &Unlisted) -> String {
    let mut s = String::from("\n## Gate\n\n| Rule | Count | Max | Status |\n|---|---:|---:|---|\n");
    for r in &outcome.rows {
        let max = r.max.map(|m| m.to_string()).unwrap_or_else(|| "—".to_string());
        let status = match r.status {
            RowStatus::Ok => "ok",
            RowStatus::Exceeded => "FAIL (exceeds max)",
            RowStatus::Unlisted => match unlisted {
                Unlisted::Fail => "FAIL (unlisted rule)",
                Unlisted::Warn => "warn (unlisted rule)",
            },
        };
        s.push_str(&format!("| `{}` | {} | {} | {} |\n", r.rule, r.count, max, status));
    }
    if outcome.passed() {
        s.push_str("\nGate: **passed**\n");
    } else {
        s.push_str("\nGate: **FAILED**\n");
        for f in &outcome.failures {
            s.push_str(&format!("- {f}\n"));
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::findings::{Dimension, Severity};

    fn policy(json: &str) -> GatePolicy {
        GatePolicy::parse(json).unwrap()
    }
    fn finding(rule: &str, count: usize) -> Finding {
        Finding::new(rule, Dimension::Internationalization, Severity::Low, "x", "y", "z").with_count(count)
    }

    #[test]
    fn gate_fails_when_listed_rule_exceeds_max() {
        let p = policy(r#"{"unlisted":"warn","rules":{"i18n.missing-key":{"max":0}}}"#);
        let out = evaluate(&p, &[finding("i18n.missing-key", 2)]);
        assert!(!out.passed());
        assert!(out.failures[0].starts_with("i18n.missing-key: 2 > 0"), "{:?}", out.failures);
        assert_eq!(out.rows[0].status, RowStatus::Exceeded);
    }

    #[test]
    fn gate_passes_when_counts_within_max() {
        let p = policy(r#"{"unlisted":"fail","rules":{"i18n.untranslated-screen":{"max":37}}}"#);
        let out = evaluate(&p, &[finding("i18n.untranslated-screen", 37)]);
        assert!(out.passed(), "{:?}", out.failures);
        // Two findings for the same rule are summed.
        let out = evaluate(&p, &[finding("i18n.untranslated-screen", 20), finding("i18n.untranslated-screen", 18)]);
        assert!(!out.passed());
        // A listed rule with no findings still gets an `ok` row.
        let out = evaluate(&p, &[]);
        assert!(out.passed());
        assert_eq!(out.rows.len(), 1);
        assert_eq!(out.rows[0].count, 0);
    }

    #[test]
    fn gate_unlisted_rule_respects_policy() {
        let strict = policy(r#"{"unlisted":"fail","rules":{}}"#);
        let out = evaluate(&strict, &[finding("visual.something-new", 1)]);
        assert!(!out.passed());
        assert!(out.failures[0].starts_with("visual.something-new:"));
        assert_eq!(out.rows[0].status, RowStatus::Unlisted);

        let lenient = policy(r#"{"unlisted":"warn","rules":{}}"#);
        let out = evaluate(&lenient, &[finding("visual.something-new", 1)]);
        assert!(out.passed());
        assert_eq!(out.rows[0].status, RowStatus::Unlisted);
    }

    #[test]
    fn shipped_gate_policy_is_a_ratchet_baseline() {
        let json = include_str!("../gate.json");
        let p = policy(json);
        assert_eq!(p.unlisted, Unlisted::Fail);
        for zero in ["i18n.missing-key", "visual.hardcoded-color", "legibility.token-contrast"] {
            assert_eq!(p.rules.get(zero).map(|r| r.max), Some(0), "{zero} must be 0");
        }
        assert_eq!(p.rules["i18n.untranslated-screen"].max, 37);
        assert_eq!(p.rules["i18n.untranslated-value"].max, 4);
        // Every rule carries a note explaining the threshold.
        for (rule, rp) in &p.rules {
            assert!(!rp.note.trim().is_empty(), "{rule} needs a note");
        }
    }
}
