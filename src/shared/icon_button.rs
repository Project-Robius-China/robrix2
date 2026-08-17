use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // The base Robrix button widget.
    // Uses COLOR_ACTIVE_PRIMARY (the accent teal) background with white text by default.
    // See also the preset variants below:
    //   RobrixPositiveIconButton, RobrixNegativeIconButton, RobrixNeutralIconButton.
    mod.widgets.RobrixIconButton = Button {
        width: Fit,
        height: Fit,
        spacing: 10,
        padding: 10,
        align: Align{x: 0, y: 0.5}

        // A mouse click must not leave the button wearing the focus ring.
        //
        // `Button` grabs key focus on `FingerDown` when `grab_key_focus` is set,
        // which fires the same `Hit::KeyFocus` a Tab press does — so a click used
        // to light the ring and leave it lit after the pointer moved away. This
        // was previously worked around by pinning the `focus` uniform to 0.0 in
        // both animator states, which also meant keyboard users never saw a ring
        // at all: `RBX_FOCUS_RING` had no readers anywhere in the app.
        //
        // Not grabbing focus on click removes the cause instead of the symptom.
        // The animator can then do its job, and the ring appears only when focus
        // arrives by keyboard — the behaviour CSS calls `:focus-visible`.
        grab_key_focus: false

        // Focus ring: a 2px accent outline, drawn on top of whatever border the
        // variant already has (spec §7.1). Text colour is left alone — the ring
        // carries the signal.
        animator +: {
            focus: {
                default: @off
                off: AnimatorState {
                    from: {all: Forward {duration: 0.0}}
                    apply: {
                        draw_bg: {focus: 0.0}
                    }
                }
                on: AnimatorState {
                    from: {all: Forward {duration: 0.0}}
                    apply: {
                        draw_bg: {focus: 1.0}
                    }
                }
            }
        }

        draw_bg +: {
            border_size: 0.0
            border_radius: 4.0

            color: (COLOR_ACTIVE_PRIMARY)
            color_hover: (COLOR_ACTIVE_PRIMARY_DARKER)
            color_down: #0C5DAA
            color_disabled: (COLOR_BG_DISABLED)

            // Keyboard focus shows as the accent ring on variants that already
            // draw a border, and as a tinted fill on the ones that do not.
            //
            // The ring cannot simply be switched on for every button: the shader
            // insets its box by `border_size` (`sdf.box(border_size, border_size,
            // w - 2*border_size, …)`), so giving the flat variants a width just
            // for focus would shrink every button by 4px. `border_size` is also a
            // scalar the shader never mixes by `focus`, so it cannot be animated
            // per-state without overriding the 130-odd call sites that set their
            // own. Tinting the fill is the part that works everywhere.
            border_color: #0000
            border_color_hover: #0000
            border_color_down: #0000
            border_color_focus: (RBX_FOCUS_RING)
            border_color_disabled: #0000

            // A soft wash, not the saturated accent. The shader replaces the
            // fill outright — `fill = color_fill.mix(color_fill_focus, focus)` —
            // so on the ghost variants (composer tools, the message copy button)
            // a solid tint turned the whole button into an accent block with a
            // grey icon on it: inverted, not outlined. The token itself must stay
            // opaque, because a translucent one would let the page show through
            // the solid variants.
            color_focus: (RBX_ACCENT_SOFT)

            // Disable gradient (color_2) by default
            color_2: vec4(-1.0, -1.0, -1.0, -1.0)
            border_color_2: vec4(-1.0, -1.0, -1.0, -1.0)
        }

        draw_icon.color: (COLOR_PRIMARY)
        icon_walk: Walk{width: 16, height: 16}

        draw_text +: {
            color: (COLOR_PRIMARY)
            color_hover: (COLOR_PRIMARY)
            color_down: (COLOR_PRIMARY)
            color_disabled: (COLOR_FG_DISABLED)
            text_style: mod.widgets.REGULAR_TEXT {font_size: 10},
        }
        text: ""
    }

    // Green button for positive/accept actions: joining a room, confirming, accepting an invite.
    mod.widgets.RobrixPositiveIconButton = mod.widgets.RobrixIconButton {
        draw_bg +: {
            border_color: (COLOR_FG_ACCEPT_GREEN)
            border_color_hover: (COLOR_FG_ACCEPT_GREEN)
            border_color_down: (COLOR_FG_ACCEPT_GREEN)
            color: (COLOR_BG_ACCEPT_GREEN)
            color_hover: #D4EED4
            color_down: #B8E0B8
            // Focus must not repaint a semantic fill: the ring on the border
            // carries the signal for this variant.
            color_focus: (COLOR_BG_ACCEPT_GREEN)
        }
        draw_icon.color: (COLOR_FG_ACCEPT_GREEN)
        draw_text +: {
            color: (COLOR_FG_ACCEPT_GREEN)
            color_hover: (COLOR_FG_ACCEPT_GREEN)
            color_down: (COLOR_FG_ACCEPT_GREEN)
        }
    }

    // Red button for negative/dangerous actions: rejecting, leaving, deleting, blocking.
    mod.widgets.RobrixNegativeIconButton = mod.widgets.RobrixIconButton {
        draw_bg +: {
            border_color: (COLOR_FG_DANGER_RED)
            border_color_hover: (COLOR_FG_DANGER_RED)
            border_color_down: (COLOR_FG_DANGER_RED)
            color: (COLOR_BG_DANGER_RED)
            color_hover: #F0D4D4
            color_down: #E0B8B8
            // Focus must not repaint a semantic fill: the ring on the border
            // carries the signal for this variant.
            color_focus: (COLOR_BG_DANGER_RED)
        }
        draw_icon.color: (COLOR_FG_DANGER_RED)
        draw_text +: {
            color: (COLOR_FG_DANGER_RED)
            color_hover: (COLOR_FG_DANGER_RED)
            color_down: (COLOR_FG_DANGER_RED)
        }
    }

    // Gray button for cancel/dismiss actions: canceling, closing, going back.
    mod.widgets.RobrixNeutralIconButton = mod.widgets.RobrixIconButton {
        draw_bg +: {
            border_color: (COLOR_BG_DISABLED)
            border_color_hover: (COLOR_BG_DISABLED)
            border_color_down: (COLOR_BG_DISABLED)
            color: (COLOR_SECONDARY)
            color_hover: #D0D0D0
            color_down: #C0C0C0
            // Focus must not repaint a semantic fill: the ring on the border
            // carries the signal for this variant.
            color_focus: (COLOR_SECONDARY)
        }
        draw_icon.color: (COLOR_TEXT)
        draw_text +: {
            color: (COLOR_TEXT)
            color_hover: (COLOR_TEXT)
            color_down: (COLOR_TEXT)
        }
    }

    // Teal primary button for the settings screen (accent CTA — Upload, Save,
    // Check for Updates, etc.). Uses the RBX_ACCENT brand color instead of the
    // legacy blue so settings stays on-theme.
    mod.widgets.SettingsPrimaryButton = mod.widgets.RobrixIconButton {
        draw_bg +: {
            color: (RBX_ACCENT)
            color_hover: (RBX_ACCENT_HOVER)
            color_down: (RBX_ACCENT_PRESSED)
            // Solid accent CTA: keep the fill, the ring shows focus.
            color_focus: (RBX_ACCENT)
            color_disabled: (RBX_BG_DISABLED)
            border_color: #0000
            border_color_hover: #0000
            border_color_down: #0000
            border_radius: (RBX_RADIUS_SM)
        }
        draw_icon.color: (RBX_FG_ON_ACCENT)
        draw_text +: {
            color: (RBX_FG_ON_ACCENT)
            color_hover: (RBX_FG_ON_ACCENT)
            color_down: (RBX_FG_ON_ACCENT)
        }
    }

    // Leading round icon badge for a settings card header / row. Soft accent
    // tint by default — override `draw_bg.color` for a different tint, and drop
    // an `Icon { ... }` child inside per use.
    mod.widgets.SettingsIconCircle = CircleView {
        width: 34, height: 34
        align: Align{x: 0.5, y: 0.5}
        show_bg: true
        draw_bg +: { color: (RBX_ACCENT_SOFT) }
    }

    // Status pill: soft-tinted, fully-rounded badge with a bold label. Neutral by
    // default — override draw_bg.color + badge_label's color for success / warning
    // / danger / info / accent states.
    mod.widgets.SettingsStatusBadge = RoundedView {
        width: Fit, height: Fit
        align: Align{x: 0.5, y: 0.5}
        padding: Inset{left: 10, right: 10, top: 4, bottom: 4}
        show_bg: true
        draw_bg +: { color: (RBX_NEUTRAL_BG), border_radius: (RBX_RADIUS_PILL) }
        badge_label := Label {
            width: Fit, height: Fit
            draw_text +: { text_style: RBX_TEXT_BADGE {}, color: (RBX_NEUTRAL_FG) }
            text: ""
        }
    }
}
