//! Test-only R3 presentation experiment for the Agent Operations Panel.
//!
//! These types do **not** mirror agent-chat's current Dashboard response. They
//! are non-canonical review material shared with
//! `docs/robrix-with-agentchat/agent-ops-client-contract-v1-proposal.md` and are
//! compiled only by tests. They deliberately use a Robrix-owned experimental
//! schema so they cannot be mistaken for agent-chat's canonical V1 wire model.

use std::fmt;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

const PROPOSAL_SCHEMA: &str = "io.robrix.agent_ops.proposal.r3";
const MAX_JSON_SAFE_INTEGER: u64 = 9_007_199_254_740_991;
const MAX_CAPABILITY_BYTES: usize = 8 * 1024;

/// Text that is safe to present in Robrix.
///
/// The producer remains responsible for redaction. This consumer-side check is
/// only a path heuristic; it is not a general secret detector.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct PrivacyFilteredText(String);

impl PrivacyFilteredText {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for PrivacyFilteredText {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// A server-issued capability that must never be rendered or logged.
#[derive(Clone, PartialEq, Eq)]
pub struct OpaqueCapability(String);

impl fmt::Debug for OpaqueCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OpaqueCapability(<redacted>)")
    }
}

impl OpaqueCapability {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl Serialize for OpaqueCapability {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for OpaqueCapability {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.trim().is_empty() {
            return Err(D::Error::custom("capability must not be empty"));
        }
        if value.len() > MAX_CAPABILITY_BYTES {
            return Err(D::Error::custom("capability exceeds the proposed size limit"));
        }
        Ok(Self(value))
    }
}

impl<'de> Deserialize<'de> for PrivacyFilteredText {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        if value.trim().is_empty() {
            return Err(D::Error::custom("presentation text must not be empty"));
        }
        if contains_absolute_path(&value) {
            return Err(D::Error::custom(
                "filesystem paths are not allowed in the Agent Operations projection",
            ));
        }
        Ok(Self(value))
    }
}

fn contains_absolute_path(value: &str) -> bool {
    if value
        .split(|character: char| {
            character.is_whitespace()
                || matches!(character, '=' | ':' | '(' | ')' | '[' | ']' | '{' | '}' | '<' | '>' | ',' | ';' | '"' | '\'')
        })
        .filter(|token| !token.is_empty())
        .any(looks_like_filesystem_path)
    {
        return true;
    }

    let bytes = value.as_bytes();
    bytes.windows(3).enumerate().any(|(index, window)| {
        let boundary = index == 0
            || bytes[index - 1].is_ascii_whitespace()
            || matches!(bytes[index - 1], b'=' | b':' | b'(' | b'[' | b'{' | b'"' | b'\'');
        boundary
            && window[0].is_ascii_alphabetic()
            && window[1] == b':'
            && matches!(window[2], b'/' | b'\\')
    })
}

fn looks_like_filesystem_path(value: &str) -> bool {
    let value = value.trim();
    std::path::Path::new(value).is_absolute()
        || (value.starts_with('~') && value[1..].contains(['/', '\\']))
        || value.starts_with("\\\\")
}

fn require_id(value: &str, field: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("{field} must not be empty"))
    } else {
        Ok(())
    }
}

fn require_safe_integer(value: u64, field: &str) -> Result<(), String> {
    if value > MAX_JSON_SAFE_INTEGER {
        Err(format!("{field} exceeds the JSON safe-integer range"))
    } else {
        Ok(())
    }
}

fn require_positive_version(value: u64, field: &str) -> Result<(), String> {
    require_safe_integer(value, field)?;
    if value == 0 {
        Err(format!("{field} must be greater than zero"))
    } else {
        Ok(())
    }
}

/// Lifecycle of one dispatch, as reported by the backend projection.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DispatchState {
    Queued,
    Leased,
    Started,
    Parked,
    Completed,
    CancelledBeforeStart,
    OutcomeUnknown,
    #[serde(other)]
    Other,
}

/// Why the projection says a row needs a human.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AttentionKind {
    OutcomeUnknown,
    ParkedApproval,
    ThreadDeliveryFailed,
    WorkspaceDirty,
    #[serde(other)]
    Other,
}

/// Entity kind bound into a short-lived action capability.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ActionEntityKind {
    Dispatch,
    Resource,
    #[serde(other)]
    Other,
}

/// Action kinds the proposed V1 client understands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AvailableActionKind {
    CancelDispatch,
    BeginOutcomeInspection,
    ResolveOutcome,
    MarkResourceInspected,
    #[serde(other)]
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeResolutionKind {
    Continue,
    AcceptCompleted,
    KeepBlocked,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionTarget {
    pub entity_kind: ActionEntityKind,
    pub entity_id: String,
    pub entity_version: u64,
}

impl ActionTarget {
    fn validate(&self) -> Result<(), String> {
        require_id(&self.entity_id, "target.entity_id")?;
        require_positive_version(self.entity_version, "target.entity_version")?;
        if self.entity_kind == ActionEntityKind::Other {
            return Err("unknown target entity kind is not actionable".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourcePrecondition {
    pub resource_id: String,
    pub dirty_generation: u64,
}

impl ResourcePrecondition {
    fn validate(&self) -> Result<(), String> {
        require_id(&self.resource_id, "resource_precondition.resource_id")?;
        require_safe_integer(
            self.dirty_generation,
            "resource_precondition.dirty_generation",
        )
    }
}

/// Backend-issued, entity-scoped permission to request one operation.
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AvailableAction {
    pub action_id: String,
    pub kind: AvailableActionKind,
    pub target: ActionTarget,
    #[serde(default)]
    pub resource_precondition: Option<ResourcePrecondition>,
    #[serde(default)]
    pub allowed_resolutions: Vec<OutcomeResolutionKind>,
    pub expires_at_unix_ms: u64,
    pub capability: OpaqueCapability,
}

impl AvailableAction {
    fn validate(&self) -> Result<(), String> {
        require_id(&self.action_id, "action_id")?;
        self.target.validate()?;
        require_safe_integer(self.expires_at_unix_ms, "expires_at_unix_ms")?;
        if self.expires_at_unix_ms == 0 {
            return Err("expires_at_unix_ms must be greater than zero".into());
        }
        if let Some(resource) = &self.resource_precondition {
            resource.validate()?;
        }

        match self.kind {
            AvailableActionKind::CancelDispatch => {
                if self.target.entity_kind != ActionEntityKind::Dispatch {
                    return Err("cancel_dispatch must target a dispatch".into());
                }
            }
            AvailableActionKind::BeginOutcomeInspection => {
                if self.target.entity_kind != ActionEntityKind::Dispatch
                    || self.resource_precondition.is_none()
                {
                    return Err(
                        "begin_outcome_inspection must bind a dispatch and resource generation"
                            .into(),
                    );
                }
            }
            AvailableActionKind::ResolveOutcome => {
                if self.target.entity_kind != ActionEntityKind::Dispatch
                    || self.resource_precondition.is_none()
                    || self.allowed_resolutions.is_empty()
                {
                    return Err(
                        "resolve_outcome must bind a dispatch, resource, and allowed resolutions"
                            .into(),
                    );
                }
            }
            AvailableActionKind::MarkResourceInspected => {
                let resource = self.resource_precondition.as_ref().ok_or_else(|| {
                    "mark_resource_inspected requires a resource precondition".to_string()
                })?;
                if self.target.entity_kind != ActionEntityKind::Resource
                    || self.target.entity_id != resource.resource_id
                {
                    return Err(
                        "mark_resource_inspected target must match its resource precondition"
                            .into(),
                    );
                }
            }
            AvailableActionKind::Other => {
                return Err("unknown actions are not part of the trusted projection".into());
            }
        }

        if self.kind != AvailableActionKind::ResolveOutcome
            && !self.allowed_resolutions.is_empty()
        {
            return Err("only resolve_outcome may carry allowed_resolutions".into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AgentOpsScope {
    pub scope_id: String,
    pub project_room_id: String,
    pub agent_id: String,
    pub owner_mxid: String,
    pub owner_dm_room_id: String,
}

impl AgentOpsScope {
    fn validate(&self) -> Result<(), String> {
        require_id(&self.scope_id, "scope.scope_id")?;
        require_id(&self.project_room_id, "scope.project_room_id")?;
        require_id(&self.agent_id, "scope.agent_id")?;
        require_id(&self.owner_mxid, "scope.owner_mxid")?;
        require_id(&self.owner_dm_room_id, "scope.owner_dm_room_id")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThreadRef {
    pub room_id: String,
    pub thread_root_event_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AttentionItem {
    pub id: String,
    pub kind: AttentionKind,
    pub summary: PrivacyFilteredText,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub dispatch_id: Option<String>,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default)]
    pub agent: Option<PrivacyFilteredText>,
    #[serde(default)]
    pub waiting_ms: Option<u64>,
    #[serde(default)]
    pub thread: Option<ThreadRef>,
    #[serde(default)]
    pub available_actions: Vec<AvailableAction>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct TaskRow {
    pub task_id: String,
    pub entity_version: u64,
    pub title: PrivacyFilteredText,
    pub agent: PrivacyFilteredText,
    pub state: PrivacyFilteredText,
    #[serde(default)]
    pub dispatch_state: Option<DispatchState>,
    #[serde(default)]
    pub elapsed_ms: Option<u64>,
    #[serde(default)]
    pub thread: Option<ThreadRef>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueueRow {
    pub dispatch_id: String,
    pub entity_version: u64,
    #[serde(default)]
    pub task_id: Option<String>,
    pub agent: PrivacyFilteredText,
    pub state: DispatchState,
    #[serde(default)]
    pub waiting_on: Option<PrivacyFilteredText>,
    #[serde(default)]
    pub held_by_dispatch_id: Option<String>,
    #[serde(default)]
    pub attention_kind: Option<AttentionKind>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorktreeRow {
    pub resource_id: String,
    pub entity_version: u64,
    pub label: PrivacyFilteredText,
    pub branch: PrivacyFilteredText,
    #[serde(default)]
    pub dirty: bool,
    pub dirty_generation: u64,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub available_actions: Vec<AvailableAction>,
}

/// One authoritative, privacy-filtered presentation snapshot for exactly one
/// owner/project/agent scope. A future UI may compose several such snapshots.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Snapshot {
    pub schema: String,
    pub scope: AgentOpsScope,
    pub projection_id: String,
    pub stream_epoch: String,
    pub auth_fence_generation: u64,
    pub seq: u64,
    pub attention: Vec<AttentionItem>,
    pub tasks: Vec<TaskRow>,
    pub queue: Vec<QueueRow>,
    pub worktrees: Vec<WorktreeRow>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SnapshotWire {
    schema: String,
    scope: AgentOpsScope,
    projection_id: String,
    stream_epoch: String,
    auth_fence_generation: u64,
    seq: u64,
    #[serde(default)]
    attention: Vec<AttentionItem>,
    #[serde(default)]
    tasks: Vec<TaskRow>,
    #[serde(default)]
    queue: Vec<QueueRow>,
    #[serde(default)]
    worktrees: Vec<WorktreeRow>,
}

impl TryFrom<SnapshotWire> for Snapshot {
    type Error = String;

    fn try_from(wire: SnapshotWire) -> Result<Self, Self::Error> {
        if wire.schema != PROPOSAL_SCHEMA {
            return Err("snapshot schema is not the proposed contract schema".into());
        }
        wire.scope.validate()?;
        require_id(&wire.projection_id, "projection_id")?;
        require_id(&wire.stream_epoch, "stream_epoch")?;
        require_safe_integer(wire.auth_fence_generation, "auth_fence_generation")?;
        require_safe_integer(wire.seq, "seq")?;

        for item in &wire.attention {
            require_id(&item.id, "attention.id")?;
            if let Some(waiting_ms) = item.waiting_ms {
                require_safe_integer(waiting_ms, "attention.waiting_ms")?;
            }
            for action in &item.available_actions {
                action.validate()?;
                if action.target.entity_kind == ActionEntityKind::Dispatch
                    && item.dispatch_id.as_deref() != Some(&action.target.entity_id)
                {
                    return Err("attention action targets a different dispatch".into());
                }
                if let Some(resource) = &action.resource_precondition {
                    if item.resource_id.as_deref() != Some(&resource.resource_id) {
                        return Err("attention action targets a different resource".into());
                    }
                }
            }
        }
        for task in &wire.tasks {
            require_id(&task.task_id, "task.task_id")?;
            require_positive_version(task.entity_version, "task.entity_version")?;
            if let Some(elapsed_ms) = task.elapsed_ms {
                require_safe_integer(elapsed_ms, "task.elapsed_ms")?;
            }
        }
        for row in &wire.queue {
            require_id(&row.dispatch_id, "queue.dispatch_id")?;
            require_positive_version(row.entity_version, "queue.entity_version")?;
        }
        for row in &wire.worktrees {
            require_id(&row.resource_id, "worktree.resource_id")?;
            require_positive_version(row.entity_version, "worktree.entity_version")?;
            require_safe_integer(row.dirty_generation, "worktree.dirty_generation")?;
            for action in &row.available_actions {
                action.validate()?;
                if action.target.entity_kind != ActionEntityKind::Resource
                    || action.target.entity_id != row.resource_id
                    || action.target.entity_version != row.entity_version
                {
                    return Err("worktree action target does not match its row".into());
                }
                let resource = action.resource_precondition.as_ref().ok_or_else(|| {
                    "worktree mutation lacks a resource precondition".to_string()
                })?;
                if resource.resource_id != row.resource_id
                    || resource.dirty_generation != row.dirty_generation
                {
                    return Err("worktree action generation does not match its row".into());
                }
            }
        }

        Ok(Self {
            schema: wire.schema,
            scope: wire.scope,
            projection_id: wire.projection_id,
            stream_epoch: wire.stream_epoch,
            auth_fence_generation: wire.auth_fence_generation,
            seq: wire.seq,
            attention: wire.attention,
            tasks: wire.tasks,
            queue: wire.queue,
            worktrees: wire.worktrees,
        })
    }
}

impl<'de> Deserialize<'de> for Snapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        SnapshotWire::deserialize(deserializer)?
            .try_into()
            .map_err(D::Error::custom)
    }
}

/// An invalidation signal. A future client re-requests a complete snapshot; it
/// does not reconstruct backend state by applying local deltas.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct Invalidation {
    pub schema: String,
    pub scope_id: String,
    pub projection_id: String,
    pub stream_epoch: String,
    pub auth_fence_generation: u64,
    pub seq: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct InvalidationWire {
    schema: String,
    scope_id: String,
    projection_id: String,
    stream_epoch: String,
    auth_fence_generation: u64,
    seq: u64,
}

impl<'de> Deserialize<'de> for Invalidation {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = InvalidationWire::deserialize(deserializer)?;
        if wire.schema != PROPOSAL_SCHEMA {
            return Err(D::Error::custom(
                "invalidation schema is not the proposed contract schema",
            ));
        }
        require_id(&wire.scope_id, "scope_id").map_err(D::Error::custom)?;
        require_id(&wire.projection_id, "projection_id").map_err(D::Error::custom)?;
        require_id(&wire.stream_epoch, "stream_epoch").map_err(D::Error::custom)?;
        require_safe_integer(wire.auth_fence_generation, "auth_fence_generation")
            .map_err(D::Error::custom)?;
        require_safe_integer(wire.seq, "seq").map_err(D::Error::custom)?;
        Ok(Self {
            schema: wire.schema,
            scope_id: wire.scope_id,
            projection_id: wire.projection_id,
            stream_epoch: wire.stream_epoch,
            auth_fence_generation: wire.auth_fence_generation,
            seq: wire.seq,
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InspectionResource {
    pub resource_id: String,
    pub entity_version: u64,
    pub dirty_generation: u64,
    pub label: PrivacyFilteredText,
    #[serde(default)]
    pub branch: Option<PrivacyFilteredText>,
    #[serde(default)]
    pub dirty_reason: Option<PrivacyFilteredText>,
}

/// Result of beginning inspection. Both capabilities are memory-only secrets:
/// neither belongs in AppState, logs, or Matrix timeline content.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct OutcomeInspection {
    pub schema: String,
    pub scope_id: String,
    pub projection_id: String,
    pub stream_epoch: String,
    pub auth_fence_generation: u64,
    pub snapshot_seq: u64,
    pub client_session_id: String,
    pub inspection_id: String,
    pub inspection_token: OpaqueCapability,
    pub dispatch_target: ActionTarget,
    pub task_id: Option<String>,
    pub terminal_reason: PrivacyFilteredText,
    pub expires_at_unix_ms: u64,
    pub resource: InspectionResource,
    pub resolution_action: AvailableAction,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OutcomeInspectionWire {
    schema: String,
    scope_id: String,
    projection_id: String,
    stream_epoch: String,
    auth_fence_generation: u64,
    snapshot_seq: u64,
    client_session_id: String,
    inspection_id: String,
    inspection_token: OpaqueCapability,
    dispatch_target: ActionTarget,
    #[serde(default)]
    task_id: Option<String>,
    terminal_reason: PrivacyFilteredText,
    expires_at_unix_ms: u64,
    resource: InspectionResource,
    resolution_action: AvailableAction,
}

impl TryFrom<OutcomeInspectionWire> for OutcomeInspection {
    type Error = String;

    fn try_from(wire: OutcomeInspectionWire) -> Result<Self, Self::Error> {
        if wire.schema != PROPOSAL_SCHEMA {
            return Err("inspection schema is not the proposed contract schema".into());
        }
        require_id(&wire.scope_id, "scope_id")?;
        require_id(&wire.projection_id, "projection_id")?;
        require_id(&wire.stream_epoch, "stream_epoch")?;
        require_safe_integer(wire.auth_fence_generation, "auth_fence_generation")?;
        require_safe_integer(wire.snapshot_seq, "snapshot_seq")?;
        require_id(&wire.client_session_id, "client_session_id")?;
        require_id(&wire.inspection_id, "inspection_id")?;
        require_safe_integer(wire.expires_at_unix_ms, "expires_at_unix_ms")?;
        wire.dispatch_target.validate()?;
        if wire.dispatch_target.entity_kind != ActionEntityKind::Dispatch {
            return Err("inspection target must be a dispatch".into());
        }
        require_id(&wire.resource.resource_id, "resource.resource_id")?;
        require_positive_version(wire.resource.entity_version, "resource.entity_version")?;
        require_safe_integer(wire.resource.dirty_generation, "resource.dirty_generation")?;
        wire.resolution_action.validate()?;
        if wire.resolution_action.kind != AvailableActionKind::ResolveOutcome
            || wire.resolution_action.target != wire.dispatch_target
        {
            return Err("resolution action does not match the inspected dispatch".into());
        }
        let resource = wire
            .resolution_action
            .resource_precondition
            .as_ref()
            .ok_or_else(|| "resolution action lacks a resource precondition".to_string())?;
        if resource.resource_id != wire.resource.resource_id
            || resource.dirty_generation != wire.resource.dirty_generation
        {
            return Err("resolution action does not match the inspected resource".into());
        }
        if wire.task_id.is_none()
            && wire.resolution_action.allowed_resolutions
                != [OutcomeResolutionKind::Continue]
        {
            return Err("non-task inspection may only authorize continue".into());
        }

        Ok(Self {
            schema: wire.schema,
            scope_id: wire.scope_id,
            projection_id: wire.projection_id,
            stream_epoch: wire.stream_epoch,
            auth_fence_generation: wire.auth_fence_generation,
            snapshot_seq: wire.snapshot_seq,
            client_session_id: wire.client_session_id,
            inspection_id: wire.inspection_id,
            inspection_token: wire.inspection_token,
            dispatch_target: wire.dispatch_target,
            task_id: wire.task_id,
            terminal_reason: wire.terminal_reason,
            expires_at_unix_ms: wire.expires_at_unix_ms,
            resource: wire.resource,
            resolution_action: wire.resolution_action,
        })
    }
}

impl<'de> Deserialize<'de> for OutcomeInspection {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        OutcomeInspectionWire::deserialize(deserializer)?
            .try_into()
            .map_err(D::Error::custom)
    }
}

/// Strict resolution choice. Terminal choices reject recovery instructions as
/// unknown fields instead of silently discarding them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutcomeResolutionChoice {
    Continue {
        recovery_instruction: PrivacyFilteredText,
    },
    AcceptCompleted,
    KeepBlocked,
}

impl<'de> Deserialize<'de> for OutcomeResolutionChoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        let object = value
            .as_object()
            .ok_or_else(|| D::Error::custom("resolution must be an object"))?;
        let kind = object
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| D::Error::custom("resolution.kind must be a string"))?;

        match kind {
            "continue" => {
                if object.len() != 2 || !object.contains_key("recovery_instruction") {
                    return Err(D::Error::custom(
                        "continue requires only kind and recovery_instruction",
                    ));
                }
                let instruction = object
                    .get("recovery_instruction")
                    .cloned()
                    .ok_or_else(|| D::Error::custom("missing recovery_instruction"))?;
                let recovery_instruction = serde_json::from_value(instruction)
                    .map_err(D::Error::custom)?;
                Ok(Self::Continue {
                    recovery_instruction,
                })
            }
            "accept_completed" => {
                if object.len() != 1 {
                    return Err(D::Error::custom(
                        "accept_completed must not carry additional fields",
                    ));
                }
                Ok(Self::AcceptCompleted)
            }
            "keep_blocked" => {
                if object.len() != 1 {
                    return Err(D::Error::custom(
                        "keep_blocked must not carry additional fields",
                    ));
                }
                Ok(Self::KeepBlocked)
            }
            _ => Err(D::Error::custom("unknown outcome resolution")),
        }
    }
}

/// Proposed command for completing an inspected `outcome_unknown` dispatch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct ResolveOutcomeCommand {
    pub schema: String,
    pub request_id: String,
    pub client_session_id: String,
    pub scope_id: String,
    pub projection_id: String,
    pub stream_epoch: String,
    pub auth_fence_generation: u64,
    pub snapshot_seq: u64,
    pub action_capability: OpaqueCapability,
    pub target: ActionTarget,
    pub resource_precondition: ResourcePrecondition,
    pub inspection_id: String,
    pub inspection_token: OpaqueCapability,
    pub operator_note: PrivacyFilteredText,
    pub resolution: OutcomeResolutionChoice,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ResolveOutcomeCommandWire {
    schema: String,
    request_id: String,
    client_session_id: String,
    scope_id: String,
    projection_id: String,
    stream_epoch: String,
    auth_fence_generation: u64,
    snapshot_seq: u64,
    action_capability: OpaqueCapability,
    target: ActionTarget,
    resource_precondition: ResourcePrecondition,
    inspection_id: String,
    inspection_token: OpaqueCapability,
    operator_note: PrivacyFilteredText,
    resolution: OutcomeResolutionChoice,
}

impl TryFrom<ResolveOutcomeCommandWire> for ResolveOutcomeCommand {
    type Error = String;

    fn try_from(wire: ResolveOutcomeCommandWire) -> Result<Self, Self::Error> {
        if wire.schema != PROPOSAL_SCHEMA {
            return Err("command schema is not the proposed contract schema".into());
        }
        require_id(&wire.request_id, "request_id")?;
        require_id(&wire.client_session_id, "client_session_id")?;
        require_id(&wire.scope_id, "scope_id")?;
        require_id(&wire.projection_id, "projection_id")?;
        require_id(&wire.stream_epoch, "stream_epoch")?;
        require_safe_integer(wire.auth_fence_generation, "auth_fence_generation")?;
        require_safe_integer(wire.snapshot_seq, "snapshot_seq")?;
        wire.target.validate()?;
        if wire.target.entity_kind != ActionEntityKind::Dispatch {
            return Err("resolve_outcome must target a dispatch".into());
        }
        wire.resource_precondition.validate()?;
        require_id(&wire.inspection_id, "inspection_id")?;

        Ok(Self {
            schema: wire.schema,
            request_id: wire.request_id,
            client_session_id: wire.client_session_id,
            scope_id: wire.scope_id,
            projection_id: wire.projection_id,
            stream_epoch: wire.stream_epoch,
            auth_fence_generation: wire.auth_fence_generation,
            snapshot_seq: wire.snapshot_seq,
            action_capability: wire.action_capability,
            target: wire.target,
            resource_precondition: wire.resource_precondition,
            inspection_id: wire.inspection_id,
            inspection_token: wire.inspection_token,
            operator_note: wire.operator_note,
            resolution: wire.resolution,
        })
    }
}

impl<'de> Deserialize<'de> for ResolveOutcomeCommand {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        ResolveOutcomeCommandWire::deserialize(deserializer)?
            .try_into()
            .map_err(D::Error::custom)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposal_snapshot_fixture_matches_the_proposed_model() {
        let fixture = include_str!("../../specs/fixtures/agent-ops-client-v1-proposal/snapshot.json");
        let snapshot: Snapshot = serde_json::from_str(fixture).unwrap();
        assert_eq!(snapshot.schema, PROPOSAL_SCHEMA);
        assert_eq!(snapshot.seq, 42);
        assert_eq!(snapshot.scope.scope_id, "scope-room-agent-1");
        assert_eq!(snapshot.worktrees[0].resource_id, "resource-1");
        assert_eq!(snapshot.worktrees[0].dirty_generation, 2);

        let actions = snapshot
            .attention
            .iter()
            .flat_map(|item| &item.available_actions)
            .chain(
                snapshot
                    .worktrees
                    .iter()
                    .flat_map(|row| &row.available_actions),
            )
            .collect::<Vec<_>>();
        assert!(!actions.is_empty());
        for action in actions {
            assert!(action.target.entity_version > 0);
            assert!(action.expires_at_unix_ms > 0);
            assert!(!action.capability.is_empty());
            if action.kind == AvailableActionKind::MarkResourceInspected {
                assert!(action.resource_precondition.is_some());
            }
        }
    }

    #[test]
    fn proposal_invalidation_fixture_matches_the_proposed_model() {
        let fixture = include_str!("../../specs/fixtures/agent-ops-client-v1-proposal/invalidation.json");
        let invalidation: Invalidation = serde_json::from_str(fixture).unwrap();
        assert_eq!(invalidation.schema, PROPOSAL_SCHEMA);
        assert_eq!(invalidation.scope_id, "scope-room-agent-1");
        assert_eq!(invalidation.seq, 43);
    }

    #[test]
    fn wrong_schema_is_rejected_during_deserialization() {
        let mut fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../specs/fixtures/agent-ops-client-v1-proposal/snapshot.json"
        ))
        .unwrap();
        fixture["schema"] = "io.agentchat.agent_ops.future".into();
        assert!(serde_json::from_value::<Snapshot>(fixture).is_err());
    }

    #[test]
    fn unknown_enum_values_fail_closed() {
        let mut fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../specs/fixtures/agent-ops-client-v1-proposal/snapshot.json"
        ))
        .unwrap();
        fixture["attention"][0]["available_actions"][0]["kind"] =
            "some_future_action".into();
        assert!(serde_json::from_value::<Snapshot>(fixture).is_err());
    }

    #[test]
    fn complete_projection_rejects_embedded_filesystem_paths() {
        for text in [
            "/Users/alex/robrix2",
            "workspace: /Users/alex/robrix2 is dirty",
            "path=~/Work/robrix2",
            "path=~alice/Work/robrix2",
            r"workspace C:\Users\alex\robrix2",
            r"resource \\server\share\robrix2",
        ] {
            let mut fixture: serde_json::Value = serde_json::from_str(include_str!(
                "../../specs/fixtures/agent-ops-client-v1-proposal/snapshot.json"
            ))
            .unwrap();
            fixture["attention"][0]["summary"] = text.into();
            assert!(
                serde_json::from_value::<Snapshot>(fixture).is_err(),
                "filesystem path should reject the projection: {text}",
            );
        }
    }

    #[test]
    fn branch_names_are_not_mistaken_for_absolute_paths() {
        let parsed: PrivacyFilteredText =
            serde_json::from_str("\"agentchat/wf/thread-a\"").unwrap();
        assert_eq!(parsed.as_str(), "agentchat/wf/thread-a");
    }

    #[test]
    fn proposal_inspection_fixture_carries_explicit_resolution_grants() {
        let fixture = include_str!("../../specs/fixtures/agent-ops-client-v1-proposal/outcome-inspection.json");
        let inspection: OutcomeInspection = serde_json::from_str(fixture).unwrap();
        assert_eq!(inspection.schema, PROPOSAL_SCHEMA);
        assert_eq!(
            inspection.resolution_action.kind,
            AvailableActionKind::ResolveOutcome,
        );
        assert_eq!(
            inspection.resolution_action.allowed_resolutions,
            [
                OutcomeResolutionKind::Continue,
                OutcomeResolutionKind::AcceptCompleted,
                OutcomeResolutionKind::KeepBlocked,
            ],
        );
        assert_eq!(inspection.resource.dirty_generation, 2);
    }

    #[test]
    fn resolution_fixtures_cover_the_complete_outcome_closure() {
        let fixtures = [
            include_str!("../../specs/fixtures/agent-ops-client-v1-proposal/resolve-outcome-request-continue.json"),
            include_str!("../../specs/fixtures/agent-ops-client-v1-proposal/resolve-outcome-request-accept-completed.json"),
            include_str!("../../specs/fixtures/agent-ops-client-v1-proposal/resolve-outcome-request-keep-blocked.json"),
        ];
        let commands: Vec<ResolveOutcomeCommand> = fixtures
            .into_iter()
            .map(|fixture| serde_json::from_str(fixture).unwrap())
            .collect();

        assert!(matches!(
            commands[0].resolution,
            OutcomeResolutionChoice::Continue { .. },
        ));
        assert_eq!(
            commands[1].resolution,
            OutcomeResolutionChoice::AcceptCompleted,
        );
        assert_eq!(
            commands[2].resolution,
            OutcomeResolutionChoice::KeepBlocked,
        );
        for command in commands {
            let encoded = serde_json::to_value(&command).unwrap();
            let decoded: ResolveOutcomeCommand = serde_json::from_value(encoded).unwrap();
            assert_eq!(decoded, command);
        }
    }

    #[test]
    fn terminal_resolution_rejects_recovery_instruction() {
        let fixture = include_str!(
            "../../specs/fixtures/agent-ops-client-v1-proposal/invalid-resolve-outcome-request-accept-with-recovery.json"
        );
        assert!(serde_json::from_str::<ResolveOutcomeCommand>(fixture).is_err());
    }

    #[test]
    fn capability_debug_output_is_redacted_and_empty_values_are_rejected() {
        let capability: OpaqueCapability = serde_json::from_str("\"fixture-secret\"").unwrap();
        let debug = format!("{capability:?}");
        assert!(!debug.contains("fixture-secret"));
        assert!(debug.contains("<redacted>"));
        assert!(!capability.is_empty());
        assert!(serde_json::from_str::<OpaqueCapability>("\"   \"").is_err());
    }
}
