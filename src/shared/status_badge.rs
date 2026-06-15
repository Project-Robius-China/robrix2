//! StatusBadge — pill status badges for the redesigned (mobile) settings UI.
//!
//! These implement the spec §4.2 contract: a capsule (pill) with a light
//! semantic background and a same-family dark foreground, used to surface a
//! state (success / warning / danger / info / neutral) or a brand-accent
//! emphasis. They are derived from `RobrixIconButton` so they inherit the
//! focus-disabling animator (a badge is a *label*, not an action, so hover /
//! down / focus must never change its appearance — every interaction color is
//! pinned to the resting color).
//!
//! Usage (DSL): `RbxBadgeSuccess { text: "Connected" }`.
//! The text is a normal `Button` `text:` prop, so it can be set at runtime via
//! `view.button(cx, ids!(my_badge)).set_text(cx, "...")`.
//!
//! NOTE: inside `script_mod!` only `//` comments are allowed.

use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // Shared pill skeleton: no icon, pill radius, no border, badge type scale.
    // Every variant below only overrides the bg / fg color pair.
    mod.widgets.RbxBadgeBase = mod.widgets.RobrixIconButton {
        width: Fit, height: Fit,
        spacing: 0,
        margin: 0,
        padding: Inset{left: (SPACE_SM), right: (SPACE_SM), top: 3.0, bottom: 3.0}
        align: Align{x: 0.5, y: 0.5}
        icon_walk: Walk{width: 0, height: 0, margin: 0}

        draw_bg +: {
            border_radius: (RBX_RADIUS_PILL)
            border_size: 0.0
            border_color: #0000
            border_color_hover: #0000
            border_color_down: #0000
            border_color_focus: #0000
            color_2: vec4(-1.0, -1.0, -1.0, -1.0)
            border_color_2: vec4(-1.0, -1.0, -1.0, -1.0)
        }

        draw_text +: {
            text_style: RBX_TEXT_BADGE {}
        }
        text: ""
    }

    // success = Connected / Healthy / Enabled / Active / Synced
    mod.widgets.RbxBadgeSuccess = mod.widgets.RbxBadgeBase {
        draw_bg +: {
            color: (RBX_SUCCESS_BG)
            color_hover: (RBX_SUCCESS_BG)
            color_down: (RBX_SUCCESS_BG)
        }
        draw_text +: {
            color: (RBX_SUCCESS_FG)
            color_hover: (RBX_SUCCESS_FG)
            color_down: (RBX_SUCCESS_FG)
        }
    }

    // warning = Approval required / Pending / Waiting
    mod.widgets.RbxBadgeWarning = mod.widgets.RbxBadgeBase {
        draw_bg +: {
            color: (RBX_WARNING_BG)
            color_hover: (RBX_WARNING_BG)
            color_down: (RBX_WARNING_BG)
        }
        draw_text +: {
            color: (RBX_WARNING_FG)
            color_hover: (RBX_WARNING_FG)
            color_down: (RBX_WARNING_FG)
        }
    }

    // danger = Failed / Rejected / Error / risk action
    mod.widgets.RbxBadgeDanger = mod.widgets.RbxBadgeBase {
        draw_bg +: {
            color: (RBX_DANGER_BG)
            color_hover: (RBX_DANGER_BG)
            color_down: (RBX_DANGER_BG)
        }
        draw_text +: {
            color: (RBX_DANGER_FG)
            color_hover: (RBX_DANGER_FG)
            color_down: (RBX_DANGER_FG)
        }
    }

    // info = capability / linked object / neutral metadata highlight
    mod.widgets.RbxBadgeInfo = mod.widgets.RbxBadgeBase {
        draw_bg +: {
            color: (RBX_INFO_BG)
            color_hover: (RBX_INFO_BG)
            color_down: (RBX_INFO_BG)
        }
        draw_text +: {
            color: (RBX_INFO_FG)
            color_hover: (RBX_INFO_FG)
            color_down: (RBX_INFO_FG)
        }
    }

    // neutral = Idle / secondary / disabled-ish
    mod.widgets.RbxBadgeNeutral = mod.widgets.RbxBadgeBase {
        draw_bg +: {
            color: (RBX_NEUTRAL_BG)
            color_hover: (RBX_NEUTRAL_BG)
            color_down: (RBX_NEUTRAL_BG)
        }
        draw_text +: {
            color: (RBX_NEUTRAL_FG)
            color_hover: (RBX_NEUTRAL_FG)
            color_down: (RBX_NEUTRAL_FG)
        }
    }

    // accent = brand emphasis (e.g. "Agent-enabled"). Not a state pair:
    // RBX_ACCENT_SOFT bg + RBX_ACCENT fg.
    mod.widgets.RbxBadgeAccent = mod.widgets.RbxBadgeBase {
        draw_bg +: {
            color: (RBX_ACCENT_SOFT)
            color_hover: (RBX_ACCENT_SOFT)
            color_down: (RBX_ACCENT_SOFT)
        }
        draw_text +: {
            color: (RBX_ACCENT)
            color_hover: (RBX_ACCENT)
            color_down: (RBX_ACCENT)
        }
    }
}
