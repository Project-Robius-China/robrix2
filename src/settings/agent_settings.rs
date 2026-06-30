//! The "Agents" configuration section in Settings ▸ Labs.
//!
//! This is the write entry point for the global `AgentRegistry` (PR #196):
//! the user searches the Matrix user directory, picks a framework
//! (Octos / Hermes / OpenClaw), and registers the selected account as an agent.
//! It lives alongside the existing App Service (`BotSettings`) card — Octos
//! app-service agents are still configured there; this card registers *any*
//! Matrix account as an agent into the registry.
//!
//! Visual language follows `docs/ui-visual-spec-zh.md` (SectionCard recipe §4.1,
//! StatusBadge / CapabilityChip §4.2/§4.4, SegmentedTabs §4.9) using the `RBX_*`
//! design tokens. Search reuses `invite_modal.rs`'s pre-declared result slots.
//!
//! NOTE: inside the `script_mod!` block, only `//` comments are allowed.

use makepad_widgets::*;
use ruma::OwnedUserId;

use crate::{
    app::{AgentEntry, AgentFramework, AppState, BotSettingsState},
    i18n::{AppLanguage, tr_key},
    persistence,
    profile::user_profile::UserProfile,
    settings::bot_settings::{OctosHealthState, OctosHealthStatus},
    shared::avatar::AvatarState,
    sliding_sync::{MatrixRequest, current_user_id, submit_async_request},
};

/// The frameworks a user can pick when registering an agent from search.
/// `Unknown` is reserved for migrated legacy bots and is not user-selectable.
pub fn framework_options() -> [AgentFramework; 3] {
    [
        AgentFramework::Octos,
        AgentFramework::Hermes,
        AgentFramework::OpenClaw,
    ]
}

/// Human-readable, stable label for an `AgentFramework`.
pub fn framework_label(framework: AgentFramework) -> &'static str {
    match framework {
        AgentFramework::Octos => "Octos",
        AgentFramework::Hermes => "Hermes",
        AgentFramework::OpenClaw => "OpenClaw",
        AgentFramework::Unknown => "Unknown",
    }
}

/// Two-letter monogram for an `AgentFramework`'s identity tile.
pub fn framework_mono(framework: AgentFramework) -> &'static str {
    match framework {
        AgentFramework::Octos => "Oc",
        AgentFramework::Hermes => "He",
        AgentFramework::OpenClaw => "Cl",
        AgentFramework::Unknown => "Ag",
    }
}

/// Registers a searched Matrix account as an agent in the global registry.
///
/// Idempotent: an already-registered `agent_mxid` is left untouched (its
/// existing `AgentEntry` is NOT overwritten). The App Service binding state
/// (`known_bot_user_ids`, `room_bindings`) is never modified here.
///
/// Returns `true` if a new agent was inserted, `false` if it already existed.
pub fn register_agent_from_search(
    app_state: &mut AppState,
    user_id: OwnedUserId,
    display_name: Option<String>,
    framework: AgentFramework,
) -> bool {
    app_state.agent_registry.register(
        user_id,
        AgentEntry {
            display_name,
            framework,
            ..Default::default()
        },
    )
}

pub fn parse_agent_user_id(raw: &str) -> Result<OwnedUserId, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err("Enter a full Matrix user ID.".into());
    }
    if !trimmed.contains(':') {
        return Err("Enter a full Matrix user ID, like agent:server.".into());
    }
    let normalized;
    let mxid = if trimmed.starts_with('@') {
        trimmed
    } else {
        normalized = format!("@{trimmed}");
        &normalized
    };
    mxid
        .try_into()
        .map_err(|error| format!("Invalid Matrix user ID: {error}"))
}

pub fn agent_row_shows_recheck(framework: AgentFramework) -> bool {
    framework == AgentFramework::Octos
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct AgentRegistrySummary {
    total: usize,
    octos: usize,
    direct: usize,
    unknown: usize,
}

fn agent_registry_summary(app_state: &AppState) -> AgentRegistrySummary {
    let mut summary = AgentRegistrySummary::default();
    for user_id in app_state.agent_registry.agent_user_ids() {
        summary.total += 1;
        match app_state
            .agent_registry
            .get(user_id.as_ref())
            .map(|entry| entry.framework)
            .unwrap_or_default()
        {
            AgentFramework::Octos => summary.octos += 1,
            AgentFramework::Hermes | AgentFramework::OpenClaw => summary.direct += 1,
            AgentFramework::Unknown => summary.unknown += 1,
        }
    }
    summary
}

const AGENT_SETTINGS_OCTOS_HEALTH_REQUEST_ID: LiveId = live_id!(agent_settings_octos_health);

/// Everything an [`AgentRegistryRow`] needs to fully render itself and know its
/// own identity, passed in through `scope.props` on each redraw. Health state
/// for the per-row dot and whether to show "Re-check" lives in [`AgentSettings`]
/// (its `octos_health` + `app_state.agent_registry`), not in `AgentEntry`, so we
/// fold it into this props struct rather than passing the bare `AgentEntry`.
#[derive(Clone, Debug)]
pub struct AgentRowProps {
    pub user_id: OwnedUserId,
    pub display_name: String,
    pub framework: AgentFramework,
    pub health: Option<OctosHealthStatus>,
    pub shows_recheck: bool,
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // A registered-agent card (design handoff §1 "Agent row"): framework tile +
    // name/mxid + framework badge, with a footer of row actions. Registered as a
    // Widget so the registry list can drive it from a FlatList (one item per
    // agent), mirroring DeviceCard in devices_settings.rs.
    mod.widgets.AgentRegistryRow = #(AgentRegistryRow::register_widget(vm)) {
        width: Fill
        height: Fit
        flow: Down
        spacing: 0
        margin: Inset{top: 3, bottom: 3}
        padding: Inset{left: 12, right: 12, top: 8, bottom: 8}
        new_batch: true
        show_bg: true
        draw_bg +: {
            color: (RBX_BG_SURFACE)
            border_radius: (RBX_RADIUS_XXS)
            border_size: 1.0
            border_color: (RBX_STROKE_SOFT)
        }

        agent_top_row := View {
            width: Fill
            height: Fit
            flow: Right
            align: Align{y: 0.5}
            spacing: 11

            agent_tile := RoundedView {
                width: 30
                height: 30
                align: Align{x: 0.5, y: 0.5}
                new_batch: true
                show_bg: true
                draw_bg +: {
                    color: (RBX_BG_SURFACE_SUBTLE)
                    border_radius: (RBX_RADIUS_XXS)
                }
                agent_tile_mono := Label {
                    width: Fit
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: TITLE_TEXT { font_size: 13.0 }
                    }
                    text: ""
                }
            }

            agent_text_col := View {
                width: Fill
                height: Fit
                flow: Down
                spacing: 1

                agent_name_row := View {
                    width: Fill
                    height: Fit
                    flow: Right
                    align: Align{y: 0.5}
                    spacing: 6

                    agent_name_label := Label {
                        width: Fill
                        height: Fit
                        draw_text +: {
                            color: (RBX_FG_PRIMARY)
                            text_style: TITLE_TEXT { font_size: 13.0 }
                        }
                        text: ""
                    }
                    agent_health_dot := RoundedView {
                        visible: false
                        width: 7
                        height: 7
                        show_bg: true
                        draw_bg +: {
                            color: (RBX_NEUTRAL_FG)
                            border_radius: (RBX_RADIUS_PILL)
                        }
                    }
                }
                agent_mxid_label := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_TERTIARY)
                        text_style: REGULAR_TEXT { font_size: 9.0 }
                    }
                    text: ""
                }
            }

            agent_framework_badge := RoundedView {
                width: Fit
                height: Fit
                flow: Right
                align: Align{y: 0.5}
                spacing: 5
                padding: Inset{left: 9, right: 9, top: 4, bottom: 4}
                new_batch: true
                show_bg: true
                draw_bg +: {
                    color: (RBX_ACCENT_SOFT)
                    border_radius: (RBX_RADIUS_XXS)
                }
                agent_framework_label := Label {
                    width: Fit
                    height: Fit
                    draw_text +: {
                        color: (RBX_ACCENT)
                        text_style: TITLE_TEXT { font_size: 9.0 }
                    }
                    text: ""
                }
                }
            }

            agent_actions_row := View {
                width: Fill
                height: Fit
                flow: Right
                spacing: 8
                margin: Inset{top: 7}
                show_bg: true
                draw_bg +: { color: (RBX_TRANSPARENT) }

            agent_open_chat_button := RobrixIconButton {
                width: Fill
                height: Fit
                padding: Inset{top: 5, bottom: 5, left: 8, right: 8}
                icon_walk: Walk{width: 0, height: 0}
                spacing: 0
                text: "Open chat"
                draw_bg +: {
                    color: (RBX_BG_SURFACE_SUBTLE)
                    color_hover: (RBX_BG_HOVER)
                    color_down: (RBX_BG_PRESSED)
                    border_radius: (RBX_RADIUS_XXS)
                }
                draw_text +: { color: (RBX_FG_PRIMARY), color_hover: (RBX_FG_PRIMARY), color_down: (RBX_FG_PRIMARY) }
            }
            agent_recheck_button := RobrixIconButton {
                width: Fill
                height: Fit
                padding: Inset{top: 5, bottom: 5, left: 8, right: 8}
                icon_walk: Walk{width: 0, height: 0}
                spacing: 0
                text: "Re-check"
                draw_bg +: {
                    color: (RBX_INFO_BG)
                    color_hover: (RBX_HIT_HOVER)
                    color_down: (RBX_HIT_DOWN)
                    border_radius: (RBX_RADIUS_XXS)
                }
                draw_text +: { color: (RBX_INFO_FG), color_hover: (RBX_INFO_FG), color_down: (RBX_INFO_FG) }
            }
            agent_unbind_button := RobrixIconButton {
                width: Fill
                height: Fit
                padding: Inset{top: 5, bottom: 5, left: 8, right: 8}
                icon_walk: Walk{width: 0, height: 0}
                spacing: 0
                text: "Unbind"
                draw_bg +: {
                    color: (RBX_DANGER_BG)
                    color_hover: (RBX_DANGER_BG)
                    color_down: (RBX_DANGER_BG)
                    border_radius: (RBX_RADIUS_XXS)
                }
                draw_text +: { color: (RBX_DANGER_FG), color_hover: (RBX_DANGER_FG), color_down: (RBX_DANGER_FG) }
            }
        }
    }

    let AgentStatTile = RoundedView {
        width: Fill
        height: Fit
        flow: Down
        spacing: 2
        padding: Inset{left: 10, right: 10, top: 9, bottom: 9}
        new_batch: true
        show_bg: true
        draw_bg +: {
            color: (RBX_BG_SURFACE_SUBTLE)
            border_radius: (RBX_RADIUS_XXS)
            border_size: 1.0
            border_color: (RBX_STROKE_SOFT)
        }
        stat_value := Label {
            width: Fill
            height: Fit
            draw_text +: {
                color: (RBX_FG_PRIMARY)
                text_style: TITLE_TEXT { font_size: 15.0 }
            }
            text: "0"
        }
        stat_label := Label {
            width: Fill
            height: Fit
            draw_text +: {
                color: (RBX_FG_SECONDARY)
                text_style: RBX_TEXT_BADGE {}
            }
            text: ""
        }
    }

    mod.widgets.AgentSettings = #(AgentSettings::register_widget(vm)) {
        width: Fill
        height: Fit
        flow: Down
        spacing: (SPACE_SM)

        agents_header := View {
            width: Fill
            height: Fit
            flow: Down
            spacing: (SPACE_XS)

            agents_title := Label {
                width: Fit
                height: Fit
                draw_text +: {
                    color: (RBX_FG_PRIMARY)
                    text_style: RBX_TEXT_PAGE_TITLE {}
                }
                text: "Agent Access"
            }
            agents_description := Label {
                width: Fill
                height: Fit
                draw_text +: {
                    color: (RBX_FG_SECONDARY)
                    text_style: RBX_TEXT_META {}
                }
                text: "Bind a Matrix account, mark which agent framework it uses, and let Robrix identify it across rooms."
            }
        }

        add_agent_button := RobrixIconButton {
            width: Fill
            height: (RBX_CONTROL_H_LG)
            padding: Inset{top: 11, bottom: 11, left: 16, right: 16}
            draw_icon.svg: (ICON_ADD)
            draw_icon.color: (RBX_FG_ON_ACCENT)
            icon_walk: Walk{width: 16, height: 16, margin: Inset{right: 7}}
            spacing: 0
            text: "Add an agent"
            draw_bg +: {
                color: (RBX_ACCENT)
                color_hover: (RBX_ACCENT_HOVER)
                color_down: (RBX_ACCENT_PRESSED)
                border_radius: (RBX_RADIUS_XXS)
            }
            draw_text +: { color: (RBX_FG_ON_ACCENT), color_hover: (RBX_FG_ON_ACCENT), color_down: (RBX_FG_ON_ACCENT) }
        }

        agent_center_stats := View {
            width: Fill
            height: Fit
            flow: Right
            spacing: 8

            total_agents_stat := AgentStatTile {
                stat_label.text: "Total"
            }
            direct_agents_stat := AgentStatTile {
                stat_label.text: "Direct"
            }
            octos_agents_stat := AgentStatTile {
                stat_label.text: "Octos"
            }
        }

        // Octos AppService summary card (design handoff §1.5): light-blue, a
        // configured Octos count, and an explanatory line. The actual binding
        // controls live in the Add-agent sheet (Octos step).
        appservice_summary_card := RoundedView {
            width: Fill
            height: Fit
            flow: Down
            spacing: 6
            padding: Inset{left: (SPACE_MD), right: (SPACE_MD), top: (SPACE_MD), bottom: (SPACE_MD)}
            new_batch: true
            show_bg: true
            draw_bg +: {
                color: (RBX_INFO_BG)
                border_radius: (RBX_RADIUS_XXS)
            }

            appservice_summary_header := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}

                appservice_summary_title := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (RBX_INFO_FG)
                        text_style: RBX_TEXT_BODY_STRONG {}
                    }
                    text: "Octos AppService"
                }
                appservice_online_pill := RoundedView {
                    width: Fit
                    height: Fit
                    flow: Right
                    align: Align{y: 0.5}
                    spacing: 5
                    padding: Inset{left: 9, right: 9, top: 4, bottom: 4}
                    new_batch: true
                    show_bg: true
                    draw_bg +: {
                        color: (RBX_BG_SURFACE)
                        border_radius: (RBX_RADIUS_XXS)
                    }
                    appservice_online_dot := RoundedView {
                        width: 7
                        height: 7
                        show_bg: true
                        draw_bg +: {
                            color: (RBX_NEUTRAL_FG)
                            border_radius: (RBX_RADIUS_PILL)
                        }
                    }
                    appservice_online_label := Label {
                        width: Fit
                        height: Fit
                        draw_text +: {
                            color: (RBX_FG_SECONDARY)
                            text_style: RBX_TEXT_BADGE {}
                        }
                        text: "0/0 online"
                    }
                }
            }
            appservice_summary_body := Label {
                width: Fill
                height: Fit
                draw_text +: {
                    color: (RBX_FG_SECONDARY)
                    text_style: RBX_TEXT_META {}
                }
                text: "Robrix stays a normal Matrix client. It binds local Octos services and runs the matching slash commands."
            }

            appservice_config_row := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}
                spacing: 8
                margin: Inset{top: 2}

                appservice_config_state_pill := RoundedView {
                    width: Fit
                    height: Fit
                    padding: Inset{left: 9, right: 9, top: 4, bottom: 4}
                    new_batch: true
                    show_bg: true
                    draw_bg +: {
                        color: (RBX_NEUTRAL_BG)
                        border_radius: (RBX_RADIUS_XXS)
                    }
                    appservice_config_state_label := Label {
                        width: Fit
                        height: Fit
                        draw_text +: {
                            color: (RBX_NEUTRAL_FG)
                            text_style: RBX_TEXT_BADGE {}
                        }
                        text: "Disabled"
                    }
                }

                appservice_endpoint_value := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_META {}
                    }
                    text: "Endpoint not configured"
                }
            }

            appservice_actions_row := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}
                spacing: 8
                margin: Inset{top: 6}

                appservice_bind_button := RobrixIconButton {
                    width: Fill
                    height: (RBX_CONTROL_H_MD)
                    padding: Inset{top: 8, bottom: 8, left: 12, right: 12}
                    icon_walk: Walk{width: 0, height: 0}
                    spacing: 0
                    text: "Bind Octos Bot"
                    draw_bg +: {
                        color: (RBX_ACCENT)
                        color_hover: (RBX_ACCENT_HOVER)
                        color_down: (RBX_ACCENT_PRESSED)
                        border_radius: (RBX_RADIUS_XXS)
                    }
                    draw_text +: { color: (RBX_FG_ON_ACCENT), color_hover: (RBX_FG_ON_ACCENT), color_down: (RBX_FG_ON_ACCENT) }
                }
                appservice_edit_button := RobrixIconButton {
                    width: Fill
                    height: (RBX_CONTROL_H_MD)
                    padding: Inset{top: 8, bottom: 8, left: 12, right: 12}
                    icon_walk: Walk{width: 0, height: 0}
                    spacing: 0
                    text: "Edit AppService"
                    draw_bg +: {
                        color: (RBX_BG_SURFACE)
                        color_hover: (RBX_BG_HOVER)
                        color_down: (RBX_BG_PRESSED)
                        border_radius: (RBX_RADIUS_XXS)
                        border_size: 1.0
                        border_color: (RBX_INFO_FG)
                    }
                    draw_text +: { color: (RBX_INFO_FG), color_hover: (RBX_INFO_FG), color_down: (RBX_INFO_FG) }
                }
            }
        }

        registry_card := RoundedView {
            width: Fill
            height: Fit
            flow: Down
            spacing: (SPACE_XS)
            padding: Inset{left: (SPACE_MD), right: (SPACE_MD), top: (SPACE_SM), bottom: (SPACE_SM)}
            new_batch: true
            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_XXS)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
            }

            registry_header := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}

                registry_title := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: RBX_TEXT_CARD_TITLE {}
                    }
                    text: "Registered agents"
                }
                registry_source_badge := RoundedView {
                    width: Fit
                    height: Fit
                    padding: Inset{left: 9, right: 9, top: 4, bottom: 4}
                    new_batch: true
                    show_bg: true
                    draw_bg +: {
                        color: (RBX_SUCCESS_BG)
                        border_radius: (RBX_RADIUS_XXS)
                    }
                    registry_source_label := Label {
                        width: Fit
                        height: Fit
                        draw_text +: {
                            color: (RBX_SUCCESS_FG)
                            text_style: RBX_TEXT_BADGE {}
                        }
                        text: "AgentRegistry"
                    }
                }
            }

            agents_empty_state := RoundedView {
                width: Fill
                height: Fit
                margin: Inset{top: 8, bottom: 2}
                padding: Inset{left: 12, right: 12, top: 12, bottom: 12}
                new_batch: true
                show_bg: true
                draw_bg +: {
                    color: (RBX_BG_SURFACE_SUBTLE)
                    border_radius: (RBX_RADIUS_XXS)
                    border_size: 1.0
                    border_color: (RBX_STROKE_STRONG)
                }
                empty_inner := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_TERTIARY)
                        text_style: RBX_TEXT_META {}
                    }
                    text: "No agents yet. Add one to bind its Matrix account."
                }
            }

            agents_list := FlatList {
                width: Fill
                height: Fit
                flow: Down
                spacing: 0
                grab_key_focus: false
                scroll_bars +: { show_scroll_x: false, show_scroll_y: true }

                agent_item := mod.widgets.AgentRegistryRow {}
            }
        }
    }
}

/// Actions emitted by [`AgentSettings`] for the hosting settings screen.
#[derive(Clone, Debug)]
pub enum AgentSettingsAction {
    /// The user tapped "Add an agent" — the host should open the add-agent modal.
    OpenAddAgent,
    /// The user wants to configure or bind Octos from the AppService summary.
    OpenOctosSetup,
}

/// Emitted by an [`AgentRegistryRow`] when one of its row buttons is clicked.
/// The parent [`AgentSettings`] listens and runs the open-chat / re-check /
/// unbind logic using the carried `user_id`. Mirrors `DeviceRowAction`.
#[derive(Clone, Debug, Default)]
pub enum AgentRowAction {
    #[default]
    None,
    /// "Open chat" — open or create a DM with this agent.
    OpenChat(OwnedUserId),
    /// "Re-check" — re-probe the Octos AppService health (Octos rows only).
    Recheck(OwnedUserId),
    /// "Unbind" — remove this agent from the registry.
    Unbind(OwnedUserId),
}

// ─────────────────────────── AgentRegistryRow ────────────────────────────

#[derive(Script, ScriptHook, Widget)]
pub struct AgentRegistryRow {
    #[deref]
    view: View,
    /// Set on every redraw from the parent's scope props; resolves row clicks.
    #[rust]
    user_id: Option<OwnedUserId>,
}

impl Widget for AgentRegistryRow {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        if let Event::Actions(actions) = event {
            let Some(user_id) = self.user_id.clone() else { return };
            if self
                .view
                .button(cx, ids!(agent_actions_row.agent_open_chat_button))
                .clicked(actions)
            {
                cx.action(AgentRowAction::OpenChat(user_id.clone()));
            }
            if self
                .view
                .button(cx, ids!(agent_actions_row.agent_recheck_button))
                .clicked(actions)
            {
                cx.action(AgentRowAction::Recheck(user_id.clone()));
            }
            if self
                .view
                .button(cx, ids!(agent_actions_row.agent_unbind_button))
                .clicked(actions)
            {
                cx.action(AgentRowAction::Unbind(user_id));
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if let Some(props) = scope.props.get::<AgentRowProps>() {
            self.user_id = Some(props.user_id.clone());
            let framework = props.framework;

            self.view
                .label(cx, ids!(agent_top_row.agent_text_col.agent_name_row.agent_name_label))
                .set_text(cx, &props.display_name);
            self.view
                .label(cx, ids!(agent_top_row.agent_text_col.agent_mxid_label))
                .set_text(cx, props.user_id.as_str());
            self.view
                .label(cx, ids!(agent_top_row.agent_tile.agent_tile_mono))
                .set_text(cx, framework_mono(framework));
            self.view
                .label(cx, ids!(agent_top_row.agent_framework_badge.agent_framework_label))
                .set_text(cx, framework_label(framework));

            self.apply_framework_colors(cx, framework);

            // Health dot: Octos rows only, colored by the probed health status.
            let mut dot = self.view.view(cx, ids!(
                agent_top_row.agent_text_col.agent_name_row.agent_health_dot
            ));
            dot.set_visible(cx, framework == AgentFramework::Octos);
            if framework == AgentFramework::Octos {
                match props.health.unwrap_or(OctosHealthStatus::Unknown) {
                    OctosHealthStatus::Checking => {
                        script_apply_eval!(cx, dot, { draw_bg +: { color: mod.widgets.RBX_WARNING_FG } });
                    }
                    OctosHealthStatus::Reachable => {
                        script_apply_eval!(cx, dot, { draw_bg +: { color: mod.widgets.RBX_SUCCESS_FG } });
                    }
                    OctosHealthStatus::Unreachable => {
                        script_apply_eval!(cx, dot, { draw_bg +: { color: mod.widgets.RBX_DANGER_FG } });
                    }
                    OctosHealthStatus::Unknown => {
                        script_apply_eval!(cx, dot, { draw_bg +: { color: mod.widgets.RBX_NEUTRAL_FG } });
                    }
                }
            }

            // Re-check button: Octos rows only. While a probe is in flight the
            // button reads "Checking..." and is disabled, matching the old
            // imperative `sync_recheck_buttons` behavior.
            let recheck = self.view.button(cx, ids!(agent_actions_row.agent_recheck_button));
            recheck.set_visible(cx, props.shows_recheck);
            if props.shows_recheck {
                let checking = props.health == Some(OctosHealthStatus::Checking);
                recheck.set_enabled(cx, !checking);
                recheck.set_text(cx, if checking { "Checking..." } else { "Re-check" });
            }
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

impl AgentRegistryRow {
    /// Colors this row's framework tile + badge by framework (literal tokens per
    /// branch, since `script_apply_eval!` cannot take runtime token values).
    fn apply_framework_colors(&mut self, cx: &mut Cx, framework: AgentFramework) {
        let mut tile = self.view.view(cx, ids!(agent_top_row.agent_tile));
        let mut mono = self.view.label(cx, ids!(agent_top_row.agent_tile.agent_tile_mono));
        let mut badge = self.view.view(cx, ids!(agent_top_row.agent_framework_badge));
        let mut label = self.view.label(cx, ids!(agent_top_row.agent_framework_badge.agent_framework_label));
        match framework {
            AgentFramework::Octos => {
                script_apply_eval!(cx, tile, { draw_bg +: { color: mod.widgets.RBX_FW_OCTOS_BG } });
                script_apply_eval!(cx, mono, { draw_text +: { color: mod.widgets.RBX_FW_OCTOS_FG } });
                script_apply_eval!(cx, badge, { draw_bg +: { color: mod.widgets.RBX_FW_OCTOS_BG } });
                script_apply_eval!(cx, label, { draw_text +: { color: mod.widgets.RBX_FW_OCTOS_FG } });
            }
            AgentFramework::Hermes => {
                script_apply_eval!(cx, tile, { draw_bg +: { color: mod.widgets.RBX_FW_HERMES_BG } });
                script_apply_eval!(cx, mono, { draw_text +: { color: mod.widgets.RBX_FW_HERMES_FG } });
                script_apply_eval!(cx, badge, { draw_bg +: { color: mod.widgets.RBX_FW_HERMES_BG } });
                script_apply_eval!(cx, label, { draw_text +: { color: mod.widgets.RBX_FW_HERMES_FG } });
            }
            AgentFramework::OpenClaw => {
                script_apply_eval!(cx, tile, { draw_bg +: { color: mod.widgets.RBX_FW_OPENCLAW_BG } });
                script_apply_eval!(cx, mono, { draw_text +: { color: mod.widgets.RBX_FW_OPENCLAW_FG } });
                script_apply_eval!(cx, badge, { draw_bg +: { color: mod.widgets.RBX_FW_OPENCLAW_BG } });
                script_apply_eval!(cx, label, { draw_text +: { color: mod.widgets.RBX_FW_OPENCLAW_FG } });
            }
            AgentFramework::Unknown => {
                script_apply_eval!(cx, tile, { draw_bg +: { color: mod.widgets.RBX_BG_SURFACE_SUBTLE } });
                script_apply_eval!(cx, mono, { draw_text +: { color: mod.widgets.RBX_FG_SECONDARY } });
                script_apply_eval!(cx, badge, { draw_bg +: { color: mod.widgets.RBX_NEUTRAL_BG } });
                script_apply_eval!(cx, label, { draw_text +: { color: mod.widgets.RBX_NEUTRAL_FG } });
            }
        }
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct AgentSettings {
    #[deref]
    view: View,
    #[rust]
    app_language: AppLanguage,
    /// The agent MXIDs the summary was last synced against; lets us skip the
    /// summary/empty-state recompute when the registry hasn't changed. The
    /// FlatList itself is rebuilt from scope every frame in `draw_walk`.
    #[rust]
    last_synced_agent_ids: Vec<OwnedUserId>,
    #[rust]
    last_synced_appservice_enabled: bool,
    #[rust]
    last_synced_octos_service_url: String,
    #[rust]
    has_synced_agents: bool,
    #[rust]
    octos_health: OctosHealthState,
    #[rust]
    octos_probe_base_url: Option<String>,
}

impl Widget for AgentSettings {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let app_language = scope
            .data
            .get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if self.app_language != app_language {
            self.app_language = app_language;
            self.sync_static_texts(cx);
        }
        if let Event::NetworkResponses(responses) = event {
            for response in responses {
                match response {
                    NetworkResponse::HttpResponse { request_id, response }
                        if *request_id == AGENT_SETTINGS_OCTOS_HEALTH_REQUEST_ID =>
                    {
                        let Some(probe_base_url) = self.octos_probe_base_url.clone() else { continue };
                        if let Some(fallback) = self.octos_health.handle_http_result(&probe_base_url, response.status_code) {
                            self.send_octos_health_request(cx, &fallback);
                        } else if !self.octos_health.in_flight {
                            self.octos_probe_base_url = None;
                        }
                        if let Some(app_state) = scope.data.get::<AppState>() {
                            self.sync_octos_summary_ui(cx, app_state);
                        }
                        // Re-draw so each row picks up the new health status
                        // (dot color + Re-check button state) from props.
                        self.view.redraw(cx);
                    }
                    NetworkResponse::HttpError { request_id, .. }
                        if *request_id == AGENT_SETTINGS_OCTOS_HEALTH_REQUEST_ID =>
                    {
                        let Some(probe_base_url) = self.octos_probe_base_url.clone() else { continue };
                        if let Some(fallback) = self.octos_health.handle_transport_error(&probe_base_url) {
                            self.send_octos_health_request(cx, &fallback);
                        } else if !self.octos_health.in_flight {
                            self.octos_probe_base_url = None;
                        }
                        if let Some(app_state) = scope.data.get::<AppState>() {
                            self.sync_octos_summary_ui(cx, app_state);
                        }
                        self.view.redraw(cx);
                    }
                    _ => {}
                }
            }
        }
        self.sync_agents_from_scope_if_needed(cx, scope);

        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.sync_agents_from_scope_if_needed(cx, scope);

        // Build one props bundle per registered agent from the scope's AppState,
        // folding in this widget's Octos health so each row can render its dot
        // and Re-check state, then drive the FlatList (mirrors DevicesScreen).
        let rows = self.build_agent_rows(scope);
        while let Some(subview) = self.view.draw_walk(cx, scope, walk).step() {
            let flat_list_ref = subview.as_flat_list();
            let Some(mut list) = flat_list_ref.borrow_mut() else {
                continue;
            };
            for (index, props) in rows.iter().enumerate() {
                let item_id = LiveId(index as u64);
                let item = list.item(cx, item_id, id!(agent_item)).unwrap();
                let mut scope = Scope::with_props(props);
                item.draw_all(cx, &mut scope);
            }
        }
        DrawStep::done()
    }
}

impl WidgetMatchEvent for AgentSettings {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, scope: &mut Scope) {
        if self.view.button(cx, ids!(add_agent_button)).clicked(actions) {
            cx.action(AgentSettingsAction::OpenAddAgent);
            return;
        }
        if self.view.button(cx, ids!(appservice_summary_card.appservice_actions_row.appservice_bind_button)).clicked(actions)
            || self.view.button(cx, ids!(appservice_summary_card.appservice_actions_row.appservice_edit_button)).clicked(actions)
        {
            cx.action(AgentSettingsAction::OpenOctosSetup);
            return;
        }

        // Row actions arrive as `AgentRowAction`s emitted by each
        // `AgentRegistryRow`, carrying the row's own MXID (mirrors how
        // DevicesScreen reads `DeviceRowAction`).
        for action in actions {
            match action.downcast_ref::<AgentRowAction>() {
                Some(AgentRowAction::OpenChat(user_id)) => {
                    let display_name = user_id.localpart().to_string();
                    submit_async_request(MatrixRequest::OpenOrCreateDirectMessage {
                        user_profile: UserProfile {
                            user_id: user_id.clone(),
                            username: Some(display_name),
                            avatar_state: AvatarState::Unknown,
                        },
                        allow_create: true,
                        create_encrypted: false,
                    });
                    return;
                }
                Some(AgentRowAction::Recheck(user_id)) => {
                    if let Some(app_state) = scope.data.get::<AppState>() {
                        let framework = app_state
                            .agent_registry
                            .get(user_id)
                            .map(|entry| entry.framework)
                            .unwrap_or_default();
                        if framework == AgentFramework::Octos {
                            self.begin_octos_recheck(cx, app_state);
                        }
                    }
                    return;
                }
                Some(AgentRowAction::Unbind(user_id)) => {
                    if let Some(app_state) = scope.data.get_mut::<AppState>() {
                        app_state.agent_registry.unregister(user_id);
                        if let Some(account_user_id) = current_user_id() {
                            if let Err(e) = persistence::save_app_state(app_state.clone(), account_user_id) {
                                error!("Failed to persist agent registry. Error: {e}");
                            }
                        }
                        self.refresh_agents_list(cx, app_state);
                    }
                    return;
                }
                _ => {}
            }
        }
    }
}

impl AgentSettings {
    /// Builds one [`AgentRowProps`] per registered agent from the scope's
    /// `AppState`, folding in this widget's Octos health so each row can render
    /// its dot + Re-check state. Returns an empty Vec when scope has no AppState.
    fn build_agent_rows(&self, scope: &mut Scope) -> Vec<AgentRowProps> {
        let Some(app_state) = scope.data.get::<AppState>() else {
            return Vec::new();
        };
        app_state
            .agent_registry
            .agent_user_ids()
            .into_iter()
            .map(|user_id| {
                let entry = app_state.agent_registry.get(&user_id);
                let display_name = entry
                    .and_then(|e| e.display_name.clone())
                    .filter(|name| !name.trim().is_empty())
                    .unwrap_or_else(|| user_id.localpart().to_string());
                let framework = entry.map(|e| e.framework).unwrap_or_default();
                let shows_recheck = agent_row_shows_recheck(framework);
                // Only Octos rows carry a health status (the dot they show).
                let health = if framework == AgentFramework::Octos {
                    Some(self.octos_health.status)
                } else {
                    None
                };
                AgentRowProps {
                    user_id,
                    display_name,
                    framework,
                    health,
                    shows_recheck,
                }
            })
            .collect()
    }

    fn sync_agents_from_scope_if_needed(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let (current_ids, appservice_enabled, octos_service_url) = match scope.data.get::<AppState>() {
            Some(app_state) => (
                app_state.agent_registry.agent_user_ids(),
                app_state.bot_settings.enabled,
                app_state.bot_settings.resolved_octos_service_url().to_string(),
            ),
            None => return,
        };
        if self.has_synced_agents
            && self.last_synced_agent_ids == current_ids
            && self.last_synced_appservice_enabled == appservice_enabled
            && self.last_synced_octos_service_url == octos_service_url
        {
            return;
        }
        if let Some(app_state) = scope.data.get::<AppState>() {
            self.refresh_agents_list(cx, app_state);
        }
    }

    /// Recomputes the registry summary + empty-state visibility when the set of
    /// registered agents changes. The per-row content (name, mxid, tile,
    /// framework colors, health dot, Re-check state) is no longer poked here —
    /// each `AgentRegistryRow` renders itself from props in its own `draw_walk`.
    fn refresh_agents_list(&mut self, cx: &mut Cx, app_state: &AppState) {
        let agent_ids = app_state.agent_registry.agent_user_ids();
        self.has_synced_agents = true;
        self.last_synced_agent_ids = agent_ids.clone();
        self.last_synced_appservice_enabled = app_state.bot_settings.enabled;
        self.last_synced_octos_service_url = app_state.bot_settings.resolved_octos_service_url().to_string();

        let any = !agent_ids.is_empty();
        self.view.view(cx, ids!(registry_card.agents_list)).set_visible(cx, any);
        self.view.view(cx, ids!(registry_card.agents_empty_state)).set_visible(cx, !any);
        self.sync_center_summary_ui(cx, app_state);
        self.sync_octos_summary_ui(cx, app_state);
        self.view.redraw(cx);
    }

    fn sync_center_summary_ui(&mut self, cx: &mut Cx, app_state: &AppState) {
        let summary = agent_registry_summary(app_state);
        self.view.label(cx, ids!(agent_center_stats.total_agents_stat.stat_value))
            .set_text(cx, &summary.total.to_string());
        self.view.label(cx, ids!(agent_center_stats.direct_agents_stat.stat_value))
            .set_text(cx, &summary.direct.to_string());
        self.view.label(cx, ids!(agent_center_stats.octos_agents_stat.stat_value))
            .set_text(cx, &summary.octos.to_string());
    }

    fn octos_agent_count(app_state: &AppState) -> usize {
        app_state
            .agent_registry
            .agent_user_ids()
            .iter()
            .filter(|id| {
                app_state.agent_registry.get(id).map(|entry| entry.framework) == Some(AgentFramework::Octos)
            })
            .count()
    }

    fn sync_octos_summary_ui(&mut self, cx: &mut Cx, app_state: &AppState) {
        let octos_count = Self::octos_agent_count(app_state);
        let label = match self.octos_health.status {
            OctosHealthStatus::Checking if octos_count > 0 => "Checking".to_string(),
            OctosHealthStatus::Reachable if app_state.bot_settings.enabled && octos_count > 0 => {
                format!("{octos_count}/{octos_count} online")
            }
            OctosHealthStatus::Unreachable if octos_count > 0 => format!("0/{octos_count} online"),
            _ if app_state.bot_settings.enabled && octos_count == 0 => "No Octos bound".to_string(),
            _ => format!("{octos_count} Octos"),
        };
        self.view.label(cx, ids!(appservice_summary_card.appservice_summary_header.appservice_online_pill.appservice_online_label))
            .set_text(cx, &label);
        let body = if app_state.bot_settings.enabled && octos_count == 0 {
            "AppService URL is saved. Bind an Octos Matrix ID to make it usable in Agent Access."
        } else {
            "Robrix stays a normal Matrix client. It binds local Octos services and runs the matching slash commands."
        };
        self.view.label(cx, ids!(appservice_summary_card.appservice_summary_body))
            .set_text(cx, body);
        self.view.label(cx, ids!(appservice_summary_card.appservice_config_row.appservice_endpoint_value))
            .set_text(cx, app_state.bot_settings.resolved_octos_service_url());

        let config_label = if app_state.bot_settings.enabled { "Enabled" } else { "Disabled" };
        self.view.label(cx, ids!(appservice_summary_card.appservice_config_row.appservice_config_state_pill.appservice_config_state_label))
            .set_text(cx, config_label);
        let bind_label = if !app_state.bot_settings.enabled {
            "Configure Octos"
        } else if octos_count == 0 {
            "Bind Octos Bot"
        } else {
            "Change Octos Bot"
        };
        self.view.button(cx, ids!(appservice_summary_card.appservice_actions_row.appservice_bind_button))
            .set_text(cx, bind_label);
        self.view.button(cx, ids!(appservice_summary_card.appservice_actions_row.appservice_edit_button))
            .set_visible(cx, app_state.bot_settings.enabled);

        let mut dot = self.view.view(cx, ids!(appservice_summary_card.appservice_summary_header.appservice_online_pill.appservice_online_dot));
        let mut config_pill = self.view.view(cx, ids!(appservice_summary_card.appservice_config_row.appservice_config_state_pill));
        let mut config_pill_label = self.view.label(cx, ids!(appservice_summary_card.appservice_config_row.appservice_config_state_pill.appservice_config_state_label));
        if app_state.bot_settings.enabled {
            script_apply_eval!(cx, config_pill, { draw_bg +: { color: mod.widgets.RBX_SUCCESS_BG } });
            script_apply_eval!(cx, config_pill_label, { draw_text +: { color: mod.widgets.RBX_SUCCESS_FG } });
        } else {
            script_apply_eval!(cx, config_pill, { draw_bg +: { color: mod.widgets.RBX_NEUTRAL_BG } });
            script_apply_eval!(cx, config_pill_label, { draw_text +: { color: mod.widgets.RBX_NEUTRAL_FG } });
        }
        match self.octos_health.status {
            OctosHealthStatus::Checking if octos_count > 0 => {
                script_apply_eval!(cx, dot, { draw_bg +: { color: mod.widgets.RBX_ACCENT } });
            }
            OctosHealthStatus::Reachable if app_state.bot_settings.enabled && octos_count > 0 => {
                script_apply_eval!(cx, dot, { draw_bg +: { color: mod.widgets.RBX_SUCCESS_FG } });
            }
            OctosHealthStatus::Unreachable if octos_count > 0 => {
                script_apply_eval!(cx, dot, { draw_bg +: { color: mod.widgets.RBX_DANGER_FG } });
            }
            _ => {
                script_apply_eval!(cx, dot, { draw_bg +: { color: mod.widgets.RBX_NEUTRAL_FG } });
            }
        }
        self.view.redraw(cx);
    }

    fn begin_octos_recheck(&mut self, cx: &mut Cx, app_state: &AppState) {
        let service_url = app_state.bot_settings.resolved_octos_service_url().to_string();
        if BotSettingsState::validate_octos_service_url(&service_url).is_err() {
            self.octos_health = OctosHealthState::default();
            self.octos_health.status = OctosHealthStatus::Unreachable;
            self.octos_probe_base_url = None;
            self.sync_octos_summary_ui(cx, app_state);
            // Re-draw so each Octos row reflects the new health from props.
            self.view.redraw(cx);
            return;
        }

        if let Some(probe) = self.octos_health.begin_check(&service_url) {
            self.octos_probe_base_url = Some(service_url);
            self.sync_octos_summary_ui(cx, app_state);
            self.view.redraw(cx);
            self.send_octos_health_request(cx, &probe);
        }
    }

    fn send_octos_health_request(&self, cx: &mut Cx, url: &str) {
        let req = HttpRequest::new(url.to_string(), HttpMethod::GET);
        cx.http_request(AGENT_SETTINGS_OCTOS_HEALTH_REQUEST_ID, req);
    }

    fn sync_static_texts(&mut self, cx: &mut Cx) {
        self.view.label(cx, ids!(agents_title))
            .set_text(cx, tr_key(self.app_language, "settings.labs.agents.title"));
        self.view.label(cx, ids!(agents_description))
            .set_text(cx, tr_key(self.app_language, "settings.labs.agents.description"));
        self.view.label(cx, ids!(registry_card.agents_empty_state.empty_inner))
            .set_text(cx, tr_key(self.app_language, "settings.labs.agents.empty"));
        self.view.button(cx, ids!(add_agent_button))
            .set_text(cx, tr_key(self.app_language, "settings.labs.agents.add_button"));
        self.view.redraw(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::{agent_registry_summary, agent_row_shows_recheck, framework_label, framework_options, parse_agent_user_id, register_agent_from_search};
    use crate::app::{AgentEntry, AgentFramework, AppState};
    use matrix_sdk::ruma::OwnedUserId;

    fn production_src(src: &'static str) -> &'static str {
        src.split("#[cfg(test)]").next().unwrap_or(src)
    }

    #[test]
    fn test_register_searched_agent_octos() {
        let mut app_state = AppState::default();
        let id: OwnedUserId = "@svc:example.org".try_into().unwrap();

        let added = register_agent_from_search(
            &mut app_state,
            id.clone(),
            Some("Svc".to_string()),
            AgentFramework::Octos,
        );

        assert!(added);
        assert!(app_state.agent_registry.contains(id.as_ref()));
        let entry = app_state.agent_registry.get(id.as_ref()).unwrap();
        assert_eq!(entry.framework, AgentFramework::Octos);
        assert_eq!(entry.display_name.as_deref(), Some("Svc"));
    }

    #[test]
    fn test_register_searched_agent_hermes() {
        let mut app_state = AppState::default();
        let id: OwnedUserId = "@helper:example.org".try_into().unwrap();

        register_agent_from_search(&mut app_state, id.clone(), None, AgentFramework::Hermes);

        assert!(app_state.agent_registry.contains(id.as_ref()));
        assert_eq!(
            app_state.agent_registry.get(id.as_ref()).unwrap().framework,
            AgentFramework::Hermes,
        );
    }

    #[test]
    fn test_parse_agent_user_id_accepts_full_mxid() {
        let parsed = parse_agent_user_id("@helper:example.org").unwrap();

        assert_eq!(parsed.as_str(), "@helper:example.org");
    }

    #[test]
    fn test_parse_agent_user_id_accepts_handoff_field_without_at_prefix() {
        let parsed = parse_agent_user_id("helper:example.org").unwrap();

        assert_eq!(parsed.as_str(), "@helper:example.org");
    }

    #[test]
    fn test_parse_agent_user_id_rejects_localpart() {
        let error = parse_agent_user_id("helper").unwrap_err();

        assert!(error.contains("Matrix user ID"));
    }

    #[test]
    fn test_register_searched_agent_idempotent() {
        let mut app_state = AppState::default();
        let id: OwnedUserId = "@svc:example.org".try_into().unwrap();
        assert!(register_agent_from_search(&mut app_state, id.clone(), Some("Svc".into()), AgentFramework::Octos));

        let added_again = register_agent_from_search(&mut app_state, id.clone(), Some("Changed".into()), AgentFramework::Hermes);

        assert!(!added_again);
        assert_eq!(app_state.agent_registry.len(), 1);
        // First entry preserved (framework not overwritten).
        assert_eq!(
            app_state.agent_registry.get(id.as_ref()).unwrap().framework,
            AgentFramework::Octos,
        );
    }

    #[test]
    fn test_register_agent_preserves_app_service_state() {
        let mut app_state = AppState::default();
        app_state.bot_settings.record_known_bot_user_ids(vec![
            "@botA:example.org".try_into().unwrap(),
        ]);
        let before = app_state.bot_settings.known_bot_user_ids();

        register_agent_from_search(
            &mut app_state,
            "@svc:example.org".try_into().unwrap(),
            None,
            AgentFramework::Octos,
        );

        let after = app_state.bot_settings.known_bot_user_ids();
        assert_eq!(after, before);
        assert!(after.iter().any(|id| id.as_str() == "@botA:example.org"));
    }

    #[test]
    fn test_framework_options_include_octos_hermes_openclaw() {
        let options = framework_options();
        assert!(options.contains(&AgentFramework::Octos));
        assert!(options.contains(&AgentFramework::Hermes));
        assert!(options.contains(&AgentFramework::OpenClaw));
    }

    #[test]
    fn test_recheck_action_is_octos_only() {
        assert!(agent_row_shows_recheck(AgentFramework::Octos));
        assert!(!agent_row_shows_recheck(AgentFramework::Hermes));
        assert!(!agent_row_shows_recheck(AgentFramework::OpenClaw));
        assert!(!agent_row_shows_recheck(AgentFramework::Unknown));
    }

    #[test]
    fn test_agent_registry_summary_counts_frameworks() {
        let mut app_state = AppState::default();
        register_agent_from_search(
            &mut app_state,
            "@octos:example.org".try_into().unwrap(),
            None,
            AgentFramework::Octos,
        );
        register_agent_from_search(
            &mut app_state,
            "@hermes:example.org".try_into().unwrap(),
            None,
            AgentFramework::Hermes,
        );
        register_agent_from_search(
            &mut app_state,
            "@openclaw:example.org".try_into().unwrap(),
            None,
            AgentFramework::OpenClaw,
        );
        app_state.agent_registry.register(
            "@legacy:example.org".try_into().unwrap(),
            AgentEntry {
                framework: AgentFramework::Unknown,
                ..Default::default()
            },
        );

        let summary = agent_registry_summary(&app_state);

        assert_eq!(summary.total, 4);
        assert_eq!(summary.octos, 1);
        assert_eq!(summary.direct, 2);
        assert_eq!(summary.unknown, 1);
    }

    #[test]
    fn test_add_modal_offers_three_framework_cards() {
        // The framework selector lives in the add-agent modal as selectable cards.
        let src = include_str!("agent_add_modal.rs");
        assert!(src.contains("octos_card"));
        assert!(src.contains("hermes_card"));
        assert!(src.contains("openclaw_card"));
    }

    #[test]
    fn test_add_modal_exposes_add_friend_binding() {
        // "Add friend & bind" + real DM request live in the add-agent modal.
        let src = include_str!("agent_add_modal.rs");
        assert!(src.contains("Add friend & bind"));
        assert!(src.contains("OpenOrCreateDirectMessage"));
    }

    #[test]
    fn test_main_screen_has_add_button_and_actions() {
        // Agent Access main screen: add button + row actions, no inline search.
        // Octos rows keep an explicit AppService re-check action from the handoff.
        let src = production_src(include_str!("agent_settings.rs"));
        assert!(src.contains("add_agent_button"));
        assert!(src.contains("agent_open_chat_button"));
        assert!(src.contains("agent_recheck_button"));
        assert!(src.contains("agent_unbind_button"));
    }

    #[test]
    fn test_main_screen_has_configuration_center_summary() {
        let src = production_src(include_str!("agent_settings.rs"));

        assert!(src.contains("agent_center_stats := View"));
        assert!(src.contains("total_agents_stat"));
        assert!(src.contains("direct_agents_stat"));
        assert!(src.contains("octos_agents_stat"));
        assert!(src.contains("appservice_endpoint_value"));
        assert!(src.contains("appservice_config_state_label"));
    }

    #[test]
    fn test_appservice_summary_explains_saved_url_still_needs_bound_octos_agent() {
        let src = production_src(include_str!("agent_settings.rs"));

        assert!(
            src.contains("AppService URL is saved. Bind an Octos Matrix ID to make it usable in Agent Access."),
            "Agent Access should not imply that saving the AppService URL alone creates a usable Octos agent",
        );
    }

    #[test]
    fn test_agent_access_summary_refreshes_when_only_appservice_config_changes() {
        let src = production_src(include_str!("agent_settings.rs"));

        assert!(src.contains("last_synced_appservice_enabled"));
        assert!(src.contains("last_synced_octos_service_url"));
        assert!(
            src.contains("&& self.last_synced_appservice_enabled == appservice_enabled")
                && src.contains("&& self.last_synced_octos_service_url == octos_service_url"),
            "Agent Access summary refresh must include AppService config changes, not only AgentRegistry IDs",
        );
    }

    #[test]
    fn test_appservice_summary_has_direct_octos_setup_actions() {
        let src = production_src(include_str!("agent_settings.rs"));

        assert!(src.contains("appservice_bind_button"));
        assert!(src.contains("appservice_edit_button"));
        assert!(src.contains("Bind Octos Bot"));
        assert!(src.contains("Edit AppService"));
        assert!(src.contains("OpenOctosSetup"));
    }

    #[test]
    fn test_appservice_summary_buttons_open_octos_setup() {
        let src = production_src(include_str!("agent_settings.rs"));

        assert!(
            src.contains("self.view.button(cx, ids!(appservice_summary_card.appservice_actions_row.appservice_bind_button)).clicked(actions)")
                && src.contains("self.view.button(cx, ids!(appservice_summary_card.appservice_actions_row.appservice_edit_button)).clicked(actions)")
                && src.contains("cx.action(AgentSettingsAction::OpenOctosSetup);"),
            "Octos summary actions should open the Octos setup flow directly instead of the generic framework picker",
        );
    }

    #[test]
    fn test_agent_registry_colored_text_surfaces_are_batched() {
        let src = include_str!("agent_settings.rs");

        for block_name in [
            "mod.widgets.AgentRegistryRow = #(AgentRegistryRow::register_widget(vm)) {",
            "agent_tile := RoundedView {",
            "agent_framework_badge := RoundedView {",
            "appservice_summary_card := RoundedView {",
            "appservice_online_pill := RoundedView {",
            "registry_card := RoundedView {",
            "registry_source_badge := RoundedView {",
        ] {
            let block_start = src.find(block_name).expect("expected registry UI block");
            let next_draw = src[block_start..]
                .find("draw_bg +:")
                .expect("expected colored surface");
            let block_prefix = &src[block_start..block_start + next_draw];
            assert!(
                block_prefix.contains("new_batch: true"),
                "{block_name} should start a new draw batch before drawing a colored text surface",
            );
        }
    }

    #[test]
    fn test_agent_registry_rows_match_design_handoff_structure() {
        let src = production_src(include_str!("agent_settings.rs"));

        assert!(
            src.contains("agent_tile := RoundedView {\n                width: 30\n                height: 30"),
            "framework tile should match the compacted 30px tile"
        );
        assert!(
            src.contains("agent_actions_row := View"),
            "registered-agent rows should carry a row-actions section"
        );
        assert!(
            src.contains("agents_empty_state := RoundedView"),
            "empty state should render as a bounded handoff-style state, not bare text"
        );
    }

    #[test]
    fn test_registry_list_is_dynamic_flat_list() {
        // The registry list is a dynamic FlatList driven from AppState's
        // AgentRegistry (mirrors the Devices screen), not a fixed set of rows.
        let src = production_src(include_str!("agent_settings.rs"));
        assert!(
            src.contains("agents_list := FlatList {"),
            "registered agents should render in a dynamic FlatList",
        );
        assert!(
            src.contains("agent_item := mod.widgets.AgentRegistryRow {}"),
            "the FlatList should template a single AgentRegistryRow item",
        );
        assert!(
            src.contains("mod.widgets.AgentRegistryRow = #(AgentRegistryRow::register_widget(vm))"),
            "AgentRegistryRow should be a registered widget so the list can drive it",
        );
        // The old fixed-row scaffolding must be gone.
        assert!(
            !src.contains("AGENT_ROW_IDS"),
            "fixed-row id table should be removed after the FlatList conversion",
        );
        assert!(
            !src.contains("agent_row_0 := AgentRegistryRow"),
            "the 30 fixed AgentRegistryRow instances should be removed",
        );
    }

    #[test]
    fn test_octos_registry_rows_show_health_dot_next_to_name() {
        let src = production_src(include_str!("agent_settings.rs"));

        assert!(src.contains("agent_name_row := View"));
        assert!(src.contains("agent_health_dot := RoundedView"));
        // Health-dot logic now lives in AgentRegistryRow's own draw_walk,
        // driven by the per-row health folded into AgentRowProps.
        assert!(src.contains("props.health"));
        assert!(src.contains("dot.set_visible(cx, framework == AgentFramework::Octos)"));
        assert!(src.contains("OctosHealthStatus::Reachable"));
        assert!(src.contains("mod.widgets.RBX_SUCCESS_FG"));
    }

    #[test]
    fn test_framework_label_maps_each_variant() {
        let labels = [
            framework_label(AgentFramework::Octos),
            framework_label(AgentFramework::Hermes),
            framework_label(AgentFramework::OpenClaw),
            framework_label(AgentFramework::Unknown),
        ];
        for i in 0..labels.len() {
            for j in (i + 1)..labels.len() {
                assert_ne!(labels[i], labels[j]);
            }
        }
        assert!(!framework_label(AgentFramework::Unknown).is_empty());
    }

    #[test]
    fn test_labs_embeds_agent_settings() {
        // `main`'s Settings screen has a single Labs layout (not the separate
        // Mobile/Desktop variants the original branch used), so one embed is
        // expected here.
        let src = production_src(include_str!("settings_screen.rs"));
        assert!(src.contains("AgentSettings"), "settings_screen.rs must reference AgentSettings");
        let embeds = src.matches("agent_settings := AgentSettings").count();
        assert!(embeds >= 1, "expected agent_settings embedded in the Labs page, found {embeds}");
    }

    #[test]
    fn test_labs_no_longer_has_standalone_app_service() {
        // AppService config is consolidated into the Octos "Add an agent" flow,
        // so the standalone BotSettings widget is no longer embedded in Labs.
        let src = production_src(include_str!("settings_screen.rs"));
        assert!(
            !src.contains("bot_settings := BotSettings"),
            "standalone App Service (BotSettings) should be consolidated into the Octos agent flow, not embedded in Labs",
        );
        assert!(
            !src.contains(".bot_settings(cx"),
            "SettingsScreen should not keep dead BotSettings widget refs after removing the standalone App Service card",
        );
        assert!(
            !src.contains("BotSettingsWidgetExt"),
            "SettingsScreen should not import the standalone BotSettings widget extension after consolidation",
        );
    }

    #[test]
    fn test_octos_flow_still_drives_appservice_binding() {
        // The slash-command / AppService binding must keep working: the Octos
        // registration path writes the same bot_settings fields the old widget did.
        let src = include_str!("agent_add_modal.rs");
        assert!(src.contains("bot_settings.enabled"), "Octos finish must enable the AppService binding");
        assert!(src.contains("octos_service_url"), "Octos finish must record the service URL for slash commands");
        assert!(src.contains("botfather_user_id"), "Octos finish must record the BotFather id for slash commands");
        assert!(
            !src.contains("botfather_field := AgentField"),
            "BotFather must not be a separate override field; Octos uses the same Matrix ID that was added as a friend",
        );
    }
}
