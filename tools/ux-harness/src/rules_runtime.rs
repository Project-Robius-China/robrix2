//! Rules that need the app running: they read makepad's post-layout widget
//! tree and the frames it rendered.
//!
//! Every rule states a measurement, not an opinion. If a rule cannot measure
//! something confidently it stays quiet — a UX score is only useful if the
//! findings behind it are all real.

use crate::findings::{Dimension, Finding, Severity};
use crate::frame::Frame;
use crate::proto::WidgetSnapshot;

/// A viewport the scenes run at. Minimum target size depends on it: a finger
/// needs 44dp, a mouse cursor does not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Desktop,
    Mobile,
}

impl Platform {
    pub fn min_target(self) -> i64 {
        match self {
            // Robrix2's own token layer says so: RBX_TAP_MIN = 44,
            // RBX_CONTROL_H_SM = 32.
            Platform::Mobile => 44,
            Platform::Desktop => 28,
        }
    }
}

pub struct SceneContext<'a> {
    pub name: &'a str,
    pub platform: Platform,
    pub viewport_w: i64,
    pub viewport_h: i64,
    pub dpi: f64,
}

/// Widget types that a user is expected to click, and that therefore owe the
/// user a big enough target. Substring matching catches the project's derived
/// widgets (`RobrixIconButton`, `RobrixNeutralIconButton`, …).
pub fn is_interactive(w: &WidgetSnapshot) -> bool {
    let t = w.widget_type.as_str();
    t.contains("Button")
        || t.contains("CheckBox")
        || t.contains("DropDown")
        || t.contains("TextInput")
        || t.contains("Slider")
        || t.contains("RadioButton")
        || t.contains("Toggle")
        || t.contains("Switch")
}

fn is_text_bearing(w: &WidgetSnapshot) -> bool {
    !w.label().is_empty()
}

/// Makepad reports `(0,0,0,0)` for any widget whose area was never drawn. That
/// is the state of every closed modal and every off-screen pane in the tree —
/// `visible: true` there is the DSL property, not "the user can see it". Those
/// are not on screen and no geometry rule may speak about them.
fn was_drawn(w: &WidgetSnapshot) -> bool {
    w.x != 0 || w.y != 0 || w.width != 0 || w.height != 0
}

/// Drawn *and* occupying space: the widgets a person is actually looking at.
fn is_laid_out(w: &WidgetSnapshot) -> bool {
    w.visible && was_drawn(w) && w.width > 0 && w.height > 0
}

pub fn check_geometry(ctx: &SceneContext, widgets: &[WidgetSnapshot]) -> Vec<Finding> {
    let mut out = Vec::new();

    for w in widgets {
        if !w.visible {
            continue;
        }

        // --- zero-sized but carrying text -----------------------------------
        // Only meaningful for a widget that *was* drawn, still collapsed to
        // nothing, and sits on the screen the user is looking at. A widget laid
        // out thousands of pixels below the fold belongs to a screen that is
        // not currently presented, and its geometry says nothing about this one.
        if was_drawn(w)
            && is_text_bearing(w)
            && (w.width <= 0 || w.height <= 0)
            && w.y >= 0
            && w.y < ctx.viewport_h
        {
            out.push(Finding::new(
                "layout.zero-size-text",
                Dimension::LayoutIntegrity,
                Severity::Medium,
                "Visible widget carries text but occupies no space",
                format!("{}:{} ({})", ctx.name, w.id, w.widget_type),
                format!("rect {}x{} at ({},{}), text {:?}", w.width, w.height, w.x, w.y, truncate(&w.label(), 40)),
            ));
            continue;
        }

        if !is_laid_out(w) {
            continue;
        }

        // --- pushed off the leading edges -----------------------------------
        // Content *below* or *right of* the viewport is usually just below the
        // fold in a scroll view, which is normal and must not be flagged.
        // Content at negative coordinates is different: nothing scrolls up to
        // reach it, so it is unreachable by construction.
        if (w.right() <= 0 || w.bottom() <= 0) && (is_interactive(w) || is_text_bearing(w)) {
            out.push(Finding::new(
                "layout.unreachable",
                Dimension::LayoutIntegrity,
                Severity::Medium,
                "Widget is laid out past the top/left edge, where nothing can scroll to it",
                format!("{}:{} ({})", ctx.name, w.id, w.widget_type),
                format!(
                    "rect {}x{} at ({},{}) vs viewport {}x{}",
                    w.width, w.height, w.x, w.y, ctx.viewport_w, ctx.viewport_h
                ),
            ));
            continue;
        }

        // Straddling the right edge: the classic long-string / narrow-window
        // overflow. Only horizontal — vertical overhang is just scrolling.
        let overhang_x = (w.right() - ctx.viewport_w).max(0);
        if w.x < ctx.viewport_w && overhang_x > 8 && (is_interactive(w) || is_text_bearing(w)) {
            out.push(Finding::new(
                "layout.overflow",
                Dimension::LayoutIntegrity,
                Severity::Low,
                "Widget extends past the right edge of the viewport",
                format!("{}:{} ({})", ctx.name, w.id, w.widget_type),
                format!(
                    "overhang x={overhang_x}; rect {}x{} at ({},{})",
                    w.width, w.height, w.x, w.y
                ),
            ));
        }

        // --- target size ----------------------------------------------------
        if is_interactive(w) && w.enabled {
            let min = ctx.platform.min_target();
            if w.height < min || w.width < min {
                let severity = if w.height < min / 2 || w.width < min / 2 {
                    Severity::Medium
                } else {
                    Severity::Low
                };
                out.push(Finding::new(
                    "target.too-small",
                    Dimension::TargetSize,
                    severity,
                    format!("Interactive target smaller than the {min}dp minimum for this viewport"),
                    format!("{}:{} ({})", ctx.name, w.id, w.widget_type),
                    format!("{}x{} at ({},{})", w.width, w.height, w.x, w.y),
                ));
            }
        }
    }

    // --- overlapping hit targets --------------------------------------------
    let clickable: Vec<&WidgetSnapshot> = widgets
        .iter()
        .filter(|w| is_laid_out(w) && is_interactive(w) && w.enabled)
        .collect();
    for i in 0..clickable.len() {
        for j in (i + 1)..clickable.len() {
            let a = clickable[i];
            let b = clickable[j];
            // A button sitting inside a text field is the standard trailing
            // affordance (show-password, clear); the field yields that region
            // deliberately, so it is not a hit-target conflict.
            if is_field_affordance(a, b) {
                continue;
            }
            let Some(area) = intersection_area(a, b) else { continue };
            let smaller = (a.width * a.height).min(b.width * b.height);
            if smaller <= 0 {
                continue;
            }
            let ratio = area as f64 / smaller as f64;
            if ratio > 0.9 {
                out.push(Finding::new(
                    "layout.overlapping-targets",
                    Dimension::LayoutIntegrity,
                    Severity::Low,
                    "Two enabled controls occupy almost the same rect",
                    format!("{}:{} / {}", ctx.name, a.id, b.id),
                    format!("{:.0}% overlap of the smaller target", ratio * 100.0),
                ));
            }
        }
    }

    out
}

/// True when one of the pair is a text field and the other is a small control
/// contained within it — the show-password eye, a clear button, a unit suffix.
fn is_field_affordance(a: &WidgetSnapshot, b: &WidgetSnapshot) -> bool {
    let pair = [(a, b), (b, a)];
    pair.iter().any(|(field, inner)| {
        field.widget_type.contains("TextInput")
            && !inner.widget_type.contains("TextInput")
            && inner.width * inner.height * 2 < field.width * field.height
    })
}

fn intersection_area(a: &WidgetSnapshot, b: &WidgetSnapshot) -> Option<i64> {
    let x0 = a.x.max(b.x);
    let y0 = a.y.max(b.y);
    let x1 = a.right().min(b.right());
    let y1 = a.bottom().min(b.bottom());
    if x1 <= x0 || y1 <= y0 {
        return None;
    }
    Some((x1 - x0) * (y1 - y0))
}

/// Contrast of every text-bearing widget, sampled from the rendered frame.
pub fn check_contrast(ctx: &SceneContext, widgets: &[WidgetSnapshot], frame: &Frame) -> Vec<Finding> {
    let mut out = Vec::new();
    let scale = ctx.dpi;

    for w in widgets {
        if !is_laid_out(w) || !is_text_bearing(w) {
            continue;
        }
        // Only sample things shaped like a line of text. Large containers
        // report concatenated child text, and sampling those measures the page
        // background against an accent block, which means nothing.
        if w.height > 64 || w.width > 640 || w.height < 8 {
            continue;
        }
        // The frame only contains the viewport. A widget below the fold (or
        // otherwise outside it) has no pixels here, and sampling its rect would
        // read whatever happens to sit at those coordinates instead.
        if w.x < 0 || w.y < 0 || w.right() > ctx.viewport_w || w.bottom() > ctx.viewport_h {
            continue;
        }
        // Screens that are built but never presented keep the rects from
        // whatever layout pass last touched them, and those rects can land
        // inside the viewport. Sampling one reads the *presented* screen's
        // pixels and attributes them to a widget nobody can see. The tell is
        // that the box could not hold its own text: makepad's smallest type is
        // ~9px, so a glyph is never narrower than about 4dp.
        let chars = w.label().chars().count() as i64;
        let single_line = w.height < 28;
        if single_line && chars * 4 > w.width {
            continue;
        }
        let px = (w.x as f64 * scale) as i64;
        let py = (w.y as f64 * scale) as i64;
        let pw = (w.width as f64 * scale) as i64;
        let ph = (w.height as f64 * scale) as i64;
        let Some(sample) = frame.region_contrast(px, py, pw, ph) else { continue };

        // Guard rails against measuring something that is not glyphs: a rect
        // that is nearly all "ink" is a filled block, and a rect with a sliver
        // of ink is an antialiasing artifact or a 1px border.
        if sample.ink_coverage > 0.6 {
            continue;
        }

        // WCAG AA: 4.5:1 for body text. Disabled controls are exempt — being
        // low-contrast is what "disabled" looks like.
        if !w.enabled {
            continue;
        }
        if sample.ratio < 3.0 {
            out.push(Finding::new(
                "legibility.contrast",
                Dimension::Legibility,
                Severity::High,
                "Text contrast is below the 3:1 floor for any text",
                format!("{}:{} ({})", ctx.name, w.id, w.widget_type),
                format!(
                    "{:.2}:1 — ink #{:02X}{:02X}{:02X} on #{:02X}{:02X}{:02X}, text {:?}",
                    sample.ratio,
                    sample.ink.0, sample.ink.1, sample.ink.2,
                    sample.background.0, sample.background.1, sample.background.2,
                    truncate(&w.label(), 30)
                ),
            ));
        } else if sample.ratio < 4.5 {
            out.push(Finding::new(
                "legibility.contrast",
                Dimension::Legibility,
                Severity::Medium,
                "Text contrast is below WCAG AA (4.5:1) for body text",
                format!("{}:{} ({})", ctx.name, w.id, w.widget_type),
                format!(
                    "{:.2}:1 — ink #{:02X}{:02X}{:02X} on #{:02X}{:02X}{:02X}, text {:?}",
                    sample.ratio,
                    sample.ink.0, sample.ink.1, sample.ink.2,
                    sample.background.0, sample.background.1, sample.background.2,
                    truncate(&w.label(), 30)
                ),
            ));
        }
    }
    out
}

/// A frame with essentially no content — the "white screen" the visual spec
/// (§7) forbids for loading states.
pub fn check_blank(ctx: &SceneContext, frame: &Frame) -> Vec<Finding> {
    let dominant = frame.dominant_ratio();
    let colors = frame.distinct_colors();
    if dominant > 0.985 || colors < 6 {
        return vec![Finding::new(
            "feedback.blank-frame",
            Dimension::StateCoverage,
            Severity::High,
            "Screen renders essentially blank — no skeleton, spinner or copy",
            ctx.name.to_string(),
            format!("{:.1}% of pixels are one color, {colors} distinct colors", dominant * 100.0),
        )];
    }
    Vec::new()
}

/// A click that changed neither pixels nor widget state.
pub fn check_dead_click(
    ctx: &SceneContext,
    target: &str,
    before: &Frame,
    after: &Frame,
    widgets_changed: bool,
    settle_ticks: usize,
) -> Vec<Finding> {
    let mut out = Vec::new();
    let diff = before.diff_ratio(after);
    if diff < 0.0005 && !widgets_changed {
        out.push(Finding::new(
            "feedback.dead-click",
            Dimension::Feedback,
            Severity::High,
            "Clicking this control produced no visible change and no state change",
            format!("{}:{target}", ctx.name),
            format!("frame diff {:.4}%, widget tree unchanged", diff * 100.0),
        ));
    }
    // 60 ticks is one second of simulated frames. Anything past that is the
    // user waiting without being told to.
    if settle_ticks > 60 {
        out.push(Finding::new(
            "feedback.slow-settle",
            Dimension::Feedback,
            Severity::Low,
            "UI kept redrawing for over a second of simulated time after the interaction",
            format!("{}:{target}", ctx.name),
            format!("{settle_ticks} tick(s) before the UI went quiet"),
        ));
    }
    out
}

/// Keyboard focus: pressing Tab must visibly move focus.
///
/// The measurement is deliberately pixel-based. Makepad only emits
/// `RunViewKeyFocusRect` after a *mouse* release, never after a key event, so
/// the absence of a focus rect proves nothing about the app — but a Tab press
/// that changes no pixels proves quite a lot: whatever the focus state is
/// internally, the user cannot see it.
pub fn check_focus(
    ctx: &SceneContext,
    rects: &[Option<(f64, f64, f64, f64)>],
    frame_diffs: &[f64],
) -> Vec<Finding> {
    let mut out = Vec::new();

    if frame_diffs.is_empty() {
        // No frames to compare — say so rather than inventing a verdict.
        out.push(Finding::new(
            "harness.focus-unmeasured",
            Dimension::KeyboardFocus,
            Severity::Low,
            "Keyboard focus could not be measured — no frames captured between Tab presses",
            ctx.name.to_string(),
            format!("{} Tab presses, 0 frame comparisons", rects.len()),
        ));
        return out;
    }

    let visible_moves = frame_diffs.iter().filter(|d| **d > 0.0002).count();
    if visible_moves == 0 {
        out.push(Finding::new(
            "focus.invisible",
            Dimension::KeyboardFocus,
            Severity::High,
            "Tab changes nothing on screen — there is no visible keyboard focus indicator",
            ctx.name.to_string(),
            format!(
                "{} Tab presses, largest frame delta {:.4}% (threshold 0.02%)",
                frame_diffs.len(),
                frame_diffs.iter().cloned().fold(0.0, f64::max) * 100.0
            ),
        ));
    } else if visible_moves < frame_diffs.len() / 2 {
        out.push(Finding::new(
            "focus.intermittent",
            Dimension::KeyboardFocus,
            Severity::Medium,
            "Most Tab presses do not move a visible focus indicator",
            ctx.name.to_string(),
            format!("{visible_moves} of {} Tab presses changed the frame", frame_diffs.len()),
        ));
    }

    // If the platform *did* report rects (it does after pointer input), a stuck
    // rect across many Tabs is a genuine trap.
    let reported: Vec<&(f64, f64, f64, f64)> = rects.iter().flatten().collect();
    if reported.len() >= 3 {
        let mut distinct: Vec<(i64, i64, i64, i64)> = Vec::new();
        for r in &reported {
            let key = (r.0 as i64, r.1 as i64, r.2 as i64, r.3 as i64);
            if !distinct.contains(&key) {
                distinct.push(key);
            }
        }
        if distinct.len() < 2 {
            out.push(Finding::new(
                "focus.stuck",
                Dimension::KeyboardFocus,
                Severity::High,
                "Focus does not advance — repeated Tab presses stay on one control",
                ctx.name.to_string(),
                format!("{} focus rects reported, all identical", reported.len()),
            ));
        }
    }
    out
}

/// Compare the same scene rendered in two locales. A control whose label grew
/// but whose box did not is where a translation gets silently cut off — the
/// exact failure the visual spec bans in §7.1.
pub fn check_i18n_layout(
    scene: &str,
    base_locale: &str,
    other_locale: &str,
    base: &[WidgetSnapshot],
    other: &[WidgetSnapshot],
) -> Vec<Finding> {
    let mut out = Vec::new();
    for b in base {
        if !is_laid_out(b) || b.label().is_empty() {
            continue;
        }
        let Some(o) = other.iter().find(|o| o.id == b.id && o.widget_type == b.widget_type) else {
            continue;
        };
        if !is_laid_out(o) || o.label().is_empty() {
            continue;
        }
        // Same box, meaningfully longer string.
        let base_chars = b.label().chars().count() as i64;
        let other_chars = o.label().chars().count() as i64;
        if other_chars <= base_chars || o.width != b.width {
            continue;
        }
        // Rough single-line capacity: a glyph is never narrower than ~5dp at
        // this type scale, so anything past that cannot fit on one line.
        let single_line = o.height < 28;
        if single_line && other_chars * 5 > o.width {
            out.push(Finding::new(
                "i18n.truncation-risk",
                Dimension::Internationalization,
                Severity::Medium,
                format!("Label grows in {other_locale} but its box does not — likely truncated"),
                format!("{scene}:{} ({})", b.id, b.widget_type),
                format!(
                    "{base_locale} {base_chars} chars / {other_locale} {other_chars} chars in the same {}dp box",
                    b.width
                ),
            ));
        }
    }
    out
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}
