//! The scenes the harness drives.
//!
//! A scene is a short, deterministic path a real person takes, ending in a
//! measurement. Keeping them small and independent means a failure names one
//! interaction rather than "the app".

use std::path::Path;

use crate::driver::Driver;
use crate::findings::{Dimension, Finding, SceneRecord, Severity};
use crate::frame::Frame;
use crate::proto::{Key, Modifiers, WidgetSnapshot};
use crate::rules_runtime::{
    check_blank, check_contrast, check_dead_click, check_focus, check_geometry, is_interactive,
    Platform, SceneContext,
};

pub struct Viewport {
    pub name: &'static str,
    pub platform: Platform,
    pub width: f64,
    pub height: f64,
    pub dpi: f64,
}

pub const VIEWPORTS: &[Viewport] = &[
    Viewport { name: "desktop", platform: Platform::Desktop, width: 1280.0, height: 800.0, dpi: 1.0 },
    Viewport { name: "mobile", platform: Platform::Mobile, width: 420.0, height: 900.0, dpi: 1.0 },
];

pub struct SceneOutcome {
    pub findings: Vec<Finding>,
    pub records: Vec<SceneRecord>,
}

/// Run every scene against an already-started app.
pub fn run_all(driver: &mut Driver, frames_dir: &Path) -> Result<SceneOutcome, String> {
    let mut findings = Vec::new();
    let mut records = Vec::new();

    for vp in VIEWPORTS {
        driver.set_viewport(vp.width, vp.height, vp.dpi)?;
        let ticks = driver.settle(60)?;

        let widgets = driver.widget_snapshot()?;
        let frame_path = driver.capture(&format!("{}_initial", vp.name))?;

        let ctx = SceneContext {
            name: vp.name,
            platform: vp.platform,
            viewport_w: vp.width as i64,
            viewport_h: vp.height as i64,
            dpi: vp.dpi,
        };

        findings.extend(check_geometry(&ctx, &widgets));

        let mut frame = None;
        if let Some(path) = &frame_path {
            match Frame::load(path) {
                Ok(f) => {
                    findings.extend(check_blank(&ctx, &f));
                    findings.extend(check_contrast(&ctx, &widgets, &f));
                    frame = Some(f);
                }
                Err(e) => findings.push(Finding::new(
                    "harness.frame-unreadable",
                    Dimension::Feedback,
                    Severity::Low,
                    "Captured frame could not be decoded",
                    ctx.name.to_string(),
                    e,
                )),
            }
        } else {
            findings.push(Finding::new(
                "harness.no-frame",
                Dimension::Feedback,
                Severity::Low,
                "App never produced a frame for this viewport",
                ctx.name.to_string(),
                "no PNG appeared in the headless output dir".to_string(),
            ));
        }

        records.push(SceneRecord {
            name: format!("{}-initial", vp.name),
            viewport: format!("{}x{} @{}x", vp.width, vp.height, vp.dpi),
            widgets: widgets.len(),
            visible_widgets: widgets.iter().filter(|w| w.visible).count(),
            frame: frame_path.as_ref().map(|p| file_name(p)),
            settle_ticks: ticks,
        });

        // --- keyboard traversal --------------------------------------------
        let (focus_findings, focus_record) =
            keyboard_scene(driver, &ctx, frames_dir, frame.as_ref())?;
        findings.extend(focus_findings);
        records.push(focus_record);

        // --- primary action --------------------------------------------------
        if let Some(base) = frame.as_ref() {
            let (click_findings, click_record) = primary_action_scene(driver, &ctx, &widgets, base)?;
            findings.extend(click_findings);
            if let Some(r) = click_record {
                records.push(r);
            }
        }
    }

    Ok(SceneOutcome { findings, records })
}

/// Tab through the screen, watching where focus lands and whether it is drawn.
fn keyboard_scene(
    driver: &mut Driver,
    ctx: &SceneContext,
    _frames_dir: &Path,
    baseline: Option<&Frame>,
) -> Result<(Vec<Finding>, SceneRecord), String> {
    let mut rects = Vec::new();
    let mut diffs = Vec::new();
    let mut previous = baseline.map(|f| f.data.clone());
    let mut total_ticks = 0usize;

    for i in 0..4 {
        total_ticks += driver.key(Key::Tab, Modifiers::default())?;
        rects.push(driver.last_focus_rect);

        if let Some(path) = driver.capture(&format!("{}_tab{}", ctx.name, i))? {
            if let Ok(f) = Frame::load(&path) {
                if let Some(prev) = &previous {
                    if prev.len() == f.data.len() {
                        let prev_frame = Frame {
                            width: f.width,
                            height: f.height,
                            data: prev.clone(),
                        };
                        diffs.push(prev_frame.diff_ratio(&f));
                    }
                }
                previous = Some(f.data.clone());
            }
        }
    }

    let findings = check_focus(ctx, &rects, &diffs);
    let record = SceneRecord {
        name: format!("{}-keyboard", ctx.name),
        viewport: format!("{}x{}", ctx.viewport_w, ctx.viewport_h),
        widgets: 0,
        visible_widgets: rects.iter().filter(|r| r.is_some()).count(),
        frame: None,
        settle_ticks: total_ticks,
    };
    Ok((findings, record))
}

/// Click the most prominent enabled control and check the UI answers.
fn primary_action_scene(
    driver: &mut Driver,
    ctx: &SceneContext,
    widgets: &[WidgetSnapshot],
    baseline: &Frame,
) -> Result<(Vec<Finding>, Option<SceneRecord>), String> {
    // "Most prominent" = the largest enabled, visible, on-screen button. On the
    // login screen that is the sign-in CTA, which is exactly the control whose
    // empty-input behaviour matters most.
    let candidates: Vec<&WidgetSnapshot> = widgets
        .iter()
        .filter(|w| {
            w.visible
                && w.enabled
                && is_interactive(w)
                && w.width > 0
                && w.height > 0
                && w.x >= 0
                && w.y >= 0
                && w.right() <= ctx.viewport_w
                && w.bottom() <= ctx.viewport_h
        })
        .collect();

    // Screens that are built but not presented still report laid-out
    // geometry, so an id can appear more than once — the login button of the
    // login screen and of the add-account flow, say. When that happens the
    // harness cannot tell which one the user is looking at, and clicking the
    // wrong one measures nothing. Drop ambiguous ids rather than guess.
    let target = candidates
        .iter()
        .filter(|w| candidates.iter().filter(|o| o.id == w.id).count() == 1)
        .max_by_key(|w| w.width * w.height)
        .copied();

    let Some(target) = target else {
        return Ok((
            vec![Finding::new(
                "state.no-actionable-control",
                Dimension::StateCoverage,
                Severity::High,
                "No enabled, on-screen control was found for this viewport",
                ctx.name.to_string(),
                "widget snapshot contained no clickable target".to_string(),
            )],
            None,
        ));
    };

    let cx = target.x as f64 + target.width as f64 / 2.0;
    let cy = target.y as f64 + target.height as f64 / 2.0;
    let ticks = driver.click(cx, cy)?;
    let after_widgets = driver.widget_snapshot()?;
    let changed = summarize(&after_widgets) != summarize(widgets);

    let mut findings = Vec::new();
    let mut frame_name = None;
    if let Some(path) = driver.capture(&format!("{}_primary_click", ctx.name))? {
        frame_name = Some(file_name(&path));
        if let Ok(after) = Frame::load(&path) {
            findings.extend(check_dead_click(ctx, &target.id, baseline, &after, changed, ticks));
        }
    }

    let record = SceneRecord {
        name: format!("{}-primary-action ({})", ctx.name, target.id),
        viewport: format!("{}x{}", ctx.viewport_w, ctx.viewport_h),
        widgets: after_widgets.len(),
        visible_widgets: after_widgets.iter().filter(|w| w.visible).count(),
        frame: frame_name,
        settle_ticks: ticks,
    };
    Ok((findings, Some(record)))
}

/// A cheap structural fingerprint: which widgets are visible, and what they say.
fn summarize(widgets: &[WidgetSnapshot]) -> Vec<(String, bool, String)> {
    let mut v: Vec<(String, bool, String)> = widgets
        .iter()
        .map(|w| (w.id.clone(), w.visible, w.label()))
        .collect();
    v.sort();
    v
}

fn file_name(p: &Path) -> String {
    p.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default()
}
