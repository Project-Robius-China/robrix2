//! Account-scoped mission operations dashboard.

use serde_json::Value as JsonValue;

use crate::i18n::AppLanguage;

use super::capability_descriptors;
use super::splash_host::{CapabilitySchema, HostError};
use super::{AppFactory, RenderFailure, RenderedApp, ValidationError};

pub const TYPE_KEY: &str = "mission_dashboard";

const MAX_TEXT_CHARS: usize = 120;
const MAX_MISSIONS: usize = 4;

pub static FACTORY: MissionDashboardFactory = MissionDashboardFactory;
pub(crate) static MISSION_DASHBOARD_CAPABILITY_SCHEMA: MissionDashboardCapabilitySchema =
    MissionDashboardCapabilitySchema;

pub(crate) struct MissionDashboardCapabilitySchema;

pub struct MissionDashboardFactory;

impl AppFactory for MissionDashboardFactory {
    fn supported_version(&self) -> u32 {
        1
    }

    fn init(&self, initial_state: &JsonValue) -> Result<Box<dyn RenderedApp>, ValidationError> {
        let obj = initial_state
            .as_object()
            .ok_or_else(|| ValidationError::new("initial_state", "must be a JSON object"))?;

        let summary = parse_summary(obj)?;
        let missions = parse_missions(obj)?;

        Ok(Box::new(RenderedMissionDashboard {
            summary,
            missions,
        }))
    }
}

impl CapabilitySchema for MissionDashboardCapabilitySchema {
    fn app_type(&self) -> &'static str {
        TYPE_KEY
    }

    fn app_version(&self) -> u32 {
        FACTORY.supported_version()
    }

    fn contains_path(&self, path: &str) -> bool {
        matches!(
            path,
            "$state.summary.title"
                | "$state.summary.status"
                | "$state.summary.open_count"
                | "$state.mission_1.visible"
                | "$state.mission_1.title"
                | "$state.mission_1.room"
                | "$state.mission_1.phase"
                | "$state.mission_1.pending"
                | "$state.mission_1.blockers"
                | "$state.mission_1.agents"
                | "$state.mission_2.visible"
                | "$state.mission_2.title"
                | "$state.mission_2.room"
                | "$state.mission_2.phase"
                | "$state.mission_2.pending"
                | "$state.mission_2.blockers"
                | "$state.mission_2.agents"
                | "$state.mission_3.visible"
                | "$state.mission_3.title"
                | "$state.mission_3.room"
                | "$state.mission_3.phase"
                | "$state.mission_3.pending"
                | "$state.mission_3.blockers"
                | "$state.mission_3.agents"
                | "$state.mission_4.visible"
                | "$state.mission_4.title"
                | "$state.mission_4.room"
                | "$state.mission_4.phase"
                | "$state.mission_4.pending"
                | "$state.mission_4.blockers"
                | "$state.mission_4.agents"
        )
    }
}

#[derive(Debug, Clone)]
struct DashboardSummary {
    title: String,
    status: String,
}

#[derive(Debug, Clone)]
struct MissionSummary {
    title: String,
    room: String,
    phase: String,
    pending_human_actions: u64,
    blocked_tasks: u64,
    active_agents: u64,
}

#[derive(Debug, Clone)]
pub struct RenderedMissionDashboard {
    summary: DashboardSummary,
    missions: Vec<MissionSummary>,
}

impl RenderedApp for RenderedMissionDashboard {
    fn app_type(&self) -> &'static str {
        TYPE_KEY
    }

    fn render(&self, _app_language: AppLanguage) -> Result<String, RenderFailure> {
        render_mission_dashboard(self)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
struct MissionSlot {
    visible: bool,
    title: String,
    room: String,
    phase: String,
    pending: String,
    blockers: String,
    agents: String,
}

fn render_mission_dashboard(state: &RenderedMissionDashboard) -> Result<String, RenderFailure> {
    let host = super::splash_host::splash_host();
    let chrome = capability_descriptors::chrome_for(TYPE_KEY).ok_or_else(|| {
        RenderFailure::Internal {
            reason: format!("missing capability descriptor for {TYPE_KEY}"),
        }
    })?;
    let handle = host
        .load_template("mission_dashboard", "account_overview")
        .map_err(dashboard_host_error_to_render_failure)?;
    host.render_to_splash(&handle, &template_state(state), &chrome)
        .map_err(dashboard_host_error_to_render_failure)
}

fn dashboard_host_error_to_render_failure(err: HostError) -> RenderFailure {
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

fn template_state(state: &RenderedMissionDashboard) -> JsonValue {
    serde_json::json!({
        "summary": {
            "title": state.summary.title,
            "status": state.summary.status,
            "open_count": format!("{} missions", state.missions.len()),
        },
        "mission_1": mission_slot(state.missions.first()),
        "mission_2": mission_slot(state.missions.get(1)),
        "mission_3": mission_slot(state.missions.get(2)),
        "mission_4": mission_slot(state.missions.get(3)),
    })
}

fn mission_slot(mission: Option<&MissionSummary>) -> MissionSlot {
    mission.map(|mission| MissionSlot {
        visible: true,
        title: mission.title.clone(),
        room: mission.room.clone(),
        phase: mission.phase.clone(),
        pending: format!("{} pending", mission.pending_human_actions),
        blockers: format!("{} blocked", mission.blocked_tasks),
        agents: format!("{} active agents", mission.active_agents),
    })
    .unwrap_or_default()
}

fn parse_summary(obj: &serde_json::Map<String, JsonValue>) -> Result<DashboardSummary, ValidationError> {
    let summary = obj
        .get("summary")
        .and_then(JsonValue::as_object)
        .ok_or_else(|| ValidationError::new("summary", "required field missing or not an object"))?;
    Ok(DashboardSummary {
        title: parse_required_text(summary, "title")?,
        status: parse_status(summary, "status", &["active", "paused", "completed"])?,
    })
}

fn parse_missions(obj: &serde_json::Map<String, JsonValue>) -> Result<Vec<MissionSummary>, ValidationError> {
    let Some(missions) = obj.get("missions").and_then(JsonValue::as_array) else {
        return Ok(Vec::new());
    };
    missions
        .iter()
        .take(MAX_MISSIONS)
        .enumerate()
        .map(|(index, raw)| {
            let mission = raw.as_object().ok_or_else(|| {
                ValidationError::new("missions", format!("entry {index} must be an object"))
            })?;
            Ok(MissionSummary {
                title: parse_required_text(mission, "title")?,
                room: parse_required_text(mission, "room")?,
                phase: parse_status(
                    mission,
                    "phase",
                    &["planning", "active", "paused", "completed"],
                )?,
                pending_human_actions: parse_count(mission, "pending_human_actions")?,
                blocked_tasks: parse_count(mission, "blocked_tasks")?,
                active_agents: parse_count(mission, "active_agents")?,
            })
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

fn parse_count(
    obj: &serde_json::Map<String, JsonValue>,
    key: &'static str,
) -> Result<u64, ValidationError> {
    obj.get(key)
        .and_then(JsonValue::as_u64)
        .ok_or_else(|| ValidationError::new(key, "required field missing or not a non-negative integer"))
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
