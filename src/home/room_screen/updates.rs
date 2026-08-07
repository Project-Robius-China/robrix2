//! Applying `TimelineUpdate`s to the displayed timeline: the batched
//! update-processing loop, plus the streaming and approval-expiry
//! timers it arms.

use super::*;

pub(super) const TIMELINE_UPDATE_TIME_BUDGET: Duration = Duration::from_millis(4);
pub(super) const MAX_TIMELINE_UPDATES_PER_PASS: usize = 128;

impl RoomScreen {
    pub(super) fn redraw_timeline_list(&self, cx: &mut Cx) {
        let portal_list = self.portal_list(cx, ids!(timeline.list));
        if let Some(mut list) = portal_list.borrow_mut() {
            list.redraw(cx);
        }
    }

    pub(super) fn invalidate_timeline_event_content(&mut self, event_id: &EventId) -> bool {
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

    pub(super) fn schedule_stream_timeout(&mut self, cx: &mut Cx) {
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

    pub(super) fn schedule_approval_expiry(&mut self, cx: &mut Cx) {
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

    pub(super) fn expire_approval_contexts(&mut self, now_millis: u64) -> bool {
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

    /// Processes all pending background updates to the currently-shown timeline.
    ///
    /// Redraws this RoomScreen view if any updates were applied.
    pub(super) fn process_timeline_updates(
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
}
