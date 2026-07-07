//! A small popup menu, anchored next to the "+" button, that offers the various
//! ways to add a room or conversation:
//!   * New room            → opens the CreateRoomModal
//!   * New direct message  → opens the StartChatModal (this is "add friend")
//!   * Join with a link    → opens the JoinRoomModal
//!   * Explore public rooms → navigates to the public room DirectoryScreen
//!
//! It reuses the same anchored-overlay pattern as `RoomContextMenu`: a full-screen
//! scrim whose inner `main_content` card is positioned by the App (which clamps it
//! to the overlay container and sets its `margin` via `script_apply_eval!`).

use makepad_widgets::*;

use crate::{
    app::AppState,
    home::{
        add_room::{CreateRoomModalAction, JoinRoomModalAction, StartChatModalAction},
        navigation_tab_bar::NavigationBarAction,
    },
    i18n::{AppLanguage, tr_key},
    settings::app_preferences::effective_is_desktop,
};

/// The fixed width of the add menu card, in DIPs. Emitters use this to right-align
/// the menu under a top-right anchor (e.g. the mobile header "+").
pub const ADD_MENU_WIDTH: f64 = 244.0;
/// Approximate total height of the menu card, used only to clamp its position so
/// it never overflows the bottom edge of the overlay container.
const ADD_MENU_HEIGHT: f64 = 210.0;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*


    // A single row in the add menu: a left-aligned icon + label, with an RBX
    // surface/hover/pressed background and a soft rounded highlight.
    mod.widgets.AddMenuItem = RobrixIconButton {
        height: 44,
        width: Fill,
        margin: 0,
        padding: Inset{left: 12, right: 12, top: 8, bottom: 8}
        spacing: 12,
        align: Align{x: 0.0, y: 0.5}
        icon_walk: Walk{width: 20, height: 20, margin: Inset{right: 2}}

        draw_bg +: {
            color: (RBX_BG_SURFACE)
            color_hover: (RBX_BG_HOVER)
            color_down: (RBX_BG_PRESSED)
            border_size: 0.0
            border_radius: (RBX_RADIUS_SM)
        }
        draw_icon.color: (RBX_ACCENT)
        draw_text +: {
            color: (RBX_FG_PRIMARY)
            color_hover: (RBX_FG_PRIMARY)
            color_down: (RBX_FG_PRIMARY)
            text_style: RBX_TEXT_BODY {}
        }
    }

    mod.widgets.AddMenu = set_type_default() do #(AddMenu::register_widget(vm)) {
        ..mod.widgets.SolidView

        visible: false,
        flow: Overlay,
        width: Fill,
        height: Fill,
        cursor: MouseCursor.Default,
        align: Align{x: 0, y: 0}

        show_bg: true
        draw_bg +: {
            color: (RBX_SCRIM)
        }

        main_content := RoundedView {
            flow: Down
            width: 244,
            height: Fit,
            padding: 6,
            spacing: 2,

            show_bg: true
            // Flat card: tighter corners, a defined border, and no drop shadow
            // (the scrim already separates it from the content behind).
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_SM)
                border_size: 1.0
                border_color: (RBX_STROKE_STRONG)
            }

            new_room_item := mod.widgets.AddMenuItem {
                draw_icon +: { svg: (ICON_ADD) }
                text: "New room"
            }

            new_dm_item := mod.widgets.AddMenuItem {
                draw_icon +: { svg: (ICON_ADD_USER) }
                text: "New direct message"
            }

            join_item := mod.widgets.AddMenuItem {
                draw_icon +: { svg: (ICON_LINK) }
                text: "Join with a link"
            }

            divider := LineH {
                margin: Inset{top: 4, bottom: 4, left: 8, right: 8}
                draw_bg.color: (RBX_DIVIDER)
            }

            explore_item := mod.widgets.AddMenuItem {
                draw_icon +: { svg: (ICON_GLOBE) }
                text: "Explore public rooms"
            }
        }
    }
}


/// Action to request showing the add menu, anchored so its top-left corner sits
/// at the given absolute position (already computed by the emitter using
/// [`ADD_MENU_WIDTH`] as needed). The App clamps this into the overlay container.
#[derive(Clone, Debug, Default)]
pub enum AddMenuAction {
    Open { pos: DVec2 },
    #[default]
    None,
}


#[derive(Script, ScriptHook, Widget)]
pub struct AddMenu {
    #[deref] view: View,
    #[source] source: ScriptObjectRef,
    #[rust] app_language: AppLanguage,
    /// The effective layout (desktop vs mobile) at the moment the menu was opened.
    /// If it changes while the menu is open (e.g. a window resize crossing the
    /// breakpoint), the menu auto-closes — its anchored position is only valid for
    /// the layout it was opened in, so it would otherwise linger, misplaced.
    #[rust(true)] opened_is_desktop: bool,
}

impl Widget for AddMenu {
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let step = self.view.draw_walk(cx, scope, walk);
        if self.visible {
            let main_content_area = self.view(cx, ids!(main_content)).area();
            cx.block_scrolling_except_within(main_content_area);
        }
        step
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if !self.visible {
            return;
        }
        // Close if the layout switched between desktop and mobile while open, so
        // the menu can't linger at a now-wrong anchor in the other layout.
        if effective_is_desktop(cx) != self.opened_is_desktop {
            self.close(cx);
            return;
        }
        if let Some(app_state) = scope.data.get::<AppState>()
            && self.app_language != app_state.app_language
        {
            self.set_app_language(cx, app_state.app_language);
        }
        self.view.handle_event(cx, event, scope);

        // Close on backdrop click, Escape, or a system back gesture. There is no
        // opening-gesture dedup here (unlike RoomContextMenu): the menu is opened
        // from a button *click* (FingerUp), which is already fully consumed by the
        // time we become visible, so no stray FingerUp lands on the scrim.
        let area = self.view.area();
        let close_menu = event.back_pressed()
            || match event.hits_with_capture_overload(cx, area, true) {
                Hit::KeyUp(key) => key.key_code == KeyCode::Escape,
                Hit::FingerUp(fue) if fue.is_over => {
                    !self.view(cx, ids!(main_content)).area().rect(cx).contains(fue.abs)
                }
                _ => false,
            };
        if close_menu {
            self.close(cx);
            return;
        }

        self.widget_match_event(cx, event, scope);
    }
}

impl WidgetMatchEvent for AddMenu {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        let mut close_menu = false;

        if self.button(cx, ids!(new_room_item)).clicked(actions) {
            cx.action(CreateRoomModalAction::Open { parent_space_id: None });
            close_menu = true;
        } else if self.button(cx, ids!(new_dm_item)).clicked(actions) {
            cx.action(StartChatModalAction::Open);
            close_menu = true;
        } else if self.button(cx, ids!(join_item)).clicked(actions) {
            cx.action(JoinRoomModalAction::Open);
            close_menu = true;
        } else if self.button(cx, ids!(explore_item)).clicked(actions) {
            cx.action(NavigationBarAction::GoToDirectory);
            close_menu = true;
        }

        if close_menu {
            self.close(cx);
        }
    }
}

impl AddMenu {
    fn set_app_language(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        self.app_language = app_language;
        self.button(cx, ids!(new_room_item))
            .set_text(cx, tr_key(self.app_language, "add_menu.item.new_room"));
        self.button(cx, ids!(new_dm_item))
            .set_text(cx, tr_key(self.app_language, "add_menu.item.new_dm"));
        self.button(cx, ids!(join_item))
            .set_text(cx, tr_key(self.app_language, "add_menu.item.join"));
        self.button(cx, ids!(explore_item))
            .set_text(cx, tr_key(self.app_language, "add_menu.item.explore"));
    }

    /// Shows the menu and returns its expected `(width, height)` so the App can
    /// clamp its anchored position within the overlay container.
    fn show(&mut self, cx: &mut Cx, app_language: AppLanguage) -> DVec2 {
        self.opened_is_desktop = effective_is_desktop(cx);
        self.set_app_language(cx, app_language);
        for id in [ids!(new_room_item), ids!(new_dm_item), ids!(join_item), ids!(explore_item)] {
            self.button(cx, id).reset_hover(cx);
        }
        self.visible = true;
        cx.set_key_focus(self.view.area());
        self.redraw(cx);
        dvec2(ADD_MENU_WIDTH, ADD_MENU_HEIGHT)
    }

    fn close(&mut self, cx: &mut Cx) {
        self.visible = false;
        cx.revert_key_focus();
        cx.unblock_scrolling();
        self.redraw(cx);
    }
}

impl AddMenuRef {
    pub fn show(&self, cx: &mut Cx, app_language: AppLanguage) -> DVec2 {
        let Some(mut inner) = self.borrow_mut() else { return DVec2::default() };
        inner.show(cx, app_language)
    }

    pub fn is_currently_shown(&self) -> bool {
        self.borrow().is_some_and(|inner| inner.visible)
    }
}
