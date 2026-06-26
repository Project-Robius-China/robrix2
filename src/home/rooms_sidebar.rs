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
use crate::settings::app_preferences::{AppPreferencesGlobal, AppPreferencesAction, ViewModeOverride};
use crate::shared::room_filter_input_bar::{MainFilterAction, RoomFilterInputBarWidgetExt};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*


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
            
            // White app-bar: title row + filter field. A clear bottom divider
            // (below) gives depth over the white room list, mirroring the bottom
            // tab bar's top divider.
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
                    // Mobile reaches the SpacesBar via this header toggle (the
                    // bottom tab bar no longer carries a Spaces button).
                    toggle_spaces_bar_button +: {
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

            // App-bar bottom divider — clearer line for depth (symmetric with the
            // tab bar's top divider).
            LineH {
                width: Fill, height: 1.5
                draw_bg.color: (RBX_STROKE_STRONG)
            }

            SolidView {
                width: Fill, height: Fill
                padding: Inset{left: 15, right: 15}
                show_bg: true
                // White room list between the gray app-bar and gray tab bar.
                draw_bg.color: (RBX_BG_SURFACE)

                CachedWidget {
                    rooms_list := RoomsList {}
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
            for action in actions {
                if let Some(AppPreferencesAction::ViewModeChanged(new_mode)) = action.downcast_ref() {
                    if *new_mode != self.applied_view_mode {
                        self.apply_view_mode(*new_mode);
                        self.view.redraw(cx);
                    }
                }
            }
        }
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}
