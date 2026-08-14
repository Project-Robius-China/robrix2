//! Findings, the scoring rubric, and report rendering.
//!
//! The score is deliberately mechanical: every dimension starts at 10 and loses
//! a fixed amount per finding by severity. No judgement is applied at scoring
//! time, so two runs of the same build always produce the same number and a
//! diff in the score is always traceable to a diff in the findings.

use std::collections::BTreeMap;
use std::fmt::Write as _;

use serde::Serialize;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Critical,
    High,
    Medium,
    Low,
}

impl Severity {
    pub fn penalty(self) -> f64 {
        match self {
            Severity::Critical => 3.0,
            Severity::High => 1.5,
            Severity::Medium => 0.6,
            Severity::Low => 0.2,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Severity::Critical => "critical",
            Severity::High => "high",
            Severity::Medium => "medium",
            Severity::Low => "low",
        }
    }
}

/// The eight axes the score is built from. Each maps to something a user
/// actually feels, not to a code-quality abstraction.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Dimension {
    /// Nothing is clipped, off-screen, zero-sized, or stacked ambiguously.
    LayoutIntegrity,
    /// Every control is big enough to hit, on the platform it ships to.
    TargetSize,
    /// Text is readable against what is behind it.
    Legibility,
    /// The UI answers input, and answers it quickly.
    Feedback,
    /// The app is usable without a mouse, and shows where focus is.
    KeyboardFocus,
    /// loading / empty / error / offline are designed, not accidental.
    StateCoverage,
    /// One visual system: tokens, not per-screen constants.
    VisualConsistency,
    /// Translatable, and still laid out correctly once translated.
    Internationalization,
}

impl Dimension {
    pub fn all() -> [Dimension; 8] {
        [
            Dimension::LayoutIntegrity,
            Dimension::TargetSize,
            Dimension::Legibility,
            Dimension::Feedback,
            Dimension::KeyboardFocus,
            Dimension::StateCoverage,
            Dimension::VisualConsistency,
            Dimension::Internationalization,
        ]
    }
    pub fn title(self) -> &'static str {
        match self {
            Dimension::LayoutIntegrity => "Layout integrity",
            Dimension::TargetSize => "Target size",
            Dimension::Legibility => "Legibility & contrast",
            Dimension::Feedback => "Feedback & responsiveness",
            Dimension::KeyboardFocus => "Keyboard & focus",
            Dimension::StateCoverage => "State coverage",
            Dimension::VisualConsistency => "Visual consistency",
            Dimension::Internationalization => "Internationalization",
        }
    }
    /// Weights lean toward the things that block a user outright.
    pub fn weight(self) -> f64 {
        match self {
            Dimension::LayoutIntegrity => 1.5,
            Dimension::TargetSize => 1.0,
            Dimension::Legibility => 1.5,
            Dimension::Feedback => 1.5,
            Dimension::KeyboardFocus => 1.5,
            Dimension::StateCoverage => 1.5,
            Dimension::VisualConsistency => 1.0,
            Dimension::Internationalization => 1.5,
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct Finding {
    pub rule: String,
    pub dimension: Dimension,
    pub severity: Severity,
    /// What is wrong, in one sentence.
    pub summary: String,
    /// Where: a scene name, a widget id, or a repo-relative path.
    pub locus: String,
    /// The measurement that produced the finding, so it can be re-checked.
    pub evidence: String,
}

impl Finding {
    pub fn new(
        rule: &str,
        dimension: Dimension,
        severity: Severity,
        summary: impl Into<String>,
        locus: impl Into<String>,
        evidence: impl Into<String>,
    ) -> Finding {
        Finding {
            rule: rule.to_string(),
            dimension,
            severity,
            summary: summary.into(),
            locus: locus.into(),
            evidence: evidence.into(),
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct DimensionScore {
    pub dimension: Dimension,
    pub title: String,
    /// `None` when no rule for this dimension actually ran. An unexercised
    /// dimension is *not* a passing one, so it is excluded from the average
    /// rather than silently counted as 10.
    pub score: Option<f64>,
    pub weight: f64,
    pub findings: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub overall: f64,
    pub dimensions: Vec<DimensionScore>,
    pub findings: Vec<Finding>,
    pub scenes: Vec<SceneRecord>,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Default)]
pub struct SceneRecord {
    pub name: String,
    pub viewport: String,
    pub widgets: usize,
    pub visible_widgets: usize,
    pub frame: Option<String>,
    pub settle_ticks: usize,
}

/// `exercised` lists the dimensions whose rules actually ran this session.
pub fn score(
    findings: &[Finding],
    exercised: &[Dimension],
    scenes: Vec<SceneRecord>,
    notes: Vec<String>,
) -> Report {
    let mut penalties: BTreeMap<Dimension, f64> = BTreeMap::new();
    let mut counts: BTreeMap<Dimension, usize> = BTreeMap::new();
    for f in findings {
        *penalties.entry(f.dimension).or_insert(0.0) += f.severity.penalty();
        *counts.entry(f.dimension).or_insert(0) += 1;
    }

    let mut dimensions = Vec::new();
    let mut weighted_sum = 0.0;
    let mut weight_total = 0.0;
    for d in Dimension::all() {
        let ran = exercised.contains(&d);
        let score = if ran {
            let penalty = penalties.get(&d).copied().unwrap_or(0.0);
            let s = (10.0 - penalty).max(0.0);
            weighted_sum += s * d.weight();
            weight_total += d.weight();
            Some(round1(s))
        } else {
            None
        };
        dimensions.push(DimensionScore {
            dimension: d,
            title: d.title().to_string(),
            score,
            weight: d.weight(),
            findings: counts.get(&d).copied().unwrap_or(0),
        });
    }

    let overall = if weight_total > 0.0 {
        round1(weighted_sum / weight_total)
    } else {
        0.0
    };

    let mut findings = findings.to_vec();
    findings.sort_by(|a, b| {
        a.severity
            .cmp(&b.severity)
            .then_with(|| a.dimension.cmp(&b.dimension))
            .then_with(|| a.rule.cmp(&b.rule))
    });

    Report { overall, dimensions, findings, scenes, notes }
}

fn round1(v: f64) -> f64 {
    (v * 10.0).round() / 10.0
}

pub fn render_markdown(report: &Report) -> String {
    let mut out = String::new();
    let scored = report.dimensions.iter().filter(|d| d.score.is_some()).count();
    let _ = writeln!(
        out,
        "# Robrix2 UX score: {:.1} / 10\n\n_{} of {} dimensions exercised._\n",
        report.overall,
        scored,
        report.dimensions.len()
    );

    let _ = writeln!(out, "| Dimension | Score | Weight | Findings |");
    let _ = writeln!(out, "|---|---:|---:|---:|");
    for d in &report.dimensions {
        let score = match d.score {
            Some(s) => format!("{s:.1}"),
            None => "not run".to_string(),
        };
        let _ = writeln!(
            out,
            "| {} | {} | {:.1} | {} |",
            d.title, score, d.weight, d.findings
        );
    }

    if !report.scenes.is_empty() {
        let _ = writeln!(out, "\n## Scenes exercised\n");
        let _ = writeln!(out, "| Scene | Viewport | Widgets (visible) | Settle ticks | Frame |");
        let _ = writeln!(out, "|---|---|---:|---:|---|");
        for s in &report.scenes {
            let _ = writeln!(
                out,
                "| {} | {} | {} ({}) | {} | {} |",
                s.name,
                s.viewport,
                s.widgets,
                s.visible_widgets,
                s.settle_ticks,
                s.frame.clone().unwrap_or_else(|| "—".to_string())
            );
        }
    }

    let _ = writeln!(out, "\n## Findings\n");
    if report.findings.is_empty() {
        let _ = writeln!(out, "None.");
    } else {
        let _ = writeln!(out, "| Severity | Rule | Dimension | Where | What | Evidence |");
        let _ = writeln!(out, "|---|---|---|---|---|---|");
        for f in &report.findings {
            let _ = writeln!(
                out,
                "| {} | `{}` | {} | {} | {} | {} |",
                f.severity.label(),
                f.rule,
                f.dimension.title(),
                escape_pipes(&f.locus),
                escape_pipes(&f.summary),
                escape_pipes(&f.evidence),
            );
        }
    }

    if !report.notes.is_empty() {
        let _ = writeln!(out, "\n## Notes\n");
        for n in &report.notes {
            let _ = writeln!(out, "- {n}");
        }
    }
    out
}

fn escape_pipes(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}
