//! The `RoomTopBar` is Robrix's own mobile room header bar.
//!
//! It replaces the content of Makepad's `StackNavigationView` header for room
//! views, giving us full control over the visual layout instead of poking the
//! framework header's children by id-path from `app.rs`.
//!
//! It renders, top to bottom:
//! 1. A header row: a back button, the room name + a member-count line, an
//!    encryption shield (green dot = encrypted, red = not), a search button,
//!    and a "more"/info button.
//! 2. A `Chat | Info` segmented tab row with an animated accent underline on
//!    the active tab (built on `RadioButton`, mirroring `navigation_tab_bar`).
//!
//! The bar owns no navigation/business logic — it only emits semantic
//! [`RoomTopBarAction`]s that the enclosing `RoomScreen` handles (back → pop,
//! search → open search pane, info → open room-info pane, tab → switch body).

use makepad_widgets::*;
use crate::shared::design_tokens::{RBX_SUCCESS_FG, RBX_DANGER_FG};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // Ghost icon button for the header row — same visual language as the
    // composer toolbar's `ComposerToolButton` (transparent fill, subtle
    // hover/press wash, secondary-grey icon, RBX_RADIUS_SM corners).
    mod.widgets.RoomTopBarIconButton = RobrixIconButton {
        width: 40, height: 40,
        margin: 0,
        padding: Inset{left: 8, right: 8, top: 6, bottom: 6}
        spacing: 0,
        align: Align{x: 0.5, y: 0.5}
        text: ""
        draw_icon +: { color: (RBX_FG_SECONDARY) }
        // No hover/press background wash ("ripple") — fully transparent in
        // every state; the icon itself is the only thing drawn.
        draw_bg +: {
            color: #0000
            color_hover: #0000
            color_down: #0000
            color_focus: #0000
            border_size: 0.0
            border_radius: (RBX_RADIUS_SM)
        }
        icon_walk: Walk{ width: 20, height: 20 }
    }

    // A single `Chat`/`Info` tab. Built on `RadioButtonFlatter` (no radio dot)
    // with a custom `pixel` shader that draws an accent underline whose opacity
    // is driven by the animator's `active` instance var. Active = dark text +
    // teal underline; inactive = secondary-grey text, no underline.
    mod.widgets.RoomTopBarTab = RadioButtonFlatter {
        width: Fit, height: Fill,
        align: Align{x: 0.5, y: 0.5}
        padding: Inset{left: 12, right: 12, top: 0, bottom: 0}
        icon_walk: Walk{ width: 0, height: 0, margin: 0 }
        label_walk: Walk{ width: Fit, height: Fit, margin: 0 }

        draw_bg +: {
            underline_color: instance((RBX_ACCENT))
            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                let h = 3.0
                sdf.box(
                    0.0,
                    self.rect_size.y - h,
                    self.rect_size.x,
                    h,
                    1.5
                )
                sdf.fill(mix(#0000, self.underline_color, self.active))
                return sdf.result
            }
        }

        draw_text +: {
            color: (RBX_FG_SECONDARY)
            color_hover: (RBX_FG_PRIMARY)
            color_down: (RBX_FG_PRIMARY)
            color_active: (RBX_FG_PRIMARY)
            color_focus: (RBX_FG_PRIMARY)
            text_style: theme.font_bold { font_size: 11.0 }
        }
    }

    mod.widgets.RoomTopBar = #(RoomTopBar::register_widget(vm)) {
        width: Fill, height: Fit,
        flow: Down,

        show_bg: true,
        draw_bg +: { color: (RBX_BG_SURFACE) }

        // ── Header row ──────────────────────────────────────────────
        header_row := View {
            width: Fill, height: 56,
            flow: Right,
            align: Align{y: 0.5}
            padding: Inset{left: 6, right: 6}
            spacing: 0

            back_button := mod.widgets.RoomTopBarIconButton {
                width: 32,
                padding: Inset{left: 6, right: 4, top: 6, bottom: 6}
                draw_icon +: { svg: (ICON_ARROW_BACK), color: (RBX_FG_PRIMARY) }
                icon_walk: Walk{ width: 15, height: 15 }
            }

            title_col := View {
                width: Fill, height: Fit,
                flow: Down,
                align: Align{x: 0.0, y: 0.5}
                spacing: 0
                padding: Inset{left: 0, right: 6}

                room_name := Label {
                    width: Fill, height: Fit, margin: 0,
                    max_lines: 1, text_overflow: Ellipsis,
                    draw_text +: {
                        text_style: theme.font_bold { font_size: 13.0, line_spacing: 1.0 }
                        color: (RBX_FG_PRIMARY)
                    }
                    text: ""
                }
                member_count := Label {
                    width: Fill, height: Fit,
                    margin: Inset{top: -4}
                    max_lines: 1, text_overflow: Ellipsis,
                    draw_text +: {
                        text_style: theme.font_regular { font_size: 9.5, line_spacing: 1.0 }
                        color: (RBX_FG_SECONDARY)
                    }
                    text: ""
                }
            }

            // Encryption status: the shield icon itself is tinted — green when
            // the room is encrypted, red when not. Hidden when unknown. (The
            // icon color is set imperatively in Rust; DrawSvg has no per-state
            // color and a separate overlaid dot would not position reliably.)
            encryption_indicator := View {
                visible: false,
                width: Fit, height: Fit,
                align: Align{y: 0.5}
                margin: Inset{left: 2, right: 4}
                enc_shield := Icon {
                    width: 22, height: 22,
                    draw_icon +: { svg: (ICON_SHIELD), color: (RBX_FG_SECONDARY) }
                    icon_walk: Walk{ width: 22, height: 22 }
                }
            }

            search_button := mod.widgets.RoomTopBarIconButton {
                draw_icon +: { svg: (ICON_SEARCH) }
            }
        }

        // ── Tab row (Chat | Info) ───────────────────────────────────
        tab_row := View {
            width: Fill, height: 40,
            flow: Right,
            align: Align{y: 1.0}
            padding: Inset{left: 8, right: 8}
            spacing: 2

            chat_tab := mod.widgets.RoomTopBarTab { text: "Chat" }
            info_tab := mod.widgets.RoomTopBarTab { text: "Info" }
        }

        // ── Bottom hairline ─────────────────────────────────────────
        divider := SolidView {
            width: Fill, height: 1.0,
            show_bg: true,
            draw_bg.color: (RBX_STROKE_SOFT)
        }
    }
}

/// Which body tab of a room is currently shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum RoomTab {
    /// The message timeline (default).
    #[default]
    Chat,
    /// The inline room-info view.
    Info,
}

/// Semantic actions emitted by [`RoomTopBar`]. The enclosing `RoomScreen`
/// translates these into navigation / pane / tab-switch behavior.
#[derive(Clone, Debug, Default)]
pub enum RoomTopBarAction {
    #[default]
    None,
    /// The back button was pressed (the screen should pop the nav stack).
    Back,
    /// The search button was pressed.
    Search,
    /// A different body tab was selected.
    TabSelected(RoomTab),
}

#[derive(Script, Widget)]
pub struct RoomTopBar {
    #[deref] view: View,

    /// Currently-selected tab. Mirrored from the radio set so callers can read
    /// it without touching the radio widgets directly.
    #[rust] active_tab: RoomTab,

    // Change-guards so `set_room` only re-applies (and redraws) on real changes
    // rather than every draw frame.
    #[rust] last_name: String,
    #[rust] last_members: String,
    #[rust] last_encrypted: Option<Option<bool>>,
}

impl ScriptHook for RoomTopBar {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        vm.with_cx_mut(|cx| {
            // The DSL animator-default override doesn't take effect, so make
            // the Chat tab active programmatically on startup (mirrors
            // `navigation_tab_bar`'s `on_after_new`).
            self.set_active_tab(cx, RoomTab::Chat);
        });
    }
}

impl Widget for RoomTopBar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        if let Event::Actions(actions) = event {
            if self.view.button(cx, ids!(back_button)).clicked(actions) {
                cx.widget_action(self.widget_uid(), RoomTopBarAction::Back);
            }
            if self.view.button(cx, ids!(search_button)).clicked(actions) {
                cx.widget_action(self.widget_uid(), RoomTopBarAction::Search);
            }

            // The radio set de-selects the other tab for us; we just translate
            // the selection into a semantic action.
            let tabs = self.view.radio_button_set(cx, ids_array!(chat_tab, info_tab));
            match tabs.selected(cx, actions) {
                Some(0) => {
                    self.active_tab = RoomTab::Chat;
                    cx.widget_action(self.widget_uid(), RoomTopBarAction::TabSelected(RoomTab::Chat));
                }
                Some(1) => {
                    self.active_tab = RoomTab::Info;
                    cx.widget_action(self.widget_uid(), RoomTopBarAction::TabSelected(RoomTab::Info));
                }
                _ => {}
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl RoomTopBar {
    /// Apply the room name / member-count line / encryption state to the header.
    ///
    /// `encrypted`: `Some(true)` → green shield, `Some(false)` → red shield,
    /// `None` → hide the encryption indicator entirely.
    fn set_room(&mut self, cx: &mut Cx, name: &str, members: &str, encrypted: Option<bool>) {
        if self.last_name != name {
            self.view.label(cx, ids!(room_name)).set_text(cx, name);
            self.last_name = name.to_owned();
        }
        if self.last_members != members {
            self.view.label(cx, ids!(member_count)).set_text(cx, members);
            self.last_members = members.to_owned();
        }
        if self.last_encrypted != Some(encrypted) {
            self.view.view(cx, ids!(encryption_indicator)).set_visible(cx, encrypted.is_some());
            // Tint the shield: green when encrypted, red when not. DrawSvg has
            // no per-state color, so set `draw_icon.color` imperatively.
            let shield_color = if encrypted.unwrap_or(false) {
                RBX_SUCCESS_FG
            } else {
                RBX_DANGER_FG
            };
            let mut shield = self.view.icon(cx, ids!(encryption_indicator.enc_shield));
            script_apply_eval!(cx, shield, {
                draw_icon.color: #(shield_color)
            });
            self.last_encrypted = Some(encrypted);
        }
    }

    /// Programmatically set the active tab (e.g. when the displayed room
    /// changes). Drives the radio animators directly so it does not re-emit a
    /// `Clicked`/`TabSelected` action.
    fn set_active_tab(&mut self, cx: &mut Cx, tab: RoomTab) {
        self.active_tab = tab;
        let chat_on = matches!(tab, RoomTab::Chat);
        if let Some(mut rb) = self.view.radio_button(cx, ids!(chat_tab)).borrow_mut() {
            rb.animator_play(cx, if chat_on { ids!(active.on) } else { ids!(active.off) });
        }
        if let Some(mut rb) = self.view.radio_button(cx, ids!(info_tab)).borrow_mut() {
            rb.animator_play(cx, if chat_on { ids!(active.off) } else { ids!(active.on) });
        }
        self.view.redraw(cx);
    }
}

impl RoomTopBarRef {
    /// See [`RoomTopBar::set_room`].
    pub fn set_room(&self, cx: &mut Cx, name: &str, members: &str, encrypted: Option<bool>) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_room(cx, name, members, encrypted);
        }
    }

    /// See [`RoomTopBar::set_active_tab`].
    pub fn set_active_tab(&self, cx: &mut Cx, tab: RoomTab) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_active_tab(cx, tab);
        }
    }

    /// The currently-selected tab.
    pub fn active_tab(&self) -> RoomTab {
        self.borrow().map(|inner| inner.active_tab).unwrap_or_default()
    }
}
