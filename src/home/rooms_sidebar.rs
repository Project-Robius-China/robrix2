//! The RoomsSideBar is the widget that contains the RoomsList and other items.
//!
//! It differs in what content it includes based on the adaptive view:
//! * On a narrow mobile view, it acts as the root_view of StackNavigation
//!   * It includes a title label, a search bar, and the RoomsList.
//! * On a wide desktop view, it acts as a permanent tab that is on the left side of the dock.
//!   * It only includes a title label and the RoomsList, because the SearcBar
//!     is at the top of the HomeScreen in Desktop view.

use makepad_widgets::*;

use crate::home::rooms_list::RoomsListWidgetExt;
use crate::home::navigation_tab_bar::NavigationBarAction;
use crate::settings::app_preferences::{AppPreferencesGlobal, AppPreferencesAction, ViewModeOverride};
use crate::shared::room_filter_input_bar::{MainFilterAction, RoomFilterInputBarWidgetExt};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // A `Rooms`/`Workspace` home tab — mirrors the room screen's RoomTopBarTab
    // (src/room/room_top_bar.rs): a RadioButtonFlatter (no radio dot) whose custom
    // shader draws a teal accent underline that fades in with the `active` state.
    // Active = dark text + teal underline; inactive = secondary-grey text.
    mod.widgets.HomeTopBarTab = RadioButtonFlatter {
        width: Fit, height: Fill,
        align: Align{x: 0.5, y: 0.5}
        padding: Inset{left: 14, right: 14, top: 0, bottom: 0}
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


    mod.widgets.RoomsSideBar = #(RoomsSideBar::register_widget(vm)) {
        Desktop := SolidView {
            padding: Inset{top: 20, left: 10, right: 10}
            flow: Down, spacing: 5
            width: Fill, height: Fill

            draw_bg.color: (RBX_BG_SURFACE)

            CachedWidget {
                rooms_list_header := RoomsListHeader {}
            }
            CachedWidget {
                rooms_list := RoomsList {}
            }
        },

        Mobile := View {
            width: Fill, height: Fill
            flow: Down,

            // White app-bar: title row + filter field. (The workspace/squares icon
            // is gone — switching to spaces is now the `Workspace` tab below.)
            SolidView {
                width: Fill, height: Fit
                padding: Inset{top: 15, left: 15, right: 15, bottom: 10}
                flow: Down,

                show_bg: true
                draw_bg.color: (RBX_BG_SURFACE)

                rooms_list_header := RoomsListHeader {
                    spacing: 10
                    open_room_filter_modal_button +: {
                        visible: true
                    }
                }

                View {
                    width: Fill,
                    height: 45,
                    flow: Right
                    padding: Inset{top: 5, bottom: 2}
                    spacing: 5
                    align: Align{y: 0.5}

                    CachedWidget {
                        room_filter_input_bar := RoomFilterInputBar {}
                    }

                }
            }

            // `Rooms | Workspace` tab row (mirrors the room screen's top-bar tabs).
            SolidView {
                width: Fill, height: 44,
                flow: Right,
                align: Align{y: 1.0}
                padding: Inset{left: 8, right: 8}
                spacing: 2
                show_bg: true
                draw_bg.color: (RBX_BG_SURFACE)

                rooms_tab := mod.widgets.HomeTopBarTab { text: "Rooms" }
                workspace_tab := mod.widgets.HomeTopBarTab { text: "Workspace" }
            }

            // Bottom divider under the tab row — clear line for depth.
            LineH {
                width: Fill, height: 1.5
                draw_bg.color: (RBX_STROKE_STRONG)
            }

            // Body: the Rooms list OR the vertical Workspace (spaces) list. The
            // SpacesBar singleton (also used by the desktop rail) lives here on
            // mobile so it stays in the widget tree and keeps draining its updates.
            home_body := PageFlip {
                width: Fill, height: Fill
                active_page: @rooms_page

                rooms_page := SolidView {
                    width: Fill, height: Fill
                    padding: Inset{left: 15, right: 15}
                    show_bg: true
                    draw_bg.color: (RBX_BG_SURFACE)

                    CachedWidget {
                        rooms_list := RoomsList {}
                    }
                }

                workspace_page := SolidView {
                    width: Fill, height: Fill
                    show_bg: true
                    draw_bg.color: (RBX_BG_SURFACE)

                    CachedWidget {
                        root_spaces_bar := mod.widgets.SpacesBar {}
                    }
                }
            }
        }
    }
}

/// A simple wrapper around `AdaptiveView` that contains several global singleton widgets.
///
/// * In the mobile view, it serves as the root view of the StackNavigation,
///   showing the title label, the search bar, and the RoomsList.
/// * In the desktop view, it is a permanent tab in the dock,
///   showing only the title label and the RoomsList
///   (because the search bar is at the top of the HomeScreen).
#[derive(Script, Widget)]
pub struct RoomsSideBar {
    #[deref] view: AdaptiveView,

    #[rust] applied_view_mode: ViewModeOverride,
    /// Mobile only: whether the home tabs (Rooms/Workspace) have had their initial
    /// active highlight applied. The Mobile variant is built lazily on first draw,
    /// so this can't be done in `on_after_new`.
    #[rust] home_tab_initialized: bool,
}

impl ScriptHook for RoomsSideBar {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        vm.with_cx_mut(|cx| {
            // Here we set the global singleton for the RoomsList widget,
            // which is used to access the list of rooms from anywhere in the app.
            cx.set_global(self.view.rooms_list(cx, ids!(rooms_list)));
            let mode = cx.global::<AppPreferencesGlobal>().0.view_mode;
            self.apply_view_mode(mode);
        });
    }
}

impl RoomsSideBar {
    fn apply_view_mode(&mut self, mode: ViewModeOverride) {
        self.view.set_variant_selector(mode.variant_selector());
        self.applied_view_mode = mode;
        // A variant switch rebuilds the (lazy) Mobile tabs, so re-apply the default
        // highlight next chance.
        self.home_tab_initialized = false;
    }

    /// Switch the mobile home body to the Rooms tab and highlight it. Used on
    /// startup and when a space is opened from the Workspace tab (which shows that
    /// space's rooms). No-ops on desktop, where these ids don't exist.
    fn show_rooms_tab(&mut self, cx: &mut Cx) {
        self.view.page_flip(cx, ids!(home_body)).set_active_page(cx, id!(rooms_page));
        if let Some(mut rb) = self.view.radio_button(cx, ids!(rooms_tab)).borrow_mut() {
            rb.animator_play(cx, ids!(active.on));
        }
        if let Some(mut rb) = self.view.radio_button(cx, ids!(workspace_tab)).borrow_mut() {
            rb.animator_play(cx, ids!(active.off));
        }
    }
}
impl Widget for RoomsSideBar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        // If the main room filter input bar changed keywords, re-emit that action
        // as a MainFilterAction so that other widgets can handle it.
        if let Event::Actions(actions) = event {
            if let Some(keywords) = self.view.room_filter_input_bar(cx, ids!(room_filter_input_bar)).changed(actions) {
                cx.action(MainFilterAction::Changed(keywords));
            }

            // Mobile home tabs: switch the body PageFlip between the Rooms list and
            // the Workspace (spaces) list. The radio set de-selects the other tab.
            let home_tabs = self.view.radio_button_set(cx, ids_array!(rooms_tab, workspace_tab));
            match home_tabs.selected(cx, actions) {
                Some(0) => { self.view.page_flip(cx, ids!(home_body)).set_active_page(cx, id!(rooms_page)); }
                Some(1) => { self.view.page_flip(cx, ids!(home_body)).set_active_page(cx, id!(workspace_page)); }
                _ => {}
            }

            for action in actions {
                if let Some(AppPreferencesAction::ViewModeChanged(new_mode)) = action.downcast_ref() {
                    if *new_mode != self.applied_view_mode {
                        self.apply_view_mode(*new_mode);
                        self.view.redraw(cx);
                    }
                }
                // Tapping a space in the Workspace tab opens that space's rooms, so
                // jump back to the Rooms tab to show them.
                if let Some(NavigationBarAction::GoToSpace { .. }) = action.downcast_ref() {
                    self.show_rooms_tab(cx);
                }
            }
        }
        self.view.handle_event(cx, event, scope);

        // The Mobile variant's tabs are built lazily on first draw, so apply the
        // default Rooms-tab highlight the first time they exist. No-ops on desktop.
        if !self.home_tab_initialized
            && self.view.radio_button(cx, ids!(rooms_tab)).borrow().is_some()
        {
            self.home_tab_initialized = true;
            self.show_rooms_tab(cx);
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}
