//! The populate pass: fills recycled timeline item widgets with content
//! for every message kind, replies, thread summaries, and send state.

use super::*;
use crate::shared::bouncing_dots::BouncingDotsWidgetRefExt;

/// How a message's delivery should be shown in the timeline.
///
/// Only messages sent through the send queue have a delivery state at all — a
/// remote event that arrived over sync has none, and neither do the paths that
/// bypass the queue (`Room::send_raw`, used for agent-chat/octos routing, and
/// `send_attachment`). Those all render exactly as before.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum MessageDeliveryState {
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
    pub(super) fn from_item(item: &EventTimelineItem) -> Option<Self> {
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
pub(super) fn wedge_error_reason(error: &matrix_sdk::Error) -> String {
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
pub(super) const COLOR_THREAD_SUMMARY_BG: Vec4 = vec4(1.0, 0.957, 0.898, 1.0);
/// #FFEACC
pub(super) const COLOR_THREAD_SUMMARY_BG_HOVER: Vec4 = vec4(1.0, 0.918, 0.8, 1.0);

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct ItemDrawnStatus {
    /// Whether the profile info (avatar and displayable username) were drawn for this item.
    pub(super) profile_drawn: bool,
    /// Whether the content of the item was drawn (e.g., the message text, image, video, sticker, etc).
    pub(super) content_drawn: bool,
}

#[derive(Clone, Debug)]
pub(super) struct FetchedThreadSummary {
    pub(super) num_replies: u32,
    pub(super) latest_reply_preview_text: Option<String>,
}

impl ItemDrawnStatus {
    /// Returns a new `ItemDrawnStatus` with both `profile_drawn` and `content_drawn` set to `false`.
    pub(super) const fn new() -> Self {
        Self {
            profile_drawn: false,
            content_drawn: false,
        }
    }
    /// Returns a new `ItemDrawnStatus` with both `profile_drawn` and `content_drawn` set to `true`.
    pub(super) const fn both_drawn() -> Self {
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
pub(super) fn populate_message_view(
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
    streaming_messages: &mut HashMap<OwnedEventId, crate::home::streaming_animation::StreamingAnimState>,
    action_button_contexts: &mut HashMap<(OwnedEventId, usize), OctosActionButtonContext>,
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
pub(super) fn populate_text_message_content(
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

/// Renders a message's delivery state in the bottom-right indicator: animated
/// dots while the send queue still owns the message, a warning button once the
/// send has failed (clicking it opens the resend confirmation modal).
///
/// Every widget is written on every call, never only on the branch that needs
/// it. These item widgets are recycled by the PortalList, so an indicator left
/// visible would reappear under an unrelated message.
pub(super) fn populate_send_state(cx: &mut Cx, item: &WidgetRef, event_tl_item: &EventTimelineItem) {
    let state = MessageDeliveryState::from_item(event_tl_item);

    let indicator = item.view(cx, ids!(content.message_action_bar.send_state_indicator));
    let dots = item.bouncing_dots(
        cx,
        ids!(content.message_action_bar.send_state_indicator.sending_dots),
    );
    let failure_button = item.button(
        cx,
        ids!(content.message_action_bar.send_state_indicator.send_failure_button),
    );

    let sending = matches!(state, Some(MessageDeliveryState::Sending));
    let failed = matches!(
        state,
        Some(MessageDeliveryState::FailedRetrying | MessageDeliveryState::FailedWedged { .. })
    );

    indicator.set_visible(cx, sending || failed);
    dots.set_visible(cx, sending);
    if sending {
        dots.start_animation(cx);
    } else {
        dots.stop_animation(cx);
    }
    failure_button.set_visible(cx, failed);
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
pub(super) fn populate_image_message_content(
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
            (MediaCacheEntry::Failed(status_code), _media_format) => {
                if text_or_image_ref.view(cx, ids!(default_image_view)).visible() {
                    fully_drawn = true;
                    return;
                }
                // Show the message's own body (its alt text / filename) and the
                // HTTP status, not the `mxc://` URI: the URI is a server-side
                // identifier the reader cannot open, copy anywhere useful, or
                // act on, and printing it into the timeline just looks broken.
                text_or_image_ref.show_error(
                    cx,
                    tr_fmt(app_language, "room_screen.image.failed_to_fetch", &[
                        ("body", body),
                        ("status", status_code.as_str()),
                    ]),
                    media_source.clone(),
                );
                // Drawing is complete — the retry button is the way forward
                // from here, handled in `handle_actions`.
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
pub(super) fn populate_file_message_content(
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
pub(super) fn populate_audio_message_content(
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
                .map(|m| format!("  {},", htmlize::escape_text(m)))
                .unwrap_or_default(),
            info.size
                .map(|bytes| format!("  ({}),", ByteSize::b(bytes.into())))
                .unwrap_or_default(),
        ))
        .unwrap_or_default();
    let caption = audio.formatted_caption()
        .filter(|fb| fb.format == MessageFormat::Html)
        .map(|fb| format!("<br><i>{}</i>", fb.body))
        // Escape caption to prevent HTML injection
        .or_else(|| audio.caption().map(|c| format!("<br><i>{}</i>", htmlize::escape_text(c))))
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
pub(super) fn populate_video_message_content(
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
                .map(|m| format!("  {},", htmlize::escape_text(m)))
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
        // Escape caption to prevent HTML injection
        .or_else(|| video.caption().map(|c| format!("<br><i>{}</i>", htmlize::escape_text(c))))
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
pub(super) fn populate_location_message_content(
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
pub(super) fn populate_redacted_message_content(
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
pub(super) fn draw_replied_to_message(
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
pub(super) fn populate_thread_root_summary(
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
