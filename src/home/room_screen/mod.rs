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
    app::{AppState, AppStateAction, BotSettingsState, ConfirmDeleteAction, PositiveConfirmationModalAction, SelectedRoom}, avatar_cache, event_preview::{plaintext_body_of_timeline_item, text_preview_of_encrypted_message, text_preview_of_member_profile_change, text_preview_of_other_message_like, text_preview_of_other_state, text_preview_of_room_membership_change, text_preview_of_timeline_item}, home::{bot_binding_modal::BotBindingModalAction, create_bot_modal::{CreateBotModalAction, CreateBotModalWidgetExt}, delete_bot_modal::{DeleteBotModalAction, DeleteBotModalWidgetExt}, edited_indicator::EditedIndicatorWidgetRefExt, encryption_notice::{EncryptionNoticeWidgetRefExt, first_other_member_display_name}, invite_modal::InviteModalAction, link_preview::{LinkPreviewCache, LinkPreviewRef, LinkPreviewWidgetRefExt}, loading_pane::{LoadingPaneState, LoadingPaneWidgetExt}, room_image_viewer::{get_image_name_and_filesize, populate_matrix_image_modal}, rooms_list::{RoomsListAction, RoomsListRef}, rooms_list_header::RoomsListHeaderAction, tombstone_footer::SuccessorRoomDetails}, i18n::{AppLanguage, tr_fmt, tr_key}, media_cache::{MediaCache, MediaCacheEntry}, profile::{
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
mod interactions;
mod message;
mod octos_actions;
mod populate;
mod report_modal;
mod room_info_pane;
mod search;
mod small_state;
mod state;
mod threads_pane;
mod updates;

pub use bot_admin::*;
use bot_message::*;
pub use message::*;
use octos_actions::*;
pub use populate::*;
pub use report_modal::*;
pub use room_info_pane::*;
pub use search::*;
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


fn item_event_id(item: &Arc<TimelineItem>) -> Option<&EventId> {
    let TimelineItemKind::Event(event) = item.kind() else {
        return None;
    };
    event.event_id()
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
    message::script_mod(vm);
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

                // Failure icon on a message whose send failed: confirm, then resend.
                if wr
                    .button(cx, ids!(content.message_action_bar.send_state_indicator.send_failure_button))
                    .clicked(actions)
                {
                    let Some(tl_idx) = tl_idx_from_item_id(index, has_encryption_notice) else { continue };
                    let Some(tl) = self.tl_state.as_ref() else { continue };
                    let Some(event_tl_item) = tl.items.get(tl_idx).and_then(|item| item.as_event()) else { continue };
                    // Resending goes through the send queue by transaction id, so a
                    // local echo without one (which should not exist) cannot recover.
                    let Some(transaction_id) = event_tl_item.transaction_id() else {
                        error!("BUG: cannot resend failed local echo without a transaction ID");
                        continue;
                    };
                    let app_language = self.app_language;
                    let body_text = match MessageDeliveryState::from_item(event_tl_item) {
                        Some(MessageDeliveryState::FailedWedged { reason }) => tr_fmt(
                            app_language,
                            "room_screen.send_state.resend_modal.body_reason",
                            &[("reason", &reason)],
                        ),
                        _ => tr_key(app_language, "room_screen.send_state.resend_modal.body").to_string(),
                    };
                    let timeline_kind = tl.kind.clone();
                    let transaction_id = transaction_id.to_owned();
                    let content = ConfirmationModalContent {
                        title_text: tr_key(app_language, "room_screen.send_state.resend_modal.title").into(),
                        body_text: body_text.into(),
                        accept_button_text: Some(tr_key(app_language, "room_screen.send_state.resend_modal.resend").into()),
                        cancel_button_text: Some(tr_key(app_language, "room_screen.send_state.resend_modal.cancel").into()),
                        on_accept_clicked: Some(Box::new(move |_cx| {
                            // Re-enables the room's send queue and unwedges the echo.
                            submit_async_request(MatrixRequest::RetrySend { timeline_kind, transaction_id });
                        })),
                        ..Default::default()
                    };
                    cx.action(PositiveConfirmationModalAction::Show(RefCell::new(Some(content))));
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
    fn test_room_members_fetch_updates_rooms_list_member_ids() {
        let src = include_str!("mod.rs");

        assert!(src.contains("TimelineUpdate::RoomMembersListFetched"));
        assert!(src.contains("RoomsListUpdate::UpdateRoomMemberUserIds"));
        assert!(src.contains("member.user_id().to_owned()"));
    }

}
