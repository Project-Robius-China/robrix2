//! The `RoomScreen` widget is the UI view that displays a single room or thread's timeline
//! of events (messages，state changes, etc.), along with an input bar at the bottom.

use std::{borrow::Cow, cell::{Cell, RefCell}, collections::VecDeque, ops::{DerefMut, Range}, sync::Arc, time::{Duration, Instant, SystemTime, UNIX_EPOCH}};

use bytesize::ByteSize;
use hashbrown::{HashMap, HashSet};
use imbl::Vector;
use makepad_widgets::{image_cache::ImageBuffer, *};
use matrix_sdk::{
    OwnedServerName, media::{MediaFormat, MediaRequestParameters}, room::{RoomMember, RoomMemberRole}, ruma::{
        EventId, MatrixToUri, MatrixUri, OwnedEventId, OwnedMxcUri, OwnedRoomId, RoomId, UserId, events::{
            receipt::Receipt,
            room::{
                ImageInfo, MediaSource, message::{
                    AudioMessageEventContent, EmoteMessageEventContent, FileMessageEventContent, FormattedBody, ImageMessageEventContent, KeyVerificationRequestEventContent, LocationMessageEventContent, MessageFormat, MessageType, NoticeMessageEventContent, RoomMessageEventContent, TextMessageEventContent, VideoMessageEventContent
                }
            },
            sticker::StickerEventContent,
        }, matrix_uri::MatrixId, uint
    }
};
use matrix_sdk_ui::timeline::{
    self, EmbeddedEvent, EncryptedMessage, EventSendState, EventTimelineItem, InReplyToDetails, LiveLocationState, MemberProfileChange, MembershipChange, MsgLikeContent, MsgLikeKind, OtherMessageLike, PollState, RoomMembershipChange, TimelineDetails, TimelineEventItemId, TimelineItem, TimelineItemContent, TimelineItemKind, VirtualTimelineItem
};
use ruma::{OwnedUserId, api::client::receipt::create_receipt::v3::ReceiptType, events::{AnySyncMessageLikeEvent, AnySyncTimelineEvent, SyncMessageLikeEvent}};

use matrix_sdk_ui::sync_service::State;
use crate::{
    app::{AppState, AppStateAction, BotSettingsState, ConfirmDeleteAction, SelectedRoom}, avatar_cache, event_preview::{plaintext_body_of_timeline_item, text_preview_of_encrypted_message, text_preview_of_member_profile_change, text_preview_of_other_message_like, text_preview_of_other_state, text_preview_of_room_membership_change, text_preview_of_timeline_item}, home::{bot_binding_modal::BotBindingModalAction, create_bot_modal::{CreateBotModalAction, CreateBotModalWidgetExt}, delete_bot_modal::{DeleteBotModalAction, DeleteBotModalWidgetExt}, edited_indicator::EditedIndicatorWidgetRefExt, encryption_notice::{EncryptionNoticeWidgetRefExt, first_other_member_display_name}, invite_modal::InviteModalAction, link_preview::{LinkPreviewCache, LinkPreviewRef, LinkPreviewWidgetRefExt}, loading_pane::{LoadingPaneState, LoadingPaneWidgetExt}, room_image_viewer::{get_image_name_and_filesize, populate_matrix_image_modal}, rooms_list::{RoomsListAction, RoomsListRef}, rooms_list_header::RoomsListHeaderAction, tombstone_footer::SuccessorRoomDetails}, i18n::{AppLanguage, tr_fmt, tr_key}, media_cache::{MediaCache, MediaCacheEntry}, profile::{
        user_profile::{ShowUserProfileAction, UserProfile, UserProfileAndRoomId, UserProfilePaneInfo, UserProfileSlidingPaneRef, UserProfileSlidingPaneWidgetExt},
        user_profile_cache,
    },
    room::{BasicRoomDetails, room_input_bar::{RoomInputBarState, RoomInputBarWidgetRefExt}, translation, typing_notice::TypingNoticeWidgetExt},
    shared::{
        attachment_download::{DownloadDisplayState, DownloadKind, DownloadableAttachment, PendingDownload, PendingDownloadState, mark_pending_download_finished, media_source_mxc, reset_pending_download, start_attachment_download}, avatar::{AvatarRef, AvatarState, AvatarWidgetExt, AvatarWidgetRefExt}, confirmation_modal::ConfirmationModalContent, forward_modal::{ForwardMessageContent, ForwardMessageModalAction}, html_or_plaintext::{HtmlOrPlaintextRef, HtmlOrPlaintextWidgetExt, HtmlOrPlaintextWidgetRefExt, RobrixHtmlLinkAction}, image_viewer::{ImageViewerAction, ImageViewerMetaData, LoadState}, jump_to_bottom_button::{JumpToBottomButtonWidgetExt, UnreadMessageCount}, popup_list::{PopupKind, enqueue_popup_notification, enqueue_notification, NotificationItem, NotificationAction, NotifActionStyle}, restore_status_view::RestoreStatusViewWidgetExt, styles::*, text_or_image::{TextOrImageAction, TextOrImageRef, TextOrImageWidgetRefExt}, timestamp::TimestampWidgetRefExt
    },
    sliding_sync::{BackwardsPaginateUntilEventRequest, FetchedRoomThread, MatrixRequest, PaginationDirection, RoomThreadsAction, SearchMessagesResultAction, SearchedMessage, TimelineEndpoints, TimelineKind, TimelinePaginationStatus, TimelineRequestSender, UserPowerLevels, current_user_id, get_client, submit_async_request, take_timeline_endpoints}, utils::{self, ImageFormat, MEDIA_THUMBNAIL_FORMAT, RoomNameId, unix_time_millis_to_datetime}
};
use crate::home::event_reaction_list::ReactionListWidgetRefExt;
use crate::home::room_read_receipt::AvatarRowWidgetRefExt;
use crate::home::rooms_list_entry::room_shows_agent_badge;
use crate::home::search_messages::{
    MessageSearchHit, SearchMessagesAction, SearchMessagesButtonWidgetExt,
    SearchMessagesSlidingPaneRef, SearchMessagesSlidingPaneWidgetExt,
};
use crate::home::streaming_animation::StreamingAnimState;
use crate::room::room_input_bar::RoomInputBarWidgetExt;
use crate::room::room_top_bar::{RoomTab, RoomTopBarAction, RoomTopBarWidgetExt};
use crate::shared::mentionable_text_input::MentionableTextInputAction;
use crate::shared::audio_message_player::AudioMessagePlayerWidgetRefExt;
use crate::shared::video_message_player::VideoMessagePlayerWidgetRefExt;
use crate::event_preview::{summarize_audio_message, summarize_video_message};
use crate::shared::animated_image::{AnimatedImageRef, AnimatedImageWidgetRefExt};
use crate::settings::app_preferences::effective_is_desktop;

use rangemap::RangeSet;

use super::{ContextMenuOpenGesture, event_reaction_list::ReactionData, invite_modal::is_invite_modal_open, loading_pane::LoadingPaneRef, new_message_context_menu::{MessageAbilities, MessageDetails}, room_read_receipt::{self, populate_read_receipts, MAX_VISIBLE_AVATARS_IN_READ_RECEIPT}};

mod bot_admin;
mod bot_message;
mod dsl;
mod octos_actions;
mod report_modal;
mod room_info_pane;
mod small_state;
mod state;
mod threads_pane;

pub use bot_admin::*;
use bot_message::*;
use octos_actions::*;
pub use report_modal::*;
pub use room_info_pane::*;
use small_state::*;
use state::*;
pub use threads_pane::*;
pub(crate) use bot_admin::is_known_or_likely_bot;
pub use state::{RoomScreenProps, RoomScreenTooltipActions, TimelineUpdate, clear_timeline_states};

/// The maximum number of timeline items to search through
/// when looking for a particular event.
///
/// This is a safety measure to prevent the main UI thread
/// from getting into a long-running loop if an event cannot be found quickly.
const MAX_ITEMS_TO_SEARCH_THROUGH: usize = 100;

/// The max size (width or height) of a blurhash image to decode.
/// Blurhash is a blurred placeholder — it is designed to be decoded at a small
/// size and then stretched by the GPU. Decoding at large sizes is extremely
/// expensive (CPU-bound, O(width*height)) and completely unnecessary since the
/// result is inherently blurry. A 32×32 decode is ~240x faster than 500×500
/// while being visually indistinguishable when scaled up.
pub(crate) const BLURHASH_IMAGE_MAX_SIZE: u32 = 32;

/// Use a larger batch when we are trying to fill the initial viewport,
/// otherwise many short messages can trigger a long chain of tiny paginations.
const VIEWPORT_FILL_PAGINATION_SIZE: u16 = 150;
const TIMELINE_UPDATE_TIME_BUDGET: Duration = Duration::from_millis(4);
const MAX_TIMELINE_UPDATES_PER_PASS: usize = 128;
const TOPIC_PREVIEW_CHARS: usize = 140;
const ROOM_INFO_PANE_DESKTOP_WIDTH: f32 = 320.0;
const ROOM_INFO_PANE_MOBILE_BREAKPOINT: f32 = 700.0;
const TRANSLATION_LANG_POPUP_WIDTH: f64 = 220.0;
const TRANSLATION_LANG_POPUP_SCROLL_HEIGHT: f64 = 288.0;
const TRANSLATION_LANG_POPUP_HEIGHT: f64 = TRANSLATION_LANG_POPUP_SCROLL_HEIGHT + 8.0;
const TRANSLATION_LANG_POPUP_GAP: f64 = 6.0;
const TRANSLATION_LANG_POPUP_MARGIN: f64 = 8.0;

fn invite_result_belongs_to_room_screen(
    pending_invited_users: &HashSet<OwnedUserId>,
    user_id: &OwnedUserId,
) -> bool {
    pending_invited_users.contains(user_id)
}

fn tl_idx_from_item_id(item_id: usize, has_encryption_notice: bool) -> Option<usize> {
    if has_encryption_notice {
        item_id.checked_sub(1)
    } else {
        Some(item_id)
    }
}

fn item_id_from_tl_idx(tl_idx: usize, has_encryption_notice: bool) -> usize {
    tl_idx + usize::from(has_encryption_notice)
}

/// Returns a single-line preview of `s` collapsing internal whitespace and
/// trimming to `max_chars` chars (counted in unicode scalar values). Appends
/// an ellipsis when truncation occurred.
fn truncate_preview(s: &str, max_chars: usize) -> String {
    // Collapse runs of whitespace (including newlines) into single spaces so
    // the preview reads as one line.
    let normalized: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= max_chars {
        return normalized;
    }
    let mut out: String = normalized.chars().take(max_chars).collect();
    out.push('…');
    out
}

/// Convert a server-search `SearchedMessage` into the UI-facing
/// `MessageSearchHit` consumed by `SearchMessagesSlidingPane`.
fn message_search_hit_from_searched_message(m: &SearchedMessage) -> MessageSearchHit {
    let sender_display = m
        .sender_display_name
        .clone()
        .unwrap_or_else(|| m.sender_user_id.to_string());
    let timestamp_display = unix_time_millis_to_datetime(m.timestamp)
        .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_default();
    MessageSearchHit {
        event_id: m.event_id.clone(),
        sender_display,
        timestamp_display,
        body_preview: truncate_preview(&m.body, 240),
    }
}
const MESSAGE_PROFILE_TOP_MARGIN: f64 = 4.5;
const MESSAGE_PROFILE_AVATAR_SIZE: f64 = 48.0;
const MESSAGE_USERNAME_ROW_HEIGHT: f64 = 18.0;
const MESSAGE_USERNAME_ROW_BOTTOM_MARGIN: f64 = 9.0;
const MESSAGE_USERNAME_RIGHT_MARGIN: f64 = 4.0;
const BOT_BADGE_HEIGHT: f64 = 16.0;
const BOT_BADGE_HORIZONTAL_PADDING: f64 = 6.0;
const BOT_BADGE_BORDER_RADIUS: f64 = 3.0;
const BOT_BADGE_TEXT_FONT_SIZE: f64 = 8.5;
const BOT_BADGE_TEXT_TOP_DROP: f64 = -0.08;

const fn centered_top_margin(outer_top_margin: f64, outer_height: f64, inner_height: f64) -> f64 {
    outer_top_margin + ((outer_height - inner_height) * 0.5)
}

#[cfg(test)]
const fn center_y(top_margin: f64, height: f64) -> f64 {
    top_margin + (height * 0.5)
}

const MESSAGE_USERNAME_ROW_TOP_MARGIN: f64 = centered_top_margin(
    MESSAGE_PROFILE_TOP_MARGIN,
    MESSAGE_PROFILE_AVATAR_SIZE,
    MESSAGE_USERNAME_ROW_HEIGHT,
);

#[cfg(test)]
fn message_profile_avatar_center_y() -> f64 {
    center_y(MESSAGE_PROFILE_TOP_MARGIN, MESSAGE_PROFILE_AVATAR_SIZE)
}

#[cfg(test)]
fn message_username_row_center_y() -> f64 {
    center_y(MESSAGE_USERNAME_ROW_TOP_MARGIN, MESSAGE_USERNAME_ROW_HEIGHT)
}

#[cfg(test)]
fn bot_badge_center_y_within_username_row() -> f64 {
    let bot_badge_top_margin = MESSAGE_USERNAME_ROW_TOP_MARGIN
        + ((MESSAGE_USERNAME_ROW_HEIGHT - BOT_BADGE_HEIGHT) * 0.5);
    center_y(bot_badge_top_margin, BOT_BADGE_HEIGHT)
}

#[cfg(test)]
fn bot_badge_label_center_y() -> f64 {
    let bot_badge_label_top_margin = (BOT_BADGE_HEIGHT - BOT_BADGE_TEXT_FONT_SIZE) * 0.5
        + (BOT_BADGE_TEXT_FONT_SIZE * BOT_BADGE_TEXT_TOP_DROP);
    center_y(bot_badge_label_top_margin, BOT_BADGE_TEXT_FONT_SIZE)
}

/// How a message's delivery should be shown in the timeline.
///
/// Only messages sent through the send queue have a delivery state at all — a
/// remote event that arrived over sync has none, and neither do the paths that
/// bypass the queue (`Room::send_raw`, used for agent-chat/octos routing, and
/// `send_attachment`). Those all render exactly as before.
#[derive(Debug, Clone, PartialEq, Eq)]
enum MessageDeliveryState {
    /// In flight. Normally lasts a couple of hundred milliseconds.
    Sending,
    /// Failed, but the queue will retry once connectivity returns.
    FailedRetrying,
    /// Failed and parked: it will not move again until the user acts.
    FailedWedged { reason: String },
}

impl MessageDeliveryState {
    /// Reads the delivery state of a timeline item, or `None` when the message
    /// has none to show (already delivered, or never went through the queue).
    fn from_item(item: &EventTimelineItem) -> Option<Self> {
        match item.send_state()? {
            EventSendState::NotSentYet { .. } => Some(Self::Sending),
            // `Sent` still describes a local echo, but one the server has
            // accepted — indistinguishable from delivered, so show nothing.
            EventSendState::Sent { .. } => None,
            EventSendState::SendingFailed { error, is_recoverable } => {
                if *is_recoverable {
                    Some(Self::FailedRetrying)
                } else {
                    Some(Self::FailedWedged {
                        reason: wedge_error_reason(error),
                    })
                }
            }
        }
    }
}

/// A short, human-readable reason for a send that is parked and needs the user.
///
/// `matrix_sdk::Error` is `#[non_exhaustive]`, so the outer match keeps a
/// fallback arm; `QueueWedgeError` is not, and is matched exhaustively so a new
/// SDK variant becomes a compile error here rather than a silently generic
/// message.
fn wedge_error_reason(error: &matrix_sdk::Error) -> String {
    use matrix_sdk::store::QueueWedgeError;
    match error {
        matrix_sdk::Error::SendQueueWedgeError(wedge) => match wedge.as_ref() {
            QueueWedgeError::InsecureDevices { .. } =>
                "Some devices in this room are unverified".to_string(),
            QueueWedgeError::IdentityViolations { .. } =>
                "A member's verified identity has changed".to_string(),
            QueueWedgeError::CrossVerificationRequired =>
                "This session must be verified before sending".to_string(),
            QueueWedgeError::MissingMediaContent =>
                "The attachment is no longer available".to_string(),
            QueueWedgeError::InvalidMimeType { mime_type } =>
                format!("Unsupported attachment type: {mime_type}"),
            QueueWedgeError::GenericApiError { msg } => msg.clone(),
        },
        other => other.to_string(),
    }
}


/// #FFF4E5
const COLOR_THREAD_SUMMARY_BG: Vec4 = vec4(1.0, 0.957, 0.898, 1.0);
/// #FFEACC
const COLOR_THREAD_SUMMARY_BG_HOVER: Vec4 = vec4(1.0, 0.918, 0.8, 1.0);

fn item_event_id(item: &Arc<TimelineItem>) -> Option<&EventId> {
    let TimelineItemKind::Event(event) = item.kind() else {
        return None;
    };
    event.event_id()
}

fn compute_translation_lang_popup_abs_pos(button_rect: Rect, container_rect: Rect) -> DVec2 {
    let min_x = container_rect.pos.x + TRANSLATION_LANG_POPUP_MARGIN;
    let max_x = (container_rect.pos.x + container_rect.size.x - TRANSLATION_LANG_POPUP_WIDTH - TRANSLATION_LANG_POPUP_MARGIN)
        .max(min_x);
    let popup_x = button_rect.pos.x
        .max(min_x)
        .min(max_x);

    let min_y = container_rect.pos.y + TRANSLATION_LANG_POPUP_MARGIN;
    let max_y = (container_rect.pos.y + container_rect.size.y - TRANSLATION_LANG_POPUP_HEIGHT - TRANSLATION_LANG_POPUP_MARGIN)
        .max(min_y);
    let popup_y_above = button_rect.pos.y - TRANSLATION_LANG_POPUP_HEIGHT - TRANSLATION_LANG_POPUP_GAP;
    let popup_y = if popup_y_above >= min_y {
        popup_y_above
    } else {
        (button_rect.pos.y + button_rect.size.y + TRANSLATION_LANG_POPUP_GAP)
            .max(min_y)
            .min(max_y)
    };

    dvec2(popup_x, popup_y)
}

/// Registers this module's DSL blocks in dependency order: the sliding
/// panes, the report modal, and the app-service panel first — the
/// `RoomScreen` template in `dsl` references them via `mod.widgets.*`, and a
/// template must already be registered when a later block resolves such a
/// reference.
pub fn script_mod(vm: &mut ScriptVm) {
    threads_pane::script_mod(vm);
    room_info_pane::script_mod(vm);
    report_modal::script_mod(vm);
    bot_admin::script_mod(vm);
    dsl::script_mod(vm);
}

/// The main widget that displays a single Matrix room.
#[derive(Script, Widget)]
pub struct RoomScreen {
    #[deref] view: View,

    /// The name and ID of the currently-shown room, if any.
    #[rust] room_name_id: Option<RoomNameId>,
    /// The avatar URL of the currently-shown room, if any.
    #[rust] room_avatar_url: Option<OwnedMxcUri>,
    /// The timeline currently displayed by this RoomScreen, if any.
    #[rust] timeline_kind: Option<TimelineKind>,
    /// The persistent UI-relevant states for the room that this widget is currently displaying.
    #[rust] tl_state: Option<TimelineUiState>,
    /// Whether this RoomScreen is currently visible and should consume room-specific signals.
    #[rust] timeline_updates_enabled: bool,
    /// Restarts paused streaming timers on the first signal after becoming visible.
    #[rust] resume_timeline_on_next_signal: bool,
    /// Cached, prebuilt member rows for the room-info People list (see
    /// [`RoomInfoMembersCache`]). Avoids rebuilding/sorting the full member list
    /// on every Signal-driven info refresh.
    #[rust] room_info_members_cache: Option<RoomInfoMembersCache>,
    /// Derived bot identities for the current immutable room-member snapshot.
    #[rust] timeline_bot_context_cache: Option<CachedTimelineBotContext>,
    /// The set of pinned events in this room.
    #[rust] pinned_events: Vec<OwnedEventId>,
    /// Whether this room has been successfully loaded (received from the homeserver).
    #[rust] is_loaded: bool,
    /// Whether or not all rooms have been loaded (received from the homeserver).
    #[rust] all_rooms_loaded: bool,
    /// NextFrame subscription for driving streaming typewriter animation.
    #[rust]
    streaming_next_frame: NextFrame,
    /// Timeout used to evict stalled streaming states without per-frame polling.
    #[rust]
    streaming_timeout_timer: Timer,
    /// Timeout that redraws visible approval cards when their deadline passes.
    #[rust]
    approval_expiry_timer: Timer,
    /// Absolute deadline represented by `approval_expiry_timer`.
    ///
    /// Keeping this separately avoids stopping and recreating the same timer on
    /// every draw while a room is being scrolled.
    #[rust]
    approval_expiry_deadline_millis: Option<u64>,
    /// Desktop/mobile layout state already pushed into the widget tree.
    #[rust]
    applied_layout_state: Option<AppliedRoomLayoutState>,
    /// Whether the previous draw included the leading encryption notice row.
    ///
    /// A change shifts every PortalList item ID, so the per-timeline drawn
    /// caches must be invalidated before cached widgets can be reused.
    #[rust]
    last_has_encryption_notice: Option<bool>,
    /// Whether the in-room app service quick actions card is currently visible.
    #[rust] show_app_service_actions: bool,
    #[rust] threads_pane_state: ThreadsPaneState,
    #[rust] app_language: AppLanguage,
    #[rust] app_language_initialized: bool,
    #[rust] pending_invited_users: HashSet<OwnedUserId>,
    #[rust] octos_action_button_contexts: HashMap<WidgetUid, OctosActionButtonContext>,
    #[rust] disabled_octos_action_source_event_ids: HashSet<OwnedEventId>,
    #[rust] selected_octos_action_by_source_event_id: HashMap<OwnedEventId, SelectedOctosActionState>,
    /// Per-room state for the server-side search pane. Tracks the active
    /// query, the room it targets, the most recent `next_batch` token, and
    /// whether a request is currently in flight.
    #[rust] search_state: RoomSearchState,
    /// Which body tab (Chat / Info) is currently shown in the mobile layout.
    /// Reset to `Chat` whenever a new room is displayed. Unused on desktop.
    #[rust] active_room_tab: RoomTab,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AppliedRoomLayoutState {
    is_desktop: bool,
    active_room_tab: RoomTab,
}

/// Tracks the active server-side message search shown in the
/// `SearchMessagesSlidingPane`. Reset whenever the pane is closed, the
/// query is cleared, or the room changes.
#[derive(Default, Debug)]
pub struct RoomSearchState {
    /// The query string currently being searched. Empty when idle.
    pub query: String,
    /// The room the active query targets (used to ignore stale results
    /// arriving after a room switch). `None` when idle.
    pub room_id: Option<OwnedRoomId>,
    /// `next_batch` token returned by the most recent search response.
    /// `Some` means more pages are available.
    pub next_batch: Option<String>,
    /// Whether a request is currently in flight (initial or paginated).
    pub request_in_flight: bool,
}

impl RoomSearchState {
    fn reset(&mut self) {
        self.query.clear();
        self.room_id = None;
        self.next_batch = None;
        self.request_in_flight = false;
    }
}

impl Drop for RoomScreen {
    fn drop(&mut self) {
        // This ensures that the `TimelineUiState` instance owned by this room is *always* returned
        // back to to `TIMELINE_STATES`, which ensures that its UI state(s) are not lost
        // and that other RoomScreen instances can show this room in the future.
        // RoomScreen will be dropped whenever its widget instance is destroyed, e.g.,
        // when a Tab is closed or the app is resized to a different AdaptiveView layout.
        self.hide_timeline();
    }
}

impl ScriptHook for RoomScreen {
    fn on_after_reload(&mut self, vm: &mut ScriptVm) {
        vm.with_cx_mut(|cx| {
            self.applied_layout_state = None;
            self.last_has_encryption_notice = None;
            if let Some(tl_state) = &mut self.tl_state.as_mut() {
                // Clear the timeline's drawn items caches and redraw it.
                tl_state.content_drawn_since_last_update.clear();
                tl_state.profile_drawn_since_last_update.clear();
                self.view.redraw(cx);
            }
        });
    }
}

impl Widget for RoomScreen {
    // Handle events and actions for the RoomScreen widget and its inner Timeline view.
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if !self.timeline_updates_enabled
            && (
                matches!(event, Event::Signal)
                    || self.streaming_next_frame.is_event(event).is_some()
                    || self.streaming_timeout_timer.is_event(event).is_some()
                    || self.approval_expiry_timer.is_event(event).is_some()
            )
        {
            return;
        }
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if !self.app_language_initialized || self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }
        let room_screen_widget_uid = self.widget_uid();
        let portal_list = self.portal_list(cx, ids!(timeline.list));
        let user_profile_sliding_pane = self.user_profile_sliding_pane(cx, ids!(user_profile_sliding_pane));
        let threads_sliding_pane = self.threads_sliding_pane(cx, ids!(threads_sliding_pane));
        let room_info_sliding_pane = self.room_info_sliding_pane(cx, ids!(room_info_sliding_pane));
        let room_info_sliding_pane_widget_uid = room_info_sliding_pane.widget_uid();
        // The mobile "Info" tab reuses a second RoomInfoSlidingPane instance
        // (`info_content`, inline). Its action buttons (Invite / People /
        // Report / Leave / member taps) emit `RoomInfoPaneAction`s under this
        // uid, so route them the same as the overlay pane's.
        let info_content_widget_uid = self.room_info_sliding_pane(cx, ids!(info_content)).widget_uid();
        let loading_pane = self.loading_pane(cx, ids!(loading_pane));

        // Streaming animation frame handler
        if let Some(_ne) = self.streaming_next_frame.is_event(event) {
            #[cfg(debug_assertions)]
            #[allow(unused_variables)]
            let frame_start = std::time::Instant::now();
            let has_encryption_notice = self.current_has_encryption_notice(cx);

            if let Some(tl) = self.tl_state.as_mut() {
                let mut needs_another_frame = false;
                let mut completed_ids = Vec::new();
                let mut redraw_candidate_indices = Vec::new();

                for (event_id, state) in tl.streaming_messages.iter_mut() {
                    if state.needs_frame() {
                        if state.tick() {
                            // Invalidate draw cache so item gets re-populated
                            if let Some(idx) = state.timeline_index {
                                tl.content_drawn_since_last_update.remove(idx..idx + 1);
                            }
                            redraw_candidate_indices.push(state.timeline_index);
                        }
                        needs_another_frame |= state.needs_frame();
                    }

                    if state.is_complete() || state.is_timed_out() {
                        completed_ids.push(event_id.clone());
                        redraw_candidate_indices.push(state.timeline_index);
                    }
                }

                for id in &completed_ids {
                    tl.streaming_messages.remove(id);
                }

                // Safety cap: max 50 streaming entries
                while tl.streaming_messages.len() > 50 {
                    if let Some((oldest_id, oldest_idx)) = tl.streaming_messages.iter()
                        .min_by_key(|(_, s)| s.animation_start_time)
                        .map(|(id, state)| (id.clone(), state.timeline_index))
                    {
                        tl.streaming_messages.remove(&oldest_id);
                        redraw_candidate_indices.push(oldest_idx);
                    }
                }

                if needs_another_frame {
                    self.streaming_next_frame = cx.new_next_frame();
                }

                if any_timeline_indices_visible(
                    redraw_candidate_indices.iter().copied(),
                    |idx| {
                        portal_list
                            .get_item(item_id_from_tl_idx(idx, has_encryption_notice))
                            .is_some()
                    },
                ) {
                    self.redraw_timeline_list(cx);
                }
            }

            #[cfg(debug_assertions)]
            {
                if let Some(tl) = self.tl_state.as_ref() {
                    let elapsed = frame_start.elapsed();
                    if elapsed.as_millis() > 2 {
                        log!("Streaming animation frame took {}ms ({} active streams)",
                            elapsed.as_millis(), tl.streaming_messages.len());
                    }
                }
            }

            self.schedule_stream_timeout(cx);
        }

        if self.streaming_timeout_timer.is_event(event).is_some() {
            let has_encryption_notice = self.current_has_encryption_notice(cx);
            if let Some(tl) = self.tl_state.as_mut() {
                let timed_out_entries: Vec<(OwnedEventId, Option<usize>)> = tl
                    .streaming_messages
                    .iter()
                    .filter_map(|(event_id, state)| {
                        if state.is_timed_out() || state.is_complete() {
                            Some((event_id.clone(), state.timeline_index))
                        } else {
                            None
                        }
                    })
                    .collect();

                for (event_id, _) in &timed_out_entries {
                    tl.streaming_messages.remove(event_id);
                }

                if any_timeline_indices_visible(
                    timed_out_entries.iter().map(|(_, idx)| *idx),
                    |idx| {
                        portal_list
                            .get_item(item_id_from_tl_idx(idx, has_encryption_notice))
                            .is_some()
                    },
                ) {
                    self.redraw_timeline_list(cx);
                }
            }

            self.schedule_stream_timeout(cx);
        }

        if self.approval_expiry_timer.is_event(event).is_some() {
            self.approval_expiry_timer = Timer::empty();
            self.approval_expiry_deadline_millis = None;
            if self.expire_approval_contexts(current_unix_time_millis()) {
                self.redraw_timeline_list(cx);
            }
            // A timeout can fire marginally before its wall-clock deadline.
            // Re-arm the next still-pending request after expired contexts
            // have been removed.
            self.schedule_approval_expiry(cx);
        }

        // Handle actions here before processing timeline updates.
        // Normally (in most other widgets), the order of event handling doesn't matter much.
        // However, since actions may refer to a specific timeline item's index,
        // we want to handle those before processing any updates that might change
        // the set of timeline indices (which would invalidate the index values in any actions).
        if let Event::Actions(actions) = event {
            let has_encryption_notice = self.current_has_encryption_notice(cx);
            for (index, wr) in portal_list.items_with_actions(actions) {
                // Handle a hover-in action on the reaction list: show a reaction summary.
                let reaction_list = wr.reaction_list(cx, ids!(reaction_list));
                if let RoomScreenTooltipActions::HoverInReactionButton {
                    widget_rect,
                    reaction_data,
                } = reaction_list.hovered_in(actions) {
                    let Some(_tl_state) = self.tl_state.as_ref() else { continue };
                    let tooltip_text_arr: Vec<String> = reaction_data.reaction_senders
                        .iter()
                        .map(|(sender, _react_info)| {
                            user_profile_cache::get_user_display_name_for_room(
                                cx,
                                sender.clone(),
                                Some(&reaction_data.room_id),
                                true,
                            )
                            .into_option()
                            .unwrap_or_else(|| sender.to_string())
                        })
                        .collect();

                    let mut tooltip_text = utils::human_readable_list(&tooltip_text_arr, MAX_VISIBLE_AVATARS_IN_READ_RECEIPT);
                    tooltip_text.push_str(&tr_fmt(self.app_language, "room_screen.tooltip.reacted_with_suffix", &[
                        ("reaction", reaction_data.reaction.as_str()),
                    ]));
                    cx.widget_action(
                        room_screen_widget_uid, 
                        TooltipAction::HoverIn {
                            text: tooltip_text,
                            widget_rect,
                            options: CalloutTooltipOptions {
                                position: TooltipPosition::Bottom,
                                ..Default::default()
                            },
                        },
                    );
                }

                // Handle a hover-out action on the reaction list or avatar row.
                let avatar_row_ref = wr.avatar_row(cx, ids!(avatar_row));
                if reaction_list.hovered_out(actions)
                    || avatar_row_ref.hover_out(actions)
                {
                    cx.widget_action(
                        room_screen_widget_uid, 
                        TooltipAction::HoverOut,
                    );
                }

                // Handle a hover-in action on the avatar row: show a read receipts summary.
                if let RoomScreenTooltipActions::HoverInReadReceipt {
                    widget_rect,
                    read_receipts
                } = avatar_row_ref.hover_in(actions) {
                    let Some(room_id) = self.room_id() else { return; };
                    let tooltip_text= room_read_receipt::populate_tooltip(cx, read_receipts, room_id);
                    cx.widget_action(
                        room_screen_widget_uid, 
                        TooltipAction::HoverIn {
                            text: tooltip_text,
                            widget_rect,
                            options: CalloutTooltipOptions {
                                position: TooltipPosition::Left,
                                ..Default::default()
                            },
                        },
                    );
                }

                // Handle an image within the message being clicked.
                let content_message = wr.text_or_image(cx, ids!(content.message));
                if let TextOrImageAction::Clicked(mxc_uri) = actions.find_widget_action(content_message.widget_uid()).cast() {
                    let texture = content_message.get_texture(cx);
                    self.handle_image_click(
                        cx,
                        mxc_uri,
                        texture,
                        index,
                    );
                    continue;
                }

                let summary_clicked = wr.button(cx, ids!(state_group_toggle_button)).clicked(actions);
                let header_clicked = wr.button(cx, ids!(group_header.state_group_toggle_button)).clicked(actions);
                if summary_clicked || header_clicked {
                    log!(
                        "[encryption-notice/toggle] click reached: index={index}, has_encryption_notice={has_encryption_notice}, summary_clicked={summary_clicked}, header_clicked={header_clicked}"
                    );
                    let Some(tl_idx) = tl_idx_from_item_id(index, has_encryption_notice) else {
                        log!("[encryption-notice/toggle] tl_idx_from_item_id returned None for index={index}, skipping");
                        continue;
                    };
                    log!("[encryption-notice/toggle] calling toggle_small_state_event_group(tl_idx={tl_idx})");
                    self.toggle_small_state_event_group(cx, tl_idx);
                    continue;
                }

                // "Show more" / "Show less" on a folded long bot reply.
                if wr
                    .button(cx, ids!(content.bot_message_card.bot_body_card.bot_body_fold_toggle))
                    .clicked(actions)
                {
                    if let Some(tl_idx) = tl_idx_from_item_id(index, has_encryption_notice) {
                        self.toggle_bot_body_expanded(cx, tl_idx);
                    }
                    continue;
                }

                // "Show more" / "Show less" on a folded plain message. Shares the
                // expanded-ids set with the bot-card toggle: both are "this event
                // is unfolded", and a message is only ever one of the two.
                if wr
                    .button(cx, ids!(content.message_action_bar.plain_fold_toggle))
                    .clicked(actions)
                {
                    if let Some(tl_idx) = tl_idx_from_item_id(index, has_encryption_notice) {
                        self.toggle_bot_body_expanded(cx, tl_idx);
                    }
                    continue;
                }

                // "Retry" / "Discard" on a message whose send is parked.
                let retry_clicked = wr
                    .button(cx, ids!(content.send_failure_section.send_failure_actions.send_retry_button))
                    .clicked(actions);
                let discard_clicked = wr
                    .button(cx, ids!(content.send_failure_section.send_failure_actions.send_discard_button))
                    .clicked(actions);
                if retry_clicked || discard_clicked {
                    if let Some(tl_idx) = tl_idx_from_item_id(index, has_encryption_notice) {
                        self.act_on_failed_send(cx, tl_idx, retry_clicked);
                    }
                    continue;
                }

                // Handle the invite_user_button (in a SmallStateEvent) being clicked.
                if wr.button(cx, ids!(event_row.invite_user_button)).clicked(actions) {
                    let Some(tl_idx) = tl_idx_from_item_id(index, has_encryption_notice) else { continue };
                    let Some(tl) = self.tl_state.as_ref() else { continue };
                    if let Some(event_tl_item) = tl.items.get(tl_idx).and_then(|item| item.as_event()) {
                        let user_id = event_tl_item.sender().to_owned();
                        let username = if let TimelineDetails::Ready(profile) = event_tl_item.sender_profile() {
                            profile.display_name.as_deref().unwrap_or(user_id.as_str())
                        } else {
                            user_id.as_str()
                        };
                        let room_id = tl.kind.room_id().clone();
                        let app_language = self.app_language;
                        let content = ConfirmationModalContent {
                            title_text: tr_key(app_language, "room_screen.modal.invite.title").into(),
                            body_text: tr_fmt(app_language, "room_screen.modal.invite.body", &[("username", username)]).into(),
                            accept_button_text: Some(tr_key(app_language, "room_screen.modal.invite.accept").into()),
                            on_accept_clicked: Some(Box::new(move |cx| {
                                // Record pending ownership in every RoomScreen of this
                                // room BEFORE the result arrives (see InviteUserRequested).
                                cx.action(InviteAction::InviteUserRequested {
                                    room_id: room_id.clone(),
                                    user_id: user_id.clone(),
                                });
                                submit_async_request(MatrixRequest::InviteUser { room_id, user_id });
                            })),
                            ..Default::default()
                        };
                        cx.action(InviteAction::ShowInviteConfirmationModal(RefCell::new(Some(content))));
                    }
                }
            }

            self.handle_message_actions(cx, actions, &portal_list, &loading_pane, scope);

            for action in actions {
                // Mobile RoomTopBar (header + Chat/Info tabs) actions.
                match action.as_widget_action().cast::<RoomTopBarAction>() {
                    RoomTopBarAction::Back => {
                        cx.widget_action(room_screen_widget_uid, StackNavigationAction::Pop);
                    }
                    RoomTopBarAction::Search => {
                        cx.widget_action(room_screen_widget_uid, SearchMessagesAction::OpenRequested);
                    }
                    RoomTopBarAction::TabSelected(tab) => {
                        self.active_room_tab = tab;
                        if matches!(tab, RoomTab::Info) {
                            // Lazy-load: only fetch the room's members the first
                            // time the user opens the Info tab (the members list
                            // backs the inline info / People sub-page). When they
                            // arrive, the inline pane is re-populated below.
                            if let Some(tl) = self.tl_state.as_ref() {
                                if tl.room_members.is_none() {
                                    submit_async_request(MatrixRequest::GetRoomMembers {
                                        timeline_kind: tl.kind.clone(),
                                        memberships: matrix_sdk::RoomMemberships::JOIN,
                                        local_only: false,
                                    });
                                }
                            }
                            self.refresh_inline_room_info(cx, scope.data.get::<AppState>());
                        }
                        self.redraw(cx);
                    }
                    RoomTopBarAction::None => {}
                }

                if let Some(AppStateAction::AgentRegistryUpdated) = action.downcast_ref() {
                    self.invalidate_timeline_bot_context();
                    if room_info_sliding_pane.is_currently_shown(cx) {
                        self.refresh_room_info_pane(cx, scope.data.get::<AppState>());
                    }
                    if matches!(self.active_room_tab, RoomTab::Info) {
                        self.refresh_inline_room_info(cx, scope.data.get::<AppState>());
                    }
                    self.redraw(cx);
                }

                // Handle actions related to restoring the previously-saved state of rooms.
                if let Some(AppStateAction::RoomLoadedSuccessfully { room_name_id, ..}) = action.downcast_ref() {
                    if self.room_name_id.as_ref().is_some_and(|rn| rn.room_id() == room_name_id.room_id()) {
                        // `set_displayed_room()` does nothing if the room_name_id is unchanged, so we clear it first.
                        self.room_name_id = None;
                        let thread_root_event_id = self.timeline_kind.as_ref()
                            .and_then(|k| k.thread_root_event_id().cloned());
                        self.set_displayed_room(cx, room_name_id, thread_root_event_id);
                        return;
                    }
                }

                // Handle a bot picked in the `/invitebot` picker: dispatch the invite.
                // The action is widget-addressed to exactly this RoomScreen (see
                // on_bot_invite_selected), so even when the same room is shown by
                // multiple RoomScreens (main timeline + thread tab) only one
                // instance dispatches. Success/failure feedback arrives via the
                // InviteResultAction pipeline below.
                if let Some(widget_action) = action.as_widget_action() {
                    if widget_action.widget_uid == self.widget_uid() {
                        if let MentionableTextInputAction::InviteBotSelected { room_id, user_id } =
                            widget_action.cast()
                        {
                            // Optimistically record the pending invite so a
                            // reopened picker can't offer the same bot again
                            // during the network round-trip; the
                            // InviteResultAction::Failed handler rolls it back.
                            self.pending_invited_users.insert(user_id.clone());
                            submit_async_request(MatrixRequest::InviteUser { room_id, user_id });
                        }
                    }
                }

                // An invite was just submitted from a closure-based initiator
                // (knock-approve, Retry) or the invite modal: record pending
                // ownership so the InviteResultAction feedback below fires.
                if let Some(InviteAction::InviteUserRequested { room_id, user_id }) =
                    action.downcast_ref()
                {
                    if self.room_name_id.as_ref().is_some_and(|rn| rn.room_id() == room_id) {
                        self.pending_invited_users.insert(user_id.clone());
                    }
                }

                // Handle InviteResultAction to show popup notifications.
                if let Some(InviteResultAction::Sent { room_id, user_id }) = action.downcast_ref() {
                    // Only the RoomScreen that originated the invite owns its UI feedback.
                    if self.room_name_id.as_ref().is_some_and(|rn| rn.room_id() == room_id)
                        && invite_result_belongs_to_room_screen(&self.pending_invited_users, user_id)
                    {
                        self.pending_invited_users.insert(user_id.clone());
                        enqueue_popup_notification(
                            "Invite sent. Waiting for acceptance.",
                            PopupKind::Info,
                            Some(4.0),
                        );
                        if let Some(app_state) = scope.data.get::<AppState>()
                            && app_state.bot_settings.enabled
                        {
                            if let Ok(bot_user_id) = app_state
                                .bot_settings
                                .resolved_bot_user_id_for_room(room_id, current_user_id().as_deref())
                            {
                                if &bot_user_id == user_id
                                    && app_state
                                        .bot_settings
                                        .bound_bot_user_id(room_id.as_ref())
                                        .is_none_or(|existing_bot_user_id| existing_bot_user_id.as_str() != user_id.as_str())
                                {
                                    cx.action(AppStateAction::BotRoomBindingUpdated {
                                        room_id: room_id.clone(),
                                        bound: true,
                                        bot_user_id: Some(user_id.clone()),
                                        warning: None,
                                    });
                                }
                            }
                        }
                    }
                }
                if let Some(InviteResultAction::Failed { room_id, user_id, error }) = action.downcast_ref() {
                    // Only the RoomScreen that originated the invite owns its UI feedback.
                    if self.room_name_id.as_ref().is_some_and(|rn| rn.room_id() == room_id)
                        && invite_result_belongs_to_room_screen(&self.pending_invited_users, user_id)
                    {
                        self.pending_invited_users.remove(user_id);
                        let error_text = error.to_string();
                        let error_display = error_text.clone();
                        let room_id_retry = room_id.clone();
                        let user_id_retry = user_id.clone();
                        enqueue_notification(NotificationItem {
                            kind: PopupKind::Error,
                            title: Some("Invite failed".into()),
                            message: tr_fmt(self.app_language, "room_screen.popup.invite.failed", &[
                                ("error", error_text.as_str()),
                            ]).into(),
                            actions: vec![
                                NotificationAction::new("Retry", NotifActionStyle::Primary, move |cx| {
                                    // Re-establish pending ownership (the Failed handler
                                    // just removed it) so the retry's result shows feedback.
                                    cx.action(InviteAction::InviteUserRequested {
                                        room_id: room_id_retry.clone(),
                                        user_id: user_id_retry.clone(),
                                    });
                                    submit_async_request(MatrixRequest::InviteUser {
                                        room_id: room_id_retry.clone(),
                                        user_id: user_id_retry.clone(),
                                    });
                                }),
                                NotificationAction::new("Copy details", NotifActionStyle::Neutral, move |cx| {
                                    cx.copy_to_clipboard(&error_display);
                                }),
                            ],
                            auto_dismissal_duration: None,
                            ..Default::default()
                        });
                    }
                }
                if let Some(ActionResponseResultAction::Failed { room_id, source_event_id, error }) = action.downcast_ref() {
                    if self.room_name_id.as_ref().is_some_and(|rn| rn.room_id() == room_id) {
                        clear_action_buttons_disabled(
                            &mut self.disabled_octos_action_source_event_ids,
                            source_event_id.as_ref(),
                        );
                        clear_selected_octos_action(
                            &mut self.selected_octos_action_by_source_event_id,
                            source_event_id.as_ref(),
                        );
                        self.invalidate_timeline_event_content(source_event_id.as_ref());
                        self.redraw_timeline_list(cx);
                        enqueue_popup_notification(
                            tr_fmt(
                                self.app_language,
                                "room_screen.popup.action_response.failed",
                                &[("error", error.as_str())],
                            ),
                            PopupKind::Error,
                            Some(5.0),
                        );
                    }
                }

                // No `widget_uid_eq` filter here — `OpenThread` is emitted from
                // a `ThreadsPaneEntry` (a list item), not from the pane itself,
                // so its widget_uid is the entry's. `LoadMoreRequested` and
                // `CloseRequested` come from the pane, but `cast_ref` handles
                // all three regardless of emitter.
                match action.as_widget_action().cast_ref::<ThreadsPaneAction>() {
                    ThreadsPaneAction::OpenThread(thread_root_event_id) => {
                        log!("RoomScreen: OpenThread received, jumping to {}", thread_root_event_id);
                        threads_sliding_pane.hide(cx);
                        self.view.threads_button(cx, ids!(timeline.threads_button))
                            .set_visible(cx, true);
                        self.jump_to_event(
                            cx,
                            thread_root_event_id,
                            None,
                            &portal_list,
                            &loading_pane,
                        );
                    }
                    ThreadsPaneAction::LoadMoreRequested => {
                        self.request_more_threads(cx, true);
                    }
                    ThreadsPaneAction::CloseRequested => {
                        threads_sliding_pane.hide(cx);
                        self.view.threads_button(cx, ids!(timeline.threads_button))
                            .set_visible(cx, true);
                    }
                    ThreadsPaneAction::None => {}
                }

                let room_info_widget_action = action.as_widget_action();
                match room_info_widget_action
                    .widget_uid_eq(room_info_sliding_pane_widget_uid)
                    .or_else(|| room_info_widget_action.widget_uid_eq(info_content_widget_uid))
                    .cast_ref()
                {
                    RoomInfoPaneAction::InviteUser => {
                        if let Some(room_name_id) = self.room_name_id.as_ref().cloned() {
                            cx.action(InviteModalAction::Open(room_name_id));
                        }
                    }
                    RoomInfoPaneAction::ShowPeoplePage => {
                        if let Some(tl) = self.tl_state.as_ref() {
                            submit_async_request(MatrixRequest::GetRoomMembers {
                                timeline_kind: tl.kind.clone(),
                                memberships: matrix_sdk::RoomMemberships::JOIN,
                                local_only: false,
                            });
                        }
                    }
                    RoomInfoPaneAction::OpenPeopleProfile(user_id) => {
                        let Some(room_name_id) = self.room_name_id.as_ref().cloned() else { continue };
                        let room_member = self.tl_state.as_ref()
                            .and_then(|tl| tl.room_members.as_ref())
                            .and_then(|members| members.iter().find(|member| member.user_id() == user_id).cloned());
                        let username = room_member.as_ref()
                            .and_then(|member| member.display_name().map(ToOwned::to_owned));
                        let avatar_state = AvatarState::Known(
                            room_member
                                .as_ref()
                                .and_then(|member| member.avatar_url().map(ToOwned::to_owned))
                        );
                        let can_change_room_power_levels = self.tl_state.as_ref()
                            .is_some_and(|tl| tl.user_power.can_change_room_power_levels());
                        self.show_user_profile(
                            cx,
                            &user_profile_sliding_pane,
                            UserProfilePaneInfo {
                                profile_and_room_id: UserProfileAndRoomId {
                                    user_profile: UserProfile {
                                        user_id: user_id.clone(),
                                        username,
                                        avatar_state,
                                    },
                                    room_id: room_name_id.room_id().clone(),
                                },
                                room_name: room_name_id.to_string(),
                                room_member,
                                can_change_room_power_levels,
                            },
                        );
                    }
                    RoomInfoPaneAction::ReportRoom => {
                        // Open the GLOBAL report modal (app root) so it survives
                        // mobile<->desktop AdaptiveView rebuilds. Carry the room.
                        if let Some(room_name_id) = self.room_name_id.clone() {
                            cx.action(ReportRoomModalAction::Open {
                                room_id: room_name_id.room_id().clone(),
                                room_name_id,
                            });
                        }
                    }
                    RoomInfoPaneAction::LeaveRoom => {
                        // Route to the GLOBAL delete-confirmation modal (app root)
                        // so it survives mobile<->desktop AdaptiveView rebuilds.
                        // The room_id is captured in the accept callback.
                        if let Some(room_id) = self.room_id().cloned() {
                            let room_name = self
                                .room_name_id
                                .as_ref()
                                .map(|r| r.to_string())
                                .unwrap_or_default();
                            let content = ConfirmationModalContent {
                                title_text: String::from("Leave Room").into(),
                                body_text: format!("Are you sure you want to leave {room_name}?").into(),
                                accept_button_text: Some(String::from("Leave").into()),
                                cancel_button_text: Some(String::from("Cancel").into()),
                                on_accept_clicked: Some(Box::new(move |_cx| {
                                    submit_async_request(MatrixRequest::LeaveRoom { room_id });
                                })),
                                ..Default::default()
                            };
                            cx.action(ConfirmDeleteAction::Show(RefCell::new(Some(content))));
                        }
                    }
                    // Bubbled by the pane itself into `OpenPeopleProfile`
                    // (handled above); nothing to do here.
                    RoomInfoPaneAction::PersonClicked(_) => {}
                    RoomInfoPaneAction::None => {}
                }

                if let Some(RoomThreadsAction::Loaded { room_id, from, threads, prev_batch_token }) = action.downcast_ref() {
                    if self.threads_pane_state.room_id.as_ref().is_some_and(|current| current == room_id) {
                        self.on_threads_loaded(
                            cx,
                            from.as_ref(),
                            threads,
                            prev_batch_token.clone(),
                        );
                    }
                }
                if let Some(RoomThreadsAction::Failed { room_id, from: _, error }) = action.downcast_ref() {
                    if self.threads_pane_state.room_id.as_ref().is_some_and(|current| current == room_id) {
                        self.on_threads_failed(cx, error);
                    }
                }

                // When transitioning from offline to online, clear stale `Requested`/`Failed`
                // entries from per-room caches so they can be re-fetched.
                if let Some(RoomsListHeaderAction::StateUpdate(new_state)) = action.downcast_ref() {
                    if !matches!(new_state, State::Offline) {
                        if let Some(tl) = self.tl_state.as_mut() {
                            tl.media_cache.clear_all_pending_and_failed_requests();
                            tl.link_preview_cache.clear_all_pending_and_failed_requests();
                        }
                    }
                    continue;
                }

                // Handle the highlight animation for a message.
                let Some(tl) = self.tl_state.as_mut() else { continue };
                if let MessageHighlightAnimationState::Pending { item_id } = tl.message_highlight_animation_state {
                    if portal_list.smooth_scroll_reached(actions) {
                        cx.widget_action(
                            room_screen_widget_uid,
                            MessageAction::HighlightMessage(item_id),
                        );
                        tl.message_highlight_animation_state = MessageHighlightAnimationState::Off;
                        // Adjust the scrolled-to item's position to be slightly beneath the top of the viewport.
                        // portal_list.set_first_id_and_scroll(portal_list.first_id(), 15.0);
                    }
                }
            }

            // In-room message search actions: open/close the pane, react to
            // query changes, and jump to a clicked result. The pane lives at
            // `ids!(search_messages_pane)` (a top-level wrapper overlay) and the
            // floating button at `ids!(timeline.search_messages_button)`.
            self.handle_search_messages_actions(cx, actions, &portal_list, &loading_pane);

            // Floating threads button click → open the threads sliding pane.
            for action in actions {
                if let ThreadsButtonAction::OpenRequested = action.as_widget_action().cast_ref() {
                    self.show_threads_pane(cx);
                    break;
                }
            }

            // Floating info button click → open the room info sliding pane
            // (desktop only — the button is hidden on mobile).
            for action in actions {
                if let InfoButtonAction::OpenRequested = action.as_widget_action().cast_ref() {
                    self.show_room_info_pane(cx, scope.data.get::<AppState>());
                    break;
                }
            }

            // Server-side search results dispatched from sliding_sync.rs.
            self.handle_search_messages_results(cx, actions);

            /*
            // close message action bar if scrolled.
            if portal_list.scrolled(actions) {
                let message_action_bar_popup = self.popup_notification(cx, ids!(message_action_bar_popup));
                message_action_bar_popup.close(cx);
            }
            */

            // Set visibility of loading message banner based of pagination logic
            self.send_pagination_request_based_on_scroll_pos(cx, actions, &portal_list);
            // Handle sending any read receipts for the current logged-in user.
            self.send_user_read_receipts_based_on_scroll_pos(cx, actions, &portal_list);

            // Handle the jump to bottom button: update its visibility, and handle clicks.
            self.jump_to_bottom_button(cx, ids!(jump_to_bottom_button)).update_from_actions(
                cx,
                &portal_list,
                actions,
            );
        }

        // Currently, a Signal event is only used to tell this widget:
        // 1. to check if the room has been loaded from the homeserver yet, or
        // 2. that its timeline events have been updated in the background.
        if let Event::Signal = event {
            if std::mem::take(&mut self.resume_timeline_on_next_signal) {
                let needs_streaming_frame = self.tl_state
                    .as_ref()
                    .is_some_and(|tl|
                        tl.streaming_messages.values().any(|state| state.needs_frame())
                    );
                if needs_streaming_frame {
                    self.streaming_next_frame = cx.new_next_frame();
                }
                self.schedule_stream_timeout(cx);
                if self.expire_approval_contexts(current_unix_time_millis()) {
                    self.redraw_timeline_list(cx);
                }
                self.schedule_approval_expiry(cx);
            }
            if let (false, Some(room_name_id), true) = (self.is_loaded, self.room_name_id.as_ref(), cx.has_global::<RoomsListRef>()) {
                let rooms_list_ref = cx.get_global::<RoomsListRef>();
                if rooms_list_ref.is_room_loaded(room_name_id.room_id()) {
                    let room_name_clone = room_name_id.clone();
                    let thread_root_event_id = self.timeline_kind.as_ref()
                        .and_then(|k| k.thread_root_event_id().cloned());
                    // This room has been loaded now, so we call `set_displayed_room()`.
                    // We first clear the `room_name_id`, otherwise that function will do nothing.
                    self.room_name_id = None;
                    self.set_displayed_room(cx, &room_name_clone, thread_root_event_id);
                } else {
                    self.all_rooms_loaded = rooms_list_ref.all_rooms_loaded();
                    return;
                }
            }

            // If this RoomScreen is waiting to show a thread timeline (not the main room timeline),
            // then we need to retry showing the timeline now (upon a Signal),
            // because the thread timeline may have been successfully created.
            if self.tl_state.is_none() && self.timeline_kind.is_some() {
                self.show_timeline(cx);
            }

            self.process_timeline_updates(cx, &portal_list, scope.data.get::<AppState>());
            if threads_sliding_pane.is_currently_shown(cx) {
                self.refresh_threads_pane(cx);
            }
            if room_info_sliding_pane.is_currently_shown(cx) {
                self.refresh_room_info_pane(cx, scope.data.get::<AppState>());
            }
            // Keep the inline "Info" tab body current as room data (members,
            // topic, etc.) arrives, mirroring the overlay pane above.
            if matches!(self.active_room_tab, RoomTab::Info) {
                self.refresh_inline_room_info(cx, scope.data.get::<AppState>());
            }

            // Ideally we would do this elsewhere on the main thread, because it's not room-specific,
            // but it doesn't hurt to do it here.
            // TODO: move this up a layer to something higher in the UI tree,
            //       and wrap it in a `if let Event::Signal` conditional.
            user_profile_cache::process_user_profile_updates(cx);
            avatar_cache::process_avatar_updates(cx);
        }

        // We only forward "interactive hit" events to the inner timeline view
        // if none of the various overlay views are visible.
        // We always forward "non-interactive hit" events to the inner timeline view.
        // We check which overlay views are visible in the order of those views' z-ordering,
        // such that the top-most views get a chance to handle the event first.
        //
        // Report / leave-confirm are now GLOBAL modals (app root); the flag is
        // maintained by app.rs. Read it so the pane still yields correctly.
        let room_info_action_modal_open = is_room_info_action_modal_open();
        let is_interactive_hit = utils::is_interactive_hit_event(event);
        let is_pane_shown: bool;
        if room_info_action_modal_open {
            is_pane_shown = true;
        }
        else if loading_pane.is_currently_shown(cx) {
            is_pane_shown = true;
            if is_interactive_hit {
                loading_pane.handle_event(cx, event, scope);
            }
        }
        else if threads_sliding_pane.is_currently_shown(cx) {
            is_pane_shown = true;
            if is_interactive_hit {
                threads_sliding_pane.handle_event(cx, event, scope);
            }
        }
        else if user_profile_sliding_pane.is_currently_shown(cx) {
            is_pane_shown = true;
            if is_interactive_hit {
                user_profile_sliding_pane.handle_event(cx, event, scope);
            }
        }
        else if room_info_sliding_pane.is_currently_shown(cx) {
            is_pane_shown = true;
            if is_interactive_hit {
                room_info_sliding_pane.handle_event(cx, event, scope);
            }
        }
        else {
            is_pane_shown = false;
        }

        // TODO: once we use the `hits()` API, should be able to remove the above conditionals
        //       about whether the loading pane or user profile pane are shown, because
        //       Makepad already delivers most events to all views regardless of visibility,
        //       so the only thing we'd need here is the conditional below.

        if room_info_action_modal_open || !is_pane_shown || !is_interactive_hit {
            let Some(room_props) = self.build_room_screen_props(cx, scope, room_screen_widget_uid) else {
                if !is_pane_shown || !is_interactive_hit {
                    return;
                }
                log!("RoomScreen handling event with no room_name_id and no tl_state, skipping room-dependent event handling");
                return;
            };
            let mut room_scope = if let Some(app_state) = scope.data.get_mut::<AppState>() {
                Scope::with_data_props(app_state, &room_props)
            } else {
                Scope::with_props(&room_props)
            };


            // Forward the event to the inner timeline view, but capture any actions it produces
            // such that we can handle the ones relevant to only THIS RoomScreen widget right here and now,
            // ensuring they are not mistakenly handled by other RoomScreen widget instances.
            let mut actions_generated_within_this_room_screen = cx.capture_actions(|cx|
                self.view.handle_event(cx, event, &mut room_scope)
            );
            // Here, we handle and remove any general actions that are relevant to only this RoomScreen.
            // Removing the handled actions ensures they are not mistakenly handled by other RoomScreen widget instances.
            actions_generated_within_this_room_screen.retain(|action| {
                if self.handle_link_clicked(cx, action, &user_profile_sliding_pane) {
                    return false;
                }

                match action
                    .as_widget_action()
                    .widget_uid_eq(room_screen_widget_uid)
                    .cast()
                {
                    AppServicePanelAction::Dismiss => {
                        self.set_app_service_actions_visible(cx, false);
                        return false;
                    }
                    AppServicePanelAction::OpenCreateBotModal => {
                        if let Some(app_state) = scope.data.get::<AppState>() {
                            if !app_state.bot_settings.enabled {
                                self.send_app_service_feedback_message(
                                    tr_key(self.app_language, "room_screen.popup.app_service.enable_before_create"),
                                );
                                self.set_app_service_actions_visible(cx, false);
                            } else if !room_props.app_service_room_bound {
                                self.send_app_service_feedback_message(
                                    tr_key(self.app_language, "room_screen.popup.app_service.bind_before_create"),
                                );
                                self.set_app_service_actions_visible(cx, false);
                            } else {
                                self.open_create_bot_modal(cx);
                            }
                        } else {
                            self.send_app_service_feedback_message(
                                tr_key(self.app_language, "room_screen.popup.app_service.state_unavailable_create"),
                            );
                            self.set_app_service_actions_visible(cx, false);
                        }
                        return false;
                    }
                    AppServicePanelAction::OpenDeleteBotModal => {
                        if let Some(app_state) = scope.data.get::<AppState>() {
                            if !app_state.bot_settings.enabled {
                                self.send_app_service_feedback_message(
                                    tr_key(self.app_language, "room_screen.popup.app_service.enable_before_delete"),
                                );
                                self.set_app_service_actions_visible(cx, false);
                            } else if !room_props.app_service_room_bound {
                                self.send_app_service_feedback_message(
                                    tr_key(self.app_language, "room_screen.popup.app_service.bind_before_delete"),
                                );
                                self.set_app_service_actions_visible(cx, false);
                            } else {
                                self.open_delete_bot_modal(cx);
                            }
                        } else {
                            self.send_app_service_feedback_message(
                                tr_key(self.app_language, "room_screen.popup.app_service.state_unavailable_delete"),
                            );
                            self.set_app_service_actions_visible(cx, false);
                        }
                        return false;
                    }
                    AppServicePanelAction::SendListBots => {
                        if let Some(app_state) = scope.data.get::<AppState>() {
                            self.send_botfather_command(
                                cx,
                                app_state,
                                "/listbots",
                                tr_key(self.app_language, "room_screen.popup.bot.sent_listbots").to_string(),
                            );
                        }
                        return false;
                    }
                    AppServicePanelAction::SendBotHelp => {
                        if let Some(app_state) = scope.data.get::<AppState>() {
                            self.send_botfather_command(
                                cx,
                                app_state,
                                "/bothelp",
                                tr_key(self.app_language, "room_screen.popup.bot.sent_bothelp").to_string(),
                            );
                        }
                        return false;
                    }
                    AppServicePanelAction::ShowBoundBots => {
                        cx.action(BotBindingModalAction::Open(
                            room_props.room_name_id.clone(),
                        ));
                        self.set_app_service_actions_visible(cx, false);
                        return false;
                    }
                    AppServicePanelAction::Unbind => {
                        if let Some(app_state) = scope.data.get::<AppState>() {
                            if !room_props.app_service_room_bound {
                                self.send_app_service_feedback_message(
                                    tr_key(self.app_language, "room_screen.popup.app_service.room_not_bound"),
                                );
                            } else {
                                match app_state
                                    .bot_settings
                                    .resolved_bot_user_id_for_room(
                                        room_props.room_name_id.room_id(),
                                        current_user_id().as_deref(),
                                    )
                                {
                                    Ok(bot_user_id) => {
                                        submit_async_request(MatrixRequest::SetRoomBotBinding {
                                            room_id: room_props.room_name_id.room_id().clone(),
                                            bound: false,
                                            bot_user_id: bot_user_id.clone(),
                                        });
                                        self.send_app_service_feedback_message(
                                            tr_fmt(self.app_language, "room_screen.popup.app_service.removing_botfather", &[
                                                ("bot_user_id", bot_user_id.as_str()),
                                            ]),
                                        );
                                    }
                                    Err(error) => {
                                        self.send_app_service_feedback_message(
                                            error,
                                        );
                                    }
                                }
                            }
                        } else {
                            self.send_app_service_feedback_message(
                                tr_key(self.app_language, "room_screen.popup.app_service.state_unavailable_unbind"),
                            );
                        }
                        self.set_app_service_actions_visible(cx, false);
                        return false;
                    }
                    _ => {}
                }

                // Handle precomputed member sort ready (from background thread).
                // Validate by Arc::ptr_eq to reject stale results from a different
                // member snapshot. The Arc is kept alive in the action to prevent ABA.
                if let Some(sort_ready) = action.downcast_ref::<crate::cpu_worker::PrecomputedMemberSortReady>() {
                    if let Some(tl) = self.tl_state.as_mut() {
                        if tl.kind == sort_ready.timeline_kind {
                            let is_same = tl.room_members.as_ref()
                                .is_some_and(|m| Arc::ptr_eq(m, &sort_ready.members_arc));
                            if is_same {
                                tl.room_members_sort = Some(sort_ready.sort.clone());
                            }
                        }
                    }
                }

                match action.downcast_ref::<CreateBotModalAction>() {
                    Some(CreateBotModalAction::Close) => {
                        self.close_create_bot_modal(cx);
                        return false;
                    }
                    Some(CreateBotModalAction::Submit(request)) => {
                        let Some(app_state) = scope.data.get::<AppState>() else {
                            self.send_app_service_feedback_message(
                                tr_key(self.app_language, "room_screen.popup.bot.state_unavailable_create_command"),
                            );
                            self.close_create_bot_modal(cx);
                            return false;
                        };
                        self.send_create_bot_command(
                            cx,
                            app_state,
                            &request.username,
                            &request.display_name,
                            request.system_prompt.as_deref(),
                        );
                        return false;
                    }
                    None => {}
                }

                match action.downcast_ref::<DeleteBotModalAction>() {
                    Some(DeleteBotModalAction::Close) => {
                        self.close_delete_bot_modal(cx);
                        return false;
                    }
                    Some(DeleteBotModalAction::Submit(request)) => {
                        let Some(app_state) = scope.data.get::<AppState>() else {
                            self.send_app_service_feedback_message(
                                tr_key(self.app_language, "room_screen.popup.bot.state_unavailable_delete_command"),
                            );
                            self.close_delete_bot_modal(cx);
                            return false;
                        };
                        self.send_delete_bot_command(cx, app_state, &request.user_id_or_localpart);
                        return false;
                    }
                    None => {}
                }

                if let MessageAction::ToggleAppServiceActions = action
                    .as_widget_action()
                    .widget_uid_eq(room_screen_widget_uid)
                    .cast()
                {
                    if room_props.timeline_kind.thread_root_event_id().is_some() {
                        self.send_app_service_feedback_message(
                            tr_key(self.app_language, "room_screen.popup.bot.main_timeline_only"),
                        );
                    } else if !room_props.app_service_enabled {
                        self.send_app_service_feedback_message(
                            tr_key(self.app_language, "room_screen.popup.bot.enable_in_settings_before_bot"),
                        );
                    } else {
                        self.toggle_app_service_actions(cx);
                    }
                    return false;
                }

                // Handle the action that requests to show the user profile sliding pane.
                if let ShowUserProfileAction::ShowUserProfile(profile_and_room_id) = action.as_widget_action().cast() {
                    let mut profile_and_room_id = profile_and_room_id;
                    let room_member = self.tl_state.as_ref()
                        .and_then(|tl| tl.room_members.as_ref())
                        .and_then(|members| members.iter().find(|member| member.user_id() == profile_and_room_id.user_id).cloned());
                    if let Some(room_member) = room_member.as_ref() {
                        if profile_and_room_id.username.is_none() {
                            profile_and_room_id.username = room_member.display_name().map(ToOwned::to_owned);
                        }
                        if !profile_and_room_id.avatar_state.has_avatar() {
                            profile_and_room_id.avatar_state = AvatarState::Known(
                                room_member.avatar_url().map(ToOwned::to_owned)
                            );
                        }
                    }
                    let can_change_room_power_levels = self.tl_state.as_ref()
                        .is_some_and(|tl| tl.user_power.can_change_room_power_levels());
                    self.show_user_profile(
                        cx,
                        &user_profile_sliding_pane,
                        UserProfilePaneInfo {
                            profile_and_room_id,
                            room_name: self.room_name_id.as_ref().map_or_else(
                                || tr_key(self.app_language, "room_screen.fallback.unnamed_room").to_string(),
                                |r| r.to_string(),
                            ),
                            room_member,
                            can_change_room_power_levels,
                        },
                    );
                }

                /*
                match action.as_widget_action().widget_uid_eq(room_screen_widget_uid).cast() {
                    MessageAction::ActionBarClose => {
                        let message_action_bar_popup = self.popup_notification(cx, ids!(message_action_bar_popup));
                        let message_action_bar = message_action_bar_popup.message_action_bar(cx, ids!(message_action_bar));

                        // close only if the active message is requesting it to avoid double closes.
                        if let Some(message_widget_uid) = message_action_bar.message_widget_uid() {
                            if action.as_widget_action().widget_uid_eq(message_widget_uid).is_some() {
                                message_action_bar_popup.close(cx);
                            }
                        }
                    }
                    MessageAction::ActionBarOpen { item_id, message_rect } => {
                        let message_action_bar_popup = self.popup_notification(cx, ids!(message_action_bar_popup));
                        let message_action_bar = message_action_bar_popup.message_action_bar(cx, ids!(message_action_bar));

                        let margin_x = 50.;

                        let coords = dvec2(
                            (message_rect.pos.x + message_rect.size.x) - margin_x,
                            message_rect.pos.y,
                        );

                        script_apply_eval!(cx, message_action_bar_popup, {
                            content +: { margin +: { left: #(coords.x), top: #(coords.y) } }
                        });

                        if let Some(message_widget_uid) = action.as_widget_action().map(|a| a.widget_uid) {
                            message_action_bar_popup.open(cx);
                            message_action_bar.initialize_with_data(cx, widget_uid, message_widget_uid, item_id);
                        }
                    }
                    _ => {}
                }
                */

                // Keep all unhandled actions so we can add them back to the global action list below.
                true
            });
            self.handle_translation_lang_popup_actions(cx, &actions_generated_within_this_room_screen);
            // Add back any unhandled actions to the global action list.
            cx.extend_actions(actions_generated_within_this_room_screen);
        }
    }


    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if !self.app_language_initialized || self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }
        // If the room isn't loaded yet, we show the restore status label only.
        if !self.is_loaded {
            let Some(room_name) = &self.room_name_id else {
                // No room selected yet, nothing to show.
                return DrawStep::done();
            };
            let mut restore_status_view = self.view.restore_status_view(cx, ids!(restore_status_view));
            restore_status_view.set_content(cx, self.all_rooms_loaded, room_name);
            return restore_status_view.draw(cx, scope);
        }
        if self.tl_state.is_none() {
            // Tl_state may not be ready after dock loading.
            // If return DrawStep::done() inside self.view.draw_walk, turtle will misalign and panic.
            return DrawStep::done();
        }


        let room_screen_widget_uid = self.widget_uid();
        let Some(room_props) = self.build_room_screen_props(cx, scope, room_screen_widget_uid) else {
            return DrawStep::done();
        };

        // On mobile, hide the floating in-timeline search + threads buttons:
        // search now lives in the RoomTopBar header and both have other entry
        // points. Keep them on desktop, which has no mobile header. NOTE: these
        // are custom widgets, so toggle via `.widget()` — `.view()` returns an
        // empty ref for non-View widgets and `set_visible` would no-op.
        let is_desktop = effective_is_desktop(cx);
        let show_top_bar = !is_desktop;

        // Drive the Robrix-owned mobile room header (RoomTopBar). Its room
        // details can change independently of layout, so refresh them whenever
        // the mobile header is active.
        if show_top_bar {
            let room_name = self.room_name_id.as_ref()
                .map(ToString::to_string)
                .unwrap_or_default();
            let member_count_text = self.room_id()
                .and_then(|rid| get_client().and_then(|c| c.get_room(rid)))
                .map(|room| room.joined_members_count())
                .filter(|&n| n > 0)
                .map(|n| format!("{n} members"))
                .unwrap_or_default();
            let is_encrypted = room_props.is_encrypted;
            self.room_top_bar(cx, ids!(room_top_bar))
                .set_room(cx, &room_name, &member_count_text, is_encrypted);
        }

        // Layout setters and `script_apply_eval!` mutate the widget tree. They
        // only need to run when the responsive mode or selected mobile tab
        // changes, not for every frame emitted by a scroll gesture.
        let layout_state = AppliedRoomLayoutState {
            is_desktop,
            active_room_tab: self.active_room_tab,
        };
        if self.applied_layout_state != Some(layout_state) {
            self.view.widget(cx, ids!(timeline.search_messages_button)).set_visible(cx, is_desktop);
            self.view.widget(cx, ids!(timeline.threads_button)).set_visible(cx, is_desktop);
            self.view.widget(cx, ids!(timeline.info_button)).set_visible(cx, is_desktop);
            self.room_top_bar(cx, ids!(room_top_bar)).set_visible(cx, show_top_bar);

            // RoomTopBar height = header(56) + tabs(40) + divider(1).
            let top_space_offset = if show_top_bar { 97.0 } else { 0.0 };
            let mut top_space = self.view(cx, ids!(top_space));
            script_apply_eval!(cx, top_space, {
                margin.top: #(top_space_offset)
            });

            let on_info_tab = show_top_bar && matches!(self.active_room_tab, RoomTab::Info);
            self.room_info_sliding_pane(cx, ids!(info_content)).set_inline(true);
            self.view.view(cx, ids!(chat_content)).set_visible(cx, !on_info_tab);
            self.view.view(cx, ids!(info_tab_body)).set_visible(cx, on_info_tab);
            self.applied_layout_state = Some(layout_state);
        }

        let has_encryption_notice = room_props.is_encrypted.is_some();
        if self.last_has_encryption_notice
            .is_some_and(|previous| previous != has_encryption_notice)
            && let Some(tl_state) = self.tl_state.as_mut()
        {
            tl_state.content_drawn_since_last_update.clear();
            tl_state.profile_drawn_since_last_update.clear();
            tl_state.small_state_event_group_index = None;
        }
        self.last_has_encryption_notice = Some(has_encryption_notice);

        let mut room_scope = if let Some(app_state) = scope.data.get_mut::<AppState>() {
            Scope::with_data_props(app_state, &room_props)
        } else {
            Scope::with_props(&room_props)
        };
        let mut action_contexts_changed = false;
        while let Some(subview) = self.view.draw_walk(cx, &mut room_scope, walk).step() {
            // Here, we only need to handle drawing the portal list.
            let portal_list_ref = subview.as_portal_list();
            let Some(mut list_ref) = portal_list_ref.borrow_mut() else { continue };
            let Some(tl_state) = self.tl_state.as_mut() else {
                return DrawStep::done();
            };

            // Set the portal list's range based on the number of timeline items.
            if tl_state.small_state_event_group_index.is_none() {
                tl_state.small_state_event_group_index = Some(
                    build_small_state_event_group_index(
                        &tl_state.items,
                        &tl_state.kind,
                        &tl_state.expanded_small_state_group_event_ids,
                        self.app_language,
                    ),
                );
            }
            let small_state_event_group_index = tl_state
                .small_state_event_group_index
                .as_ref()
                .expect("small-state group index was initialized above");
            let tl_items = &tl_state.items;
            let last_item_id = tl_items.len()
                + usize::from(self.show_app_service_actions)
                + usize::from(has_encryption_notice);

            let list = list_ref.deref_mut();
            list.set_item_range(cx, 0, last_item_id);

            while let Some(item_id) = list.next_visible_item(cx) {
                let item = {
                    if let Some(is_encrypted) = room_props.is_encrypted
                        && item_id == 0
                    {
                        let item = list.item(cx, item_id, id!(EncryptionNotice));
                        item.as_encryption_notice().set_content(
                            cx,
                            is_encrypted,
                            first_other_member_display_name(
                                tl_state.room_members.as_ref().map(|members| members.as_slice()),
                            ),
                        );
                        item.draw_all(cx, &mut room_scope);
                        continue;
                    }
                    let Some(tl_idx) = tl_idx_from_item_id(item_id, has_encryption_notice) else {
                        list.item(cx, item_id, id!(Empty));
                        continue;
                    };
                    if self.show_app_service_actions && tl_idx == tl_items.len() {
                        list.item(cx, item_id, id!(AppServicePanel))
                    } else {
                    let Some(timeline_item) = tl_items.get(tl_idx) else {
                        // This shouldn't happen (unless the timeline gets corrupted or some other weird error),
                        // but we can always safely fill the item with an empty widget that takes up no space.
                        list.item(cx, item_id, id!(Empty));
                        continue;
                    };

                    // Determine whether this item's content and profile have been drawn since the last update.
                    // Pass this state to each of the `populate_*` functions so they can attempt to re-use
                    // an item in the timeline's portallist that was previously populated, if one exists.
                    let item_drawn_status = ItemDrawnStatus {
                        content_drawn: tl_state.content_drawn_since_last_update.contains(&tl_idx),
                        profile_drawn: tl_state.profile_drawn_since_last_update.contains(&tl_idx),
                    };
                    let collapse_button_text_for_expanded_group = small_state_event_group_index
                        .by_start
                        .get(&tl_idx)
                        .and_then(|group|
                            (!group.collapsed).then_some(
                                tr_key(self.app_language, "room_screen.small_state_group.collapse"),
                            )
                        );
                    let (item, item_new_draw_status) = if let Some(group) = small_state_event_group_index.by_start.get(&tl_idx)
                        && group.collapsed
                    {
                        let item = list.item(cx, item_id, id!(SmallStateEventsSummary));
                        item.label(cx, ids!(summary_label)).set_text(
                            cx,
                            small_state_event_group_index
                                .summary_by_start
                                .get(&tl_idx)
                                .map(String::as_str)
                                .unwrap_or_default(),
                        );
                        item.button(cx, ids!(state_group_toggle_button)).set_text(
                            cx,
                            tr_key(self.app_language, "room_screen.small_state_group.expand"),
                        );
                        (item, ItemDrawnStatus::both_drawn())
                    } else if small_state_event_group_index.collapsed_hidden_indices.contains(&tl_idx) {
                        (list.item(cx, item_id, id!(Empty)), ItemDrawnStatus::both_drawn())
                    } else {
                    match timeline_item.kind() {
                        TimelineItemKind::Event(event_tl_item) => match event_tl_item.content() {
                            TimelineItemContent::MsgLike(msg_like_content) => {
                                if tl_state.kind.thread_root_event_id().is_none()
                                    && msg_like_content.thread_root.is_some()
                                {
                                    // Hide threaded replies from the main room timeline UI.
                                    (list.item(cx, item_id, id!(Empty)), ItemDrawnStatus::both_drawn())
                                } else {
                                    match &msg_like_content.kind {
                                        MsgLikeKind::Message(_)
                                        | MsgLikeKind::Sticker(_)
                                        | MsgLikeKind::Redacted => {
                                            let prev_event = tl_idx.checked_sub(1).and_then(|i| tl_items.get(i));
                                            let (item, drawn_status, contexts_rebound) = populate_message_view(
                                                cx,
                                                list,
                                                item_id,
                                                &tl_state.kind,
                                                self.app_language,
                                                event_tl_item,
                                                msg_like_content,
                                                prev_event,
                                                &mut tl_state.media_cache,
                                                &mut tl_state.link_preview_cache,
                                                &tl_state.fetched_thread_summaries,
                                                &mut tl_state.pending_thread_summary_fetches,
                                                &tl_state.user_power,
                                                &self.pinned_events,
                                                &tl_state.pending_downloads,
                                                item_drawn_status,
                                                room_screen_widget_uid,
                                                room_props.resolved_parent_bot_user_id.as_deref(),
                                                &room_props.room_bot_user_ids,
                                                &room_props.known_bot_user_ids,
                                                &mut tl_state.streaming_messages,
                                                &mut self.octos_action_button_contexts,
                                                &self.disabled_octos_action_source_event_ids,
                                                &self.selected_octos_action_by_source_event_id,
                                                &tl_state.expanded_bot_body_event_ids,
                                            );
                                            action_contexts_changed |= contexts_rebound;
                                            (item, drawn_status)
                                        },
                                        // TODO: properly implement `Poll` as a regular Message-like timeline item.
                                        MsgLikeKind::Poll(poll_state) => populate_small_state_event(
                                            cx,
                                            list,
                                            item_id,
                                            &tl_state.kind,
                                            self.app_language,
                                            event_tl_item,
                                            poll_state,
                                            item_drawn_status,
                                            None,
                                            collapse_button_text_for_expanded_group,
                                        ),
                                        MsgLikeKind::UnableToDecrypt(utd) => populate_small_state_event(
                                            cx,
                                            list,
                                            item_id,
                                            &tl_state.kind,
                                            self.app_language,
                                            event_tl_item,
                                            utd,
                                            item_drawn_status,
                                            None,
                                            collapse_button_text_for_expanded_group,
                                        ),
                                        MsgLikeKind::LiveLocation(live_loc) => populate_small_state_event(
                                            cx,
                                            list,
                                            item_id,
                                            &tl_state.kind,
                                            app_language,
                                            event_tl_item,
                                            live_loc,
                                            item_drawn_status,
                                            None,
                                            collapse_button_text_for_expanded_group,
                                        ),
                                        MsgLikeKind::Other(other) => populate_small_state_event(
                                            cx,
                                            list,
                                            item_id,
                                            &tl_state.kind,
                                            self.app_language,
                                            event_tl_item,
                                            other,
                                            item_drawn_status,
                                            None,
                                            collapse_button_text_for_expanded_group,
                                        ),
                                    }
                                }
                            },
                            TimelineItemContent::MembershipChange(membership_change) => populate_small_state_event(
                                cx,
                                list,
                                item_id,
                                &tl_state.kind,
                                self.app_language,
                                event_tl_item,
                                membership_change,
                                item_drawn_status,
                                None,
                                collapse_button_text_for_expanded_group,
                            ),
                            TimelineItemContent::ProfileChange(profile_change) => populate_small_state_event(
                                cx,
                                list,
                                item_id,
                                &tl_state.kind,
                                self.app_language,
                                event_tl_item,
                                profile_change,
                                item_drawn_status,
                                None,
                                collapse_button_text_for_expanded_group,
                            ),
                            TimelineItemContent::OtherState(other) => populate_small_state_event(
                                cx,
                                list,
                                item_id,
                                &tl_state.kind,
                                self.app_language,
                                event_tl_item,
                                other,
                                item_drawn_status,
                                None,
                                collapse_button_text_for_expanded_group,
                            ),
                            unhandled => {
                                let item = list.item(cx, item_id, id!(SmallStateEvent));
                                item.label(cx, ids!(event_row.content)).set_text(
                                    cx,
                                    &format!("{} {:?}", tr_key(self.app_language, "room_screen.unsupported.prefix"), unhandled),
                                );
                                (item, ItemDrawnStatus::both_drawn())
                            }
                        }
                        TimelineItemKind::Virtual(VirtualTimelineItem::DateDivider(millis)) => {
                            let item = list.item(cx, item_id, id!(DateDivider));
                            let text = unix_time_millis_to_datetime(*millis)
                                // format the time as a shortened date (Sat, Sept 5, 2021)
                                .map(|dt| format!("{}", dt.date_naive().format("%a %b %-d, %Y")))
                                .unwrap_or_else(|| format!("{:?}", millis));
                            item.label(cx, ids!(date)).set_text(cx, &text);
                            (item, ItemDrawnStatus::both_drawn())
                        }
                        TimelineItemKind::Virtual(VirtualTimelineItem::ReadMarker) => {
                            let item = list.item(cx, item_id, id!(ReadMarker));
                            item.label(cx, ids!(date)).set_text(
                                cx,
                                tr_key(self.app_language, "room_screen.read_marker.new_messages"),
                            );
                            (item, ItemDrawnStatus::both_drawn())
                        }
                        TimelineItemKind::Virtual(VirtualTimelineItem::TimelineStart) => {
                            let item = list.item(cx, item_id, id!(Empty));
                            (item, ItemDrawnStatus::both_drawn())
                        }
                    }
                    };

                    // Now that we've drawn the item, add its index to the set of drawn items.
                    if item_new_draw_status.content_drawn {
                        tl_state.content_drawn_since_last_update.insert(tl_idx .. tl_idx + 1);
                    }
                    if item_new_draw_status.profile_drawn {
                        tl_state.profile_drawn_since_last_update.insert(tl_idx .. tl_idx + 1);
                    }
                    item
                    }
                };
                item.draw_all(cx, &mut room_scope);
            }

            // If the list is not filling the viewport, we need to back paginate the timeline
            // until we have enough events items to fill the viewport.
            if tl_state.kind.thread_root_event_id().is_none()
                && !tl_state.fully_paginated
                && !tl_state.backwards_pagination_in_flight
                && !list.is_filling_viewport()
            {
                tl_state.backwards_pagination_in_flight = true;
                log!("Automatically paginating timeline to fill viewport for room {:?}", self.room_name_id);
                submit_async_request(MatrixRequest::PaginateTimeline {
                    timeline_kind: tl_state.kind.clone(),
                    num_events: VIEWPORT_FILL_PAGINATION_SIZE,
                    direction: PaginationDirection::Backwards,
                });
            }
        }
        let previous_context_count = self.octos_action_button_contexts.len();
        let timeline_list = self.portal_list(cx, ids!(timeline.list));
        self.octos_action_button_contexts
            .retain(|_, context| {
                timeline_list
                    .get_item(context.item_id)
                    .is_some_and(|(_, item)| item.widget_uid() == context.item_widget_uid)
            });
        action_contexts_changed |=
            previous_context_count != self.octos_action_button_contexts.len();
        if action_contexts_changed {
            self.schedule_approval_expiry(cx);
        }
        DrawStep::done()
    }
}

impl RoomScreen {
    fn set_app_language(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        self.app_language = app_language;
        self.app_language_initialized = true;
        if let Some(tl_state) = self.tl_state.as_mut() {
            tl_state.content_drawn_since_last_update.clear();
            tl_state.profile_drawn_since_last_update.clear();
            tl_state.small_state_event_group_index = None;
        }
        self.sync_app_language(cx);
    }

    fn sync_app_language(&mut self, cx: &mut Cx) {
        self.view
            .label(cx, ids!(top_space.label))
            .set_text(cx, tr_key(self.app_language, "room_screen.top_space.loading_earlier"));
        self.view
            .room_input_bar(cx, ids!(room_input_bar))
            .set_app_language(cx, self.app_language);
        self.sync_translation_lang_popup(cx);
        self.view.redraw(cx);
    }

    fn redraw_timeline_list(&self, cx: &mut Cx) {
        let portal_list = self.portal_list(cx, ids!(timeline.list));
        if let Some(mut list) = portal_list.borrow_mut() {
            list.redraw(cx);
        }
    }

    fn invalidate_timeline_event_content(&mut self, event_id: &EventId) -> bool {
        let Some(tl_state) = self.tl_state.as_mut() else { return false };
        let Some(index) = tl_state.items.iter().position(|item| {
            item.as_event()
                .and_then(EventTimelineItem::event_id)
                .is_some_and(|candidate| candidate == event_id)
        }) else {
            return false;
        };
        tl_state.content_drawn_since_last_update.remove(index .. index + 1);
        true
    }

    fn toggle_small_state_event_group(&mut self, cx: &mut Cx, group_start_index: usize) {
        let Some(tl_state) = self.tl_state.as_mut() else {
            log!("[encryption-notice/toggle] tl_state is None, aborting");
            return;
        };
        let groups = compute_small_state_event_groups(
            &tl_state.items,
            &tl_state.kind,
            &tl_state.expanded_small_state_group_event_ids,
        );
        let group_starts: Vec<usize> = groups.iter().map(|g| g.start).collect();
        let Some(group) = groups.into_iter().find(|group| group.start == group_start_index) else {
            log!(
                "[encryption-notice/toggle] FIND FAILED: looking for group.start={group_start_index}, available group.starts={group_starts:?}"
            );
            return;
        };

        log!(
            "[encryption-notice/toggle] FIND OK: group.start={}, group.end={}, group.collapsed={}",
            group.start, group.end, group.collapsed
        );
        if group.collapsed {
            tl_state.expanded_small_state_group_event_ids.insert(group.first_event_id);
        } else {
            tl_state.expanded_small_state_group_event_ids.remove(&group.first_event_id);
        }
        tl_state.small_state_event_group_index = None;
        tl_state.content_drawn_since_last_update.remove(group.start .. group.end);
        tl_state.profile_drawn_since_last_update.remove(group.start .. group.end);
        self.redraw_timeline_list(cx);
        log!("[encryption-notice/toggle] state mutated, redraw_timeline_list called");
    }

    /// Folds/unfolds the long bot reply at `tl_idx`, keyed by its event ID so the
    /// choice survives PortalList recycling. Invalidates that item's content-draw
    /// cache so the body re-populates at its new length.
    /// Retries or discards the parked send at `tl_idx`.
    ///
    /// Discard reuses `RedactMessage`: for a local echo the SDK's `redact`
    /// aborts the queued send rather than sending a redaction, so no separate
    /// request is needed.
    fn act_on_failed_send(&mut self, _cx: &mut Cx, tl_idx: usize, retry: bool) {
        let Some(tl_state) = self.tl_state.as_ref() else { return };
        let Some(event) = tl_state.items.get(tl_idx).and_then(|item| item.as_event()) else {
            return;
        };
        let timeline_kind = tl_state.kind.clone();
        if retry {
            let Some(transaction_id) = event.transaction_id().map(|id| id.to_owned()) else {
                log!("BUG: retry requested for an item with no transaction id.");
                return;
            };
            submit_async_request(MatrixRequest::RetrySend { timeline_kind, transaction_id });
        } else {
            submit_async_request(MatrixRequest::RedactMessage {
                timeline_kind,
                timeline_event_id: event.identifier(),
                reason: None,
            });
        }
    }

    fn toggle_bot_body_expanded(&mut self, cx: &mut Cx, tl_idx: usize) {
        let Some(tl_state) = self.tl_state.as_mut() else { return };
        let Some(event_id) = tl_state
            .items
            .get(tl_idx)
            .and_then(|item| item.as_event())
            .and_then(|ev| ev.event_id())
            .map(|id| id.to_owned())
        else {
            return;
        };
        if !tl_state.expanded_bot_body_event_ids.remove(&event_id) {
            tl_state.expanded_bot_body_event_ids.insert(event_id);
        }
        tl_state
            .content_drawn_since_last_update
            .remove(tl_idx .. tl_idx + 1);
        self.redraw_timeline_list(cx);
    }

    fn sync_translation_lang_popup(&mut self, cx: &mut Cx) {
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_en))
            .set_text(cx, &translation::language_popup_label("en"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_zh))
            .set_text(cx, &translation::language_popup_label("zh"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_zh_tw))
            .set_text(cx, &translation::language_popup_label("zh-TW"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_ja))
            .set_text(cx, &translation::language_popup_label("ja"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_ko))
            .set_text(cx, &translation::language_popup_label("ko"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_es))
            .set_text(cx, &translation::language_popup_label("es"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_fr))
            .set_text(cx, &translation::language_popup_label("fr"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_de))
            .set_text(cx, &translation::language_popup_label("de"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_ru))
            .set_text(cx, &translation::language_popup_label("ru"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_pt))
            .set_text(cx, &translation::language_popup_label("pt"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_ar))
            .set_text(cx, &translation::language_popup_label("ar"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_vi))
            .set_text(cx, &translation::language_popup_label("vi"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_th))
            .set_text(cx, &translation::language_popup_label("th"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_id))
            .set_text(cx, &translation::language_popup_label("id"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_ms))
            .set_text(cx, &translation::language_popup_label("ms"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_tr))
            .set_text(cx, &translation::language_popup_label("tr"));
        self.view
            .button(cx, ids!(translation_lang_modal.content.translation_lang_popup.translation_lang_scroll.lang_hi))
            .set_text(cx, &translation::language_popup_label("hi"));
    }

    fn timeline_bot_context(
        &mut self,
        app_state: Option<&AppState>,
        room_id: &OwnedRoomId,
        room_members: Option<&Arc<Vec<RoomMember>>>,
    ) -> TimelineBotContext {
        let Some(app_state) = app_state else {
            return TimelineBotContext::default();
        };

        let app_service_enabled = app_state.bot_settings.enabled;
        let room_is_bound = app_state.bot_settings.is_room_bound(room_id);
        let persisted_bound_bot_user_id = if app_service_enabled {
            app_state.bot_settings.bound_bot_user_id(room_id).map(ToOwned::to_owned)
        } else {
            None
        };
        let persisted_bound_bot_user_ids = if app_service_enabled {
            app_state.bot_settings.bound_bot_user_ids(room_id)
        } else {
            Vec::new()
        };
        let resolved_parent_bot_user_id = if app_service_enabled {
            app_state
                .bot_settings
                .resolved_bot_user_id(current_user_id().as_deref())
                .ok()
        } else {
            None
        };
        let known_bot_user_ids = timeline_known_bot_user_ids(app_state);

        if let Some(cached) = self.timeline_bot_context_cache.as_ref()
            && cached.matches(
                room_id,
                room_members,
                app_service_enabled,
                room_is_bound,
                persisted_bound_bot_user_id.as_ref(),
                &persisted_bound_bot_user_ids,
                resolved_parent_bot_user_id.as_ref(),
                &known_bot_user_ids,
            )
        {
            return cached.value.clone();
        }
        if self.timeline_bot_context_cache.is_some() {
            self.invalidate_timeline_bot_context();
        }

        let has_persisted_management_binding = resolved_parent_bot_user_id
            .as_ref()
            .is_some_and(|resolved_parent_bot_user_id|
                persisted_bound_bot_user_ids
                    .iter()
                    .any(|bot_user_id| bot_user_id == resolved_parent_bot_user_id)
            );
        let room_bot_user_ids = room_members
            .map(|members|
                collect_room_bot_user_ids(
                    members.as_ref(),
                    resolved_parent_bot_user_id.as_deref(),
                    &known_bot_user_ids,
                    &persisted_bound_bot_user_ids,
                )
            )
            .unwrap_or_else(|| persisted_bound_bot_user_ids.clone());
        let detected_bound_bot_user_id = if app_service_enabled {
            room_members.and_then(|members|
                detected_bot_binding_for_members(
                    app_state,
                    room_id,
                    members.as_ref(),
                )
            )
        } else {
            None
        };
        let bound_bot_user_id = persisted_bound_bot_user_id
            .clone()
            .or(detected_bound_bot_user_id);
        let value = TimelineBotContext {
            app_service_enabled,
            app_service_room_bound: bound_bot_user_id.is_some(),
            has_persisted_management_binding,
            bound_bot_user_id,
            resolved_parent_bot_user_id: resolved_parent_bot_user_id.clone(),
            persisted_bound_bot_user_ids: persisted_bound_bot_user_ids.clone(),
            room_bot_user_ids,
            known_bot_user_ids: known_bot_user_ids.clone(),
        };
        self.timeline_bot_context_cache = Some(CachedTimelineBotContext {
            room_id: room_id.clone(),
            room_members: room_members.cloned(),
            app_service_enabled,
            room_is_bound,
            persisted_bound_bot_user_id,
            persisted_bound_bot_user_ids,
            resolved_parent_bot_user_id,
            known_bot_user_ids,
            value: value.clone(),
        });
        value
    }

    fn invalidate_timeline_bot_context(&mut self) {
        self.timeline_bot_context_cache = None;
        if let Some(tl) = self.tl_state.as_mut() {
            tl.content_drawn_since_last_update.clear();
            tl.profile_drawn_since_last_update.clear();
        }
    }

    fn build_room_screen_props(
        &mut self,
        cx: &mut Cx,
        scope: &mut Scope,
        room_screen_widget_uid: WidgetUid,
    ) -> Option<RoomScreenProps> {
        if let Some(tl) = self.tl_state.as_ref() {
            let room_id = tl.kind.room_id().clone();
            let timeline_kind = tl.kind.clone();
            let room_members = tl.room_members.clone();
            let room_members_sync_pending = tl.room_members_sync_pending;
            let room_members_sort = tl.room_members_sort.clone();
            let can_invite = tl.user_power.can_invite();
            let is_direct_room = cx.get_global::<RoomsListRef>()
                .is_direct_room(&room_id)
                .unwrap_or(false);
            let is_encrypted = cx.get_global::<RoomsListRef>()
                .joined_room_is_encrypted(&room_id)
                .flatten();
            let bot_context = self.timeline_bot_context(
                scope.data.get::<AppState>(),
                &room_id,
                room_members.as_ref(),
            );

            Some(RoomScreenProps {
                room_screen_widget_uid,
                room_name_id: self.room_name_id.clone().unwrap_or_else(|| RoomNameId::empty(room_id.clone())),
                timeline_kind,
                room_members,
                is_encrypted,
                is_direct_room,
                room_bot_user_ids: bot_context.room_bot_user_ids,
                room_members_sync_pending,
                room_members_sort,
                room_avatar_url: self.room_avatar_url.clone(),
                app_service_enabled: bot_context.app_service_enabled,
                app_service_room_bound: bot_context.app_service_room_bound,
                has_persisted_management_binding: bot_context.has_persisted_management_binding,
                bound_bot_user_id: bot_context.bound_bot_user_id,
                resolved_parent_bot_user_id: bot_context.resolved_parent_bot_user_id,
                persisted_bound_bot_user_ids: bot_context.persisted_bound_bot_user_ids,
                known_bot_user_ids: bot_context.known_bot_user_ids,
                can_invite,
                pending_invited_users: self.pending_invited_users.iter().cloned().collect(),
            })
        } else {
            self.room_name_id.as_ref().map(|room_name| RoomScreenProps {
                room_screen_widget_uid,
                room_name_id: room_name.clone(),
                timeline_kind: self.timeline_kind.clone()
                    .expect("BUG: room_name_id was set but timeline_kind was missing"),
                room_members: None,
                is_encrypted: None,
                is_direct_room: false,
                room_bot_user_ids: Vec::new(),
                room_members_sort: None,
                room_members_sync_pending: false,
                room_avatar_url: None,
                app_service_enabled: false,
                app_service_room_bound: false,
                has_persisted_management_binding: false,
                bound_bot_user_id: None,
                resolved_parent_bot_user_id: None,
                persisted_bound_bot_user_ids: Vec::new(),
                known_bot_user_ids: Vec::new(),
                can_invite: false,
                pending_invited_users: Vec::new(),
            })
        }
    }

    fn room_id(&self) -> Option<&OwnedRoomId> {
        self.room_name_id.as_ref().map(|r| r.room_id())
    }

    fn current_has_encryption_notice(&self, cx: &mut Cx) -> bool {
        self.room_id()
            .and_then(|room_id|
                cx.get_global::<RoomsListRef>()
                    .joined_room_is_encrypted(room_id)
                    .flatten()
            )
            .is_some()
    }

    /// Extract the text body from a timeline item, if it's a text message.
    fn extract_message_text(item: &Arc<TimelineItem>) -> Option<String> {
        let TimelineItemKind::Event(event) = item.kind() else { return None };
        let TimelineItemContent::MsgLike(_) = event.content() else { return None };
        Some(plaintext_body_of_timeline_item(event))
    }

    fn discover_known_bot_user_ids_from_timeline_items(
        app_state: &AppState,
        timeline_items: &Vector<Arc<TimelineItem>>,
    ) -> Vec<OwnedUserId> {
        let Ok(parent_bot_user_id) = app_state
            .bot_settings
            .resolved_bot_user_id(current_user_id().as_deref())
        else {
            return Vec::new();
        };

        let default_server_name = current_user_id()
            .map(|user_id| user_id.server_name().to_owned());
        let mut discovered_bot_user_ids = Vec::<OwnedUserId>::new();
        let mut push_bot_user_id = |bot_user_id: OwnedUserId| {
            if bot_user_id.as_str() == parent_bot_user_id.as_str() {
                return;
            }
            if !discovered_bot_user_ids
                .iter()
                .any(|existing_bot_user_id| existing_bot_user_id.as_str() == bot_user_id.as_str())
            {
                discovered_bot_user_ids.push(bot_user_id);
            }
        };

        for item in timeline_items {
            let TimelineItemKind::Event(event_tl_item) = item.kind() else { continue };
            if event_tl_item.sender().as_str() != parent_bot_user_id.as_str() {
                continue;
            }
            let Some(message_text) = Self::extract_message_text(item) else { continue };
            for bot_user_id in extract_bot_user_ids_from_listbots_reply(
                &message_text,
                default_server_name.as_ref(),
            ) {
                push_bot_user_id(bot_user_id);
            }
        }

        discovered_bot_user_ids
    }

    fn schedule_stream_timeout(&mut self, cx: &mut Cx) {
        cx.stop_timer(self.streaming_timeout_timer);
        self.streaming_timeout_timer = next_stream_timeout(
            self.tl_state
                .as_ref()
                .into_iter()
                .flat_map(|tl| tl.streaming_messages.values()),
        )
        .map(|duration| cx.start_timeout(duration.as_secs_f64()))
        .unwrap_or_else(Timer::empty);
    }

    fn schedule_approval_expiry(&mut self, cx: &mut Cx) {
        let next_deadline_millis = earliest_approval_expiry_millis(
            self.octos_action_button_contexts
                .values()
                .map(|context| &context.request),
        );
        if next_deadline_millis == self.approval_expiry_deadline_millis {
            return;
        }

        cx.stop_timer(self.approval_expiry_timer);
        self.approval_expiry_deadline_millis = next_deadline_millis;
        self.approval_expiry_timer = next_deadline_millis
            .map(|deadline| {
                let timeout = Duration::from_millis(
                    deadline
                        .saturating_sub(current_unix_time_millis())
                        .max(1),
                );
                cx.start_timeout(timeout.as_secs_f64())
            })
            .unwrap_or_else(Timer::empty);
    }

    fn expire_approval_contexts(&mut self, now_millis: u64) -> bool {
        let expired_source_event_ids: HashSet<OwnedEventId> =
            self.octos_action_button_contexts
                .values()
                .filter(|context| context.request.is_expired(now_millis))
                .map(|context| context.source_event_id.clone())
                .collect();
        if expired_source_event_ids.is_empty() {
            return false;
        }

        self.octos_action_button_contexts.retain(|_, context| {
            !expired_source_event_ids.contains(&context.source_event_id)
        });
        for source_event_id in expired_source_event_ids {
            self.invalidate_timeline_event_content(source_event_id.as_ref());
        }
        true
    }

    fn set_app_service_actions_visible(&mut self, cx: &mut Cx, visible: bool) {
        self.show_app_service_actions = visible;
        self.redraw(cx);
    }

    fn toggle_app_service_actions(&mut self, cx: &mut Cx) {
        self.set_app_service_actions_visible(cx, !self.show_app_service_actions);
    }

    fn close_create_bot_modal(&self, cx: &mut Cx) {
        self.view.modal(cx, ids!(create_bot_modal)).close(cx);
    }

    fn close_delete_bot_modal(&self, cx: &mut Cx) {
        self.view.modal(cx, ids!(delete_bot_modal)).close(cx);
    }

    fn open_create_bot_modal(&mut self, cx: &mut Cx) {
        let Some(room_name_id) = self.room_name_id.clone() else {
            return;
        };
        self.set_app_service_actions_visible(cx, false);
        self.view
            .create_bot_modal(cx, ids!(create_bot_modal_inner))
            .show(cx, room_name_id);
        self.view.modal(cx, ids!(create_bot_modal)).open(cx);
    }

    fn open_delete_bot_modal(&mut self, cx: &mut Cx) {
        let Some(room_name_id) = self.room_name_id.clone() else {
            return;
        };
        self.set_app_service_actions_visible(cx, false);
        self.view
            .delete_bot_modal(cx, ids!(delete_bot_modal_inner))
            .show(cx, room_name_id);
        self.view.modal(cx, ids!(delete_bot_modal)).open(cx);
    }

    fn reset_app_service_ui(&mut self, cx: &mut Cx) {
        self.set_app_service_actions_visible(cx, false);
        self.close_create_bot_modal(cx);
        self.close_delete_bot_modal(cx);
    }

    fn resolved_app_service_bot_user_id(
        &self,
        app_state: &AppState,
        room_id: &OwnedRoomId,
    ) -> Option<OwnedUserId> {
        if let Some(bot_user_id) = app_state.bot_settings.bound_bot_user_id(room_id.as_ref()) {
            return Some(bot_user_id.to_owned());
        }

        self.tl_state
            .as_ref()
            .filter(|tl| tl.kind.room_id() == room_id)
            .and_then(|tl| tl.room_members.as_ref())
            .and_then(|members|
                detected_bot_binding_for_members(
                    app_state,
                    room_id,
                    members.as_ref(),
                )
            )
    }

    fn is_app_service_room_bound(&self, app_state: &AppState, room_id: &OwnedRoomId) -> bool {
        self.resolved_app_service_bot_user_id(app_state, room_id).is_some()
    }

    fn send_app_service_feedback_message(&self, message: impl Into<String>) {
        let Some(room_id) = self.room_id().cloned() else {
            return;
        };
        let message = format!("[App Service] {}", message.into());
        submit_async_request(MatrixRequest::SendMessage {
            timeline_kind: TimelineKind::MainRoom { room_id },
            message: RoomMessageEventContent::notice_plain(message),
            replied_to: None,
            target_user_id: None,
            explicit_room: false,
            broadcast_target_user_ids: None,
            #[cfg(feature = "tsp")]
            sign_with_tsp: false,
        });
    }

    fn send_botfather_command(
        &mut self,
        cx: &mut Cx,
        app_state: &AppState,
        command: &str,
        success_message: String,
    ) -> bool {
        let Some(timeline_kind) = self.timeline_kind.clone() else {
            return false;
        };
        if timeline_kind.thread_root_event_id().is_some() {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.bot.main_timeline_only"),
            );
            return false;
        }

        let Some(room_id) = self.room_id().cloned() else {
            return false;
        };
        if !app_state.bot_settings.enabled {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.bot.enable_before_commands"),
            );
            return false;
        }
        let bound_bot_user_id = self.resolved_app_service_bot_user_id(app_state, &room_id);
        if bound_bot_user_id.is_none() {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.bot.bind_before_commands"),
            );
            return false;
        }

        submit_async_request(MatrixRequest::SendMessage {
            timeline_kind,
            message: RoomMessageEventContent::text_plain(command),
            replied_to: None,
            target_user_id: bound_bot_user_id,
            explicit_room: false,
            broadcast_target_user_ids: None,
            #[cfg(feature = "tsp")]
            sign_with_tsp: false,
        });

        self.send_app_service_feedback_message(success_message);
        self.set_app_service_actions_visible(cx, false);
        true
    }

    fn send_create_bot_command(
        &mut self,
        cx: &mut Cx,
        app_state: &AppState,
        username: &str,
        display_name: &str,
        system_prompt: Option<&str>,
    ) {
        let Some(timeline_kind) = self.timeline_kind.clone() else {
            return;
        };
        if timeline_kind.thread_root_event_id().is_some() {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.bot.creation_main_timeline_only"),
            );
            return;
        }

        let Some(room_id) = self.room_id().cloned() else {
            return;
        };
        if !app_state.bot_settings.enabled {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.app_service.enable_before_create"),
            );
            return;
        }
        if !self.is_app_service_room_bound(app_state, &room_id) {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.app_service.bind_before_create"),
            );
            return;
        }

        let command = format_create_bot_command(username, display_name, system_prompt);
        if self.send_botfather_command(
            cx,
            app_state,
            &command,
            tr_fmt(self.app_language, "room_screen.popup.bot.sent_createbot", &[("username", username)]),
        ) {
            self.close_create_bot_modal(cx);
        }
    }

    fn send_delete_bot_command(
        &mut self,
        cx: &mut Cx,
        app_state: &AppState,
        user_id_or_localpart: &str,
    ) {
        let matrix_user_id =
            match resolve_delete_bot_user_id(user_id_or_localpart, current_user_id().as_deref(), self.app_language) {
                Ok(user_id) => user_id,
                Err(error) => {
                    self.send_app_service_feedback_message(error);
                    return;
                }
            };

        let command = format_delete_bot_command(matrix_user_id.as_ref());
        if self.send_botfather_command(
            cx,
            app_state,
            &command,
            tr_fmt(self.app_language, "room_screen.popup.bot.sent_deletebot", &[("matrix_user_id", matrix_user_id.as_str())]),
        ) {
            self.close_delete_bot_modal(cx);
        }
    }

    /// Processes all pending background updates to the currently-shown timeline.
    ///
    /// Redraws this RoomScreen view if any updates were applied.
    fn process_timeline_updates(
        &mut self,
        cx: &mut Cx,
        portal_list: &PortalListRef,
        app_state: Option<&AppState>,
    ) {
        let update_pass_started = Instant::now();
        let (room_id, room_members, has_pending_updates) = {
            let Some(tl) = self.tl_state.as_mut() else { return };
            let available_slots =
                MAX_TIMELINE_UPDATES_PER_PASS.saturating_sub(tl.pending_updates.len());
            for _ in 0..available_slots {
                if update_pass_started.elapsed() >= TIMELINE_UPDATE_TIME_BUDGET {
                    break;
                }
                let Ok(update) = tl.update_receiver.try_recv() else {
                    break;
                };
                enqueue_timeline_update(&mut tl.pending_updates, update);
            }
            (
                tl.kind.room_id().clone(),
                tl.room_members.clone(),
                !tl.pending_updates.is_empty(),
            )
        };
        if !has_pending_updates {
            return;
        }

        let bot_context =
            self.timeline_bot_context(app_state, &room_id, room_members.as_ref());
        let top_space = self.view(cx, ids!(top_space));
        let jump_to_bottom_button = self.jump_to_bottom_button(cx, ids!(jump_to_bottom_button));
        let has_encryption_notice = self.current_has_encryption_notice(cx);
        let curr_first_id = portal_list.first_id();
        let curr_first_tl_idx = tl_idx_from_item_id(curr_first_id, has_encryption_notice).unwrap_or(0);
        let ui = self.widget_uid();
        let Some(tl) = self.tl_state.as_mut() else { return };
        let (
            resolved_parent_bot_user_id,
            room_bot_user_ids,
            known_bot_user_ids,
        ) = (
            bot_context.resolved_parent_bot_user_id,
            bot_context.room_bot_user_ids,
            bot_context.known_bot_user_ids,
        );

        let mut done_loading = false;
        let mut should_continue_backwards_pagination = false;
        let mut typing_users = None;
        let mut num_updates = 0;
        while let Some(update) = tl.pending_updates.pop_front() {
            let update_is_new_items = matches!(&update, TimelineUpdate::NewItems { .. });
            num_updates += 1;
            match update {
                TimelineUpdate::FirstUpdate { initial_items } => {
                    if let Some(app_state) = app_state {
                        let discovered_bot_user_ids =
                            Self::discover_known_bot_user_ids_from_timeline_items(
                                app_state,
                                &initial_items,
                            );
                        if !discovered_bot_user_ids.is_empty() {
                            Cx::post_action(AppStateAction::KnownBotUserIdsDiscovered {
                                bot_user_ids: discovered_bot_user_ids,
                            });
                        }
                    }
                    tl.content_drawn_since_last_update.clear();
                    tl.profile_drawn_since_last_update.clear();
                    tl.fully_paginated = false;
                    // Set the portal list to the very bottom of the timeline.
                    portal_list.set_first_id_and_scroll(
                        item_id_from_tl_idx(initial_items.len().saturating_sub(1), has_encryption_notice),
                        0.0,
                    );
                    portal_list.set_tail_range(true);
                    jump_to_bottom_button.update_visibility(cx, true);

                    let previous_streaming_messages = std::mem::take(&mut tl.streaming_messages);
                    let (rebuilt_streaming_messages, should_schedule_frame) =
                        rebuild_streaming_messages_for_full_snapshot(
                            streaming_candidates_from_items(&initial_items),
                            Some(&previous_streaming_messages),
                        );

                    tl.items = initial_items;
                    prune_expanded_small_state_group_ids(
                        &tl.items,
                        &tl.kind,
                        &mut tl.expanded_small_state_group_event_ids,
                    );
                    tl.small_state_event_group_index = None;
                    tl.streaming_messages = rebuilt_streaming_messages;
                    refresh_stream_indices(
                        tl.items.iter().map(item_event_id),
                        &mut tl.streaming_messages,
                    );
                    if should_schedule_frame {
                        self.streaming_next_frame = cx.new_next_frame();
                    }
                    done_loading = true;
                }
                TimelineUpdate::NewItems {
                    new_items,
                    changed_indices,
                    is_append,
                    clear_cache,
                } => {
                    if let Some(app_state) = app_state {
                        let discovered_bot_user_ids =
                            Self::discover_known_bot_user_ids_from_timeline_items(
                                app_state,
                                &new_items,
                            );
                        if !discovered_bot_user_ids.is_empty() {
                            Cx::post_action(AppStateAction::KnownBotUserIdsDiscovered {
                                bot_user_ids: discovered_bot_user_ids,
                            });
                        }
                    }
                    if new_items.is_empty() {
                        if !tl.items.is_empty() {
                            log!("process_timeline_updates(): timeline (had {} items) was cleared for room {}", tl.items.len(), tl.kind.room_id());
                            // The matrix SDK frequently emits a *transient* `Clear` (an empty
                            // snapshot) immediately before re-pushing the rebuilt timeline --
                            // e.g. on every message send/receive in some sliding-sync setups.
                            // If we applied this empty snapshot, the portal list would render
                            // nothing for a frame or two, exposing the near-white room
                            // background as a jarring "white flash", and we'd blank the viewport
                            // before the rebuilt items arrive.
                            //
                            // Instead, keep the currently-rendered items in place and skip
                            // applying this empty snapshot entirely. We still kick off a
                            // backwards pagination so a genuinely-cleared timeline can be
                            // refilled; the follow-up rebuild (or that pagination) delivers the
                            // full item list and refreshes the UI without any blank frame.
                            should_continue_backwards_pagination = true;
                            continue;
                        }

                        // If the bottom of the timeline (the last event) is visible, then we should
                        // set the timeline to live mode.
                        // If the bottom of the timeline is *not* visible, then we should
                        // set the timeline to Focused mode.

                        // TODO: Save the event IDs of the top 3 items before we apply this update,
                        //       which indicates this timeline is in the process of being restored,
                        //       such that we can jump back to that position later after applying this update.

                        // TODO: here we need to re-build the timeline via TimelineBuilder
                        //       and set the TimelineFocus to one of the above-saved event IDs.

                        // TODO: the docs for `TimelineBuilder::with_focus()` claim that the timeline's focus mode
                        //       can be changed after creation, but I do not see any methods to actually do that.
                        //       <https://matrix-org.github.io/matrix-rust-sdk/matrix_sdk_ui/timeline/struct.TimelineBuilder.html#method.with_focus>
                        //
                        //       As such, we probably need to create a new async request enum variant
                        //       that tells the background async task to build a new timeline
                        //       (either in live mode or focused mode around one or more events)
                        //       and then replaces the existing timeline in ALL_ROOMS_INFO with the new one.
                    }

                    let prior_items_changed = clear_cache || changed_indices.start <= curr_first_tl_idx;

                    if new_items.len() == tl.items.len() {
                        // log!("process_timeline_updates(): no jump necessary for updated timeline of same length: {}", items.len());
                    }
                    else if curr_first_tl_idx > new_items.len() {
                        log!("process_timeline_updates(): jumping to bottom: curr_first_tl_idx {} is out of bounds for {} new items", curr_first_tl_idx, new_items.len());
                        portal_list.set_first_id_and_scroll(
                            item_id_from_tl_idx(new_items.len().saturating_sub(1), has_encryption_notice),
                            0.0,
                        );
                        portal_list.set_tail_range(true);
                        jump_to_bottom_button.update_visibility(cx, true);
                    }
                    // If the prior items changed, we need to find the new index of an item that was visible
                    // in the timeline viewport so that we can maintain the scroll position of that item,
                    // which ensures that the timeline doesn't jump around unexpectedly and ruin the user's experience.
                    else if let Some((curr_item_idx, new_item_idx, new_item_scroll, _event_id)) =
                        prior_items_changed.then(||
                            find_new_item_matching_current_item(cx, portal_list, curr_first_tl_idx, &tl.items, &new_items, has_encryption_notice)
                        )
                        .flatten()
                    {
                        if curr_item_idx != new_item_idx {
                            log!("process_timeline_updates(): jumping view from event index {curr_item_idx} to new index {new_item_idx}, scroll {new_item_scroll}, event ID {_event_id}");
                            portal_list.set_first_id_and_scroll(
                                item_id_from_tl_idx(new_item_idx, has_encryption_notice),
                                new_item_scroll,
                            );
                            tl.prev_first_index = Some(new_item_idx);
                            // Set scrolled_past_read_marker false when we jump to a new event
                            tl.scrolled_past_read_marker = false;
                            // Hide the tooltip when the timeline jumps, as a hover-out event won't occur.
                            cx.widget_action(ui,  RoomScreenTooltipActions::HoverOut);
                        }
                    }
                    //
                    // TODO: after an (un)ignore user event, all timelines are cleared. Handle that here.
                    //
                    else {
                        // warning!("!!! Couldn't find new event with matching ID for ANY event currently visible in the portal list");
                    }

                    // If new items were appended to the end of the timeline, show an unread messages badge on the jump to bottom button.
                    if is_append && !portal_list.is_at_end() {
                        // We only show unread message badges on the jump to bottom button for main room timelines,
                        // because the matrix SDK doesn't currently support querying unread message counts for threads.
                        if matches!(tl.kind, TimelineKind::MainRoom { .. }) {
                            // Immediately show the unread badge with no count while we fetch the actual count in the background.
                            jump_to_bottom_button.show_unread_message_badge(cx, UnreadMessageCount::Unknown);
                            submit_async_request(MatrixRequest::GetNumberUnreadMessages{
                                timeline_kind: tl.kind.clone(),
                            });
                        }
                    }

                    let start = changed_indices.start.min(new_items.len());
                    let end = changed_indices.end.min(new_items.len());
                    let mut accepted_users: Vec<OwnedUserId> = Vec::new();
                    let mut room_members_changed = false;
                    for idx in start..end {
                        let Some(new_item) = new_items.get(idx) else { continue };
                        let TimelineItemKind::Event(event_tl_item) = new_item.kind() else { continue };
                        let TimelineItemContent::MembershipChange(membership_change) = event_tl_item.content() else { continue };
                        if is_append {
                            room_members_changed = true;
                        }
                        let accepted = matches!(
                            membership_change.change(),
                            Some(MembershipChange::InvitationAccepted)
                            | Some(MembershipChange::Joined)
                        );
                        if accepted {
                            let invited_user_id = event_tl_item.sender().to_owned();
                            if self.pending_invited_users.contains(&invited_user_id) {
                                accepted_users.push(invited_user_id);
                            }
                        }
                    }
                    if room_members_changed {
                        submit_async_request(MatrixRequest::GetRoomMembers {
                            timeline_kind: tl.kind.clone(),
                            memberships: matrix_sdk::RoomMemberships::JOIN,
                            local_only: false,
                        });
                    }
                    for accepted_user in accepted_users {
                        self.pending_invited_users.remove(&accepted_user);
                        enqueue_popup_notification(
                            format!("{accepted_user} accepted the invite and joined."),
                            PopupKind::Success,
                            Some(4.0),
                        );
                    }

                    if prior_items_changed {
                        // If this RoomScreen is showing the loading pane and has an ongoing backwards pagination request,
                        // then we should update the status message in that loading pane
                        // and then continue paginating backwards until we find the target event.
                        // Note that we do this here because `clear_cache` will always be true if backwards pagination occurred.
                        let loading_pane = self.view.loading_pane(cx, ids!(loading_pane));
                        let mut loading_pane_state = loading_pane.take_state();
                        if let LoadingPaneState::BackwardsPaginateUntilEvent {
                            events_paginated, target_event_id, ..
                        } = &mut loading_pane_state {
                            *events_paginated += new_items.len().saturating_sub(tl.items.len());
                            log!("While finding target event {target_event_id}, we have now loaded {events_paginated} messages...");
                            // Here, we assume that we have not yet found the target event,
                            // so we need to continue paginating backwards.
                            // If the target event has already been found, it will be handled
                            // in the `TargetEventFound` match arm below, which will set
                            // `should_continue_backwards_pagination` to `false`.
                            // So either way, it's okay to set this to `true` here.
                            should_continue_backwards_pagination = true;
                        }
                        loading_pane.set_state(cx, loading_pane_state);
                    }

                    if clear_cache {
                        tl.content_drawn_since_last_update.clear();
                        tl.profile_drawn_since_last_update.clear();
                        tl.fully_paginated = false;
                    } else {
                        tl.content_drawn_since_last_update.remove(changed_indices.clone());
                        tl.profile_drawn_since_last_update.remove(changed_indices.clone());
                        // log!("process_timeline_updates(): changed_indices: {changed_indices:?}, items len: {}\ncontent drawn: {:#?}\nprofile drawn: {:#?}", items.len(), tl.content_drawn_since_last_update, tl.profile_drawn_since_last_update);
                    }

                    // --- MSC4357 streaming detection ---
                    if clear_cache {
                        let previous_streaming_messages = std::mem::take(&mut tl.streaming_messages);
                        let (rebuilt_streaming_messages, should_schedule_frame) =
                            rebuild_streaming_messages_for_full_snapshot(
                                streaming_candidates_from_items(&new_items),
                                Some(&previous_streaming_messages),
                            );
                        tl.streaming_messages = rebuilt_streaming_messages;
                        if should_schedule_frame {
                            self.streaming_next_frame = cx.new_next_frame();
                        }
                    } else if !new_items.is_empty() {
                        let mut should_schedule_frame = false;
                        let scan_range = streaming_scan_range(
                            clear_cache,
                            &changed_indices,
                            tl.items.len(),
                            new_items.len(),
                        );

                        let old_event_ids: HashSet<&EventId> = tl.items.iter()
                            .filter_map(|item| item_event_id(item))
                            .collect();

                        for idx in scan_range {
                            let Some(new_item) = new_items.get(idx) else { continue };
                            let TimelineItemKind::Event(new_evt) = new_item.kind() else { continue };
                            let Some(event_id) = new_evt.event_id().map(|id| id.to_owned()) else { continue };
                            let live = is_msc4357_live(new_evt);
                            let Some(new_text) = Self::extract_message_text(new_item) else { continue };
                            let render_full_target = should_render_streaming_full_snapshot(
                                &new_text,
                                new_evt.content()
                                    .as_message()
                                    .and_then(|message| match message.msgtype() {
                                        MessageType::Text(TextMessageEventContent { formatted, .. }) => formatted.as_ref(),
                                        MessageType::Notice(NoticeMessageEventContent { formatted, .. }) => formatted.as_ref(),
                                        _ => None,
                                    }),
                                is_timeline_sender_bot(
                                    new_evt.sender(),
                                    resolved_parent_bot_user_id.as_deref(),
                                    &room_bot_user_ids,
                                    &known_bot_user_ids,
                                ),
                            );

                            if let Some(state) = tl.streaming_messages.get_mut(&event_id) {
                                let should_invalidate_content = streaming_update_requires_content_invalidation(
                                    state,
                                    &new_text,
                                    live,
                                    render_full_target,
                                );
                                state.update_target(&new_text, live);
                                state.set_render_full_target(render_full_target);
                                if should_invalidate_content
                                    && let Some(idx) = state.timeline_index
                                {
                                    tl.content_drawn_since_last_update.remove(idx .. idx + 1);
                                }
                                // Schedule frame for animation OR for cleanup of just-completed state
                                should_schedule_frame |= state.needs_frame() || state.is_complete();
                                continue;
                            }

                            if live && !old_event_ids.contains(&*event_id) {
                                let mut state = StreamingAnimState::new(&new_text, true);
                                state.set_render_full_target(render_full_target);
                                should_schedule_frame |= state.needs_frame();
                                tl.streaming_messages.insert(event_id, state);
                            }
                        }

                        if should_schedule_frame {
                            self.streaming_next_frame = cx.new_next_frame();
                        }
                    }
                    // --- End streaming detection ---

                    tl.items = new_items;
                    prune_expanded_small_state_group_ids(
                        &tl.items,
                        &tl.kind,
                        &mut tl.expanded_small_state_group_event_ids,
                    );
                    tl.small_state_event_group_index = None;
                    refresh_stream_indices(
                        tl.items.iter().map(item_event_id),
                        &mut tl.streaming_messages,
                    );
                    done_loading = true;
                }
                TimelineUpdate::NewUnreadMessagesCount(unread_messages_count) => {
                    // We only show unread message badges on the jump to bottom button for main room timelines,
                    // because the matrix SDK doesn't currently support querying unread message counts for threads.
                    if matches!(tl.kind, TimelineKind::MainRoom { .. }) {
                        jump_to_bottom_button.show_unread_message_badge(cx, unread_messages_count);
                    }
                }
                TimelineUpdate::TargetEventFound { target_event_id, index } => {
                    // log!("Target event found in room {}: {target_event_id}, index: {index}", tl.kind.room_id());
                    tl.request_sender.send_if_modified(|request| {
                        request.backwards_paginate.retain(|r| &r.room_id != tl.kind.room_id());
                        // no need to notify/wake-up all receivers for a completed request
                        false
                    });

                    // sanity check: ensure the target event is in the timeline at the given `index`.
                    let item = tl.items.get(index);
                    let is_valid = item.is_some_and(|item|
                        item.as_event()
                            .is_some_and(|ev| ev.event_id() == Some(&target_event_id))
                    );
                    let loading_pane = self.view.loading_pane(cx, ids!(loading_pane));

                    // log!("TargetEventFound: is_valid? {is_valid}. room {}, event {target_event_id}, index {index} of {}\n  --> item: {item:?}", tl.kind.room_id(), tl.items.len());
                    if is_valid {
                        // We successfully found the target event, so we can close the loading pane,
                        // reset the loading panestate to `None`, and stop issuing backwards pagination requests.
                        loading_pane.set_status(cx, tr_key(self.app_language, "room_screen.loading.found_related_message"));
                        loading_pane.set_state(cx, LoadingPaneState::None);

                        // NOTE: this code was copied from the `MessageAction::JumpToRelated` handler;
                        //       we should deduplicate them at some point.
                        let speed = 50.0;
                        let item_id = item_id_from_tl_idx(index, has_encryption_notice);
                        portal_list.smooth_scroll_to(cx, item_id, speed, None, 10.0);
                        // start highlight animation.
                        tl.message_highlight_animation_state = MessageHighlightAnimationState::Pending {
                            item_id
                        };
                    }
                    else {
                        // Here, the target event was not found in the current timeline,
                        // or we found it previously but it is no longer in the timeline (or has moved),
                        // which means we encountered an error and are unable to jump to the target event.
                        error!("Target event index {index} of {} is out of bounds for room {}", tl.items.len(), tl.kind.room_id());
                        // Show this error in the loading pane, which should already be open.
                        loading_pane.set_state(cx, LoadingPaneState::Error(
                            tr_key(self.app_language, "room_screen.loading.related_message_not_found").to_string()
                        ));
                    }

                    should_continue_backwards_pagination = false;

                    // redraw now before any other items get added to the timeline list.
                    self.view.redraw(cx);
                }
                TimelineUpdate::PaginationRunning(direction) => {
                    if direction == PaginationDirection::Backwards {
                        tl.backwards_pagination_in_flight = true;
                        top_space.set_visible(cx, true);
                        done_loading = false;
                    } else {
                        error!("Unexpected PaginationRunning update in the Forwards direction");
                    }
                }
                TimelineUpdate::PaginationError { error, direction } => {
                    if direction == PaginationDirection::Backwards {
                        tl.backwards_pagination_in_flight =
                            tl.pagination_status.backwards_is_in_flight();
                    }
                    error!("Pagination error ({direction}) in {:?}: {error:?}", self.room_name_id);
                    let room_name = self.room_name_id.as_ref().map(|r| r.to_string());
                    let error_display = utils::stringify_pagination_error(
                        &error,
                        room_name
                            .as_deref()
                            .unwrap_or(tr_key(self.app_language, "room_screen.fallback.unnamed_room")),
                    );
                    let tl_kind_retry = tl.kind.clone();
                    let direction_retry = direction;
                    enqueue_notification(NotificationItem {
                        kind: PopupKind::Error,
                        title: Some("Pagination failed".into()),
                        message: error_display.clone().into(),
                        actions: vec![
                            NotificationAction::new("Retry", NotifActionStyle::Primary, move |_cx| {
                                submit_async_request(MatrixRequest::PaginateTimeline {
                                    timeline_kind: tl_kind_retry.clone(),
                                    num_events: 30,
                                    direction: direction_retry,
                                });
                            }),
                            NotificationAction::new("Copy details", NotifActionStyle::Neutral, move |cx| {
                                cx.copy_to_clipboard(&error_display);
                            }),
                        ],
                        auto_dismissal_duration: Some(10.0),
                        ..Default::default()
                    });
                    done_loading = !tl.backwards_pagination_in_flight;
                }
                TimelineUpdate::PaginationIdle { fully_paginated, direction } => {
                    if direction == PaginationDirection::Backwards {
                        tl.backwards_pagination_in_flight =
                            tl.pagination_status.backwards_is_in_flight();
                        // Don't set `done_loading` to `true` here, because we want to keep the top space visible
                        // (with the "loading" message) until the corresponding `NewItems` update is received.
                        tl.fully_paginated = fully_paginated;
                        if fully_paginated && !tl.backwards_pagination_in_flight {
                            done_loading = true;
                        } else {
                            let loading_pane = self.view.loading_pane(cx, ids!(loading_pane));
                            let loading_pane_state = loading_pane.take_state();
                            if matches!(
                                loading_pane_state,
                                LoadingPaneState::BackwardsPaginateUntilEvent { .. }
                            ) {
                                should_continue_backwards_pagination = true;
                            }
                            loading_pane.set_state(cx, loading_pane_state);
                        }
                    } else {
                        error!("Unexpected PaginationIdle update in the Forwards direction");
                    }
                }
                TimelineUpdate::EventDetailsFetched {event_id, result } => {
                    if let Err(_e) = result {
                        error!("Failed to fetch details fetched for event {event_id} in room {}. Error: {_e:?}", tl.kind.room_id());
                    }
                    // Here, to be most efficient, we could redraw only the updated event,
                    // but for now we just fall through and let the final `redraw()` call re-draw the whole timeline view.
                }
                TimelineUpdate::ThreadSummaryDetailsFetched {
                    thread_root_event_id,
                    timeline_item_index,
                    num_replies,
                    latest_reply_preview_text,
                } => {
                    tl.pending_thread_summary_fetches.remove(&thread_root_event_id);
                    tl.fetched_thread_summaries.insert(
                        thread_root_event_id.clone(),
                        FetchedThreadSummary {
                            num_replies,
                            latest_reply_preview_text,
                        },
                    );
                    let event_id_matches_at_index = tl.items
                        .get(timeline_item_index)
                        .and_then(|item| item.as_event())
                        .and_then(|ev| ev.event_id())
                        .is_some_and(|id| id == thread_root_event_id);
                    if event_id_matches_at_index {
                        tl.content_drawn_since_last_update
                            .remove(timeline_item_index .. timeline_item_index + 1);
                    } else {
                        tl.content_drawn_since_last_update.clear();
                    }
                }
                TimelineUpdate::RoomMembersSynced => {
                    tl.awaiting_post_sync_member_refresh = true;
                    submit_async_request(MatrixRequest::GetRoomMembers {
                        timeline_kind: tl.kind.clone(),
                        memberships: matrix_sdk::RoomMemberships::JOIN,
                        local_only: true,
                    });
                }
                TimelineUpdate::RoomMembersListFetched { members } => {
                    if let TimelineKind::MainRoom { room_id } = &tl.kind {
                        let member_user_ids = members
                            .iter()
                            .map(|member| member.user_id().to_owned())
                            .collect();
                        crate::home::rooms_list::enqueue_rooms_list_update(
                            crate::home::rooms_list::RoomsListUpdate::UpdateRoomMemberUserIds {
                                room_id: room_id.clone(),
                                member_user_ids,
                            }
                        );
                    }
                    let members = Arc::new(members);
                    if tl.awaiting_post_sync_member_refresh {
                        tl.room_members_sync_pending = false;
                        tl.awaiting_post_sync_member_refresh = false;
                    }
                    // Invalidate old sort before replacing members to prevent
                    // stale sort + new members mismatch (index out of bounds).
                    tl.room_members_sort = None;
                    tl.room_members = Some(Arc::clone(&members));
                    // Compute new sort in background thread
                    crate::cpu_worker::spawn_cpu_job(cx, crate::cpu_worker::CpuJob::PrecomputeMemberSort(
                        crate::cpu_worker::PrecomputeMemberSortJob {
                            timeline_kind: tl.kind.clone(),
                            members,
                        }
                    ));
                },
                TimelineUpdate::MediaFetched(request) => {
                    log!("process_timeline_updates(): media fetched for room {}", tl.kind.room_id());
                    // Set Image to image viewer modal if the media is not a thumbnail.
                    if let (MediaFormat::File, media_source) = (request.format, request.source) {
                        populate_matrix_image_modal(cx, media_source, &mut tl.media_cache);
                    }
                    // Here, to be most efficient, we could redraw only the media items in the timeline,
                    // but for now we just fall through and let the final `redraw()` call re-draw the whole timeline view.
                }
                TimelineUpdate::MessageEdited { timeline_event_item_id: timeline_event_id, result } => {
                    self.view.room_input_bar(cx, ids!(room_input_bar))
                        .handle_edit_result(cx, timeline_event_id, result);
                }
                TimelineUpdate::PinResult { result, pin, .. } => {
                    let (message, auto_dismissal_duration, kind) = match &result {
                        Ok(true) => (
                            if pin {
                                tr_key(self.app_language, "room_screen.popup.pin.pinned_success").to_string()
                            } else {
                                tr_key(self.app_language, "room_screen.popup.pin.unpinned_success").to_string()
                            },
                            Some(4.0),
                            PopupKind::Success
                        ),
                        Ok(false) => (
                            if pin {
                                tr_key(self.app_language, "room_screen.popup.pin.already_pinned").to_string()
                            } else {
                                tr_key(self.app_language, "room_screen.popup.pin.already_unpinned").to_string()
                            },
                            Some(4.0),
                            PopupKind::Info
                        ),
                        Err(e) => (
                            tr_fmt(self.app_language, if pin {
                                "room_screen.popup.pin.pin_failed"
                            } else {
                                "room_screen.popup.pin.unpin_failed"
                            }, &[("error", &e.to_string())]),
                            None,
                            PopupKind::Error
                        ),
                    };
                    enqueue_popup_notification(message, kind, auto_dismissal_duration);
                }
                TimelineUpdate::TypingUsers { users } => {
                    // This update loop should be kept tight & fast, so all we do here is
                    // save the list of typing users for future use after the loop exits.
                    // Then, we "process" it later (by turning it into a string) after the
                    // update loop has completed, which avoids unnecessary expensive work
                    // if the list of typing users gets updated many times in a row.

                    typing_users = Some(users);
                }
                TimelineUpdate::PinnedEvents(pinned_events) => {
                    self.pinned_events = pinned_events;
                    // We need to redraw any events that might have been pinned or unpinned
                    // in order to have all events properly reflect their pinned state.
                    // However, it's intractable to find exactly which events in the timeline
                    // had a change in their pinned state, so we just clear all draw caches.
                    tl.content_drawn_since_last_update.clear();
                    tl.profile_drawn_since_last_update.clear();
                }
                TimelineUpdate::UserPowerLevels(user_power_levels) => {
                    tl.user_power = user_power_levels;
                    self.view.room_input_bar(cx, ids!(room_input_bar))
                        .update_user_power_levels(cx, user_power_levels);
                    // Update the @room mention capability based on the user's power level
                    cx.action(MentionableTextInputAction::PowerLevelsUpdated {
                        room_id: tl.kind.room_id().clone(),
                        can_notify_room: user_power_levels.can_notify_room(),
                    });
                    // We need to redraw all events in order to reflect the new power levels,
                    // e.g., for the message context menu to be correctly populated.
                    tl.content_drawn_since_last_update.clear();
                    tl.profile_drawn_since_last_update.clear();
                }
                TimelineUpdate::OwnUserReadReceipt(receipt) => {
                    tl.latest_own_user_receipt = Some(receipt);
                }
                TimelineUpdate::Tombstoned(successor_room_details) => {
                    self.view.room_input_bar(cx, ids!(room_input_bar))
                        .update_tombstone_footer(cx, tl.kind.room_id(), Some(&successor_room_details));
                    tl.tombstone_info = Some(successor_room_details);
                }
                TimelineUpdate::LinkPreviewFetched => {}
                TimelineUpdate::AttachmentDownloadFinished(mxc_uri, result) => {
                    if mark_pending_download_finished(&mut tl.pending_downloads, &mxc_uri, &result) {
                        tl.content_drawn_since_last_update.clear();
                    }
                    portal_list.redraw(cx);
                }
                TimelineUpdate::AttachmentDownloadReset(mxc_uri) => {
                    if reset_pending_download(&mut tl.pending_downloads, &mxc_uri) {
                        tl.content_drawn_since_last_update.clear();
                    }
                    portal_list.redraw(cx);
                }
                TimelineUpdate::FileUploadConfirmed(file_data) => {
                    let room_input_bar = self.view.room_input_bar(cx, ids!(room_input_bar));
                    if let Some(replied_to) = room_input_bar.handle_file_upload_confirmed(cx, &file_data.name) {
                        submit_async_request(MatrixRequest::SendAttachment {
                            timeline_kind: tl.kind.clone(),
                            file_data,
                            replied_to,
                            #[cfg(feature = "tsp")]
                            sign_with_tsp: room_input_bar.is_tsp_signing_enabled(cx),
                        });
                    }
                }
                TimelineUpdate::FileUploadUpdate { current, total } => {
                    self.view.room_input_bar(cx, ids!(room_input_bar))
                        .set_upload_progress(cx, current, total);
                }
                TimelineUpdate::FileUploadAbortHandle(handle) => {
                    self.view.room_input_bar(cx, ids!(room_input_bar))
                        .set_upload_abort_handle(handle);
                }
                TimelineUpdate::FileUploadError { error, file_data, retryable } => {
                    self.view.room_input_bar(cx, ids!(room_input_bar))
                        .show_upload_error(cx, &error, file_data, retryable);
                }
                TimelineUpdate::FileUploadComplete => {
                    self.view.room_input_bar(cx, ids!(room_input_bar))
                        .hide_upload_progress(cx);
                }
            }
            let target_event_follows_snapshot = update_is_new_items
                && matches!(
                    tl.pending_updates.front(),
                    Some(TimelineUpdate::TargetEventFound { .. })
                );
            if update_pass_started.elapsed() >= TIMELINE_UPDATE_TIME_BUDGET
                && !target_event_follows_snapshot
            {
                break;
            }
        }

        let has_more_updates =
            !tl.pending_updates.is_empty() || !tl.update_receiver.is_empty();

        if should_continue_backwards_pagination {
            done_loading = false;
            tl.backwards_pagination_in_flight = true;
            submit_async_request(MatrixRequest::PaginateTimeline {
                timeline_kind: tl.kind.clone(),
                num_events: VIEWPORT_FILL_PAGINATION_SIZE,
                direction: PaginationDirection::Backwards,
            });
        }

        if done_loading {
            top_space.set_visible(cx, false);
        }

        if let Some(users) = typing_users {
            self.view
                .typing_notice(cx, ids!(typing_notice))
                .show_or_hide(cx, &users);
        }

        if num_updates > 0 {
            self.schedule_stream_timeout(cx);
            // log!("Applied {} timeline updates for room {}, redrawing with {} items...", num_updates, tl.kind.room_id(), tl.items.len());
            self.redraw(cx);
        }
        if has_more_updates {
            SignalToUI::set_ui_signal();
        }
    }


    /// Handles a link being clicked in any child widgets of this RoomScreen.
    ///
    /// Returns `true` if the given `action` was handled as a link click.
    fn handle_link_clicked(
        &mut self,
        cx: &mut Cx,
        action: &Action,
        pane: &UserProfileSlidingPaneRef,
    ) -> bool {
        // A closure that handles both MatrixToUri and MatrixUri links,
        // and returns whether the link was handled.
        let mut handle_matrix_link = |id: &MatrixId, _via: &[OwnedServerName]| -> bool {
            match id {
                MatrixId::User(user_id) => {
                    let Some(room_name_id) = self.room_name_id.as_ref() else {
                        return false;
                    };
                    let room_member = self.tl_state.as_ref()
                        .and_then(|tl| tl.room_members.as_ref())
                        .and_then(|members| members.iter().find(|member| member.user_id() == user_id).cloned());
                    let username = room_member.as_ref()
                        .and_then(|member| member.display_name().map(ToOwned::to_owned));
                    let avatar_state = room_member.as_ref()
                        .and_then(|member| member.avatar_url().map(ToOwned::to_owned))
                        .map_or(AvatarState::Unknown, |avatar_url| AvatarState::Known(Some(avatar_url)));
                    let can_change_room_power_levels = self.tl_state.as_ref()
                        .is_some_and(|tl| tl.user_power.can_change_room_power_levels());
                    // There is no synchronous way to get the user's full profile info
                    // including the details of their room membership,
                    // so we fill in with the details we *do* know currently,
                    // show the UserProfileSlidingPane, and then after that,
                    // the UserProfileSlidingPane itself will fire off
                    // an async request to get the rest of the details.
                    self.show_user_profile(
                        cx,
                        pane,
                        UserProfilePaneInfo {
                            profile_and_room_id: UserProfileAndRoomId {
                                user_profile: UserProfile {
                                    user_id: user_id.to_owned(),
                                    username,
                                    avatar_state,
                                },
                                room_id: room_name_id.room_id().clone(),
                            },
                            room_name: room_name_id.to_string(),
                            // TODO: use the extra `via` parameters
                            room_member,
                            can_change_room_power_levels,
                        },
                    );
                    true
                }
                MatrixId::Room(room_id) => {
                    if self.room_name_id.as_ref().is_some_and(|r| r.room_id() == room_id) {
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.already_viewing_room"),
                            PopupKind::Info,
                            Some(4.0),
                        );
                        return true;
                    }
                    if let Some(room_name_id) = cx.get_global::<RoomsListRef>().get_room_name(room_id) {
                        cx.action(AppStateAction::NavigateToRoom {
                            room_to_close: None,
                            destination_room: BasicRoomDetails::Name(room_name_id),
                        });
                        return true;
                    } else {
                        log!("TODO: fetch and display room preview for room {}", room_id);
                    }
                    false
                }
                MatrixId::RoomAlias(room_alias) => {
                    log!("TODO: open room alias {}", room_alias);
                    // TODO: open a room loading screen that shows a spinner
                    //       while our background async task calls Client::resolve_room_alias()
                    //       and then either jumps to the room if known, or fetches and displays
                    //       a room preview for that room.
                    false
                }
                MatrixId::Event(room_id, event_id) => {
                    log!("TODO: open event {} in room {}", event_id, room_id);
                    // TODO: this requires the same first step as the `MatrixId::Room` case above,
                    //       but then we need to call Room::event_with_context() to get the event
                    //       and its context (surrounding events ?).
                    false
                }
                _ => false,
            }
        };

        if let HtmlLinkAction::Clicked { url, .. } = action.as_widget_action().cast() {
            // Handle mxc:// links (file downloads from Matrix media server)
            if url.starts_with("mxc://") {
                let mxc_uri = OwnedMxcUri::from(url.clone());
                self.handle_mxc_file_download(cx, mxc_uri, None);
                return true;
            }

            let mut link_was_handled = false;
            if let Ok(matrix_to_uri) = MatrixToUri::parse(&url) {
                link_was_handled |= handle_matrix_link(matrix_to_uri.id(), matrix_to_uri.via());
            }
            else if let Ok(matrix_uri) = MatrixUri::parse(&url) {
                link_was_handled |= handle_matrix_link(matrix_uri.id(), matrix_uri.via());
            }

            if !link_was_handled {
                log!("Opening URL \"{}\"", url);
                if let Err(e) = robius_open::Uri::new(&url).open() {
                    error!("Failed to open URL {:?}. Error: {:?}", url, e);
                    enqueue_popup_notification(
                        tr_fmt(self.app_language, "room_screen.popup.open_url_failed", &[("url", url.as_str())]),
                        PopupKind::Error,
                        Some(10.0),
                    );
                }
            }
            true
        }
        else if let RobrixHtmlLinkAction::ClickedMatrixLink { url, matrix_id, via, .. } = action.as_widget_action().cast() {
            let link_was_handled = handle_matrix_link(&matrix_id, &via);
            if !link_was_handled {
                log!("Opening URL \"{}\"", url);
                if let Err(e) = robius_open::Uri::new(&url).open() {
                    error!("Failed to open URL {:?}. Error: {:?}", url, e);
                    enqueue_popup_notification(
                        tr_fmt(self.app_language, "room_screen.popup.open_url_failed", &[("url", url.as_str())]),
                        PopupKind::Error,
                        Some(10.0),
                    );
                }
            }
            true
        }
        else {
            false
        }
    }

    /// Handles an mxc:// file download link click.
    /// Fetches the file from the Matrix media server, saves it with a unique name,
    /// and opens it with the system default application.
    fn handle_mxc_file_download(
        &mut self,
        _cx: &mut Cx,
        mxc_uri: OwnedMxcUri,
        update_sender: Option<crossbeam_channel::Sender<TimelineUpdate>>,
    ) {
        log!("handle_mxc_file_download: mxc_uri={mxc_uri}");

        enqueue_popup_notification(
            tr_key(self.app_language, "room_screen.file.downloading").to_string(),
            PopupKind::Info,
            Some(3.0),
        );

        // Download directly using the Matrix client (bypasses MediaCache to avoid
        // header parsing issues with non-ASCII Content-Disposition headers).
        let app_language = self.app_language;
        submit_async_request(MatrixRequest::DownloadAndSaveFile {
            mxc_uri,
            app_language,
            update_sender,
        });
    }

    /// Handles image clicks in message content by opening the image viewer.
    fn handle_image_click(
        &mut self,
        cx: &mut Cx,
        mxc_uri: Option<MediaSource>,
        texture: Option<Texture>,
        item_id: usize,
    ) {
        let Some(media_source) = mxc_uri else {
            return;
        };
        let has_encryption_notice = self.current_has_encryption_notice(cx);
        let Some(tl_state) = self.tl_state.as_mut() else { return };
        let Some(tl_idx) = tl_idx_from_item_id(item_id, has_encryption_notice) else { return };
        let Some(event_tl_item) = tl_state.items.get(tl_idx).and_then(|item| item.as_event()) else { return };

        let timestamp_millis = event_tl_item.timestamp();
        let (image_name, image_file_size) = get_image_name_and_filesize(event_tl_item);
        let downloadable = Some(DownloadableAttachment {
            media_source: media_source.clone(),
            filename: image_name.clone(),
            size: (image_file_size > 0).then_some(image_file_size),
            kind: DownloadKind::Image,
        });
        cx.action(ImageViewerAction::Show(LoadState::Loading(
            texture.clone(),
            Some(ImageViewerMetaData {
                image_name,
                image_file_size,
                timestamp: unix_time_millis_to_datetime(timestamp_millis),
                avatar_parameter: Some((
                    tl_state.kind.clone(),
                    event_tl_item.clone(),
                )),
                downloadable,
            }),
        )));

        populate_matrix_image_modal(cx, media_source, &mut tl_state.media_cache);
    }

    /// Looks up the event specified by the given message details in the given timeline.
    ///
    /// This will first try an instant index-based lookup via `details.item_id`,
    /// and then fall back to searching the timeline in reverse for the `details.event_id`
    /// if the index is "stale", meaning the timeline items have changed (e.g., due to pagination)
    /// since the message context menu was opened or the `MessageAction` was received by the `RoomScreen`.
    ///
    /// We search in reverse because it is far more likely that the user is interacting
    /// with an event that is close to the end of the timeline.
    fn find_event_in_timeline<'a>(
        items: &'a Vector<Arc<TimelineItem>>,
        details: &MessageDetails,
        has_encryption_notice: bool,
    ) -> Option<&'a EventTimelineItem> {
        let target_event_id = details.event_id()?;
        let tl_idx = tl_idx_from_item_id(details.item_id, has_encryption_notice)?;
        if let Some(event) = items.get(tl_idx)
            .and_then(|item| item.as_event())
            .filter(|ev| ev.event_id().is_some_and(|id| id == target_event_id))
        {
            return Some(event);
        }
        items.iter()
            .rev()
            .take(MAX_ITEMS_TO_SEARCH_THROUGH)
            .filter_map(|item| item.as_event())
            .find(|ev| ev.event_id().is_some_and(|id| id == target_event_id))
    }

    fn forward_message_content(
        timeline_kind: &TimelineKind,
        event_tl_item: &EventTimelineItem,
    ) -> Option<ForwardMessageContent> {
        let message = latest_effective_event_content_json(event_tl_item)
            .and_then(forwardable_room_message_content_from_json)?;
        Some(ForwardMessageContent {
            source_room_id: timeline_kind.room_id().clone(),
            source_event_id: event_tl_item.event_id()?.to_owned(),
            message,
        })
    }

    /// Handles any [`MessageAction`]s received by this RoomScreen.
    fn handle_message_actions(
        &mut self,
        cx: &mut Cx,
        actions: &ActionsBuf,
        portal_list: &PortalListRef,
        loading_pane: &LoadingPaneRef,
        scope: &mut Scope,
    ) {
        if let Some(clicked_context) = self.octos_action_button_contexts
            .iter()
            .find_map(|(widget_uid, context)| {
                actions.find_widget_action(*widget_uid)
                    .and_then(|item| matches!(item.cast(), ButtonAction::Clicked(_)).then(|| context.clone()))
            })
        {
            if clicked_context.request.is_expired(current_unix_time_millis()) {
                mark_action_buttons_disabled(
                    &mut self.disabled_octos_action_source_event_ids,
                    &clicked_context.source_event_id,
                );
                self.invalidate_timeline_event_content(
                    clicked_context.source_event_id.as_ref(),
                );
                self.redraw_timeline_list(cx);
                enqueue_popup_notification(
                    tr_key(self.app_language, "room_screen.popup.approval_expired"),
                    PopupKind::Error,
                    Some(5.0),
                );
                return;
            }
            if !are_action_buttons_disabled(
                &self.disabled_octos_action_source_event_ids,
                clicked_context.source_event_id.as_ref(),
            ) {
                let Some(tl) = self.tl_state.as_ref() else { return };
                let request = match &clicked_context.request {
                    OctosActionButtonRequest::Generic { action_id, label, .. } => build_octos_action_response_request(
                        &tl.kind,
                        label,
                        action_id,
                        clicked_context.source_event_id.as_ref(),
                        clicked_context.original_sender.as_ref(),
                    ),
                    OctosActionButtonRequest::Approval { protocol, request_id, title, decision, label, tool_args_digest, .. } => {
                        match protocol {
                            ApprovalProtocol::Octos => build_octos_approval_response_request(
                                &tl.kind,
                                title,
                                request_id,
                                decision,
                                tool_args_digest,
                                clicked_context.source_event_id.as_ref(),
                                clicked_context.original_sender.as_ref(),
                            ),
                            ApprovalProtocol::AgentChat { agent, project, project_room_id } => {
                                build_agentchat_approval_verdict_request(
                                    &tl.kind,
                                    label,
                                    request_id,
                                    decision,
                                    tool_args_digest,
                                    agent,
                                    project,
                                    project_room_id,
                                    clicked_context.source_event_id.as_ref(),
                                    clicked_context.original_sender.as_ref(),
                                )
                            }
                        }
                    }
                };
                mark_action_buttons_disabled(
                    &mut self.disabled_octos_action_source_event_ids,
                    &clicked_context.source_event_id,
                );
                mark_selected_octos_action(
                    &mut self.selected_octos_action_by_source_event_id,
                    &clicked_context.source_event_id,
                    clicked_context.request.action_id(),
                    clicked_context.request.label(),
                    clicked_context.request.style(),
                );
                self.invalidate_timeline_event_content(
                    clicked_context.source_event_id.as_ref(),
                );
                self.redraw_timeline_list(cx);
                submit_async_request(MatrixRequest::SendActionResponse {
                    timeline_kind: request.timeline_kind,
                    content: request.content,
                    target_user_id: request.target_user_id,
                    explicit_room: request.explicit_room,
                    source_event_id: request.source_event_id,
                });
            }
            return;
        }

        let room_screen_widget_uid = self.widget_uid();
        let has_encryption_notice = self.current_has_encryption_notice(cx);
        for action in actions {
            match action.as_widget_action().widget_uid_eq(room_screen_widget_uid).cast_ref() {
                MessageAction::React { details, reaction } => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    submit_async_request(MatrixRequest::ToggleReaction {
                        timeline_kind: tl.kind.clone(),
                        timeline_event_id: details.timeline_event_id.clone(),
                        reaction: reaction.clone(),
                    });
                }
                MessageAction::Reply(details) => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    if let Some(event_tl_item) = Self::find_event_in_timeline(&tl.items, details, has_encryption_notice).cloned() {
                        let replied_to_info = EmbeddedEvent::from_timeline_item(&event_tl_item);
                        self.view.room_input_bar(cx, ids!(room_input_bar))
                            .show_replying_to(cx, (event_tl_item, replied_to_info), &tl.kind);
                    }
                    else {
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.reply_not_found"),
                            PopupKind::Error,
                            Some(5.0),
                        );
                        error!("MessageAction::Reply: couldn't find event [{}] {:?} to reply to in room {:?}",
                            details.item_id,
                            details.timeline_event_id,
                            self.room_id(),
                        );
                    }
                }
                MessageAction::Edit(details) => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    if let Some(event_tl_item) = Self::find_event_in_timeline(&tl.items, details, has_encryption_notice) {
                        self.view.room_input_bar(cx, ids!(room_input_bar))
                            .show_editing_pane(
                                cx,
                                event_tl_item.clone(),
                                tl.kind.clone(),
                            );
                    }
                    else {
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.edit_not_found"),
                            PopupKind::Error,
                            Some(5.0),
                        );
                        error!("MessageAction::Edit: couldn't find event [{}] {:?} to edit in room {:?}",
                            details.item_id,
                            details.timeline_event_id,
                            self.room_id(),
                        );
                    }
                }
                MessageAction::EditLatest => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    if let Some(latest_sent_msg) = tl.items
                        .iter()
                        .rev()
                        .take(MAX_ITEMS_TO_SEARCH_THROUGH)
                        .find_map(|item| item.as_event().filter(|ev| ev.is_editable()).cloned())
                    {
                        self.view.room_input_bar(cx, ids!(room_input_bar))
                            .show_editing_pane(
                                cx,
                                latest_sent_msg,
                                tl.kind.clone(),
                            );
                    }
                    else {
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.no_recent_editable"),
                            PopupKind::Warning,
                            Some(5.0),
                        );
                    }
                }
                MessageAction::MessageSubmittedLocally => {
                    let Some(tl) = self.tl_state.as_ref() else { continue };
                    let last_item_idx = tl.items.len().saturating_sub(1);
                    portal_list.set_first_id_and_scroll(
                        item_id_from_tl_idx(last_item_idx, has_encryption_notice),
                        0.0,
                    );
                    portal_list.set_tail_range(true);
                    self.jump_to_bottom_button(cx, ids!(jump_to_bottom_button))
                        .update_visibility(cx, true);
                    self.redraw(cx);
                }
                MessageAction::Pin(details) => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    if let Some(event_id) = details.event_id() {
                        submit_async_request(MatrixRequest::PinEvent {
                            timeline_kind: tl.kind.clone(),
                            event_id: event_id.clone(),
                            pin: true,
                        });
                    } else {
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.cannot_pin"),
                            PopupKind::Error,
                            Some(5.0),
                        );
                    }
                }
                MessageAction::Unpin(details) => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    if let Some(event_id) = details.event_id() {
                        submit_async_request(MatrixRequest::PinEvent {
                            timeline_kind: tl.kind.clone(),
                            event_id: event_id.clone(),
                            pin: false,
                        });
                    } else {
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.cannot_unpin"),
                            PopupKind::Error,
                            Some(5.0),
                        );
                    }
                }
                MessageAction::CopyText(details) => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    if let Some(event_tl_item) = Self::find_event_in_timeline(&tl.items, details, has_encryption_notice) {
                        // Mirror the timeline's bot detection so the clipboard
                        // matches what the bubble displays (scaffolding stripped).
                        let (resolved_parent_bot_user_id, room_bot_user_ids, known_bot_user_ids) =
                            compute_timeline_bot_context(
                                scope.data.get::<AppState>(),
                                tl.kind.room_id(),
                                tl.room_members.as_ref(),
                            );
                        let sender_is_bot = is_timeline_sender_bot(
                            event_tl_item.sender(),
                            resolved_parent_bot_user_id.as_deref(),
                            &room_bot_user_ids,
                            &known_bot_user_ids,
                        );
                        let copy_text = clipboard_text_for_message_body(
                            plaintext_body_of_timeline_item(event_tl_item),
                            sender_is_bot,
                        );
                        // A bot message can be pure scaffolding (e.g. a progress/
                        // metrics-only update) whose stripped body is empty —
                        // don't overwrite the clipboard or claim success then.
                        if copy_text.is_empty() {
                            enqueue_popup_notification(
                                tr_key(self.app_language, "room_screen.popup.message.copy_empty"),
                                PopupKind::Info,
                                Some(2.0),
                            );
                        } else {
                            cx.copy_to_clipboard(&copy_text);
                            enqueue_popup_notification(
                                tr_key(self.app_language, "room_screen.popup.message.copied"),
                                PopupKind::Success,
                                Some(2.0),
                            );
                        }
                    }
                    else {
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.copy_text_not_found"),
                            PopupKind::Error,
                            Some(5.0),
                        );
                        error!("MessageAction::CopyText: couldn't find event [{}] {:?} to copy text from in room {}",
                            details.item_id,
                            details.timeline_event_id,
                            tl.kind.room_id(),
                        );
                    }
                }
                MessageAction::CopyHtml(details) => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    // The logic for getting the formatted body of a message is the same
                    // as the logic used in `populate_message_view()`.
                    let mut success = false;
                    if let Some(event_tl_item) = Self::find_event_in_timeline(&tl.items, details, has_encryption_notice) {
                        if let Some(message) = event_tl_item.content().as_message() {
                            match message.msgtype() {
                                MessageType::Text(TextMessageEventContent { formatted: Some(FormattedBody { body, .. }), .. })
                                | MessageType::Notice(NoticeMessageEventContent { formatted: Some(FormattedBody { body, .. }), .. })
                                | MessageType::Emote(EmoteMessageEventContent { formatted: Some(FormattedBody { body, .. }), .. })
                                | MessageType::Image(ImageMessageEventContent { formatted: Some(FormattedBody { body, .. }), .. })
                                | MessageType::File(FileMessageEventContent { formatted: Some(FormattedBody { body, .. }), .. })
                                | MessageType::Audio(AudioMessageEventContent { formatted: Some(FormattedBody { body, .. }), .. })
                                | MessageType::Video(VideoMessageEventContent { formatted: Some(FormattedBody { body, .. }), .. })
                                | MessageType::VerificationRequest(KeyVerificationRequestEventContent { formatted: Some(FormattedBody { body, .. }), .. }) =>
                                {
                                    cx.copy_to_clipboard(body);
                                    success = true;
                                }
                                _ => {}
                            }
                        }
                    }
                    if !success {
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.copy_html_not_found"),
                            PopupKind::Error,
                            Some(5.0),
                        );
                        error!("MessageAction::CopyHtml: couldn't find event [{}] {:?} to copy HTML from in room {}",
                            details.item_id,
                            details.timeline_event_id,
                            tl.kind.room_id(),
                        );
                    }
                }
                MessageAction::CopyLink(details) => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    if let Some(event_id) = details.event_id() {
                        let matrix_to_uri = tl.kind.room_id().matrix_to_event_uri(event_id.clone());
                        cx.copy_to_clipboard(&matrix_to_uri.to_string());
                    } else {
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.copy_link_failed"),
                            PopupKind::Error,
                            Some(5.0),
                        );
                        error!("MessageAction::CopyLink: no `event_id`: [{}] {:?} in room {}",
                            details.item_id,
                            details.timeline_event_id,
                            tl.kind.room_id(),
                        );
                    }
                }
                MessageAction::Forward(details) => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    if let Some(event_tl_item) = Self::find_event_in_timeline(&tl.items, details, has_encryption_notice)
                        && let Some(content) = Self::forward_message_content(&tl.kind, event_tl_item)
                    {
                        cx.action(ForwardMessageModalAction::Open(Box::new(content)));
                    } else {
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.forward_not_found"),
                            PopupKind::Error,
                            Some(5.0),
                        );
                        error!("MessageAction::Forward: couldn't find forwardable event [{}] {:?} in room {}",
                            details.item_id,
                            details.timeline_event_id,
                            tl.kind.room_id(),
                        );
                    }
                }
                MessageAction::ViewSource(details) => {
                    let Some(tl) = self.tl_state.as_ref() else { continue };
                    let Some(event_tl_item) = Self::find_event_in_timeline(&tl.items, details, has_encryption_notice) else {
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.view_source_not_found"),
                            PopupKind::Error,
                            Some(5.0),
                        );
                        continue;
                    };
                    // Get the original JSON from the event and pretty-print it
                    let latest_json: Option<String> = event_tl_item
                        .latest_json()
                        .and_then(|raw_event| serde_json::to_value(raw_event).ok())
                        .and_then(|value| serde_json::to_string_pretty(&value).ok());

                    let event_id = event_tl_item.event_id().map(|e| e.to_owned());

                    cx.action(super::event_source_modal::EventSourceModalAction::Open {
                        room_id: tl.kind.room_id().clone(),
                        event_id,
                        latest_json,
                    });
                }
                MessageAction::JumpToRelated(details) => {
                    let Some(related_event_id) = details.related_event_id.as_ref() else {
                        error!("BUG: MessageAction::JumpToRelated had no related event ID.\n{details:#?}");
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.related_not_found"),
                            PopupKind::Error,
                            Some(5.0),
                        );
                        continue;
                    };
                    self.jump_to_event(
                        cx,
                        related_event_id,
                        Some(details.item_id),
                        portal_list,
                        loading_pane
                    );
                }
                MessageAction::JumpToEvent(event_id) => {
                    self.jump_to_event(
                        cx,
                        event_id,
                        None,
                        portal_list,
                        loading_pane
                    );
                }
                MessageAction::OpenThread(thread_root_event_id) => {
                    let Some(room_name_id) = self.room_name_id.as_ref().cloned() else {
                        error!("### ERROR: MessageAction::OpenThread: thread_root_event_id: {thread_root_event_id}, but room_name_id was None!");
                        continue
                    };
                    cx.widget_action(
                        room_screen_widget_uid, 
                        RoomsListAction::Selected(SelectedRoom::Thread {
                            room_name_id,
                            thread_root_event_id: thread_root_event_id.clone(),
                        }),
                    );
                }
                MessageAction::ShowThreadsPane => {
                    self.show_threads_pane(cx);
                }
                MessageAction::ShowRoomInfoPane => {
                    self.show_room_info_pane(cx, scope.data.get::<AppState>());
                }
                MessageAction::ToggleTranslationLangPopup { button_rect } => {
                    self.toggle_translation_lang_popup(cx, *button_rect);
                }
                MessageAction::Redact { details, reason } => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    let timeline_event_id = details.timeline_event_id.clone();
                    let timeline_kind = tl.kind.clone();
                    let reason = reason.clone();
                    let app_language = self.app_language;
                    let content = ConfirmationModalContent {
                        title_text: tr_key(app_language, "room_screen.modal.delete_message.title").into(),
                        body_text: tr_key(app_language, "room_screen.modal.delete_message.body").into(),
                        accept_button_text: Some(tr_key(app_language, "room_screen.modal.delete_message.accept").into()),
                        on_accept_clicked: Some(Box::new(move |_cx| {
                            submit_async_request(MatrixRequest::RedactMessage {
                                timeline_kind,
                                timeline_event_id,
                                reason,
                            });
                        })),
                        ..Default::default()
                    };
                    cx.action(ConfirmDeleteAction::Show(RefCell::new(Some(content))));
                }
                // MessageAction::Report(details) => {
                //     // TODO
                // }

                MessageAction::DownloadAttachment(info) => {
                    let Some(tl) = self.tl_state.as_mut() else { continue };
                    let mxc_uri = media_source_mxc(&info.media_source).clone();
                    if tl.pending_downloads.iter().any(|pending| pending.mxc == mxc_uri) {
                        continue;
                    }
                    tl.pending_downloads.push(PendingDownload {
                        mxc: mxc_uri,
                        state: PendingDownloadState::InProgress,
                    });
                    tl.content_drawn_since_last_update.clear();
                    portal_list.redraw(cx);
                    let update_sender = tl.media_cache.timeline_update_sender().cloned();
                    start_attachment_download(info.clone(), update_sender);
                }
                MessageAction::CancelDownload(mxc) => {
                    if let Some(tl) = self.tl_state.as_mut()
                        && reset_pending_download(&mut tl.pending_downloads, mxc)
                    {
                        tl.content_drawn_since_last_update.clear();
                        portal_list.redraw(cx);
                    }
                    submit_async_request(MatrixRequest::CancelDownload(mxc.clone()));
                }
                // This is handled within the Message widget itself.
                MessageAction::HighlightMessage(..) => { }
                // This is handled by the top-level App itself.
                MessageAction::OpenMessageContextMenu { .. } => { }
                // This isn't yet handled, as we need to completely redesign it.
                MessageAction::ActionBarOpen { .. } => { }
                // This isn't yet handled, as we need to completely redesign it.
                MessageAction::ActionBarClose => { }
                MessageAction::ToggleAppServiceActions => { }
                MessageAction::None => { }
            }
        }
    }

    fn toggle_translation_lang_popup(&mut self, cx: &mut Cx, button_rect: Rect) {
        let translation_lang_modal = self.view.modal(cx, ids!(translation_lang_modal));
        if translation_lang_modal.is_open() {
            translation_lang_modal.close(cx);
            return;
        }

        let room_screen_rect = self.view.area().clipped_rect(cx);
        let popup_abs_pos = compute_translation_lang_popup_abs_pos(button_rect, room_screen_rect);
        self.sync_translation_lang_popup(cx);
        log!(
            "Translation popup: button_rect={button_rect:?}, room_screen_rect={room_screen_rect:?}, popup_abs_pos={popup_abs_pos:?}"
        );
        if let Some(mut translation_lang_popup) = self
            .view
            .view(cx, ids!(translation_lang_modal.content.translation_lang_popup))
            .borrow_mut()
        {
            translation_lang_popup.walk.abs_pos = Some(popup_abs_pos);
            translation_lang_popup.walk.margin.left = 0.0;
            translation_lang_popup.walk.margin.top = 0.0;
            translation_lang_popup.walk.margin.right = 0.0;
            translation_lang_popup.walk.margin.bottom = 0.0;
        }
        translation_lang_modal.open(cx);
    }

    fn handle_translation_lang_popup_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        let translation_lang_modal = self.view.modal(cx, ids!(translation_lang_modal));
        if !translation_lang_modal.is_open() {
            return;
        }

        let lang_ids: &[(&str, &[LiveId])] = &[
            ("en", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_en)]),
            ("zh", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_zh)]),
            ("zh-TW", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_zh_tw)]),
            ("ja", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_ja)]),
            ("ko", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_ko)]),
            ("es", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_es)]),
            ("fr", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_fr)]),
            ("de", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_de)]),
            ("ru", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_ru)]),
            ("pt", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_pt)]),
            ("ar", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_ar)]),
            ("vi", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_vi)]),
            ("th", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_th)]),
            ("id", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_id)]),
            ("ms", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_ms)]),
            ("tr", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_tr)]),
            ("hi", &[live_id!(translation_lang_modal), live_id!(content), live_id!(translation_lang_popup), live_id!(translation_lang_scroll), live_id!(lang_hi)]),
        ];
        for &(code, id_path) in lang_ids {
            if self.button(cx, id_path).clicked(actions) {
                self.view.room_input_bar(cx, ids!(room_input_bar)).activate_translation_language(cx, code);
                translation_lang_modal.close(cx);
                break;
            }
        }
    }

    /// Jumps to the target event ID in this timeline by smooth scrolling to it.
    ///
    /// This function searches backwards from the given `max_tl_idx` in the timeline
    /// for the given `event_id`. If found, it smooth-scrolls the portal list to that event.
    /// If not found, it displays the loading pane and starts a background search for the event.
    fn jump_to_event(
        &mut self,
        cx: &mut Cx,
        target_event_id: &OwnedEventId,
        max_tl_idx: Option<usize>,
        portal_list: &PortalListRef,
        loading_pane: &LoadingPaneRef,
    ) {
        let has_encryption_notice = self.current_has_encryption_notice(cx);
        let Some(tl) = self.tl_state.as_mut() else { return };
        let max_tl_idx = max_tl_idx
            .and_then(|item_id| tl_idx_from_item_id(item_id, has_encryption_notice))
            .unwrap_or_else(|| tl.items.len());

        // Attempt to find the index of replied-to message in the timeline.
        // Start from the current item's index (`tl_idx`) and search backwards,
        // since we know the related message must come before the current item.
        let mut num_items_searched = 0;
        let related_msg_tl_index = tl.items
            .focus()
            .narrow(..max_tl_idx)
            .into_iter()
            .rev()
            .take(MAX_ITEMS_TO_SEARCH_THROUGH)
            .position(|i| {
                num_items_searched += 1;
                i.as_event()
                    .and_then(|e| e.event_id())
                    .is_some_and(|ev_id| ev_id == target_event_id)
            })
            .map(|position| max_tl_idx.saturating_sub(position).saturating_sub(1));

        if let Some(index) = related_msg_tl_index {
            // log!("The related message {replied_to_event} was immediately found in room {}, scrolling to from index {reply_message_item_id} --> {index} (first ID {}).", tl.kind.room_id(), portal_list.first_id());
            let speed = 50.0;
            let item_id = item_id_from_tl_idx(index, has_encryption_notice);
            portal_list.smooth_scroll_to(cx, item_id, speed, None, 10.0);
            // start highlight animation.
            tl.message_highlight_animation_state = MessageHighlightAnimationState::Pending {
                item_id
            };
        } else {
            log!("The related event {target_event_id} wasn't immediately available in room {}, searching for it in the background...", tl.kind.room_id());
            // Here, we set the state of the loading pane and display it to the user.
            // The main logic will be handled in `process_timeline_updates()`, which is the only
            // place where we can receive updates to the timeline from the background tasks.
            loading_pane.set_state(
                cx,
                LoadingPaneState::BackwardsPaginateUntilEvent {
                    target_event_id: target_event_id.clone(),
                    events_paginated: 0,
                    request_sender: tl.request_sender.clone(),
                },
            );
            loading_pane.show(cx);

            tl.request_sender.send_if_modified(|request| {
                if let Some(existing) = request.backwards_paginate.iter_mut().find(|r| &r.room_id == tl.kind.room_id()) {
                    warning!("Unexpected: room {} already had an existing timeline request in progress, event: {:?}", tl.kind.room_id(), existing.target_event_id);
                    // We might as well re-use this existing request...
                    existing.target_event_id = target_event_id.clone();
                } else {
                    request.backwards_paginate.push(BackwardsPaginateUntilEventRequest {
                        room_id: tl.kind.room_id().clone(),
                        target_event_id: target_event_id.clone(),
                        // avoid re-searching through items we already searched through.
                        starting_index: max_tl_idx.saturating_sub(num_items_searched),
                        current_tl_len: tl.items.len(),
                    });
                }
                true
            });

            // Don't unconditionally start backwards pagination here, because we want to give the
            // background `timeline_subscriber_handler` task a chance to process the request first
            // and search our locally-known timeline history for the replied-to message.
        }
        self.redraw(cx);
    }

    // ============================== In-room message search ==============================

    /// Reacts to the actions emitted by the search button + sliding pane:
    ///   * `OpenRequested` → show the pane and grab key focus.
    ///   * `CloseRequested` → animate the pane out and restore the button.
    ///   * `QueryChanged` → submit a fresh `MatrixRequest::SearchMessages`
    ///     (after the pane's own debounce). An empty query aborts any
    ///     in-flight request and resets the pane to its idle state.
    ///   * `LoadMoreRequested` → submit a paginated follow-up using the
    ///     `next_batch` token stored on the room screen.
    ///   * `JumpToEvent` → call `jump_to_event` and hide the pane.
    fn handle_search_messages_actions(
        &mut self,
        cx: &mut Cx,
        actions: &Actions,
        portal_list: &PortalListRef,
        loading_pane: &LoadingPaneRef,
    ) {
        let pane = self.search_messages_sliding_pane(cx, ids!(search_messages_pane));
        let button = self.search_messages_button(cx, ids!(timeline.search_messages_button));

        let mut requested_close = false;
        let mut requested_open = false;
        let mut new_query: Option<String> = None;
        let mut load_more = false;
        let mut jump_target: Option<OwnedEventId> = None;

        for action in actions {
            // Widget-emitted actions are wrapped in a `WidgetAction`, so we
            // must unwrap via `as_widget_action()` before downcasting to the
            // inner `SearchMessagesAction`. `cast_ref` falls back to the
            // `None` sentinel for non-matching actions.
            match action.as_widget_action().cast_ref::<SearchMessagesAction>() {
                SearchMessagesAction::OpenRequested => requested_open = true,
                SearchMessagesAction::CloseRequested => requested_close = true,
                SearchMessagesAction::QueryChanged(q) => new_query = Some(q.clone()),
                SearchMessagesAction::LoadMoreRequested => load_more = true,
                SearchMessagesAction::JumpToEvent(ev) => jump_target = Some(ev.clone()),
                SearchMessagesAction::None => {}
            }
        }

        if requested_close {
            pane.hide(cx);
            button.set_visible(cx, true);
            // Abort any in-flight search so its result doesn't race the
            // pane's animate-out and re-show stale content.
            submit_async_request(MatrixRequest::SearchMessages {
                room_id: self.current_room_id_or_placeholder(),
                search_term: String::new(),
                next_batch: None,
                abort_previous: true,
            });
            self.search_state.reset();
            self.redraw(cx);
            return;
        }
        if let Some(target) = jump_target {
            pane.hide(cx);
            button.set_visible(cx, true);
            self.jump_to_event(cx, &target, None, portal_list, loading_pane);
            return;
        }
        if requested_open {
            pane.reset(cx);
            pane.show(cx);
            button.set_visible(cx, false);
            self.search_state.reset();
        }
        if let Some(query) = new_query {
            self.submit_message_search(cx, &pane, query);
        }
        if load_more {
            self.submit_message_search_next_page(cx, &pane);
        }
    }

    /// Submit a fresh server-side message search for `query`. Empty queries
    /// reset the pane to its idle state and abort any in-flight search.
    fn submit_message_search(
        &mut self,
        cx: &mut Cx,
        pane: &SearchMessagesSlidingPaneRef,
        query: String,
    ) {
        let trimmed = query.trim();
        let Some(tl) = self.tl_state.as_ref() else {
            pane.set_idle(cx);
            return;
        };
        let room_id = tl.kind.room_id().clone();

        if trimmed.is_empty() {
            // Abort whatever's running and clear the pane.
            submit_async_request(MatrixRequest::SearchMessages {
                room_id,
                search_term: String::new(),
                next_batch: None,
                abort_previous: true,
            });
            self.search_state.reset();
            pane.set_idle(cx);
            return;
        }

        // Bail out early if this room is encrypted — Matrix server-side
        // search cannot see encrypted message bodies.
        if let Some(room) = get_client().and_then(|c| c.get_room(&room_id)) {
            if room.encryption_state().is_encrypted() {
                self.search_state.reset();
                pane.set_encrypted(cx);
                return;
            }
        }

        let query_owned = trimmed.to_string();
        self.search_state = RoomSearchState {
            query: query_owned.clone(),
            room_id: Some(room_id.clone()),
            next_batch: None,
            request_in_flight: true,
        };
        pane.set_loading(cx, query_owned.clone());
        submit_async_request(MatrixRequest::SearchMessages {
            room_id,
            search_term: query_owned,
            next_batch: None,
            abort_previous: true,
        });
    }

    /// Submit a paginated follow-up for the currently-displayed search.
    /// No-op when there is no `next_batch` token, when a request is already
    /// in flight, or when the room has changed.
    fn submit_message_search_next_page(
        &mut self,
        cx: &mut Cx,
        pane: &SearchMessagesSlidingPaneRef,
    ) {
        if self.search_state.request_in_flight {
            return;
        }
        let Some(next_batch) = self.search_state.next_batch.clone() else {
            return;
        };
        let Some(state_room_id) = self.search_state.room_id.clone() else {
            return;
        };
        let Some(tl) = self.tl_state.as_ref() else { return };
        if tl.kind.room_id() != &state_room_id {
            return;
        }
        if self.search_state.query.is_empty() {
            return;
        }
        self.search_state.request_in_flight = true;
        pane.set_loading(cx, self.search_state.query.clone());
        submit_async_request(MatrixRequest::SearchMessages {
            room_id: state_room_id,
            search_term: self.search_state.query.clone(),
            next_batch: Some(next_batch),
            abort_previous: false,
        });
    }

    /// Returns the current room ID for cancel-only search requests; falls
    /// back to a placeholder (an empty `!:server`-style ID) when no room is
    /// active. The placeholder is only used by abort calls where the
    /// server-side handler short-circuits on empty `search_term` anyway.
    fn current_room_id_or_placeholder(&self) -> OwnedRoomId {
        self.tl_state
            .as_ref()
            .map(|tl| tl.kind.room_id().clone())
            .unwrap_or_else(|| matrix_sdk::ruma::owned_room_id!("!none:none.invalid"))
    }

    /// Processes results posted by the sliding_sync.rs `SearchMessages`
    /// handler. Stale results (different room or different query) are
    /// dropped; matching results are pushed into the pane.
    fn handle_search_messages_results(&mut self, cx: &mut Cx, actions: &Actions) {
        let pane = self.search_messages_sliding_pane(cx, ids!(search_messages_pane));

        for action in actions {
            let Some(result) = action.downcast_ref::<SearchMessagesResultAction>() else {
                continue;
            };
            match result {
                SearchMessagesResultAction::Received {
                    room_id,
                    search_term,
                    results,
                    next_batch,
                    total_count,
                    is_initial_page,
                } => {
                    if !self.is_search_result_current(room_id, search_term) {
                        continue;
                    }
                    self.search_state.request_in_flight = false;
                    self.search_state.next_batch = next_batch.clone();
                    let has_more = next_batch.is_some();
                    let hits: Vec<MessageSearchHit> = results
                        .iter()
                        .map(message_search_hit_from_searched_message)
                        .collect();
                    if *is_initial_page {
                        pane.set_results(cx, search_term.clone(), hits, *total_count, has_more);
                    } else {
                        pane.append_results(cx, hits, *total_count, has_more);
                    }
                }
                SearchMessagesResultAction::Failed {
                    room_id,
                    search_term,
                    error,
                    was_initial_page: _,
                } => {
                    if !self.is_search_result_current(room_id, search_term) {
                        continue;
                    }
                    self.search_state.request_in_flight = false;
                    pane.set_error(cx, error.clone());
                }
            }
        }
    }

    /// Returns true if a server search response targeting `(room_id,
    /// search_term)` should be honored — i.e. it matches what the room
    /// screen is currently displaying.
    fn is_search_result_current(&self, room_id: &OwnedRoomId, search_term: &str) -> bool {
        self.search_state.room_id.as_ref() == Some(room_id)
            && self.search_state.query == search_term
    }

    /// Shows the user profile sliding pane with the given avatar info.
    fn show_user_profile(
        &mut self,
        cx: &mut Cx,
        pane: &UserProfileSlidingPaneRef,
        info: UserProfilePaneInfo,
    ) {
        pane.set_info(cx, info);
        pane.show(cx);
        self.redraw(cx);
    }

    fn show_threads_pane(&mut self, cx: &mut Cx) {
        self.hide_room_info_pane(cx);
        self.ensure_threads_state_for_current_room();
        if !self.threads_pane_state.initialized && !self.threads_pane_state.is_loading {
            self.request_more_threads(cx, false);
        }
        self.refresh_threads_pane(cx);
        self.threads_sliding_pane(cx, ids!(threads_sliding_pane)).show(cx);
        self.threads_button(cx, ids!(timeline.threads_button)).set_visible(cx, false);
        self.redraw(cx);
    }

    fn refresh_threads_pane(&mut self, cx: &mut Cx) {
        let Some(room_name_id) = self.room_name_id.as_ref() else { return };
        self.threads_sliding_pane(cx, ids!(threads_sliding_pane)).set_info(
            cx,
            ThreadsPaneInfo {
                room_name: room_name_id.to_string(),
                entries: self.threads_pane_state.entries.iter()
                    .map(|entry| ThreadsPaneEntryInfo {
                        thread_root_event_id: entry.thread_root_event_id.clone(),
                        title: entry.title.clone(),
                        subtitle: match entry.reply_count {
                            1 => String::from("1 reply"),
                            n => format!("{n} replies"),
                        },
                        time: utils::relative_format(entry.timestamp)
                            .unwrap_or_else(|| String::from("")),
                        preview: entry.latest_reply_preview.clone().unwrap_or_else(|| String::from("Tap to open thread")),
                    })
                    .collect(),
                status_text: self.threads_pane_state.status_text.clone(),
                show_entries: !self.threads_pane_state.entries.is_empty(),
                loading_text: if self.threads_pane_state.entries.is_empty() {
                    String::from("Loading threads...")
                } else {
                    String::from("Loading more threads...")
                },
                show_loading: self.threads_pane_state.is_loading,
            },
        );
    }

    fn hide_threads_pane(&mut self, cx: &mut Cx) {
        self.threads_sliding_pane(cx, ids!(threads_sliding_pane)).hide(cx);
        let show_threads_button = effective_is_desktop(cx);
        self.threads_button(cx, ids!(timeline.threads_button))
            .set_visible(cx, show_threads_button);
    }

    /// Build the room-info payload from current state, or `None` if no room is
    /// displayed. Shared by both the sliding info pane and the inline "Info"
    /// tab body so the two presentations stay in sync.
    ///
    /// `app_state`, when available, makes the member list's "Bot" marker
    /// registry-aware (AgentRegistry ∪ app-service known bots), mirroring the
    /// timeline. Callers without a reachable `AppState` (no `Scope` in hand)
    /// pass `None`, which falls back to the name-only heuristic.
    fn build_room_info_pane_info(
        &mut self,
        app_state: Option<&AppState>,
        is_direct_room: bool,
    ) -> Option<RoomInfoPaneInfo> {
        let room_id = self.room_id().cloned()?;
        let room_name = self.room_name_id.as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| room_id.to_string());
        let room_avatar_fallback_text = self.room_name_id.as_ref()
            .and_then(|room_name_id| room_name_id.name_for_avatar().map(ToOwned::to_owned))
            .unwrap_or_else(|| String::from("?"));
        let room_avatar_uri = self.room_avatar_url.clone();
        let (topic, visibility, encryption, is_encrypted, is_favorite, joined_count) = get_client()
            .and_then(|client| client.get_room(&room_id))
            .map(|room| {
                let topic = room.topic()
                    .unwrap_or_else(|| String::from("No topic"));
                let visibility = match room.is_public() {
                    Some(true) => String::from("Public"),
                    Some(false) => String::from("Private"),
                    None => String::from("Unknown"),
                };
                let encryption_state = room.encryption_state();
                let is_encrypted = encryption_state.is_encrypted();
                let encryption = if encryption_state.is_unknown() {
                    String::from("Unknown")
                } else if is_encrypted {
                    String::from("Encrypted")
                } else {
                    String::from("Unencrypted")
                };
                // Authoritative joined count from the room summary, available
                // even before the full member list is fetched.
                let joined_count = room.joined_members_count() as usize;
                (topic, visibility, encryption, is_encrypted, room.is_favourite(), joined_count)
            })
            .unwrap_or_else(|| (
                String::from("No topic"),
                String::from("Unknown"),
                String::from("Unknown"),
                false,
                false,
                0,
            ));

        let my_user_id = current_user_id();
        let bot_identity = room_info_bot_identity_fingerprint(app_state, my_user_id.as_deref());
        // Clone the members `Arc` out first (cheap) so the `tl_state` borrow is
        // released before we (mutably) touch the cache below.
        let members_arc = self.tl_state.as_ref().and_then(|tl| tl.room_members.clone());
        let room_info_dm_target = if is_direct_room {
            members_arc.as_ref().and_then(|members|
                room_info_dm_target_from_user_ids(
                    members.iter().map(|member| member.user_id()),
                    my_user_id.as_deref(),
                )
            )
        } else {
            None
        };
        let show_title_bot_pill = room_info_title_shows_agent_badge(
            app_state,
            room_id.as_ref(),
            room_info_dm_target.as_deref(),
            members_arc.iter()
                .flat_map(|members| members.iter())
                .map(|member| member.user_id()),
        );

        let (people_entries, show_people_loading, member_count, is_agent_enabled, my_role) =
            if let Some(members) = members_arc {
                let cache_valid = self.room_info_members_cache.as_ref().is_some_and(|c|
                    c.room_id == room_id
                        && Arc::ptr_eq(&c.members, &members)
                        && c.bot_identity == bot_identity
                );
                if !cache_valid {
                    // Expensive path — only when the member list actually changed.
                    let my_role = members.iter()
                        .find(|member| my_user_id.as_deref() == Some(member.user_id()))
                        .map(|member| match member.suggested_role_for_power_level() {
                            RoomMemberRole::Creator => "Owner",
                            RoomMemberRole::Administrator => "Admin",
                            RoomMemberRole::Moderator => "Moderator",
                            RoomMemberRole::User => "Member",
                        })
                        .unwrap_or("")
                        .to_string();

                    let level_weight = |level: &str| -> u8 {
                        match level {
                            "Creator" => 0,
                            "Admin" => 1,
                            "Moderator" => 2,
                            _ => 3,
                        }
                    };

                    // Registry-aware bot detection, mirroring the timeline's
                    // `is_timeline_sender_bot`: the union of the AgentRegistry
                    // and (app-service-gated) known-bot list, plus the
                    // resolved parent BotFather MXID. Computed once here
                    // (not per member) since it's the same for every entry.
                    let known_bot_user_ids = &bot_identity.known_bot_user_ids;
                    let resolved_parent_bot_user_id =
                        bot_identity.resolved_parent_bot_user_id.as_deref();

                    // Build with a precomputed (role-weight, lowercased-name) sort
                    // key so sorting doesn't allocate a String per comparison.
                    let mut keyed: Vec<(u8, String, RoomInfoPeopleEntryInfo)> = members.iter()
                        .map(|member| {
                            let display_name = member.display_name()
                                .map(ToOwned::to_owned)
                                .unwrap_or_else(|| member.user_id().to_string());
                            let is_bot = is_known_or_likely_bot(
                                    member.user_id(),
                                    resolved_parent_bot_user_id,
                                    known_bot_user_ids,
                                ) || is_likely_bot_member(member, resolved_parent_bot_user_id);
                            let level = match member.suggested_role_for_power_level() {
                                RoomMemberRole::Creator => String::from("Creator"),
                                RoomMemberRole::Administrator => String::from("Admin"),
                                RoomMemberRole::Moderator => String::from("Moderator"),
                                RoomMemberRole::User => String::new(),
                            };
                            let avatar_fallback_text = utils::user_name_first_letter(&display_name)
                                .map(ToOwned::to_owned)
                                .unwrap_or_else(|| String::from("?"));
                            let weight = level_weight(&level);
                            let sort_name = display_name.to_lowercase();
                            (weight, sort_name, RoomInfoPeopleEntryInfo {
                                user_id: member.user_id().to_owned(),
                                display_name,
                                level,
                                is_bot,
                                avatar_uri: member.avatar_url().map(ToOwned::to_owned),
                                avatar_fallback_text,
                            })
                        })
                        .collect();
                    keyed.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
                    let entries: Vec<RoomInfoPeopleEntryInfo> =
                        keyed.into_iter().map(|(_, _, entry)| entry).collect();

                    // A room is "Agent-enabled" if any of its members is detected
                    // as a bot (mirrors `is_likely_bot_member`).
                    let is_agent_enabled = entries.iter().any(|entry| entry.is_bot);
                    let entries = Arc::new(entries);
                    self.room_info_members_cache = Some(RoomInfoMembersCache {
                        room_id: room_id.clone(),
                        members: Arc::clone(&members),
                        bot_identity: bot_identity.clone(),
                        entries,
                        is_agent_enabled,
                        my_role,
                    });
                }

                // The cache is now valid for this (room, member-list) pair; reuse
                // the prebuilt rows via a cheap `Arc` clone.
                let cache = self.room_info_members_cache.as_ref().expect("just populated");
                (
                    Arc::clone(&cache.entries),
                    false,
                    cache.entries.len(),
                    cache.is_agent_enabled,
                    cache.my_role.clone(),
                )
            } else {
                (Arc::new(Vec::new()), true, 0, false, String::new())
            };

        // Prefer the actually-loaded member-list length so the header count, the
        // avatar-stack "+N", and the People sub-page list all agree. Fall back to
        // the room-summary joined count only before the member list has loaded
        // (gives an accurate number immediately instead of a "0" flash).
        let member_count = if member_count > 0 { member_count } else { joined_count };
        let people_count_text = if show_people_loading {
            String::from("People")
        } else {
            format!("{member_count} Members")
        };

        Some(RoomInfoPaneInfo {
            room_name,
            room_id: room_id.to_string(),
            owned_room_id: room_id,
            topic,
            visibility,
            encryption,
            is_encrypted,
            is_favorite,
            is_agent_enabled,
            show_title_bot_pill,
            member_count,
            my_role,
            room_avatar_uri,
            room_avatar_fallback_text,
            people_entries,
            people_count_text,
            show_people_loading,
        })
    }

    fn refresh_room_info_pane(&mut self, cx: &mut Cx, app_state: Option<&AppState>) {
        let is_direct_room = self.current_room_is_direct(cx);
        if let Some(info) = self.build_room_info_pane_info(app_state, is_direct_room) {
            self.room_info_sliding_pane(cx, ids!(room_info_sliding_pane)).set_info(cx, info);
        }
    }

    /// Populate the inline "Info" tab body (a second `RoomInfoSlidingPane`
    /// instance mounted inline inside `keyboard_view`).
    fn refresh_inline_room_info(&mut self, cx: &mut Cx, app_state: Option<&AppState>) {
        let is_direct_room = self.current_room_is_direct(cx);
        if let Some(info) = self.build_room_info_pane_info(app_state, is_direct_room) {
            self.room_info_sliding_pane(cx, ids!(info_content)).set_info(cx, info);
        }
    }

    fn current_room_is_direct(&self, cx: &mut Cx) -> bool {
        let Some(room_id) = self.room_id() else { return false };
        if !cx.has_global::<RoomsListRef>() {
            return false;
        }
        cx.get_global::<RoomsListRef>()
            .is_direct_room(room_id)
            .unwrap_or(false)
    }

    fn show_room_info_pane(&mut self, cx: &mut Cx, app_state: Option<&AppState>) {
        self.hide_threads_pane(cx);
        self.refresh_room_info_pane(cx, app_state);
        self.room_info_sliding_pane(cx, ids!(room_info_sliding_pane)).show(cx);
        self.redraw(cx);
    }

    fn hide_room_info_pane(&mut self, cx: &mut Cx) {
        self.room_info_sliding_pane(cx, ids!(room_info_sliding_pane)).hide(cx);
    }

    fn ensure_threads_state_for_current_room(&mut self) {
        let Some(room_id) = self.room_id().cloned() else { return };
        if self.threads_pane_state.room_id.as_ref().is_some_and(|current| current == &room_id) {
            return;
        }
        self.threads_pane_state = ThreadsPaneState {
            room_id: Some(room_id),
            status_text: String::from("Loading threads..."),
            ..Default::default()
        };
    }

    fn request_more_threads(&mut self, _cx: &mut Cx, load_more: bool) {
        self.ensure_threads_state_for_current_room();
        let Some(room_id) = self.threads_pane_state.room_id.clone() else { return };
        if self.threads_pane_state.is_loading {
            return;
        }
        let from = if load_more {
            let Some(from) = self.threads_pane_state.prev_batch_token.clone() else { return };
            Some(from)
        } else {
            None
        };
        self.threads_pane_state.is_loading = true;
        if !self.threads_pane_state.initialized {
            self.threads_pane_state.status_text = String::from("Loading threads...");
        }
        submit_async_request(MatrixRequest::ListRoomThreads {
            room_id,
            from,
        });
    }

    fn on_threads_loaded(
        &mut self,
        cx: &mut Cx,
        _from: Option<&String>,
        threads: &[FetchedRoomThread],
        prev_batch_token: Option<String>,
    ) {
        self.threads_pane_state.is_loading = false;
        self.threads_pane_state.initialized = true;
        self.threads_pane_state.prev_batch_token = prev_batch_token;
        self.threads_pane_state.entries.extend_from_slice(threads);
        self.threads_pane_state.entries.sort_by_key(|entry| u64::from(entry.timestamp.0));
        self.threads_pane_state.entries.dedup_by(|a, b| a.thread_root_event_id == b.thread_root_event_id);
        self.threads_pane_state.status_text = if self.threads_pane_state.entries.is_empty() {
            String::from("No threads yet.")
        } else {
            String::new()
        };
        self.refresh_threads_pane(cx);
        self.redraw(cx);
    }

    fn on_threads_failed(&mut self, cx: &mut Cx, error: &str) {
        self.threads_pane_state.is_loading = false;
        self.threads_pane_state.initialized = true;
        if self.threads_pane_state.entries.is_empty() {
            self.threads_pane_state.status_text = format!("Failed to load threads.\n\nError: {error}");
        } else {
            let error_display = error.to_string();
            let room_id_retry = self.threads_pane_state.room_id.clone();
            let from_retry = self.threads_pane_state.prev_batch_token.clone();
            enqueue_notification(NotificationItem {
                kind: PopupKind::Error,
                title: Some("Load threads failed".into()),
                message: format!("Failed to load more threads.\n\nError: {error}").into(),
                actions: vec![
                    NotificationAction::new("Retry", NotifActionStyle::Primary, move |_cx| {
                        if let Some(room_id) = room_id_retry.clone() {
                            submit_async_request(MatrixRequest::ListRoomThreads {
                                room_id,
                                from: from_retry.clone(),
                            });
                        }
                    }),
                    NotificationAction::new("Copy details", NotifActionStyle::Neutral, move |cx| {
                        cx.copy_to_clipboard(&error_display);
                    }),
                ],
                auto_dismissal_duration: Some(5.0),
                ..Default::default()
            });
        }
        self.refresh_threads_pane(cx);
        self.redraw(cx);
    }

    /// Invoke this when this timeline is being shown,
    /// e.g., when the user navigates to this timeline.
    fn show_timeline(&mut self, cx: &mut Cx) {
        let kind = self.timeline_kind.clone()
            .expect("BUG: Timeline::show_timeline(): no timeline_kind was set.");
        let room_id = kind.room_id().clone();

        let state_opt = TIMELINE_STATES.with_borrow_mut(|ts| ts.remove(&kind));
        let (mut tl_state, mut is_first_time_being_loaded) = if let Some(existing) = state_opt {
            (existing, false)
        } else {
            let Some(timeline_endpoints) = take_timeline_endpoints(&kind) else {
                if let Some(thread_root_event_id) = kind.thread_root_event_id() {
                    submit_async_request(MatrixRequest::CreateThreadTimeline {
                        room_id: room_id.clone(),
                        thread_root_event_id: thread_root_event_id.clone(),
                    });
                    self.set_timeline_updates_enabled(true);
                    return;
                }
                if !self.is_loaded && self.all_rooms_loaded {
                    panic!("BUG: timeline {kind} is not loaded, but its RoomScreen \
                    was not waiting for its timeline to be loaded either.");
                }
                self.set_timeline_updates_enabled(true);
                return;
            };
            let TimelineEndpoints {
                update_receiver,
                update_sender,
                request_sender,
                pagination_status,
                successor_room,
            } = timeline_endpoints;

            // Start with the basic tombstone info, and fetch the full details
            // if the room has been tombstoned.
            let tombstone_info = if let Some(sr) = successor_room {
                submit_async_request(MatrixRequest::GetSuccessorRoomDetails {
                    tombstoned_room_id: room_id.clone(),
                });
                Some(SuccessorRoomDetails::Basic(sr))
            } else {
                None
            };

            let tl_state = TimelineUiState {
                kind,
                // Initially, we assume the user has all power levels by default.
                // This avoids unexpectedly hiding any UI elements that should be visible to the user.
                // This doesn't mean that the user can actually perform all actions;
                // the power levels will be updated from the homeserver once the room is opened.
                user_power: UserPowerLevels::all(),
                // Room members start as None and get populated when fetched from the server
                room_members: None,
                room_members_sort: None,
                    room_members_sync_pending: false,
                awaiting_post_sync_member_refresh: false,
                // We assume timelines being viewed for the first time haven't been fully paginated.
                fully_paginated: false,
                backwards_pagination_in_flight: false,
                items: Vector::new(),
                expanded_small_state_group_event_ids: HashSet::new(),
                small_state_event_group_index: None,
                expanded_bot_body_event_ids: HashSet::new(),
                content_drawn_since_last_update: RangeSet::new(),
                profile_drawn_since_last_update: RangeSet::new(),
                update_receiver,
                request_sender,
                pagination_status,
                pending_updates: VecDeque::new(),
                media_cache: MediaCache::new(Some(update_sender.clone())),
                link_preview_cache: LinkPreviewCache::new(Some(update_sender)),
                fetched_thread_summaries: HashMap::new(),
                pending_thread_summary_fetches: HashSet::new(),
                saved_state: SavedState::default(),
                message_highlight_animation_state: MessageHighlightAnimationState::default(),
                streaming_messages: HashMap::new(),
                last_scrolled_index: usize::MAX,
                prev_first_index: None,
                scrolled_past_read_marker: false,
                latest_own_user_receipt: None,
                tombstone_info,
                pending_downloads: Vec::new(),
            };
            (tl_state, true)
        };

        // It is possible that this room has already been loaded (received from the server)
        // but that the RoomsList doesn't yet know about it.
        // In that case, `is_first_time_being_loaded` will already be `true` here,
        // so we can bypass checking the RoomsList to determine if a room is loaded.
        //
        // Note that we *do* still need to check the RoomsList to see whether this room is loaded
        // in order to handle the case when we're switching between rooms within
        // the same RoomScreen widget, as one room may be loaded while another is not.
        if is_first_time_being_loaded {
            self.is_loaded = true;
        } else if cx.has_global::<RoomsListRef>() {
            let rooms_list_ref = cx.get_global::<RoomsListRef>();
            let is_loaded_now = rooms_list_ref.is_room_loaded(&room_id);
            if is_loaded_now && !self.is_loaded {
                // log!("Detected that {}} is now loaded for the first time", tl_state.kind);
                is_first_time_being_loaded = true;
            }
            self.is_loaded = is_loaded_now;
        }

        self.view.restore_status_view(cx, ids!(restore_status_view)).set_visible(cx, !self.is_loaded);

        // Kick off a back pagination request if it's the first time loading this room,
        // because we want to show the user some messages as soon as possible
        // when they first open the room, and there might not be any messages yet.
        if is_first_time_being_loaded {
            if !tl_state.fully_paginated
                && !tl_state.pagination_status.backwards_has_started()
            {
                tl_state.backwards_pagination_in_flight = true;
                log!("Sending a first-time backwards pagination request for {}", tl_state.kind);
                submit_async_request(MatrixRequest::PaginateTimeline {
                    timeline_kind: tl_state.kind.clone(),
                    num_events: VIEWPORT_FILL_PAGINATION_SIZE,
                    direction: PaginationDirection::Backwards,
                });
            } else {
                tl_state.backwards_pagination_in_flight =
                    tl_state.pagination_status.backwards_is_in_flight();
            }

            // Even though we specify that room member profiles should be lazy-loaded,
            // the matrix server still doesn't consistently send them to our client properly.
            // So we kick off a request to fetch the room members here upon first viewing the room.
            tl_state.room_members_sync_pending = true;
            tl_state.awaiting_post_sync_member_refresh = false;
            submit_async_request(MatrixRequest::SyncRoomMemberList {
                timeline_kind: tl_state.kind.clone(),
            });
        }

        // Hide the typing notice view initially.
        self.view(cx, ids!(typing_notice)).set_visible(cx, false);
        // If the room is loaded, we need to get a few key states:
        // 1. Get the current user's power levels for this room so that we can
        //    show/hide UI elements based on the user's permissions.
        // 2. Get the list of members in this room (from the SDK's local cache).
        // Room-specific subscriptions are enabled below, after `tl_state` is stored.
        if self.is_loaded {
            submit_async_request(MatrixRequest::GetRoomPowerLevels {
                timeline_kind: tl_state.kind.clone(),
            });
            submit_async_request(MatrixRequest::GetRoomMembers {
                timeline_kind: tl_state.kind.clone(),
                memberships: matrix_sdk::RoomMemberships::JOIN,
                // Fetch from the local cache, as we already requested to sync
                // the room members from the homeserver above.
                local_only: true,
            });
        }

        // Now, restore the visual state of this timeline from its previously-saved state.
        self.restore_state(cx, &mut tl_state);

        // Drawn-status ranges belong to the PortalList widget that populated
        // them. A shared RoomScreen (the mobile layout) can restore a
        // different room into the same widget pool, so force the restored
        // timeline's visible rows to bind to this pool once.
        tl_state.content_drawn_since_last_update.clear();
        tl_state.profile_drawn_since_last_update.clear();
        tl_state.small_state_event_group_index = None;

        // Store the tl_state for this room into this RoomScreen widget,
        // such that it can be accessed in future functions like event/draw handlers.
        self.tl_state = Some(tl_state);
        // A pending timeline can be marked visible before its endpoints exist.
        // Force the first real initialization through the subscription path.
        self.timeline_updates_enabled = false;
        self.set_timeline_updates_enabled(true);
        self.schedule_stream_timeout(cx);

        // Now that we have restored the TimelineUiState into this RoomScreen widget,
        // we can proceed to processing pending background updates.
        self.process_timeline_updates(cx, &self.portal_list(cx, ids!(list)), None);

        self.redraw(cx);
    }

    /// Invoke this when this RoomScreen/timeline is being hidden or no longer being shown.
    fn hide_timeline(&mut self) {
        let Some(timeline_kind) = self.timeline_kind.clone() else { return };
        self.streaming_timeout_timer = Timer::empty();
        self.approval_expiry_timer = Timer::empty();
        self.approval_expiry_deadline_millis = None;
        self.octos_action_button_contexts.clear();
        self.disabled_octos_action_source_event_ids.clear();
        self.selected_octos_action_by_source_event_id.clear();

        self.set_timeline_updates_enabled(false);
        self.save_state();

        // When closing a room view, we do the following with non-persistent states.
        // (This should be the inverse of what's done in `show_timeline()`.)
        // * Unsubscribe from typing notices, since we don't care about them
        //   when a given room isn't visible.
        // * Unsubscribe from updates to this room's pinned events, for the same reason.
        // * Unsubscribe from updates to our own user's read receipts, for the same reason.
        if matches!(timeline_kind, TimelineKind::MainRoom { .. }) {
            submit_async_request(MatrixRequest::SubscribeToTypingNotices {
                room_id: timeline_kind.room_id().clone(),
                subscribe: false,
            });
            submit_async_request(MatrixRequest::SubscribeToPinnedEvents {
                room_id: timeline_kind.room_id().clone(),
                subscribe: false,
            });
        }
        submit_async_request(MatrixRequest::SubscribeToOwnUserReadReceiptsChanged {
            timeline_kind,
            subscribe: false,
        });
        self.room_avatar_url = None;
        self.pending_invited_users.clear();
    }

    fn set_timeline_updates_enabled(&mut self, enabled: bool) {
        if self.timeline_updates_enabled == enabled {
            return;
        }
        self.timeline_updates_enabled = enabled;
        if enabled {
            self.resume_timeline_on_next_signal = true;
            SignalToUI::set_ui_signal();
        } else {
            // We cannot stop a timer without `Cx` here, but replacing its ID
            // makes the old event inert. The resume signal re-arms pending
            // approvals using their absolute wall-clock deadlines.
            self.approval_expiry_timer = Timer::empty();
            self.approval_expiry_deadline_millis = None;
        }

        let Some(tl) = self.tl_state.as_ref() else {
            return;
        };
        tl.request_sender.send_if_modified(|request| {
            if request.is_timeline_open == enabled {
                false
            } else {
                request.is_timeline_open = enabled;
                true
            }
        });
        if !self.is_loaded {
            return;
        }

        submit_async_request(MatrixRequest::SubscribeToOwnUserReadReceiptsChanged {
            timeline_kind: tl.kind.clone(),
            subscribe: enabled,
        });
        if matches!(tl.kind, TimelineKind::MainRoom { .. }) {
            submit_async_request(MatrixRequest::SubscribeToTypingNotices {
                room_id: tl.kind.room_id().clone(),
                subscribe: enabled,
            });
            submit_async_request(MatrixRequest::SubscribeToPinnedEvents {
                room_id: tl.kind.room_id().clone(),
                subscribe: enabled,
            });
        }
    }

    /// Removes the current room's visual UI state from this widget
    /// and saves it to the map of `TIMELINE_STATES` such that it can be restored later.
    ///
    /// Note: after calling this function, the widget's `tl_state` will be `None`.
    fn save_state(&mut self) {
        let Some(mut tl) = self.tl_state.take() else {
            error!("Timeline::save_state(): skipping due to missing state, room {:?}, {:?}", self.timeline_kind, self.room_name_id.as_ref().map(|r| r.display_name()));
            return;
        };

        let portal_list = self.child_by_path(ids!(timeline.list)).as_portal_list();
        let room_input_bar = self.child_by_path(ids!(room_input_bar)).as_room_input_bar();
        log!("Saving state for room {:?}\n\t{:?}\n\tfirst_id: {:?}, scroll: {}", self.room_name_id.as_ref().map(|r| r.display_name()), self.timeline_kind, portal_list.first_id(), portal_list.scroll_position());
        let state = SavedState {
            first_index_and_scroll: Some((portal_list.first_id(), portal_list.scroll_position())),
            room_input_bar_state: room_input_bar.save_state(),
        };
        tl.saved_state = state;
        // Clear room_members and precomputed sort to avoid wasting memory
        // (in case this room is never re-opened).
        tl.room_members = None;
        tl.room_members_sort = None;
        // Drop the room-info member-row cache too — it holds its own clone of the
        // (potentially huge) member Arc, which would otherwise survive this
        // memory reclaim until the next room's info pane is built.
        self.room_info_members_cache = None;
        self.timeline_bot_context_cache = None;
        // Store this Timeline's `TimelineUiState` in the global map of states.
        TIMELINE_STATES.with_borrow_mut(|ts| ts.insert(tl.kind.clone(), tl));
    }

    /// Restores the previously-saved visual UI state of this room.
    ///
    /// Note: this accepts a direct reference to the timeline's UI state,
    /// so this function must not try to re-obtain it by accessing `self.tl_state`.
    fn restore_state(&mut self, cx: &mut Cx, tl_state: &mut TimelineUiState) {
        let SavedState {
            first_index_and_scroll,
            room_input_bar_state,
        } = &mut tl_state.saved_state;

        // 1. Restore the position of the timeline.
        let portal_list = self.portal_list(cx, ids!(timeline.list));
        if let Some((first_index, scroll_from_first_id)) = first_index_and_scroll {
            log!("Restoring state for room {:?}: first_id: {:?}, scroll: {}", self.room_name_id, first_index, scroll_from_first_id);
            portal_list.set_first_id_and_scroll(*first_index, *scroll_from_first_id);
            portal_list.set_tail_range(false);
        } else {
            // If the first index is not set, then the timeline has not yet been scrolled by the user,
            // so we reset the portal list's scroll position and set it to "tail" (track) the bottom.
            // The explicit reset is necessary when the same RoomScreen widget is reused for a
            // different room (e.g., via stack navigation view alternation), otherwise the portal list
            // would retain the previous room's scroll position which may be out of bounds.
            log!("Restoring state for room {:?}: first_id: None, scroll: None", self.room_name_id);
            portal_list.set_first_id_and_scroll(0, 0.0);
            portal_list.set_tail_range(true);
        }

        // 2. Restore the state of the room input bar.
        let room_input_bar = self.child_by_path(ids!(room_input_bar)).as_room_input_bar();
        let saved_room_input_bar_state = std::mem::take(room_input_bar_state);
        room_input_bar.restore_state(
            cx,
            tl_state.kind.clone(),
            saved_room_input_bar_state,
            tl_state.user_power,
            tl_state.tombstone_info.as_ref(),
        );

        refresh_stream_indices(
            tl_state.items.iter().map(item_event_id),
            &mut tl_state.streaming_messages,
        );

        // 3. If there are active streaming animations that can still reveal text,
        //    re-request the NextFrame event so the animation loop resumes.
        if tl_state.streaming_messages.values().any(|state| state.needs_frame()) {
            self.streaming_next_frame = cx.new_next_frame();
        }
    }

    /// Sets this `RoomScreen` widget to display the timeline for the given room.
    pub fn set_displayed_room(
        &mut self,
        cx: &mut Cx,
        room_name_id: &RoomNameId,
        thread_root_event_id: Option<OwnedEventId>,
    ) {
        let timeline_kind = if let Some(thread_root_event_id) = thread_root_event_id {
            TimelineKind::Thread {
                room_id: room_name_id.room_id().clone(),
                thread_root_event_id,
            }
        } else {
            TimelineKind::MainRoom {
                room_id: room_name_id.room_id().clone(),
            }
        };

        // If this timeline is already displayed, we don't need to do anything major,
        // but we do need update the `room_name_id` in case it has changed, or it has been cleared.
        if self.timeline_kind.as_ref().is_some_and(|kind| kind == &timeline_kind) {
            self.room_name_id = Some(room_name_id.clone());
            self.room_avatar_url = get_client()
                .and_then(|client| client.get_room(room_name_id.room_id()))
                .and_then(|room| room.avatar_url());
            self.set_timeline_updates_enabled(true);
            if self.tl_state.is_none() {
                self.show_timeline(cx);
            }
            return;
        }

        self.hide_timeline();
        self.reset_app_service_ui(cx);
        self.hide_threads_pane(cx);
        self.hide_room_info_pane(cx);
        self.threads_pane_state = Default::default();
        // Reset the the state of the inner loading pane.
        self.loading_pane(cx, ids!(loading_pane)).take_state();

        self.room_name_id = Some(room_name_id.clone());
        self.room_avatar_url = get_client()
            .and_then(|client| client.get_room(room_name_id.room_id()))
            .and_then(|room| room.avatar_url());
        self.timeline_kind = Some(timeline_kind.clone());

        // We initially tell every MentionableTextInput widget that the current user
        // *does not* have privileges to notify the entire room;
        // this gets properly updated when room PowerLevels get fetched.
        cx.action(MentionableTextInputAction::PowerLevelsUpdated {
            room_id: timeline_kind.room_id().clone(),
            can_notify_room: false,
        });

        // A freshly-displayed room always starts on the "Chat" tab (the mobile
        // RoomTopBar's tab selection persists per widget instance, so reset it).
        self.active_room_tab = RoomTab::Chat;
        self.room_top_bar(cx, ids!(room_top_bar)).set_active_tab(cx, RoomTab::Chat);

        self.show_timeline(cx);
    }

    /// Sends read receipts based on the current scroll position of the timeline.
    fn send_user_read_receipts_based_on_scroll_pos(
        &mut self,
        cx: &mut Cx,
        actions: &ActionsBuf,
        portal_list: &PortalListRef,
    ) {
        //stopped scrolling
        if portal_list.scrolled(actions) {
            return;
        }
        let has_encryption_notice = self.current_has_encryption_notice(cx);
        let first_item_id = portal_list.first_id();
        let first_index = tl_idx_from_item_id(first_item_id, has_encryption_notice).unwrap_or(0);
        let Some(tl_state) = self.tl_state.as_mut() else { return };

        if let Some(ref mut index) = tl_state.prev_first_index {
            // to detect change of scroll when scroll ends
            if *index != first_index {
                if first_index >= *index {
                    // Get event_id and timestamp for the last visible event
                    let Some((last_event_id, last_timestamp)) = tl_state
                        .items
                        .get(std::cmp::min(
                            first_index + portal_list.visible_items(),
                            tl_state.items.len().saturating_sub(1)
                        ))
                        .and_then(|f| f.as_event())
                        .and_then(|f| f.event_id().map(|e| (e, f.timestamp())))
                    else {
                        *index = first_index;
                        return;
                    };
                    submit_async_request(MatrixRequest::ReadReceipt {
                        timeline_kind: tl_state.kind.clone(),
                        event_id: last_event_id.to_owned(),
                        receipt_type: ReceiptType::Read,
                    });
                    if tl_state.scrolled_past_read_marker {
                        submit_async_request(MatrixRequest::ReadReceipt {
                            timeline_kind: tl_state.kind.clone(),
                            event_id: last_event_id.to_owned(),
                            receipt_type: ReceiptType::FullyRead,
                        });
                    } else {
                        if let Some(own_user_receipt_timestamp) = &tl_state.latest_own_user_receipt.clone()
                        .and_then(|receipt| receipt.ts) {
                            let Some((_first_event_id, first_timestamp)) = tl_state
                                .items
                                .get(first_index)
                                .and_then(|f| f.as_event())
                                .and_then(|f| f.event_id().map(|e| (e, f.timestamp())))
                                else {
                                    *index = first_index;
                                    return;
                                };
                            if own_user_receipt_timestamp >= &first_timestamp
                                && own_user_receipt_timestamp <= &last_timestamp
                            {
                                tl_state.scrolled_past_read_marker = true;
                                submit_async_request(MatrixRequest::ReadReceipt {
                                    timeline_kind: tl_state.kind.clone(),
                                    event_id: last_event_id.to_owned(),
                                    receipt_type: ReceiptType::FullyRead,
                                });
                            }

                        }
                    }
                }
                *index = first_index;
            }
        } else {
            tl_state.prev_first_index = Some(first_index);
        }
    }

    /// Sends a backwards pagination request if the user is scrolling up
    /// and is approaching the top of the timeline.
    fn send_pagination_request_based_on_scroll_pos(
        &mut self,
        cx: &mut Cx,
        actions: &ActionsBuf,
        portal_list: &PortalListRef,
    ) {
        let has_encryption_notice = self.current_has_encryption_notice(cx);
        let Some(tl) = self.tl_state.as_mut() else { return };
        if tl.fully_paginated { return };
        if !portal_list.scrolled(actions) { return };

        let first_index = tl_idx_from_item_id(portal_list.first_id(), has_encryption_notice).unwrap_or(0);
        if first_index == 0 && tl.last_scrolled_index > 0 && !tl.backwards_pagination_in_flight {
            tl.backwards_pagination_in_flight = true;
            log!("Scrolled up from item {} --> 0, sending back pagination request for room {}",
                tl.last_scrolled_index, tl.kind,
            );
            submit_async_request(MatrixRequest::PaginateTimeline {
                timeline_kind: tl.kind.clone(),
                num_events: 50,
                direction: PaginationDirection::Backwards,
            });
        }
        tl.last_scrolled_index = first_index;
    }
}

impl RoomScreenRef {
    /// See [`RoomScreen::set_displayed_room()`].
    pub fn set_displayed_room(
        &self,
        cx: &mut Cx,
        room_name_id: &RoomNameId,
        thread_root_event_id: Option<OwnedEventId>,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_displayed_room(cx, room_name_id, thread_root_event_id);
    }

    /// Enables or pauses background updates for this timeline without discarding its UI state.
    pub fn set_timeline_updates_enabled(&self, enabled: bool) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_timeline_updates_enabled(enabled);
    }
}

/// Returns info about the item in the list of `new_items` that matches the event ID
/// of a visible item in the given `curr_items` list.
///
/// This info includes a tuple of:
/// 1. the index of the item in the current items list,
/// 2. the index of the item in the new items list,
/// 3. the positional "scroll" offset of the corresponding current item in the portal list,
/// 4. the unique event ID of the item.
fn find_new_item_matching_current_item(
    cx: &mut Cx,
    portal_list: &PortalListRef,
    starting_at_curr_idx: usize,
    curr_items: &Vector<Arc<TimelineItem>>,
    new_items: &Vector<Arc<TimelineItem>>,
    has_encryption_notice: bool,
) -> Option<(usize, usize, f64, OwnedEventId)> {
    let mut curr_item_focus = curr_items.focus();
    let mut idx_curr = starting_at_curr_idx;
    let mut curr_items_with_ids: Vec<(usize, OwnedEventId)> = Vec::with_capacity(
        portal_list.visible_items()
    );

    // Find all items with real event IDs that are currently visible in the portal list.
    // TODO: if this is slow, we could limit it to 3-5 events at the most.
    if curr_items_with_ids.len() <= portal_list.visible_items() {
        while let Some(curr_item) = curr_item_focus.get(idx_curr) {
            if let Some(event_id) = curr_item.as_event().and_then(|ev| ev.event_id()) {
                curr_items_with_ids.push((idx_curr, event_id.to_owned()));
            }
            if curr_items_with_ids.len() >= portal_list.visible_items() {
                break;
            }
            idx_curr += 1;
        }
    }

    // Find a new item that has the same real event ID as any of the current items.
    for (idx_new, new_item) in new_items.iter().enumerate() {
        let Some(event_id) = new_item.as_event().and_then(|ev| ev.event_id()) else {
            continue;
        };
        if let Some((idx_curr, _)) = curr_items_with_ids
            .iter()
            .find(|(_, ev_id)| ev_id == event_id)
        {
            // Not all items in the portal list are guaranteed to have a position offset,
            // some may be zeroed-out, so we need to account for that possibility by only
            // using events that have a real non-zero area
            if let Some(pos_offset) = portal_list.position_of_item(
                cx,
                item_id_from_tl_idx(*idx_curr, has_encryption_notice),
            ) {
                log!("Found matching event ID {event_id} at index {idx_new} in new items list, corresponding to current item index {idx_curr} at pos offset {pos_offset}");
                return Some((*idx_curr, idx_new, pos_offset, event_id.to_owned()));
            }
        }
    }

    None
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct ItemDrawnStatus {
    /// Whether the profile info (avatar and displayable username) were drawn for this item.
    profile_drawn: bool,
    /// Whether the content of the item was drawn (e.g., the message text, image, video, sticker, etc).
    content_drawn: bool,
}

#[derive(Clone, Debug)]
struct FetchedThreadSummary {
    num_replies: u32,
    latest_reply_preview_text: Option<String>,
}

impl ItemDrawnStatus {
    /// Returns a new `ItemDrawnStatus` with both `profile_drawn` and `content_drawn` set to `false`.
    const fn new() -> Self {
        Self {
            profile_drawn: false,
            content_drawn: false,
        }
    }
    /// Returns a new `ItemDrawnStatus` with both `profile_drawn` and `content_drawn` set to `true`.
    const fn both_drawn() -> Self {
        Self {
            profile_drawn: true,
            content_drawn: true,
        }
    }
}

/// Creates, populates, and adds a Message liveview widget to the given `PortalList`
/// with the given `item_id`.
///
/// The content of the returned `Message` widget is populated with data from a message
/// or sticker and its containing `EventTimelineItem`.
fn populate_message_view(
    cx: &mut Cx2d,
    list: &mut PortalList,
    item_id: usize,
    timeline_kind: &TimelineKind,
    app_language: AppLanguage,
    event_tl_item: &EventTimelineItem,
    msg_like_content: &MsgLikeContent,
    prev_event: Option<&Arc<TimelineItem>>,
    media_cache: &mut MediaCache,
    link_preview_cache: &mut LinkPreviewCache,
    fetched_thread_summaries: &HashMap<OwnedEventId, FetchedThreadSummary>,
    pending_thread_summary_fetches: &mut HashSet<OwnedEventId>,
    user_power_levels: &UserPowerLevels,
    pinned_events: &[OwnedEventId],
    pending_downloads: &[PendingDownload],
    item_drawn_status: ItemDrawnStatus,
    room_screen_widget_uid: WidgetUid,
    resolved_parent_bot_user_id: Option<&UserId>,
    room_bot_user_ids: &[OwnedUserId],
    known_bot_user_ids: &[OwnedUserId],
    streaming_messages: &mut HashMap<OwnedEventId, super::streaming_animation::StreamingAnimState>,
    action_button_contexts: &mut HashMap<WidgetUid, OctosActionButtonContext>,
    disabled_action_source_event_ids: &HashSet<OwnedEventId>,
    selected_actions: &HashMap<OwnedEventId, SelectedOctosActionState>,
    expanded_bot_body_event_ids: &HashSet<OwnedEventId>,
) -> (WidgetRef, ItemDrawnStatus, bool) {
    let mut new_drawn_status = item_drawn_status;
    let ts_millis = event_tl_item.timestamp();
    // Whether the user unfolded this (long) bot reply; folded is the default.
    let bot_body_expanded = event_tl_item
        .event_id()
        .is_some_and(|id| expanded_bot_body_event_ids.contains(id));
    let sender_is_bot_cache = Cell::new(None);
    let sender_is_bot = || {
        if let Some(is_bot) = sender_is_bot_cache.get() {
            return is_bot;
        }
        let is_bot = is_timeline_sender_bot(
            event_tl_item.sender(),
            resolved_parent_bot_user_id,
            room_bot_user_ids,
            known_bot_user_ids,
        );
        sender_is_bot_cache.set(Some(is_bot));
        is_bot
    };

    let mut is_notice = false; // whether this message is a Notice (automated bot message)
    let mut is_server_notice = false; // whether this message is a Server Notice

    // Determine whether we can use a more compact UI view that hides the user's profile info
    // if the previous message (including stickers) was sent by the same user within 10 minutes.
    let use_compact_view = match prev_event.map(|p| p.kind()) {
        Some(TimelineItemKind::Event(prev_event_tl_item)) => match prev_event_tl_item.content() {
            TimelineItemContent::MsgLike(_msg_like_content) => {
                let prev_msg_sender = prev_event_tl_item.sender();
                prev_msg_sender == event_tl_item.sender()
                    && ts_millis.0
                        .checked_sub(prev_event_tl_item.timestamp().0)
                        .is_some_and(|d| d < uint!(600000)) // 10 mins in millis
            }
            _ => false,
        },
        _ => false,
    };

    let has_html_body: bool;

    // Sometimes we need to call this up-front, so we save the result in this variable
    // to avoid having to call it twice.
    let mut set_username_and_get_avatar_retval = None;
    // Model/provider metadata for the meta band below the message content,
    // produced by the bot populate path; None for everything else.
    let mut band_metadata: Option<String> = None;
    let (item, used_cached_item) = match &msg_like_content.kind {
        MsgLikeKind::Message(msg) => {
            let room_mention_room_id = if msg.mentions().is_some_and(|m| m.room) {
                Some(timeline_kind.room_id())
            } else {
                None
            };
            match msg.msgtype() {
                MessageType::Text(TextMessageEventContent { body, formatted, .. }) => {
                    has_html_body = formatted.as_ref().is_some_and(|f| f.format == MessageFormat::Html);
                    let template = if use_compact_view {
                        id!(CondensedMessage)
                    } else {
                        id!(Message)
                    };
                    let (item, existed) = list.item_with_existed(cx, item_id, template);
                    if existed && item_drawn_status.content_drawn {
                        (item, true)
                    } else {
                        // Check if this message is being streamed
                        let is_streaming = event_tl_item.event_id()
                            .and_then(|eid| streaming_messages.get_mut(&eid.to_owned()));

                        if let Some(state) = is_streaming {
                            let render_full_snapshot = should_render_streaming_full_snapshot(
                                body,
                                formatted.as_ref(),
                                sender_is_bot(),
                            );
                            state.set_render_full_target(render_full_snapshot);

                            // STREAMING MODE:
                            // - markdown-rich bot replies render the latest full snapshot directly
                            // - plain text keeps the local typewriter prefix with cursor
                            let mut link_preview_ref =
                                item.link_preview(cx, ids!(content.link_preview_view));
                            let (stream_body, stream_formatted) = if render_full_snapshot {
                                (body.as_str(), formatted.as_ref())
                            } else {
                                state.fill_display_buffer();
                                (state.display_buffer.as_str(), None)
                            };
                            let (_, stream_meta) = populate_bot_text_message_content(
                                cx,
                                &item,
                                app_language,
                                stream_body,
                                stream_formatted,
                                room_mention_room_id,
                                Some(&mut link_preview_ref),
                                Some(media_cache),
                                Some(link_preview_cache),
                                sender_is_bot(),
                                bot_body_expanded,
                                true,  // streaming
                            );
                            band_metadata = stream_meta;
                            new_drawn_status.content_drawn = false; // force re-render
                        } else {
                            // Check for Splash card in custom event field
                            let splash_code = event_raw_json_contains_any(
                                event_tl_item,
                                &["\"org.octos.splash_card\""],
                            )
                            .then(|| latest_effective_event_content_json(event_tl_item))
                            .flatten()
                            .and_then(|content|
                                content
                                    .get("org.octos.splash_card")
                                    .and_then(|v| v.as_str().map(ToOwned::to_owned))
                            );

                            if let Some(ref splash) = splash_code {
                                // SPLASH CARD MODE: render native Makepad card
                                item.view(cx, ids!(content.message)).set_visible(cx, false);
                                let splash_widget = item.splash(cx, ids!(content.splash_card));
                                splash_widget.set_visible(cx, true);
                                splash_widget.set_text(cx, splash);
                                new_drawn_status.content_drawn = true;
                            } else {
                                // NORMAL MODE: existing logic
                                let mut link_preview_ref =
                                    item.link_preview(cx, ids!(content.link_preview_view));
                                let (bot_drawn, bot_meta) = populate_bot_text_message_content(
                                    cx,
                                    &item,
                                    app_language,
                                    body,
                                    formatted.as_ref(),
                                    room_mention_room_id,
                                    Some(&mut link_preview_ref),
                                    Some(media_cache),
                                    Some(link_preview_cache),
                                    sender_is_bot(),
                                    bot_body_expanded,
                                    false,
                                );
                                new_drawn_status.content_drawn = bot_drawn;
                                band_metadata = bot_meta;
                            }
                        }
                        (item, false)
                    }
                }
                // A notice message is just a message sent by an automated bot,
                // so we treat it just like a message but use a different font color.
                MessageType::Notice(NoticeMessageEventContent{body, formatted, ..}) => {
                    is_notice = true;
                    has_html_body = formatted.as_ref().is_some_and(|f| f.format == MessageFormat::Html);
                    let template = if use_compact_view {
                        id!(CondensedMessage)
                    } else {
                        id!(Message)
                    };
                    let (item, existed) = list.item_with_existed(cx, item_id, template);
                    if existed && item_drawn_status.content_drawn {
                        (item, true)
                    } else {
                        if !sender_is_bot() {
                            let html_or_plaintext_ref = item.html_or_plaintext(cx, ids!(content.message));
                            // Apply gray color to all text styles for notice messages.
                            // This covers both rendering paths in HtmlOrPlaintext: the rich
                            // `html_view.html` widget (used when the message has an HTML body)
                            // and the `plaintext_view.pt_label` (used for plain-text notices).
                            let mut html_widget = html_or_plaintext_ref.html(cx, ids!(html_view.html));
                            script_apply_eval!(cx, html_widget, {
                                font_color: mod.widgets.COLOR_MESSAGE_NOTICE_TEXT,
                                draw_block +: {
                                    quote_fg_color: mod.widgets.COLOR_MESSAGE_NOTICE_TEXT,
                                }
                            });
                            let mut pt_label = html_or_plaintext_ref.label(cx, ids!(plaintext_view.pt_label));
                            script_apply_eval!(cx, pt_label, {
                                draw_text +: {
                                    color: mod.widgets.COLOR_MESSAGE_NOTICE_TEXT
                                }
                            });
                        }
                        let mut link_preview_ref =
                            item.link_preview(cx, ids!(content.link_preview_view));
                        let (bot_drawn, bot_meta) = populate_bot_text_message_content(
                            cx,
                            &item,
                            app_language,
                            body,
                            formatted.as_ref(),
                            room_mention_room_id,
                            Some(&mut link_preview_ref),
                            Some(media_cache),
                            Some(link_preview_cache),
                            sender_is_bot(),
                            bot_body_expanded,
                            false,
                        );
                        new_drawn_status.content_drawn = bot_drawn;
                        band_metadata = bot_meta;
                        (item, false)
                    }
                }
                MessageType::ServerNotice(sn) => {
                    is_server_notice = true;
                    has_html_body = false;
                    let (item, existed) = list.item_with_existed(cx, item_id, id!(Message));
                    if existed && item_drawn_status.content_drawn {
                        (item, true)
                    } else {
                        reset_bot_message_card(cx, &item);
                        let html_or_plaintext_ref = item.html_or_plaintext(cx, ids!(content.message));
                        // Apply red color to all text styles for server notices.
                        let mut html_widget = html_or_plaintext_ref.html(cx, ids!(html_view.html));
                        script_apply_eval!(cx, html_widget, {
                            font_color: mod.widgets.COLOR_FG_DANGER_RED
                            draw_text +: { color: mod.widgets.COLOR_FG_DANGER_RED }
                            draw_block +: {
                                line_color: mod.widgets.COLOR_FG_DANGER_RED
                                quote_fg_color: mod.widgets.COLOR_FG_DANGER_RED
                            }
                        });
                        let formatted = format!(
                            "<b>{}</b> {}\n\n<i>{}</i>: {}{}{}",
                            tr_key(app_language, "room_screen.server_notice.title"),
                            sn.body,
                            tr_key(app_language, "room_screen.server_notice.notice_type"),
                            sn.server_notice_type.as_str(),
                            sn.limit_type.as_ref()
                                .map(|l| format!("\n<i>{}</i> {}", tr_key(app_language, "room_screen.server_notice.limit_type"), l.as_str()))
                                .unwrap_or_default(),
                            sn.admin_contact.as_ref()
                                .map(|c| format!("\n<i>{}</i> {}", tr_key(app_language, "room_screen.server_notice.admin_contact"), c))
                                .unwrap_or_default(),
                        );
                        let mut link_preview_ref =
                            item.link_preview(cx, ids!(content.link_preview_view));
                        new_drawn_status.content_drawn = populate_text_message_content(
                            cx,
                            &html_or_plaintext_ref,
                            app_language,
                            &sn.body,
                            Some(&FormattedBody {
                                format: MessageFormat::Html,
                                body: formatted,
                            }),
                            room_mention_room_id,
                            Some(&mut link_preview_ref),
                            Some(media_cache),
                            Some(link_preview_cache),
                        );
                        (item, false)
                    }
                }
                // An emote is just like a message but is prepended with the user's name
                // to indicate that it's an "action" that the user is performing.
                MessageType::Emote(EmoteMessageEventContent { body, formatted, .. }) => {
                    has_html_body = formatted.as_ref().is_some_and(|f| f.format == MessageFormat::Html);
                    let template = if use_compact_view {
                        id!(CondensedMessage)
                    } else {
                        id!(Message)
                    };
                    let (item, existed) = list.item_with_existed(cx, item_id, template);
                    if existed && item_drawn_status.content_drawn {
                        (item, true)
                    } else {
                        // Draw the profile up front here because we need the username for the emote body.
                        let (username, profile_drawn) = item.avatar(cx, ids!(profile.avatar)).set_avatar_and_get_username(
                            cx,
                            timeline_kind,
                            event_tl_item.sender(),
                            Some(event_tl_item.sender_profile()),
                            event_tl_item.event_id(),
                            true,
                        );

                        // Prepend a "* <username> " to the emote body, as suggested by the Matrix spec.
                        let (body, formatted) = if let Some(fb) = formatted.as_ref() {
                            (
                                Cow::from(&fb.body),
                                Some(FormattedBody {
                                    format: fb.format.clone(),
                                    body: format!("* {} {}", &username, &fb.body),
                                })
                            )
                        } else {
                            (Cow::from(format!("* {} {}", &username, body)), None)
                        };
                        let html_or_plaintext_ref =
                            item.html_or_plaintext(cx, ids!(content.message));
                        let mut link_preview_ref =
                            item.link_preview(cx, ids!(content.link_preview_view));
                        let link_previews_drawn = populate_text_message_content(
                            cx,
                            &html_or_plaintext_ref,
                            app_language,
                            &body,
                            formatted.as_ref(),
                            room_mention_room_id,
                            Some(&mut link_preview_ref),
                            Some(media_cache),
                            Some(link_preview_cache),
                        );
                        set_username_and_get_avatar_retval = Some((username, profile_drawn));
                        new_drawn_status.content_drawn = link_previews_drawn;
                        (item, false)
                    }
                }
                MessageType::Image(image) => {
                    has_html_body = image.formatted.as_ref()
                        .is_some_and(|f| f.format == MessageFormat::Html);
                    let template = if use_compact_view {
                        id!(CondensedImageMessage)
                    } else {
                        id!(ImageMessage)
                    };
                    let (item, existed) = list.item_with_existed(cx, item_id, template);
                    if existed && item_drawn_status.content_drawn {
                        (item, true)
                    } else {
                        let image_info = image.info.clone();
                        let text_or_image_ref = item.text_or_image(cx, ids!(content.message));
                        let animated_image_ref = item.animated_image(cx, ids!(content.animated_message));
                        let is_image_fully_drawn = populate_image_message_content(
                            cx,
                            &text_or_image_ref,
                            Some(&animated_image_ref),
                            app_language,
                            image_info,
                            image.source.clone(),
                            msg.body(),
                            media_cache,
                        );
                        new_drawn_status.content_drawn = is_image_fully_drawn;
                        (item, false)
                    }
                }
                MessageType::Location(location) => {
                    has_html_body = false;
                    let template = if use_compact_view {
                        id!(CondensedMessage)
                    } else {
                        id!(Message)
                    };
                    let (item, existed) = list.item_with_existed(cx, item_id, template);
                    if existed && item_drawn_status.content_drawn {
                        (item, true)
                    } else {
                        let html_or_plaintext_ref =
                            item.html_or_plaintext(cx, ids!(content.message));
                        let is_location_fully_drawn = populate_location_message_content(
                            cx,
                            &html_or_plaintext_ref,
                            app_language,
                            location,
                        );
                        new_drawn_status.content_drawn = is_location_fully_drawn;
                        (item, false)
                    }
                }
                MessageType::File(file_content) => {
                    has_html_body = file_content.formatted.as_ref().is_some_and(|f| f.format == MessageFormat::Html);
                    let template = if use_compact_view {
                        id!(CondensedMessage)
                    } else {
                        id!(Message)
                    };
                    let (item, existed) = list.item_with_existed(cx, item_id, template);
                    if existed && item_drawn_status.content_drawn {
                        (item, true)
                    } else {
                        let html_or_plaintext_ref =
                            item.html_or_plaintext(cx, ids!(content.message));
                        new_drawn_status.content_drawn = populate_file_message_content(
                            cx,
                            &html_or_plaintext_ref,
                            app_language,
                            file_content,
                            media_cache,
                        );
                        (item, false)
                    }
                }
                MessageType::Audio(audio) => {
                    has_html_body = audio.formatted.as_ref().is_some_and(|f| f.format == MessageFormat::Html);
                    let template = if use_compact_view {
                        id!(CondensedMessage)
                    } else {
                        id!(AudioMessage)
                    };
                    let (item, existed) = list.item_with_existed(cx, item_id, template);
                    if existed && item_drawn_status.content_drawn {
                        (item, true)
                    } else {
                        let html_or_plaintext_ref =
                            item.html_or_plaintext(cx, ids!(content.message));
                        new_drawn_status.content_drawn = populate_audio_message_content(
                            cx,
                            &item,
                            &html_or_plaintext_ref,
                            app_language,
                            audio,
                            media_cache,
                        );
                        (item, false)
                    }
                }
                MessageType::Video(video) => {
                    has_html_body = video.formatted.as_ref().is_some_and(|f| f.format == MessageFormat::Html);
                    let template = if use_compact_view {
                        id!(CondensedMessage)
                    } else {
                        id!(VideoMessage)
                    };
                    let (item, existed) = list.item_with_existed(cx, item_id, template);
                    if existed && item_drawn_status.content_drawn {
                        (item, true)
                    } else {
                        let html_or_plaintext_ref =
                            item.html_or_plaintext(cx, ids!(content.message));
                        new_drawn_status.content_drawn = populate_video_message_content(
                            cx,
                            &item,
                            &html_or_plaintext_ref,
                            app_language,
                            video,
                            media_cache,
                        );
                        (item, false)
                    }
                }
                MessageType::VerificationRequest(verification) => {
                    has_html_body = verification.formatted.as_ref().is_some_and(|f| f.format == MessageFormat::Html);
                    let template = id!(Message);
                    let (item, existed) = list.item_with_existed(cx, item_id, template);
                    if existed && item_drawn_status.content_drawn {
                        (item, true)
                    } else {
                        // Use `FormattedBody` to hold our custom summary of this verification request.
                        let formatted = FormattedBody {
                            format: MessageFormat::Html,
                            body: format!(
                                "<i>{}<b>{}</b>{}<br>({}: {})</i>",
                                tr_key(app_language, "room_screen.verification.sent_prefix"),
                                tr_key(app_language, "room_screen.verification.request"),
                                tr_fmt(app_language, "room_screen.verification.sent_to_suffix", &[("user_id", verification.to.as_str())]),
                                tr_key(app_language, "room_screen.verification.supported_methods"),
                                verification.methods
                                    .iter()
                                    .map(|m| m.as_str())
                                    .collect::<Vec<_>>()
                                    .join(", "),
                            ),
                        };
                        let html_or_plaintext_ref =
                            item.html_or_plaintext(cx, ids!(content.message));
                        let mut link_preview_ref =
                            item.link_preview(cx, ids!(content.link_preview_view));

                        new_drawn_status.content_drawn = populate_text_message_content(
                            cx,
                            &html_or_plaintext_ref,
                            app_language,
                            &verification.body,
                            Some(&formatted),
                            room_mention_room_id,
                            Some(&mut link_preview_ref),
                            Some(media_cache),
                            Some(link_preview_cache),
                        );
                        (item, false)
                    }
                }
                _ => {
                    has_html_body = false;
                    // Security-sensitive agent-chat fields come from the
                    // original event, never from an m.replace edit. Parsing is
                    // deferred to this rare custom-msgtype branch so ordinary
                    // messages do not deserialize JSON on every draw.
                    let agentchat_custom_body = original_event_content_json(event_tl_item)
                        .as_ref()
                        .and_then(agentchat_custom_message_body_from_content)
                        .map(str::to_owned);
                    let template = if agentchat_custom_body.is_some() && use_compact_view {
                        id!(CondensedMessage)
                    } else {
                        id!(Message)
                    };
                    let (item, existed) = list.item_with_existed(cx, item_id, template);
                    if existed && item_drawn_status.content_drawn {
                        (item, true)
                    } else if let Some(agentchat_custom_body) = agentchat_custom_body {
                        let html_or_plaintext_ref = item.html_or_plaintext(cx, ids!(content.message));
                        let mut link_preview_ref = item.link_preview(cx, ids!(content.link_preview_view));
                        new_drawn_status.content_drawn = populate_text_message_content(
                            cx,
                            &html_or_plaintext_ref,
                            app_language,
                            &agentchat_custom_body,
                            None,
                            None,
                            Some(&mut link_preview_ref),
                            Some(media_cache),
                            Some(link_preview_cache),
                        );
                        (item, false)
                    } else {
                        item.label(cx, ids!(content.message)).set_text(
                            cx,
                            &format!("{} {:?}", tr_key(app_language, "room_screen.unsupported.prefix"), msg_like_content.kind),
                        );
                        new_drawn_status.content_drawn = true;
                        (item, false)
                    }
                }
            }
        }
        // Handle sticker messages that are static images.
        MsgLikeKind::Sticker(sticker) => {
            has_html_body = false;
            let StickerEventContent { body, info, source, .. } = sticker.content();

            let template = if use_compact_view {
                id!(CondensedStickerMessage)
            } else {
                id!(StickerMessage)
            };
            let (item, existed) = list.item_with_existed(cx, item_id, template);

            if existed && item_drawn_status.content_drawn {
                (item, true)
            } else {
                let image_info = info;
                let text_or_image_ref = item.text_or_image(cx, ids!(content.message));
                let is_image_fully_drawn = populate_image_message_content(
                    cx,
                    &text_or_image_ref,
                    None,
                    app_language,
                    Some(Box::new(image_info.clone())),
                    source.clone().into(),
                    body,
                    media_cache,
                );
                new_drawn_status.content_drawn = is_image_fully_drawn;
                (item, false)
            }
        } 
        // Handle messages that have been redacted (deleted).
        MsgLikeKind::Redacted => {
            has_html_body = false;
            let template = if use_compact_view {
                id!(CondensedMessage)
            } else {
                id!(Message)
            };
            let (item, existed) = list.item_with_existed(cx, item_id, template);
            if existed && item_drawn_status.content_drawn {
                (item, true)
            } else {
                let html_or_plaintext_ref = item.html_or_plaintext(cx, ids!(content.message));
                // Apply a smaller font size for redacted messages.
                let mut html_widget = html_or_plaintext_ref.html(cx, ids!(html_view.html));
                script_apply_eval!(cx, html_widget, {
                    font_size: mod.widgets.REDACTED_MESSAGE_FONT_SIZE
                    text_style_normal +: { font_size: mod.widgets.REDACTED_MESSAGE_FONT_SIZE }
                    text_style_italic +: { font_size: mod.widgets.REDACTED_MESSAGE_FONT_SIZE }
                    text_style_bold +: { font_size: mod.widgets.REDACTED_MESSAGE_FONT_SIZE }
                    text_style_bold_italic +: { font_size: mod.widgets.REDACTED_MESSAGE_FONT_SIZE }
                    text_style_fixed +: { font_size: mod.widgets.REDACTED_MESSAGE_FONT_SIZE }
                });
                new_drawn_status.content_drawn = populate_redacted_message_content(
                    cx,
                    &html_or_plaintext_ref,
                    app_language,
                    event_tl_item,
                    timeline_kind.room_id(),
                );
                (item, false)
            }
        }
        other => {
            has_html_body = false;
            let (item, existed) = list.item_with_existed(cx, item_id, id!(Message));
            if existed && item_drawn_status.content_drawn {
                (item, true)
            } else {
                reset_bot_message_card(cx, &item);
                item.label(cx, ids!(content.message)).set_text(
                    cx,
                    &format!("{} {:?} ", tr_key(app_language, "room_screen.unsupported.prefix"), other),
                );
                new_drawn_status.content_drawn = true;
                (item, false)
            }
        }
    };

    // If we didn't use a cached item, we need to draw all other message content:
    // reactions, read receipts, reply preview, metadata, and action controls.
    if !used_cached_item {
        let timeline_event_id = event_tl_item.identifier();
        item.reaction_list(cx, ids!(content.reaction_list)).set_list(
            cx,
            event_tl_item.content().reactions(),
            timeline_kind.clone(),
            timeline_event_id.clone(),
            item_id,
        );
        populate_read_receipts(&item, cx, timeline_kind, event_tl_item);
        let is_reply_fully_drawn = draw_replied_to_message(
            cx,
            &item.view(cx, ids!(replied_to_message)),
            timeline_kind,
            app_language,
            msg_like_content.in_reply_to.as_ref(),
            event_tl_item.event_id(),
        );
        let is_thread_summary_fully_drawn = populate_thread_root_summary(
            cx,
            &item,
            item_id,
            timeline_kind,
            app_language,
            msg_like_content,
            event_tl_item,
            fetched_thread_summaries,
            pending_thread_summary_fetches,
        );

        // The content is only considered to be fully drawn if the logic above marked it as such
        // *and* if the reply preview was also fully drawn
        // *and* if the thread root summary (if applicable) was also fully drawn.
        new_drawn_status.content_drawn &= is_reply_fully_drawn;
        new_drawn_status.content_drawn &= is_thread_summary_fully_drawn;

        let has_room_mention = matches!(
            &msg_like_content.kind,
            MsgLikeKind::Message(msg) if msg.mentions().is_some_and(|m| m.room)
        );
        let message_details = MessageDetails {
            thread_root_event_id: msg_like_content.thread_root.clone().or_else(|| {
                msg_like_content.thread_summary.as_ref()
                    .and_then(|_| event_tl_item.event_id().map(|id| id.to_owned()))
            }),
            timeline_event_id,
            item_id,
            related_event_id: msg_like_content.in_reply_to.as_ref().map(|r| r.event_id.clone()),
            room_screen_widget_uid,
            is_thread_timeline: timeline_kind.thread_root_event_id().is_some(),
            abilities: MessageAbilities::from_user_power_and_event(
                user_power_levels,
                event_tl_item,
                msg_like_content,
                pinned_events,
                has_html_body,
            ),
            should_be_highlighted: event_tl_item.is_highlighted() || has_room_mention,
        };
        let download_info = match &msg_like_content.kind {
            MsgLikeKind::Message(message) => match message.msgtype() {
                MessageType::File(file) => Some(DownloadableAttachment {
                    media_source: file.source.clone(),
                    filename: file.filename().to_owned(),
                    size: file.info.as_ref().and_then(|info| info.size).map(u64::from),
                    kind: DownloadKind::File,
                }),
                MessageType::Audio(audio) => Some(DownloadableAttachment {
                    media_source: audio.source.clone(),
                    filename: audio.filename().to_owned(),
                    size: audio.info.as_ref().and_then(|info| info.size).map(u64::from),
                    kind: DownloadKind::Audio,
                }),
                MessageType::Video(video) => Some(DownloadableAttachment {
                    media_source: video.source.clone(),
                    filename: video.filename().to_owned(),
                    size: video.info.as_ref().and_then(|info| info.size).map(u64::from),
                    kind: DownloadKind::Video,
                }),
                _ => None,
            },
            _ => None,
        };
        let download_state = download_info.as_ref()
            .and_then(|info|
                pending_downloads.iter()
                    .find(|pending| pending.mxc == *media_source_mxc(&info.media_source))
            )
            .map(|entry| entry.state.display())
            .unwrap_or_default();
        // Notices only get a copy button from bot senders: agents replying via
        // m.notice are conversational, while human notices are management feedback.
        let show_copy_button = matches!(
            &msg_like_content.kind,
            MsgLikeKind::Message(msg) if match msg.msgtype() {
                MessageType::Text(_) | MessageType::Emote(_) => true,
                MessageType::Notice(_) => sender_is_bot(),
                _ => false,
            }
        );
        item.as_message().set_data(
            cx,
            message_details,
            download_info,
            download_state,
            show_copy_button,
        );
        item.as_message().set_band_metadata(cx, band_metadata);
        populate_send_state(cx, &item, event_tl_item);

        let has_action_payload = event_raw_json_contains_any(
            event_tl_item,
            &[
                "\"org.octos.actions\"",
                "\"org.octos.approval_request\"",
                "\"com.agentchat.approval\"",
            ],
        );
        let action_button_content = has_action_payload
            .then(|| latest_effective_event_content_json(event_tl_item))
            .flatten();
        let original_action_button_content = has_action_payload
            .then(|| original_event_content_json(event_tl_item))
            .flatten();
        let source_event_id = event_tl_item.event_id().map(|event_id| event_id.to_owned());
        populate_octos_action_buttons(
            cx,
            app_language,
            &item,
            item_id,
            action_button_content.as_ref(),
            original_action_button_content.as_ref(),
            source_event_id.as_ref(),
            event_tl_item.sender(),
            action_button_contexts,
            disabled_action_source_event_ids,
            selected_actions,
        );
    }


    // If `used_cached_item` is false, we should always redraw the profile, even if profile_drawn is true.
    let skip_draw_profile =
        use_compact_view || (used_cached_item && item_drawn_status.profile_drawn);
    if skip_draw_profile {
        // log!("\t --> populate_message_view(): SKIPPING profile draw for item_id: {item_id}");
        new_drawn_status.profile_drawn = true;
    } else {
        // log!("\t --> populate_message_view(): DRAWING  profile draw for item_id: {item_id}");
        let mut username_label = item.label(cx, ids!(content.username));

        if !is_server_notice { // the normal case
            let (username, profile_drawn) = set_username_and_get_avatar_retval.unwrap_or_else(||
                item.avatar(cx, ids!(profile.avatar)).set_avatar_and_get_username(
                    cx,
                    timeline_kind,
                    event_tl_item.sender(),
                    Some(event_tl_item.sender_profile()),
                    event_tl_item.event_id(),
                    true,
                )
            );
            if is_notice {
                script_apply_eval!(cx, username_label, {
                    draw_text +: {
                        color: mod.widgets.COLOR_MESSAGE_NOTICE_TEXT
                    }
                });
            }
            username_label.set_text(cx, &username);
            new_drawn_status.profile_drawn = profile_drawn;

            // Show/hide the bot badge based on sender's user ID
            item.view(cx, ids!(content.username_view.bot_badge)).set_visible(cx, sender_is_bot());
            if sender_is_bot() {
                populate_bot_badge_identity(cx, &item, event_tl_item.sender().localpart());
            }
        }
        else {
            // Server notices are drawn with a red color avatar background and username.
            let avatar = item.avatar(cx, ids!(profile.avatar));
            avatar.show_text(cx, Some(COLOR_FG_DANGER_RED), None, "⚠");
            username_label.set_text(cx, tr_key(app_language, "room_screen.server_notice.username"));
            script_apply_eval!(cx, username_label, {
                draw_text +: {
                    color: (mod.widgets.COLOR_FG_DANGER_RED)
                }
            });
            item.view(cx, ids!(content.username_view.bot_badge)).set_visible(cx, false);
            new_drawn_status.profile_drawn = true;
        }
    }

    // If we've previously drawn the item content, skip all other steps.
    if used_cached_item && item_drawn_status.content_drawn && item_drawn_status.profile_drawn {
        return (item, new_drawn_status, false);
    }

    // Set the timestamp.
    if let Some(dt) = unix_time_millis_to_datetime(ts_millis) {
        // Name-only lookup: resolves `username_view.timestamp` on the full
        // Message template and `profile.timestamp` (gutter) on CondensedMessage.
        // INVARIANT: each Message-derived template must contain exactly ONE
        // widget named `timestamp`, or this lookup silently binds to the wrong one.
        item.timestamp(cx, ids!(timestamp)).set_date_time(cx, dt);
    }

    // Suppress "edited" indicator for actively streaming messages.
    let is_streaming = event_tl_item.event_id()
        .is_some_and(|eid| streaming_messages.contains_key(&eid.to_owned()));
    if msg_like_content.as_message().is_some_and(|m| m.is_edited()) && !is_streaming {
        item.edited_indicator(cx, ids!(profile.edited_indicator))
            .set_latest_edit(cx, event_tl_item);
    }

    #[cfg(feature = "tsp")] {
        use matrix_sdk::ruma::serde::Base64;
        use crate::tsp::{self, tsp_sign_indicator::{TspSignState, TspSignIndicatorWidgetRefExt}};

        if let Some(mut tsp_sig) = event_tl_item.latest_json()
            .and_then(|raw| raw.get_field::<serde_json::Value>("content").ok())
            .flatten()
            .and_then(|content_obj| content_obj.get("org.robius.tsp_signature").cloned())
            .and_then(|tsp_sig_value| serde_json::from_value::<Base64>(tsp_sig_value).ok())
            .map(|b64| b64.into_inner())
        {
            log!("Found event {:?} with TSP signature.", event_tl_item.event_id());
            let tsp_sign_state = if let Some(sender_vid) = tsp::tsp_state_ref().lock().unwrap()
                .get_verified_vid_for(event_tl_item.sender())
            {
                log!("Found verified VID for sender {}: \"{}\"", event_tl_item.sender(), sender_vid.identifier());
                tsp_sdk::crypto::verify(&*sender_vid, &mut tsp_sig).map_or(
                    TspSignState::WrongSignature,
                    |(msg, msg_type)| {
                        log!("TSP signature verified successfully!\n    Msg type: {msg_type:?}\n    Message: {:?} ({msg:X?})", std::str::from_utf8(msg));
                        TspSignState::Verified
                    }
                )
            } else {
                TspSignState::Unknown
            };

            log!("TSP signature state for event {:?} is {:?}", event_tl_item.event_id(), tsp_sign_state);
            item.tsp_sign_indicator(cx, ids!(profile.tsp_sign_indicator))
                .show_with_state(cx, tsp_sign_state);
        }
    }

    (item, new_drawn_status, !used_cached_item)
}

/// Draws the Html or plaintext body of the given Text or Notice message into the `message_content_widget`.
/// Also populates link previews if a link_preview_ref is provided.
/// Returns whether the text items were fully drawn.
fn populate_text_message_content(
    cx: &mut Cx,
    message_content_widget: &HtmlOrPlaintextRef,
    app_language: AppLanguage,
    body: &str,
    formatted_body: Option<&FormattedBody>,
    room_mention_room_id: Option<&OwnedRoomId>,
    link_preview_ref: Option<&mut LinkPreviewRef>,
    media_cache: Option<&mut MediaCache>,
    link_preview_cache: Option<&mut LinkPreviewCache>,
) -> bool {
    /// If this is a room mention, replace `@room` text in `html` with a pill
    /// link to the room so it renders as a red room pill with the room's avatar.
    fn apply_room_mention<'a>(html: Cow<'a, str>, room_id: Option<&OwnedRoomId>) -> Cow<'a, str> {
        if let Some(room_id) = room_id {
            if html.contains("@room") {
                return Cow::Owned(html.replace(
                    "@room",
                    &format!("<a href=\"https://matrix.to/#/{room_id}\">@room</a>"),
                ));
            }
        }
        html
    }

    // The message was HTML-formatted rich text.
    let mut links = Vec::new();
    if let Some(fb) = formatted_body.as_ref()
        .and_then(|fb| (fb.format == MessageFormat::Html).then_some(fb))
    {
        let linkified_html = utils::linkify_get_urls(
            utils::trim_start_html_whitespace(&fb.body),
            true,
            Some(&mut links),
        );
        let html = apply_room_mention(linkified_html, room_mention_room_id);
        message_content_widget.show_html(cx, html);
    }
    // The message was non-HTML plaintext.
    else {
        let linkified_html = utils::linkify_get_urls(body, false, Some(&mut links));
        let html = apply_room_mention(linkified_html, room_mention_room_id);
        match html {
            Cow::Owned(linkified_html) => message_content_widget.show_html(cx, &linkified_html),
            Cow::Borrowed(plaintext) => message_content_widget.show_plaintext(cx, plaintext),
        }
    };

    // Populate link previews if all required parameters are provided
    if let (Some(link_preview_ref), Some(media_cache), Some(link_preview_cache)) = 
        (link_preview_ref, media_cache, link_preview_cache)
    {
        link_preview_ref.populate_below_message(
            cx,
            &links,
            media_cache,
            link_preview_cache,
            &|cx, text_or_image_ref, image_info_source, original_source, body, media_cache| {
                populate_image_message_content(
                    cx,
                    text_or_image_ref,
                    None,
                    app_language,
                    image_info_source,
                    original_source,
                    body,
                    media_cache,
                )
            },
        )
    } else {
        true
    }
}

/// Renders a message's delivery state: a pill in the meta band, plus the reason
/// and the two recovery actions when the send is parked for good.
///
/// Every widget is written on every call, never only on the branch that needs
/// it. These item widgets are recycled by the PortalList, so a pill left visible
/// would reappear under an unrelated message.
fn populate_send_state(cx: &mut Cx, item: &WidgetRef, event_tl_item: &EventTimelineItem) {
    let state = MessageDeliveryState::from_item(event_tl_item);

    let pill = item.view(cx, ids!(content.message_action_bar.send_state_pill));
    let failure = item.view(cx, ids!(content.send_failure_section));

    let (pill_text, pill_bg, pill_fg) = match &state {
        Some(MessageDeliveryState::Sending) => (
            "Sending",
            crate::shared::design_tokens::RBX_NEUTRAL_BG,
            crate::shared::design_tokens::RBX_NEUTRAL_FG,
        ),
        Some(MessageDeliveryState::FailedRetrying) => (
            "Waiting to retry",
            crate::shared::design_tokens::RBX_WARNING_BG,
            crate::shared::design_tokens::RBX_WARNING_FG,
        ),
        Some(MessageDeliveryState::FailedWedged { .. }) => (
            "Not sent",
            crate::shared::design_tokens::RBX_DANGER_BG,
            crate::shared::design_tokens::RBX_DANGER_FG,
        ),
        None => ("", crate::shared::design_tokens::RBX_NEUTRAL_BG, crate::shared::design_tokens::RBX_NEUTRAL_FG),
    };

    pill.set_visible(cx, state.is_some());
    if state.is_some() {
        item.label(cx, ids!(content.message_action_bar.send_state_pill.send_state_label))
            .set_text(cx, pill_text);
        let mut pill_view = pill;
        script_apply_eval!(cx, pill_view, {
            draw_bg +: { color: #(pill_bg) }
        });
        let mut pill_label =
            item.label(cx, ids!(content.message_action_bar.send_state_pill.send_state_label));
        script_apply_eval!(cx, pill_label, {
            draw_text +: { color: #(pill_fg) }
        });
    }

    match &state {
        Some(MessageDeliveryState::FailedWedged { reason }) => {
            failure.set_visible(cx, true);
            item.label(cx, ids!(content.send_failure_section.send_failure_reason))
                .set_text(cx, reason);
        }
        _ => failure.set_visible(cx, false),
    }
}

/// Draws the given image message's content into the `message_content_widget`.
///
/// `animated_image_ref` is the optional `AnimatedImage` slot on the message
/// template. When the message's mimetype/filename identifies it as animated
/// (gif/apng/webp), that slot is made visible and populated instead of the
/// regular `TextOrImage`. `None` is passed by stickers and link previews,
/// which never animate.
///
/// Returns whether the image message content was fully drawn.
fn populate_image_message_content(
    cx: &mut Cx,
    text_or_image_ref: &TextOrImageRef,
    animated_image_ref: Option<&AnimatedImageRef>,
    app_language: AppLanguage,
    image_info_source: Option<Box<ImageInfo>>,
    original_source: MediaSource,
    body: &str,
    media_cache: &mut MediaCache,
) -> bool {
    // We don't use thumbnails, as their resolution is too low to be visually useful.
    // We also don't trust the provided mimetype, as it can be incorrect.
    let (mimetype, _width, _height) = image_info_source.as_ref()
        .map(|info| (info.mimetype.as_deref(), info.width, info.height))
        .unwrap_or_default();

    let is_animated_image = mimetype
        .map(utils::is_animated_image_mime)
        .unwrap_or_else(|| utils::is_animated_image_filename(body));
    if is_animated_image {
        if let Some(animated_image_ref) = animated_image_ref {
            text_or_image_ref.set_visible(cx, false);
            animated_image_ref.set_visible(cx, true);
            return animated_image_ref.populate_from_media_source(
                cx,
                original_source,
                body,
                media_cache,
            );
        }

        text_or_image_ref.show_text(
            cx,
            format!("{body}\n\nAnimated image messages require the animated image widget."),
        );
        return true;
    }

    if let Some(animated_image_ref) = animated_image_ref {
        animated_image_ref.set_visible(cx, false);
    }
    text_or_image_ref.set_visible(cx, true);

    // If we have a known mimetype and it's not a static image,
    // then show a message about it being unsupported (e.g., for animated gifs).
    if let Some(mime) = mimetype.as_ref() {
        if ImageFormat::from_mimetype(mime).is_none() {
            text_or_image_ref.show_text(
                cx,
                tr_fmt(app_language, "room_screen.image.unsupported_type", &[("body", body), ("mime", mime)]),
            );
            return true; // consider this as fully drawn
        }
    }

    let mut fully_drawn = false;

    let mut fetch_and_show_media_source = |cx: &mut Cx, media_source: MediaSource, image_info: Box<ImageInfo>| {
        match media_cache.try_get_media_or_fetch(&media_source, MEDIA_THUMBNAIL_FORMAT.into()) {
            (MediaCacheEntry::Loaded(data), _media_format) => {
                let show_image_result = text_or_image_ref.show_image(cx, Some(media_source), |cx, img| {
                    utils::load_png_or_jpg(&img, cx, &data)
                        .map(|()| img.size_in_pixels(cx).unwrap_or_default())
                });
                if let Err(e) = show_image_result {
                    let err_str = tr_fmt(app_language, "room_screen.image.failed_to_display", &[("body", body), ("error", &format!("{e:?}"))]);
                    error!("{err_str}");
                    text_or_image_ref.show_text(cx, &err_str);
                }

                // We're done drawing the image, so mark it as fully drawn.
                fully_drawn = true;
            }
            (MediaCacheEntry::Requested, _media_format) => {
                // If the image is being fetched, we try to show its blurhash.
                if let (Some(ref blurhash), Some(width), Some(height)) = (image_info.blurhash.clone(), image_info.width, image_info.height) {
                    let show_image_result = text_or_image_ref.show_image(cx, Some(media_source), |cx, img| {
                        let (Ok(width), Ok(height)) = (width.try_into(), height.try_into()) else {
                            return Err(image_cache::ImageError::EmptyData)
                        };
                        let (width, height): (u32, u32) = (width, height);
                        if width == 0 || height == 0 {
                            warning!("Image had an invalid aspect ratio (width or height of 0).");
                            return Err(image_cache::ImageError::EmptyData);
                        }
                        let aspect_ratio: f32 = width as f32 / height as f32;
                        // Cap the blurhash to a max size of 500 pixels in each dimension
                        // because the `blurhash::decode()` function can be rather expensive.
                        let (mut capped_width, mut capped_height) = (width, height);
                        if capped_height > BLURHASH_IMAGE_MAX_SIZE {
                            capped_height = BLURHASH_IMAGE_MAX_SIZE;
                            capped_width = (capped_height as f32 * aspect_ratio).floor() as u32;
                        }
                        if capped_width > BLURHASH_IMAGE_MAX_SIZE {
                            capped_width = BLURHASH_IMAGE_MAX_SIZE;
                            capped_height = (capped_width as f32 / aspect_ratio).floor() as u32;
                        }

                        match blurhash::decode(blurhash, capped_width, capped_height, 1.0) {
                            Ok(data) => {
                                ImageBuffer::new(&data, capped_width as usize, capped_height as usize).map(|img_buff| {
                                    let texture = Some(img_buff.into_new_texture(cx));
                                    img.set_texture(cx, texture);
                                    img.size_in_pixels(cx).unwrap_or_default()
                                })
                            }
                            Err(e) => {
                                error!("Failed to decode blurhash {e:?}");
                                Err(image_cache::ImageError::EmptyData)
                            }
                        }
                    });
                    if let Err(e) = show_image_result {
                        let err_str = tr_fmt(app_language, "room_screen.image.failed_to_display", &[("body", body), ("error", &format!("{e:?}"))]);
                        error!("{err_str}");
                        text_or_image_ref.show_text(cx, &err_str);
                    }
                }
                fully_drawn = false;
            }
            (MediaCacheEntry::Failed(_status_code), _media_format) => {
                if text_or_image_ref.view(cx, ids!(default_image_view)).visible() {
                    fully_drawn = true;
                    return;
                }
                text_or_image_ref
                    .show_text(cx, tr_fmt(app_language, "room_screen.image.failed_to_fetch", &[("body", body), ("mxc_uri", &format!("{:?}", media_source_mxc(&media_source)))]));
                // For now, we consider this as being "complete". In the future, we could support
                // retrying to fetch thumbnail of the image on a user click/tap.
                fully_drawn = true;
            }
        }
    };

    match image_info_source {
        Some(image_info) => {
            // Use the provided thumbnail URI if it exists; otherwise use the original URI.
            let media_source = image_info.thumbnail_source.clone()
                .unwrap_or(original_source);
            fetch_and_show_media_source(cx, media_source, image_info);
        }
        None => {
            text_or_image_ref.show_text(cx, tr_fmt(app_language, "room_screen.image.no_source_url", &[("body", body)]));
            fully_drawn = true;
        }
    }

    fully_drawn
}


/// Draws a file message's content into the given `message_content_widget`.
///
/// Returns whether the file message content was fully drawn.
///
/// File download is NOT triggered automatically during rendering.
/// The user must click the `mxc://` link in the rendered HTML to initiate
/// the download via the existing `RobrixHtmlLinkAction` handler.
fn populate_file_message_content(
    cx: &mut Cx,
    message_content_widget: &HtmlOrPlaintextRef,
    app_language: AppLanguage,
    file_content: &FileMessageEventContent,
    _media_cache: &mut MediaCache,
) -> bool {
    let filename = htmlize::escape_text(file_content.filename());
    let size = file_content
        .info
        .as_ref()
        .and_then(|info| info.size)
        .map(|bytes| format!("  ({})", ByteSize::b(bytes.into())))
        .unwrap_or_default();
    // Escape caption to prevent HTML injection from untrusted message content
    let caption = file_content.formatted_caption()
        .filter(|fb| fb.format == MessageFormat::Html)
        .map(|fb| format!("<br><i>{}</i>", fb.body))
        .or_else(|| file_content.caption().map(|c| format!("<br><i>{}</i>", htmlize::escape_text(c))))
        .unwrap_or_default();

    // Build a clickable mxc:// link so the user can explicitly trigger download.
    // The link is handled by `RobrixHtmlLinkAction` / `robius_open` in the room screen.
    let download_link = match &file_content.source {
        MediaSource::Plain(mxc_uri) => {
            format!(
                "<br>→ <a href=\"{}\">{}</a>",
                htmlize::escape_text(mxc_uri.as_str()),
                tr_key(app_language, "room_screen.file.download"),
            )
        }
        MediaSource::Encrypted(_) => {
            format!("<br>→ <i>{}</i>", tr_key(app_language, "room_screen.file.encrypted_not_supported"))
        }
    };

    message_content_widget.show_html(
        cx,
        format!("<b>{filename}</b>{size}{caption}{download_link}"),
    );
    true
}

/// Draws an audio message's content into the given message item.
///
/// Populates the embedded `AudioMessagePlayer` widget from the message's
/// `source` and also writes a textual summary into the html fallback for
/// accessibility / when playback is unavailable.
///
/// Returns whether the audio message content was fully drawn.
fn populate_audio_message_content(
    cx: &mut Cx,
    item: &WidgetRef,
    message_content_widget: &HtmlOrPlaintextRef,
    app_language: AppLanguage,
    audio: &AudioMessageEventContent,
    media_cache: &mut MediaCache,
) -> bool {
    // Populate the embedded inline audio player. The player handles
    // fetching, decoding and playback; we just hand it a summary +
    // source.
    let summary = summarize_audio_message(audio);
    item.audio_message_player(cx, ids!(content.audio_player))
        .populate_from_summary(cx, summary, audio.source.clone(), media_cache);

    // Display the file name, human-readable size, caption, and a button to download it.
    let filename = htmlize::escape_text(audio.filename());
    let (duration, mime, size) = audio
        .info
        .as_ref()
        .map(|info| (
            info.duration
                .map(|d| format!("  {:.2} sec,", d.as_secs_f64()))
                .unwrap_or_default(),
            info.mimetype
                .as_ref()
                .map(|m| format!("  {m},"))
                .unwrap_or_default(),
            info.size
                .map(|bytes| format!("  ({}),", ByteSize::b(bytes.into())))
                .unwrap_or_default(),
        ))
        .unwrap_or_default();
    let caption = audio.formatted_caption()
        .filter(|fb| fb.format == MessageFormat::Html)
        .map(|fb| format!("<br><i>{}</i>", fb.body))
        .or_else(|| audio.caption().map(|c| format!("<br><i>{c}</i>")))
        .unwrap_or_default();

    message_content_widget.show_html(
        cx,
        tr_fmt(app_language, "room_screen.audio.preview_html", &[
            ("filename", &filename),
            ("mime", mime.as_str()),
            ("duration", duration.as_str()),
            ("size", size.as_str()),
            ("caption", caption.as_str()),
        ]),
    );
    true
}


/// Draws a video message's content into the given message item.
///
/// Populates the embedded `VideoMessagePlayer` widget from the message's
/// `source` (with `info.thumbnail_source` as the poster) and also writes
/// a textual summary into the html fallback.
///
/// Returns whether the video message content was fully drawn.
fn populate_video_message_content(
    cx: &mut Cx,
    item: &WidgetRef,
    message_content_widget: &HtmlOrPlaintextRef,
    app_language: AppLanguage,
    video: &VideoMessageEventContent,
    media_cache: &mut MediaCache,
) -> bool {
    let summary = summarize_video_message(video);
    let poster_source = video.info.as_ref().and_then(|info| info.thumbnail_source.clone());
    item.video_message_player(cx, ids!(content.video_player))
        .populate_from_summary(cx, summary, video.source.clone(), poster_source, media_cache);

    // Display the file name, human-readable size, caption, and a button to download it.
    let filename = htmlize::escape_text(video.filename());
    let (duration, mime, size, dimensions) = video
        .info
        .as_ref()
        .map(|info| (
            info.duration
                .map(|d| format!("  {:.2} sec,", d.as_secs_f64()))
                .unwrap_or_default(),
            info.mimetype
                .as_ref()
                .map(|m| format!("  {m},"))
                .unwrap_or_default(),
            info.size
                .map(|bytes| format!("  ({}),", ByteSize::b(bytes.into())))
                .unwrap_or_default(),
            info.width.and_then(|width|
                info.height.map(|height| format!("  {width}x{height},"))
            ).unwrap_or_default(),
        ))
        .unwrap_or_default();
    let caption = video.formatted_caption()
        .filter(|fb| fb.format == MessageFormat::Html)
        .map(|fb| format!("<br><i>{}</i>", fb.body))
        .or_else(|| video.caption().map(|c| format!("<br><i>{c}</i>")))
        .unwrap_or_default();

    // TODO: add an video to play the video file

    message_content_widget.show_html(
        cx,
        tr_fmt(app_language, "room_screen.video.preview_html", &[
            ("filename", &filename),
            ("mime", mime.as_str()),
            ("duration", duration.as_str()),
            ("size", size.as_str()),
            ("dimensions", dimensions.as_str()),
            ("caption", caption.as_str()),
        ]),
    );
    true
}



/// Draws the given location message's content into the `message_content_widget`.
///
/// Returns whether the location message content was fully drawn.
fn populate_location_message_content(
    cx: &mut Cx,
    message_content_widget: &HtmlOrPlaintextRef,
    app_language: AppLanguage,
    location: &LocationMessageEventContent,
) -> bool {
    let coords = location.geo_uri
        .get(utils::GEO_URI_SCHEME.len() ..)
        .and_then(|s| {
            let mut iter = s.split(',');
            if let (Some(lat), Some(long)) = (iter.next(), iter.next()) {
                Some((lat, long))
            } else {
                None
            }
        });
    if let Some((lat, long)) = coords {
        let short_lat = lat.find('.').and_then(|dot| lat.get(..dot + 7)).unwrap_or(lat);
        let short_long = long.find('.').and_then(|dot| long.get(..dot + 7)).unwrap_or(long);
        let safe_lat = htmlize::escape_attribute(lat);
        let safe_long = htmlize::escape_attribute(long);
        let safe_geo_uri = htmlize::escape_attribute(&location.geo_uri);
        let safe_short_lat = htmlize::escape_text(short_lat);
        let safe_short_long = htmlize::escape_text(short_long);
        let html_body = format!(
            "{} <a href=\"{}\">{safe_short_lat},{safe_short_long}</a><br>\
            <ul>\
            <li><a href=\"https://www.openstreetmap.org/?mlat={safe_lat}&amp;mlon={safe_long}#map=15/{safe_lat}/{safe_long}\">{}</a></li>\
            <li><a href=\"https://www.google.com/maps/search/?api=1&amp;query={safe_lat},{safe_long}\">{}</a></li>\
            <li><a href=\"https://maps.apple.com/?ll={safe_lat},{safe_long}&amp;q={safe_lat},{safe_long}\">{}</a></li>\
            </ul>",
            tr_key(app_language, "room_screen.location.label"),
            safe_geo_uri,
            tr_key(app_language, "room_screen.location.open_osm"),
            tr_key(app_language, "room_screen.location.open_google_maps"),
            tr_key(app_language, "room_screen.location.open_apple_maps"),
        );
        message_content_widget.show_html(cx, html_body);
    } else {
        let escaped_body = htmlize::escape_text(&location.body);
        message_content_widget.show_html(
            cx,
            tr_fmt(app_language, "room_screen.location.invalid_html", &[
                ("body", &escaped_body),
            ])
        );
    }

    // Currently we do not fetch location thumbnail previews, so we consider this as fully drawn.
    // In the future, when we do support this, we'll return false until the thumbnail is fetched,
    // at which point we can return true.
    true
}


/// Draws the given redacted message's content into the `message_content_widget`.
///
/// Returns whether the redacted message content was fully drawn.
fn populate_redacted_message_content(
    cx: &mut Cx,
    message_content_widget: &HtmlOrPlaintextRef,
    app_language: AppLanguage,
    event_tl_item: &EventTimelineItem,
    room_id: &OwnedRoomId,
) -> bool {
    let fully_drawn: bool;
    let mut redactor_id_and_reason = None;
    if let Some(redacted_msg) = event_tl_item.latest_json() {
        if let Ok(AnySyncTimelineEvent::MessageLike(
            AnySyncMessageLikeEvent::RoomMessage(
                SyncMessageLikeEvent::Redacted(redaction)
            )
        )) = redacted_msg.deserialize() {
            if let Ok(redacted_because) = redaction.unsigned.redacted_because.deserialize() {
                redactor_id_and_reason = Some((
                    redacted_because.sender,
                    redacted_because.content.reason,
                ));
            }
        }
    }

    let html = if let Some((redactor, reason)) = redactor_id_and_reason {
        if redactor == event_tl_item.sender() {
            fully_drawn = true;
            match reason {
                Some(r) => {
                    let escaped_reason = htmlize::escape_text(r);
                    tr_fmt(app_language, "room_screen.redacted.self_with_reason", &[
                        ("reason", &escaped_reason),
                    ])
                }
                None => tr_key(app_language, "room_screen.redacted.self").to_string(),
            }
        } else {
            // Try to get the displayable name of the user who redacted this message.
            let redactor_name = user_profile_cache::get_user_display_name_for_room(
                cx,
                redactor.clone(),
                Some(room_id),
                true,
            );
            fully_drawn = redactor_name.was_found();
            let redactor_name_esc = htmlize::escape_text(redactor_name.as_deref().unwrap_or(redactor.as_str()));
            match reason {
                Some(r) => {
                    let escaped_reason = htmlize::escape_text(r);
                    tr_fmt(app_language, "room_screen.redacted.other_with_reason", &[
                        ("redactor", &redactor_name_esc),
                        ("reason", &escaped_reason),
                    ])
                }
                None => tr_fmt(app_language, "room_screen.redacted.other", &[
                    ("redactor", &redactor_name_esc),
                ]),
            }
        }
    } else {
        fully_drawn = true;
        tr_key(app_language, "room_screen.redacted.generic").to_string()
    };
    message_content_widget.show_html(cx, html);
    fully_drawn
}


/// Draws a ReplyPreview above a message if it was in-reply to another message.
///
/// ## Arguments
/// * `replied_to_message_view`: the destination `RepliedToMessage` view that will be populated.
/// * `timeline_kind`: the [`TimelineKind`] of the timeline that is being drawn.
/// * `in_reply_to`: if `Some`, the details that will be used to populate the `replied_to_message_view`.
///   If `None`, this function will mark it as non-visible and consider it fully drawn.
/// * `message_event_id`: the [`EventId`] of the message that is the reply itself (the response).
///   This is needed to fetch the details of the replied-to message (if not yet available).
///
/// Returns whether the in-reply-to information was available and fully drawn,
/// i.e., whether it can be considered cached and not needing to be redrawn later.
fn draw_replied_to_message(
    cx: &mut Cx2d,
    replied_to_message_view: &ViewRef,
    timeline_kind: &TimelineKind,
    app_language: AppLanguage,
    in_reply_to: Option<&InReplyToDetails>,
    message_event_id: Option<&EventId>,
) -> bool {
    let fully_drawn: bool;
    let show_reply: bool;

    if let Some(in_reply_to_details) = in_reply_to {
        show_reply = true;
        match &in_reply_to_details.event {
            TimelineDetails::Ready(replied_to_event) => {
                let (in_reply_to_username, is_avatar_fully_drawn) =
                    replied_to_message_view
                        .avatar(cx, ids!(replied_to_message_content.reply_preview_avatar))
                        .set_avatar_and_get_username(
                            cx,
                            timeline_kind,
                            &replied_to_event.sender,
                            Some(&replied_to_event.sender_profile),
                            Some(in_reply_to_details.event_id.as_ref()),
                            true,
                        );

                fully_drawn = is_avatar_fully_drawn;

                replied_to_message_view
                    .label(cx, ids!(replied_to_message_content.reply_preview_username))
                    .set_text(cx, in_reply_to_username.as_str());
                let msg_body = replied_to_message_view.html_or_plaintext(cx, ids!(reply_preview_body));
                populate_preview_of_timeline_item(
                    cx,
                    &msg_body,
                    app_language,
                    &replied_to_event.content,
                    &replied_to_event.sender,
                    &in_reply_to_username,
                );
            }
            TimelineDetails::Error(_e) => {
                fully_drawn = true;
                replied_to_message_view
                    .label(cx, ids!(replied_to_message_content.reply_preview_username))
                    .set_text(cx, tr_key(app_language, "room_screen.reply_preview.error_username"));
                replied_to_message_view
                    .avatar(cx, ids!(replied_to_message_content.reply_preview_avatar))
                    .show_text(cx, None, None, "?");
                replied_to_message_view
                    .html_or_plaintext(cx, ids!(replied_to_message_content.reply_preview_body))
                    .show_plaintext(cx, tr_key(app_language, "room_screen.reply_preview.error_event"));
            }
            td @ TimelineDetails::Pending | td @ TimelineDetails::Unavailable => {
                // We don't have the replied-to message yet, so we can't fully draw the preview.
                fully_drawn = false;
                replied_to_message_view
                    .label(cx, ids!(replied_to_message_content.reply_preview_username))
                    .set_text(cx, tr_key(app_language, "room_screen.reply_preview.loading_username"));
                replied_to_message_view
                    .avatar(cx, ids!(replied_to_message_content.reply_preview_avatar))
                    .show_text(cx, None, None, "?");
                replied_to_message_view
                    .html_or_plaintext(cx, ids!(replied_to_message_content.reply_preview_body))
                    .show_plaintext(cx, tr_key(app_language, "room_screen.reply_preview.loading_event"));

                // Confusingly, we need to fetch the details of the `message` (the event that is the reply),
                // not the details of the original event that this `message` is replying to.
                if matches!(td, TimelineDetails::Unavailable) {
                    if let Some(event_id) = message_event_id {
                        submit_async_request(MatrixRequest::FetchDetailsForEvent {
                            timeline_kind: timeline_kind.clone(),
                            event_id: event_id.to_owned(),
                        });
                    }
                }
            }
        }
    } else {
        // This message was not in reply to another message, so we don't need to show a reply.
        show_reply = false;
        fully_drawn = true;
    }

    replied_to_message_view.set_visible(cx, show_reply);
    fully_drawn
}

/// Draws a one-line thread summary at the bottom of a message if it is the root of a thread.
///
/// Returns whether the thread summary information was available and fully drawn,
/// i.e., whether it can be considered cached and not needing to be redrawn later.
fn populate_thread_root_summary(
    cx: &mut Cx2d,
    item: &WidgetRef,
    timeline_item_index: usize,
    timeline_kind: &TimelineKind,
    app_language: AppLanguage,
    msg_like_content: &MsgLikeContent,
    event_tl_item: &EventTimelineItem,
    fetched_thread_summaries: &HashMap<OwnedEventId, FetchedThreadSummary>,
    pending_thread_summary_fetches: &mut HashSet<OwnedEventId>,
) -> bool {
    let thread_summary_view = item.view(cx, ids!(thread_root_summary));
    thread_summary_view.set_visible(cx, false); // hide by default
    let fully_drawn: bool;

    if matches!(timeline_kind, TimelineKind::Thread { .. }) {
        // If we're already drawing a message in a thread-focused timeline,
        // it doesn't make sense to show a redundant thread summary.
        fully_drawn = true;
        return fully_drawn;
    }

    let Some(thread_summary) = msg_like_content.thread_summary.as_ref() else {
        // consider this as fully drawn since there's no thread summary to show.
        fully_drawn = true;
        return fully_drawn;
    };

    // Here, we actually need to show the thread summary.
    thread_summary_view.set_visible(cx, true);
    let local_num_replies = thread_summary.num_replies;
    let thread_root_event_id = event_tl_item.event_id().map(|id| id.to_owned());
    let fetched_summary = thread_root_event_id
        .as_ref()
        .and_then(|root_id| fetched_thread_summaries.get(root_id));
    let replies_count = fetched_summary
        .map(|f| f.num_replies)
        .unwrap_or(local_num_replies);

    let latest_preview: Cow<str> = match &thread_summary.latest_event {
        TimelineDetails::Ready(embedded_event) => {
            fully_drawn = true;
            let sender_username = match &embedded_event.sender_profile {
                TimelineDetails::Ready(profile) => profile
                    .display_name
                    .as_deref()
                    .unwrap_or(embedded_event.sender.as_str()),
                _ => embedded_event.sender.as_str(),
            };
            let preview = text_preview_of_timeline_item(
                &embedded_event.content,
                &embedded_event.sender,
                sender_username,
            ).format_with(sender_username, true);
            match utils::replace_linebreaks_separators(&preview, true) {
                Cow::Borrowed(_) => Cow::Owned(preview),
                Cow::Owned(replaced) => Cow::Owned(replaced),
            }
        }
        td @ TimelineDetails::Pending | td @ TimelineDetails::Unavailable => {
            fully_drawn = true;
            if td.is_unavailable()
                && let Some(thread_root_event_id) = thread_root_event_id.clone()
            {
                let needs_refresh = fetched_summary
                    .is_none_or(|fs| fs.latest_reply_preview_text.is_none());
                if needs_refresh && pending_thread_summary_fetches.insert(thread_root_event_id.clone()) {
                    submit_async_request(MatrixRequest::FetchThreadSummaryDetails {
                        timeline_kind: timeline_kind.clone(),
                        thread_root_event_id,
                        timeline_item_index,
                    });
                }
            }
            fetched_summary.and_then(|fs| fs.latest_reply_preview_text.as_deref())
                .unwrap_or(tr_key(app_language, "room_screen.thread_summary.loading_latest_reply"))
                .into()
        }
        TimelineDetails::Error(_) => {
            fully_drawn = true; // consider this fully drawn since there's no point retrying.
            tr_key(app_language, "room_screen.thread_summary.error_latest_reply").into()
        }
    };

    let replies_count_text = match replies_count {
        1 => Cow::Borrowed(tr_key(app_language, "room_screen.thread_summary.one_reply")),
        n => Cow::Owned(tr_fmt(app_language, "room_screen.thread_summary.n_replies", &[("n", &n.to_string())]))
    };
    item.label(cx, ids!(thread_summary_count))
        .set_text(cx, &replies_count_text);
    item.html(cx, ids!(thread_summary_latest))
        .set_text(cx, &latest_preview);
    fully_drawn
}

/// Generates a rich HTML text preview of the given `timeline_item_content`
/// and populates the given `widget_out` with that content.
pub fn populate_preview_of_timeline_item(
    cx: &mut Cx,
    widget_out: &HtmlOrPlaintextRef,
    app_language: AppLanguage,
    timeline_item_content: &TimelineItemContent,
    sender_user_id: &UserId,
    sender_username: &str,
) {
    if let Some(m) = timeline_item_content.as_message() {
        match m.msgtype() {
            MessageType::Text(TextMessageEventContent { body, formatted, .. })
            | MessageType::Notice(NoticeMessageEventContent { body, formatted, .. }) => {
                let _ = populate_text_message_content(cx, widget_out, app_language, body, formatted.as_ref(), None, None, None, None);
                return;
            }
            _ => { } // fall through to the general case for all timeline items below.
        }
    }
    let html = text_preview_of_timeline_item(
        timeline_item_content,
        sender_user_id,
        sender_username,
    ).format_with(sender_username, true);
    widget_out.show_html(cx, html);
}



/// Actions related to invites within a room.
///
/// These are NOT widget actions, just regular actions.
#[derive(Debug)]
pub enum InviteAction {
    /// Show a confirmation modal for sending an invite.
    ///
    /// The content is wrapped in a `RefCell` to ensure that only one entity handles it
    /// and that that one entity can take ownership of the content object,
    /// which avoids having to clone it.
    ShowInviteConfirmationModal(RefCell<Option<ConfirmationModalContent>>),
    /// Announces that an invite request was just submitted for `user_id` in
    /// `room_id`, so every RoomScreen showing that room records it in
    /// `pending_invited_users` and thus owns the resulting
    /// [`InviteResultAction`] feedback. Emitted by invite initiators that run
    /// in closures (knock-approve modal, failed-invite Retry) and by the
    /// invite modal; the `/invitebot` picker instead records pending directly
    /// in its widget-addressed handler.
    InviteUserRequested {
        room_id: OwnedRoomId,
        user_id: OwnedUserId,
    },
}

/// The result of inviting a user to a room.
///
#[derive(Debug)]
pub enum InviteResultAction {
    /// The invite was sent successfully.
    ///
    /// This action is posted in response to the [`MatrixRequest::InviteUser`] request.
    Sent {
        room_id: OwnedRoomId,
        user_id: OwnedUserId,
    },
    /// The invite failed to be sent.
    ///
    /// This action is posted in response to the [`MatrixRequest::InviteUser`] request.
    Failed {
        room_id: OwnedRoomId,
        user_id: OwnedUserId,
        error: matrix_sdk::Error,
    },
}

/// The result of reporting a room.
#[derive(Debug)]
pub enum ReportRoomResultAction {
    Sent {
        room_id: OwnedRoomId,
    },
    Failed {
        room_id: OwnedRoomId,
        error: matrix_sdk::Error,
    },
}

#[derive(Debug)]
pub enum ActionResponseResultAction {
    Sent {
        room_id: OwnedRoomId,
        source_event_id: OwnedEventId,
    },
    Failed {
        room_id: OwnedRoomId,
        source_event_id: OwnedEventId,
        error: String,
    },
}

#[derive(Clone, Default, Debug)]
pub enum MessageAction {
    /// The user clicked the "react" button on a message
    /// and wants to send the given `reaction` to that message.
    React {
        details: MessageDetails,
        reaction: String,
    },
    /// The user clicked the "reply" button on a message.
    Reply(MessageDetails),
    /// The user clicked the "edit" button on a message.
    Edit(MessageDetails),
    /// The user requested to edit their latest message in this room.
    EditLatest,
    /// The user submitted a new local message and the timeline should follow the live tail.
    MessageSubmittedLocally,
    /// The user clicked the "pin" button on a message.
    Pin(MessageDetails),
    /// The user clicked the "unpin" button on a message.
    Unpin(MessageDetails),
    /// The user clicked the "copy text" button on a message.
    CopyText(MessageDetails),
    /// The user clicked the "copy HTML" button on a message.
    CopyHtml(MessageDetails),
    /// The user clicked the "copy link" button on a message.
    CopyLink(MessageDetails),
    /// The user clicked the "forward message" button on a message.
    Forward(MessageDetails),
    /// The user clicked the "view source" button on a message.
    ViewSource(MessageDetails),
    /// The user clicked the "jump to related" button on a message,
    /// indicating that they want to auto-scroll back to the related message,
    /// e.g., a replied-to message.
    JumpToRelated(MessageDetails),
    /// The user clicked the thread summary on a thread-root message.
    OpenThread(OwnedEventId),
    /// The user requested to jump to a specific event in this room.
    JumpToEvent(OwnedEventId),
    /// The user clicked the "delete" button on a message.
    #[doc(alias("delete"))]
    Redact {
        details: MessageDetails,
        reason: Option<String>,
    },

    // /// The user clicked the "report" button on a message.
    // Report(MessageDetails),

    /// The user clicked the "Download" button on a media/file message.
    DownloadAttachment(DownloadableAttachment),
    /// The user clicked "Cancel" on an in-progress attachment download.
    CancelDownload(OwnedMxcUri),
    /// The message at the given item index in the timeline should be highlighted.
    HighlightMessage(usize),
    /// The user requested that we show a context menu with actions
    /// that can be performed on a given message.
    OpenMessageContextMenu {
        details: MessageDetails,
        /// The absolute position where we should show the context menu,
        /// in which the (0,0) origin coordinate is the top left corner of the app window.
        abs_pos: DVec2,
        opening_gesture: ContextMenuOpenGesture,
    },
    ToggleTranslationLangPopup {
        button_rect: Rect,
    },
    /// The user requested opening the message action bar
    ActionBarOpen {
        /// At the given timeline item index
        item_id: usize,
        /// The message rect, so the action bar can be positioned relative to it
        message_rect: Rect,
    },
    /// The user requested closing the message action bar
    ActionBarClose,
    /// The user requested toggling the in-room app service quick actions card.
    ToggleAppServiceActions,
    ShowThreadsPane,
    ShowRoomInfoPane,
    #[default]
    None,
}

impl ActionDefaultRef for MessageAction {
    fn default_ref() -> &'static Self {
        static DEFAULT: MessageAction = MessageAction::None;
        &DEFAULT
    }
}

/// A widget representing a single message of any kind within a room timeline.
#[derive(Script, ScriptHook, Widget, Animator)]
pub struct Message {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    #[apply_default] animator: Animator,

    #[rust] details: Option<MessageDetails>,
    /// Set on file/image/audio/video messages so the download button knows
    /// what to save when the user clicks it. `None` for plain text messages,
    /// which hide the download button entirely.
    #[rust] download_info: Option<DownloadableAttachment>,
    /// Cached so `set_data` can reset_hover only on the button that just
    /// transitioned into visibility, not on every redraw.
    #[rust] download_state: DownloadDisplayState,
    /// Whether the meta band's copy button is currently shown. Tracked so
    /// `set_data` only touches the widget when the value flips, and so
    /// `handle_event` can skip the per-Actions `clicked()` lookup entirely
    /// while the button is hidden.
    #[rust] show_copy_button: bool,
    /// The meta band's model-metadata line currently displayed (None = hidden).
    /// Tracked so recycled items only touch the label when the text changes.
    #[rust] band_metadata: Option<String>,
}

impl Widget for Message {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.animator_handle_event(cx, event).must_redraw() {
            self.redraw(cx);
        }

        if !self.animator.is_track_animating(id!(highlight))
            && self.animator_in_state(cx, ids!(highlight.on))
        {
            self.animator_play(cx, ids!(highlight.off));
        }

        let Some(details) = self.details.clone() else { return };

        // We first handle a click on the replied-to message preview, if present,
        // because we don't want any widgets within the replied-to message to be
        // clickable or otherwise interactive.
        match event.hits(cx, self.view(cx, ids!(replied_to_message)).area()) {
            Hit::FingerDown(fe) if fe.device.mouse_button().is_some_and(|b| b.is_secondary()) => {
                cx.widget_action(
                    details.room_screen_widget_uid,
                    MessageAction::OpenMessageContextMenu {
                        details: details.clone(),
                        abs_pos: fe.abs,
                        opening_gesture: ContextMenuOpenGesture::from_finger_down(&fe),
                    }
                );
            }
            Hit::FingerDown(_) => {}
            Hit::FingerLongPress(lp) => {
                cx.widget_action(
                    details.room_screen_widget_uid, 
                    MessageAction::OpenMessageContextMenu {
                        details: details.clone(),
                        abs_pos: lp.abs,
                        opening_gesture: ContextMenuOpenGesture::from_long_press(&lp),
                    }
                );
            }
            // If the hit occurred on the replied-to message preview, jump to it.
            Hit::FingerUp(fe) if fe.is_over && fe.is_primary_hit() && fe.was_tap() => {
                cx.widget_action(
                    details.room_screen_widget_uid, 
                    MessageAction::JumpToRelated(details.clone()),
                );
            }
            _ => { }
        }

        // Handle clicks on the thread summary shown beneath a thread-root message.
        if let Some(thread_root_event_id) = details.thread_root_event_id.as_ref() {
            let thread_root_summary = self.view(cx, ids!(thread_root_summary));
            let apply_hover = |cx: &mut Cx, bg_color: Vec4| {
                let mut thread_root_summary_ref = thread_root_summary.clone();
                script_apply_eval!(cx, thread_root_summary_ref, {
                    draw_bg.color: #(bg_color)
                });
            };
            match event.hits(cx, thread_root_summary.area()) {
                Hit::FingerDown(fe) => {
                    apply_hover(cx, COLOR_THREAD_SUMMARY_BG_HOVER);
                    if fe.device.mouse_button().is_some_and(|b| b.is_secondary()) {
                        cx.widget_action(
                            details.room_screen_widget_uid, 
                            MessageAction::OpenMessageContextMenu {
                                details: details.clone(),
                                abs_pos: fe.abs,
                                opening_gesture: ContextMenuOpenGesture::from_finger_down(&fe),
                            }
                        );
                    }
                }
                Hit::FingerHoverIn(_) => {
                    apply_hover(cx, COLOR_THREAD_SUMMARY_BG_HOVER);
                }
                Hit::FingerHoverOut(_) => {
                    apply_hover(cx, COLOR_THREAD_SUMMARY_BG);
                }
                Hit::FingerLongPress(lp) => {
                    cx.widget_action(
                        details.room_screen_widget_uid, 
                        MessageAction::OpenMessageContextMenu {
                            details: details.clone(),
                            abs_pos: lp.abs,
                            opening_gesture: ContextMenuOpenGesture::from_long_press(&lp),
                        }
                    );
                }
                Hit::FingerUp(fe) => {
                    apply_hover(cx, COLOR_THREAD_SUMMARY_BG);
                    if fe.is_over && fe.is_primary_hit() && fe.was_tap() {
                        cx.widget_action(
                            details.room_screen_widget_uid, 
                            MessageAction::OpenThread(thread_root_event_id.clone()),
                        );
                    }
                }
                _ => { }
            }
        }

        // Next, we forward the event to the child view such that it has the chance
        // to handle it before the Message widget handles it.
        // This ensures that events like right-clicking/long-pressing a reaction button
        // or a link within a message will be treated as an action upon that child view
        // rather than an action upon the message itself.
        self.view.handle_event(cx, event, scope);

        // Finally, handle any hits on the rest of the message body itself.
        let message_view_area = self.view.area();
        match event.hits(cx, message_view_area) {
            Hit::FingerDown(fe) => {
                cx.set_key_focus(message_view_area);
                // A right click means we should display the context menu.
                if fe.device.mouse_button().is_some_and(|b| b.is_secondary()) {
                    cx.widget_action(
                        details.room_screen_widget_uid, 
                        MessageAction::OpenMessageContextMenu {
                            details: details.clone(),
                            abs_pos: fe.abs,
                            opening_gesture: ContextMenuOpenGesture::from_finger_down(&fe),
                        }
                    );
                }
            }
            Hit::FingerLongPress(lp) => {
                cx.widget_action(
                    details.room_screen_widget_uid, 
                    MessageAction::OpenMessageContextMenu {
                        details: details.clone(),
                        abs_pos: lp.abs,
                        opening_gesture: ContextMenuOpenGesture::from_long_press(&lp),
                    }
                );
            }
            Hit::FingerHoverIn(..) => {
                self.animator_play(cx, ids!(hover.on));
                // TODO: here, show the "action bar" buttons upon hover-in
            }
            Hit::FingerHoverOut(_fho) => {
                self.animator_play(cx, ids!(hover.off));
                // TODO: here, hide the "action bar" buttons upon hover-out
            }
            _ => { }
        }

        if let Event::Actions(actions) = event {
            if let Some(info) = self.download_info.as_ref()
                && self.view.button(cx, ids!(content.download_section.download_button)).clicked(actions)
            {
                cx.widget_action(
                    details.room_screen_widget_uid,
                    MessageAction::DownloadAttachment(info.clone()),
                );
            }
            if let Some(info) = self.download_info.as_ref()
                && self.view.button(cx, ids!(content.download_section.downloading_view.cancel_button)).clicked(actions)
            {
                cx.widget_action(
                    details.room_screen_widget_uid,
                    MessageAction::CancelDownload(media_source_mxc(&info.media_source).clone()),
                );
            }
            if self.show_copy_button
                && self.view.button(cx, ids!(content.message_action_bar.copy_button)).clicked(actions)
            {
                cx.widget_action(
                    details.room_screen_widget_uid,
                    MessageAction::CopyText(details.clone()),
                );
            }
            for action in actions {
                match action.as_widget_action().widget_uid_eq(details.room_screen_widget_uid).cast_ref() {
                    MessageAction::HighlightMessage(id) if id == &details.item_id => {
                        self.animator_play(cx, ids!(highlight.on));
                        self.redraw(cx);
                    }
                    _ => {}
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if self.details.as_ref().is_some_and(|d| d.should_be_highlighted) {
            script_apply_eval!(cx, self, {
                draw_bg +: {
                    color: #ffffd1,
                    mentions_bar_color: #ffd54f
                }
            });
        }

        self.view.draw_walk(cx, scope, walk)
    }
}

impl Message {
    fn set_data(
        &mut self,
        cx: &mut Cx,
        details: MessageDetails,
        download_info: Option<DownloadableAttachment>,
        download_state: DownloadDisplayState,
        show_copy_button: bool,
    ) {
        let prev_section_visible = self.download_info.is_some();
        let prev_state = self.download_state;

        self.details = Some(details);
        self.download_info = download_info;
        let section_visible = self.download_info.is_some();
        self.view.view(cx, ids!(content.download_section))
            .set_visible(cx, section_visible);
        if self.show_copy_button != show_copy_button {
            let copy_button = self.view.button(cx, ids!(content.message_action_bar.copy_button));
            copy_button.set_visible(cx, show_copy_button);
            if show_copy_button {
                copy_button.reset_hover(cx);
            }
            self.show_copy_button = show_copy_button;
        }
        if let Some(info) = self.download_info.as_ref() {
            let download_button = self.view.button(cx, ids!(content.download_section.download_button));
            let downloading_view = self.view.view(cx, ids!(content.download_section.downloading_view));
            let cancel_button = self.view.button(cx, ids!(content.download_section.downloading_view.cancel_button));
            let success_button = self.view.button(cx, ids!(content.download_section.success_button));
            let failure_button = self.view.button(cx, ids!(content.download_section.failure_button));
            download_button.set_text(cx, info.kind.button_text());
            download_button.set_visible(cx, matches!(download_state, DownloadDisplayState::Idle));
            downloading_view.set_visible(cx, matches!(download_state, DownloadDisplayState::InProgress));
            success_button.set_visible(cx, matches!(download_state, DownloadDisplayState::Succeeded));
            failure_button.set_visible(cx, matches!(download_state, DownloadDisplayState::Failed));
            let newly_visible = !prev_section_visible || prev_state != download_state;
            if newly_visible {
                match download_state {
                    DownloadDisplayState::Idle => download_button.reset_hover(cx),
                    DownloadDisplayState::InProgress => cancel_button.reset_hover(cx),
                    DownloadDisplayState::Succeeded => success_button.reset_hover(cx),
                    DownloadDisplayState::Failed => failure_button.reset_hover(cx),
                }
            }
        }
        self.download_state = download_state;
    }

    /// Sets the meta band's model-metadata line (None hides it).
    ///
    /// Only touches the label widget when the value actually changes, so
    /// recycled PortalList items and the timeline-majority case (human
    /// messages, metadata None → None) cost nothing beyond the comparison.
    fn set_band_metadata(&mut self, cx: &mut Cx, band_metadata: Option<String>) {
        if self.band_metadata == band_metadata {
            return;
        }
        let label = self.view.label(cx, ids!(content.message_action_bar.metadata_label));
        label.set_visible(cx, band_metadata.is_some());
        label.set_text(cx, band_metadata.as_deref().unwrap_or(""));
        self.band_metadata = band_metadata;
    }
}

impl MessageRef {
    fn set_data(
        &self,
        cx: &mut Cx,
        details: MessageDetails,
        download_info: Option<DownloadableAttachment>,
        download_state: DownloadDisplayState,
        show_copy_button: bool,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_data(cx, details, download_info, download_state, show_copy_button);
    }

    fn set_band_metadata(&self, cx: &mut Cx, band_metadata: Option<String>) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_band_metadata(cx, band_metadata);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invite_result_belongs_only_to_pending_screen() {
        let invited_user = OwnedUserId::try_from("@octos:example.org").unwrap();
        let other_user = OwnedUserId::try_from("@hermes:example.org").unwrap();
        let pending = HashSet::from([invited_user.clone()]);

        assert!(invite_result_belongs_to_room_screen(&pending, &invited_user));
        assert!(!invite_result_belongs_to_room_screen(&pending, &other_user));
        assert!(!invite_result_belongs_to_room_screen(&HashSet::new(), &invited_user));
    }

    #[test]
    fn test_notice_offset_actions() {
        assert_eq!(tl_idx_from_item_id(0, true), None);
        assert_eq!(tl_idx_from_item_id(1, true), Some(0));
        assert_eq!(tl_idx_from_item_id(7, true), Some(6));
        assert_eq!(tl_idx_from_item_id(7, false), Some(7));
        assert_eq!(item_id_from_tl_idx(0, true), 1);
        assert_eq!(item_id_from_tl_idx(6, true), 7);
        assert_eq!(item_id_from_tl_idx(6, false), 6);
    }

    #[test]
    fn translation_lang_popup_abs_pos_prefers_above_button() {
        let button_rect = Rect {
            pos: dvec2(48.0, 680.0),
            size: dvec2(32.0, 32.0),
        };
        let container_rect = Rect {
            pos: dvec2(0.0, 0.0),
            size: dvec2(1280.0, 760.0),
        };

        let popup_pos = compute_translation_lang_popup_abs_pos(button_rect, container_rect);

        assert!(popup_pos.y < button_rect.pos.y);
        assert!(popup_pos.y >= TRANSLATION_LANG_POPUP_MARGIN);
        assert!(popup_pos.x >= TRANSLATION_LANG_POPUP_MARGIN);
    }

    #[test]
    fn translation_lang_popup_abs_pos_falls_below_when_top_space_is_insufficient() {
        let button_rect = Rect {
            pos: dvec2(48.0, 20.0),
            size: dvec2(32.0, 32.0),
        };
        let container_rect = Rect {
            pos: dvec2(0.0, 0.0),
            size: dvec2(1280.0, 760.0),
        };

        let popup_pos = compute_translation_lang_popup_abs_pos(button_rect, container_rect);

        assert!(popup_pos.y > button_rect.pos.y);
        assert!(popup_pos.y >= TRANSLATION_LANG_POPUP_MARGIN);
    }

    #[test]
    fn translation_lang_popup_abs_pos_clamps_to_room_screen_right_edge() {
        let button_rect = Rect {
            pos: dvec2(1240.0, 680.0),
            size: dvec2(32.0, 32.0),
        };
        let container_rect = Rect {
            pos: dvec2(0.0, 0.0),
            size: dvec2(1280.0, 760.0),
        };

        let popup_pos = compute_translation_lang_popup_abs_pos(button_rect, container_rect);

        assert_eq!(
            popup_pos.x + TRANSLATION_LANG_POPUP_WIDTH,
            container_rect.size.x - TRANSLATION_LANG_POPUP_MARGIN
        );
    }

    #[test]
    fn center_username_row_aligns_with_avatar_center() {
        assert_eq!(
            message_profile_avatar_center_y(),
            message_username_row_center_y(),
        );
    }

    #[test]
    fn center_bot_badge_aligns_with_username_row_center() {
        assert_eq!(
            message_username_row_center_y(),
            bot_badge_center_y_within_username_row(),
        );
    }

    #[test]
    fn bot_badge_text_is_centered_within_badge() {
        assert!(bot_badge_label_center_y() < (BOT_BADGE_HEIGHT * 0.5));
    }

    #[test]
    fn test_room_members_fetch_updates_rooms_list_member_ids() {
        let src = include_str!("mod.rs");

        assert!(src.contains("TimelineUpdate::RoomMembersListFetched"));
        assert!(src.contains("RoomsListUpdate::UpdateRoomMemberUserIds"));
        assert!(src.contains("member.user_id().to_owned()"));
    }

}
