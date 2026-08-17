//! A modal that links one of the user's existing rooms into a space.
//!
//! This is the counterpart to the "remove from space" button in the
//! [`SpaceLobbyScreen`](super::space_lobby::SpaceLobbyScreen): both edit the
//! space's `m.space.child` state, they just go in opposite directions.
//!
//! The list is a local search over already-joined rooms — there is no server-side
//! query involved, and rooms that the space already contains are filtered out.

use std::collections::HashSet;

use makepad_widgets::*;
use matrix_sdk::ruma::OwnedRoomId;

use crate::{
    app::AppState,
    home::rooms_list::RoomsListRef,
    i18n::{AppLanguage, tr_fmt, tr_key},
    room::FetchedRoomAvatar,
    shared::avatar::AvatarWidgetRefExt,
    sliding_sync::{MatrixRequest, submit_async_request},
    utils::{self, RoomNameId},
};

/// The number of result rows in the DSL below; also the cap on how many
/// candidate rooms are shown at once.
const MAX_RESULTS: usize = 8;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    let AddExistingRoomResultItem = View {
        visible: false
        width: Fill
        height: 48
        flow: Overlay

        row := View {
            width: Fill
            height: Fill
            flow: Right
            align: Align{y: 0.5}
            spacing: 8
            padding: Inset{left: 8, right: 8, top: 5, bottom: 5}

            avatar := Avatar { width: 30, height: 30 }

            text_col := View {
                width: Fill
                height: Fit
                flow: Down
                spacing: 0

                name_label := Label {
                    width: Fill
                    height: Fit
                    flow: Flow.Right{wrap: false}
                    max_lines: 1
                    text_overflow: Ellipsis
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: RBX_TEXT_BODY {}
                    }
                    text: ""
                }

                id_label := Label {
                    width: Fill
                    height: Fit
                    flow: Flow.Right{wrap: false}
                    max_lines: 1
                    text_overflow: Ellipsis
                    draw_text +: {
                        color: (RBX_FG_TERTIARY)
                        text_style: RBX_TEXT_META {}
                    }
                    text: ""
                }
            }
        }

        click_button := RobrixNeutralIconButton {
            width: Fill
            height: Fill
            text: ""
            icon_walk: Walk{width: 0, height: 0}
            draw_bg +: {
                color: (RBX_TRANSPARENT)
                color_hover: (RBX_BG_HOVER)
                color_down: (RBX_BG_PRESSED)
            }
        }
    }

    mod.widgets.AddExistingRoomModal = #(AddExistingRoomModal::register_widget(vm)) {
        width: Fill { max: 400 }
        height: Fit
        margin: Inset{left: 12, right: 12}

        RoundedShadowView {
            width: Fill
            height: Fit
            align: Align{x: 0.5}
            flow: Down
            padding: Inset{top: 20, right: 20, bottom: 16, left: 20}

            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_SM)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
                shadow_color: (RBX_SHADOW_STRONG)
                shadow_radius: 10.0
                shadow_offset: vec2(0.0, 3.0)
            }

            title := Label {
                width: Fill
                height: Fit
                margin: Inset{bottom: 4}
                flow: Flow.Right{wrap: true}
                draw_text +: {
                    text_style: RBX_TEXT_SECTION_TITLE {}
                    color: (RBX_FG_PRIMARY)
                }
                text: ""
            }

            subtitle := Label {
                width: Fill
                height: Fit
                margin: Inset{bottom: 12}
                flow: Flow.Right{wrap: true}
                draw_text +: {
                    color: (RBX_FG_SECONDARY)
                    text_style: RBX_TEXT_BODY {}
                }
                text: ""
            }

            filter_input := RobrixTextInput {
                width: Fill
                height: 40
                padding: Inset{left: 12, right: 12, top: 11, bottom: 0}
                empty_text: ""
            }

            status_label := Label {
                visible: false
                width: Fill
                height: Fit
                margin: Inset{top: 10, left: 1}
                flow: Flow.Right{wrap: true}
                draw_text +: {
                    text_style: RBX_TEXT_META {}
                    color: (RBX_FG_SECONDARY)
                }
                text: ""
            }

            results_scroll := ScrollYView {
                width: Fill
                height: 220
                margin: Inset{top: 6}

                results := View {
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 3

                    result_item_0 := AddExistingRoomResultItem {}
                    result_item_1 := AddExistingRoomResultItem {}
                    result_item_2 := AddExistingRoomResultItem {}
                    result_item_3 := AddExistingRoomResultItem {}
                    result_item_4 := AddExistingRoomResultItem {}
                    result_item_5 := AddExistingRoomResultItem {}
                    result_item_6 := AddExistingRoomResultItem {}
                    result_item_7 := AddExistingRoomResultItem {}
                }
            }

            buttons_view := View {
                width: Fill
                height: Fit
                flow: Right
                padding: Inset{top: 14, bottom: 2}
                align: Align{x: 1.0, y: 0.5}

                cancel_button := RobrixNeutralIconButton {
                    width: 120
                    align: Align{x: 0.5, y: 0.5}
                    padding: 12
                    draw_icon.svg: (ICON_FORBIDDEN)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: ""
                }
            }
        }
    }
}

/// Actions emitted by other widgets to show or hide the [`AddExistingRoomModal`].
#[derive(Debug)]
pub enum AddExistingRoomModalAction {
    /// Open the modal to add one of the user's rooms to the given space.
    Open {
        space_name_id: RoomNameId,
        /// Rooms the space already contains, which are excluded from the list.
        existing_children: HashSet<OwnedRoomId>,
    },
    Close,
}

#[derive(Script, ScriptHook, Widget)]
pub struct AddExistingRoomModal {
    #[deref] view: View,
    #[rust] space_name_id: Option<RoomNameId>,
    #[rust] existing_children: HashSet<OwnedRoomId>,
    /// The rooms currently listed, in the order they are drawn.
    #[rust] results: Vec<(RoomNameId, FetchedRoomAvatar)>,
    #[rust] app_language: AppLanguage,
}

impl Widget for AddExistingRoomModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if self.app_language != app_language {
            self.app_language = app_language;
            self.update_static_texts(cx);
        }
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl WidgetMatchEvent for AddExistingRoomModal {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        if self.view.button(cx, ids!(cancel_button)).clicked(actions) {
            cx.action(AddExistingRoomModalAction::Close);
            return;
        }

        if let Some(keywords) = self.view.text_input(cx, ids!(filter_input)).changed(actions) {
            self.refresh_results(cx, &keywords);
            return;
        }

        if let Some(index) = self.clicked_result_index(cx, actions) {
            let Some(space_name_id) = self.space_name_id.clone() else { return };
            let Some((child, _)) = self.results.get(index).cloned() else { return };
            submit_async_request(MatrixRequest::AddRoomToSpace {
                space_id: space_name_id.room_id().clone(),
                child,
            });
            // The SpaceLobbyScreen reports the outcome and refreshes its tree,
            // so this modal's job is done as soon as the request is submitted.
            cx.action(AddExistingRoomModalAction::Close);
        }
    }
}

impl AddExistingRoomModal {
    fn clicked_result_index(&self, cx: &mut Cx, actions: &Actions) -> Option<usize> {
        let results_view = self.view.view(cx, ids!(results_scroll.results));
        (0..MAX_RESULTS).find(|index| {
            results_view
                .button(cx, &[result_item_id(*index), live_id!(click_button)])
                .clicked(actions)
        })
    }

    /// Rebuilds the visible list for the given filter text.
    ///
    /// With no filter we show the most recent rooms, so the list is useful
    /// before the user types anything.
    fn refresh_results(&mut self, cx: &mut Cx, keywords: &str) {
        let keywords = keywords.trim();
        let mut candidates: Vec<(RoomNameId, FetchedRoomAvatar)> = if !cx.has_global::<RoomsListRef>() {
            Vec::new()
        } else if keywords.is_empty() {
            cx.get_global::<RoomsListRef>()
                // Over-fetch, because some candidates are dropped just below.
                .get_recent_rooms(MAX_RESULTS + self.existing_children.len())
                .into_iter()
                .map(|room| (room.room_name_id, room.room_avatar))
                .collect()
        } else {
            cx.get_global::<RoomsListRef>()
                .get_matching_room_items(keywords, MAX_RESULTS + self.existing_children.len())
        };

        // Never offer a room the space already contains, nor the space itself.
        let space_id = self.space_name_id.as_ref().map(RoomNameId::room_id).cloned();
        candidates.retain(|(room_name_id, _)| {
            !self.existing_children.contains(room_name_id.room_id())
                && space_id.as_ref() != Some(room_name_id.room_id())
        });
        candidates.truncate(MAX_RESULTS);
        self.results = candidates;

        let results_view = self.view.view(cx, ids!(results_scroll.results));
        for index in 0..MAX_RESULTS {
            let item = results_view.view(cx, &[result_item_id(index)]);
            let Some((room_name_id, avatar)) = self.results.get(index) else {
                item.set_visible(cx, false);
                continue;
            };
            item.set_visible(cx, true);
            item.button(cx, ids!(click_button)).reset_hover(cx);
            item.label(cx, ids!(row.text_col.name_label))
                .set_text(cx, &room_name_id.to_string());
            item.label(cx, ids!(row.text_col.id_label))
                .set_text(cx, room_name_id.room_id().as_str());

            let avatar_ref = item.avatar(cx, ids!(row.avatar));
            match avatar {
                FetchedRoomAvatar::Text(text) => {
                    avatar_ref.show_text(cx, None, None, text);
                }
                FetchedRoomAvatar::Image(image_data) => {
                    let drew = avatar_ref.show_image(
                        cx,
                        None, // avatars here aren't clickable
                        |cx, img| utils::load_png_or_jpg(&img, cx, image_data),
                    ).is_ok();
                    if !drew {
                        avatar_ref.show_text(cx, None, None, &room_name_id.to_string());
                    }
                }
            }
        }

        let status_label = self.view.label(cx, ids!(status_label));
        if self.results.is_empty() {
            status_label.set_visible(cx, true);
            status_label.set_text(cx, tr_key(self.app_language, if keywords.is_empty() {
                "add_existing_room.status.no_rooms"
            } else {
                "add_existing_room.status.no_matches"
            }));
        } else {
            status_label.set_visible(cx, false);
        }
        self.view.redraw(cx);
    }

    fn update_static_texts(&mut self, cx: &mut Cx) {
        let space_name = self.space_name_id.as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        self.view.label(cx, ids!(title)).set_text(
            cx,
            &tr_fmt(self.app_language, "add_existing_room.title", &[("space_name", space_name.as_str())]),
        );
        self.view.label(cx, ids!(subtitle))
            .set_text(cx, tr_key(self.app_language, "add_existing_room.subtitle"));
        self.view.text_input(cx, ids!(filter_input)).set_empty_text(
            cx,
            tr_key(self.app_language, "add_existing_room.filter.placeholder").to_string(),
        );
        self.view.button(cx, ids!(cancel_button))
            .set_text(cx, tr_key(self.app_language, "add_room.button.cancel"));
    }

    pub fn show(
        &mut self,
        cx: &mut Cx,
        space_name_id: RoomNameId,
        existing_children: HashSet<OwnedRoomId>,
        app_language: AppLanguage,
    ) {
        self.app_language = app_language;
        self.space_name_id = Some(space_name_id);
        self.existing_children = existing_children;
        let filter_input = self.view.text_input(cx, ids!(filter_input));
        filter_input.set_text(cx, "");
        self.update_static_texts(cx);
        self.refresh_results(cx, "");
        self.view.button(cx, ids!(cancel_button)).reset_hover(cx);
        filter_input.set_key_focus(cx);
        self.view.redraw(cx);
    }
}

impl AddExistingRoomModalRef {
    pub fn show(
        &self,
        cx: &mut Cx,
        space_name_id: RoomNameId,
        existing_children: HashSet<OwnedRoomId>,
        app_language: AppLanguage,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show(cx, space_name_id, existing_children, app_language);
    }
}

/// The DSL id of the nth result row.
fn result_item_id(index: usize) -> LiveId {
    match index {
        0 => live_id!(result_item_0),
        1 => live_id!(result_item_1),
        2 => live_id!(result_item_2),
        3 => live_id!(result_item_3),
        4 => live_id!(result_item_4),
        5 => live_id!(result_item_5),
        6 => live_id!(result_item_6),
        _ => live_id!(result_item_7),
    }
}
