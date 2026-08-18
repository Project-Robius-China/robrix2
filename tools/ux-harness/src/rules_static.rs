//! Rules that read the repository rather than the running app.
//!
//! These cover the parts of UX a screenshot cannot show: whether the color
//! system is actually one system, whether every string can be translated, and
//! whether the contrast of the *design tokens themselves* clears WCAG before a
//! single pixel is drawn. They also mean the harness still produces a score on
//! a machine that cannot build the app.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::findings::{Dimension, Finding, Severity};
use crate::frame::{contrast_ratio, parse_hex, relative_luminance};

/// Semantic foreground/background pairs the design system promises will be
/// used together, with the size class of the text that sits on them.
/// `large` follows WCAG: ≥18.66px bold or ≥24px, which at robrix2's type scale
/// only the page title reaches — so almost everything here is normal text.
const TOKEN_PAIRS: &[(&str, &str, bool, &str)] = &[
    ("RBX_FG_PRIMARY", "RBX_BG_CANVAS", false, "body text on the page canvas"),
    ("RBX_FG_PRIMARY", "RBX_BG_SURFACE", false, "body text on a card"),
    ("RBX_FG_SECONDARY", "RBX_BG_SURFACE", false, "subtitles / meta on a card"),
    ("RBX_FG_SECONDARY", "RBX_BG_CANVAS", false, "subtitles / meta on the canvas"),
    ("RBX_FG_TERTIARY", "RBX_BG_SURFACE", false, "timestamps / captions on a card"),
    ("RBX_FG_TERTIARY", "RBX_BG_CANVAS", false, "timestamps / captions on the canvas"),
    ("RBX_FG_ON_ACCENT", "RBX_ACCENT", false, "label on the primary CTA"),
    ("RBX_FG_ON_ACCENT", "RBX_ACCENT_HOVER", false, "label on a hovered CTA"),
    ("RBX_LINK", "RBX_BG_SURFACE", false, "inline link on a card"),
    ("RBX_ACCENT", "RBX_ACCENT_SOFT", false, "accent badge / selected chip"),
    ("RBX_SUCCESS_FG", "RBX_SUCCESS_BG", false, "success badge"),
    ("RBX_WARNING_FG", "RBX_WARNING_BG", false, "warning / pending badge"),
    ("RBX_DANGER_FG", "RBX_DANGER_BG", false, "danger badge"),
    ("RBX_INFO_FG", "RBX_INFO_BG", false, "info / capability chip"),
    ("RBX_NEUTRAL_FG", "RBX_NEUTRAL_BG", false, "neutral / idle badge"),
    ("RBX_NAV_FG", "RBX_NAV_BG", false, "idle item in the desktop nav rail"),
    ("RBX_NAV_FG_ACTIVE", "RBX_NAV_BG", false, "active item in the desktop nav rail"),
    ("RBX_CODE_FG", "RBX_CODE_BG", false, "code panel body"),
    ("RBX_CODE_KEYWORD", "RBX_CODE_BG", false, "code panel keyword"),
    ("RBX_CODE_STRING", "RBX_CODE_BG", false, "code panel string"),
    ("RBX_CODE_COMMENT", "RBX_CODE_BG", false, "code panel comment"),
];

/// Parse `mod.widgets.NAME = #xRRGGBB[AA]` out of the token module.
pub fn parse_tokens(design_tokens_rs: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in design_tokens_rs.lines() {
        let line = line.trim();
        if !line.starts_with("mod.widgets.") {
            continue;
        }
        let Some((lhs, rhs)) = line.split_once('=') else { continue };
        let name = lhs.trim().trim_start_matches("mod.widgets.").trim().to_string();
        let value = rhs.trim();
        let value = value.split("//").next().unwrap_or(value).trim();
        if !value.starts_with("#x") {
            continue;
        }
        out.insert(name, value.trim_start_matches("#x").to_string());
    }
    out
}

pub fn check_token_contrast(tokens: &BTreeMap<String, String>) -> Vec<Finding> {
    let mut out = Vec::new();
    for (fg_name, bg_name, large, usage) in TOKEN_PAIRS {
        let (Some(fg_hex), Some(bg_hex)) = (tokens.get(*fg_name), tokens.get(*bg_name)) else {
            continue;
        };
        let (Some(fg), Some(bg)) = (parse_hex(fg_hex), parse_hex(bg_hex)) else { continue };
        let ratio = contrast_ratio(
            relative_luminance(fg.0, fg.1, fg.2),
            relative_luminance(bg.0, bg.1, bg.2),
        );
        let required = if *large { 3.0 } else { 4.5 };
        if ratio >= required {
            continue;
        }
        // Below 3:1 nothing is readable at any size; between 3:1 and 4.5:1 it
        // is readable as a heading but not as body copy.
        let severity = if ratio < 3.0 { Severity::High } else { Severity::Medium };
        out.push(Finding::new(
            "legibility.token-contrast",
            Dimension::Legibility,
            severity,
            format!("Token pair fails WCAG AA ({required:.1}:1) for {usage}"),
            format!("{fg_name} on {bg_name}"),
            format!("#{fg_hex} on #{bg_hex} = {ratio:.2}:1"),
        ));
    }
    out
}

pub struct SourceScan {
    pub hardcoded_hex: Vec<(String, usize, String)>,
    pub dsl_files_without_i18n: Vec<String>,
    pub total_dsl_files: usize,
}

/// Walk `src/`, counting raw hex literals outside the token modules and DSL
/// files that never call the translation layer.
pub fn scan_sources(src_dir: &Path) -> Result<SourceScan, String> {
    let mut hardcoded_hex = Vec::new();
    let mut dsl_files_without_i18n = Vec::new();
    let mut total_dsl_files = 0usize;

    let files = collect_rs_files(src_dir)?;
    for path in files {
        let rel = path
            .strip_prefix(src_dir)
            .unwrap_or(&path)
            .to_string_lossy()
            .to_string();
        // The token modules are where colors are *supposed* to be literal.
        let is_token_module = rel.ends_with("design_tokens.rs") || rel.ends_with("styles.rs");
        let Ok(text) = std::fs::read_to_string(&path) else { continue };

        if !is_token_module {
            for (i, line) in text.lines().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") {
                    continue;
                }
                // `mod.widgets.NAME = #xRRGGBB` *declares* a token. Declaring
                // one necessarily spells out a literal; the rule is about
                // screens bypassing the token layer, not about the layer
                // itself.
                if trimmed.starts_with("mod.widgets.") && trimmed.contains('=') {
                    continue;
                }
                if let Some(hex) = find_hex_literal(line) {
                    hardcoded_hex.push((format!("src/{rel}"), i + 1, hex));
                }
            }
        }

        if text.contains("script_mod!") {
            total_dsl_files += 1;
            let has_user_text = text
                .lines()
                .any(|l| user_facing_text_literal(l).is_some());
            let uses_i18n = text.contains("tr_key(") || text.contains("tr_fmt(") || text.contains("I18nKey::");
            if has_user_text && !uses_i18n {
                dsl_files_without_i18n.push(format!("src/{rel}"));
            }
        }
    }

    hardcoded_hex.sort();
    dsl_files_without_i18n.sort();
    Ok(SourceScan { hardcoded_hex, dsl_files_without_i18n, total_dsl_files })
}

/// `#xRRGGBB` or `#xRRGGBBAA` used as a color value.
fn find_hex_literal(line: &str) -> Option<String> {
    let idx = line.find("#x")?;
    let rest = &line[idx + 2..];
    let hex: String = rest
        .chars()
        .take_while(|c| c.is_ascii_hexdigit())
        .collect();
    if hex.len() == 6 || hex.len() == 8 {
        Some(format!("#x{hex}"))
    } else {
        None
    }
}

/// A DSL `text: "Some words"` that a user will read. Single tokens that are
/// obviously not prose (icon glyphs, punctuation, format placeholders) are
/// skipped so the count reflects real translation debt.
fn user_facing_text_literal(line: &str) -> Option<String> {
    let idx = line.find("text:")?;
    let rest = line[idx + 5..].trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    let value = &rest[..end];
    if value.len() < 2 {
        return None;
    }
    if !value.chars().any(|c| c.is_ascii_alphabetic()) {
        return None;
    }
    Some(value.to_string())
}

fn collect_rs_files(dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let entries = std::fs::read_dir(&d)
            .map_err(|e| format!("read_dir {}: {e}", d.display()))?;
        for entry in entries {
            let Ok(entry) = entry else { continue };
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

pub fn check_visual_consistency(scan: &SourceScan) -> Vec<Finding> {
    let mut out = Vec::new();
    if scan.hardcoded_hex.is_empty() {
        return out;
    }
    // One stray hex is a slip; dozens mean the token layer is not actually the
    // source of truth, which is what the project's own spec (§0.1) forbids.
    let count = scan.hardcoded_hex.len();
    let files: BTreeSet<&str> = scan.hardcoded_hex.iter().map(|(f, _, _)| f.as_str()).collect();
    let severity = match count {
        0 => return out,
        1..=5 => Severity::Low,
        6..=25 => Severity::Medium,
        _ => Severity::High,
    };
    let sample: Vec<String> = scan
        .hardcoded_hex
        .iter()
        .take(4)
        .map(|(f, l, h)| format!("{f}:{l} {h}"))
        .collect();
    out.push(Finding::new(
        "visual.hardcoded-color",
        Dimension::VisualConsistency,
        severity,
        "Screens hardcode raw hex colors instead of design tokens",
        format!("{} file(s) under src/", files.len()),
        format!("{count} literal(s); e.g. {}", sample.join(", ")),
    ).with_count(count));
    out
}

/// Screens whose copy never reaches the translation layer.
pub fn check_i18n_coverage(scan: &SourceScan) -> Vec<Finding> {
    let mut out = Vec::new();

    if !scan.dsl_files_without_i18n.is_empty() {
        let n = scan.dsl_files_without_i18n.len();
        let severity = if n * 3 >= scan.total_dsl_files.max(1) {
            Severity::Medium
        } else {
            Severity::Low
        };
        out.push(Finding::new(
            "i18n.untranslated-screen",
            Dimension::Internationalization,
            severity,
            "UI files ship user-facing copy that never goes through the translation layer",
            format!("{n} of {} DSL file(s)", scan.total_dsl_files),
            scan.dsl_files_without_i18n
                .iter()
                .take(5)
                .cloned()
                .collect::<Vec<_>>()
                .join(", "),
        ).with_count(n));
    }
    out
}

/// One translated dictionary against the English source of truth.
pub fn check_locale_dict(locale: &str, en: &Dict, dict: &Dict) -> Vec<Finding> {
    let mut out = Vec::new();

    let missing: Vec<&String> = en.keys().filter(|k| !dict.contains_key(*k)).collect();
    if !missing.is_empty() {
        out.push(Finding::new(
            "i18n.missing-key",
            Dimension::Internationalization,
            Severity::Medium,
            format!("Keys present in en are missing from {locale}"),
            format!("resources/i18n/{locale}.json"),
            format!(
                "{} missing, e.g. {}",
                missing.len(),
                missing.iter().take(3).map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            ),
        ).with_count(missing.len()));
    }

    let identical: Vec<&String> = en
        .iter()
        .filter(|(k, v)| dict.get(*k).map(|o| o == *v).unwrap_or(false))
        .filter(|(_, v)| !looks_locale_neutral(v))
        .map(|(k, _)| k)
        .collect();
    if !identical.is_empty() {
        out.push(Finding::new(
            "i18n.untranslated-value",
            Dimension::Internationalization,
            Severity::Low,
            format!("Strings in {locale} are byte-identical to English"),
            format!("resources/i18n/{locale}.json"),
            format!(
                "{} key(s), e.g. {}",
                identical.len(),
                identical.iter().take(3).map(|s| s.as_str()).collect::<Vec<_>>().join(", ")
            ),
        ).with_count(identical.len()));
    }
    out
}

/// Proper nouns, URLs and single symbols are supposed to be identical across
/// locales; counting them as untranslated would be noise.
fn looks_locale_neutral(value: &str) -> bool {
    let v = value.trim();
    if v.is_empty() {
        return true;
    }
    if v.starts_with("http") || v.starts_with('@') || v.starts_with('#') {
        return true;
    }
    if !v.chars().any(|c| c.is_ascii_alphabetic()) {
        return true;
    }
    // Single capitalised word with no spaces: almost always a brand or a code.
    !v.contains(' ') && v.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
}

pub type Dict = BTreeMap<String, String>;

pub fn load_dict(path: &Path) -> Result<Dict, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("read {}: {e}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| format!("parse {}: {e}", path.display()))?;
    let obj = value
        .as_object()
        .ok_or_else(|| format!("{} is not a JSON object", path.display()))?;
    Ok(obj
        .iter()
        .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregate_findings_expose_counts() {
        let scan = SourceScan {
            hardcoded_hex: vec![
                ("src/a.rs".into(), 1, "#ff0000".into()),
                ("src/a.rs".into(), 2, "#00ff00".into()),
            ],
            dsl_files_without_i18n: vec!["src/a.rs".into(), "src/b.rs".into(), "src/c.rs".into()],
            total_dsl_files: 10,
        };
        let f = check_i18n_coverage(&scan);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule, "i18n.untranslated-screen");
        assert_eq!(f[0].count, 3);

        let f = check_visual_consistency(&scan);
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].rule, "visual.hardcoded-color");
        assert_eq!(f[0].count, 2);

        let mut en = Dict::new();
        en.insert("a".into(), "Hello there".into());
        en.insert("b".into(), "Good morning".into());
        en.insert("c".into(), "See you soon".into());
        en.insert("d".into(), "Take care now".into());
        let mut zh = Dict::new();
        zh.insert("a".into(), "你好".into());
        zh.insert("d".into(), "Take care now".into()); // identical → untranslated value; b, c missing
        let f = check_locale_dict("zh-CN", &en, &zh);
        let missing = f.iter().find(|x| x.rule == "i18n.missing-key").expect("missing-key finding");
        assert_eq!(missing.count, 2, "keys b and c are missing");
        let identical = f.iter().find(|x| x.rule == "i18n.untranslated-value").expect("untranslated-value finding");
        assert_eq!(identical.count, 1);
    }
}
