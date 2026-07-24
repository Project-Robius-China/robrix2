//! The public room directory browser screen.
//!
//! Reached from the compass button in the RoomsListHeader. Lets the user search
//! their homeserver's public room directory and join rooms directly from the list.
//!
//! ## Visual language
//! This screen follows the `RBX_*` design system (see `docs/ui-visual-spec-zh.md`
//! and `src/shared/design_tokens.rs`): a light `RBX_BG_CANVAS` page, a header with
//! a back affordance + live result count, a shared inset search field (search icon
//! + clear button, matching `RoomFilterInputBar`), and a virtualized list of white
//! `RBX_BG_SURFACE` room cards. Each card reuses the shared `SettingsPrimaryButton`
//! (teal accent CTA) for Join and `SettingsStatusBadge` (success) for the joined
//! state, so the directory reads as one system with Settings / Devices.

use std::collections::HashSet;

use makepad_widgets::*;
use matrix_sdk::ruma::OwnedRoomId;

use crate::{
    avatar_cache::{self, AvatarCacheEntry},
    home::{invite_screen::JoinRoomResultAction, navigation_tab_bar::NavigationBarAction},
    shared::avatar::AvatarWidgetExt,
    sliding_sync::{
        DirectoryRoomKind, MatrixRequest, PublicDirectoryAction, PublicRoomDirectoryEntry,
        submit_async_request,
    },
    utils,
};

const PAGE_LIMIT: u64 = 20;

/// Delay (seconds) after the user stops typing before firing a search request.
const SEARCH_DEBOUNCE_SECS: f64 = 0.3;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*


    // A bare (no-fill) icon button used for the header back affordance and the
    // search field's clear button: transparent by default, a soft hover/press
    // wash, no border. Matches the Settings screen's close button.
    mod.widgets.DirectoryGhostIconButton = mod.widgets.RobrixNeutralIconButton {
        width: Fit, height: Fit
        spacing: 0
        align: Align{x: 0.5, y: 0.5}
        draw_bg +: {
            color: #0000
            color_hover: (RBX_BG_HOVER)
            color_down: (RBX_BG_PRESSED)
            border_size: 0.0
            border_color: #0000
            border_color_hover: #0000
            border_color_down: #0000
            border_radius: (RBX_RADIUS_XS)
        }
        draw_icon +: { color: (RBX_FG_SECONDARY) }
    }


    // One room in the directory list — a white card (transparent custom-widget
    // root + inner `entry_body` RoundedView surface, per the "plain custom root
    // doesn't paint" pitfall) with avatar, name/topic/meta column, and a Join CTA.
    mod.widgets.DirectoryRoomEntry = #(DirectoryRoomEntry::register_widget(vm)) {
        ..mod.widgets.View
        width: Fill, height: Fit
        flow: Down
        margin: Inset{bottom: (SPACE_SM)}

        entry_body := RoundedView {
            width: Fill, height: Fit
            flow: Right
            align: Align{y: 0.5}
            padding: Inset{top: (SPACE_MD), bottom: (SPACE_MD), left: (SPACE_MD), right: (SPACE_MD)}
            spacing: (SPACE_MD)
            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_SM)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
            }

            avatar := Avatar {
                width: (RBX_AVATAR_LG), height: (RBX_AVATAR_LG)
            }

            text_column := View {
                width: Fill, height: Fit
                flow: Down
                spacing: 3

                name_label := Label {
                    width: Fill, height: Fit
                    flow: Flow.Right{wrap: false}
                    max_lines: 1
                    text_overflow: Ellipsis
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: RBX_TEXT_CARD_TITLE {}
                    }
                    text: ""
                }

                topic_label := Label {
                    width: Fill, height: Fit
                    max_lines: 2
                    text_overflow: Ellipsis
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_META {}
                    }
                    text: ""
                }

                meta_row := View {
                    width: Fill, height: Fit
                    flow: Right
                    align: Align{y: 0.5}
                    spacing: 5

                    meta_icon := Icon {
                        draw_icon +: {
                            svg: (ICON_PEOPLE)
                            color: (RBX_FG_TERTIARY)
                        }
                        icon_walk: Walk{width: 12, height: 12}
                    }

                    meta_label := Label {
                        width: Fill, height: Fit
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

            // Trailing action: teal accent CTA (Join) OR success badge (Joined).
            // Only one is visible at a time; the hidden one collapses out of the
            // row layout. Both are built on core widgets (no derived-template
            // child override) so they render reliably inside the PortalList item.
            join_button := SettingsPrimaryButton {
                width: Fit, height: (RBX_CONTROL_H_SM)
                padding: Inset{top: 7, bottom: 7, left: 12, right: 14}
                spacing: 6
                draw_icon.svg: (ICON_JOIN_ROOM)
                icon_walk: Walk{width: 14, height: 14}
                draw_text +: { text_style: RBX_TEXT_BODY_STRONG {} }
                text: "Join"
            }

            joined_badge := RoundedView {
                visible: false
                width: Fit, height: Fit
                align: Align{x: 0.5, y: 0.5}
                padding: Inset{left: 12, right: 12, top: 6, bottom: 6}
                show_bg: true
                draw_bg +: {
                    color: (RBX_SUCCESS_BG)
                    border_radius: (RBX_RADIUS_PILL)
                }
                Label {
                    width: Fit, height: Fit
                    draw_text +: {
                        color: (RBX_SUCCESS_FG)
                        text_style: RBX_TEXT_BADGE {}
                    }
                    text: "Joined"
                }
            }
        }
    }


    mod.widgets.DirectoryScreen = #(DirectoryScreen::register_widget(vm)) {
        ..mod.widgets.View
        width: Fill, height: Fill
        flow: Down
        padding: Inset{top: (SPACE_MD), left: (SPACE_LG), right: (SPACE_LG), bottom: 0}
        spacing: (SPACE_SM)
        show_bg: true
        draw_bg +: { color: (RBX_BG_CANVAS) }

        // ── Header: back affordance + title + live result count ──────────────
        header := View {
            width: Fill, height: Fit
            flow: Right
            align: Align{y: 0.5}
            spacing: (SPACE_SM)

            back_button := DirectoryGhostIconButton {
                padding: (SPACE_SM)
                draw_icon.svg: (ICON_ARROW_BACK)
                icon_walk: Walk{width: 18, height: 18}
            }

            header_col := View {
                width: Fill, height: Fit
                flow: Down
                spacing: 2

                title := Label {
                    width: Fill, height: Fit
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: RBX_TEXT_PAGE_TITLE {}
                    }
                    text: "Public rooms"
                }

                subtitle := Label {
                    width: Fill, height: Fit
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_META {}
                    }
                    text: "Discover and join public rooms"
                }
            }
        }

        LineH {
            height: 1.0
            margin: Inset{top: 2, bottom: 2}
            draw_bg.color: (RBX_STROKE_SOFT)
        }

        // ── Search field: shared inset look (search icon + clear button) ─────
        search_field := RoundedView {
            width: Fill, height: 40
            flow: Right
            align: Align{y: 0.5}
            spacing: 4
            padding: Inset{left: 12, right: 5}
            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE_SUBTLE)
                border_radius: (RBX_RADIUS_XS)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
            }

            Icon {
                draw_icon +: {
                    svg: (ICON_SEARCH)
                    color: (RBX_FG_TERTIARY)
                }
                icon_walk: Walk{width: 16, height: 16}
            }

            search_input := RobrixTextInput {
                width: Fill, height: Fit
                flow: Right
                padding: Inset{left: 4, right: 4, top: 0, bottom: 0}
                empty_text: "Search public rooms..."
                // Make the field read as one inset surface (no white patch, no
                // second border ring over the container's ring).
                draw_bg +: {
                    border_size: 0.0
                    color: (RBX_BG_SURFACE_SUBTLE)
                    color_hover: (RBX_BG_SURFACE_SUBTLE)
                    color_focus: (RBX_BG_SURFACE_SUBTLE)
                    color_down: (RBX_BG_SURFACE_SUBTLE)
                    color_empty: (RBX_BG_SURFACE_SUBTLE)
                }
                draw_text +: {
                    color: (RBX_FG_PRIMARY)
                    text_style: RBX_TEXT_BODY {}
                }
            }

            clear_button := DirectoryGhostIconButton {
                visible: false
                padding: Inset{top: 7, bottom: 7, left: 7, right: 7}
                draw_icon.svg: (ICON_CLOSE)
                icon_walk: Walk{width: 12, height: 12}
            }
        }

        // ── Inline error card (danger-tinted) ───────────────────────────────
        error_card := RoundedView {
            visible: false
            width: Fill, height: Fit
            flow: Right
            align: Align{y: 0.5}
            spacing: (SPACE_SM)
            padding: Inset{left: (SPACE_MD), right: (SPACE_MD), top: (SPACE_SM), bottom: (SPACE_SM)}
            margin: Inset{top: 2}
            show_bg: true
            draw_bg +: {
                color: (RBX_DANGER_BG)
                border_radius: (RBX_RADIUS_XS)
                border_size: 1.0
                border_color: (RBX_DANGER_FG)
            }

            Icon {
                draw_icon +: {
                    svg: (ICON_WARNING)
                    color: (RBX_DANGER_FG)
                }
                icon_walk: Walk{width: 16, height: 16}
            }

            error_label := Label {
                width: Fill, height: Fit
                flow: Flow.Right{wrap: true}
                draw_text +: {
                    color: (RBX_DANGER_FG)
                    text_style: RBX_TEXT_BODY {}
                }
                text: ""
            }
        }

        // ── Result list (virtualized) ───────────────────────────────────────
        room_list := PortalList {
            width: Fill, height: Fill
            keep_invisible: false,
            max_pull_down: 0.0,
            auto_tail: false,
            flow: Down
            margin: Inset{top: (SPACE_XS)}

            room_entry := mod.widgets.DirectoryRoomEntry {}

            loading_entry := View {
                width: Fill, height: 60
                flow: Right
                align: Align{x: 0.5, y: 0.5}
                LoadingSpinner {
                    width: 24, height: 24
                    draw_bg +: {
                        color: (RBX_ACCENT)
                        border_size: 3.0
                    }
                }
            }

            status_entry := View {
                width: Fill, height: Fit
                padding: Inset{top: 40, bottom: 20, left: 16, right: 16}
                flow: Down
                align: Align{x: 0.5, y: 0.5}
                spacing: (SPACE_SM)

                status_icon := Icon {
                    draw_icon +: {
                        svg: (ICON_GLOBE)
                        color: (RBX_FG_TERTIARY)
                    }
                    icon_walk: Walk{width: 30, height: 30}
                }

                status_label := Label {
                    width: Fit, height: Fit
                    align: Align{x: 0.5}
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_BODY {}
                    }
                    text: ""
                }
            }
        }
    }
}


#[derive(Script, ScriptHook, Widget)]
pub struct DirectoryRoomEntry {
    #[deref] view: View,
    #[rust] room_id: Option<OwnedRoomId>,
    #[rust(false)] is_joining: bool,
    #[rust(false)] is_joined: bool,
}

#[derive(Clone, Default, Debug)]
pub enum DirectoryEntryAction {
    JoinClicked(OwnedRoomId),
    #[default]
    None,
}

impl Widget for DirectoryRoomEntry {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let uid = self.widget_uid();
        if let Event::Actions(actions) = event {
            let join_button = self.view.button(cx, ids!(join_button));
            if join_button.clicked(actions) {
                if let Some(rid) = &self.room_id {
                    log!("[public_directory] DirectoryRoomEntry::join clicked: room_id={rid}");
                    if !self.is_joining && !self.is_joined {
                        cx.widget_action(uid, DirectoryEntryAction::JoinClicked(rid.clone()));
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

impl DirectoryRoomEntry {
    pub fn populate(
        &mut self,
        cx: &mut Cx,
        entry: &PublicRoomDirectoryEntry,
        is_joining: bool,
        is_joined: bool,
    ) {
        self.room_id = Some(entry.room_id.clone());
        self.is_joining = is_joining;
        self.is_joined = is_joined;

        self.view
            .label(cx, ids!(text_column.name_label))
            .set_text(cx, &entry.display_name);

        // Topic: collapse to one clean paragraph; hide the row entirely if empty
        // so cards without a topic don't leave a dead gap.
        let topic_text = entry
            .topic
            .as_deref()
            .map(collapse_to_single_line)
            .unwrap_or_default();
        let topic_label = self.view.label(cx, ids!(text_column.topic_label));
        topic_label.set_visible(cx, !topic_text.is_empty());
        topic_label.set_text(cx, &topic_text);

        // Meta line: "N members" (with a people icon), plus the canonical alias
        // when present, so the room is addressable at a glance.
        let mut meta = format!("{} members", entry.num_joined_members);
        if let Some(alias) = &entry.canonical_alias {
            meta.push_str("  ·  ");
            meta.push_str(alias);
        }
        self.view
            .label(cx, ids!(text_column.meta_label))
            .set_text(cx, &meta);

        let avatar = self.view.avatar(cx, ids!(avatar));
        let mut drew_image = false;
        if let Some(uri) = &entry.avatar_uri {
            if let AvatarCacheEntry::Loaded(data) = avatar_cache::get_or_fetch_avatar(cx, uri) {
                let res = avatar.show_image(cx, None, |cx, img| {
                    utils::load_png_or_jpg(&img, cx, &data)
                });
                drew_image = res.is_ok();
            }
        }
        if !drew_image {
            avatar.show_text(cx, None, None, entry.display_name.as_str());
        }

        // Trailing action state machine: Join (CTA) → Joining… (disabled) →
        // Joined (success badge, button hidden).
        let join_button = self.view.button(cx, ids!(join_button));
        let joined_badge = self.view.view(cx, ids!(joined_badge));
        if is_joined {
            join_button.set_visible(cx, false);
            joined_badge.set_visible(cx, true);
        } else {
            joined_badge.set_visible(cx, false);
            join_button.set_visible(cx, true);
            if is_joining {
                join_button.set_text(cx, "Joining…");
                join_button.set_enabled(cx, false);
            } else {
                join_button.set_text(cx, "Join");
                join_button.set_enabled(cx, true);
            }
        }
    }
}


#[derive(Script, ScriptHook, Widget)]
pub struct DirectoryScreen {
    #[deref] view: View,
    #[rust(0u64)] query_id: u64,
    #[rust] search_text: String,
    #[rust] rooms: Vec<PublicRoomDirectoryEntry>,
    #[rust] next_batch: Option<String>,
    #[rust(false)] is_loading: bool,
    #[rust(true)] needs_initial_fetch: bool,
    #[rust] last_error: Option<String>,
    #[rust] pending_joins: HashSet<OwnedRoomId>,
    #[rust] joined_rooms: HashSet<OwnedRoomId>,
    #[rust(Timer::empty())] search_debounce_timer: Timer,
    #[rust] pending_search_text: String,
    /// Last subtitle string pushed to the header, so we only call the
    /// (unconditionally-redrawing) `Label::set_text` when it actually changes.
    #[rust] last_subtitle: String,
}

impl Widget for DirectoryScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        // Debounce timer fired: run the pending search if the text actually
        // differs from what we last queried.
        if let Event::Timer(te) = event {
            if self.search_debounce_timer.is_timer(te).is_some() {
                self.search_debounce_timer = Timer::empty();
                let pending = std::mem::take(&mut self.pending_search_text);
                log!(
                    "[public_directory] debounce fired: pending={pending:?} current={:?}",
                    self.search_text,
                );
                if pending != self.search_text {
                    self.start_fresh_query(cx, pending);
                }
            }
        }

        if let Event::Actions(actions) = event {
            // Back button: return to the Home view.
            if self.view.button(cx, ids!(back_button)).clicked(actions) {
                cx.action(NavigationBarAction::GoToHome);
            }

            // Clear button: wipe the field and re-run the default (empty) query.
            if self.view.button(cx, ids!(search_field.clear_button)).clicked(actions) {
                self.view
                    .text_input(cx, ids!(search_input))
                    .set_text(cx, "");
                cx.stop_timer(self.search_debounce_timer);
                self.search_debounce_timer = Timer::empty();
                self.pending_search_text.clear();
                self.update_clear_button(cx, "");
                self.start_fresh_query(cx, String::new());
            }

            let search_input = self.view.text_input(cx, ids!(search_input));
            if let Some(text) = search_input.changed(actions) {
                log!("[public_directory] search input changed: text={text:?}");
                self.update_clear_button(cx, &text);
                cx.stop_timer(self.search_debounce_timer);
                self.pending_search_text = text;
                self.search_debounce_timer = cx.start_timeout(SEARCH_DEBOUNCE_SECS);
            }
            if let Some((text, _)) = search_input.returned(actions) {
                log!("[public_directory] search submitted (Enter): text={text:?}");
                cx.stop_timer(self.search_debounce_timer);
                self.search_debounce_timer = Timer::empty();
                self.pending_search_text.clear();
                self.update_clear_button(cx, &text);
                self.start_fresh_query(cx, text);
            }

            for action in actions {
                if let Some(dir_action) = action.downcast_ref::<PublicDirectoryAction>() {
                    self.handle_public_directory_action(cx, dir_action);
                    continue;
                }
                if let Some(jra) = action.downcast_ref::<JoinRoomResultAction>() {
                    self.handle_join_result(cx, jra);
                    continue;
                }
                if let DirectoryEntryAction::JoinClicked(rid) =
                    action.as_widget_action().cast()
                {
                    log!("[public_directory] JoinClicked received: room_id={rid}");
                    if !self.pending_joins.contains(&rid) && !self.joined_rooms.contains(&rid) {
                        self.pending_joins.insert(rid.clone());
                        submit_async_request(MatrixRequest::JoinRoom { room_id: rid });
                        self.view.redraw(cx);
                    }
                    continue;
                }
            }
        }

        // Detect bottom-reach for pagination.
        if self.next_batch.is_some() && !self.is_loading {
            let list = self.view.portal_list(cx, ids!(room_list));
            if list.is_at_end() {
                self.submit_fetch(false);
            }
        }

        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if self.needs_initial_fetch {
            self.needs_initial_fetch = false;
            self.start_fresh_query(cx, String::new());
        }

        // Keep the error card's *visibility* in sync every frame — `set_visible`
        // is change-guarded, so this is cheap. Its text and the header subtitle
        // are only mutated at state-change points (see `refresh_status`) because
        // `set_text` always reschedules a redraw and must not run per-frame.
        self.view
            .view(cx, ids!(error_card))
            .set_visible(cx, self.last_error.is_some());

        while let Some(widget_to_draw) = self.view.draw_walk(cx, scope, walk).step() {
            let plist = widget_to_draw.as_portal_list();
            let Some(mut list) = plist.borrow_mut() else { continue };

            let n = self.rooms.len();
            let has_more = self.next_batch.is_some() || self.is_loading;
            let show_empty_status = n == 0 && !self.is_loading;
            let total = if show_empty_status {
                1
            } else {
                n + if has_more { 1 } else { 0 }
            };
            list.set_item_range(cx, 0, total);

            while let Some(item_id) = list.next_visible_item(cx) {
                let item = if show_empty_status && item_id == 0 {
                    let item = list.item(cx, item_id, id!(status_entry));
                    let msg = if self.search_text.trim().is_empty() {
                        "No public rooms found on this homeserver."
                    } else {
                        "No rooms match your search."
                    };
                    item.child_by_path(ids!(status_label))
                        .as_label()
                        .set_text(cx, msg);
                    item
                } else if item_id < n {
                    let entry = self.rooms[item_id].clone();
                    let is_joining = self.pending_joins.contains(&entry.room_id);
                    let is_joined = self.joined_rooms.contains(&entry.room_id);
                    let item = list.item(cx, item_id, id!(room_entry));
                    if let Some(mut inner) = item.borrow_mut::<DirectoryRoomEntry>() {
                        inner.populate(cx, &entry, is_joining, is_joined);
                    }
                    item
                } else if has_more && item_id == n {
                    list.item(cx, item_id, id!(loading_entry))
                } else {
                    continue;
                };
                item.draw_all(cx, scope);
            }
        }

        DrawStep::done()
    }
}

impl DirectoryScreen {
    /// Refresh the header subtitle and error-card text to reflect the current
    /// load / result / error state. Called only at state-change points (never
    /// per-frame) because `Label::set_text` reschedules a redraw every call.
    fn refresh_status(&mut self, cx: &mut Cx) {
        let n = self.rooms.len();
        let has_more = self.next_batch.is_some();
        let subtitle = if self.is_loading && n == 0 {
            "Searching…".to_string()
        } else if self.search_text.trim().is_empty() {
            match (n, has_more) {
                (0, _) => "Discover and join public rooms".to_string(),
                (1, false) => "1 public room".to_string(),
                (_, false) => format!("{n} public rooms"),
                (_, true) => format!("{n}+ public rooms"),
            }
        } else {
            match (n, has_more) {
                (0, _) => "No matching rooms".to_string(),
                (1, false) => "1 result".to_string(),
                (_, false) => format!("{n} results"),
                (_, true) => format!("{n}+ results"),
            }
        };
        if subtitle != self.last_subtitle {
            self.view
                .label(cx, ids!(header.header_col.subtitle))
                .set_text(cx, &subtitle);
            self.last_subtitle = subtitle;
        }

        if let Some(err) = self.last_error.clone() {
            self.view
                .label(cx, ids!(error_card.error_label))
                .set_text(cx, &err);
        }
    }

    /// Show the clear (✕) affordance only when the field has text.
    fn update_clear_button(&mut self, cx: &mut Cx, text: &str) {
        self.view
            .button(cx, ids!(search_field.clear_button))
            .set_visible(cx, !text.is_empty());
    }

    fn start_fresh_query(&mut self, cx: &mut Cx, text: String) {
        self.search_text = text;
        self.query_id = self.query_id.wrapping_add(1);
        self.rooms.clear();
        self.next_batch = None;
        self.last_error = None;
        self.is_loading = false;
        log!(
            "[public_directory] start_fresh_query: query_id={} search_text={:?}",
            self.query_id,
            self.search_text,
        );
        self.submit_fetch(true);
        self.refresh_status(cx);
        self.view.redraw(cx);
    }

    fn submit_fetch(&mut self, is_first_page: bool) {
        if self.is_loading {
            log!(
                "[public_directory] submit_fetch skipped (already loading): \
                 query_id={} is_first_page={is_first_page}",
                self.query_id,
            );
            return;
        }
        self.is_loading = true;
        log!(
            "[public_directory] search_id={} submit_fetch: query_id={} is_first_page={is_first_page} \
             since={:?} limit={}",
            self.search_text,
            self.query_id,
            self.next_batch,
            PAGE_LIMIT,
        );
        submit_async_request(MatrixRequest::FetchPublicDirectoryPage {
            search_term: self.search_text.clone(),
            kind: DirectoryRoomKind::Rooms,
            since: if is_first_page { None } else { self.next_batch.clone() },
            limit: Some(PAGE_LIMIT),
            query_id: self.query_id,
        });
    }

    fn handle_public_directory_action(&mut self, cx: &mut Cx, action: &PublicDirectoryAction) {
        match action {
            PublicDirectoryAction::Page {
                query_id,
                is_first_page,
                rooms,
                next_batch,
            } => {
                if *query_id != self.query_id {
                    return;
                }
                self.is_loading = false;
                if *is_first_page {
                    self.rooms.clear();
                }
                self.rooms.extend(rooms.iter().cloned());
                self.next_batch = next_batch.clone();
                self.last_error = None;
                self.refresh_status(cx);
                self.view.redraw(cx);
            }
            PublicDirectoryAction::Failed {
                query_id,
                is_first_page: _,
                error,
            } => {
                if *query_id != self.query_id {
                    return;
                }
                self.is_loading = false;
                self.last_error = Some(error.clone());
                self.refresh_status(cx);
                self.view.redraw(cx);
            }
        }
    }

    fn handle_join_result(&mut self, cx: &mut Cx, action: &JoinRoomResultAction) {
        match action {
            JoinRoomResultAction::Joined { room_id } => {
                if self.pending_joins.remove(room_id) {
                    self.joined_rooms.insert(room_id.clone());
                    self.view.redraw(cx);
                }
            }
            JoinRoomResultAction::Failed { room_id, error: _ } => {
                if self.pending_joins.remove(room_id) {
                    self.view.redraw(cx);
                }
            }
        }
    }
}


fn collapse_to_single_line(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_ws = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_ws {
                out.push(' ');
                prev_ws = true;
            }
        } else {
            out.push(c);
            prev_ws = false;
        }
    }
    out.trim().to_string()
}
