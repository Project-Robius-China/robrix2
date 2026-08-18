//! CI policy conformance (spec: task-ci-test-and-ux-gate, Rules ci-1..ci-4).
//!
//! These tests read repository files rather than exercising the app: they
//! pin the *shape* of the CI gate so a future edit cannot silently stop
//! running tests or drop the UX audit.

use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read(rel: &str) -> String {
    let p = root().join(rel);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {}: {e}", p.display()))
}

/// The `spec_gate` job body: from its `spec_gate:` key to the next top-level job.
fn spec_gate_job(workflow: &str) -> String {
    let start = workflow.find("\n  spec_gate:").expect("main.yml has a spec_gate job");
    let rest = &workflow[start + 1..];
    // Next job starts at a two-space-indented key that is not deeper.
    let mut end = rest.len();
    for (i, line) in rest.match_indices('\n') {
        let _ = line;
        let after = &rest[i + 1..];
        if after.starts_with("  ") && !after.starts_with("   ") && !after.starts_with("  spec_gate:") {
            end = i;
            break;
        }
    }
    rest[..end].to_string()
}

/// Command lines of the job: `run: <cmd>` one-liners and the bodies of
/// `run: |` blocks, comments stripped. `name:` and other keys are excluded.
fn run_lines(job: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut in_block = false;
    let mut block_indent = 0usize;
    for line in job.lines() {
        let indent = line.len() - line.trim_start().len();
        let t = line.trim();
        if in_block {
            if t.is_empty() || indent > block_indent {
                if !t.starts_with('#') { out.push(t.to_string()); }
                continue;
            }
            in_block = false;
        }
        if let Some(rest) = t.strip_prefix("run:") {
            let rest = rest.trim();
            if rest == "|" || rest == ">" || rest == "|-" {
                in_block = true;
                block_indent = indent;
            } else if !rest.is_empty() {
                out.push(rest.to_string());
            }
        }
    }
    out
}

#[test]
fn ci_workflow_runs_all_cargo_test_targets() {
    let wf = read(".github/workflows/main.yml");
    let job = spec_gate_job(&wf);
    let lines = run_lines(&job);
    let test_line = lines
        .iter()
        .find(|l| l.contains("cargo test"))
        .expect("spec_gate must run cargo test");
    assert!(
        test_line.contains("--workspace"),
        "cargo test must cover the whole workspace, got: {test_line}"
    );
    assert!(
        !test_line.contains("--lib"),
        "cargo test must not be restricted to --lib (bins + doctests must run), got: {test_line}"
    );
    // The job is a normal PR check: not workflow_dispatch-only, not draft-only.
    assert!(job.contains("if: github.event.pull_request.draft == false"));
    assert!(!wf.contains("workflow_dispatch"), "main.yml must stay push/pull_request driven");
}

#[test]
fn ci_workflow_runs_ux_harness_static_gate() {
    let wf = read(".github/workflows/main.yml");
    let job = spec_gate_job(&wf);
    let lines = run_lines(&job);
    let audit = lines
        .iter()
        .find(|l| l.contains("ux-harness") && l.contains(" static "))
        .expect("spec_gate must run the ux-harness static audit");
    assert!(
        audit.contains("--gate tools/ux-harness/gate.json"),
        "static audit must be gated by tools/ux-harness/gate.json, got: {audit}"
    );
    assert!(root().join("tools/ux-harness/gate.json").is_file());
    // The workflow re-runs when the harness or its policy changes.
    assert!(wf.contains("- tools/**"), "main.yml paths must include tools/**");
}

#[test]
fn shipped_gate_policy_is_a_ratchet_baseline() {
    let text = read("tools/ux-harness/gate.json");
    let v: serde_json::Value = serde_json::from_str(&text).expect("gate.json parses");
    assert_eq!(v["unlisted"], "fail");
    let rules = v["rules"].as_object().expect("rules object");
    for zero in ["i18n.missing-key", "visual.hardcoded-color", "legibility.token-contrast"] {
        assert_eq!(rules[zero]["max"], 0, "{zero} must be 0");
    }
    assert_eq!(rules["i18n.untranslated-screen"]["max"], 37);
    assert_eq!(rules["i18n.untranslated-value"]["max"], 4);
    for (rule, rp) in rules {
        assert!(
            rp["note"].as_str().map(|n| !n.trim().is_empty()).unwrap_or(false),
            "{rule} needs a note explaining its threshold"
        );
    }
}

#[test]
fn fixed_doctests_do_not_use_ignore() {
    for rel in [
        "src/logout/logout_state_machine.rs",
        "src/room/room_display_filter.rs",
        "src/shared/confirmation_modal.rs",
        "src/utils.rs",
    ] {
        let src = read(rel);
        for (i, line) in src.lines().enumerate() {
            let t = line.trim_start();
            let is_doc = t.starts_with("///") || t.starts_with("//!");
            if is_doc && t.contains("```") && t.contains("ignore") {
                panic!("{rel}:{}: doctest fence uses `ignore`: {line}", i + 1);
            }
        }
    }
    let _ = Path::new(".");
}
