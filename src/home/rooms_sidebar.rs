//! The RoomsSideBar is the widget that contains the RoomsList and other items.
//!
//! It differs in what content it includes based on the adaptive view:
//! * On a narrow mobile view, it acts as the root_view of StackNavigation
//!   * It includes a title label, a search bar, and the RoomsList.
//! * On a wide desktop view, it acts as a permanent tab that is on the left side of the dock.
//!   * It only includes a title label and the RoomsList, because the SearcBar
//!     is at the top of the HomeScreen in Desktop view.

use makepad_widgets::*;
use matrix_sdk::ruma::OwnedMxcUri;

use crate::{
    app::{AppState, RoomFilterRemoteSearchAction},
    avatar_cache::{self, AvatarCacheEntry},
    home::{
        rooms_list_header::RoomsListHeaderWidgetExt,
        rooms_list::{RoomsListRef, RoomsListWidgetExt},
        spaces_bar::SpacesBarRef,
    },
    i18n::{AppLanguage, tr_fmt, tr_key},
    join_leave_room_modal::{JoinLeaveModalKind, JoinLeaveRoomModalAction},
    profile::user_profile::UserProfile,
    room::BasicRoomDetails,
    shared::{
        avatar::{AvatarState, AvatarWidgetRefExt},
        room_filter_input_bar::{MainFilterAction, RoomFilterInputBarWidgetExt},
    },
    sliding_sync::{
        MatrixRequest, RemoteDirectorySearchKind, RemoteDirectorySearchResult,
        current_user_id, submit_async_request,
    },
    utils::RoomNameId,
};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    let MobileRoomFilterResultItem = View {
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
                    draw_text +: {
                        color: (COLOR_TEXT)
                        text_style: REGULAR_TEXT {font_size: 10}
                    }
                }

                id_label := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (COLOR_TEXT_INPUT_IDLE)
                        text_style: REGULAR_TEXT {font_size: 8.5}
                    }
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


    mod.widgets.RoomsSideBar = #(RoomsSideBar::register_widget(vm)) {
        Desktop := SolidView {
            padding: Inset{top: 20, left: 10, right: 10}
            flow: Down, spacing: 5
            width: Fill, height: Fill

            draw_bg.color: (COLOR_PRIMARY_DARKER)

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
            
            RoundedShadowView {
                width: Fill, height: Fit
                padding: Inset{top: 15, left: 15, right: 15, bottom: 10}
                flow: Down,

                show_bg: true
                draw_bg +: {
                    color: (COLOR_PRIMARY_DARKER)
                    border_radius: 4.0
                    border_size: 0.0
                    shadow_color: #0005
                    shadow_radius: 12.0
                    shadow_offset: vec2(0.0, 0.0)

                    pixel: fn() {
                        let sdf = Sdf2d.viewport(self.pos * self.rect_size3)

                        let mut fill_color = self.color
                        if self.color_2.x > -0.5 {
                            let dither = Math.random_2d(self.pos.xy) * 0.04 * self.color_dither
                            let dir = if self.gradient_fill_horizontal > 0.5 self.pos.x else self.pos.y
                            fill_color = mix(self.color self.color_2 dir + dither)
                        }

                        let mut stroke_color = self.border_color
                        if self.border_color_2.x > -0.5 {
                            let dither = Math.random_2d(self.pos.xy) * 0.04 * self.color_dither
                            let dir = if self.gradient_border_horizontal > 0.5 self.pos.x else self.pos.y
                            stroke_color = mix(self.border_color self.border_color_2 dir + dither)
                        }

                        sdf.box(
                            self.sdf_rect_pos.x
                            self.sdf_rect_pos.y
                            self.sdf_rect_size.x
                            self.sdf_rect_size.y
                            max(1.0 self.border_radius)
                        )
                        if sdf.shape > -1.0 {
                            let m = self.shadow_radius
                            let o = self.shadow_offset + self.rect_shift
                            let v = GaussShadow.rounded_box_shadow(vec2(m) + o self.rect_size2+o self.pos * (self.rect_size3+vec2(m)) self.shadow_radius*0.5 self.border_radius*2.0)
                            // Only draw shadow on the bottom half of the view
                            let pixel_y = self.pos.y * self.rect_size3.y
                            let mid_y = self.sdf_rect_pos.y + self.sdf_rect_size.y * 0.5
                            let bottom_mask = smoothstep(mid_y - m * 0.3 mid_y + m * 0.3 pixel_y)
                            sdf.clear(self.shadow_color * v * bottom_mask)
                        }

                        sdf.fill_keep(fill_color)

                        if self.border_size > 0.0 {
                            sdf.stroke(stroke_color self.border_size)
                        }
                        return sdf.result
                    }
                }

                CachedWidget {
                    rooms_list_header := RoomsListHeader {
                        open_room_filter_modal_button +: {
                            visible: false
                        }
                    }
                }

                View {
                    width: Fill,
                    height: 45,
                    flow: Right
                    padding: Inset{top: 5, bottom: 2}
                    spacing: 0 
                    align: Align{y: 0.5}

                    CachedWidget {
                        room_filter_input_bar := RoomFilterInputBar {
                            search_icon +: {
                                visible: false
                            }
                        }
                    }
                }

                mobile_inline_search_panel := View {
                    visible: false
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 4
                    margin: Inset{top: 4}

                    search_results_empty := Label {
                        visible: false
                        width: Fill,
                        height: Fit,
                        flow: Flow.Right{wrap: true},
                        text: ""
                        draw_text +: {
                            color: (COLOR_TEXT)
                            text_style: REGULAR_TEXT {font_size: 10}
                        }
                    }

                    remote_search_hint := Label {
                        visible: false
                        width: Fill,
                        height: Fit,
                        text: ""
                        draw_text +: {
                            color: (COLOR_TEXT_INPUT_IDLE)
                            text_style: REGULAR_TEXT {font_size: 9.5}
                        }
                    }

                    remote_search_options := View {
                        visible: false
                        width: Fill
                        height: Fit
                        flow: Right
                        spacing: 6
                        margin: Inset{top: 2}

                        mobile_remote_search_people_button := RobrixNeutralIconButton {
                            width: Fit,
                            text: ""
                        }
                        mobile_remote_search_rooms_button := RobrixNeutralIconButton {
                            width: Fit,
                            text: ""
                        }
                        mobile_remote_search_spaces_button := RobrixNeutralIconButton {
                            width: Fit,
                            text: ""
                        }
                    }

                    search_results_list := View {
                        visible: false
                        width: Fill
                        height: Fit
                        flow: Down
                        spacing: 3

                        result_item_0 := MobileRoomFilterResultItem {}
                        result_item_1 := MobileRoomFilterResultItem {}
                        result_item_2 := MobileRoomFilterResultItem {}
                        result_item_3 := MobileRoomFilterResultItem {}
                        result_item_4 := MobileRoomFilterResultItem {}
                        result_item_5 := MobileRoomFilterResultItem {}
                        result_item_6 := MobileRoomFilterResultItem {}
                        result_item_7 := MobileRoomFilterResultItem {}
                    }
                }
            }

            rooms_list_container := View {
                padding: Inset{left: 15, right: 15}

                CachedWidget {
                    rooms_list := RoomsList {}
                }
            }
        }
    }
}

#[derive(Clone)]
enum MobileInlineSearchResultTarget {
    RemoteSpace { space_name_id: RoomNameId, avatar_uri: Option<OwnedMxcUri> },
    RemoteRoom { room_name_id: RoomNameId, avatar_uri: Option<OwnedMxcUri> },
    RemoteUser(UserProfile),
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
    #[rust] app_language: AppLanguage,
    #[rust(Timer::empty())] mobile_inline_search_debounce_timer: Timer,
    #[rust] pending_mobile_inline_search_keywords: String,
    #[rust] mobile_inline_search_results: Vec<MobileInlineSearchResultTarget>,
}

impl ScriptHook for RoomsSideBar {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        vm.with_cx_mut(|cx| {
            // Here we set the global singleton for the RoomsList widget,
            // which is used to access the list of rooms from anywhere in the app.
            cx.set_global(self.view.rooms_list(cx, ids!(rooms_list)));
            self.set_app_language(cx, AppLanguage::default());
            self.sync_adaptive_search_ui(cx);
        });
    }
}
impl Widget for RoomsSideBar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }
        self.sync_adaptive_search_ui(cx);

        if self.mobile_inline_search_debounce_timer.is_event(event).is_some() {
            self.mobile_inline_search_debounce_timer = Timer::empty();
            let keywords = std::mem::take(&mut self.pending_mobile_inline_search_keywords);
            self.update_mobile_inline_local_state(cx, &keywords);
        }

        // If the main room filter input bar changed keywords, re-emit that action
        // as a MainFilterAction so that other widgets can handle it.
        if let Event::Actions(actions) = event {
            if let Some(keywords) = self.view.room_filter_input_bar(cx, ids!(room_filter_input_bar)).changed(actions) {
                cx.action(MainFilterAction::Changed(keywords.clone()));
                cx.stop_timer(self.mobile_inline_search_debounce_timer);
                if keywords.is_empty() {
                    self.pending_mobile_inline_search_keywords.clear();
                    self.mobile_inline_search_debounce_timer = Timer::empty();
                    self.update_mobile_inline_local_state(cx, "");
                } else {
                    self.pending_mobile_inline_search_keywords = keywords;
                    self.mobile_inline_search_debounce_timer = cx.start_timeout(0.12);
                }
            }

            if let Some(clicked_index) = self.clicked_mobile_inline_result_index(cx, actions) {
                if let Some(target) = self.mobile_inline_search_results.get(clicked_index).cloned() {
                    match target {
                        MobileInlineSearchResultTarget::RemoteSpace { space_name_id, .. } => {
                            cx.action(JoinLeaveRoomModalAction::Open {
                                kind: JoinLeaveModalKind::JoinRoom {
                                    details: BasicRoomDetails::Name(space_name_id),
                                    is_space: true,
                                },
                                show_tip: false,
                            });
                        }
                        MobileInlineSearchResultTarget::RemoteRoom { room_name_id, .. } => {
                            cx.action(JoinLeaveRoomModalAction::Open {
                                kind: JoinLeaveModalKind::JoinRoom {
                                    details: BasicRoomDetails::Name(room_name_id),
                                    is_space: false,
                                },
                                show_tip: false,
                            });
                        }
                        MobileInlineSearchResultTarget::RemoteUser(user_profile) => {
                            let create_encrypted = scope.data.get::<AppState>()
                                .map(|app_state| {
                                    app_state.bot_settings.should_create_encrypted_dm(
                                        user_profile.user_id.as_ref(),
                                        current_user_id().as_deref(),
                                    )
                                })
                                .unwrap_or(false);
                            submit_async_request(MatrixRequest::OpenOrCreateDirectMessage {
                                create_encrypted,
                                user_profile,
                                allow_create: false,
                            });
                        }
                    }
                    return;
                }
            }

            if let Some(kind) = self.clicked_mobile_inline_remote_option(cx, actions) {
                let query = self.current_mobile_filter_keywords(cx);
                if !query.is_empty() {
                    let kind_text = match &kind {
                        RemoteDirectorySearchKind::People => tr_key(self.app_language, "app.room_filter.remote.kind.people"),
                        RemoteDirectorySearchKind::Rooms => tr_key(self.app_language, "app.room_filter.remote.kind.rooms"),
                        RemoteDirectorySearchKind::Spaces => tr_key(self.app_language, "app.room_filter.remote.kind.spaces"),
                    };
                    let searching_text = tr_fmt(self.app_language, "app.room_filter.searching_remote", &[("kind", kind_text)]);
                    self.mobile_inline_search_results.clear();
                    self.refresh_mobile_inline_result_buttons(cx);
                    self.set_mobile_inline_state(cx, &searching_text, false, false);
                    self.set_mobile_rooms_list_visible(cx, false);
                    submit_async_request(MatrixRequest::SearchDirectory {
                        query,
                        kind,
                        limit: 16,
                    });
                }
                return;
            }

            for action in actions {
                match action.downcast_ref() {
                    Some(RoomFilterRemoteSearchAction::Results { query, kind: _, results }) => {
                        if self.current_mobile_filter_keywords(cx) != query.trim() {
                            continue;
                        }
                        self.mobile_inline_search_results.clear();
                        for result in results {
                            match result {
                                RemoteDirectorySearchResult::User(user_profile) => {
                                    self.mobile_inline_search_results.push(MobileInlineSearchResultTarget::RemoteUser(user_profile.clone()));
                                }
                                RemoteDirectorySearchResult::Room { room_name_id, avatar_uri } => {
                                    self.mobile_inline_search_results.push(MobileInlineSearchResultTarget::RemoteRoom {
                                        room_name_id: room_name_id.clone(),
                                        avatar_uri: avatar_uri.clone(),
                                    });
                                }
                                RemoteDirectorySearchResult::Space { space_name_id, avatar_uri } => {
                                    self.mobile_inline_search_results.push(MobileInlineSearchResultTarget::RemoteSpace {
                                        space_name_id: space_name_id.clone(),
                                        avatar_uri: avatar_uri.clone(),
                                    });
                                }
                            }
                            if self.mobile_inline_search_results.len() >= Self::MOBILE_INLINE_RESULT_ITEM_IDS.len() {
                                break;
                            }
                        }
                        self.refresh_mobile_inline_result_buttons(cx);
                        if self.mobile_inline_search_results.is_empty() {
                            self.set_mobile_inline_state(
                                cx,
                                &tr_fmt(self.app_language, "app.room_filter.no_server_results", &[
                                    ("query", query),
                                ]),
                                true,
                                false,
                            );
                        } else {
                            self.set_mobile_inline_state(cx, "", false, true);
                        }
                        self.set_mobile_rooms_list_visible(cx, false);
                        continue;
                    }
                    Some(RoomFilterRemoteSearchAction::Failed { query, kind: _, error }) => {
                        if self.current_mobile_filter_keywords(cx) != query.trim() {
                            continue;
                        }
                        self.mobile_inline_search_results.clear();
                        self.refresh_mobile_inline_result_buttons(cx);
                        self.set_mobile_inline_state(
                            cx,
                            &tr_fmt(self.app_language, "app.room_filter.search_remote_failed", &[
                                ("error", error),
                            ]),
                            true,
                            false,
                        );
                        self.set_mobile_rooms_list_visible(cx, false);
                        continue;
                    }
                    _ => {}
                }
            }
        }
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.sync_adaptive_search_ui(cx);
        self.view.draw_walk(cx, scope, walk)
    }
}

impl RoomsSideBar {
    const MOBILE_INLINE_RESULT_ITEM_IDS: [LiveId; 8] = [
        live_id!(result_item_0), live_id!(result_item_1),
        live_id!(result_item_2), live_id!(result_item_3),
        live_id!(result_item_4), live_id!(result_item_5),
        live_id!(result_item_6), live_id!(result_item_7),
    ];

    fn set_app_language(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        self.app_language = app_language;
        self.sync_mobile_remote_option_labels(cx);
        self.view.redraw(cx);
    }

    fn sync_adaptive_search_ui(&self, cx: &mut Cx) {
        let is_desktop = cx.display_context.is_desktop();
        let mobile_filter_visible = self.view.view(cx, ids!(room_filter_input_bar)).visible();
        let show_desktop_search_controls = is_desktop && !mobile_filter_visible;
        self.view.rooms_list_header(cx, ids!(rooms_list_header))
            .set_open_room_filter_button_visible(cx, show_desktop_search_controls);
        self.view.room_filter_input_bar(cx, ids!(room_filter_input_bar))
            .set_search_icon_visible(cx, show_desktop_search_controls);
    }

    fn sync_mobile_remote_option_labels(&self, cx: &mut Cx) {
        self.view.label(cx, ids!(mobile_inline_search_panel.remote_search_hint))
            .set_text(cx, tr_key(self.app_language, "app.room_filter.remote.hint"));
        let options_view = self.view.view(cx, ids!(mobile_inline_search_panel.remote_search_options));
        options_view.button(cx, ids!(mobile_remote_search_people_button))
            .set_text(cx, tr_key(self.app_language, "app.room_filter.remote.people"));
        options_view.button(cx, ids!(mobile_remote_search_rooms_button))
            .set_text(cx, tr_key(self.app_language, "app.room_filter.remote.rooms"));
        options_view.button(cx, ids!(mobile_remote_search_spaces_button))
            .set_text(cx, tr_key(self.app_language, "app.room_filter.remote.spaces"));
    }

    fn current_mobile_filter_keywords(&self, cx: &mut Cx) -> String {
        self.view
            .text_input(cx, ids!(room_filter_input_bar.input))
            .text()
            .trim()
            .to_owned()
    }

    fn clicked_mobile_inline_result_index(&self, cx: &mut Cx, actions: &Actions) -> Option<usize> {
        let list_view = self.view.view(cx, ids!(mobile_inline_search_panel.search_results_list));
        for (index, item_id) in Self::MOBILE_INLINE_RESULT_ITEM_IDS.iter().enumerate() {
            if list_view.button(cx, &[*item_id, live_id!(click_button)]).clicked(actions) {
                return Some(index);
            }
        }
        None
    }

    fn clicked_mobile_inline_remote_option(&self, cx: &mut Cx, actions: &Actions) -> Option<RemoteDirectorySearchKind> {
        let options_view = self.view.view(cx, ids!(mobile_inline_search_panel.remote_search_options));
        if options_view.button(cx, ids!(mobile_remote_search_people_button)).clicked(actions) {
            return Some(RemoteDirectorySearchKind::People);
        }
        if options_view.button(cx, ids!(mobile_remote_search_rooms_button)).clicked(actions) {
            return Some(RemoteDirectorySearchKind::Rooms);
        }
        if options_view.button(cx, ids!(mobile_remote_search_spaces_button)).clicked(actions) {
            return Some(RemoteDirectorySearchKind::Spaces);
        }
        None
    }

    fn set_mobile_rooms_list_visible(&self, cx: &mut Cx, visible: bool) {
        self.view.view(cx, ids!(rooms_list_container)).set_visible(cx, visible);
    }

    fn set_mobile_inline_state(
        &self,
        cx: &mut Cx,
        text: &str,
        show_remote_options: bool,
        show_results_list: bool,
    ) {
        self.sync_mobile_remote_option_labels(cx);
        let empty_label = self.view.label(cx, ids!(mobile_inline_search_panel.search_results_empty));
        empty_label.set_visible(cx, !text.is_empty());
        if !text.is_empty() {
            empty_label.set_text(cx, text);
        }
        self.view.label(cx, ids!(mobile_inline_search_panel.remote_search_hint))
            .set_visible(cx, show_remote_options);
        self.view.view(cx, ids!(mobile_inline_search_panel.remote_search_options))
            .set_visible(cx, show_remote_options);
        self.view.view(cx, ids!(mobile_inline_search_panel.search_results_list))
            .set_visible(cx, show_results_list);
        self.view.view(cx, ids!(mobile_inline_search_panel))
            .set_visible(cx, !text.is_empty() || show_remote_options || show_results_list);
    }

    fn set_mobile_inline_result_avatar(
        &self,
        cx: &mut Cx,
        avatar_ref: &crate::shared::avatar::AvatarRef,
        fallback_text: &str,
        remote_avatar_uri: Option<&OwnedMxcUri>,
        remote_avatar_state: Option<&AvatarState>,
    ) {
        if let Some(avatar_state) = remote_avatar_state {
            if let Some(image_data) = avatar_state.data() {
                let res = avatar_ref.show_image(
                    cx,
                    None,
                    |cx, img_ref| crate::utils::load_png_or_jpg(&img_ref, cx, image_data),
                );
                if res.is_ok() {
                    return;
                }
            }
            if let Some(uri) = avatar_state.uri() {
                if let AvatarCacheEntry::Loaded(image_data) = avatar_cache::get_or_fetch_avatar(cx, uri) {
                    let res = avatar_ref.show_image(
                        cx,
                        None,
                        |cx, img_ref| crate::utils::load_png_or_jpg(&img_ref, cx, &image_data),
                    );
                    if res.is_ok() {
                        return;
                    }
                }
            }
        }

        if let Some(uri) = remote_avatar_uri {
            if let AvatarCacheEntry::Loaded(image_data) = avatar_cache::get_or_fetch_avatar(cx, uri) {
                let res = avatar_ref.show_image(
                    cx,
                    None,
                    |cx, img_ref| crate::utils::load_png_or_jpg(&img_ref, cx, &image_data),
                );
                if res.is_ok() {
                    return;
                }
            }
        }

        avatar_ref.show_text(cx, None, None, fallback_text);
    }

    fn refresh_mobile_inline_result_buttons(&self, cx: &mut Cx) {
        let list_view = self.view.view(cx, ids!(mobile_inline_search_panel.search_results_list));
        for (index, item_id) in Self::MOBILE_INLINE_RESULT_ITEM_IDS.iter().enumerate() {
            let item = list_view.view(cx, &[*item_id]);
            if let Some(target) = self.mobile_inline_search_results.get(index) {
                let (name, raw_id) = match target {
                    MobileInlineSearchResultTarget::RemoteSpace { space_name_id, .. }
                    | MobileInlineSearchResultTarget::RemoteRoom { room_name_id: space_name_id, .. } => {
                        (space_name_id.to_string(), space_name_id.room_id().to_string())
                    }
                    MobileInlineSearchResultTarget::RemoteUser(user_profile) => {
                        (user_profile.displayable_name().to_owned(), user_profile.user_id.to_string())
                    }
                };

                item.label(cx, ids!(row.text_col.name_label)).set_text(cx, &name);
                item.label(cx, ids!(row.text_col.id_label)).set_text(cx, &raw_id);

                let avatar_ref = item.avatar(cx, ids!(row.avatar));
                match target {
                    MobileInlineSearchResultTarget::RemoteSpace { avatar_uri, .. }
                    | MobileInlineSearchResultTarget::RemoteRoom { avatar_uri, .. } => {
                        self.set_mobile_inline_result_avatar(cx, &avatar_ref, &name, avatar_uri.as_ref(), None);
                    }
                    MobileInlineSearchResultTarget::RemoteUser(user_profile) => {
                        self.set_mobile_inline_result_avatar(
                            cx,
                            &avatar_ref,
                            &name,
                            None,
                            Some(&user_profile.avatar_state),
                        );
                    }
                }

                item.set_visible(cx, true);
            } else {
                item.set_visible(cx, false);
            }
        }
    }

    fn update_mobile_inline_local_state(&mut self, cx: &mut Cx, keywords: &str) {
        let keywords = keywords.trim();
        self.mobile_inline_search_results.clear();
        self.refresh_mobile_inline_result_buttons(cx);
        if keywords.is_empty() {
            self.set_mobile_inline_state(cx, "", false, false);
            self.set_mobile_rooms_list_visible(cx, true);
            return;
        }

        let max_results = Self::MOBILE_INLINE_RESULT_ITEM_IDS.len();
        let mut local_result_count = 0;
        let space_items = cx.get_global::<SpacesBarRef>()
            .get_matching_space_items(keywords, 4);
        for _ in &space_items {
            local_result_count += 1;
            if local_result_count >= max_results {
                break;
            }
        }
        if local_result_count < max_results {
            let room_items = cx.get_global::<RoomsListRef>()
                .get_matching_room_items(keywords, max_results - local_result_count);
            local_result_count += room_items.len();
        }

        if local_result_count == 0 {
            self.set_mobile_inline_state(
                cx,
                &tr_fmt(
                    self.app_language,
                    "app.room_filter.no_local_results",
                    &[("keywords", keywords)],
                ),
                true,
                false,
            );
            self.set_mobile_rooms_list_visible(cx, false);
        } else {
            self.set_mobile_inline_state(cx, "", true, false);
            self.set_mobile_rooms_list_visible(cx, true);
        }
    }
}
