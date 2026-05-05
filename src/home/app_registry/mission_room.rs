//! Room-scoped Agent Mission Room card.

use serde_json::Value as JsonValue;

use crate::i18n::AppLanguage;

use super::{AppFactory, RenderFailure, RenderedApp, ValidationError};
use super::capability_descriptors;
use super::splash_host::{CapabilitySchema, HostError};

pub const TYPE_KEY: &str = "mission_room";

const MAX_TEXT_CHARS: usize = 120;
const MAX_TASKS: usize = 4;
const MAX_AGENTS: usize = 4;
const MAX_ACTIONS: usize = 3;

pub static FACTORY: MissionRoomFactory = MissionRoomFactory;
pub(crate) static MISSION_ROOM_CAPABILITY_SCHEMA: MissionRoomCapabilitySchema =
    MissionRoomCapabilitySchema;

pub(crate) struct MissionRoomCapabilitySchema;

pub struct MissionRoomFactory;

impl AppFactory for MissionRoomFactory {
    fn supported_version(&self) -> u32 {
        1
    }

    fn init(&self, initial_state: &JsonValue) -> Result<Box<dyn RenderedApp>, ValidationError> {
        let obj = initial_state
            .as_object()
            .ok_or_else(|| ValidationError::new("initial_state", "must be a JSON object"))?;

        let goal = parse_goal(obj)?;
        let phase = parse_status(obj, "phase", &["planning", "active", "paused", "completed"])?;
        let tasks = parse_tasks(obj)?;
        let agents = parse_agents(obj)?;
        let pending_human_actions = parse_pending_actions(obj)?;
        let decisions = parse_text_array(obj, "decisions")?;
        let blockers = parse_text_array(obj, "blockers")?;

        Ok(Box::new(RenderedMissionRoom {
            goal,
            phase,
            tasks,
            agents,
            pending_human_actions,
            decisions,
            blockers,
        }))
    }
}

impl CapabilitySchema for MissionRoomCapabilitySchema {
    fn app_type(&self) -> &'static str {
        TYPE_KEY
    }

    fn app_version(&self) -> u32 {
        FACTORY.supported_version()
    }

    fn contains_path(&self, path: &str) -> bool {
        matches!(
            path,
            "$state.goal.title"
                | "$state.goal.status"
                | "$state.phase"
                | "$state.progress.text"
                | "$state.next_gate.visible"
                | "$state.next_gate.text"
                | "$state.task_1.visible"
                | "$state.task_1.title"
                | "$state.task_1.status"
                | "$state.task_1.owner"
                | "$state.task_1.priority"
                | "$state.task_2.visible"
                | "$state.task_2.title"
                | "$state.task_2.status"
                | "$state.task_2.owner"
                | "$state.task_2.priority"
                | "$state.task_3.visible"
                | "$state.task_3.title"
                | "$state.task_3.status"
                | "$state.task_3.owner"
                | "$state.task_3.priority"
                | "$state.task_4.visible"
                | "$state.task_4.title"
                | "$state.task_4.status"
                | "$state.task_4.owner"
                | "$state.task_4.priority"
                | "$state.agent_1.visible"
                | "$state.agent_1.label"
                | "$state.agent_2.visible"
                | "$state.agent_2.label"
                | "$state.agent_3.visible"
                | "$state.agent_3.label"
                | "$state.agent_4.visible"
                | "$state.agent_4.label"
                | "$state.action_1.visible"
                | "$state.action_1.label"
                | "$state.action_2.visible"
                | "$state.action_2.label"
                | "$state.action_3.visible"
                | "$state.action_3.label"
                | "$state.decision_summary.visible"
                | "$state.decision_summary.text"
                | "$state.blocker_summary.visible"
                | "$state.blocker_summary.text"
        )
    }
}

#[derive(Debug, Clone)]
struct MissionGoal {
    title: String,
    status: String,
}

#[derive(Debug, Clone)]
struct MissionTask {
    title: String,
    status: String,
    owner_agent: String,
    priority: String,
}

#[derive(Debug, Clone)]
struct MissionAgent {
    id: String,
    role: String,
    status: String,
    current_task_id: String,
}

#[derive(Debug, Clone)]
struct MissionAction {
    label: String,
}

#[derive(Debug, Clone)]
pub struct RenderedMissionRoom {
    goal: MissionGoal,
    phase: String,
    tasks: Vec<MissionTask>,
    agents: Vec<MissionAgent>,
    pending_human_actions: Vec<MissionAction>,
    decisions: Vec<String>,
    blockers: Vec<String>,
}

impl RenderedApp for RenderedMissionRoom {
    fn app_type(&self) -> &'static str {
        TYPE_KEY
    }

    fn render(&self, _app_language: AppLanguage) -> Result<String, RenderFailure> {
        render_mission_room(self)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
struct VisibleSlot {
    visible: bool,
    title: String,
    status: String,
    owner: String,
    priority: String,
    label: String,
}

fn render_mission_room(state: &RenderedMissionRoom) -> Result<String, RenderFailure> {
    let host = super::splash_host::splash_host();
    let chrome = capability_descriptors::chrome_for(TYPE_KEY).ok_or_else(|| {
        RenderFailure::Internal {
            reason: format!("missing capability descriptor for {TYPE_KEY}"),
        }
    })?;
    let handle = host
        .load_template("mission_room", "mission_control")
        .map_err(mission_host_error_to_render_failure)?;
    host.render_to_splash(&handle, &template_state(state), &chrome)
        .map_err(mission_host_error_to_render_failure)
}

fn mission_host_error_to_render_failure(err: HostError) -> RenderFailure {
    match err {
        HostError::TemplateNotFound {
            capability_id,
            template_id,
        } => RenderFailure::TemplateMissing {
            capability_id,
            template_id,
        },
        HostError::ParseError { .. }
        | HostError::WidgetNotAllowed { .. }
        | HostError::LocalFunctionNotAllowed { .. }
        | HostError::AttributionFieldInTemplate { .. }
        | HostError::BindingPathNotInSchema { .. } => RenderFailure::HostRejected {
            reason: err.to_string(),
        },
        HostError::BindingError { .. }
        | HostError::UpdateOpNotYetSupported { .. }
        | HostError::GeneratedTemplateNotYetSupported => RenderFailure::HostError {
            reason: err.to_string(),
        },
    }
}

fn template_state(state: &RenderedMissionRoom) -> JsonValue {
    let task_1 = task_slot(state.tasks.first());
    let task_2 = task_slot(state.tasks.get(1));
    let task_3 = task_slot(state.tasks.get(2));
    let task_4 = task_slot(state.tasks.get(3));
    let agent_1 = agent_slot(state.agents.first());
    let agent_2 = agent_slot(state.agents.get(1));
    let agent_3 = agent_slot(state.agents.get(2));
    let agent_4 = agent_slot(state.agents.get(3));
    let action_1 = action_slot(state.pending_human_actions.first());
    let action_2 = action_slot(state.pending_human_actions.get(1));
    let action_3 = action_slot(state.pending_human_actions.get(2));
    let progress = if state.tasks.is_empty() {
        "No tasks".to_string()
    } else {
        let done = state.tasks.iter().filter(|task| task.status == "done").count();
        format!("{done}/{} tasks done", state.tasks.len())
    };
    serde_json::json!({
        "goal": {
            "title": state.goal.title,
            "status": state.goal.status,
        },
        "phase": state.phase,
        "progress": { "text": progress },
        "next_gate": {
            "visible": !state.pending_human_actions.is_empty(),
            "text": state.pending_human_actions.first().map(|a| a.label.as_str()).unwrap_or(""),
        },
        "task_1": task_1,
        "task_2": task_2,
        "task_3": task_3,
        "task_4": task_4,
        "agent_1": agent_1,
        "agent_2": agent_2,
        "agent_3": agent_3,
        "agent_4": agent_4,
        "action_1": action_1,
        "action_2": action_2,
        "action_3": action_3,
        "decision_summary": {
            "visible": !state.decisions.is_empty(),
            "text": state.decisions.first().map(String::as_str).unwrap_or(""),
        },
        "blocker_summary": {
            "visible": !state.blockers.is_empty(),
            "text": state.blockers.first().map(String::as_str).unwrap_or(""),
        },
    })
}

fn task_slot(task: Option<&MissionTask>) -> VisibleSlot {
    task.map(|task| VisibleSlot {
        visible: true,
        title: task.title.clone(),
        status: task.status.clone(),
        owner: task.owner_agent.clone(),
        priority: task.priority.clone(),
        label: String::new(),
    })
    .unwrap_or_default()
}

fn agent_slot(agent: Option<&MissionAgent>) -> VisibleSlot {
    agent.map(|agent| VisibleSlot {
        visible: true,
        label: format!(
            "{} / {} / {} / {}",
            agent.id, agent.role, agent.status, agent.current_task_id
        ),
        ..VisibleSlot::default()
    })
    .unwrap_or_default()
}

fn action_slot(action: Option<&MissionAction>) -> VisibleSlot {
    action.map(|action| VisibleSlot {
        visible: true,
        label: action.label.clone(),
        ..VisibleSlot::default()
    })
    .unwrap_or_default()
}

fn parse_goal(obj: &serde_json::Map<String, JsonValue>) -> Result<MissionGoal, ValidationError> {
    let goal = obj
        .get("goal")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| ValidationError::new("goal", "required field missing or not an object"))?;
    Ok(MissionGoal {
        title: parse_required_text(goal, "title")?,
        status: parse_status(goal, "status", &["planning", "active", "paused", "completed"])?,
    })
}

fn parse_tasks(obj: &serde_json::Map<String, JsonValue>) -> Result<Vec<MissionTask>, ValidationError> {
    let Some(tasks) = obj.get("tasks").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    tasks
        .iter()
        .take(MAX_TASKS)
        .enumerate()
        .map(|(index, raw)| {
            let task = raw.as_object().ok_or_else(|| {
                ValidationError::new("tasks", format!("entry {index} must be an object"))
            })?;
            Ok(MissionTask {
                title: parse_required_text(task, "title")?,
                status: parse_status(
                    task,
                    "status",
                    &["planning", "approved", "doing", "review", "blocked", "done"],
                )?,
                owner_agent: parse_optional_text(task, "owner_agent")?.unwrap_or_default(),
                priority: parse_status(task, "priority", &["low", "normal", "high"])?,
            })
        })
        .collect()
}

fn parse_agents(obj: &serde_json::Map<String, JsonValue>) -> Result<Vec<MissionAgent>, ValidationError> {
    let Some(agents) = obj.get("agents").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    agents
        .iter()
        .take(MAX_AGENTS)
        .enumerate()
        .map(|(index, raw)| {
            let agent = raw.as_object().ok_or_else(|| {
                ValidationError::new("agents", format!("entry {index} must be an object"))
            })?;
            Ok(MissionAgent {
                id: parse_required_text(agent, "id")?,
                role: parse_required_text(agent, "role")?,
                status: parse_status(
                    agent,
                    "status",
                    &["idle", "working", "blocked", "waiting_human"],
                )?,
                current_task_id: parse_optional_text(agent, "current_task_id")?.unwrap_or_default(),
            })
        })
        .collect()
}

fn parse_pending_actions(obj: &serde_json::Map<String, JsonValue>) -> Result<Vec<MissionAction>, ValidationError> {
    let Some(actions) = obj.get("pending_human_actions").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    actions
        .iter()
        .take(MAX_ACTIONS)
        .enumerate()
        .map(|(index, raw)| {
            let action = raw.as_object().ok_or_else(|| {
                ValidationError::new(
                    "pending_human_actions",
                    format!("entry {index} must be an object"),
                )
            })?;
            let _kind = parse_status(
                action,
                "kind",
                &[
                    "approve_plan",
                    "request_plan_changes",
                    "pause_mission",
                    "resume_mission",
                    "reassign_task",
                    "change_priority",
                    "mark_blocked",
                    "request_review",
                    "approve_result",
                ],
            )?;
            Ok(MissionAction {
                label: parse_required_text(action, "label")?,
            })
        })
        .collect()
}

fn parse_text_array(obj: &serde_json::Map<String, JsonValue>, key: &'static str) -> Result<Vec<String>, ValidationError> {
    let Some(values) = obj.get(key) else {
        return Ok(Vec::new());
    };
    let values = values
        .as_array()
        .ok_or_else(|| ValidationError::new(key, "must be an array"))?;
    values
        .iter()
        .take(3)
        .map(|value| {
            let raw = value
                .as_str()
                .ok_or_else(|| ValidationError::new(key, "entries must be strings"))?;
            Ok(truncate_text(raw.trim()))
        })
        .collect()
}

fn parse_required_text(
    obj: &serde_json::Map<String, JsonValue>,
    key: &'static str,
) -> Result<String, ValidationError> {
    let raw = obj
        .get(key)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| ValidationError::new(key, "required field missing or not a string"))?;
    let value = truncate_text(raw.trim());
    if value.is_empty() {
        return Err(ValidationError::new(key, "must not be empty"));
    }
    Ok(value)
}

fn parse_optional_text(
    obj: &serde_json::Map<String, JsonValue>,
    key: &'static str,
) -> Result<Option<String>, ValidationError> {
    match obj.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => {
            let raw = value
                .as_str()
                .ok_or_else(|| ValidationError::new(key, "must be a string"))?;
            let value = truncate_text(raw.trim());
            Ok((!value.is_empty()).then_some(value))
        }
    }
}

fn parse_status(
    obj: &serde_json::Map<String, JsonValue>,
    key: &'static str,
    allowed: &[&str],
) -> Result<String, ValidationError> {
    let raw = obj
        .get(key)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| ValidationError::new(key, "required field missing or not a string"))?;
    if allowed.contains(&raw) {
        Ok(raw.to_string())
    } else {
        Err(ValidationError::new(
            key,
            format!("must be one of {}", allowed.join(", ")),
        ))
    }
}

fn truncate_text(raw: &str) -> String {
    let mut out = raw.chars().take(MAX_TEXT_CHARS).collect::<String>();
    if raw.chars().count() > MAX_TEXT_CHARS {
        out.push_str("...");
    }
    out
}
