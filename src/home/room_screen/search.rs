//! In-room message search state and results handling, plus jumping the
//! timeline to a specific event.

use super::*;

/// Returns a single-line preview of `s` collapsing internal whitespace and
/// trimming to `max_chars` chars (counted in unicode scalar values). Appends
/// an ellipsis when truncation occurred.
pub(super) fn truncate_preview(s: &str, max_chars: usize) -> String {
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
pub(super) fn message_search_hit_from_searched_message(m: &SearchedMessage) -> MessageSearchHit {
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

impl RoomScreen {
    /// Jumps to the target event ID in this timeline by smooth scrolling to it.
    ///
    /// This function searches backwards from the given `max_tl_idx` in the timeline
    /// for the given `event_id`. If found, it smooth-scrolls the portal list to that event.
    /// If not found, it displays the loading pane and starts a background search for the event.
    pub(super) fn jump_to_event(
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
    pub(super) fn handle_search_messages_actions(
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
    pub(super) fn submit_message_search(
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
    pub(super) fn submit_message_search_next_page(
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
    pub(super) fn current_room_id_or_placeholder(&self) -> OwnedRoomId {
        self.tl_state
            .as_ref()
            .map(|tl| tl.kind.room_id().clone())
            .unwrap_or_else(|| matrix_sdk::ruma::owned_room_id!("!none:none.invalid"))
    }

    /// Processes results posted by the sliding_sync.rs `SearchMessages`
    /// handler. Stale results (different room or different query) are
    /// dropped; matching results are pushed into the pane.
    pub(super) fn handle_search_messages_results(&mut self, cx: &mut Cx, actions: &Actions) {
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
    pub(super) fn is_search_result_current(&self, room_id: &OwnedRoomId, search_term: &str) -> bool {
        self.search_state.room_id.as_ref() == Some(room_id)
            && self.search_state.query == search_term
    }
}
