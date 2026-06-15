//! ToggleSwitch — the iOS-style on/off switch for the redesigned (mobile)
//! settings UI (spec §4.11).
//!
//! This is the built-in `Toggle` (a `CheckBox` variant that renders a sliding
//! track + knob) restyled onto the `RBX_*` tokens: an off track of
//! `RBX_STROKE_STRONG`, an on track of `RBX_ACCENT`, and a white knob in both
//! states. Because it is a `CheckBox` under the hood, drive it in Rust via
//! `view.check_box(cx, ids!(my_switch))`: `.changed(actions) -> Option<bool>`,
//! `.active(cx) -> bool`, `.set_active(cx, bool, Animate::No)`.
//!
//! Usage (DSL): `my_switch := RbxToggle {}` (set `active: true` for default-on).
//!
//! NOTE: inside `script_mod!` only `//` comments are allowed.

use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.RbxToggle = Toggle {
        width: Fit, height: Fit,
        text: "",
        active: false,

        draw_bg +: {
            size: 22.0
            // off track
            color: (RBX_STROKE_STRONG)
            color_hover: (RBX_STROKE_STRONG)
            color_down: (RBX_STROKE_STRONG)
            border_color: (RBX_STROKE_STRONG)
            // on track
            color_active: (RBX_ACCENT)
            border_color_active: (RBX_ACCENT)
            // knob (white in both states)
            mark_color: #fff
            mark_color_active: #fff
        }
    }
}
