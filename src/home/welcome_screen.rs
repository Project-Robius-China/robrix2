//! The Home screen shown in the desktop dock's permanent "Home" tab.
//!
//! This is a lightweight dashboard: a personalized greeting, quick-action
//! entry points (create room, start chat, join room, explore public rooms),
//! a "recent conversations" list, and a small community/about card.
//!
//! The recent-conversations data is a read-only snapshot pulled from the
//! global [`RoomsListRef`]; it is refreshed whenever a background update
//! signal arrives (the same signal that drives the RoomsList itself).

use makepad_widgets::*;
use crate::{
    app::{AppState, AppStateAction},
    home::{
        add_room::{CreateRoomModalAction, StartChatModalAction},
        navigation_tab_bar::{NavigationBarAction, get_own_profile},
        rooms_list::{RecentRoomInfo, RoomsListRef},
    },
    i18n::{AppLanguage, tr_fmt, tr_key},
    logout::logout_confirm_modal::LogoutAction,
    room::{BasicRoomDetails, FetchedRoomAvatar},
    shared::{
        avatar::AvatarWidgetExt,
        unread_badge::UnreadBadgeWidgetExt,
    },
    sliding_sync::AccountSwitchAction,
    utils::{self, relative_format},
};

/// The maximum number of rooms shown in the "Recent conversations" section.
///
/// Must match the number of `recent_room_row_*` views declared in the DSL below.
const MAX_RECENT_ROOMS: usize = 5;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*


    // A section heading shown above each group of home-screen content.
    mod.widgets.HomeSectionTitle = Label {
        width: Fit, height: Fit
        draw_text +: {
            color: (RBX_FG_PRIMARY)
            text_style: RBX_TEXT_SECTION_TITLE {}
        }
        text: ""
    }

    // A quick-action entry tile: subtle inset surface + soft stroke + teal icon,
    // matching the in-card button treatment used by the agent settings tiles.
    // Derived from RobrixIconButton so hover/press states come for free.
    mod.widgets.HomeQuickActionButton = mod.widgets.RobrixIconButton {
        width: 168, height: Fit
        padding: Inset{top: 14, bottom: 14, left: 14, right: 14}
        spacing: 10
        align: Align{x: 0.0, y: 0.5}
        draw_bg +: {
            color: (RBX_BG_SURFACE_SUBTLE)
            color_hover: (RBX_BG_HOVER)
            color_down: (RBX_BG_PRESSED)
            border_size: 1.0
            border_color: (RBX_STROKE_SOFT)
            border_color_hover: (RBX_STROKE_STRONG)
            border_color_down: (RBX_STROKE_STRONG)
            border_radius: (RBX_RADIUS_MD)
        }
        draw_icon +: { color: (RBX_ACCENT) }
        icon_walk: Walk{width: (RBX_ICON_SM), height: (RBX_ICON_SM)}
        draw_text +: {
            color: (RBX_FG_PRIMARY)
            color_hover: (RBX_FG_PRIMARY)
            color_down: (RBX_FG_PRIMARY)
            text_style: RBX_TEXT_BODY_STRONG {}
        }
    }

    // One row in the "Recent conversations" card:
    // [avatar] [room name + latest-message preview] [timestamp + unread badge].
    // All rows are declared statically and populated/hidden from Rust.
    // One recent-conversation row. The preview is deliberately a single-line
    // PLAIN TEXT label (HTML stripped in Rust via `utils::html_preview_text`):
    // a multi-paragraph HTML preview widget reserves layout height for all of
    // its blocks, which either stretches the row or gets sliced mid-line by a
    // fixed-height clip. A one-line Label with `max_lines: 1` + Ellipsis
    // truncates reliably, and lets the row keep a natural Fit height.
    mod.widgets.HomeRecentRoomRow = View {
        visible: false
        width: Fill, height: Fit
        flow: Right, spacing: 10
        padding: 10
        align: Align{y: 0.5}
        cursor: MouseCursor.Hand

        avatar := Avatar {}

        room_info := View {
            width: Fill, height: Fit
            flow: Down, spacing: 4
            align: Align{x: 0.0, y: 0.0}

            top_row := View {
                width: Fill, height: Fit
                flow: Right, spacing: (SPACE_SM)

                room_name := Label {
                    width: Fill, height: Fit
                    flow: Flow.Right{wrap: false}
                    max_lines: 1
                    text_overflow: Ellipsis
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: RBX_TEXT_BODY_STRONG {}
                    }
                    text: ""
                }

                timestamp := Label {
                    width: Fit, height: Fit
                    padding: Inset{top: 1}
                    flow: Flow.Right{wrap: false}
                    draw_text +: {
                        color: (RBX_FG_TERTIARY)
                        text_style: RBX_TEXT_META {}
                    }
                    text: ""
                }
            }

            bottom_row := View {
                width: Fill, height: Fit
                flow: Right, spacing: (SPACE_SM)
                align: Align{y: 0.5}

                preview := Label {
                    width: Fill, height: Fit
                    flow: Flow.Right{wrap: false}
                    max_lines: 1
                    text_overflow: Ellipsis
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_BODY {}
                    }
                    text: ""
                }

                badge_wrap := View {
                    width: Fit, height: Fit
                    align: Align{x: 1.0}
                    unread_badge := UnreadBadge {}
                }
            }
        }
    }

    // A hairline divider between recent-conversation rows,
    // inset to align with the text column (row pad 10 + avatar 36 + spacing 10).
    mod.widgets.HomeRecentRoomDivider = SolidView {
        visible: false
        width: Fill, height: 1
        margin: Inset{left: 56}
        draw_bg.color: (RBX_DIVIDER)
    }

    mod.widgets.WelcomeScreen = #(WelcomeScreen::register_widget(vm)) {
        // Must be a SolidView: a plain View has no background shader, so its
        // draw_bg is dead and the grey desktop backdrop (COLOR_SECONDARY)
        // shows through the dock panel.
        ..mod.widgets.SolidView

        width: Fill, height: Fill
        flow: Down
        align: Align{x: 0.5}

        show_bg: true,
        // Same white surface as the rooms sidebar and the other dock panels.
        draw_bg +: { color: (RBX_BG_SURFACE) }

        // make this a ScrollYView
        scroll_bars: mod.widgets.ScrollBars {
            show_scroll_x: false show_scroll_y: true
            scroll_bar_y.drag_scrolling: true
        }

        content_column := View {
            width: Fill{max: 720}, height: Fit
            flow: Down, spacing: (SPACE_XL)
            padding: Inset{top: (SPACE_XL), bottom: (RBX_SPACE_2XL), left: (RBX_SPACE_2XL), right: (RBX_SPACE_2XL)}

            hero := View {
                width: Fill, height: Fit
                flow: Down, spacing: 6

                greeting := Label {
                    width: Fill, height: Fit
                    flow: Flow.Right{wrap: true}
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: RBX_TEXT_PAGE_TITLE {}
                    }
                    text: ""
                }

                subtitle := Label {
                    width: Fill, height: Fit
                    flow: Flow.Right{wrap: true}
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_BODY {}
                    }
                    text: ""
                }
            }

            quick_actions_section := View {
                width: Fill, height: Fit
                flow: Down, spacing: (SPACE_MD)

                quick_actions_title := mod.widgets.HomeSectionTitle {}

                quick_actions_row := View {
                    width: Fill, height: Fit
                    flow: Flow.Right{wrap: true}
                    spacing: (SPACE_MD)

                    qa_create_room := mod.widgets.HomeQuickActionButton {
                        draw_icon.svg: (ICON_ADD)
                    }
                    qa_start_chat := mod.widgets.HomeQuickActionButton {
                        draw_icon.svg: (ICON_ADD_USER)
                    }
                    qa_join_room := mod.widgets.HomeQuickActionButton {
                        draw_icon.svg: (ICON_JOIN_ROOM)
                    }
                    qa_explore := mod.widgets.HomeQuickActionButton {
                        draw_icon.svg: (ICON_GLOBE)
                    }
                }
            }

            recent_section := View {
                width: Fill, height: Fit
                flow: Down, spacing: (SPACE_MD)

                recent_title := mod.widgets.HomeSectionTitle {}

                // SectionCard recipe (visual spec §4.1): white shell + soft stroke.
                recent_card := RoundedView {
                    width: Fill, height: Fit, flow: Down
                    padding: Inset{left: (SPACE_SM), right: (SPACE_SM), top: (SPACE_SM), bottom: (SPACE_SM)}
                    show_bg: true
                    draw_bg +: {
                        color: (RBX_BG_SURFACE)
                        border_radius: (RBX_RADIUS_MD)
                        border_size: 1.0
                        border_color: (RBX_STROKE_SOFT)
                    }

                    // Shown while rooms are still loading, or when there are none.
                    recent_status_view := View {
                        width: Fill, height: Fit
                        padding: Inset{left: (SPACE_SM), right: (SPACE_SM), top: (SPACE_MD), bottom: (SPACE_MD)}

                        recent_status_label := Label {
                            width: Fill, height: Fit
                            flow: Flow.Right{wrap: true}
                            draw_text +: {
                                color: (RBX_FG_TERTIARY)
                                text_style: RBX_TEXT_BODY {}
                            }
                            text: ""
                        }
                    }

                    recent_room_row_0 := mod.widgets.HomeRecentRoomRow {}
                    recent_divider_0 := mod.widgets.HomeRecentRoomDivider {}
                    recent_room_row_1 := mod.widgets.HomeRecentRoomRow {}
                    recent_divider_1 := mod.widgets.HomeRecentRoomDivider {}
                    recent_room_row_2 := mod.widgets.HomeRecentRoomRow {}
                    recent_divider_2 := mod.widgets.HomeRecentRoomDivider {}
                    recent_room_row_3 := mod.widgets.HomeRecentRoomRow {}
                    recent_divider_3 := mod.widgets.HomeRecentRoomDivider {}
                    recent_room_row_4 := mod.widgets.HomeRecentRoomRow {}
                }
            }

            community_section := RoundedView {
                width: Fill, height: Fit, flow: Down
                spacing: (SPACE_SM)
                padding: Inset{left: (SPACE_LG), right: (SPACE_LG), top: (SPACE_LG), bottom: (SPACE_LG)}
                show_bg: true
                draw_bg +: {
                    color: (RBX_BG_SURFACE)
                    border_radius: (RBX_RADIUS_MD)
                    border_size: 1.0
                    border_color: (RBX_STROKE_SOFT)
                }

                community_title := mod.widgets.HomeSectionTitle {}

                community_body := MessageHtml {
                    width: Fill, height: Fit
                    padding: 0
                    font_size: 11.
                    font_color: (RBX_FG_SECONDARY)
                    text_style_normal: theme.font_regular { font_size: 11.0 }
                    body: ""
                }
            }
        }
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct WelcomeScreen {
    #[deref] view: View,
    #[rust] app_language: AppLanguage,
    #[rust] app_language_initialized: bool,
    /// The current user's display name, once known, used in the greeting.
    #[rust] greeting_name: Option<String>,
    /// The most recently displayed snapshot of recent rooms.
    #[rust] recent_rooms: Vec<RecentRoomInfo>,
    /// Whether the RoomsList had finished loading all rooms as of the last refresh.
    #[rust] all_rooms_loaded: bool,
    /// Whether we have populated the recent-rooms section at least once.
    #[rust] populated_once: bool,
}

/// The `LiveId`s of the recent-room row views declared in the DSL above.
fn recent_room_row_ids() -> [LiveId; MAX_RECENT_ROOMS] {
    [
        live_id!(recent_room_row_0),
        live_id!(recent_room_row_1),
        live_id!(recent_room_row_2),
        live_id!(recent_room_row_3),
        live_id!(recent_room_row_4),
    ]
}

/// The `LiveId`s of the dividers shown below each recent-room row (except the last).
fn recent_divider_ids() -> [LiveId; MAX_RECENT_ROOMS - 1] {
    [
        live_id!(recent_divider_0),
        live_id!(recent_divider_1),
        live_id!(recent_divider_2),
        live_id!(recent_divider_3),
    ]
}

impl Widget for WelcomeScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if !self.app_language_initialized || self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }

        // Handle clicks on recent-room rows first (before children), so that
        // no child widget (e.g., the avatar) can hijack the tap — the same
        // approach used by RoomsListEntry.
        for (i, row_id) in recent_room_row_ids().into_iter().enumerate() {
            let Some(info) = self.recent_rooms.get(i) else { break };
            let row = self.view.view(cx, &[row_id]);
            if let Hit::FingerUp(fe) = event.hits(cx, row.area()) {
                if fe.is_over && fe.is_primary_hit() && fe.was_tap() {
                    cx.action(AppStateAction::NavigateToRoom {
                        room_to_close: None,
                        destination_room: BasicRoomDetails::Name(info.room_name_id.clone()),
                    });
                }
            }
        }

        // Background updates (rooms list, user profile) arrive via UI signals.
        if matches!(event, Event::Signal) || !self.populated_once {
            self.refresh_dynamic_content(cx);
        }

        if let Event::Actions(actions) = event {
            // Note: each click opens a modal or flips to another page, which
            // swallows the FingerHoverOut event; reset the hover state so the
            // button isn't stuck on its hover color when we return to Home.
            let qa_create_room = self.view.button(cx, ids!(qa_create_room));
            if qa_create_room.clicked(actions) {
                qa_create_room.reset_hover(cx);
                cx.action(CreateRoomModalAction::Open { parent_space_id: None });
            }
            let qa_start_chat = self.view.button(cx, ids!(qa_start_chat));
            if qa_start_chat.clicked(actions) {
                qa_start_chat.reset_hover(cx);
                cx.action(StartChatModalAction::Open);
            }
            let qa_join_room = self.view.button(cx, ids!(qa_join_room));
            if qa_join_room.clicked(actions) {
                qa_join_room.reset_hover(cx);
                cx.action(NavigationBarAction::GoToAddRoom);
            }
            let qa_explore = self.view.button(cx, ids!(qa_explore));
            if qa_explore.clicked(actions) {
                qa_explore.reset_hover(cx);
                cx.action(NavigationBarAction::GoToDirectory);
            }

            for action in actions {
                // On logout or account switch, clear all per-account content.
                let should_clear = matches!(
                    action.downcast_ref(),
                    Some(LogoutAction::ClearAppState { .. })
                ) || matches!(
                    action.downcast_ref(),
                    Some(AccountSwitchAction::Starting(_))
                );
                if should_clear {
                    self.greeting_name = None;
                    self.recent_rooms.clear();
                    self.all_rooms_loaded = false;
                    self.populated_once = false;
                    self.update_greeting(cx);
                    self.populate_recent_rooms(cx);
                    self.view.redraw(cx);
                }
            }
        }

        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if !self.app_language_initialized || self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

impl WelcomeScreen {
    fn set_app_language(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        self.app_language = app_language;
        self.app_language_initialized = true;

        self.update_greeting(cx);
        self.view.label(cx, ids!(subtitle))
            .set_text(cx, tr_key(self.app_language, "home.subtitle"));
        self.view.label(cx, ids!(quick_actions_title))
            .set_text(cx, tr_key(self.app_language, "home.section.quick_actions"));
        self.view.button(cx, ids!(qa_create_room))
            .set_text(cx, tr_key(self.app_language, "home.action.create_room"));
        self.view.button(cx, ids!(qa_start_chat))
            .set_text(cx, tr_key(self.app_language, "home.action.start_chat"));
        self.view.button(cx, ids!(qa_join_room))
            .set_text(cx, tr_key(self.app_language, "home.action.join_room"));
        self.view.button(cx, ids!(qa_explore))
            .set_text(cx, tr_key(self.app_language, "home.action.explore_rooms"));
        self.view.label(cx, ids!(recent_title))
            .set_text(cx, tr_key(self.app_language, "home.section.recent"));
        self.view.label(cx, ids!(community_title))
            .set_text(cx, tr_key(self.app_language, "home.section.community"));
        self.view.html(cx, ids!(community_body))
            .set_text(cx, tr_key(self.app_language, "welcome_screen.body_html"));

        // Re-render the language-dependent status text in the recent section.
        self.populate_recent_rooms(cx);
        self.view.redraw(cx);
    }

    /// Sets the greeting title, personalized with the user's display name if known.
    fn update_greeting(&mut self, cx: &mut Cx) {
        let text = match self.greeting_name.as_deref() {
            Some(name) => tr_fmt(self.app_language, "home.greeting", &[("name", name)]),
            None => tr_key(self.app_language, "welcome_screen.title").to_string(),
        };
        self.view.label(cx, ids!(greeting)).set_text(cx, &text);
    }

    /// Re-fetches the greeting name and the recent-rooms snapshot,
    /// and re-populates the UI if anything has changed.
    fn refresh_dynamic_content(&mut self, cx: &mut Cx) {
        if self.greeting_name.is_none() {
            if let Some(profile) = get_own_profile(cx) {
                self.greeting_name = Some(profile.displayable_name().to_string());
                self.update_greeting(cx);
                self.view.redraw(cx);
            }
        }

        if !cx.has_global::<RoomsListRef>() {
            return;
        }
        let (recents, all_loaded) = {
            let rooms_list = cx.get_global::<RoomsListRef>();
            (rooms_list.get_recent_rooms(MAX_RECENT_ROOMS), rooms_list.all_rooms_loaded())
        };
        if self.populated_once
            && recents == self.recent_rooms
            && all_loaded == self.all_rooms_loaded
        {
            return;
        }
        self.recent_rooms = recents;
        self.all_rooms_loaded = all_loaded;
        self.populated_once = true;
        self.populate_recent_rooms(cx);
        self.view.redraw(cx);
    }

    /// Populates the recent-room rows (and the loading/empty status label)
    /// from the current `self.recent_rooms` snapshot.
    fn populate_recent_rooms(&mut self, cx: &mut Cx) {
        // Show the status label while loading or when there are no rooms at all.
        let show_status = self.recent_rooms.is_empty();
        self.view.view(cx, ids!(recent_status_view)).set_visible(cx, show_status);
        if show_status {
            let status_key = if self.all_rooms_loaded {
                "home.recent.empty"
            } else {
                "home.recent.loading"
            };
            self.view.label(cx, ids!(recent_status_label))
                .set_text(cx, tr_key(self.app_language, status_key));
        }

        for (i, row_id) in recent_room_row_ids().into_iter().enumerate() {
            let row = self.view.view(cx, &[row_id]);
            let Some(info) = self.recent_rooms.get(i) else {
                row.set_visible(cx, false);
                continue;
            };
            row.set_visible(cx, true);

            self.view.label(cx, &[row_id, live_id!(room_name)])
                .set_text(cx, &info.room_name_id.to_string());

            let timestamp_text = info.latest.as_ref()
                .and_then(|(ts, _)| relative_format(*ts))
                .unwrap_or_default();
            self.view.label(cx, &[row_id, live_id!(timestamp)])
                .set_text(cx, &timestamp_text);

            let preview_text = info.latest.as_ref()
                .map(|(_, latest_html)| utils::html_preview_text(latest_html))
                .unwrap_or_default();
            self.view.label(cx, &[row_id, live_id!(preview)])
                .set_text(cx, &preview_text);

            self.view.unread_badge(cx, &[row_id, live_id!(unread_badge)]).update_counts(
                info.is_marked_unread,
                info.num_unread_mentions,
                info.num_unread_messages,
            );

            match &info.room_avatar {
                FetchedRoomAvatar::Text(text) => {
                    self.view.avatar(cx, &[row_id, live_id!(avatar)])
                        .show_text(cx, None, None, text);
                }
                FetchedRoomAvatar::Image(img_bytes) => {
                    let _ = self.view.avatar(cx, &[row_id, live_id!(avatar)]).show_image(
                        cx,
                        None, // Avatars on the home screen shouldn't be clickable.
                        |cx, img| utils::load_png_or_jpg(&img, cx, img_bytes),
                    );
                }
            }
        }

        // A divider is shown below each row that is followed by another row.
        for (i, divider_id) in recent_divider_ids().into_iter().enumerate() {
            self.view.view(cx, &[divider_id])
                .set_visible(cx, i + 1 < self.recent_rooms.len());
        }
    }
}
