//! User interactions with timeline items: the message-action handler,
//! link clicks, media downloads, forwarding, fold/retry toggles, and
//! the translation language popup.

use super::*;

pub(super) const TRANSLATION_LANG_POPUP_WIDTH: f64 = 220.0;
pub(super) const TRANSLATION_LANG_POPUP_SCROLL_HEIGHT: f64 = 288.0;
pub(super) const TRANSLATION_LANG_POPUP_HEIGHT: f64 = TRANSLATION_LANG_POPUP_SCROLL_HEIGHT + 8.0;
pub(super) const TRANSLATION_LANG_POPUP_GAP: f64 = 6.0;
pub(super) const TRANSLATION_LANG_POPUP_MARGIN: f64 = 8.0;

pub(super) fn compute_translation_lang_popup_abs_pos(button_rect: Rect, container_rect: Rect) -> DVec2 {
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

impl RoomScreen {
    pub(super) fn toggle_small_state_event_group(&mut self, cx: &mut Cx, group_start_index: usize) {
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
    pub(super) fn toggle_bot_body_expanded(&mut self, cx: &mut Cx, tl_idx: usize) {
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

    pub(super) fn sync_translation_lang_popup(&mut self, cx: &mut Cx) {
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

    /// Handles a link being clicked in any child widgets of this RoomScreen.
    ///
    /// Returns `true` if the given `action` was handled as a link click.
    pub(super) fn handle_link_clicked(
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
    pub(super) fn handle_mxc_file_download(
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
    pub(super) fn handle_image_click(
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
    pub(super) fn find_event_in_timeline<'a>(
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

    pub(super) fn forward_message_content(
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
    pub(super) fn handle_message_actions(
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
                    // Every reaction — from the emoji row, the custom input, or
                    // a click on an existing pill — funnels through here, so
                    // this is the one place worth guarding against sending an
                    // empty key to the server.
                    let reaction = reaction.trim();
                    if reaction.is_empty() { continue }
                    submit_async_request(MatrixRequest::ToggleReaction {
                        timeline_kind: tl.kind.clone(),
                        timeline_event_id: details.timeline_event_id.clone(),
                        reaction: reaction.to_string(),
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

                    cx.action(crate::home::event_source_modal::EventSourceModalAction::Open {
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

    pub(super) fn toggle_translation_lang_popup(&mut self, cx: &mut Cx, button_rect: Rect) {
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

    pub(super) fn handle_translation_lang_popup_actions(&mut self, cx: &mut Cx, actions: &Actions) {
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

    /// Shows the user profile sliding pane with the given avatar info.
    pub(super) fn show_user_profile(
        &mut self,
        cx: &mut Cx,
        pane: &UserProfileSlidingPaneRef,
        info: UserProfilePaneInfo,
    ) {
        pane.set_info(cx, info);
        pane.show(cx);
        self.redraw(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
