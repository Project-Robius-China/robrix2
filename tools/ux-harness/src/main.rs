//! Robrix2 UX harness.
//!
//! Two modes:
//!
//!   ux-harness static --repo <dir>
//!       Scores what can be measured from the source tree alone: design-token
//!       contrast, token discipline, translation coverage. Runs anywhere.
//!
//!   ux-harness run --repo <dir> --app <headless-binary>
//!       Additionally launches the app through makepad's headless backend,
//!       drives it with synthetic events over the studio stdin protocol,
//!       captures the frames it renders, and scores the result.
//!
//! Build the app for `run` with makepad's CPU renderer:
//!
//!   MAKEPAD=headless cargo build --release
//!
//! See README.md for the full loop.

mod driver;
mod findings;
mod frame;
mod proto;
mod rules_runtime;
mod rules_static;
mod scenes;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use findings::{render_markdown, score, Dimension, Finding, SceneRecord};
use frame::{contrast_hex, contrast_ratio, parse_hex, relative_luminance};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mode = args.get(1).map(|s| s.as_str()).unwrap_or("");

    let mut repo = PathBuf::from(".");
    let mut app: Option<PathBuf> = None;
    let mut out = PathBuf::from("target/ux-audit");
    let mut fg = String::new();
    let mut bg = String::new();
    let mut target = 4.5f64;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--fg" => {
                fg = args.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--bg" => {
                bg = args.get(i + 1).cloned().unwrap_or_default();
                i += 2;
            }
            "--target" => {
                target = args
                    .get(i + 1)
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(4.5);
                i += 2;
            }
            "--repo" => {
                repo = PathBuf::from(args.get(i + 1).cloned().unwrap_or_default());
                i += 2;
            }
            "--app" => {
                app = args.get(i + 1).map(PathBuf::from);
                i += 2;
            }
            "--out" => {
                out = PathBuf::from(args.get(i + 1).cloned().unwrap_or_default());
                i += 2;
            }
            other => {
                eprintln!("unknown argument: {other}");
                std::process::exit(2);
            }
        }
    }

    match mode {
        "static" => {
            if let Err(e) = run(&repo, None, &out) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        "run" => {
            let Some(app) = app else {
                eprintln!("`run` needs --app <path to MAKEPAD=headless binary>");
                std::process::exit(2);
            };
            if let Err(e) = run(&repo, Some(&app), &out) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        "contrast" => {
            if let Err(e) = contrast_command(&fg, &bg, target) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!(
                "usage:\n  \
                 ux-harness static --repo <dir> [--out <dir>]\n  \
                 ux-harness run --repo <dir> --app <bin> [--out <dir>]\n  \
                 ux-harness contrast --fg #RRGGBB --bg #RRGGBB [--target 4.5]"
            );
            std::process::exit(2);
        }
    }
}

fn run(repo: &Path, app: Option<&Path>, out: &Path) -> Result<(), String> {
    std::fs::create_dir_all(out).map_err(|e| format!("create {}: {e}", out.display()))?;

    let mut all: Vec<Finding> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let mut scenes_run: Vec<SceneRecord> = Vec::new();
    // The static pass can only speak to these three.
    let mut exercised: Vec<Dimension> = vec![
        Dimension::Legibility,
        Dimension::VisualConsistency,
        Dimension::Internationalization,
    ];

    // ---- static pass -------------------------------------------------------
    let tokens_path = repo.join("src/shared/design_tokens.rs");
    match std::fs::read_to_string(&tokens_path) {
        Ok(text) => {
            let tokens = rules_static::parse_tokens(&text);
            notes.push(format!("{} design tokens parsed from {}", tokens.len(), rel(repo, &tokens_path)));
            all.extend(rules_static::check_token_contrast(&tokens));
        }
        Err(e) => notes.push(format!("design tokens not read ({e}) — token contrast rules skipped")),
    }

    let src_dir = repo.join("src");
    match rules_static::scan_sources(&src_dir) {
        Ok(scan) => {
            notes.push(format!(
                "{} DSL file(s) scanned; {} raw hex literal(s) outside the token layer",
                scan.total_dsl_files,
                scan.hardcoded_hex.len()
            ));
            all.extend(rules_static::check_visual_consistency(&scan));

            all.extend(rules_static::check_i18n_coverage(&scan));

            let en_path = repo.join("resources/i18n/en.json");
            match rules_static::load_dict(&en_path) {
                Ok(en) => {
                    let mut dicts: BTreeMap<String, rules_static::Dict> = BTreeMap::new();
                    if let Ok(entries) = std::fs::read_dir(repo.join("resources/i18n")) {
                        for entry in entries.flatten() {
                            let p = entry.path();
                            let Some(stem) = p.file_stem().map(|s| s.to_string_lossy().to_string()) else {
                                continue;
                            };
                            if stem == "en" || p.extension().map(|e| e != "json").unwrap_or(true) {
                                continue;
                            }
                            if let Ok(d) = rules_static::load_dict(&p) {
                                dicts.insert(stem, d);
                            }
                        }
                    }
                    for (locale, dict) in &dicts {
                        all.extend(rules_static::check_locale_dict(locale, &en, dict));
                    }
                    notes.push(format!(
                        "{} locale dictionary/dictionaries compared against en ({} keys)",
                        dicts.len(),
                        en.len()
                    ));
                }
                Err(e) => notes.push(format!("en.json not read ({e}) — i18n rules skipped")),
            }
        }
        Err(e) => notes.push(format!("src/ not scanned ({e}) — source rules skipped")),
    }

    // ---- runtime pass ------------------------------------------------------
    match app {
        None => {
            notes.push(
                "Runtime pass skipped (no --app): layout, target-size, rendered-contrast, \
                 feedback and focus rules did not run, and those dimensions are reported \
                 as not run rather than folded into the score."
                    .to_string(),
            );
        }
        Some(app_bin) => {
            let frames_dir = out.join("frames");
            // Point the app at a throwaway HOME so an audit can never read or
            // overwrite the developer's real Matrix session, and give the
            // headless shader JIT its own cache directory (it shells out to
            // rustc, so it needs the toolchain on PATH — inherited).
            let home = out.join("home");
            let jit = out.join("jit");
            std::fs::create_dir_all(&home).map_err(|e| format!("create {}: {e}", home.display()))?;
            let env = vec![
                ("HOME".to_string(), home.to_string_lossy().to_string()),
                ("MAKEPAD_HEADLESS_JIT_DIR".to_string(), jit.to_string_lossy().to_string()),
            ];
            let mut driver = driver::Driver::launch(app_bin, &frames_dir, &env)?;
            driver.wait_for_startup()?;
            let outcome = scenes::run_all(&mut driver, &frames_dir)?;
            driver.shutdown();
            all.extend(outcome.findings);
            scenes_run.extend(outcome.records);
            exercised.extend([
                Dimension::LayoutIntegrity,
                Dimension::TargetSize,
                Dimension::Feedback,
                Dimension::KeyboardFocus,
                Dimension::StateCoverage,
            ]);
            notes.push(format!("frames written to {}", frames_dir.display()));
        }
    }

    let report = score(&all, &exercised, scenes_run, notes);

    let md = render_markdown(&report);
    let md_path = out.join("ux-report.md");
    std::fs::write(&md_path, &md).map_err(|e| format!("write {}: {e}", md_path.display()))?;

    let json_path = out.join("ux-report.json");
    let json = serde_json::to_string_pretty(&report)
        .map_err(|e| format!("serialize report: {e}"))?;
    std::fs::write(&json_path, json).map_err(|e| format!("write {}: {e}", json_path.display()))?;

    println!("{md}");
    println!("\nwrote {} and {}", md_path.display(), json_path.display());
    Ok(())
}

/// Report the contrast of a pair, and — when it fails — the nearest compliant
/// color along the same hue. Darkening (or lightening) in a straight line
/// toward black/white keeps the palette's hue relationships intact, which is
/// what the visual spec's "one meaning = one color" rule needs.
fn contrast_command(fg: &str, bg: &str, target: f64) -> Result<(), String> {
    let ratio = contrast_hex(fg, bg).ok_or("could not parse --fg / --bg as #RRGGBB")?;
    println!("{fg} on {bg} = {ratio:.2}:1 (target {target:.1}:1)");
    if ratio >= target {
        println!("passes");
        return Ok(());
    }

    let (fr, fg_, fb) = parse_hex(fg).ok_or("bad --fg")?;
    let (br, bg_, bb) = parse_hex(bg).ok_or("bad --bg")?;
    let bg_lum = relative_luminance(br, bg_, bb);
    let fg_lum = relative_luminance(fr, fg_, fb);
    // Move the foreground away from the background: darker on a light surface,
    // lighter on a dark one.
    let toward_black = fg_lum < bg_lum;

    let mut best: Option<(f64, (u8, u8, u8), f64)> = None;
    for step in 1..=100 {
        let t = step as f64 / 100.0;
        let mix = |c: u8| -> u8 {
            let target_c = if toward_black { 0.0 } else { 255.0 };
            (c as f64 + (target_c - c as f64) * t).round().clamp(0.0, 255.0) as u8
        };
        let cand = (mix(fr), mix(fg_), mix(fb));
        let r = contrast_ratio(relative_luminance(cand.0, cand.1, cand.2), bg_lum);
        if r >= target {
            best = Some((r, cand, t));
            break;
        }
    }

    match best {
        Some((r, c, t)) => println!(
            "nearest compliant: #{:02X}{:02X}{:02X} = {r:.2}:1 ({:.0}% toward {})",
            c.0,
            c.1,
            c.2,
            t * 100.0,
            if toward_black { "black" } else { "white" }
        ),
        None => println!("no compliant color on this axis — change the background instead"),
    }
    Ok(())
}

fn rel(repo: &Path, path: &Path) -> String {
    path.strip_prefix(repo)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string()
}
