//! Octos / agent-chat interactive messages: action buttons, the approval
//! protocol (parsing, expiry, verdict/response builders), and their
//! populate pass.

use super::*;

pub(super) const MAX_OCTOS_ACTION_BUTTONS: usize = 6;
pub(super) const AGENTCHAT_APPROVAL_EVENT_KEY: &str = "com.agentchat.approval";
pub(super) const AGENTCHAT_APPROVAL_REQUEST_MSGTYPE: &str = "com.agentchat.approval.request.v1";
pub(super) const AGENTCHAT_APPROVAL_STATUS_MSGTYPE: &str = "com.agentchat.approval.status.v1";
pub(super) const AGENTCHAT_APPROVAL_VERDICT_MSGTYPE: &str = "com.agentchat.approval.verdict.v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OctosActionStyle {
    Primary,
    Secondary,
    Danger,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OctosActionButton {
    id: String,
    label: String,
    style: OctosActionStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActionButtonRenderSlot {
    id: String,
    label: String,
    style: OctosActionStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SelectedOctosActionState {
    id: String,
    label: String,
    style: OctosActionStyle,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ApprovalCardRenderState {
    title: String,
    summary: String,
    buttons_enabled: bool,
    expired: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ActionButtonRenderState {
    show_container: bool,
    show_button_row: bool,
    approval_card: Option<ApprovalCardRenderState>,
    buttons_enabled: bool,
    visible_slots: Vec<ActionButtonRenderSlot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ParsedOctosActionPayload {
    approval_request: Option<OctosApprovalRequest>,
    actions: Vec<OctosActionButton>,
    malformed_approval_request: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OctosApprovalRiskLevel {
    Normal,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OctosApprovalTimeoutBehavior {
    Notify,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ApprovalProtocol {
    Octos,
    AgentChat {
        agent: String,
        project: String,
        project_room_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OctosApprovalRequest {
    protocol: ApprovalProtocol,
    request_id: String,
    tool_name: String,
    tool_args_digest: String,
    title: String,
    summary: String,
    risk_level: OctosApprovalRiskLevel,
    authorized_approvers: Vec<String>,
    expires_at: String,
    on_timeout: OctosApprovalTimeoutBehavior,
}

pub(super) fn parse_octos_action_style(style: Option<&str>) -> OctosActionStyle {
    match style {
        Some("primary") => OctosActionStyle::Primary,
        Some("danger") => OctosActionStyle::Danger,
        _ => OctosActionStyle::Secondary,
    }
}

pub(super) fn effective_octos_message_content(content: &serde_json::Value) -> &serde_json::Value {
    content.get("m.new_content").unwrap_or(content)
}

pub(super) fn latest_effective_event_content_json(
    event_tl_item: &EventTimelineItem,
) -> Option<serde_json::Value> {
    event_tl_item.latest_edit_json()
        .or_else(|| event_tl_item.original_json())
        .and_then(|raw| raw.get_field::<serde_json::Value>("content").ok())
        .flatten()
        .map(|content| effective_octos_message_content(&content).clone())
}

pub(super) fn original_event_content_json(
    event_tl_item: &EventTimelineItem,
) -> Option<serde_json::Value> {
    event_tl_item.original_json()
        .and_then(|raw| raw.get_field::<serde_json::Value>("content").ok())
        .flatten()
}

pub(super) fn event_raw_json_contains_any(
    event_tl_item: &EventTimelineItem,
    markers: &[&str],
) -> bool {
    event_tl_item
        .latest_edit_json()
        .into_iter()
        .chain(event_tl_item.original_json())
        .any(|raw| {
            let json = raw.json().get();
            markers.iter().any(|marker| json.contains(marker))
        })
}

pub(super) fn forwardable_room_message_content_from_json(
    content: serde_json::Value,
) -> Option<RoomMessageEventContent> {
    let mut message = serde_json::from_value::<RoomMessageEventContent>(content).ok()?;
    let is_forwardable = matches!(
        &message.msgtype,
        MessageType::Text(..) | MessageType::Notice(..) | MessageType::Emote(..)
    );
    message.relates_to = None;
    message.tsp_signature = None;
    is_forwardable.then_some(message)
}

pub(super) fn parse_octos_approval_risk_level(value: Option<&str>) -> Option<OctosApprovalRiskLevel> {
    match value {
        Some("normal") => Some(OctosApprovalRiskLevel::Normal),
        Some("critical") => Some(OctosApprovalRiskLevel::Critical),
        _ => None,
    }
}

pub(super) fn parse_octos_approval_timeout_behavior(value: Option<&str>) -> Option<OctosApprovalTimeoutBehavior> {
    match value {
        Some("notify") => Some(OctosApprovalTimeoutBehavior::Notify),
        _ => None,
    }
}

pub(super) fn parse_octos_approval_request_from_content(content: &serde_json::Value) -> Option<OctosApprovalRequest> {
    let approval = content.get("org.octos.approval_request")?;
    let request_id = approval.get("request_id")?.as_str()?.trim();
    let tool_name = approval.get("tool_name")?.as_str()?.trim();
    let tool_args_digest = approval.get("tool_args_digest")?.as_str()?.trim();
    let title = approval.get("title")?.as_str()?.trim();
    let summary = approval.get("summary")?.as_str()?.trim();
    let risk_level = parse_octos_approval_risk_level(
        approval.get("risk_level").and_then(|value| value.as_str()).map(str::trim),
    )?;

    let approvers = approval.get("authorized_approvers")?.as_array()?;
    let authorized_approvers = approvers
        .iter()
        .filter_map(|value| value.as_str().map(str::trim))
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
        .collect::<Vec<_>>();
    if authorized_approvers.is_empty() {
        return None;
    }

    let expires_at = approval.get("expires_at")?.as_str()?.trim();
    let on_timeout = parse_octos_approval_timeout_behavior(
        approval.get("on_timeout").and_then(|value| value.as_str()).map(str::trim),
    )?;

    if request_id.is_empty()
        || tool_name.is_empty()
        || tool_args_digest.is_empty()
        || title.is_empty()
        || summary.is_empty()
        || expires_at.is_empty()
    {
        return None;
    }

    Some(OctosApprovalRequest {
        protocol: ApprovalProtocol::Octos,
        request_id: request_id.to_owned(),
        tool_name: tool_name.to_owned(),
        tool_args_digest: tool_args_digest.to_owned(),
        title: title.to_owned(),
        summary: summary.to_owned(),
        risk_level,
        authorized_approvers,
        expires_at: expires_at.to_owned(),
        on_timeout,
    })
}

pub(super) fn is_lowercase_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

pub(super) fn parse_agentchat_approval_actions_from_detail(
    approval: &serde_json::Value,
) -> Vec<OctosActionButton> {
    let Some(actions) = approval.get("actions").and_then(|value| value.as_array()) else {
        return Vec::new();
    };
    if actions.len() != 2 {
        return Vec::new();
    }

    let mut parsed = Vec::with_capacity(2);
    for action in actions {
        let Some(id) = action.get("id").and_then(|value| value.as_str()).map(str::trim) else {
            return Vec::new();
        };
        let Some(label) = action.get("label").and_then(|value| value.as_str()).map(str::trim) else {
            return Vec::new();
        };
        if label.is_empty() {
            return Vec::new();
        }
        parsed.push(OctosActionButton {
            id: id.to_owned(),
            label: label.to_owned(),
            style: parse_octos_action_style(action.get("style").and_then(|value| value.as_str())),
        });
    }

    if parsed[0].id != "approve_once"
        || parsed[0].style != OctosActionStyle::Primary
        || parsed[1].id != "deny"
        || parsed[1].style != OctosActionStyle::Danger
    {
        return Vec::new();
    }
    parsed
}

pub(super) fn parse_agentchat_approval_request_from_content(
    content: &serde_json::Value,
) -> Option<OctosApprovalRequest> {
    if content.get("msgtype").and_then(|value| value.as_str()) != Some(AGENTCHAT_APPROVAL_REQUEST_MSGTYPE) {
        return None;
    }
    let approval = content.get(AGENTCHAT_APPROVAL_EVENT_KEY)?;
    if approval.get("version").and_then(|value| value.as_u64()) != Some(1)
        || approval.get("kind").and_then(|value| value.as_str()) != Some("request")
        || parse_agentchat_approval_actions_from_detail(approval).len() != 2
    {
        return None;
    }

    let agent = approval.get("agent")?.as_str()?.trim();
    let project = approval.get("project")?.as_str()?.trim();
    let project_room_id = approval.get("project_room_id")?.as_str()?.trim();
    let request_id = approval.get("request_id")?.as_str()?.trim();
    let upstream_request_id = approval.get("upstream_request_id")?.as_str()?.trim();
    let input_digest = approval.get("input_digest")?.as_str()?.trim();
    let runtime = approval.get("runtime")?.as_str()?.trim();
    let tool_name = approval.get("tool_name")?.as_str()?.trim();
    let description = approval.get("description")?.as_str()?.trim();
    let input_preview = approval.get("input_preview")?.as_str()?.trim();
    let expires_at = approval.get("expires_at")?.as_u64()?;

    let request_suffix = request_id.strip_prefix("approval_")?;
    if agent.is_empty()
        || project.is_empty()
        || !project_room_id.starts_with('!')
        || !project_room_id.contains(':')
        || !is_lowercase_hex(request_suffix, 32)
        || upstream_request_id.is_empty()
        || !is_lowercase_hex(input_digest, 64)
        || !matches!(runtime, "claude" | "codex")
        || tool_name.is_empty()
        || expires_at == 0
    {
        return None;
    }

    let summary = match (description.is_empty(), input_preview.is_empty()) {
        (false, false) => format!("{description}\n{input_preview}"),
        (false, true) => description.to_owned(),
        (true, false) => input_preview.to_owned(),
        (true, true) => project.to_owned(),
    };

    Some(OctosApprovalRequest {
        protocol: ApprovalProtocol::AgentChat {
            agent: agent.to_owned(),
            project: project.to_owned(),
            project_room_id: project_room_id.to_owned(),
        },
        request_id: request_id.to_owned(),
        tool_name: tool_name.to_owned(),
        tool_args_digest: input_digest.to_owned(),
        title: tool_name.to_owned(),
        summary,
        risk_level: OctosApprovalRiskLevel::Normal,
        authorized_approvers: Vec::new(),
        expires_at: expires_at.to_string(),
        on_timeout: OctosApprovalTimeoutBehavior::Notify,
    })
}

pub(super) fn agentchat_custom_message_body_from_content(content: &serde_json::Value) -> Option<&str> {
    let msgtype = content.get("msgtype")?.as_str()?;
    let expected_kind = match msgtype {
        AGENTCHAT_APPROVAL_REQUEST_MSGTYPE => "request",
        AGENTCHAT_APPROVAL_STATUS_MSGTYPE => "status",
        AGENTCHAT_APPROVAL_VERDICT_MSGTYPE => "verdict",
        _ => return None,
    };
    let approval = content.get(AGENTCHAT_APPROVAL_EVENT_KEY)?;
    if approval.get("version").and_then(|value| value.as_u64()) != Some(1)
        || approval.get("kind").and_then(|value| value.as_str()) != Some(expected_kind)
    {
        return None;
    }
    content.get("body")?.as_str()
}

pub(super) fn parse_octos_actions_from_content(content: &serde_json::Value) -> Vec<OctosActionButton> {
    let Some(actions) = effective_octos_message_content(content)
        .get("org.octos.actions")
        .and_then(|value| value.as_array())
    else {
        return Vec::new();
    };

    let mut parsed = Vec::new();
    for (index, action) in actions.iter().enumerate() {
        if parsed.len() >= MAX_OCTOS_ACTION_BUTTONS {
            warning!(
                "org.octos.actions: truncated {} extra buttons",
                actions.len().saturating_sub(MAX_OCTOS_ACTION_BUTTONS)
            );
            break;
        }

        let Some(id) = action.get("id").and_then(|value| value.as_str()).map(str::trim) else {
            warning!("org.octos.actions: skipping malformed entry at index {index}");
            continue;
        };
        let Some(label) = action.get("label").and_then(|value| value.as_str()).map(str::trim) else {
            warning!("org.octos.actions: skipping malformed entry at index {index}");
            continue;
        };
        if id.is_empty() || label.is_empty() {
            warning!("org.octos.actions: skipping malformed entry at index {index}");
            continue;
        }

        parsed.push(OctosActionButton {
            id: id.to_owned(),
            label: label.to_owned(),
            style: parse_octos_action_style(action.get("style").and_then(|value| value.as_str())),
        });
    }

    parsed
}

pub(super) fn parse_octos_approval_actions_from_content(content: &serde_json::Value) -> Vec<OctosActionButton> {
    parse_octos_actions_from_content(content)
        .into_iter()
        .filter(|action| matches!(action.id.as_str(), "approve" | "deny"))
        .collect()
}

pub(super) fn parse_octos_action_payload_for_render(
    content: Option<&serde_json::Value>,
    original_content: Option<&serde_json::Value>,
) -> ParsedOctosActionPayload {
    let is_agentchat_request = original_content.is_some_and(|content| {
        content.get("msgtype").and_then(|value| value.as_str()) == Some(AGENTCHAT_APPROVAL_REQUEST_MSGTYPE)
    });
    let has_octos_request = original_content
        .is_some_and(|content| content.get("org.octos.approval_request").is_some());
    let approval_request = if is_agentchat_request {
        original_content.and_then(parse_agentchat_approval_request_from_content)
    } else {
        original_content.and_then(parse_octos_approval_request_from_content)
    };
    let malformed_approval_request = (is_agentchat_request || has_octos_request)
        && approval_request.is_none();

    let actions = if malformed_approval_request {
        Vec::new()
    } else if let Some(approval_request) = approval_request.as_ref() {
        match approval_request.protocol {
            ApprovalProtocol::Octos => original_content
                .map(parse_octos_approval_actions_from_content)
                .unwrap_or_default(),
            ApprovalProtocol::AgentChat { .. } => original_content
                .and_then(|content| content.get(AGENTCHAT_APPROVAL_EVENT_KEY))
                .map(parse_agentchat_approval_actions_from_detail)
                .unwrap_or_default(),
        }
    } else {
        content
            .map(parse_octos_actions_from_content)
            .unwrap_or_default()
    };

    ParsedOctosActionPayload {
        approval_request,
        actions,
        malformed_approval_request,
    }
}

pub(super) fn current_unix_time_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()
        .and_then(|duration| u64::try_from(duration.as_millis()).ok())
        .unwrap_or(u64::MAX)
}

pub(super) fn approval_expiry_millis(protocol: &ApprovalProtocol, expires_at: &str) -> Option<u64> {
    match protocol {
        ApprovalProtocol::AgentChat { .. } => expires_at.parse().ok(),
        ApprovalProtocol::Octos => chrono::DateTime::parse_from_rfc3339(expires_at)
            .ok()
            .and_then(|expires_at| u64::try_from(expires_at.timestamp_millis()).ok()),
    }
}

pub(super) fn approval_request_is_expired(
    approval_request: &OctosApprovalRequest,
    now_millis: u64,
) -> bool {
    approval_expiry_millis(&approval_request.protocol, &approval_request.expires_at)
        .map(|expires_at| now_millis >= expires_at)
        .unwrap_or(true)
}

pub(super) fn compute_action_button_render_state_at(
    actions: &[OctosActionButton],
    approval_request: Option<&OctosApprovalRequest>,
    current_user_id: Option<&UserId>,
    now_millis: u64,
) -> ActionButtonRenderState {
    let approval_card = approval_request
        .and_then(|approval_request| (!actions.is_empty()).then(|| {
            let expired = approval_request_is_expired(approval_request, now_millis);
            ApprovalCardRenderState {
                title: approval_request.title.clone(),
                summary: approval_request.summary.clone(),
                buttons_enabled: !expired && local_user_can_approve(approval_request, current_user_id),
                expired,
            }
        }));
    let visible_slots = actions
        .iter()
        .take(MAX_OCTOS_ACTION_BUTTONS)
        .map(|action| ActionButtonRenderSlot {
            id: action.id.clone(),
            label: action.label.clone(),
            style: action.style,
        })
        .collect::<Vec<_>>();

    let buttons_enabled = approval_card
        .as_ref()
        .map(|approval_card| approval_card.buttons_enabled)
        .unwrap_or(true);
    let show_button_row = !visible_slots.is_empty();

    ActionButtonRenderState {
        show_container: approval_card.is_some() || show_button_row,
        show_button_row,
        approval_card,
        buttons_enabled,
        visible_slots,
    }
}

pub(super) fn compute_action_button_render_state(
    actions: &[OctosActionButton],
    approval_request: Option<&OctosApprovalRequest>,
    current_user_id: Option<&UserId>,
) -> ActionButtonRenderState {
    compute_action_button_render_state_at(
        actions,
        approval_request,
        current_user_id,
        current_unix_time_millis(),
    )
}

pub(super) fn action_button_render_slots_for_display(
    render_state: &ActionButtonRenderState,
    selected_action: Option<&SelectedOctosActionState>,
) -> Vec<ActionButtonRenderSlot> {
    if let Some(selected_action) = selected_action {
        vec![ActionButtonRenderSlot {
            id: selected_action.id.clone(),
            label: format!("✓ {}", selected_action.label),
            style: selected_action.style,
        }]
    } else {
        render_state.visible_slots.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OctosActionResponseRequest {
    pub(super) timeline_kind: TimelineKind,
    pub(super) content: serde_json::Value,
    pub(super) target_user_id: OwnedUserId,
    pub(super) explicit_room: bool,
    pub(super) source_event_id: OwnedEventId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum OctosActionButtonRequest {
    Generic {
        action_id: String,
        label: String,
        style: OctosActionStyle,
    },
    Approval {
        protocol: ApprovalProtocol,
        request_id: String,
        title: String,
        decision: String,
        label: String,
        tool_args_digest: String,
        expires_at: String,
        style: OctosActionStyle,
    },
}

impl OctosActionButtonRequest {
    pub(super) fn action_id(&self) -> &str {
        match self {
            Self::Generic { action_id, .. } => action_id,
            Self::Approval { decision, .. } => decision,
        }
    }

    pub(super) fn label(&self) -> &str {
        match self {
            Self::Generic { label, .. } => label,
            Self::Approval { label, .. } => label,
        }
    }

    pub(super) fn style(&self) -> OctosActionStyle {
        match self {
            Self::Generic { style, .. } | Self::Approval { style, .. } => *style,
        }
    }

    fn expiry_millis(&self) -> Option<u64> {
        match self {
            Self::Generic { .. } => None,
            Self::Approval { protocol, expires_at, .. } => {
                approval_expiry_millis(protocol, expires_at)
            }
        }
    }

    pub(super) fn is_expired(&self, now_millis: u64) -> bool {
        self.expiry_millis()
            .map(|expires_at| now_millis >= expires_at)
            .unwrap_or(matches!(self, Self::Approval { .. }))
    }
}

#[cfg(test)]
pub(super) fn next_approval_expiry_timeout<'a>(
    requests: impl IntoIterator<Item = &'a OctosActionButtonRequest>,
    now_millis: u64,
) -> Option<Duration> {
    earliest_approval_expiry_millis(requests)
        .map(|expires_at| Duration::from_millis(
            expires_at.saturating_sub(now_millis).max(1)
        ))
}

pub(super) fn earliest_approval_expiry_millis<'a>(
    requests: impl IntoIterator<Item = &'a OctosActionButtonRequest>,
) -> Option<u64> {
    requests
        .into_iter()
        .filter_map(OctosActionButtonRequest::expiry_millis)
        .min()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct OctosActionButtonContext {
    pub(super) item_id: usize,
    pub(super) item_widget_uid: WidgetUid,
    pub(super) source_event_id: OwnedEventId,
    pub(super) original_sender: OwnedUserId,
    pub(super) request: OctosActionButtonRequest,
}

pub(super) fn build_octos_approval_response_request(
    timeline_kind: &TimelineKind,
    title: &str,
    request_id: &str,
    decision: &str,
    tool_args_digest: &str,
    source_event_id: &EventId,
    original_sender: &UserId,
) -> OctosActionResponseRequest {
    OctosActionResponseRequest {
        timeline_kind: timeline_kind.clone(),
        content: serde_json::json!({
            "msgtype": "m.text",
            "body": format!("[Approval: {decision}] {title}"),
            "org.octos.approval_response": {
                "request_id": request_id,
                "decision": decision,
                "source_event_id": source_event_id.as_str(),
                "tool_args_digest": tool_args_digest,
            },
            "m.relates_to": {
                "m.in_reply_to": {
                    "event_id": source_event_id.as_str(),
                }
            }
        }),
        target_user_id: original_sender.to_owned(),
        explicit_room: false,
        source_event_id: source_event_id.to_owned(),
    }
}

pub(super) fn build_agentchat_approval_verdict_request(
    timeline_kind: &TimelineKind,
    label: &str,
    request_id: &str,
    action: &str,
    input_digest: &str,
    agent: &str,
    project: &str,
    project_room_id: &str,
    source_event_id: &EventId,
    original_sender: &UserId,
) -> OctosActionResponseRequest {
    OctosActionResponseRequest {
        timeline_kind: timeline_kind.clone(),
        content: serde_json::json!({
            "msgtype": AGENTCHAT_APPROVAL_VERDICT_MSGTYPE,
            "body": label,
            "com.agentchat.approval": {
                "version": 1,
                "kind": "verdict",
                "agent": agent,
                "project": project,
                "project_room_id": project_room_id,
                "request_id": request_id,
                "input_digest": input_digest,
                "action": action,
            },
            "m.relates_to": {
                "m.in_reply_to": {
                    "event_id": source_event_id.as_str(),
                }
            }
        }),
        target_user_id: original_sender.to_owned(),
        explicit_room: false,
        source_event_id: source_event_id.to_owned(),
    }
}

pub(super) fn build_octos_action_response_request(
    timeline_kind: &TimelineKind,
    label: &str,
    action_id: &str,
    source_event_id: &EventId,
    original_sender: &UserId,
) -> OctosActionResponseRequest {
    OctosActionResponseRequest {
        timeline_kind: timeline_kind.clone(),
        content: serde_json::json!({
            "msgtype": "m.text",
            "body": format!("[Action: {label}]"),
            "org.octos.action_response": {
                "action_id": action_id,
                "source_event_id": source_event_id.as_str(),
            },
            "m.relates_to": {
                "m.in_reply_to": {
                    "event_id": source_event_id.as_str(),
                }
            }
        }),
        target_user_id: original_sender.to_owned(),
        explicit_room: false,
        source_event_id: source_event_id.to_owned(),
    }
}

pub(super) fn local_user_can_approve(
    approval_request: &OctosApprovalRequest,
    current_user_id: Option<&UserId>,
) -> bool {
    let Some(current_user_id) = current_user_id else {
        return false;
    };

    match &approval_request.protocol {
        ApprovalProtocol::Octos => approval_request.authorized_approvers
            .iter()
            .any(|approver| approver == current_user_id.as_str()),
        // The dedicated encrypted approval room only contains the owner and
        // managed service accounts. This enables the UI affordance, but is not
        // an authorization decision: agent-chat still validates event.sender.
        ApprovalProtocol::AgentChat { .. } => true,
    }
}

pub(super) fn mark_action_buttons_disabled(
    disabled_source_event_ids: &mut HashSet<OwnedEventId>,
    source_event_id: &OwnedEventId,
) {
    disabled_source_event_ids.insert(source_event_id.clone());
}

pub(super) fn mark_selected_octos_action(
    selected_actions: &mut HashMap<OwnedEventId, SelectedOctosActionState>,
    source_event_id: &OwnedEventId,
    action_id: &str,
    label: &str,
    style: OctosActionStyle,
) {
    selected_actions.insert(source_event_id.clone(), SelectedOctosActionState {
        id: action_id.to_owned(),
        label: label.to_owned(),
        style,
    });
}

pub(super) fn clear_selected_octos_action(
    selected_actions: &mut HashMap<OwnedEventId, SelectedOctosActionState>,
    source_event_id: &EventId,
) {
    selected_actions.remove(source_event_id);
}

pub(super) fn clear_action_buttons_disabled(
    disabled_source_event_ids: &mut HashSet<OwnedEventId>,
    source_event_id: &EventId,
) {
    disabled_source_event_ids.remove(source_event_id);
}

pub(super) fn are_action_buttons_disabled(
    disabled_source_event_ids: &HashSet<OwnedEventId>,
    source_event_id: &EventId,
) -> bool {
    disabled_source_event_ids.contains(source_event_id)
}


pub(super) fn populate_octos_action_buttons(
    cx: &mut Cx,
    app_language: AppLanguage,
    item: &WidgetRef,
    item_id: usize,
    content: Option<&serde_json::Value>,
    original_content: Option<&serde_json::Value>,
    source_event_id: Option<&OwnedEventId>,
    original_sender: &UserId,
    action_button_contexts: &mut HashMap<(OwnedEventId, usize), OctosActionButtonContext>,
    disabled_source_event_ids: &HashSet<OwnedEventId>,
    selected_actions: &HashMap<OwnedEventId, SelectedOctosActionState>,
) {
    let container = item.view(cx, ids!(content.action_buttons));
    if content.is_none() && original_content.is_none() {
        action_button_contexts.retain(|_, context| context.item_id != item_id);
        container.set_visible(cx, false);
        return;
    }
    let Some(source_event_id) = source_event_id else {
        action_button_contexts.retain(|_, context| context.item_id != item_id);
        container.set_visible(cx, false);
        return;
    };

    let parsed_payload = parse_octos_action_payload_for_render(content, original_content);

    if parsed_payload.malformed_approval_request {
        warning!("approval request: skipping malformed structured payload");
    }

    let render_state = compute_action_button_render_state(
        &parsed_payload.actions,
        parsed_payload.approval_request.as_ref(),
        current_user_id().as_deref(),
    );
    let is_disabled = are_action_buttons_disabled(disabled_source_event_ids, source_event_id.as_ref())
        || !render_state.buttons_enabled;
    let selected_action = selected_actions.get(source_event_id);
    let visible_slots = action_button_render_slots_for_display(&render_state, selected_action);
    let is_approval = render_state.approval_card.is_some();

    container.set_visible(cx, render_state.show_container);
    if !render_state.show_container {
        action_button_contexts.retain(|_, context| context.item_id != item_id);
        return;
    }

    let approval_request_view = item.view(cx, ids!(content.action_buttons.approval_request_view));
    let button_row = item.view(cx, ids!(content.action_buttons.action_button_row));
    let approval_button_row = item.view(cx, ids!(content.action_buttons.approval_request_view.approval_action_button_row));
    button_row.set_visible(cx, !is_approval && render_state.show_button_row && !visible_slots.is_empty());
    approval_button_row.set_visible(cx, is_approval && render_state.show_button_row && !visible_slots.is_empty());
    approval_request_view.set_visible(cx, is_approval);
    if let Some(approval_card) = render_state.approval_card.as_ref() {
        item.label(cx, ids!(content.action_buttons.approval_request_view.approval_header.approval_title_label))
            .set_text(cx, &approval_card.title);
        item.label(cx, ids!(content.action_buttons.approval_request_view.approval_summary_label))
            .set_text(cx, &approval_card.summary);
        item.label(cx, ids!(content.action_buttons.approval_request_view.approval_header.pending_badge.pending_label))
            .set_text(cx, tr_key(
                app_language,
                if approval_card.expired {
                    "room_screen.approval.expired"
                } else {
                    "room_screen.approval.pending"
                },
            ));
    }

    // Dynamic action row: ONE Splash per message replaces the former pool of
    // 6 slots x 3 style-variant buttons (doubled by the approval card) that
    // every message carried as permanently-invisible widgets. The row is
    // built inside the Splash isolate from the generated body below; clicks
    // come back over `agent.notify` (handled in `handle_message_actions`),
    // keyed by (source event id, slot index) instead of widget uids.
    action_button_contexts.retain(|_, context| context.item_id != item_id);

    let regular_splash = item.splash(cx, ids!(content.action_buttons.actions_splash));
    let approval_splash = item.splash(
        cx,
        ids!(content.action_buttons.approval_request_view.approval_actions_splash),
    );
    let (active_splash, inactive_splash) = if is_approval {
        (approval_splash, regular_splash)
    } else {
        (regular_splash, approval_splash)
    };
    // A recycled item may still hold the other family's evaluated view;
    // `set_text("")` alone would keep it (empty bodies skip re-eval).
    inactive_splash.set_visible(cx, false);

    if visible_slots.is_empty() {
        active_splash.set_visible(cx, false);
        return;
    }
    active_splash.set_visible(cx, true);
    active_splash.set_text(
        cx,
        &build_octos_actions_splash_body(&visible_slots, !is_disabled, source_event_id.as_str()),
    );

    if !is_disabled {
        for (index, render_slot) in visible_slots.iter().enumerate() {
            let request = if let Some(approval_request) = parsed_payload.approval_request.as_ref() {
                OctosActionButtonRequest::Approval {
                    protocol: approval_request.protocol.clone(),
                    request_id: approval_request.request_id.clone(),
                    title: approval_request.title.clone(),
                    decision: render_slot.id.clone(),
                    label: render_slot.label.clone(),
                    tool_args_digest: approval_request.tool_args_digest.clone(),
                    expires_at: approval_request.expires_at.clone(),
                    style: render_slot.style,
                }
            } else {
                OctosActionButtonRequest::Generic {
                    action_id: render_slot.id.clone(),
                    label: render_slot.label.clone(),
                    style: render_slot.style,
                }
            };

            action_button_contexts.insert(
                (source_event_id.clone(), index),
                OctosActionButtonContext {
                    item_id,
                    item_widget_uid: item.widget_uid(),
                    source_event_id: source_event_id.clone(),
                    original_sender: original_sender.to_owned(),
                    request,
                },
            );
        }
    }
}

/// The `agent.notify` event id splash-built action buttons report through.
pub(super) const OCTOS_SPLASH_NOTIFY_EVENT: &str = "octos_action";

/// Escape a string for embedding inside a double-quoted splash script literal.
/// Labels arrive from the network; without this a crafted label could break
/// out of the string and run arbitrary script in the (sandboxed) isolate.
fn escape_splash_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Build the splash body for a message's action row. Enabled slots become
/// Buttons whose `on_click` reports (source event id, slot index) back to
/// Rust via `agent.notify`; a disabled row (selected or expired) renders as
/// plain labels so no dead controls are shown.
///
/// Styling note: buttons use the isolate's stock light-theme look for now.
/// Matching the RBX_* primary/secondary/danger styling needs a themed widget
/// kit registered into the isolate VM -- tracked as a follow-up.
pub(super) fn build_octos_actions_splash_body(
    slots: &[ActionButtonRenderSlot],
    enabled: bool,
    source_event_id: &str,
) -> String {
    use std::fmt::Write;
    let mut body =
        String::from("width: Fill\nheight: Fit\nflow: Flow.Right{wrap: true}\nspacing: 8\n");
    let source = escape_splash_string(source_event_id);
    for (index, slot) in slots.iter().enumerate() {
        let label = escape_splash_string(&slot.label);
        if enabled {
            let _ = write!(
                body,
                "b{index} := Button {{ text: \"{label}\" on_click: || {{ agent.notify(\"{event}\", {{source: \"{source}\", slot: {index}}}) }} }}\n",
                event = OCTOS_SPLASH_NOTIFY_EVENT,
            );
        } else {
            let _ = write!(body, "s{index} := Label {{ text: \"{label}\" }}\n");
        }
    }
    body
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_menu() {
        let content = serde_json::json!({
            "msgtype": "m.text",
            "body": "hello"
        });

        let message = forwardable_room_message_content_from_json(content).unwrap();

        assert!(matches!(message.msgtype, MessageType::Text(..)));
    }

    #[test]
    fn test_forward_menu_hidden_non_message() {
        let content = serde_json::json!({
            "msgtype": "m.image",
            "body": "photo.jpg",
            "url": "mxc://example.org/media"
        });

        assert!(forwardable_room_message_content_from_json(content).is_none());
    }

    #[test]
    fn test_forward_uses_latest_effective_content() {
        let content = serde_json::json!({
            "msgtype": "m.text",
            "body": "original",
            "m.new_content": {
                "msgtype": "m.text",
                "body": "edited"
            }
        });
        let effective_content = effective_octos_message_content(&content).clone();
        let message = forwardable_room_message_content_from_json(effective_content).unwrap();

        assert!(matches!(
            message.msgtype,
            MessageType::Text(TextMessageEventContent { body, .. }) if body == "edited"
        ));
    }

    #[test]
    fn test_forward_does_not_send_reply_metadata() {
        let content = serde_json::json!({
            "msgtype": "m.text",
            "body": "reply text",
            "m.relates_to": {
                "m.in_reply_to": {
                    "event_id": "$source:example.org"
                }
            }
        });
        let message = forwardable_room_message_content_from_json(content).unwrap();

        assert!(message.relates_to.is_none());
    }

    #[test]
    fn test_parse_octos_actions_skips_malformed_entries() {
        let actions = parse_octos_actions_from_content(&serde_json::json!({
            "org.octos.actions": [
                { "id": "retry_pptx", "label": "Regenerate PPT", "style": "primary" },
                { "label": "Missing id" },
                { "id": "cancel", "label": "Cancel", "style": "secondary" }
            ]
        }));

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].id, "retry_pptx");
        assert_eq!(actions[1].id, "cancel");
    }

    #[test]
    fn test_parse_octos_actions_truncates_after_six() {
        let actions = parse_octos_actions_from_content(&serde_json::json!({
            "org.octos.actions": [
                { "id": "a1", "label": "A1" },
                { "id": "a2", "label": "A2" },
                { "id": "a3", "label": "A3" },
                { "id": "a4", "label": "A4" },
                { "id": "a5", "label": "A5" },
                { "id": "a6", "label": "A6" },
                { "id": "a7", "label": "A7" }
            ]
        }));

        assert_eq!(actions.len(), 6);
        assert_eq!(actions.last().map(|action| action.id.as_str()), Some("a6"));
    }

    #[test]
    fn test_parse_octos_actions_reads_m_new_content_wrapper() {
        let actions = parse_octos_actions_from_content(&serde_json::json!({
            "m.new_content": {
                "org.octos.actions": [
                    { "id": "confirm", "label": "确认", "style": "primary" },
                    { "id": "cancel", "label": "取消", "style": "secondary" }
                ]
            },
            "org.octos.actions": [
                { "id": "stale", "label": "旧按钮" }
            ]
        }));

        assert_eq!(actions.len(), 2);
        assert_eq!(actions[0].id, "confirm");
        assert_eq!(actions[1].id, "cancel");
    }

    const TEST_AGENTCHAT_AGENT: &str = "test_agent";
    const TEST_AGENTCHAT_PROJECT: &str = "test_project";
    const TEST_AGENTCHAT_PROJECT_ROOM_ID: &str = "!project:example.test";

    fn valid_agentchat_approval_content() -> serde_json::Value {
        serde_json::json!({
            "msgtype": "com.agentchat.approval.request.v1",
            "body": format!("Approval required for {TEST_AGENTCHAT_AGENT}"),
            "com.agentchat.approval": {
                "version": 1,
                "kind": "request",
                "agent": TEST_AGENTCHAT_AGENT,
                "project": TEST_AGENTCHAT_PROJECT,
                "project_room_id": TEST_AGENTCHAT_PROJECT_ROOM_ID,
                "request_id": "approval_0123456789abcdef0123456789abcdef",
                "upstream_request_id": "turn-1:Bash",
                "input_digest": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                "runtime": "codex",
                "tool_name": "Bash",
                "description": "Create a GitHub issue",
                "input_preview": "gh issue create --title test",
                "expires_at": 1784745600000u64,
                "actions": [
                    { "id": "approve_once", "label": "Approve once", "style": "primary" },
                    { "id": "deny", "label": "Deny", "style": "danger" }
                ]
            }
        })
    }

    #[test]
    fn test_parse_agentchat_owner_approval_request() {
        let content = valid_agentchat_approval_content();
        let payload = parse_octos_action_payload_for_render(Some(&content), Some(&content));
        let approval = payload.approval_request.expect("agent-chat approval should parse");

        assert_eq!(approval.request_id, "approval_0123456789abcdef0123456789abcdef");
        assert_eq!(approval.tool_args_digest, "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        assert_eq!(payload.actions.iter().map(|action| action.id.as_str()).collect::<Vec<_>>(), vec!["approve_once", "deny"]);
        assert!(!payload.malformed_approval_request);
        assert!(matches!(
            approval.protocol,
            ApprovalProtocol::AgentChat { ref agent, ref project, ref project_room_id }
                if agent == TEST_AGENTCHAT_AGENT
                    && project == TEST_AGENTCHAT_PROJECT
                    && project_room_id == TEST_AGENTCHAT_PROJECT_ROOM_ID
        ));
    }

    #[test]
    fn test_agentchat_approval_buttons_expire_at_deadline() {
        let content = valid_agentchat_approval_content();
        let payload = parse_octos_action_payload_for_render(Some(&content), Some(&content));
        let approval = payload.approval_request.as_ref().expect("agent-chat approval should parse");
        let expires_at = approval.expires_at.parse::<u64>().unwrap();
        let current_user_id = UserId::parse("@owner:example.test").unwrap();

        let live = compute_action_button_render_state_at(
            &payload.actions,
            Some(approval),
            Some(current_user_id.as_ref()),
            expires_at - 1,
        );
        assert!(live.buttons_enabled);
        assert_eq!(live.approval_card.as_ref().map(|card| card.expired), Some(false));

        let expired = compute_action_button_render_state_at(
            &payload.actions,
            Some(approval),
            Some(current_user_id.as_ref()),
            expires_at,
        );
        assert!(!expired.buttons_enabled);
        assert_eq!(expired.approval_card.as_ref().map(|card| card.expired), Some(true));
    }

    fn agentchat_approval_button_request(expires_at: &str) -> OctosActionButtonRequest {
        OctosActionButtonRequest::Approval {
            protocol: ApprovalProtocol::AgentChat {
                agent: TEST_AGENTCHAT_AGENT.into(),
                project: TEST_AGENTCHAT_PROJECT.into(),
                project_room_id: TEST_AGENTCHAT_PROJECT_ROOM_ID.into(),
            },
            request_id: "approval_0123456789abcdef0123456789abcdef".into(),
            title: "Run command".into(),
            decision: "approve_once".into(),
            label: "Approve once".into(),
            tool_args_digest: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".into(),
            expires_at: expires_at.into(),
            style: OctosActionStyle::Primary,
        }
    }

    #[test]
    fn test_approval_expiry_timer_uses_earliest_visible_deadline() {
        let generic = OctosActionButtonRequest::Generic {
            action_id: "retry".into(),
            label: "Retry".into(),
            style: OctosActionStyle::Secondary,
        };
        let later = agentchat_approval_button_request("1500");
        let sooner = agentchat_approval_button_request("1250");

        assert_eq!(
            next_approval_expiry_timeout([&generic, &later, &sooner], 1000),
            Some(Duration::from_millis(250)),
        );
        assert_eq!(
            next_approval_expiry_timeout([&sooner], 1250),
            Some(Duration::from_millis(1)),
        );
        assert_eq!(
            next_approval_expiry_timeout([&generic], 1000),
            None,
        );
    }

    #[test]
    fn test_agentchat_public_status_has_no_actions() {
        let body = format!("Agent {TEST_AGENTCHAT_AGENT} is waiting for approval from its owner.");
        let content = serde_json::json!({
            "msgtype": "com.agentchat.approval.status.v1",
            "body": body.clone(),
            "com.agentchat.approval": {
                "version": 1,
                "kind": "status",
                "agent": TEST_AGENTCHAT_AGENT,
                "project": TEST_AGENTCHAT_PROJECT,
                "state": "waiting_for_owner"
            }
        });
        let payload = parse_octos_action_payload_for_render(Some(&content), Some(&content));

        assert_eq!(
            agentchat_custom_message_body_from_content(&content),
            Some(body.as_str()),
        );
        assert!(payload.approval_request.is_none());
        assert!(payload.actions.is_empty());
        assert!(!payload.malformed_approval_request);
    }

    #[test]
    fn test_malformed_agentchat_owner_approval_request_hides_buttons() {
        let mut content = valid_agentchat_approval_content();
        content["com.agentchat.approval"]["input_digest"] = serde_json::json!("not-a-digest");
        let payload = parse_octos_action_payload_for_render(Some(&content), Some(&content));
        let state = compute_action_button_render_state(&payload.actions, payload.approval_request.as_ref(), None);

        assert!(payload.malformed_approval_request);
        assert!(payload.actions.is_empty());
        assert!(!state.show_container);
    }

    #[test]
    fn test_build_agentchat_approval_verdict() {
        let timeline_kind = TimelineKind::MainRoom {
            room_id: "!approval:example.test".try_into().unwrap(),
        };
        let source_event_id: OwnedEventId = "$approval-request".try_into().unwrap();
        let original_sender: OwnedUserId = "@agent-bridge:example.test".try_into().unwrap();
        let request = build_agentchat_approval_verdict_request(
            &timeline_kind,
            "Approve once",
            "approval_0123456789abcdef0123456789abcdef",
            "approve_once",
            "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            TEST_AGENTCHAT_AGENT,
            TEST_AGENTCHAT_PROJECT,
            TEST_AGENTCHAT_PROJECT_ROOM_ID,
            source_event_id.as_ref(),
            original_sender.as_ref(),
        );

        let verdict = &request.content["com.agentchat.approval"];
        assert_eq!(request.content["msgtype"], AGENTCHAT_APPROVAL_VERDICT_MSGTYPE);
        assert_eq!(verdict["kind"], "verdict");
        assert_eq!(verdict["action"], "approve_once");
        assert_eq!(verdict["agent"], TEST_AGENTCHAT_AGENT);
        assert_eq!(verdict["project"], TEST_AGENTCHAT_PROJECT);
        assert_eq!(verdict["project_room_id"], TEST_AGENTCHAT_PROJECT_ROOM_ID);
        assert_eq!(verdict["request_id"], "approval_0123456789abcdef0123456789abcdef");
        assert_eq!(verdict["input_digest"], "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
        assert_eq!(request.target_user_id, original_sender);
    }

    #[test]
    fn test_agentchat_approval_uses_original_content() {
        let original = valid_agentchat_approval_content();
        let mut edited = original.clone();
        edited["com.agentchat.approval"]["request_id"] = serde_json::json!("approval_ffffffffffffffffffffffffffffffff");
        edited["com.agentchat.approval"]["input_digest"] = serde_json::json!("ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff");

        let payload = parse_octos_action_payload_for_render(Some(&edited), Some(&original));
        let approval = payload.approval_request.expect("original approval should parse");
        assert_eq!(approval.request_id, "approval_0123456789abcdef0123456789abcdef");
        assert_eq!(approval.tool_args_digest, "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef");
    }

    #[test]
    fn test_parse_octos_approval_request_from_content() {
        let approval = parse_octos_approval_request_from_content(&serde_json::json!({
            "org.octos.approval_request": {
                "request_id": "req_abc123",
                "tool_name": "shell",
                "tool_args_digest": "sha256:4bf5",
                "title": "Execute shell command",
                "summary": "rm -rf ~/tmp/cache",
                "risk_level": "critical",
                "authorized_approvers": ["@alice:example.org"],
                "expires_at": "2026-04-14T14:30:00Z",
                "on_timeout": "notify"
            }
        })).expect("approval request should parse");

        assert_eq!(approval.request_id, "req_abc123");
        assert_eq!(approval.tool_name, "shell");
        assert_eq!(approval.tool_args_digest, "sha256:4bf5");
        assert_eq!(approval.title, "Execute shell command");
        assert_eq!(approval.summary, "rm -rf ~/tmp/cache");
        assert_eq!(approval.risk_level, OctosApprovalRiskLevel::Critical);
        assert_eq!(approval.authorized_approvers, vec!["@alice:example.org"]);
        assert_eq!(approval.on_timeout, OctosApprovalTimeoutBehavior::Notify);
    }

    #[test]
    fn test_parse_octos_approval_request_ignores_m_new_content_wrapper() {
        let approval = parse_octos_approval_request_from_content(&serde_json::json!({
            "org.octos.approval_request": {
                "request_id": "req_original",
                "tool_name": "shell",
                "tool_args_digest": "sha256:4bf5",
                "title": "Original request",
                "summary": "rm -rf ~/tmp/cache",
                "risk_level": "critical",
                "authorized_approvers": ["@alice:example.org"],
                "expires_at": "2026-04-14T14:30:00Z",
                "on_timeout": "notify"
            },
            "m.new_content": {
                "org.octos.approval_request": {
                    "request_id": "req_edited",
                    "tool_name": "shell",
                    "tool_args_digest": "sha256:mallory",
                    "title": "Edited request",
                    "summary": "whoami",
                    "risk_level": "normal",
                    "authorized_approvers": ["@mallory:example.org"],
                    "expires_at": "2026-04-14T14:30:00Z",
                    "on_timeout": "notify"
                }
            }
        })).expect("approval request should parse from original content");

        assert_eq!(approval.request_id, "req_original");
        assert_eq!(approval.authorized_approvers, vec!["@alice:example.org"]);
        assert_eq!(approval.risk_level, OctosApprovalRiskLevel::Critical);
    }

    #[test]
    fn test_parse_octos_approval_request_rejects_empty_authorized_approvers() {
        assert!(parse_octos_approval_request_from_content(&serde_json::json!({
            "org.octos.approval_request": {
                "request_id": "req_abc123",
                "tool_name": "shell",
                "tool_args_digest": "sha256:4bf5",
                "title": "Execute shell command",
                "summary": "rm -rf ~/tmp/cache",
                "risk_level": "critical",
                "authorized_approvers": [],
                "expires_at": "2026-04-14T14:30:00Z",
                "on_timeout": "notify"
            }
        })).is_none());
    }

    #[test]
    fn test_build_approval_response_request_targets_original_sender() {
        let timeline_kind = TimelineKind::MainRoom {
            room_id: "!room:127.0.0.1:8128".try_into().unwrap(),
        };
        let source_event_id: OwnedEventId = "$orig123".try_into().unwrap();
        let original_sender: OwnedUserId = "@octosbot:127.0.0.1:8128".try_into().unwrap();

        let request = build_octos_approval_response_request(
            &timeline_kind,
            "Execute shell command",
            "req_abc123",
            "approve",
            "sha256:4bf5",
            source_event_id.as_ref(),
            original_sender.as_ref(),
        );

        assert_eq!(request.timeline_kind, timeline_kind);
        assert_eq!(request.target_user_id, original_sender);
        assert!(!request.explicit_room);
        assert_eq!(request.content["org.octos.approval_response"]["request_id"], "req_abc123");
        assert_eq!(request.content["org.octos.approval_response"]["decision"], "approve");
        assert_eq!(request.content["org.octos.approval_response"]["tool_args_digest"], "sha256:4bf5");
    }

    #[test]
    fn test_action_buttons_render_state_hidden_without_actions() {
        let state = compute_action_button_render_state(&[], None, None);

        assert!(!state.show_container);
        assert!(state.visible_slots.is_empty());
    }

    #[test]
    fn test_action_buttons_render_state_with_primary_secondary_danger() {
        let state = compute_action_button_render_state(&[
            OctosActionButton {
                id: "retry".into(),
                label: "Regenerate PPT".into(),
                style: OctosActionStyle::Primary,
            },
            OctosActionButton {
                id: "cancel".into(),
                label: "Cancel".into(),
                style: OctosActionStyle::Secondary,
            },
            OctosActionButton {
                id: "delete".into(),
                label: "Delete".into(),
                style: OctosActionStyle::Danger,
            },
        ], None, None);

        assert!(state.show_container);
        assert!(state.show_button_row);
        assert!(state.buttons_enabled);
        assert!(state.approval_card.is_none());
        assert_eq!(state.visible_slots.len(), 3);
        assert_eq!(state.visible_slots[0].style, OctosActionStyle::Primary);
        assert_eq!(state.visible_slots[1].style, OctosActionStyle::Secondary);
        assert_eq!(state.visible_slots[2].style, OctosActionStyle::Danger);
    }

    #[test]
    fn test_approval_buttons_disabled_for_unauthorized_user() {
        let approval_request = OctosApprovalRequest {
            protocol: ApprovalProtocol::Octos,
            request_id: "req_abc123".into(),
            tool_name: "shell".into(),
            tool_args_digest: "sha256:4bf5".into(),
            title: "Execute shell command".into(),
            summary: "rm -rf ~/tmp/cache".into(),
            risk_level: OctosApprovalRiskLevel::Critical,
            authorized_approvers: vec!["@alice:example.org".into()],
            expires_at: "2026-04-14T14:30:00Z".into(),
            on_timeout: OctosApprovalTimeoutBehavior::Notify,
        };
        let current_user_id = UserId::parse("@mallory:example.org").unwrap();
        let state = compute_action_button_render_state(&[
            OctosActionButton {
                id: "approve".into(),
                label: "Approve".into(),
                style: OctosActionStyle::Primary,
            },
            OctosActionButton {
                id: "deny".into(),
                label: "Deny".into(),
                style: OctosActionStyle::Danger,
            },
        ], Some(&approval_request), Some(current_user_id.as_ref()));

        assert!(state.show_container);
        assert!(state.show_button_row);
        assert!(!state.buttons_enabled);
        assert_eq!(
            state.approval_card.as_ref().map(|card| card.title.as_str()),
            Some("Execute shell command"),
        );
        assert_eq!(
            state.approval_card.as_ref().map(|card| card.summary.as_str()),
            Some("rm -rf ~/tmp/cache"),
        );
    }

    #[test]
    fn test_selected_action_reduces_visible_slots_to_clicked_button() {
        let render_state = compute_action_button_render_state(&[
            OctosActionButton {
                id: "approve".into(),
                label: "Approve".into(),
                style: OctosActionStyle::Primary,
            },
            OctosActionButton {
                id: "deny".into(),
                label: "Deny".into(),
                style: OctosActionStyle::Danger,
            },
        ], None, None);

        let visible_slots = action_button_render_slots_for_display(&render_state, Some(&SelectedOctosActionState {
            id: "deny".into(),
            label: "Deny".into(),
            style: OctosActionStyle::Danger,
        }));

        assert_eq!(visible_slots.len(), 1);
        assert_eq!(visible_slots[0].id, "deny");
        assert_eq!(visible_slots[0].label, "✓ Deny");
        assert_eq!(visible_slots[0].style, OctosActionStyle::Danger);
    }

    #[test]
    fn test_generic_actions_without_approval_request_remain_supported() {
        let payload = parse_octos_action_payload_for_render(
            Some(&serde_json::json!({
                "org.octos.actions": [
                    { "id": "retry_pptx", "label": "Regenerate PPT", "style": "primary" }
                ]
            })),
            None,
        );

        assert!(payload.approval_request.is_none());
        assert!(!payload.malformed_approval_request);
        assert_eq!(payload.actions.len(), 1);
        assert_eq!(payload.actions[0].id, "retry_pptx");
    }

    #[test]
    fn test_malformed_approval_request_hides_buttons() {
        let payload = parse_octos_action_payload_for_render(
            Some(&serde_json::json!({
                "org.octos.actions": [
                    { "id": "approve", "label": "Approve", "style": "primary" },
                    { "id": "deny", "label": "Deny", "style": "danger" }
                ]
            })),
            Some(&serde_json::json!({
                "org.octos.approval_request": {
                    "request_id": "req_abc123"
                },
                "org.octos.actions": [
                    { "id": "approve", "label": "Approve", "style": "primary" },
                    { "id": "deny", "label": "Deny", "style": "danger" }
                ]
            })),
        );
        let state = compute_action_button_render_state(
            &payload.actions,
            payload.approval_request.as_ref(),
            None,
        );

        assert!(payload.malformed_approval_request);
        assert!(!state.show_container);
        assert!(state.visible_slots.is_empty());
    }

    #[test]
    fn test_approval_request_ignores_m_replace_edits() {
        let payload = parse_octos_action_payload_for_render(
            Some(&serde_json::json!({
                "m.new_content": {
                    "org.octos.approval_request": {
                        "request_id": "req_replaced",
                        "tool_name": "shell",
                        "tool_args_digest": "sha256:replaced",
                        "title": "Replaced request",
                        "summary": "echo hacked",
                        "risk_level": "normal",
                        "authorized_approvers": ["@mallory:example.org"],
                        "expires_at": "2026-04-14T14:35:00Z",
                        "on_timeout": "notify"
                    },
                    "org.octos.actions": [
                        { "id": "approve", "label": "Approve", "style": "primary" },
                        { "id": "deny", "label": "Deny", "style": "danger" }
                    ]
                }
            })),
            Some(&serde_json::json!({
                "org.octos.approval_request": {
                    "request_id": "req_original",
                    "tool_name": "shell",
                    "tool_args_digest": "sha256:original",
                    "title": "Original request",
                    "summary": "rm -rf ~/tmp/cache",
                    "risk_level": "critical",
                    "authorized_approvers": ["@alice:example.org"],
                    "expires_at": "2026-04-14T14:30:00Z",
                    "on_timeout": "notify"
                },
                "org.octos.actions": [
                    { "id": "approve", "label": "Approve", "style": "primary" },
                    { "id": "deny", "label": "Deny", "style": "danger" }
                ]
            })),
        );
        let current_user_id = UserId::parse("@alice:example.org").unwrap();
        let state = compute_action_button_render_state_at(
            &payload.actions,
            payload.approval_request.as_ref(),
            Some(current_user_id.as_ref()),
            0,
        );

        assert_eq!(
            payload.approval_request.as_ref().map(|approval| approval.request_id.as_str()),
            Some("req_original"),
        );
        assert!(state.buttons_enabled);
        assert_eq!(
            state.approval_card.as_ref().map(|card| card.title.as_str()),
            Some("Original request"),
        );
    }

    #[test]
    fn test_build_action_response_request_targets_original_sender() {
        let timeline_kind = TimelineKind::MainRoom {
            room_id: "!room:127.0.0.1:8128".try_into().unwrap(),
        };
        let source_event_id: OwnedEventId = "$orig123".try_into().unwrap();
        let original_sender: OwnedUserId = "@octosbot_weather:127.0.0.1:8128".try_into().unwrap();

        let request = build_octos_action_response_request(
            &timeline_kind,
            "Regenerate PPT",
            "retry_pptx",
            source_event_id.as_ref(),
            original_sender.as_ref(),
        );

        assert_eq!(request.timeline_kind, timeline_kind);
        assert_eq!(request.target_user_id, original_sender);
        assert!(!request.explicit_room);
    }

    #[test]
    fn test_build_action_response_request_preserves_reply_relation() {
        let timeline_kind = TimelineKind::MainRoom {
            room_id: "!room:127.0.0.1:8128".try_into().unwrap(),
        };
        let source_event_id: OwnedEventId = "$orig123".try_into().unwrap();
        let original_sender: OwnedUserId = "@octosbot_weather:127.0.0.1:8128".try_into().unwrap();

        let request = build_octos_action_response_request(
            &timeline_kind,
            "Regenerate PPT",
            "retry_pptx",
            source_event_id.as_ref(),
            original_sender.as_ref(),
        );

        let action_response = &request.content["org.octos.action_response"];
        assert_eq!(request.content["body"], "[Action: Regenerate PPT]");
        assert_eq!(action_response["action_id"], "retry_pptx");
        assert_eq!(action_response["source_event_id"], "$orig123");
        assert_eq!(request.content["m.relates_to"]["m.in_reply_to"]["event_id"], "$orig123");
    }

    #[test]
    fn test_disable_action_buttons_marks_source_event_disabled() {
        let source_event_id: OwnedEventId = "$orig123".try_into().unwrap();
        let mut disabled = HashSet::new();

        mark_action_buttons_disabled(&mut disabled, &source_event_id);

        assert!(are_action_buttons_disabled(&disabled, source_event_id.as_ref()));
    }

    #[test]
    fn test_reenable_action_buttons_clears_disabled_state() {
        let source_event_id: OwnedEventId = "$orig123".try_into().unwrap();
        let mut disabled = HashSet::new();
        mark_action_buttons_disabled(&mut disabled, &source_event_id);

        clear_action_buttons_disabled(&mut disabled, source_event_id.as_ref());

        assert!(!are_action_buttons_disabled(&disabled, source_event_id.as_ref()));
    }

    #[test]
    fn test_selected_action_state_marks_and_clears_by_source_event_id() {
        let source_event_id: OwnedEventId = "$orig123".try_into().unwrap();
        let mut selected_actions = HashMap::new();

        mark_selected_octos_action(
            &mut selected_actions,
            &source_event_id,
            "approve",
            "Approve",
            OctosActionStyle::Primary,
        );
        assert_eq!(
            selected_actions.get(&source_event_id).map(|state| state.label.as_str()),
            Some("Approve"),
        );

        clear_selected_octos_action(&mut selected_actions, source_event_id.as_ref());
        assert!(!selected_actions.contains_key(&source_event_id));
    }
}
