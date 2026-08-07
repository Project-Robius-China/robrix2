//! Collapsible "small" state events: grouping runs of membership and
//! profile-change events, their summary phrasing, and the
//! `SmallStateEventContent` populate pass.

use super::*;

pub(super) const MIN_SMALL_STATE_EVENTS_TO_COLLAPSE: usize = 2;

#[derive(Clone, Debug)]
pub(super) struct SmallStateEventGroup {
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) count: usize,
    pub(super) first_event_id: OwnedEventId,
    pub(super) collapsed: bool,
}

#[derive(Default)]
pub(super) struct SmallStateEventGroupIndex {
    pub(super) by_start: HashMap<usize, SmallStateEventGroup>,
    pub(super) collapsed_hidden_indices: RangeSet<usize>,
    pub(super) summary_by_start: HashMap<usize, String>,
}

#[derive(Default)]
pub(super) struct SmallStateSummaryStats {
    pub(super) joined_users: Vec<String>,
    pub(super) left_users: Vec<String>,
    pub(super) profile_picture_changes: HashMap<String, usize>,
    pub(super) display_name_changes: HashMap<String, usize>,
    pub(super) other_changes: usize,
}

pub(super) fn timeline_item_is_small_state_event(
    timeline_item: &TimelineItem,
    timeline_kind: &TimelineKind,
) -> bool {
    let TimelineItemKind::Event(event_tl_item) = timeline_item.kind() else {
        return false;
    };
    match event_tl_item.content() {
        TimelineItemContent::MsgLike(msg_like_content) => {
            if timeline_kind.thread_root_event_id().is_none()
                && msg_like_content.thread_root.is_some()
            {
                return false;
            }
            matches!(
                msg_like_content.kind,
                MsgLikeKind::Poll(_)
                | MsgLikeKind::UnableToDecrypt(_)
                | MsgLikeKind::LiveLocation(_)
                | MsgLikeKind::Other(_)
            )
        }
        TimelineItemContent::MembershipChange(_)
        | TimelineItemContent::ProfileChange(_)
        | TimelineItemContent::OtherState(_) => true,
        _ => false,
    }
}

pub(super) fn compute_small_state_event_groups(
    items: &Vector<Arc<TimelineItem>>,
    timeline_kind: &TimelineKind,
    expanded_group_event_ids: &HashSet<OwnedEventId>,
) -> Vec<SmallStateEventGroup> {
    let mut groups = Vec::new();
    let mut idx = 0usize;
    while idx < items.len() {
        let is_small = items
            .get(idx)
            .is_some_and(|item| timeline_item_is_small_state_event(item, timeline_kind));
        if !is_small {
            idx += 1;
            continue;
        }

        let start = idx;
        idx += 1;
        while idx < items.len()
            && items
                .get(idx)
                .is_some_and(|item| timeline_item_is_small_state_event(item, timeline_kind))
        {
            idx += 1;
        }
        let end = idx;
        let count = end.saturating_sub(start);
        if count < MIN_SMALL_STATE_EVENTS_TO_COLLAPSE {
            continue;
        }

        let Some(first_event_id) = items
            .get(start)
            .and_then(|item| item.as_event())
            .and_then(|event| event.event_id())
            .map(ToOwned::to_owned)
        else {
            continue;
        };

        groups.push(SmallStateEventGroup {
            start,
            end,
            count,
            collapsed: !expanded_group_event_ids.contains(&first_event_id),
            first_event_id,
        });
    }
    groups
}

pub(super) fn index_small_state_event_groups(
    groups: impl IntoIterator<Item = SmallStateEventGroup>,
) -> SmallStateEventGroupIndex {
    let mut index = SmallStateEventGroupIndex::default();
    for group in groups {
        if group.collapsed {
            index
                .collapsed_hidden_indices
                .insert(group.start + 1 .. group.end);
        }
        index.by_start.insert(group.start, group);
    }
    index
}

pub(super) fn build_small_state_event_group_index(
    items: &Vector<Arc<TimelineItem>>,
    timeline_kind: &TimelineKind,
    expanded_group_event_ids: &HashSet<OwnedEventId>,
    app_language: AppLanguage,
) -> SmallStateEventGroupIndex {
    let mut index = index_small_state_event_groups(compute_small_state_event_groups(
        items,
        timeline_kind,
        expanded_group_event_ids,
    ));
    index.summary_by_start = index
        .by_start
        .iter()
        .map(|(&start, group)| {
            (
                start,
                format_small_state_group_summary_text(app_language, items, group),
            )
        })
        .collect();
    index
}

pub(super) fn prune_expanded_small_state_group_ids(
    items: &Vector<Arc<TimelineItem>>,
    timeline_kind: &TimelineKind,
    expanded_group_event_ids: &mut HashSet<OwnedEventId>,
) {
    let empty_expanded_ids: HashSet<OwnedEventId> = HashSet::new();
    let valid_group_ids: HashSet<OwnedEventId> = compute_small_state_event_groups(
        items,
        timeline_kind,
        &empty_expanded_ids,
    )
    .into_iter()
    .map(|group| group.first_event_id)
    .collect();
    expanded_group_event_ids.retain(|event_id| valid_group_ids.contains(event_id));
}

pub(super) fn summarize_sender_name(event_tl_item: &EventTimelineItem) -> String {
    if let TimelineDetails::Ready(profile) = event_tl_item.sender_profile()
        && let Some(name) = profile.display_name.as_ref()
        && !name.is_empty()
    {
        return name.clone();
    }

    let raw = event_tl_item.sender().as_str();
    let without_at = raw.strip_prefix('@').unwrap_or(raw);
    without_at.split(':').next().unwrap_or(without_at).to_string()
}

pub(super) fn push_unique_name(names: &mut Vec<String>, name: String) {
    if !names.iter().any(|n| n == &name) {
        names.push(name);
    }
}

pub(super) fn collect_small_state_summary_stats(
    items: &Vector<Arc<TimelineItem>>,
    group: &SmallStateEventGroup,
) -> SmallStateSummaryStats {
    let mut stats = SmallStateSummaryStats::default();
    for idx in group.start .. group.end {
        let Some(item) = items.get(idx) else { continue };
        let TimelineItemKind::Event(event_tl_item) = item.kind() else { continue };
        let sender_name = summarize_sender_name(event_tl_item);
        match event_tl_item.content() {
            TimelineItemContent::MembershipChange(change) => match change.change() {
                Some(MembershipChange::Joined)
                | Some(MembershipChange::InvitationAccepted) => {
                    push_unique_name(&mut stats.joined_users, sender_name);
                }
                Some(MembershipChange::Left)
                | Some(MembershipChange::KnockRetracted)
                | Some(MembershipChange::InvitationRejected) => {
                    push_unique_name(&mut stats.left_users, sender_name);
                }
                Some(MembershipChange::NotImplemented)
                | Some(MembershipChange::None)
                | Some(MembershipChange::Error)
                | None => {}
                _ => {
                    stats.other_changes += 1;
                }
            },
            TimelineItemContent::ProfileChange(change) => {
                let mut did_count = false;
                if change.avatar_url_change().is_some() {
                    *stats.profile_picture_changes.entry(sender_name.clone()).or_insert(0) += 1;
                    did_count = true;
                }
                if change.displayname_change().is_some() {
                    *stats.display_name_changes.entry(sender_name).or_insert(0) += 1;
                    did_count = true;
                }
                if !did_count {
                    stats.other_changes += 1;
                }
            }
            TimelineItemContent::OtherState(_)
            | TimelineItemContent::MsgLike(_) => {
                stats.other_changes += 1;
            }
            _ => {}
        }
    }
    stats
}

pub(super) fn format_people_phrase(
    app_language: AppLanguage,
    names: &[String],
    one_suffix_en: &str,
    plural_suffix_en: &str,
    one_suffix_zh: &str,
    plural_suffix_zh: &str,
) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    Some(match app_language {
        AppLanguage::ChineseSimplified => match names.len() {
            1 => format!("{}{}", names[0], one_suffix_zh),
            2 => format!("{}、{}{}", names[0], names[1], plural_suffix_zh),
            n => format!("{} 等 {} 人{}", names[0], n, plural_suffix_zh),
        },
        AppLanguage::English => match names.len() {
            1 => format!("{}{}", names[0], one_suffix_en),
            2 => format!("{} and one other{}", names[0], plural_suffix_en),
            n => format!("{} and {} others{}", names[0], n - 1, plural_suffix_en),
        },
    })
}

pub(super) fn format_top_user_counter_phrase(
    app_language: AppLanguage,
    counts: &HashMap<String, usize>,
    one_en: &str,
    many_en: &str,
    one_zh: &str,
    many_zh: &str,
) -> Option<String> {
    if counts.is_empty() {
        return None;
    }
    let mut entries: Vec<(&String, &usize)> = counts.iter().collect();
    entries.sort_by(|(name_a, count_a), (name_b, count_b)| {
        count_b.cmp(count_a).then_with(|| name_a.cmp(name_b))
    });
    let (name, count) = entries[0];
    Some(match app_language {
        AppLanguage::ChineseSimplified => {
            if *count > 1 {
                format!("{name}{many_zh}", many_zh = many_zh.replace("{count}", &count.to_string()))
            } else {
                format!("{name}{one_zh}")
            }
        }
        AppLanguage::English => {
            if *count > 1 {
                format!("{name}{many_en}", many_en = many_en.replace("{count}", &count.to_string()))
            } else {
                format!("{name}{one_en}")
            }
        }
    })
}

pub(super) fn format_small_state_group_summary_text(
    app_language: AppLanguage,
    items: &Vector<Arc<TimelineItem>>,
    group: &SmallStateEventGroup,
) -> String {
    let stats = collect_small_state_summary_stats(items, group);
    let mut parts = Vec::new();

    if let Some(joined) = format_people_phrase(
        app_language,
        &stats.joined_users,
        " joined",
        " joined",
        " 加入了房间",
        " 加入了房间",
    ) {
        parts.push(joined);
    }
    if let Some(left) = format_people_phrase(
        app_language,
        &stats.left_users,
        " left",
        " left",
        " 离开了房间",
        " 离开了房间",
    ) {
        parts.push(left);
    }
    if let Some(profile_pic) = format_top_user_counter_phrase(
        app_language,
        &stats.profile_picture_changes,
        " changed their profile picture",
        " changed their profile picture {count} times",
        " 更换了头像",
        " 更换了头像 {count} 次",
    ) {
        parts.push(profile_pic);
    }
    if let Some(display_name) = format_top_user_counter_phrase(
        app_language,
        &stats.display_name_changes,
        " changed their display name",
        " changed their display name {count} times",
        " 修改了昵称",
        " 修改了昵称 {count} 次",
    ) {
        parts.push(display_name);
    }
    if stats.other_changes > 0 {
        parts.push(match app_language {
            AppLanguage::ChineseSimplified => format!("另有 {} 条其他状态变更", stats.other_changes),
            AppLanguage::English => format!("{} other state changes", stats.other_changes),
        });
    }

    if parts.is_empty() {
        return match app_language {
            AppLanguage::ChineseSimplified => format!("{} 条状态事件", group.count),
            AppLanguage::English => format!("{} state events", group.count),
        };
    }
    parts.join(", ")
}

/// A trait for abstracting over the different types of timeline events
/// that can be displayed in a `SmallStateEvent` widget.
pub(super) trait SmallStateEventContent {
    /// Populates the *content* (not the profile) of the given `item` with data from
    /// the given `event_tl_item` and `self` (the specific type of event content).
    ///
    /// ## Arguments
    /// * `item`: a `SmallStateEvent` widget that has already been added to
    ///   the given `PortalList` at the given `item_id`.
    ///   This function may either modify that item or completely replace it
    ///   with a different widget if needed.
    /// * `item_drawn_status`: the old (prior) drawn status of the item.
    /// * `new_drawn_status`: the new drawn status of the item, which may have already
    ///   been updated to reflect the item's profile having been drawn right before this function.
    ///
    /// ## Return
    /// Returns a tuple of the drawn `item` and its `new_drawn_status`.
    fn populate_item_content(
        &self,
        cx: &mut Cx,
        list: &mut PortalList,
        item_id: usize,
        item: WidgetRef,
        event_tl_item: &EventTimelineItem,
        username: &str,
        item_drawn_status: ItemDrawnStatus,
        new_drawn_status: ItemDrawnStatus,
    ) -> (WidgetRef, ItemDrawnStatus);
}

// For unable to decrypt messages.
impl SmallStateEventContent for EncryptedMessage {
    fn populate_item_content(
        &self,
        cx: &mut Cx,
        _list: &mut PortalList,
        _item_id: usize,
        item: WidgetRef,
        _event_tl_item: &EventTimelineItem,
        username: &str,
        _item_drawn_status: ItemDrawnStatus,
        mut new_drawn_status: ItemDrawnStatus,
    ) -> (WidgetRef, ItemDrawnStatus) {
        item.label(cx, ids!(event_row.content)).set_text(
            cx,
            &text_preview_of_encrypted_message(self).format_with(username, false),
        );
        new_drawn_status.content_drawn = true;
        (item, new_drawn_status)
    }
}

// For other message-like content (custom message-like events).
impl SmallStateEventContent for LiveLocationState {
    fn populate_item_content(
        &self,
        cx: &mut Cx,
        _list: &mut PortalList,
        _item_id: usize,
        item: WidgetRef,
        _event_tl_item: &EventTimelineItem,
        username: &str,
        _item_drawn_status: ItemDrawnStatus,
        mut new_drawn_status: ItemDrawnStatus,
    ) -> (WidgetRef, ItemDrawnStatus) {
        item.label(cx, ids!(event_row.content)).set_text(
            cx,
            &format!("{username} shared a live location."),
        );
        new_drawn_status.content_drawn = true;
        (item, new_drawn_status)
    }
}

impl SmallStateEventContent for OtherMessageLike {
    fn populate_item_content(
        &self,
        cx: &mut Cx,
        _list: &mut PortalList,
        _item_id: usize,
        item: WidgetRef,
        _event_tl_item: &EventTimelineItem,
        username: &str,
        _item_drawn_status: ItemDrawnStatus,
        mut new_drawn_status: ItemDrawnStatus,
    ) -> (WidgetRef, ItemDrawnStatus) {
        item.label(cx, ids!(event_row.content)).set_text(
            cx,
            &text_preview_of_other_message_like(self).format_with(username, false),
        );
        new_drawn_status.content_drawn = true;
        (item, new_drawn_status)
    }
}

// TODO: once we properly display polls, we should remove this,
//       because Polls shouldn't be displayed using the SmallStateEvent widget.
impl SmallStateEventContent for PollState {
    fn populate_item_content(
        &self,
        cx: &mut Cx,
        _list: &mut PortalList,
        _item_id: usize,
        item: WidgetRef,
        _event_tl_item: &EventTimelineItem,
        _username: &str,
        _item_drawn_status: ItemDrawnStatus,
        mut new_drawn_status: ItemDrawnStatus,
    ) -> (WidgetRef, ItemDrawnStatus) {
        item.label(cx, ids!(event_row.content)).set_text(
            cx,
            self.fallback_text().unwrap_or_else(|| self.results().question).as_str(),
        );
        new_drawn_status.content_drawn = true;
        (item, new_drawn_status)
    }
}

impl SmallStateEventContent for timeline::OtherState {
    fn populate_item_content(
        &self,
        cx: &mut Cx,
        list: &mut PortalList,
        item_id: usize,
        item: WidgetRef,
        _event_tl_item: &EventTimelineItem,
        username: &str,
        _item_drawn_status: ItemDrawnStatus,
        mut new_drawn_status: ItemDrawnStatus,
    ) -> (WidgetRef, ItemDrawnStatus) {
        let item = if let Some(text_preview) = text_preview_of_other_state(self, false) {
            item.label(cx, ids!(event_row.content))
                .set_text(cx, &text_preview.format_with(username, false));
            new_drawn_status.content_drawn = true;
            item
        } else {
            let item = list.item(cx, item_id, id!(Empty));
            new_drawn_status = ItemDrawnStatus::new();
            item
        };
        (item, new_drawn_status)
    }
}

impl SmallStateEventContent for MemberProfileChange {
    fn populate_item_content(
        &self,
        cx: &mut Cx,
        _list: &mut PortalList,
        _item_id: usize,
        item: WidgetRef,
        _event_tl_item: &EventTimelineItem,
        username: &str,
        _item_drawn_status: ItemDrawnStatus,
        mut new_drawn_status: ItemDrawnStatus,
    ) -> (WidgetRef, ItemDrawnStatus) {
        item.label(cx, ids!(event_row.content)).set_text(
            cx,
            &text_preview_of_member_profile_change(self, username, false)
                .format_with(username, false),
        );
        new_drawn_status.content_drawn = true;
        (item, new_drawn_status)
    }
}

impl SmallStateEventContent for RoomMembershipChange {
    fn populate_item_content(
        &self,
        cx: &mut Cx,
        list: &mut PortalList,
        item_id: usize,
        item: WidgetRef,
        _event_tl_item: &EventTimelineItem,
        username: &str,
        _item_drawn_status: ItemDrawnStatus,
        mut new_drawn_status: ItemDrawnStatus,
    ) -> (WidgetRef, ItemDrawnStatus) {
        let Some(preview) = text_preview_of_room_membership_change(self, false) else {
            // Don't actually display anything for nonexistent/unimportant membership changes.
            return (
                list.item(cx, item_id, id!(Empty)),
                ItemDrawnStatus::new(),
            );
        };

        item.label(cx, ids!(event_row.content))
            .set_text(cx, &preview.format_with(username, false));

        // The invite_user_button is only used for "Knocked" membership change events.
        item.button(cx, ids!(event_row.invite_user_button)).set_visible(
            cx,
            matches!(self.change(), Some(MembershipChange::Knocked)),
        );

        new_drawn_status.content_drawn = true;
        (item, new_drawn_status)
    }
}

/// Creates, populates, and adds a SmallStateEvent liveview widget to the given `PortalList`
/// with the given `item_id`.
///
/// The content of the returned widget is populated with data from the
/// given room membership change and its parent `EventTimelineItem`.
pub(super) fn populate_small_state_event(
    cx: &mut Cx,
    list: &mut PortalList,
    item_id: usize,
    timeline_kind: &TimelineKind,
    app_language: AppLanguage,
    event_tl_item: &EventTimelineItem,
    event_content: &impl SmallStateEventContent,
    item_drawn_status: ItemDrawnStatus,
    group_header_summary_text: Option<&str>,
    group_toggle_button_text: Option<&str>,
) -> (WidgetRef, ItemDrawnStatus) {
    let mut new_drawn_status = item_drawn_status;
    let (item, existed) = list.item_with_existed(cx, item_id, id!(SmallStateEvent));
    // The content of a small state event view may depend on the profile info,
    // so we can only mark the content as drawn after the profile has been fully drawn and cached.
    let skip_redrawing_profile = existed && item_drawn_status.profile_drawn;
    let skip_redrawing_content = skip_redrawing_profile && item_drawn_status.content_drawn;
    populate_read_receipts(&item, cx, timeline_kind, event_tl_item);
    if skip_redrawing_content {
        return (item, new_drawn_status);
    }

    // If the profile has been drawn, we can just quickly grab the user's display name
    // instead of having to call `set_avatar_and_get_username` again.
    let username_opt = skip_redrawing_profile
        .then(|| get_profile_display_name(event_tl_item))
        .flatten();

    let username = username_opt.unwrap_or_else(|| {
        // As a fallback, call `set_avatar_and_get_username` to get the user's display name.
        let avatar_ref = item.avatar(cx, ids!(event_row.avatar));

        let (username, profile_drawn) = avatar_ref.set_avatar_and_get_username(
            cx,
            timeline_kind,
            event_tl_item.sender(),
            Some(event_tl_item.sender_profile()),
            event_tl_item.event_id(),
            true,
        );
        // Draw the timestamp as part of the profile.
        if let Some(dt) = unix_time_millis_to_datetime(event_tl_item.timestamp()) {
            item.timestamp(cx, ids!(event_row.left_container.timestamp)).set_date_time(cx, dt);
        }
        new_drawn_status.profile_drawn = profile_drawn;
        username
    });

    // Proceed to draw the actual event content.
    let (item, new_drawn_status) = event_content.populate_item_content(
        cx,
        list,
        item_id,
        item,
        event_tl_item,
        &username,
        item_drawn_status,
        new_drawn_status,
    );

    item.button(cx, ids!(event_row.invite_user_button))
        .set_text(cx, tr_key(app_language, "room_screen.small_state.invite_to_room"));
    item.view(cx, ids!(group_header))
        .set_visible(cx, group_toggle_button_text.is_some());
    item.label(cx, ids!(group_header.group_summary_label))
        .set_visible(cx, group_header_summary_text.is_some());
    if let Some(summary_text) = group_header_summary_text {
        item.label(cx, ids!(group_header.group_summary_label))
            .set_text(cx, summary_text);
    }
    if let Some(button_text) = group_toggle_button_text {
        item.button(cx, ids!(group_header.state_group_toggle_button)).set_text(cx, button_text);
    }

    (item, new_drawn_status)
}


/// Returns the display name of the sender of the given `event_tl_item`, if available.
pub(super) fn get_profile_display_name(event_tl_item: &EventTimelineItem) -> Option<String> {
    if let TimelineDetails::Ready(profile) = event_tl_item.sender_profile() {
        profile.display_name.clone()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_state_group_index_stores_group_starts_and_collapsed_ranges() {
        let collapsed_event_id =
            OwnedEventId::try_from("$collapsed:example.org").unwrap();
        let expanded_event_id =
            OwnedEventId::try_from("$expanded:example.org").unwrap();
        let index = index_small_state_event_groups([
            SmallStateEventGroup {
                start: 2,
                end: 6,
                count: 4,
                first_event_id: collapsed_event_id.clone(),
                collapsed: true,
            },
            SmallStateEventGroup {
                start: 10,
                end: 12,
                count: 2,
                first_event_id: expanded_event_id.clone(),
                collapsed: false,
            },
        ]);

        assert_eq!(index.by_start.len(), 2);
        assert_eq!(
            index.by_start.get(&2).map(|group| (&group.first_event_id, group.count)),
            Some((&collapsed_event_id, 4)),
        );
        assert_eq!(
            index.by_start.get(&10).map(|group| (&group.first_event_id, group.count)),
            Some((&expanded_event_id, 2)),
        );
        assert!(!index.collapsed_hidden_indices.contains(&2));
        assert!(index.collapsed_hidden_indices.contains(&3));
        assert!(index.collapsed_hidden_indices.contains(&5));
        assert!(!index.collapsed_hidden_indices.contains(&6));
        assert!(!index.collapsed_hidden_indices.contains(&11));
    }
}
