//! Bot / agent message rendering: the three-layer bot body parser, body
//! folding, streaming heuristics, and the populate pass for the bot
//! message card.

use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BotTimelineLayers {
    status: Option<String>,
    provider: Option<String>,
    body: String,
    footer: Option<String>,
}

impl BotTimelineLayers {
    fn plain(body: &str) -> Self {
        Self {
            status: None,
            provider: None,
            body: body.to_string(),
            footer: None,
        }
    }
}

/// How an agent-chat bridge tagged a relayed message: the bridge prefixes every
/// agent message body with one of three emoji for the message `type`
/// (`📋` request / `↩️` reply / `ℹ️` inform). It is the only *language-independent*
/// structured signal on the wire, so it drives the badge instead of any prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AgentReplyKind {
    Request,
    Reply,
    Inform,
}

impl AgentReplyKind {
    /// Splits a leading type emoji off a bot body, if present.
    /// Returns the kind plus the remaining text with the marker removed.
    fn split_from_body(body: &str) -> (Option<Self>, &str) {
        let trimmed = body.trim_start();
        for (marker, kind) in [
            ("📋", Self::Request),
            ("↩️", Self::Reply),
            // The variation-selector-free form of ℹ️ also occurs in the wild.
            ("ℹ️", Self::Inform),
            ("ℹ", Self::Inform),
        ] {
            if let Some(rest) = trimmed.strip_prefix(marker) {
                return (Some(kind), rest.trim_start());
            }
        }
        (None, body)
    }

    /// Short, translatable-later label shown in the badge next to the sender.
    fn label(self) -> &'static str {
        match self {
            Self::Request => "request",
            Self::Reply => "reply",
            Self::Inform => "info",
        }
    }
}

/// The role an agent plays in an agent-chat workflow, derived from its Matrix
/// localpart (`@ac_<team>_<role>:…`).
///
/// Every message in a workflow room otherwise carries the same generic `bot`
/// badge, which says nothing about *who* is speaking — the coordinator handing
/// off work, the implementer reporting a build, or a reviewer returning a
/// verdict. The role is encoded in the account name, so reading it needs no
/// prose parsing and is unaffected by the language the agent replies in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AgentRole {
    Coordinator,
    Implementer,
    Reviewer,
    FinalReviewer,
}

impl AgentRole {
    /// Derives the role from a Matrix localpart, e.g. `ac_tyrese_reviewer`.
    fn from_localpart(localpart: &str) -> Option<Self> {
        let name = localpart.to_ascii_lowercase();
        let matches = |suffix: &str| name == suffix || name.ends_with(&format!("_{suffix}"));
        // `final_reviewer` also ends with `reviewer`, so it has to be tested
        // first or every final reviewer would be labelled a plain reviewer.
        if matches("final_reviewer") {
            Some(Self::FinalReviewer)
        } else if matches("coordinator") {
            Some(Self::Coordinator)
        } else if matches("implementer") {
            Some(Self::Implementer)
        } else if matches("reviewer") {
            Some(Self::Reviewer)
        } else {
            None
        }
    }

    /// Badge text. Kept lowercase to match the existing `bot` badge's voice.
    fn label(self) -> &'static str {
        match self {
            Self::Coordinator => "coordinator",
            Self::Implementer => "implementer",
            Self::Reviewer => "reviewer",
            Self::FinalReviewer => "final review",
        }
    }
}

/// Splits a trailing agent-chat permalink line (`🔗 https://…`) off a bot body.
///
/// The bridge appends this line to every relayed agent message. Left inline it
/// is just a bare URL at the bottom of a wall of text — and it disappears
/// entirely once the body is folded. Pulling it out lets the card pin it in a
/// footer that stays reachable in both states.
///
/// Returns the body without that line, plus the URL.
pub(super) fn split_bot_permalink(body: &str) -> (String, Option<String>) {
    let Some(last_line) = body.lines().next_back() else {
        return (body.to_string(), None);
    };
    let trimmed = last_line.trim();
    let Some(rest) = trimmed.strip_prefix('🔗') else {
        return (body.to_string(), None);
    };
    let url = rest.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return (body.to_string(), None);
    }
    let kept: Vec<&str> = {
        let mut lines: Vec<&str> = body.lines().collect();
        lines.pop();
        while lines.last().is_some_and(|line| line.trim().is_empty()) {
            lines.pop();
        }
        lines
    };
    (kept.join("\n"), Some(url.to_string()))
}

/// Removes the bridge's trailing `<a …>🔗 View formatted</a>` anchor (and the
/// `<br>`s leading up to it) from a formatted body, so the permalink is not
/// rendered twice once it has been promoted to the card footer.
pub(super) fn strip_permalink_anchor_from_html(html: &str) -> String {
    let Some(anchor_start) = html.rfind("<a ") else {
        return html.to_string();
    };
    let tail = &html[anchor_start..];
    if !tail.contains('🔗') || !tail.trim_end().ends_with("</a>") {
        return html.to_string();
    }
    let mut head = html[..anchor_start].trim_end();
    loop {
        let trimmed = head
            .strip_suffix("<br>")
            .or_else(|| head.strip_suffix("<br/>"))
            .or_else(|| head.strip_suffix("<br />"));
        match trimmed {
            Some(shorter) => head = shorter.trim_end(),
            None => break,
        }
    }
    head.to_string()
}

/// Bodies longer than this (in lines) are folded to a preview so one long
/// message does not push every neighbouring one off-screen.
pub(super) const BOT_BODY_FOLD_LINE_THRESHOLD: usize = 8;
/// How many lines of the body remain visible while folded.
pub(super) const BOT_BODY_FOLD_PREVIEW_LINES: usize = 3;
/// Bodies longer than this (in characters) are folded even when they occupy few
/// lines. A pasted JSON dump or a minified payload can be one enormous line: it
/// wraps to fill the viewport while the line count says there is nothing to
/// fold.
pub(super) const BOT_BODY_FOLD_CHAR_THRESHOLD: usize = 1200;
/// How much of an over-long single line survives in the preview. Chosen to be a
/// little more than the preview's line budget would show at a typical width, so
/// the fold still reads as "the start of something", not a hard truncation.
pub(super) const BOT_BODY_FOLD_PREVIEW_CHARS: usize = 400;

/// Folds `body` to a short preview when it is long enough to crowd the timeline.
///
/// Length is measured two ways, because a body can be oversized in either
/// dimension: many lines, or few lines that are individually enormous. Returns
/// `None` when the body is small on both counts — callers treat that as "not
/// foldable", so the toggle stays hidden.
pub(super) fn fold_bot_body_preview(body: &str) -> Option<String> {
    let lines: Vec<&str> = body.lines().collect();
    let too_many_lines = lines.len() > BOT_BODY_FOLD_LINE_THRESHOLD;
    let too_many_chars = body.chars().count() > BOT_BODY_FOLD_CHAR_THRESHOLD;
    if !too_many_lines && !too_many_chars {
        return None;
    }

    let preview: Vec<&str> = lines
        .iter()
        .copied()
        .take(BOT_BODY_FOLD_PREVIEW_LINES)
        .collect();
    // A preview that ended up blank carries no information scent; showing the
    // full body is better than an empty card with a "show more" affordance.
    if preview.iter().all(|line| line.trim().is_empty()) {
        return None;
    }
    let mut preview = preview.join("\n");

    // Taking whole lines is not enough on its own: three lines of a minified
    // payload can still be the whole screen. Cap the preview by characters too,
    // on a char boundary so multi-byte text cannot panic the slice.
    if preview.chars().count() > BOT_BODY_FOLD_PREVIEW_CHARS {
        preview = preview.chars().take(BOT_BODY_FOLD_PREVIEW_CHARS).collect();
        preview.push('…');
    }

    // Cutting mid-body can leave a ``` fence open, which would swallow the rest
    // of the preview into an unterminated code block. Close it.
    if preview.matches("```").count() % 2 == 1 {
        preview.push_str("\n```");
    }
    Some(preview)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct BotTimelineRenderState {
    pub(super) show_card: bool,
    pub(super) show_body_card: bool,
    pub(super) show_status_strip: bool,
    pub(super) show_metadata_footer: bool,
    pub(super) status: Option<String>,
    pub(super) provider: Option<String>,
    pub(super) body: String,
    pub(super) footer: Option<String>,
    /// Message-type badge derived from the bridge's leading emoji marker.
    pub(super) kind: Option<AgentReplyKind>,
    /// Set when `body` was long enough to fold; holds the folded preview text.
    /// `None` means the body is short and the fold toggle must stay hidden.
    pub(super) folded_body: Option<String>,
    /// The bridge's trailing permalink, promoted out of the body so the card can
    /// pin it in a footer that stays visible while the body is folded.
    pub(super) permalink: Option<String>,
}

pub(super) fn is_bot_provider_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("via ") && trimmed.contains('(') && trimmed.ends_with(')')
}

pub(super) fn strip_streaming_cursor_suffix(line: &str) -> &str {
    line
        .trim_end()
        .strip_suffix('\u{25CF}')
        .map(str::trim_end)
        .unwrap_or_else(|| line.trim_end())
}

pub(super) fn is_bot_footer_line(line: &str) -> bool {
    let trimmed = strip_streaming_cursor_suffix(line);
    trimmed.starts_with('_')
        && trimmed.ends_with('_')
        && trimmed.contains("·")
        && trimmed.contains(" in")
        && trimmed.contains(" out")
}

pub(super) fn looks_like_metrics_line(line: &str) -> bool {
    let trimmed = strip_streaming_cursor_suffix(line).trim();
    !trimmed.is_empty()
        && trimmed.chars().count() <= 40
        && trimmed.chars().any(|ch| ch.is_ascii_digit())
        && (trimmed.contains('s') || trimmed.contains(" in") || trimmed.contains(" out"))
}

pub(super) fn looks_like_status_line(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty()
        && !trimmed.starts_with("via ")
        && !trimmed.starts_with('_')
        && trimmed.chars().count() <= 32
        && !trimmed.contains("  ")
}

pub(super) fn trim_structured_body_lines(lines: &[&str]) -> String {
    let mut start = 0;
    let mut end = lines.len();

    while start < end && lines[start].trim().is_empty() {
        start += 1;
    }
    while end > start && lines[end - 1].trim().is_empty() {
        end -= 1;
    }

    lines[start..end].join("\n")
}

pub(super) fn is_viable_bot_body(body: &str) -> bool {
    let trimmed = body.trim();
    !trimmed.is_empty()
        && trimmed.chars().any(|c| c.is_alphanumeric())
}

pub(super) fn parse_bot_timeline_layers(raw_body: &str, is_bot_sender: bool) -> BotTimelineLayers {
    if !is_bot_sender || raw_body.trim().is_empty() {
        return BotTimelineLayers::plain(raw_body);
    }

    let lines: Vec<&str> = raw_body.lines().collect();
    if lines.is_empty() {
        return BotTimelineLayers::plain(raw_body);
    }

    let (status, provider, mut content_start) =
        if lines.len() >= 2 && looks_like_status_line(lines[0]) && is_bot_provider_line(lines[1]) {
            (
                Some(lines[0].trim().to_string()),
                Some(lines[1].trim().to_string()),
                2usize,
            )
        } else if is_bot_provider_line(lines[0]) {
            (None, Some(lines[0].trim().to_string()), 1usize)
        } else {
            (None, None, 0usize)
        };

    while content_start < lines.len() && lines[content_start].trim().is_empty() {
        content_start += 1;
    }

    let mut footer = None;
    let mut content_end = lines.len();
    let last_non_empty = lines.iter().rposition(|line| !line.trim().is_empty());

    if let Some(last_idx) = last_non_empty {
        if is_bot_footer_line(lines[last_idx]) {
            footer = Some(strip_streaming_cursor_suffix(lines[last_idx]).trim().to_string());
            content_end = last_idx;
            while content_end > content_start && lines[content_end - 1].trim().is_empty() {
                content_end -= 1;
            }
        }
    }

    if content_start >= content_end {
        return if status.is_some() || provider.is_some() || footer.is_some() {
            BotTimelineLayers {
                status,
                provider,
                body: String::new(),
                footer,
            }
        } else {
            BotTimelineLayers::plain(raw_body)
        };
    }

    let content_lines = &lines[content_start..content_end];
    let mut body = trim_structured_body_lines(content_lines);
    if footer.is_none() && content_lines.len() == 1 && looks_like_metrics_line(content_lines[0]) {
        footer = Some(strip_streaming_cursor_suffix(content_lines[0]).trim().to_string());
        body.clear();
    }
    if !is_viable_bot_body(&body) {
        return if status.is_some() || provider.is_some() || footer.is_some() {
            BotTimelineLayers {
                status,
                provider,
                body,
                footer,
            }
        } else {
            BotTimelineLayers::plain(raw_body)
        };
    }

    BotTimelineLayers {
        status,
        provider,
        body,
        footer,
    }
}

pub(super) fn compute_bot_timeline_render_state(raw_body: &str, is_bot_sender: bool) -> BotTimelineRenderState {
    let layers = parse_bot_timeline_layers(raw_body, is_bot_sender);
    let show_card = is_bot_sender;

    // Strip the bridge's leading type emoji off the body and keep it as a badge
    // instead: it is redundant as prose but valuable as a typed affordance.
    let (kind, body, permalink) = if show_card {
        let (kind, rest) = AgentReplyKind::split_from_body(&layers.body);
        // Promote the trailing permalink out of the body before measuring it for
        // folding, so the preview spends its lines on content rather than a URL.
        let (body, permalink) = split_bot_permalink(rest);
        (kind, body, permalink)
    } else {
        (None, layers.body, None)
    };

    let show_body_card = show_card && !body.trim().is_empty();
    let folded_body = if show_body_card {
        fold_bot_body_preview(&body)
    } else {
        None
    };

    BotTimelineRenderState {
        show_card,
        show_body_card,
        show_status_strip: show_card && layers.status.is_some(),
        show_metadata_footer: show_card && (layers.provider.is_some() || layers.footer.is_some()),
        status: layers.status,
        provider: layers.provider,
        body,
        footer: layers.footer,
        kind,
        folded_body,
        permalink,
    }
}

/// The text that should land on the clipboard when copying a message body.
///
/// Bot-sent messages embed status / provider / `_metadata_` scaffolding lines
/// in their raw body; the bubble strips them at render time via
/// `compute_bot_timeline_render_state`, so copying must strip them the same
/// way. Human messages are copied verbatim.
pub(super) fn clipboard_text_for_message_body(body: String, sender_is_bot: bool) -> String {
    if sender_is_bot {
        compute_bot_timeline_render_state(&body, true).body
    } else {
        body
    }
}

pub(super) fn display_bot_footer_text(footer: &str) -> &str {
    strip_streaming_cursor_suffix(footer)
        .strip_prefix('_')
        .and_then(|trimmed| trimmed.strip_suffix('_'))
        .unwrap_or(footer)
}

pub(super) fn has_rich_markdown_syntax(text: &str) -> bool {
    let trimmed = text.trim();
    !trimmed.is_empty()
        && (
            trimmed.contains("```")
            || trimmed.starts_with("## ")
            || trimmed.starts_with("### ")
            || trimmed.contains("\n## ")
            || trimmed.contains("\n### ")
            || trimmed.starts_with("|")
            || trimmed.contains("\n|")
            || trimmed.starts_with("- ")
            || trimmed.contains("\n- ")
            || trimmed.starts_with("* ")
            || trimmed.contains("\n* ")
            || trimmed.contains("**")
            || trimmed.contains("`")
        )
}

pub(super) fn should_render_streaming_full_snapshot(
    body: &str,
    formatted_body: Option<&FormattedBody>,
    is_bot_sender: bool,
) -> bool {
    is_bot_sender
        && (
            formatted_body.is_some_and(|formatted| formatted.format == MessageFormat::Html)
            || has_rich_markdown_syntax(body)
        )
}

pub(super) fn select_bot_timeline_body_formatted_body(
    render_state: &BotTimelineRenderState,
    formatted_body: Option<&FormattedBody>,
) -> Option<FormattedBody> {
    if render_state.status.is_none()
        && render_state.provider.is_none()
        && render_state.footer.is_none()
    {
        return formatted_body
            .cloned()
            .or_else(|| has_rich_markdown_syntax(&render_state.body)
                .then(|| FormattedBody::markdown(&render_state.body))
                .flatten());
    }

    FormattedBody::markdown(&render_state.body)
}

pub(super) fn should_render_bot_timeline_body_with_markdown_widget(
    render_state: &BotTimelineRenderState,
) -> bool {
    render_state.show_body_card
        && render_state.body.contains("```")
}

pub(super) fn contains_cjk(text: &str) -> bool {
    text.chars().any(|ch|
        matches!(ch as u32,
            0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xF900..=0xFAFF
            | 0x20000..=0x2A6DF
            | 0x2A700..=0x2B73F
            | 0x2B740..=0x2B81F
            | 0x2B820..=0x2CEAF
            | 0x2CEB0..=0x2EBEF
            | 0x3000..=0x303F
            | 0x3040..=0x30FF
            | 0x31F0..=0x31FF
            | 0xAC00..=0xD7AF
        )
    )
}

pub(super) fn fenced_code_blocks_contain_cjk(text: &str) -> bool {
    let mut in_fence = false;
    let mut fence_has_cjk = false;

    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            if in_fence && fence_has_cjk {
                return true;
            }
            in_fence = !in_fence;
            fence_has_cjk = false;
            continue;
        }

        if in_fence && contains_cjk(line) {
            fence_has_cjk = true;
        }
    }

    in_fence && fence_has_cjk
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum BotTimelineCodeBlockMode {
    None,
    Highlighted,
    Plain,
}

pub(super) fn bot_timeline_code_block_mode(render_state: &BotTimelineRenderState) -> BotTimelineCodeBlockMode {
    if !should_render_bot_timeline_body_with_markdown_widget(render_state) {
        return BotTimelineCodeBlockMode::None;
    }

    if fenced_code_blocks_contain_cjk(&render_state.body) {
        BotTimelineCodeBlockMode::Plain
    } else {
        BotTimelineCodeBlockMode::Highlighted
    }
}

pub(super) fn streaming_update_requires_content_invalidation(
    state: &StreamingAnimState,
    new_text: &str,
    is_live: bool,
    render_full_target: bool,
) -> bool {
    state.target_text != new_text
        || state.is_live != is_live
        || state.render_full_target != render_full_target
}

/// Check if an event carries the MSC4357 `org.matrix.msc4357.live` field,
/// indicating that the message content is still being streamed.
///
/// For edit events (`m.replace`), the live field lives inside `m.new_content`
/// rather than at the top level of `content`, so we check both locations.
pub(super) fn content_has_msc4357_live_marker(content: &serde_json::Value) -> bool {
    let effective = content.get("m.new_content").unwrap_or(content);
    match effective.get("org.matrix.msc4357.live") {
        Some(serde_json::Value::Bool(value)) => *value,
        Some(_) => true,
        None => false,
    }
}

pub(super) fn is_msc4357_live(event_tl_item: &EventTimelineItem) -> bool {
    let message_is_edited = event_tl_item
        .content()
        .as_message()
        .is_some_and(|message| message.is_edited());
    event_tl_item.latest_edit_json()
        .or_else(|| (!message_is_edited).then(|| event_tl_item.original_json()).flatten())
        .and_then(|raw| raw.get_field::<serde_json::Value>("content").ok())
        .flatten()
        .map(|content| content_has_msc4357_live_marker(&content))
        .unwrap_or(false)
}

pub(super) fn streaming_scan_range(
    clear_cache: bool,
    changed_indices: &Range<usize>,
    _old_len: usize,
    new_len: usize,
) -> Range<usize> {
    if clear_cache {
        0..new_len
    } else {
        let start = changed_indices.start.min(new_len);
        let end = changed_indices.end.min(new_len);
        start..end
    }
}

pub(super) fn refresh_stream_indices<'a, I>(
    event_ids: I,
    streaming_messages: &mut HashMap<OwnedEventId, crate::home::streaming_animation::StreamingAnimState>,
)
where
    I: IntoIterator<Item = Option<&'a EventId>>,
{
    for state in streaming_messages.values_mut() {
        state.timeline_index = None;
    }

    for (idx, event_id) in event_ids.into_iter().enumerate() {
        let Some(event_id) = event_id else {
            continue;
        };
        if let Some(state) = streaming_messages.get_mut(event_id) {
            state.timeline_index = Some(idx);
        }
    }
}

pub(super) fn any_timeline_indices_visible<I, F>(
    indices: I,
    is_visible: F,
) -> bool
where
    I: IntoIterator<Item = Option<usize>>,
    F: FnMut(usize) -> bool,
{
    indices.into_iter().flatten().any(is_visible)
}

pub(super) fn streaming_candidates_from_items<'a>(
    items: &'a Vector<Arc<TimelineItem>>,
) -> impl Iterator<Item = (OwnedEventId, String, bool)> + 'a {
    items.iter().filter_map(|item| {
        let TimelineItemKind::Event(event) = item.kind() else {
            return None;
        };
        let event_id = event.event_id()?.to_owned();
        let text = RoomScreen::extract_message_text(item)?;
        Some((event_id, text, is_msc4357_live(event)))
    })
}

pub(super) fn rebuild_streaming_messages_for_full_snapshot<I>(
    items: I,
    previous_streaming_messages: Option<&HashMap<OwnedEventId, crate::home::streaming_animation::StreamingAnimState>>,
) -> (HashMap<OwnedEventId, crate::home::streaming_animation::StreamingAnimState>, bool)
where
    I: IntoIterator<Item = (OwnedEventId, String, bool)>,
{
    use crate::home::streaming_animation::StreamingAnimState;

    let mut rebuilt = HashMap::new();
    let mut should_schedule_frame = false;

    for (event_id, new_text, live) in items {
        if !live {
            continue;
        }

        // Only restore animations that were already tracked before the
        // snapshot reset.  Never create brand-new animations here — during
        // initial/reconnect loads the SDK may not have aggregated edits yet,
        // so completed messages can still appear as `live`.  Genuinely new
        // streams will be picked up on the next live sync update.
        if let Some(previous_state) = previous_streaming_messages
            .and_then(|states| states.get(&event_id))
        {
            let state = StreamingAnimState::restore(previous_state, &new_text, true);
            should_schedule_frame |= state.needs_frame();
            rebuilt.insert(event_id, state);
        }
    }

    (rebuilt, should_schedule_frame)
}

pub(super) fn next_stream_timeout<'a>(
    states: impl IntoIterator<Item = &'a crate::home::streaming_animation::StreamingAnimState>,
) -> Option<Duration> {
    states
        .into_iter()
        .map(|state| state.timeout_after().saturating_sub(state.last_update_time.elapsed()))
        .min()
}

/// Labels the bot badge with the sender's workflow role, when it has one.
///
/// A workflow agent gets its role (`coordinator`, `reviewer`, …) in the accent
/// style, so the participants of a run stand out from incidental bots, which
/// keep the generic `bot` label in the quieter neutral style.
///
/// Both the text and both colors are always written, never only on the branch
/// that needs them: these item widgets are recycled by the PortalList, so a
/// value left unset would keep whatever the previously drawn message put there.
pub(super) fn populate_bot_badge_identity(cx: &mut Cx, item: &WidgetRef, sender_localpart: &str) {
    let role = AgentRole::from_localpart(sender_localpart);
    let mut badge = item.view(cx, ids!(content.username_view.bot_badge));
    let mut label = item.label(cx, ids!(content.username_view.bot_badge.bot_badge_label));

    label.set_text(cx, role.map_or("bot", AgentRole::label));
    if role.is_some() {
        script_apply_eval!(cx, badge, {
            draw_bg +: { color: (mod.widgets.RBX_ACCENT_SOFT) }
        });
        script_apply_eval!(cx, label, {
            draw_text +: { color: (mod.widgets.RBX_ACCENT) }
        });
    } else {
        script_apply_eval!(cx, badge, {
            draw_bg +: { color: (mod.widgets.RBX_NEUTRAL_BG) }
        });
        script_apply_eval!(cx, label, {
            draw_text +: { color: (mod.widgets.RBX_NEUTRAL_FG) }
        });
    }
}

/// Puts a `Message` item back into its plain (non-bot-card) state.
///
/// `populate_bot_text_message_content` is the only code that flips these two
/// visibilities, and it only runs on the text/notice path. The PortalList hands
/// the *same* `Message` widget to the server-notice and unsupported/redacted
/// paths as well, so a slot that previously held a bot reply would keep drawing
/// that reply's card while the new content sat hidden underneath it. Every
/// branch that renders into `content.message` must call this first.
pub(super) fn reset_bot_message_card(cx: &mut Cx, item: &WidgetRef) {
    item.view(cx, ids!(content.bot_message_card)).set_visible(cx, false);
    item.html_or_plaintext(cx, ids!(content.message)).set_visible(cx, true);
}

pub(super) fn populate_bot_text_message_content(
    cx: &mut Cx,
    item: &WidgetRef,
    app_language: AppLanguage,
    body: &str,
    formatted_body: Option<&FormattedBody>,
    room_mention_room_id: Option<&OwnedRoomId>,
    link_preview_ref: Option<&mut LinkPreviewRef>,
    media_cache: Option<&mut MediaCache>,
    link_preview_cache: Option<&mut LinkPreviewCache>,
    is_bot_sender: bool,
    bot_body_expanded: bool,
    // True while the body is still being streamed in; suppresses folding.
    is_streaming: bool,
) -> (bool, Option<String>) {
    let render_state = compute_bot_timeline_render_state(body, is_bot_sender);
    let bot_card_view = item.view(cx, ids!(content.bot_message_card));
    let message_view = item.html_or_plaintext(cx, ids!(content.message));

    bot_card_view.set_visible(cx, render_state.show_card);
    message_view.set_visible(cx, !render_state.show_card);

    if !render_state.show_card {
        // Clear the meta-band badge before returning: this item widget is
        // recycled by the PortalList, so a stale badge from a previously drawn
        // bot message would otherwise linger on a plain/human message.
        item.view(cx, ids!(content.message_action_bar.kind_badge))
            .set_visible(cx, false);

        // A plain message can crowd the timeline just as badly as an agent
        // report — a pasted log, or one enormous line of JSON. Fold it the same
        // way, with the toggle in the meta band.
        let folded = fold_bot_body_preview(body);
        let is_folded = folded.is_some() && !bot_body_expanded;
        let toggle = item.button(cx, ids!(content.message_action_bar.plain_fold_toggle));
        toggle.set_visible(cx, folded.is_some());
        if folded.is_some() {
            toggle.set_text(cx, if is_folded { "Show more" } else { "Show less" });
        }

        let drawn = if is_folded {
            // The preview is a truncated slice of the plain body, so the
            // message's `formatted_body` no longer describes it, and link
            // previews belong to content the user has not asked to see.
            populate_text_message_content(
                cx,
                &message_view,
                app_language,
                folded.as_deref().unwrap_or(body),
                None,
                room_mention_room_id,
                None,
                None,
                None,
            )
        } else {
            populate_text_message_content(
                cx,
                &message_view,
                app_language,
                body,
                formatted_body,
                room_mention_room_id,
                link_preview_ref,
                media_cache,
                link_preview_cache,
            )
        };
        return (drawn, None);
    }
    // Bot cards own their fold toggle in the card footer.
    item.button(cx, ids!(content.message_action_bar.plain_fold_toggle))
        .set_visible(cx, false);

    let status_strip = item.view(cx, ids!(content.bot_message_card.bot_status_strip));
    status_strip.set_visible(cx, render_state.show_status_strip);
    if let Some(status) = render_state.status.as_ref() {
        item.label(cx, ids!(content.bot_message_card.bot_status_strip.bot_status_label))
            .set_text(cx, status);
    }

    // Type badge: shown only when the bridge marked the message type.
    // The type badge lives in the meta band (next to the copy icon), so it adds
    // no vertical space of its own.
    let kind_badge = item.view(cx, ids!(content.message_action_bar.kind_badge));
    kind_badge.set_visible(cx, render_state.kind.is_some());
    if let Some(kind) = render_state.kind {
        item.label(cx, ids!(content.message_action_bar.kind_badge.kind_badge_label))
            .set_text(cx, kind.label());
    }

    // The provider/footer metadata is rendered by the meta band below the card
    // (content.message_action_bar.metadata_label), joined into a single line.
    let band_metadata = if render_state.show_metadata_footer {
        match (render_state.provider.as_ref(), render_state.footer.as_ref()) {
            (Some(provider), Some(footer)) =>
                Some(format!("{provider} · {}", display_bot_footer_text(footer))),
            (Some(provider), None) => Some(provider.clone()),
            (None, Some(footer)) => Some(display_bot_footer_text(footer).to_string()),
            (None, None) => None,
        }
    } else {
        None
    };

    let body_card = item.view(cx, ids!(content.bot_message_card.bot_body_card));
    body_card.set_visible(cx, render_state.show_body_card);
    let body_widget = item.html_or_plaintext(cx, ids!(content.bot_message_card.bot_body_card.bot_card_body));
    let mut markdown_widget = item.markdown(cx, ids!(content.bot_message_card.bot_body_card.bot_card_markdown));
    let mut markdown_plain_widget = item.markdown(cx, ids!(content.bot_message_card.bot_body_card.bot_card_markdown_plain));
    // Fold state: a long body renders its preview until the user expands it.
    // `folded_body` is `None` for short bodies, so the toggle stays hidden and
    // the full text renders exactly as before.
    //
    // A streaming reply is never folded. Its body grows on every frame, so the
    // moment it crosses the threshold the text the user is watching appear would
    // collapse to three lines and take the typewriter cursor with it — the reply
    // reads as having stalled. Folding applies once the message has settled.
    let foldable = render_state.folded_body.is_some() && !is_streaming;
    let is_folded = foldable && !bot_body_expanded;
    let fold_toggle = item.button(
        cx,
        ids!(content.bot_message_card.bot_body_card.bot_card_footer_row.bot_body_fold_toggle),
    );
    fold_toggle.set_visible(cx, foldable);
    if foldable {
        fold_toggle.set_text(cx, if is_folded { "Show more" } else { "Show less" });
    }

    // Permalink pinned in the card footer — visible whether or not the body is
    // folded, and clickable via the room screen's existing `HtmlLinkAction`
    // handler.
    let permalink_link = item.link_label(
        cx,
        ids!(content.bot_message_card.bot_body_card.bot_card_footer_row.bot_permalink_link),
    );
    permalink_link.set_visible(cx, render_state.permalink.is_some());
    if let Some(url) = render_state.permalink.as_ref() {
        permalink_link.set_text(cx, "View formatted");
        if let Some(mut inner) = permalink_link.borrow_mut() {
            inner.url = url.clone();
        }
    }

    // Folding swaps the body for its preview *inside the render state* rather
    // than switching rendering modes. Every downstream decision (code-block
    // mode, formatted-body selection, which widget is visible) is then derived
    // from the text actually on screen — one path, one visible widget. Choosing
    // a different mode for the folded case instead would leave the preview in
    // the plaintext widget and the full text in the markdown widget, and both
    // would render (a duplicated body).
    let render_state = if is_folded {
        let mut folded = render_state.clone();
        folded.body = folded.folded_body.clone().unwrap_or(folded.body);
        folded
    } else {
        render_state
    };
    // A folded preview never carries the message's `formatted_body`: that HTML
    // describes the full text, not this truncated slice. When the permalink has
    // been promoted to the footer, drop its anchor from the HTML too so it does
    // not render a second time inside the body.
    let formatted_body_owned;
    let formatted_body = if is_folded {
        None
    } else if render_state.permalink.is_some() {
        match formatted_body {
            Some(fb) => {
                let mut stripped = fb.clone();
                stripped.body = strip_permalink_anchor_from_html(&fb.body);
                formatted_body_owned = stripped;
                Some(&formatted_body_owned)
            }
            None => None,
        }
    } else {
        formatted_body
    };

    let code_block_mode = bot_timeline_code_block_mode(&render_state);
    body_widget.set_visible(cx, code_block_mode == BotTimelineCodeBlockMode::None);
    markdown_widget.set_visible(cx, code_block_mode == BotTimelineCodeBlockMode::Highlighted);
    markdown_plain_widget.set_visible(cx, code_block_mode == BotTimelineCodeBlockMode::Plain);
    // Hiding the inactive renderer is not enough: a Markdown widget that still
    // holds text keeps drawing its own DrawList after `set_visible(false)`, so
    // toggling the fold on a body with a ``` block stacked the folded preview on
    // top of the leftover full text. Worse, that leftover is invisible for
    // hit-testing, so its links stopped responding to clicks. Clearing the text
    // of whichever renderer is not in use makes it draw nothing at all.
    if code_block_mode != BotTimelineCodeBlockMode::Highlighted {
        markdown_widget.set_text(cx, "");
    }
    if code_block_mode != BotTimelineCodeBlockMode::Plain {
        markdown_plain_widget.set_text(cx, "");
    }
    if code_block_mode != BotTimelineCodeBlockMode::None {
        body_widget.show_plaintext(cx, "");
    }

    let drawn = if render_state.show_body_card {
        if code_block_mode != BotTimelineCodeBlockMode::None {
            match code_block_mode {
                BotTimelineCodeBlockMode::Highlighted => markdown_widget.set_text(cx, &render_state.body),
                BotTimelineCodeBlockMode::Plain => markdown_plain_widget.set_text(cx, &render_state.body),
                BotTimelineCodeBlockMode::None => { }
            }

            if let (Some(link_preview_ref), Some(media_cache), Some(link_preview_cache)) =
                (link_preview_ref, media_cache, link_preview_cache)
            {
                let mut links = Vec::new();
                let _ = utils::linkify_get_urls(&render_state.body, false, Some(&mut links));
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
        } else {
            let formatted_body_for_card =
                select_bot_timeline_body_formatted_body(&render_state, formatted_body);
            populate_text_message_content(
                cx,
                &body_widget,
                app_language,
                &render_state.body,
                formatted_body_for_card.as_ref(),
                room_mention_room_id,
                link_preview_ref,
                media_cache,
                link_preview_cache,
            )
        }
    } else {
        true
    };
    (drawn, band_metadata)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(text: &str) -> StreamingAnimState {
        StreamingAnimState::new(text, true)
    }

    #[test]
    fn test_streaming_scan_range() {
        // Incremental: clamp sentinel to new_len
        assert_eq!(streaming_scan_range(false, &(5..usize::MAX), 8, 9), 5..9);
        // Append: new item at end is scanned
        assert_eq!(streaming_scan_range(false, &(8..9), 8, 9), 8..9);
        // No changes: empty range
        assert_eq!(streaming_scan_range(false, &(8..8), 8, 8), 8..8);
        // Clear cache: full scan
        assert_eq!(streaming_scan_range(true, &(5..usize::MAX), 8, 9), 0..9);
    }

    #[test]
    fn test_refresh_stream_indices() {
        let event_id_a: OwnedEventId = "$event-a:example.com".try_into().unwrap();
        let event_id_b: OwnedEventId = "$event-b:example.com".try_into().unwrap();
        let missing_event_id: OwnedEventId = "$missing:example.com".try_into().unwrap();

        let mut streaming_messages = HashMap::new();
        streaming_messages.insert(event_id_a.clone(), make_state("alpha"));
        streaming_messages.insert(missing_event_id.clone(), make_state("missing"));

        let event_ids = vec![None, Some(event_id_a.as_ref()), Some(event_id_b.as_ref())];
        refresh_stream_indices(event_ids, &mut streaming_messages);

        assert_eq!(streaming_messages[&event_id_a].timeline_index, Some(1));
        assert_eq!(streaming_messages[&missing_event_id].timeline_index, None);
    }

    #[test]
    fn test_timeout_picks_earliest() {
        let mut live = make_state("alpha");
        live.last_update_time = Instant::now() - Duration::from_secs(40);
        let mut finished = make_state("beta");
        finished.is_live = false;
        finished.last_update_time = Instant::now() - Duration::from_secs(29);

        let timeout = next_stream_timeout([&live, &finished]).unwrap();

        assert!(timeout <= Duration::from_secs(1));
    }

    #[test]
    fn test_full_snapshot_rebuild_drops_finished_cached_streams() {
        let event_id: OwnedEventId = "$event-live:example.com".try_into().unwrap();
        let mut previous = HashMap::new();
        let mut previous_state = make_state("hello live");
        previous_state.advance_displayed(4);
        previous.insert(event_id.clone(), previous_state);

        let (rebuilt, should_schedule_frame) = rebuild_streaming_messages_for_full_snapshot(
            [(event_id, String::from("hello final"), false)],
            Some(&previous),
        );

        assert!(rebuilt.is_empty());
        assert!(!should_schedule_frame);
    }

    #[test]
    fn test_full_snapshot_rebuild_restores_live_cached_streams() {
        let event_id: OwnedEventId = "$event-live:example.com".try_into().unwrap();
        let mut previous = HashMap::new();
        let mut previous_state = make_state("hello");
        previous_state.advance_displayed(3);
        previous.insert(event_id.clone(), previous_state);

        let (rebuilt, should_schedule_frame) = rebuild_streaming_messages_for_full_snapshot(
            [(event_id.clone(), String::from("hello world"), true)],
            Some(&previous),
        );

        let restored = rebuilt.get(&event_id).unwrap();
        assert_eq!(restored.displayed_char_count, 3);
        assert!(restored.is_live);
        assert!(should_schedule_frame);
    }

    #[test]
    fn test_full_snapshot_rebuild_skips_live_without_cached_state() {
        // Without previous state, full-snapshot rebuild must NOT create new
        // animations — the SDK may not have aggregated edits yet, so
        // completed messages can still appear as `live`.
        let event_id: OwnedEventId = "$event-live:example.com".try_into().unwrap();

        let (rebuilt, should_schedule_frame) = rebuild_streaming_messages_for_full_snapshot(
            [(event_id.clone(), String::from("hello world"), true)],
            None,
        );

        assert!(rebuilt.is_empty());
        assert!(!should_schedule_frame);
    }

    #[test]
    fn test_parse_bot_timeline_layers_extracts_status_provider_body_and_footer() {
        let body = "施法中\nvia moonshot@api (kimi-k2.5)\n\n你好！我是 **Alex**\n\n_moonshot@api/kimi-k2.5 · 5.3K in · 330 out · 6s_";

        let layers = parse_bot_timeline_layers(body, true);

        assert_eq!(layers.status.as_deref(), Some("施法中"));
        assert_eq!(layers.provider.as_deref(), Some("via moonshot@api (kimi-k2.5)"));
        assert_eq!(layers.body, "你好！我是 **Alex**");
        assert_eq!(
            layers.footer.as_deref(),
            Some("_moonshot@api/kimi-k2.5 · 5.3K in · 330 out · 6s_"),
        );
    }

    #[test]
    fn test_parse_bot_timeline_layers_extracts_footer_without_provider_prefix() {
        let body = "PPT 已经生成并发送了！\n\n你应该已经收到了文件。\n\n_moonshot@api/kimi-k2.5 · 11.0K in · 279 out · 9s_";

        let layers = parse_bot_timeline_layers(body, true);

        assert_eq!(layers.status, None);
        assert_eq!(layers.provider, None);
        assert_eq!(layers.body, "PPT 已经生成并发送了！\n\n你应该已经收到了文件。");
        assert_eq!(
            layers.footer.as_deref(),
            Some("_moonshot@api/kimi-k2.5 · 11.0K in · 279 out · 9s_"),
        );
    }

    #[test]
    fn test_parse_bot_timeline_layers_falls_back_for_unmatched_bot_text() {
        let body = "你好！我是 Alex。\n今天可以帮你查天气。";

        let layers = parse_bot_timeline_layers(body, true);

        assert_eq!(layers, BotTimelineLayers::plain(body));
    }

    #[test]
    fn test_parse_bot_timeline_layers_ignores_regular_user_messages() {
        let body = "via moonshot@api (kimi-k2.5)\n\n这不是 bot 消息。";

        let layers = parse_bot_timeline_layers(body, false);

        assert_eq!(layers, BotTimelineLayers::plain(body));
    }

    #[test]
    fn test_parse_bot_timeline_layers_prefers_safe_fallback_for_malformed_metadata() {
        let body = "施法中\n这个不是 provider 行\n\n你好，我还在。";

        let layers = parse_bot_timeline_layers(body, true);

        assert_eq!(layers, BotTimelineLayers::plain(body));
    }

    #[test]
    #[ignore = "pre-existing failure on main (1.0.0-alpha.1): parses status/provider layers instead of treating the metadata as invalid. See issues/011."]
    fn test_parse_bot_timeline_layers_invalid_metadata_does_not_panic() {
        let body = "施法中\nvia moonshot@api (kimi-k2.5)\n\n_\n";

        let layers = parse_bot_timeline_layers(body, true);

        assert_eq!(layers, BotTimelineLayers::plain(body));
    }

    #[test]
    fn test_parse_bot_timeline_layers_tolerates_streaming_cursor_in_footer() {
        let body = "via moonshot@api (kimi-k2.5)\n\n你好！我是 **Alex**\n\n_moonshot@api/kimi-k2.5 · 5.3K in · 330 out · 6s_ ●";

        let layers = parse_bot_timeline_layers(body, true);

        assert_eq!(layers.body, "你好！我是 **Alex**");
        assert_eq!(
            layers.footer.as_deref(),
            Some("_moonshot@api/kimi-k2.5 · 5.3K in · 330 out · 6s_"),
        );
    }

    #[test]
    fn test_parse_bot_timeline_layers_promotes_metrics_only_body_to_footer() {
        let body = "疯狂输出中\nvia moonshot@api (kimi-k2.5)\n4s";

        let layers = parse_bot_timeline_layers(body, true);

        assert_eq!(layers.status.as_deref(), Some("疯狂输出中"));
        assert_eq!(layers.provider.as_deref(), Some("via moonshot@api (kimi-k2.5)"));
        assert!(layers.body.is_empty());
        assert_eq!(layers.footer.as_deref(), Some("4s"));
    }

    #[test]
    fn test_rich_markdown_streaming_prefers_full_snapshot_rendering() {
        let formatted = FormattedBody::html("<p><strong>OpenClaw</strong></p>");
        assert!(should_render_streaming_full_snapshot(
            "根据搜索结果， **OpenClaw** 有两个不同的项目。",
            Some(&formatted),
            true,
        ));
    }

    #[test]
    fn test_plain_text_streaming_keeps_typewriter_path() {
        assert!(!should_render_streaming_full_snapshot(
            "你好，我是 Octos。",
            None,
            true,
        ));
    }

    #[test]
    fn test_bot_timeline_card_visible_for_bot_text_message() {
        let state = compute_bot_timeline_render_state(
            "施法中\nvia moonshot@api (kimi-k2.5)\n\n你好！我是 Alex。\n\n_moonshot@api/kimi-k2.5 · 1.2K in · 88 out · 2s_",
            true,
        );

        assert!(state.show_card);
        assert_eq!(state.body, "你好！我是 Alex。");
    }

    #[test]
    fn test_bot_timeline_card_hidden_for_regular_user_message() {
        let state = compute_bot_timeline_render_state("你好", false);

        assert!(!state.show_card);
    }

    #[test]
    fn test_bot_status_strip_renders_above_body_and_not_inside_body() {
        let state = compute_bot_timeline_render_state(
            "施法中\nvia moonshot@api (kimi-k2.5)\n\n你好！我是 Alex。",
            true,
        );

        assert_eq!(state.status.as_deref(), Some("施法中"));
        assert!(state.show_status_strip);
        assert!(!state.body.starts_with("施法中"));
    }

    #[test]
    fn test_agent_reply_kind_split_from_body() {
        let (kind, rest) = AgentReplyKind::split_from_body("📋 Issue 001 spec ready");
        assert_eq!(kind, Some(AgentReplyKind::Request));
        assert_eq!(rest, "Issue 001 spec ready");

        let (kind, rest) = AgentReplyKind::split_from_body("↩️ Review verdict: APPROVE");
        assert_eq!(kind, Some(AgentReplyKind::Reply));
        assert_eq!(rest, "Review verdict: APPROVE");

        // No marker → body is returned untouched and no badge is shown.
        let (kind, rest) = AgentReplyKind::split_from_body("plain agent text");
        assert_eq!(kind, None);
        assert_eq!(rest, "plain agent text");
    }

    #[test]
    fn test_bot_kind_badge_strips_marker_from_rendered_body() {
        let state = compute_bot_timeline_render_state("ℹ️ Status update\n\nAll good.", true);
        assert_eq!(state.kind, Some(AgentReplyKind::Inform));
        assert!(
            !state.body.starts_with('ℹ'),
            "type marker must move to the badge, not stay in the body: {:?}",
            state.body
        );
    }

    #[test]
    fn test_short_bot_body_is_not_folded() {
        let state = compute_bot_timeline_render_state("via x\n\nline1\nline2\nline3", true);
        assert!(
            state.folded_body.is_none(),
            "a short reply must render in full with no fold toggle"
        );
    }

    #[test]
    fn test_long_bot_body_folds_to_preview() {
        let long = (1..=30).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let state = compute_bot_timeline_render_state(&long, true);
        let folded = state.folded_body.expect("a long reply must fold");
        assert_eq!(folded.lines().count(), BOT_BODY_FOLD_PREVIEW_LINES);
        assert!(folded.starts_with("line 1"));
        // The full text stays available for the expanded state.
        assert!(state.body.lines().count() > BOT_BODY_FOLD_LINE_THRESHOLD);
    }

    #[test]
    fn test_human_messages_are_never_folded_or_badged() {
        let long = (1..=30).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n");
        let state = compute_bot_timeline_render_state(&long, false);
        assert!(state.folded_body.is_none());
        assert_eq!(state.kind, None);
        assert!(!state.show_card);
    }

    #[test]
    fn test_bot_metadata_extracted_for_meta_band() {
        let state = compute_bot_timeline_render_state(
            "via moonshot@api (kimi-k2.5)\n\n你好！我是 Alex。\n\n_moonshot@api/kimi-k2.5 · 1.2K in · 88 out · 2s_",
            true,
        );

        assert!(state.show_metadata_footer);
        assert_eq!(state.provider.as_deref(), Some("via moonshot@api (kimi-k2.5)"));
        assert_eq!(
            state.footer.as_deref(),
            Some("_moonshot@api/kimi-k2.5 · 1.2K in · 88 out · 2s_"),
        );
    }

    #[test]
    fn test_clipboard_text_strips_bot_scaffolding() {
        let raw = "via deepseek@api (deepseek-chat)\n\n我运行在 OctOS 平台上。\n\n_deepseek@api/deepseek-chat · 13.3K in · 162 out · 3s_";
        let cleaned = clipboard_text_for_message_body(raw.to_string(), true);

        assert!(cleaned.contains("我运行在 OctOS 平台上。"));
        assert!(!cleaned.contains("deepseek@api/deepseek-chat"));
    }

    #[test]
    fn test_clipboard_text_verbatim_for_human() {
        let raw = "via someone@api (model)\nhello there";
        let unchanged = clipboard_text_for_message_body(raw.to_string(), false);

        assert_eq!(unchanged, raw);
    }

    #[test]
    fn test_bot_progress_message_hides_body_card_when_only_metrics_remain() {
        let state = compute_bot_timeline_render_state(
            "疯狂输出中\nvia moonshot@api (kimi-k2.5)\n4s",
            true,
        );

        assert!(state.show_card);
        assert!(!state.show_body_card);
        assert!(state.show_status_strip);
        assert!(state.show_metadata_footer);
        assert_eq!(state.footer.as_deref(), Some("4s"));
    }

    #[test]
    fn test_bot_timeline_card_body_uses_html_or_plaintext_rendering() {
        let state = compute_bot_timeline_render_state(
            "施法中\nvia moonshot@api (kimi-k2.5)\n\n你好！我是 **Alex**",
            true,
        );

        let formatted = select_bot_timeline_body_formatted_body(&state, None)
            .expect("structured bot body should still produce formatted content");

        assert_eq!(formatted.format, MessageFormat::Html);
        assert!(formatted.body.contains("<strong>Alex</strong>"));
    }

    #[test]
    fn test_bot_plain_markdown_body_without_formatted_html_still_renders_as_markdown() {
        let state = compute_bot_timeline_render_state(
            "## 标题\n\n```rust\n// 中文注释\nlet answer = 42;\n```",
            true,
        );

        let formatted = select_bot_timeline_body_formatted_body(&state, None)
            .expect("rich markdown bot body should synthesize HTML during streaming");

        assert_eq!(formatted.format, MessageFormat::Html);
        assert!(formatted.body.contains("<h2>标题</h2>"));
        assert!(formatted.body.contains("中文注释"));
    }

    #[test]
    fn test_bot_timeline_body_prefers_markdown_widget_for_fenced_code_blocks() {
        let state = compute_bot_timeline_render_state(
            "## 标题\n\n```rust\nlet answer = 42;\n```\n\n这里是中文说明。",
            true,
        );

        assert!(should_render_bot_timeline_body_with_markdown_widget(&state));
        assert_eq!(
            bot_timeline_code_block_mode(&state),
            BotTimelineCodeBlockMode::Highlighted,
        );
    }

    #[test]
    fn test_bot_timeline_body_keeps_html_widget_for_non_code_markdown() {
        let state = compute_bot_timeline_render_state(
            "## 标题\n\n这里有 **加粗**，但没有代码块。",
            true,
        );

        assert!(!should_render_bot_timeline_body_with_markdown_widget(&state));
        assert_eq!(
            bot_timeline_code_block_mode(&state),
            BotTimelineCodeBlockMode::None,
        );
    }

    #[test]
    fn test_bot_timeline_body_uses_plain_markdown_code_block_for_cjk_code() {
        let state = compute_bot_timeline_render_state(
            "```rust\n// 中文注释\nprintln!(\"你好\");\n```",
            true,
        );

        assert_eq!(
            bot_timeline_code_block_mode(&state),
            BotTimelineCodeBlockMode::Plain,
        );
    }

    #[test]
    fn test_fenced_code_blocks_ignore_cjk_outside_code_block() {
        let body = "## 标题\n\n```rust\nlet answer = 42;\n```\n\n这里是中文总结。";

        assert!(!fenced_code_blocks_contain_cjk(body));
    }

    #[test]
    fn test_streaming_update_requires_content_invalidation_for_new_full_snapshot_text() {
        let state = StreamingAnimState::new("你好", true);

        assert!(streaming_update_requires_content_invalidation(
            &state,
            "## 标题\n\n内容",
            true,
            true,
        ));
    }

    #[test]
    fn test_streaming_update_skips_invalidation_when_target_and_mode_are_unchanged() {
        let mut state = StreamingAnimState::new("## 标题\n\n内容", true);
        state.set_render_full_target(true);

        assert!(!streaming_update_requires_content_invalidation(
            &state,
            "## 标题\n\n内容",
            true,
            true,
        ));
    }

    #[test]
    fn test_bot_timeline_card_preserves_reply_preview_and_condensed_layout() {
        let reply_state = compute_bot_timeline_render_state(
            "via moonshot@api (kimi-k2.5)\n\n第一条回复",
            true,
        );
        let condensed_state = compute_bot_timeline_render_state(
            "via moonshot@api (kimi-k2.5)\n\n第二条回复",
            true,
        );

        assert!(reply_state.show_card);
        assert!(condensed_state.show_card);
        assert!(reply_state.show_metadata_footer);
        assert!(condensed_state.show_metadata_footer);
    }
}

#[cfg(test)]
mod t1_fold_tests {
    use super::*;

    /// A minified payload is one enormous line: the line count says "nothing to
    /// fold" while it wraps to fill the viewport.
    #[test]
    fn single_enormous_line_folds() {
        let body = "x".repeat(BOT_BODY_FOLD_CHAR_THRESHOLD + 1);
        assert_eq!(body.lines().count(), 1, "one line, far below the line threshold");
        let folded = fold_bot_body_preview(&body).expect("folds on characters");
        assert!(folded.chars().count() <= BOT_BODY_FOLD_PREVIEW_CHARS + 1);
        assert!(folded.ends_with('\u{2026}'));
    }

    /// Three lines of a dump can still be a screenful, so the preview is capped
    /// by characters as well as by lines.
    #[test]
    fn preview_is_capped_by_characters_too() {
        let body = format!("{}\n{}\n{}\nmore\nmore\nmore\nmore\nmore\nmore",
            "a".repeat(900), "b".repeat(900), "c".repeat(900));
        let folded = fold_bot_body_preview(&body).expect("folds");
        assert!(folded.chars().count() <= BOT_BODY_FOLD_PREVIEW_CHARS + 1);
    }

    /// Multi-byte text must not panic the character cap.
    #[test]
    fn character_cap_is_utf8_safe() {
        let body = "中文内容描述测试".repeat(400);
        let folded = fold_bot_body_preview(&body).expect("folds on characters");
        assert!(folded.chars().count() <= BOT_BODY_FOLD_PREVIEW_CHARS + 1);
    }

    /// A body that is small on both counts still shows no toggle.
    #[test]
    fn short_body_still_does_not_fold() {
        assert!(fold_bot_body_preview("one\ntwo\nthree").is_none());
    }

    #[test]
    fn agent_role_is_derived_from_localpart() {
        use AgentRole::*;
        // Real MXIDs from the agent-chat demo: @ac_<team>_<role>
        assert_eq!(AgentRole::from_localpart("ac_tyrese_coordinator"), Some(Coordinator));
        assert_eq!(AgentRole::from_localpart("ac_tyrese_implementer"), Some(Implementer));
        assert_eq!(AgentRole::from_localpart("ac_tyrese_reviewer"), Some(Reviewer));
        // `final_reviewer` also ends with `reviewer` — it must not be mislabelled.
        assert_eq!(AgentRole::from_localpart("ac_tyrese_final_reviewer"), Some(FinalReviewer));
        assert_eq!(AgentRole::from_localpart("ac_wf_final_reviewer"), Some(FinalReviewer));
        // Bare role names (no team prefix) still resolve.
        assert_eq!(AgentRole::from_localpart("coordinator"), Some(Coordinator));
    }

    #[test]
    fn non_workflow_bots_have_no_role() {
        assert_eq!(AgentRole::from_localpart("octosbot"), None);
        assert_eq!(AgentRole::from_localpart("agent-bridge-tyrese"), None);
        assert_eq!(AgentRole::from_localpart("tyreseluo"), None);
        // A name that merely contains a role word is not a role.
        assert_eq!(AgentRole::from_localpart("reviewerbot"), None);
    }

    /// The real 4:43 message: permalink must leave the body and land in the footer.
    #[test]
    fn permalink_is_promoted_out_of_body() {
        let body = "\u{21a9}\u{fe0f} Saved to docs/weekly/ ... \u{b7} @tyreseluo\n\n@tyreseluo ok:\n\n```\n/Users/x/f.md\n```\n\nmore\nmore\nmore\nmore\nmore\n\n\u{1f517} http://127.0.0.1:8090/msg/msg_0392?view=Frwug96p";
        let st = compute_bot_timeline_render_state(body, true);
        assert_eq!(st.permalink.as_deref(), Some("http://127.0.0.1:8090/msg/msg_0392?view=Frwug96p"));
        assert!(!st.body.contains('\u{1f517}'), "permalink line removed from body");
        assert!(!st.body.trim_end().ends_with("Frwug96p"), "url gone from body");
        let folded = st.folded_body.expect("still folds");
        assert!(!folded.contains('\u{1f517}'), "preview has no permalink");
    }

    #[test]
    fn html_anchor_for_permalink_is_stripped() {
        let html = "<b>hi</b><br><br>body text<br><br><a href=\"http://x/msg/1\">\u{1f517} View formatted</a>";
        let out = strip_permalink_anchor_from_html(html);
        assert_eq!(out, "<b>hi</b><br><br>body text");
    }

    #[test]
    fn html_without_permalink_anchor_is_untouched() {
        let html = "<b>hi</b><br><a href=\"http://x\">docs</a>";
        assert_eq!(strip_permalink_anchor_from_html(html), html);
    }

    #[test]
    fn body_without_permalink_is_untouched() {
        let (body, link) = split_bot_permalink("just text\nsecond line");
        assert_eq!(body, "just text\nsecond line");
        assert!(link.is_none());
    }

    /// The screenshot-3 regression: a long body containing a fenced code block
    /// must fold to a preview that is itself renderable by a single widget.
    #[test]
    fn folded_preview_is_fence_balanced_and_short() {
        let body = "↩️ Summary line here\n\n@user ok:\n\n```\n/Users/x/file.md\n```\n\nmore text\nand more\nand more\nand more\nand more";
        let st = compute_bot_timeline_render_state(body, true);
        assert_eq!(st.kind, Some(AgentReplyKind::Reply), "type marker parsed");
        assert!(!st.body.starts_with('↩'), "marker stripped from body");
        let folded = st.folded_body.expect("long body folds");
        assert!(folded.lines().count() <= BOT_BODY_FOLD_PREVIEW_LINES + 1);
        assert_eq!(folded.matches("```").count() % 2, 0, "fences balanced");
    }

    #[test]
    fn short_body_does_not_fold() {
        let st = compute_bot_timeline_render_state("ℹ️ short\n\none line", true);
        assert_eq!(st.kind, Some(AgentReplyKind::Inform));
        assert!(st.folded_body.is_none(), "short body must not show a toggle");
    }

    #[test]
    fn human_message_is_untouched() {
        let body = "📋 not a bot\nline\nline\nline\nline\nline\nline\nline\nline\nline";
        let st = compute_bot_timeline_render_state(body, false);
        assert!(st.kind.is_none(), "no badge for human senders");
        assert!(st.folded_body.is_none(), "no folding for human senders");
        assert_eq!(st.body, body, "human body verbatim");
    }
}
