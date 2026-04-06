//! A modal dialog for inviting a user to a room.

use std::cell::Cell;
use makepad_widgets::*;
use ruma::OwnedUserId;

use crate::app::{AppState, RoomFilterRemoteSearchAction};
use crate::avatar_cache::{self, AvatarCacheEntry};
use crate::i18n::{AppLanguage, tr_fmt, tr_key};
use crate::home::room_screen::InviteResultAction;
use crate::profile::{user_profile::UserProfile, user_profile_cache};
use crate::shared::avatar::AvatarWidgetRefExt;
use crate::sliding_sync::{MatrixRequest, RemoteDirectorySearchKind, RemoteDirectorySearchResult, submit_async_request};
use crate::utils::RoomNameId;

thread_local! {
    static INVITE_MODAL_OPEN: Cell<bool> = const { Cell::new(false) };
}

fn set_invite_modal_open(open: bool) {
    INVITE_MODAL_OPEN.with(|state| state.set(open));
}

pub fn is_invite_modal_open() -> bool {
    INVITE_MODAL_OPEN.with(|state| state.get())
}

pub fn mark_invite_modal_closed() {
    set_invite_modal_open(false);
}


script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    let InviteSearchResultItem = View {
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
                    flow: Flow.Right{wrap: true}
                    draw_text +: {
                        color: (COLOR_TEXT)
                        text_style: REGULAR_TEXT {font_size: 10}
                    }
                    text: ""
                }

                id_label := Label {
                    width: Fill
                    height: Fit
                    flow: Flow.Right{wrap: true}
                    draw_text +: {
                        color: (COLOR_TEXT_INPUT_IDLE)
                        text_style: REGULAR_TEXT {font_size: 8.5}
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
                color: #0000
                color_hover: #FFFFFF22
                color_down: #FFFFFF11
            }
        }
    }


    mod.widgets.InviteModal = #(InviteModal::register_widget(vm)) {
        width: Fit
        height: Fit

        RoundedView {
            width: 400
            height: Fit
            align: Align{x: 0.5}
            flow: Down
            padding: Inset{top: 30, right: 25, bottom: 20, left: 25}

            show_bg: true
            draw_bg +: {
                color: (COLOR_PRIMARY)
                border_radius: 4.0
            }

            title_view := View {
                width: Fill,
                height: Fit,
                padding: Inset{top: 0, bottom: 25}
                align: Align{x: 0.5, y: 0.0}

                title := Label {
                    width: Fill
                    height: Fit
                    align: Align{x: 0.5}
                    flow: Flow.Right{wrap: true},
                    draw_text +: {
                        text_style: TITLE_TEXT {font_size: 13},
                        color: #000
                    }
                    text: ""
                }
            }

            user_id_input := RobrixTextInput {
                draw_text +: {
                    text_style: REGULAR_TEXT {font_size: 11},
                    color: #000
                }
                empty_text: "",
            }

            search_status := Label {
                visible: false
                width: Fill
                height: Fit
                margin: Inset{top: 10, left: 1}
                draw_text +: {
                    text_style: REGULAR_TEXT {font_size: 9.5}
                    color: #6D7682
                }
                text: ""
            }

            search_results_scroll := ScrollYView {
                visible: false
                width: Fill
                height: 200
                margin: Inset{top: 6}

                search_results := View {
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 3

                    result_item_0 := InviteSearchResultItem {}
                    result_item_1 := InviteSearchResultItem {}
                    result_item_2 := InviteSearchResultItem {}
                    result_item_3 := InviteSearchResultItem {}
                    result_item_4 := InviteSearchResultItem {}
                    result_item_5 := InviteSearchResultItem {}
                    result_item_6 := InviteSearchResultItem {}
                    result_item_7 := InviteSearchResultItem {}
                }
            }

            View {
                width: Fill, height: Fit
                flow: Right,
                padding: Inset{top: 20, bottom: 10}
                align: Align{x: 1.0, y: 0.5}
                spacing: 20

                cancel_button := RobrixNeutralIconButton {
                    width: 120,
                    align: Align{x: 0.5, y: 0.5}
                    padding: 12,
                    draw_icon.svg: (ICON_FORBIDDEN)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: ""
                }

                confirm_button := RobrixPositiveIconButton {
                    width: 120
                    align: Align{x: 0.5, y: 0.5}
                    padding: 12,
                    draw_icon.svg: (ICON_ADD_USER)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: ""
                }

                okay_button := RobrixIconButton {
                    visible: false
                    width: 120
                    align: Align{x: 0.5, y: 0.5}
                    padding: 12,
                    draw_icon.svg: (ICON_CHECKMARK)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: ""
                }
            }

            status_label_view := View {
                visible: false
                width: Fill,
                height: Fit,
                align: Align{x: 0.5, y: 0.0}

                status_label := Label {
                    width: Fill,
                    height: Fit,
                    flow: Flow.Right{wrap: true},
                    align: Align{x: 0.5, y: 0.0}
                    margin: Inset{top: 10}
                    draw_text +: {
                        text_style: REGULAR_TEXT {font_size: 11},
                        color: #000
                    }
                    text: ""
                }
            }
        }
    }
}

/// Actions emitted by other widgets to show or hide the `InviteModal`.
#[derive(Clone, Debug)]
pub enum InviteModalAction {
    /// Open the modal to invite a user to the given room or space.
    Open(RoomNameId),
    /// Close the modal.
    Close,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
enum InviteModalState {
    /// Waiting for the user to enter a user ID.
    #[default]
    WaitingForUserInput,
    /// Waiting for the invite to be sent.
    WaitingForInvite(OwnedUserId),
    /// The invite was sent successfully.
    InviteSuccess,
    /// An error occurred while sending the invite.
    InviteError,
}

#[derive(Clone, Debug)]
struct InviteSearchResult {
    user_profile: UserProfile,
}


#[derive(Script, ScriptHook, Widget)]
pub struct InviteModal {
    #[deref] view: View,
    #[rust] state: InviteModalState,
    #[rust] room_name_id: Option<RoomNameId>,
    #[rust] app_language: AppLanguage,
    #[rust] current_search_query: String,
    #[rust] search_results: Vec<InviteSearchResult>,
}

impl Widget for InviteModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if let Some(app_state) = scope.data.get::<AppState>()
            && self.app_language != app_state.app_language
        {
            self.app_language = app_state.app_language;
            self.update_static_texts(cx);
            if let Some(room_name_id) = self.room_name_id.clone() {
                self.set_invite_title(cx, &room_name_id);
            }
        }
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl WidgetMatchEvent for InviteModal {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        let cancel_button = self.view.button(cx, ids!(cancel_button));

        // Handle canceling/closing the modal.
        let cancel_clicked = cancel_button.clicked(actions);
        if cancel_clicked ||
            actions.iter().any(|a| matches!(a.downcast_ref(), Some(ModalAction::Dismissed)))
        {
            set_invite_modal_open(false);
            // If the modal was dismissed by clicking outside of it, we MUST NOT emit
            // a `InviteModalAction::Close` action, as that would cause
            // an infinite action feedback loop.
            if cancel_clicked {
                cx.action(InviteModalAction::Close);
            }
            return;
        }

        // Handle the okay button (shown after invite success).
        let okay_button = self.view.button(cx, ids!(okay_button));
        if okay_button.clicked(actions) {
            set_invite_modal_open(false);
            cx.action(InviteModalAction::Close);
            return;
        }

        let confirm_button = self.view.button(cx, ids!(confirm_button));
        let user_id_input = self.view.text_input(cx, ids!(user_id_input));
        let status_view = self.view.view(cx, ids!(status_label_view));
        let mut status_label = self.view.label(cx, ids!(status_label_view.status_label));

        if let Some(new_query) = user_id_input.changed(actions)
            && self.state == InviteModalState::WaitingForUserInput
        {
            self.update_search_results(cx, &new_query, true);
        }

        if self.state == InviteModalState::WaitingForUserInput
            && let Some(result_index) = self.clicked_search_result_index(cx, actions)
            && let Some(search_result) = self.search_results.get(result_index).cloned()
        {
            user_id_input.set_text(cx, search_result.user_profile.user_id.as_str());
            self.submit_invite_for_user(cx, search_result.user_profile.user_id, &confirm_button, &user_id_input, &status_view, &mut status_label);
            self.view.redraw(cx);
            return;
        }

        // Handle return key or invite button click.
        if let Some(user_id_str) = confirm_button.clicked(actions)
            .then(|| user_id_input.text())
            .or_else(|| user_id_input.returned(actions).map(|(t, _)| t))
        {
            // Validate the user ID
            if user_id_str.is_empty() {
                script_apply_eval!(cx, status_label, {
                    text: #(tr_key(self.app_language, "invite_modal.status.enter_user_id")),
                    draw_text +: {
                        color: mod.widgets.COLOR_FG_DANGER_RED,
                    },
                });
                status_view.set_visible(cx, true);
                self.view.redraw(cx);
                return;
            }

            // Try to parse the user ID
            match ruma::UserId::parse(&user_id_str) {
                Ok(user_id) => {
                    self.submit_invite_for_user(cx, user_id.to_owned(), &confirm_button, &user_id_input, &status_view, &mut status_label);
                }
                Err(_) => {
                    script_apply_eval!(cx, status_label, {
                        text: #(tr_key(self.app_language, "invite_modal.status.invalid_user_id")),
                        draw_text +: {
                            color: mod.widgets.COLOR_FG_DANGER_RED,
                        },
                    });
                    status_view.set_visible(cx, true);
                    user_id_input.set_key_focus(cx);
                }
            }
            self.view.redraw(cx);
        }

        if self.state == InviteModalState::WaitingForUserInput {
            for action in actions {
                match action.downcast_ref() {
                    Some(RoomFilterRemoteSearchAction::Results { query, kind, results })
                        if matches!(kind, RemoteDirectorySearchKind::People)
                            && self.current_search_query == query.trim()
                    => {
                        self.merge_remote_search_results(cx, results);
                    }
                    Some(RoomFilterRemoteSearchAction::Failed { query, kind, error })
                        if matches!(kind, RemoteDirectorySearchKind::People)
                            && self.current_search_query == query.trim()
                    => {
                        let text = format!("Server search failed: {error}");
                        self.view.label(cx, ids!(search_status)).set_text(cx, &text);
                        self.view.label(cx, ids!(search_status)).set_visible(cx, true);
                        self.refresh_search_result_buttons(cx);
                    }
                    _ => {}
                }
            }
        }

        // Handle the result of a previously-sent invite.
        if let InviteModalState::WaitingForInvite(invited_user_id) = &self.state {
            for action in actions {
                let new_state = match action.downcast_ref() {
                    Some(InviteResultAction::Sent { room_id, user_id })
                        if self.room_name_id.as_ref().is_some_and(|rni| rni.room_id() == room_id)
                            && invited_user_id == user_id
                    => {
                        let status = tr_fmt(
                            self.app_language,
                            "invite_modal.status.success_invited",
                            &[("user_id", user_id.as_str())],
                        );
                        script_apply_eval!(cx, status_label, {
                            text: #(status),
                            draw_text +: {
                                color: mod.widgets.COLOR_FG_ACCEPT_GREEN
                            }
                        });
                        status_view.set_visible(cx, true);
                        confirm_button.set_visible(cx, false);
                        cancel_button.set_visible(cx, false);
                        okay_button.set_visible(cx, true);
                        Some(InviteModalState::InviteSuccess)
                    }
                    Some(InviteResultAction::Failed { room_id, user_id, error })
                        if self.room_name_id.as_ref().is_some_and(|rni| rni.room_id() == room_id)
                            && invited_user_id == user_id
                    => {
                        let error_text = error.to_string();
                        let status = tr_fmt(
                            self.app_language,
                            "invite_modal.status.send_failed",
                            &[("error", error_text.as_str())],
                        );
                        script_apply_eval!(cx, status_label, {
                            text: #(status),
                            draw_text +: {
                                color: mod.widgets.COLOR_FG_DANGER_RED,
                            }
                        });
                        status_view.set_visible(cx, true);
                        confirm_button.set_enabled(cx, true);
                        user_id_input.set_is_read_only(cx, false);
                        user_id_input.set_key_focus(cx);
                        Some(InviteModalState::InviteError)
                    }
                    _ => None,
                };
                if let Some(new_state) = new_state {
                    self.state = new_state;
                    self.view.redraw(cx);
                    break;
                }
            }
        }
    }
}

impl InviteModal {
    const SEARCH_RESULT_ITEM_IDS: [LiveId; 8] = [
        live_id!(result_item_0), live_id!(result_item_1),
        live_id!(result_item_2), live_id!(result_item_3),
        live_id!(result_item_4), live_id!(result_item_5),
        live_id!(result_item_6), live_id!(result_item_7),
    ];

    fn clicked_search_result_index(&self, cx: &mut Cx, actions: &Actions) -> Option<usize> {
        let results_view = self.view.view(cx, ids!(search_results_scroll.search_results));
        for (index, item_id) in Self::SEARCH_RESULT_ITEM_IDS.iter().enumerate() {
            if results_view.button(cx, &[*item_id, live_id!(click_button)]).clicked(actions) {
                return Some(index);
            }
        }
        None
    }

    fn update_search_results(
        &mut self,
        cx: &mut Cx,
        query: &str,
        should_search_remote: bool,
    ) {
        let query = query.trim();
        self.current_search_query = query.to_owned();
        self.search_results.clear();

        let search_status = self.view.label(cx, ids!(search_status));
        if query.is_empty() {
            search_status.set_visible(cx, false);
            search_status.set_text(cx, "");
            self.refresh_search_result_buttons(cx);
            return;
        }

        for user_profile in user_profile_cache::search_user_profiles(cx, query, Self::SEARCH_RESULT_ITEM_IDS.len()) {
            self.search_results.push(InviteSearchResult {
                user_profile,
            });
        }

        if should_search_remote {
            submit_async_request(MatrixRequest::SearchDirectory {
                query: query.to_owned(),
                kind: RemoteDirectorySearchKind::People,
                limit: 24,
            });
            let local_count = self.search_results.len();
            let status_text = if local_count > 0 {
                format!("Found {local_count} local result(s). Searching server...")
            } else {
                String::from("Searching local cache and server...")
            };
            search_status.set_text(cx, &status_text);
            search_status.set_visible(cx, true);
        }

        self.refresh_search_result_buttons(cx);
    }

    fn merge_remote_search_results(
        &mut self,
        cx: &mut Cx,
        results: &[RemoteDirectorySearchResult],
    ) {
        for result in results {
            let RemoteDirectorySearchResult::User(user_profile) = result else { continue };
            let already_exists = self.search_results.iter()
                .any(|existing| existing.user_profile.user_id == user_profile.user_id);
            if already_exists {
                continue;
            }
            self.search_results.push(InviteSearchResult {
                user_profile: user_profile.clone(),
            });
        }

        self.search_results.sort_by(|a, b| {
            a.user_profile.displayable_name().to_lowercase()
                .cmp(&b.user_profile.displayable_name().to_lowercase())
                .then_with(|| a.user_profile.user_id.as_str().cmp(b.user_profile.user_id.as_str()))
        });

        let status = if self.search_results.is_empty() {
            String::from("No users found.")
        } else {
            format!("Found {} user(s).", self.search_results.len())
        };
        self.view.label(cx, ids!(search_status)).set_text(cx, &status);
        self.view.label(cx, ids!(search_status)).set_visible(cx, true);
        self.refresh_search_result_buttons(cx);
    }

    fn refresh_search_result_buttons(&mut self, cx: &mut Cx) {
        let results_view = self.view.view(cx, ids!(search_results_scroll.search_results));
        let visible_count = self.search_results.len().min(Self::SEARCH_RESULT_ITEM_IDS.len());
        for (index, item_id) in Self::SEARCH_RESULT_ITEM_IDS.iter().enumerate() {
            let item = results_view.view(cx, &[*item_id]);
            if let Some(result) = self.search_results.get(index) {
                item.label(cx, ids!(row.text_col.name_label))
                    .set_text(cx, result.user_profile.displayable_name());
                item.label(cx, ids!(row.text_col.id_label))
                    .set_text(cx, result.user_profile.user_id.as_str());
                self.set_search_result_avatar(cx, &item, result);
                item.set_visible(cx, true);
                item.button(cx, ids!(click_button)).reset_hover(cx);
            } else {
                item.set_visible(cx, false);
            }
        }
        self.view.view(cx, ids!(search_results_scroll)).set_visible(cx, visible_count > 0);
    }

    fn set_search_result_avatar(
        &self,
        cx: &mut Cx,
        item: &WidgetRef,
        result: &InviteSearchResult,
    ) {
        let avatar = item.avatar(cx, ids!(row.avatar));
        let fallback_text = result.user_profile.displayable_name();
        let mut avatar_state = result.user_profile.avatar_state.clone();

        if let Some(image_data) = avatar_state.update_from_cache(cx) {
            let res = avatar.show_image(
                cx,
                None,
                |cx, img_ref| crate::utils::load_png_or_jpg(&img_ref, cx, image_data),
            );
            if res.is_ok() {
                return;
            }
        }

        if let Some(uri) = avatar_state.uri()
            && let AvatarCacheEntry::Loaded(image_data) = avatar_cache::get_or_fetch_avatar(cx, uri)
        {
            let res = avatar.show_image(
                cx,
                None,
                |cx, img_ref| crate::utils::load_png_or_jpg(&img_ref, cx, &image_data),
            );
            if res.is_ok() {
                return;
            }
        }

        avatar.show_text(cx, None, None, fallback_text);
    }

    fn submit_invite_for_user(
        &mut self,
        cx: &mut Cx,
        user_id: OwnedUserId,
        confirm_button: &ButtonRef,
        user_id_input: &TextInputRef,
        status_view: &ViewRef,
        status_label: &mut LabelRef,
    ) {
        if let Some(room_name_id) = &self.room_name_id {
            submit_async_request(MatrixRequest::InviteUser {
                room_id: room_name_id.room_id().clone(),
                user_id: user_id.clone(),
            });
            self.state = InviteModalState::WaitingForInvite(user_id);
            script_apply_eval!(cx, status_label, {
                text: #(tr_key(self.app_language, "invite_modal.status.sending")),
                draw_text +: {
                    color: mod.widgets.COLOR_ACTIVE_PRIMARY_DARKER,
                },
            });
            status_view.set_visible(cx, true);
            confirm_button.set_enabled(cx, false);
            user_id_input.set_is_read_only(cx, true);
            self.view.view(cx, ids!(search_results_scroll)).set_visible(cx, false);
        }
    }

    fn set_invite_title(&mut self, cx: &mut Cx, room_name_id: &RoomNameId) {
        let room_name = room_name_id.to_string();
        let title = tr_fmt(
            self.app_language,
            "invite_modal.title.invite_to_room_name",
            &[("room_name", room_name.as_str())],
        );
        self.view.label(cx, ids!(title)).set_text(cx, &title);
    }

    fn update_static_texts(&mut self, cx: &mut Cx) {
        self.view.button(cx, ids!(cancel_button))
            .set_text(cx, tr_key(self.app_language, "invite_modal.button.cancel"));
        self.view.button(cx, ids!(confirm_button))
            .set_text(cx, tr_key(self.app_language, "invite_modal.button.invite"));
        self.view.button(cx, ids!(okay_button))
            .set_text(cx, tr_key(self.app_language, "invite_modal.button.okay"));
        self.view.text_input(cx, ids!(user_id_input))
            .set_empty_text(cx, tr_key(self.app_language, "invite_modal.input.placeholder").to_string());
    }

    pub fn show(&mut self, cx: &mut Cx, room_name_id: RoomNameId, app_language: AppLanguage) {
        set_invite_modal_open(true);
        self.app_language = app_language;
        self.set_invite_title(cx, &room_name_id);
        self.update_static_texts(cx);
        self.state = InviteModalState::WaitingForUserInput;
        self.room_name_id = Some(room_name_id);
        self.current_search_query.clear();
        self.search_results.clear();

        // Reset the UI state
        let confirm_button = self.view.button(cx, ids!(confirm_button));
        let cancel_button = self.view.button(cx, ids!(cancel_button));
        let okay_button = self.view.button(cx, ids!(okay_button));
        let user_id_input = self.view.text_input(cx, ids!(user_id_input));
        confirm_button.set_visible(cx, true);
        confirm_button.set_enabled(cx, true);
        confirm_button.reset_hover(cx);
        cancel_button.set_visible(cx, true);
        cancel_button.set_enabled(cx, true);
        cancel_button.reset_hover(cx);
        okay_button.set_visible(cx, false);
        okay_button.reset_hover(cx);
        user_id_input.set_is_read_only(cx, false);
        user_id_input.set_text(cx, "");
        self.view.view(cx, ids!(status_label_view)).set_visible(cx, false);
        self.view.label(cx, ids!(status_label_view.status_label)).set_text(cx, "");
        self.view.label(cx, ids!(search_status)).set_visible(cx, false);
        self.view.label(cx, ids!(search_status)).set_text(cx, "");
        self.refresh_search_result_buttons(cx);
        self.view.redraw(cx);
        user_id_input.set_key_focus(cx);
    }
}

impl InviteModalRef {
    pub fn show(&self, cx: &mut Cx, room_name_id: RoomNameId, app_language: AppLanguage) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show(cx, room_name_id, app_language);
    }
}
