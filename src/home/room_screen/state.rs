//! Timeline UI state shared between the `RoomScreen` widget and the
//! background sync layer: the `TimelineUpdate` protocol, the per-room
//! `TimelineUiState` cache, and its save/restore types.

use super::*;

/// Immutable RoomScreen states passed via Scope props
/// from a RoomScreen widget to its child widgets for event/draw handlers.
pub struct RoomScreenProps {
    pub room_screen_widget_uid: WidgetUid,
    pub room_name_id: RoomNameId,
    pub timeline_kind: TimelineKind,
    pub room_members: Option<Arc<Vec<RoomMember>>>,
    pub is_encrypted: Option<bool>,
    pub is_direct_room: bool,
    pub room_bot_user_ids: Vec<OwnedUserId>,
    pub room_members_sync_pending: bool,
    /// Pre-computed sort order for room members (for mention search optimization).
    pub room_members_sort: Option<Arc<crate::room::member_search::PrecomputedMemberSort>>,
    pub room_avatar_url: Option<OwnedMxcUri>,
    pub app_service_enabled: bool,
    pub app_service_room_bound: bool,
    pub has_persisted_management_binding: bool,
    pub bound_bot_user_id: Option<OwnedUserId>,
    pub resolved_parent_bot_user_id: Option<OwnedUserId>,
    pub persisted_bound_bot_user_ids: Vec<OwnedUserId>,
    pub known_bot_user_ids: Vec<OwnedUserId>,
    /// Whether the current user has permission to invite users to this room.
    /// Gates the `/invitebot` slash command.
    pub can_invite: bool,
    /// Invites this client has sent that are still awaiting acceptance —
    /// consumed by the `/invitebot` picker (at open time) to keep its
    /// candidate filtering idempotent during the invite round-trip.
    pub pending_invited_users: Vec<OwnedUserId>,
}


/// Actions for the room screen's tooltip.
#[derive(Clone, Debug, Default)]
pub enum RoomScreenTooltipActions {
    /// Mouse over event when the mouse is over the read receipt.
    HoverInReadReceipt {
        /// The rect of the moused over widget
        widget_rect: Rect,
        /// Includes the list of users who have seen this event
        read_receipts: indexmap::IndexMap<matrix_sdk::ruma::OwnedUserId, Receipt>,
    },
    /// Mouse over event when the mouse is over the reaction button.
    HoverInReactionButton {
        /// The rectangle (bounds) of the hovered-over widget.
        widget_rect: Rect,
        /// Includes the list of users who have reacted to the emoji.
        reaction_data: ReactionData,
    },
    /// Mouse out event and clear tooltip.
    HoverOut,
    #[default]
    None,
}

/// A message that is sent from a background async task to a room's timeline view
/// for the purpose of update the Timeline UI contents or metadata.
pub enum TimelineUpdate {
    /// The very first update a given room's timeline receives.
    FirstUpdate {
        /// The initial list of timeline items (events) for a room.
        initial_items: Vector<Arc<TimelineItem>>,
    },
    /// The content of a room's timeline was updated in the background.
    NewItems {
        /// The entire list of timeline items (events) for a room.
        new_items: Vector<Arc<TimelineItem>>,
        /// The range of indices in the `items` list that have been changed in this update
        /// and thus must be removed from any caches of drawn items in the timeline.
        /// Any items outside of this range are assumed to be unchanged and need not be redrawn.
        changed_indices: Range<usize>,
        /// An optimization that informs the UI whether the changes to the timeline
        /// resulted in new items being *appended to the end* of the timeline.
        is_append: bool,
        /// Whether to clear the entire cache of drawn items in the timeline.
        /// This supersedes `index_of_first_change` and is used when the entire timeline is being redrawn.
        clear_cache: bool,
    },
    /// The updated number of unread messages in the room.
    NewUnreadMessagesCount(UnreadMessageCount),
    /// The target event ID was found at the given `index` in the timeline items vector.
    ///
    /// This means that the RoomScreen widget can scroll the timeline up to this event,
    /// and the background `timeline_subscriber_handler` async task can stop looking for this event.
    TargetEventFound {
        target_event_id: OwnedEventId,
        index: usize,
    },
    /// A notice that the background task doing pagination for this room is currently running
    /// a pagination request in the given direction, and is waiting for that request to complete.
    PaginationRunning(PaginationDirection),
    /// An error occurred while paginating the timeline for this room.
    PaginationError {
        error: timeline::Error,
        direction: PaginationDirection,
    },
    /// A notice that the background task doing pagination for this room has become idle,
    /// meaning that it has completed its recent pagination request(s).
    PaginationIdle {
        /// If `true`, the start of the timeline has been reached, meaning that
        /// there is no need to send further pagination requests.
        fully_paginated: bool,
        direction: PaginationDirection,
    },
    /// A notice that event details have been fetched from the server,
    /// including a `result` that indicates whether the request was successful.
    EventDetailsFetched {
        event_id: OwnedEventId,
        result: Result<(), matrix_sdk_ui::timeline::Error>,
    },
    /// A notice that fresh thread-summary details were fetched for a thread root.
    ThreadSummaryDetailsFetched {
        thread_root_event_id: OwnedEventId,
        timeline_item_index: usize,
        num_replies: u32,
        latest_reply_preview_text: Option<String>,
    },
    /// The result of a request to edit a message in this timeline.
    MessageEdited {
        timeline_event_item_id: TimelineEventItemId,
        result: Result<(), matrix_sdk_ui::timeline::Error>,
    },
    /// A notice that the room's members have been fetched from the server,
    /// though the success or failure of the request is not yet known until the client
    /// requests the member info via a timeline event's `sender_profile()` method.
    RoomMembersSynced,
    /// A notice that the room's full member list has been fetched from the server,
    /// includes a complete list of room members that can be shared across components.
    /// This is different from RoomMembersSynced which only indicates members were fetched
    /// but doesn't provide the actual data.
    RoomMembersListFetched {
        members: Vec<RoomMember>,
    },
    /// A notice with an option of Media Request Parameters that one or more requested media items (images, videos, etc.)
    /// that should be displayed in this timeline have now been fetched and are available.
    MediaFetched(MediaRequestParameters),
    /// A notice that one or more members of a this room are currently typing.
    TypingUsers {
        /// The list of display names of users who are currently typing in this room.
        users: Vec<String>,
    },
    /// The result of a pin/unpin request ([`MatrixRequest::PinEvent`]).
    PinResult {
        event_id: OwnedEventId,
        result: Result<bool, matrix_sdk::Error>,
        pin: bool,
    },
    /// An update containing the set of pinned events in this room.
    PinnedEvents(Vec<OwnedEventId>),
    /// An update containing the currently logged-in user's power levels for this room.
    UserPowerLevels(UserPowerLevels),
    /// An update to the currently logged-in user's own read receipt for this room.
    OwnUserReadReceipt(Receipt),
    /// A notice that the given room has been tombstoned (closed)
    /// and replaced by the given successor room.
    Tombstoned(SuccessorRoomDetails),
    /// A notice that link preview data for a URL has been fetched and is now available.
    LinkPreviewFetched,
    /// User confirmed a file upload via the file upload modal.
    FileUploadConfirmed(crate::shared::file_upload_modal::FileData),
    /// Progress update for an ongoing file upload.
    FileUploadUpdate {
        current: u64,
        total: u64,
    },
    /// The abort handle for an in-progress file upload.
    FileUploadAbortHandle(tokio::task::AbortHandle),
    /// An error occurred during file upload.
    FileUploadError {
        error: String,
        file_data: crate::shared::file_upload_modal::FileData,
        retryable: bool,
    },
    /// File upload completed successfully.
    FileUploadComplete,
    /// A file/media attachment download has completed (or failed) for this timeline.
    ///
    /// The `Result` indicates whether the save operation succeeded or failed.
    /// This does not immediately clear the pending-download entry so the UI can
    /// briefly show success/failure state.
    AttachmentDownloadFinished(OwnedMxcUri, Result<(), String>),
    /// Remove the given pending-download entry and return to idle button state.
    AttachmentDownloadReset(OwnedMxcUri),
}

pub(super) fn enqueue_timeline_update(
    pending_updates: &mut VecDeque<TimelineUpdate>,
    update: TimelineUpdate,
) {
    match update {
        TimelineUpdate::NewItems {
            new_items,
            changed_indices,
            clear_cache,
            is_append,
        } => {
            if let Some(TimelineUpdate::NewItems {
                changed_indices: previous_changed_indices,
                clear_cache: previous_clear_cache,
                is_append: previous_is_append,
                ..
            }) = pending_updates.back()
            {
                let changed_indices = previous_changed_indices.start.min(changed_indices.start)
                    ..previous_changed_indices.end.max(changed_indices.end);
                let clear_cache = *previous_clear_cache || clear_cache;
                let is_append = *previous_is_append || is_append;
                *pending_updates.back_mut().expect("checked above") = TimelineUpdate::NewItems {
                    new_items,
                    changed_indices,
                    clear_cache,
                    is_append,
                };
            } else {
                pending_updates.push_back(TimelineUpdate::NewItems {
                    new_items,
                    changed_indices,
                    clear_cache,
                    is_append,
                });
            }
        }
        update => {
            pending_updates.push_back(update);
        }
    }
}

thread_local! {
    /// The global set of all timeline states, one entry per room.
    ///
    /// This is only useful when accessed from the main UI thread.
    pub(super) static TIMELINE_STATES: RefCell<HashMap<TimelineKind, TimelineUiState>> =
        RefCell::new(HashMap::new());

    /// Rooms whose timeline states have been invalidated (left/kicked/banned)
    /// and must not be re-saved into `TIMELINE_STATES`.
    ///
    /// A still-open RoomScreen saves its state back into `TIMELINE_STATES` when
    /// it is hidden/closed, which can happen *after* the invalidation ran.
    /// This set lets `store_timeline_state` reject such late re-insertions.
    /// A room is removed from this set once it becomes joined again.
    static INVALIDATED_TIMELINE_ROOMS: RefCell<HashSet<OwnedRoomId>> =
        RefCell::new(HashSet::new());
}

/// The UI-side state of a single room's timeline, which is only accessed/updated by the UI thread.
///
/// This struct should only include states that need to be persisted for a given room
/// across multiple `Hide`/`Show` cycles of that room's timeline within a RoomScreen.
/// If a state is more temporary and shouldn't be persisted when the timeline is hidden,
/// then it should be stored in the RoomScreen widget itself, not in this struct.
pub(super) struct TimelineUiState {
    /// Info determining whether this is a main room timeline is a thread-focused timeline.
    pub(super) kind: TimelineKind,

    /// The power levels of the currently logged-in user in this room.
    pub(super) user_power: UserPowerLevels,

    /// The list of room members for this room.
    pub(super) room_members: Option<Arc<Vec<RoomMember>>>,

    /// Pre-computed sort order for room members (for efficient mention search).
    pub(super) room_members_sort: Option<Arc<crate::room::member_search::PrecomputedMemberSort>>,

    /// Whether the initial room-member sync is still in progress for this room.
    pub(super) room_members_sync_pending: bool,

    /// Whether we're waiting for a refreshed local member snapshot after sync completion.
    pub(super) awaiting_post_sync_member_refresh: bool,

    /// Whether this room's timeline has been fully paginated, which means
    /// that the oldest (first) event in the timeline is locally synced and available.
    /// When `true`, further backwards pagination requests will not be sent.
    ///
    /// This must be reset to `false` whenever the timeline is fully cleared.
    pub(super) fully_paginated: bool,

    /// Whether a backwards pagination request has already been submitted
    /// and is still in flight.
    pub(super) backwards_pagination_in_flight: bool,

    /// The list of items (events) in this room's timeline that our client currently knows about.
    pub(super) items: Vector<Arc<TimelineItem>>,

    /// The set of first-event IDs for small-state event groups that are expanded.
    ///
    /// By default, groups are collapsed unless their first event ID appears in this set.
    pub(super) expanded_small_state_group_event_ids: HashSet<OwnedEventId>,

    /// Derived lookup used by the draw path to fold small state-event groups.
    ///
    /// This is invalidated only when timeline items or group expansion state
    /// changes, instead of rebuilding an O(timeline length) index every frame.
    pub(super) small_state_event_group_index: Option<SmallStateEventGroupIndex>,

    /// Event IDs of long bot messages the user chose to unfold.
    ///
    /// Long agent replies are folded to a short preview by default; an ID lands
    /// here only after the user taps "show more". Kept on the timeline state
    /// (not the recycled item widget) so the choice survives PortalList
    /// virtualization, exactly like `expanded_small_state_group_event_ids`.
    pub(super) expanded_bot_body_event_ids: HashSet<OwnedEventId>,

    /// The range of items (indices in the above `items` list) whose event **contents** have been drawn
    /// since the last update and thus do not need to be re-populated on future draw events.
    ///
    /// This range is partially cleared on each background update (see below) to ensure that
    /// items modified during the update are properly redrawn. Thus, it is a conservative
    /// "cache tracker" that may not include all items that have already been drawn,
    /// but that's okay because big updates that clear out large parts of the rangeset
    /// only occur during back pagination, which is both rare and slow in and of itself.
    /// During typical usage, new events are appended to the end of the timeline,
    /// meaning that the range of already-drawn items doesn't need to be cleared.
    ///
    /// Upon a background update, only item indices greater than or equal to the
    /// `index_of_first_change` are removed from this set.
    pub(super) content_drawn_since_last_update: RangeSet<usize>,

    /// Same as `content_drawn_since_last_update`, but for the event **profiles** (avatar, username).
    pub(super) profile_drawn_since_last_update: RangeSet<usize>,

    /// The channel receiver for timeline updates for this room.
    ///
    /// Here we use a synchronous (non-async) channel because the receiver runs
    /// in a sync context and the sender runs in an async context,
    /// which is okay because a sender on an unbounded channel never needs to block.
    pub(super) update_receiver: crossbeam_channel::Receiver<TimelineUpdate>,

    /// Updates already pulled from the channel but deferred to a later UI pass.
    pub(super) pending_updates: VecDeque<TimelineUpdate>,

    /// The sender for timeline requests from a RoomScreen showing this room
    /// to the background async task that handles this room's timeline updates.
    pub(super) request_sender: TimelineRequestSender,

    /// Coordinates pagination requests sent by the room list and this timeline UI.
    pub(super) pagination_status: Arc<TimelinePaginationStatus>,

    /// The cache of media items (images, videos, etc.) that appear in this timeline.
    ///
    /// Currently this excludes avatars, as those are shared across multiple rooms.
    pub(super) media_cache: MediaCache,

    /// Cache for link preview data indexed by URL to avoid redundant network requests.
    pub(super) link_preview_cache: LinkPreviewCache,
    /// Cached fetched thread-summary details, keyed by thread-root event ID.
    pub(super) fetched_thread_summaries: HashMap<OwnedEventId, FetchedThreadSummary>,
    /// Set of thread roots currently being fetched to avoid duplicate in-flight requests.
    pub(super) pending_thread_summary_fetches: HashSet<OwnedEventId>,

    /// The states relevant to the UI display of this timeline that are saved upon
    /// a `Hide` action and restored upon a `Show` action.
    pub(super) saved_state: SavedState,

    /// The state of the message highlight animation.
    ///
    /// We need to run the animation once the scrolling, triggered by the click of of a
    /// a reply preview, ends. so we keep a small state for it.
    /// By default, it starts in Off.
    /// Once the scrolling is started, the state becomes Pending.
    /// If the animation was triggered, the state goes back to Off.
    pub(super) message_highlight_animation_state: MessageHighlightAnimationState,

    /// Active streaming animations, keyed by event ID.
    /// Stores the typewriter animation state for messages being streamed by bots.
    pub(super) streaming_messages: HashMap<OwnedEventId, crate::home::streaming_animation::StreamingAnimState>,

    /// The index of the timeline item that was most recently scrolled up past it.
    /// This is used to detect when the user has scrolled up past the second visible item (index 1)
    /// upwards to the first visible item (index 0), which is the top of the timeline,
    /// at which point we submit a backwards pagination request to fetch more events.
    pub(super) last_scrolled_index: usize,

    /// The index of the first item shown in the timeline's PortalList from *before* the last "jump".
    ///
    /// This index is saved before the timeline undergoes any jumps, e.g.,
    /// receiving new items, major scroll changes, or other timeline view jumps.
    pub(super) prev_first_index: Option<usize>,

    /// Whether the user has scrolled past their latest read marker.
    ///
    /// This is used to determine whether we should send a fully-read receipt
    /// after the user scrolls past their "read marker", i.e., their latest fully-read receipt.
    /// Its value is determined by comparing the fully-read event's timestamp with the
    /// first and last timestamp of displayed events in the timeline.
    /// When scrolling down, if the value is true, we send a fully-read receipt
    /// for the last visible event in the timeline.
    ///
    /// When new message come in, this value is reset to `false`.
    pub(super) scrolled_past_read_marker: bool,
    pub(super) latest_own_user_receipt: Option<Receipt>,

    /// If `Some`, this room has been tombstoned and the details of its successor room
    /// are contained within. If `None`, the room has not been tombstoned.
    pub(super) tombstone_info: Option<SuccessorRoomDetails>,
    /// Media attachments currently being downloaded in this timeline.
    pub(super) pending_downloads: Vec<PendingDownload>,
}

#[derive(Default, Debug)]
pub(super) enum MessageHighlightAnimationState {
    Pending { item_id: usize },
    #[default]
    Off,
}

/// States that are necessary to save in order to maintain a consistent UI display for a timeline.
///
/// These are saved when navigating away from a timeline (upon `Hide`)
/// and restored when navigating back to a timeline (upon `Show`).
#[derive(Default)]
pub(super) struct SavedState {
    /// The index of the first item in the timeline's PortalList that is currently visible,
    /// and the scroll offset from the top of the list's viewport to the beginning of that item.
    /// If this is `None`, then the timeline has not yet been scrolled by the user
    /// and the portal list will be set to "tail" (track) the bottom of the list.
    pub(super) first_index_and_scroll: Option<(usize, f64)>,
    /// The state of all UI elements in the `RoomInputBar`.
    pub(super) room_input_bar_state: RoomInputBarState,
}

/// Clears all UI-related timeline states for all known rooms.
///
/// This function requires passing in a reference to `Cx`,
/// which isn't used, but acts as a guarantee that this function
/// must only be called by the main UI thread.
pub fn clear_timeline_states(_cx: &mut Cx) {
    // Clear timeline states cache
    TIMELINE_STATES.with_borrow_mut(|states| {
        states.clear();
    });
}

/// Clears all UI-related timeline state (the main room timeline plus any open
/// thread timelines) for the single room `room_id`.
///
/// Call this once a room is no longer joined (left, kicked, or banned) so its
/// stale UI state — scroll position, pending downloads, tombstone info, etc. —
/// doesn't linger in the cache indefinitely (e.g. reappearing with wrong state
/// if the user later rejoins the same room).
///
/// This function requires passing in a reference to `Cx`,
/// which isn't used, but acts as a guarantee that this function
/// must only be called by the main UI thread.
pub fn invalidate_timeline_state_for_room(_cx: &mut Cx, room_id: &RoomId) {
    TIMELINE_STATES.with_borrow_mut(|states| {
        states.retain(|kind, _| kind.room_id() != room_id);
    });
    // A RoomScreen currently displaying this room still holds its state and will
    // try to save it back upon being hidden/closed; block that re-insertion.
    INVALIDATED_TIMELINE_ROOMS.with_borrow_mut(|rooms| {
        rooms.insert(room_id.to_owned());
    });
}

/// Marks the given room as valid again, e.g., once it has been (re-)joined,
/// such that its timeline states can be saved to `TIMELINE_STATES` once more.
pub fn clear_timeline_invalidation_for_room(room_id: &RoomId) {
    INVALIDATED_TIMELINE_ROOMS.with_borrow_mut(|rooms| {
        rooms.remove(room_id);
    });
}

/// Saves the given timeline state into the global `TIMELINE_STATES` map,
/// unless its room has been invalidated (left/kicked/banned) in the meantime.
pub(super) fn store_timeline_state(tl: TimelineUiState) {
    let is_invalidated = INVALIDATED_TIMELINE_ROOMS
        .with_borrow(|rooms| rooms.contains(tl.kind.room_id()));
    if is_invalidated {
        log!("Discarding timeline state for invalidated (left/banned) room {:?}", tl.kind);
        return;
    }
    TIMELINE_STATES.with_borrow_mut(|ts| ts.insert(tl.kind.clone(), tl));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adjacent_timeline_snapshots_coalesce_without_crossing_control_updates() {
        let mut pending = VecDeque::new();
        enqueue_timeline_update(
            &mut pending,
            TimelineUpdate::NewItems {
                new_items: Vector::new(),
                changed_indices: 4..8,
                clear_cache: false,
                is_append: true,
            },
        );
        match pending.back().unwrap() {
            TimelineUpdate::NewItems {
                changed_indices,
                clear_cache,
                is_append,
                ..
            } => {
                assert_eq!(changed_indices, &(4..8));
                assert!(!*clear_cache);
                assert!(*is_append);
            }
            _ => panic!("expected timeline snapshot"),
        }

        enqueue_timeline_update(
            &mut pending,
            TimelineUpdate::NewItems {
                new_items: Vector::new(),
                changed_indices: 9..12,
                clear_cache: false,
                is_append: true,
            },
        );
        assert_eq!(pending.len(), 1);
        match pending.back().unwrap() {
            TimelineUpdate::NewItems {
                changed_indices,
                clear_cache,
                is_append,
                ..
            } => {
                assert_eq!(changed_indices, &(4..12));
                assert!(!*clear_cache);
                assert!(*is_append);
            }
            _ => panic!("expected coalesced timeline snapshot"),
        }

        enqueue_timeline_update(
            &mut pending,
            TimelineUpdate::PaginationRunning(PaginationDirection::Backwards),
        );
        enqueue_timeline_update(
            &mut pending,
            TimelineUpdate::NewItems {
                new_items: Vector::new(),
                changed_indices: 1..2,
                clear_cache: false,
                is_append: true,
            },
        );
        assert_eq!(pending.len(), 3);
    }
}
