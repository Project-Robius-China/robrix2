use makepad_widgets::*;
use matrix_sdk::ruma::{OwnedRoomId, RoomId, UserId};

use crate::{
    app::AppState,
    i18n::{AppLanguage, tr_fmt, tr_key},
    room::FetchedRoomAvatar, shared::{
        avatar::AvatarWidgetExt,
        html_or_plaintext::HtmlOrPlaintextWidgetExt, unread_badge::UnreadBadgeWidgetExt as _,
    }, utils::{self, relative_format}
};

use super::{ContextMenuOpenGesture, rooms_list::{InvitedRoomInfo, InviterInfo, JoinedRoomInfo, RoomsListScopeProps}};
script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*


    // A cancel icon to be displayed in the RoomsListEntry when the room is tombstoned.
    mod.widgets.TombstoneIcon = View {
        width: Fit, height: Fit,
        visible: false,

        Icon {
            width: 19, height: 19,
            align: Align{x: 0.5, y: 0.5}
            draw_icon +: {
                svg: (ICON_TOMBSTONE)
                color: (COLOR_FG_DANGER_RED)
            }
            icon_walk: Walk{ width: 15, height: 15 }
        }
    }

    mod.widgets.EncryptionIcon = View {
        width: Fit, height: Fit,
        visible: false,

        Icon {
            width: 19, height: 19,
            align: Align{x: 0.5, y: 0.5}
            draw_icon +: {
                svg: (ICON_LOCK_FILLED)
                color: (RBX_FG_SECONDARY)
            }
            icon_walk: Walk{ width: 15, height: 15 }
        }
    }

    mod.widgets.RoomName = Label {
        width: Fill, height: Fit
        flow: Flow.Right{wrap: false},
        padding: 0,
        max_lines: 1
        text_overflow: Ellipsis
        draw_text +: {
            color: (RBX_FG_PRIMARY),
            text_style: RBX_TEXT_BODY_STRONG {}
        }
        text: "[Room name unknown]"
    }

    // A small blue "bot" pill shown after the room name for agent-bound rooms.
    // Reproduces the timeline's bot badge look (room_screen.rs) locally so this
    // file doesn't depend on room_screen's private constants/widgets.
    mod.widgets.RoomsListBotPill = RoundedView {
        visible: false
        width: Fit
        height: 16.0
        align: Align{x: 0.5, y: 0.5}
        padding: Inset{left: 6.0, right: 6.0}
        show_bg: true
        new_batch: true
        draw_bg +: {
            color: (COLOR_ACTIVE_PRIMARY)
            border_radius: 3.0
        }
        Label {
            width: Fit, height: Fit, padding: 0
            draw_text +: {
                text_style: REGULAR_TEXT { font_size: 8.5, top_drop: -0.08 }
                color: (RBX_FG_ON_ACCENT)
            }
            text: "bot"
        }
    }

    mod.widgets.RoomsListEntryTimestamp = Label {
        padding: Inset{top: 1},
        width: Fit, height: Fit
        flow: Flow.Right{wrap: false},
        draw_text +: {
            color: (RBX_FG_TERTIARY)
            text_style: RBX_TEXT_META {}
        }
    }

    mod.widgets.MessagePreview = View {
        width: Fill, height: Fit
        latest_message := HtmlOrPlaintext {
                html_view +: {
                    html +: {
                    font_size: 9.3
                    max_lines: 2
                    text_overflow: Ellipsis
                    text_style_normal +: { font_size: 9.3, line_spacing: 1.32 }
                    text_style_italic +: { font_size: 9.3, line_spacing: 1.32 }
                    text_style_bold +: { font_size: 9.3, line_spacing: 1.32 }
                    text_style_bold_italic +: { font_size: 9.3, line_spacing: 1.32 }
                    text_style_fixed +: { font_size: 9.3, line_spacing: 1.32 }
                    // Scale down the pill (title font, avatar size, avatar text) to fit.
                    a +: {
                        matrix_link_view +: {
                            matrix_link +: {
                                pill_bg +: {
                                    margin: Inset{top: 1}
                                    padding: Inset{ left: 4.5, right: 3.0, bottom: -3.5, top: -3.5 }
                                    draw_bg +: { border_radius: 4.5 }
                                    avatar +: {
                                        width: 13.0, height: 13.0,
                                        text_view +: {
                                            text +: {
                                                draw_text +: {
                                                    text_style +: { font_size: 6 }
                                                }
                                            }
                                        }
                                    }
                                    title +: {
                                        draw_text +: {
                                            text_style +: { font_size: 8.5 }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            plaintext_view +: {
                pt_label +: {
                    max_lines: 2
                    text_overflow: Ellipsis
                    draw_text +: {
                        text_style: theme.font_regular { font_size: 9.3, line_spacing: 1.32 },
                    }
                    text: "[No recent messages]"
                }
            }
        }
    }

    mod.widgets.RoomsListEntryContent = set_type_default() do #(RoomsListEntryContent::register_widget(vm)) {

        flow: Right,
        spacing: 10,
        padding: 10,
        width: Fill, height: Fit

        show_bg: true
        draw_bg +: {
            active: instance(0.0)
            color: instance(#0000)
            color_selected: instance(RBX_BG_SELECTED)
            border_color: instance(#0000)
            // Teal accent outline that fades in on the selected/open room, so the
            // soft-teal wash is unambiguous even on the light canvas sidebar.
            border_color_selected: instance(RBX_ACCENT)
            border_size: uniform(1.5)
            border_radius: uniform(6.0)
            border_inset: uniform(vec4(0.0))

            get_color: fn() -> vec4 {
                return mix(self.color, self.color_selected, self.active)
            }

            get_border_color: fn() -> vec4 {
                return mix(self.border_color, self.border_color_selected, self.active)
            }

            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                sdf.box(
                    self.border_inset.x + self.border_size,
                    self.border_inset.y + self.border_size,
                    self.rect_size.x - (self.border_inset.x + self.border_inset.z + self.border_size * 2.0),
                    self.rect_size.y - (self.border_inset.y + self.border_inset.w + self.border_size * 2.0),
                    max(1.0, self.border_radius)
                )
                sdf.fill_keep(self.get_color())
                if self.border_size > 0.0 {
                    sdf.stroke(self.get_border_color(), self.border_size)
                }
                return sdf.result;
            }
        }
        animator: Animator{
            selected: {
                default: @off
                off: AnimatorState{
                    from: {all: Snap}
                    apply: {
                        draw_bg: {active: 0.0}
                    }
                }
                on: AnimatorState{
                    from: {all: Snap}
                    apply: {
                        draw_bg: {active: 1.0}
                    }
                }
            }
        }
    }

    mod.widgets.RoomsListEntry = #(RoomsListEntry::register_widget(vm)) {
        flow: Down, height: Fit
        cursor: MouseCursor.Default,

        // Wrap the RoomsListEntryContent in an AdaptiveView to change the displayed content
        // (and its layout) based on the available space in the sidebar.
        adaptive_preview := AdaptiveView {
            height: Fit
            // The wider variants contain `RoomsListBotPill`, a `new_batch`
            // view that owns a child DrawList. Retain variants across resize
            // swaps so ultra-narrow transitions do not drop DrawLists that the
            // previous frame may still reference.
            retain_unused_variants: true

            OnlyIcon := mod.widgets.RoomsListEntryContent {
                align: Align{x: 0.5, y: 0.5}
                padding: 5.
                View {
                    height: Fit
                    flow: Overlay
                    align: Align{ x: 1.0 }
                    avatar := Avatar {}
                    unread_badge := UnreadBadge {}
                    encryption_icon := mod.widgets.EncryptionIcon {}
                    tombstone_icon := mod.widgets.TombstoneIcon {}
                }
            }
            IconAndName := mod.widgets.RoomsListEntryContent {
                padding: 5.
                align: Align{x: 0.5, y: 0.5}
                avatar := Avatar {}
                name_wrap := View {
                    width: Fill, height: Fit
                    flow: Right
                    align: Align{y: 0.5}
                    spacing: 6
                    // Hug the name text so the bot pill packs snug right after it,
                    // instead of Fill's turtle-advance pushing the pill to the row's
                    // right edge. Capped so long names still ellipsize; IconAndName
                    // is only shown at sidebar widths <= 200px, so a smaller cap
                    // than FullPreview leaves room for the pill.
                    room_name := mod.widgets.RoomName { width: Fit{max: FitBound.Rel{base: Base.Full, factor: 0.70}} }
                    bot_pill := mod.widgets.RoomsListBotPill {}
                }
                unread_badge := UnreadBadge {}
                encryption_icon := mod.widgets.EncryptionIcon {}
                tombstone_icon := mod.widgets.TombstoneIcon {}
            }
            FullPreview := mod.widgets.RoomsListEntryContent {
                padding: 10
                avatar := Avatar {}
                View {
                    flow: Down
                    width: Fill, height: 56
                    align: Align{ x: 0.0, y: 0.0 }
                    top := View {
                        width: Fill, height: Fit,
                        spacing: 3,
                        flow: Right,
                        name_wrap := View {
                            width: Fill, height: Fit
                            flow: Right
                            align: Align{y: 0.5}
                            spacing: 6
                            // Same fix as IconAndName above, but with a larger cap since
                            // FullPreview is shown on desktop/wider sidebars.
                            room_name := mod.widgets.RoomName { width: Fit{max: FitBound.Rel{base: Base.Full, factor: 0.78}} }
                            bot_pill := mod.widgets.RoomsListBotPill {}
                        }
                        timestamp := mod.widgets.RoomsListEntryTimestamp { }
                    }
                    bottom := View {
                        width: Fill, height: Fill,
                        spacing: 2,
                        flow: Right,
                        preview := mod.widgets.MessagePreview {
                            margin: Inset{ top: 2.5 }
                        }
                        View {
                            width: Fit, height: Fit
                            align: Align{ x: 1.0 }
                            unread_badge := UnreadBadge {}
                            encryption_icon := mod.widgets.EncryptionIcon {}
                            tombstone_icon := mod.widgets.TombstoneIcon {}
                        }
                    }
                }
            }
        }
    }
}

/// An entry in the rooms list.
#[derive(Script, Widget)]
pub struct RoomsListEntry {
    #[deref] view: View,
    #[rust] room_id: Option<OwnedRoomId>,
}

impl ScriptHook for RoomsListEntry {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        vm.with_cx_mut(|cx| {
            self.set_adaptive_variant_selector(cx);
        })
    }
}

/// Widget actions that are emitted by a RoomsListEntry.
#[derive(Clone, Default, Debug)]
pub enum RoomsListEntryAction {
    /// This RoomsListEntry was primary-clicked or tapped.
    PrimaryClicked(OwnedRoomId),
    /// This RoomsListEntry was right-clicked or long-pressed.
    SecondaryClicked(OwnedRoomId, DVec2, ContextMenuOpenGesture),
    #[default]
    None,
}

impl RoomsListEntry {
    fn set_adaptive_variant_selector(&self, cx: &mut Cx) {
        self.view
            .adaptive_view(cx, ids!(adaptive_preview))
            .set_variant_selector(|cx, parent_size| {
                if cx.display_context.is_desktop() {
                    id!(FullPreview)
                } else {
                    match parent_size.x {
                        width if width <= 70.0 => id!(OnlyIcon),
                        width if width <= 200.0 => id!(IconAndName),
                        _ => id!(FullPreview),
                    }
                }
            });
    }
}

impl Widget for RoomsListEntry {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let uid = self.widget_uid();
        let rooms_list_props = scope.props.get::<RoomsListScopeProps>().unwrap();

        // We handle hits on this widget first to ensure that any clicks on it
        // will just select the room, rather than resulting in a click on any child view
        // within the RoomsListEntry content itself, such as links or avatars.
        if let Some(room_id) = &self.room_id {
            let area = self.view.area();
            match event.hits(cx, area) {
                Hit::FingerDown(fe) => {
                    cx.set_key_focus(area);
                    if fe.device.mouse_button().is_some_and(|b| b.is_secondary()) {
                        cx.widget_action(
                            uid, 
                            RoomsListEntryAction::SecondaryClicked(
                                room_id.clone(),
                                fe.abs,
                                ContextMenuOpenGesture::from_finger_down(&fe),
                            ),
                        );
                    }
                }
                Hit::FingerLongPress(fe) => {
                    cx.widget_action(
                        uid, 
                        RoomsListEntryAction::SecondaryClicked(
                            room_id.clone(),
                            fe.abs,
                            ContextMenuOpenGesture::from_long_press(&fe),
                        ),
                    );
                }
                Hit::FingerUp(fe) if !rooms_list_props.was_scrolling && fe.is_over && fe.is_primary_hit() && fe.was_tap() => {
                    cx.widget_action(uid,  RoomsListEntryAction::PrimaryClicked(room_id.clone()));
                }
                _ => { }
            }
        }

        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if let Some(room_info) = scope.props.get::<JoinedRoomInfo>() {
            self.room_id = Some(room_info.room_name_id.room_id().clone());
        }
        else if let Some(room_info) = scope.props.get::<InvitedRoomInfo>() {
            self.room_id = Some(room_info.room_name_id.room_id().clone());
        }

        self.view.draw_walk(cx, scope, walk)
    }
}

#[derive(Script, ScriptHook, Widget, Animator)]
pub struct RoomsListEntryContent {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    #[apply_default] animator: Animator,
}

impl Widget for RoomsListEntryContent {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.animator_handle_event(cx, event).must_redraw() {
            self.redraw(cx);
        }
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let app_state = scope.data.get::<AppState>();
        let app_language = app_state
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if let Some(joined_room_info) = scope.props.get::<JoinedRoomInfo>() {
            let show_agent_badge = app_state.is_some_and(|app_state| {
                room_shows_agent_badge(
                    app_state,
                    joined_room_info.room_name_id.room_id(),
                    joined_room_info.dm_target.as_deref(),
                )
            });
            self.draw_joined_room(cx, joined_room_info, show_agent_badge);
        } else if let Some(invited_room_info) = scope.props.get::<InvitedRoomInfo>() {
            self.draw_invited_room(cx, invited_room_info, app_language);
        }

        self.view.draw_walk(cx, scope, walk)
    }
}

impl RoomsListEntryContent {
    /// Populates this RoomsListEntry with info about a joined room.
    pub fn draw_joined_room(
        &mut self,
        cx: &mut Cx,
        room_info: &JoinedRoomInfo,
        show_agent_badge: bool,
    ) {
        self.view.label(cx, ids!(room_name)).set_text(cx, &room_info.room_name_id.to_string());
        if let Some((ts, msg)) = room_info.latest.as_ref() {
            if let Some(human_readable_date) = relative_format(*ts) {
                self.view
                    .label(cx, ids!(timestamp))
                    .set_text(cx, &human_readable_date);
            }
            self.view
                .html_or_plaintext(cx, ids!(latest_message))
                .show_html(cx, msg);
        }

        self.view.unread_badge(cx, ids!(unread_badge)).update_counts(
            room_info.is_marked_unread,
            room_info.num_unread_mentions,
            room_info.num_unread_messages,
        );
        self.draw_common(cx, &room_info.room_avatar, room_info.is_selected);
        self.view.view(cx, ids!(encryption_icon)).set_visible(
            cx,
            should_show_encryption_icon(room_info.is_encrypted, room_info.is_tombstoned),
        );
        self.view.view(cx, ids!(bot_pill)).set_visible(cx, show_agent_badge);
        self.view.view(cx, ids!(tombstone_icon)).set_visible(cx, room_info.is_tombstoned);
    }

    /// Populates this RoomsListEntry with info about an invited room.
    pub fn draw_invited_room(
        &mut self,
        cx: &mut Cx,
        room_info: &InvitedRoomInfo,
        app_language: AppLanguage,
    ) {
        self.view.label(cx, ids!(room_name)).set_text(cx, &room_info.room_name_id.to_string());
        // Hide the timestamp field, and use the latest message field to show the inviter.
        self.view.label(cx, ids!(timestamp)).set_text(cx, "");
        let inviter_string = match &room_info.inviter_info {
            Some(InviterInfo { user_id, display_name: Some(dn), .. }) => {
                let display_name = htmlize::escape_text(dn);
                let user_id = htmlize::escape_text(user_id.as_str());
                tr_fmt(
                    app_language,
                    "rooms_list_entry.invited.by_name_and_user",
                    &[("display_name", display_name.as_ref()), ("user_id", user_id.as_ref())],
                )
            }
            Some(InviterInfo { user_id, .. }) => {
                let user_id = htmlize::escape_text(user_id.as_str());
                tr_fmt(
                    app_language,
                    "rooms_list_entry.invited.by_user",
                    &[("user_id", user_id.as_ref())],
                )
            }
            None => tr_key(app_language, "rooms_list_entry.invited.generic").to_string(),
        };
        self.view.html_or_plaintext(cx, ids!(latest_message)).show_html(cx, &inviter_string);

        match room_info.room_avatar {
            FetchedRoomAvatar::Text(ref text) => {
                self.view.avatar(cx, ids!(avatar)).show_text(cx, None, None, text);
            }
            FetchedRoomAvatar::Image(ref img_bytes) => {
                let _ = self.view.avatar(cx, ids!(avatar)).show_image(
                    cx,
                    None, // Avatars in a RoomsListEntry shouldn't be clickable.
                    |cx, img| utils::load_png_or_jpg(&img, cx, img_bytes),
                );
            }
        }

        self.view
            .unread_badge(cx, ids!(unread_badge))
            .update_counts(false, 1, 0);

        self.view.view(cx, ids!(encryption_icon)).set_visible(cx, false);
        self.view.view(cx, ids!(bot_pill)).set_visible(cx, false);
        self.view.view(cx, ids!(tombstone_icon)).set_visible(cx, false);
        self.draw_common(cx, &room_info.room_avatar, room_info.is_selected);
    }

    /// Populates the widgets common to both invited and joined rooms list entries.
    pub fn draw_common(
        &mut self,
        cx: &mut Cx,
        room_avatar: &FetchedRoomAvatar,
        is_selected: bool,
    ) {
        match room_avatar {
            FetchedRoomAvatar::Text(text) => {
                self.view.avatar(cx, ids!(avatar)).show_text(cx, None, None, text);
            }
            FetchedRoomAvatar::Image(img_bytes) => {
                let _ = self.view.avatar(cx, ids!(avatar)).show_image(
                    cx,
                    None, // Avatars in a RoomsListEntry shouldn't be clickable.
                    |cx, img| utils::load_png_or_jpg(&img, cx, img_bytes),
                );
            }
        }

        if cx.display_context.is_desktop() {
            self.update_preview_colors(cx, is_selected);
        } else {
            // Mobile doesn't have a selected state. Always use the default colors.
            // We call the update in case the app was resized from desktop to mobile while the room was selected.
            // This can be optimized by only calling this when the app is resized.
            self.update_preview_colors(cx, false);
        }
    }

    /// Updates the styling of the preview based on whether the room is selected or not.
    pub fn update_preview_colors(&mut self, cx: &mut Cx, is_selected: bool) {
        use crate::shared::design_tokens::{
            RBX_BG_SUNKEN, RBX_FG_PRIMARY, RBX_FG_SECONDARY, RBX_FG_TERTIARY,
        };

        // The selected row uses a soft teal wash (RBX_BG_SELECTED) plus a teal
        // accent outline (see the draw_bg shader) to signal the active room, so
        // the text stays dark and fully legible in both states — i.e. the text
        // colors are identical for selected and unselected rows.
        let message_text_color = RBX_FG_SECONDARY;
        let room_name_color = RBX_FG_PRIMARY;
        let timestamp_color = RBX_FG_TERTIARY;
        let code_bg_color = RBX_BG_SUNKEN;

        // Toggle the background color via the animator (handles selected/deselected bg).
        self.animator_toggle(cx, is_selected, Animate::No, ids!(selected.on), ids!(selected.off));

        // NOTE: not every adaptive variant contains all of these widgets (e.g.
        // `IconAndName` has no timestamp/message preview, `OnlyIcon` has no room
        // name), and `script_apply_eval!` on an empty WidgetRef logs script-VM
        // errors ("__script_source__ not found"), so guard each apply.

        // Update text colors for room name.
        let mut room_name_label = self.view.label(cx, ids!(room_name));
        if !room_name_label.is_empty() {
            script_apply_eval!(cx, room_name_label, {
                draw_text +: {
                    color: #(room_name_color)
                }
            });
        }

        // Update text colors for timestamp.
        let mut timestamp_label = self.view.label(cx, ids!(timestamp));
        if !timestamp_label.is_empty() {
            script_apply_eval!(cx, timestamp_label, {
                draw_text +: {
                    color: #(timestamp_color)
                }
            });
        }

        // Update text colors for the latest message preview (both HTML and plaintext variants).
        let mut html_widget = self.view.html(cx, ids!(latest_message.html_view.html));
        if !html_widget.is_empty() {
            script_apply_eval!(cx, html_widget, {
                font_color: #(message_text_color),
                draw_text +: { color: #(message_text_color) },
                draw_block +: {
                    quote_bg_color: #(code_bg_color),
                    code_color: #(code_bg_color),
                }
            });
        }

        // Both states sit on a light surface (transparent / soft-teal wash), so
        // use the design-token link color in both cases for a consistent look.
        self.view
            .html_or_plaintext(cx, ids!(latest_message))
            .set_link_color(cx, Some(crate::shared::design_tokens::RBX_LINK));

        let mut pt_label = self.view.label(cx, ids!(latest_message.plaintext_view.pt_label));
        if !pt_label.is_empty() {
            script_apply_eval!(cx, pt_label, {
                draw_text +: {
                    color: #(message_text_color)
                }
            });
        }
    }
}

pub fn should_show_encryption_icon(is_encrypted: Option<bool>, is_tombstoned: bool) -> bool {
    matches!(is_encrypted, Some(true)) && !is_tombstoned
}

/// Whether the rooms-list row for `room_id` should display an agent badge.
///
/// True when the room is bound to a bot (app-service binding) OR it is a 1:1 DM
/// whose counterparty is a registered agent. Derived live from `AppState`, so it
/// updates as soon as an agent is registered or unregistered.
pub fn room_shows_agent_badge(
    app_state: &AppState,
    room_id: &RoomId,
    dm_target: Option<&UserId>,
) -> bool {
    app_state.bot_settings.is_room_bound(room_id)
        || dm_target.is_some_and(|user_id| app_state.agent_registry.contains(user_id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AgentEntry, AppState};
    use matrix_sdk::ruma::{OwnedRoomId, OwnedUserId};

    #[test]
    fn test_room_list_icon_visible_when_encrypted() {
        assert!(should_show_encryption_icon(Some(true), false));
    }

    #[test]
    fn test_room_list_icon_hidden_when_unencrypted() {
        assert!(!should_show_encryption_icon(Some(false), false));
    }

    #[test]
    fn test_room_list_icon_hidden_when_unknown() {
        assert!(!should_show_encryption_icon(None, false));
    }

    #[test]
    fn test_room_list_icon_yields_to_tombstone() {
        assert!(!should_show_encryption_icon(Some(true), true));
    }

    #[test]
    fn test_agent_badge_shown_when_room_bound() {
        let room_id: OwnedRoomId = "!room:example.org".try_into().unwrap();
        let mut app_state = AppState::default();
        app_state
            .bot_settings
            .record_known_bot_user_ids(["@bot:example.org".try_into().unwrap()]);
        // Bind the bot to the room so is_room_bound() is true.
        app_state.bot_settings.room_bindings.push(crate::app::RoomBotBindingState {
            room_id: room_id.clone(),
            bot_user_id: "@bot:example.org".try_into().unwrap(),
            remark: String::new(),
        });
        assert!(room_shows_agent_badge(&app_state, room_id.as_ref(), None));
    }

    #[test]
    fn test_agent_badge_shown_when_dm_target_is_registered_agent() {
        let room_id: OwnedRoomId = "!dm:example.org".try_into().unwrap();
        let agent: OwnedUserId = "@agent:example.org".try_into().unwrap();
        let mut app_state = AppState::default();
        app_state.agent_registry.register(agent.clone(), AgentEntry::default());
        assert!(room_shows_agent_badge(&app_state, room_id.as_ref(), Some(agent.as_ref())));
    }

    #[test]
    fn test_agent_badge_hidden_for_human_dm() {
        let room_id: OwnedRoomId = "!dm:example.org".try_into().unwrap();
        let human: OwnedUserId = "@human:example.org".try_into().unwrap();
        let app_state = AppState::default();
        assert!(!room_shows_agent_badge(&app_state, room_id.as_ref(), Some(human.as_ref())));
    }

    #[test]
    fn test_agent_badge_hidden_for_unbound_group_room() {
        let room_id: OwnedRoomId = "!group:example.org".try_into().unwrap();
        let app_state = AppState::default();
        assert!(!room_shows_agent_badge(&app_state, room_id.as_ref(), None));
    }

    #[test]
    fn test_agent_badge_hidden_when_dm_target_none() {
        let room_id: OwnedRoomId = "!dm:example.org".try_into().unwrap();
        let app_state = AppState::default();
        assert!(!room_shows_agent_badge(&app_state, room_id.as_ref(), None));
    }

    #[test]
    fn test_agent_badge_idempotent_when_bound_and_agent_dm() {
        let room_id: OwnedRoomId = "!dm:example.org".try_into().unwrap();
        let agent: OwnedUserId = "@agent:example.org".try_into().unwrap();
        let mut app_state = AppState::default();
        app_state.agent_registry.register(agent.clone(), AgentEntry::default());
        app_state.bot_settings.room_bindings.push(crate::app::RoomBotBindingState {
            room_id: room_id.clone(),
            bot_user_id: agent.clone(),
            remark: String::new(),
        });
        assert!(room_shows_agent_badge(&app_state, room_id.as_ref(), Some(agent.as_ref())));
    }

    #[test]
    fn test_agent_badge_hidden_after_room_unbound() {
        let room_id: OwnedRoomId = "!room:example.org".try_into().unwrap();
        let bot: OwnedUserId = "@bot:example.org".try_into().unwrap();
        let mut app_state = AppState::default();

        app_state
            .bot_settings
            .set_room_bound(room_id.clone(), Some(bot.clone()), true);
        assert!(room_shows_agent_badge(&app_state, room_id.as_ref(), None));

        app_state
            .bot_settings
            .set_room_bound(room_id.clone(), Some(bot), false);
        assert!(!room_shows_agent_badge(&app_state, room_id.as_ref(), None));
    }

    #[test]
    fn test_agent_badge_hidden_after_agent_registry_unbind() {
        let room_id: OwnedRoomId = "!dm:example.org".try_into().unwrap();
        let agent: OwnedUserId = "@agent:example.org".try_into().unwrap();
        let mut app_state = AppState::default();

        app_state.agent_registry.register(agent.clone(), AgentEntry::default());
        assert!(room_shows_agent_badge(&app_state, room_id.as_ref(), Some(agent.as_ref())));

        app_state.agent_registry.unregister(agent.as_ref());
        assert!(!room_shows_agent_badge(&app_state, room_id.as_ref(), Some(agent.as_ref())));
    }

    #[test]
    fn test_agent_badge_hidden_after_agentlab_unbind_clears_binding() {
        let current_user_id: OwnedUserId = "@alice:example.org".try_into().unwrap();
        let room_id: OwnedRoomId = "!room:example.org".try_into().unwrap();
        let agent: OwnedUserId = "@octos_mac:example.org".try_into().unwrap();
        let mut app_state = AppState::default();

        app_state.agent_registry.register(agent.clone(), AgentEntry::default());
        app_state.bot_settings.enabled = true;
        app_state.bot_settings.botfather_user_id = agent.to_string();
        app_state.bot_settings.record_known_bot_user_ids([agent.clone()]);
        app_state
            .bot_settings
            .set_room_bound(room_id.clone(), Some(agent.clone()), true);
        assert!(room_shows_agent_badge(&app_state, room_id.as_ref(), None));

        app_state.unregister_agent_and_clear_bot_identity(
            agent.as_ref(),
            Some(current_user_id.as_ref()),
        );

        assert!(!room_shows_agent_badge(&app_state, room_id.as_ref(), None));
    }

    #[test]
    fn test_rooms_list_entry_retains_adaptive_variants_for_batched_bot_pill() {
        let source = include_str!("rooms_list_entry.rs");
        let production_source = source.split("#[cfg(test)]").next().unwrap_or(source);

        assert!(
            production_source.contains("adaptive_preview := AdaptiveView {")
                && production_source.contains("retain_unused_variants: true")
                && production_source.contains("mod.widgets.RoomsListBotPill = RoundedView")
                && production_source.contains("new_batch: true"),
            "RoomsListEntry AdaptiveView must retain variants because bot pills own new_batch DrawLists",
        );
    }
}
