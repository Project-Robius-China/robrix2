//! Fail-closed Agent Operations integration status.
//!
//! The full panel must not connect to agent-chat's local Dashboard. The router
//! endpoints use a backend-wide credential and return a different wire model.
//! This screen remains intentionally inert until agent-chat's canonical client
//! contract is released and verified, and a separate client runtime is
//! implemented. The local R3 model experiment is test-only and cannot open the
//! production gate.

use makepad_widgets::*;

use crate::{
    app::AppState,
    agent_ops::state::{AgentOpsAvailability, AgentOpsContractGate},
    i18n::{AppLanguage, tr_fmt, tr_key},
    shared::design_tokens::{
        RBX_DANGER_BG, RBX_DANGER_FG, RBX_SUCCESS_BG, RBX_SUCCESS_FG,
        RBX_WARNING_BG, RBX_WARNING_FG,
    },
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AgentOpsStatusTone {
    Success,
    Warning,
    Danger,
}

impl AgentOpsStatusTone {
    fn colors(self) -> (Vec4, Vec4) {
        match self {
            Self::Success => (RBX_SUCCESS_BG, RBX_SUCCESS_FG),
            Self::Warning => (RBX_WARNING_BG, RBX_WARNING_FG),
            Self::Danger => (RBX_DANGER_BG, RBX_DANGER_FG),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AgentOpsPresentation {
    status_key: &'static str,
    next_step_key: &'static str,
    tone: AgentOpsStatusTone,
}

impl AgentOpsPresentation {
    fn for_availability(availability: &AgentOpsAvailability) -> Self {
        let (status_key, tone) = match availability {
            AgentOpsAvailability::NoContractManifest => (
                "agent_ops.status.no_manifest",
                AgentOpsStatusTone::Warning,
            ),
            AgentOpsAvailability::NotReleased { .. } => (
                "agent_ops.status.not_released",
                AgentOpsStatusTone::Warning,
            ),
            AgentOpsAvailability::UnboundSourceCommit => (
                "agent_ops.status.unbound_commit",
                AgentOpsStatusTone::Warning,
            ),
            AgentOpsAvailability::UnreadableManifest => (
                "agent_ops.status.unreadable_manifest",
                AgentOpsStatusTone::Danger,
            ),
            AgentOpsAvailability::ContractMismatch { .. } => (
                "agent_ops.status.contract_mismatch",
                AgentOpsStatusTone::Danger,
            ),
            AgentOpsAvailability::InvalidSourceCommit => (
                "agent_ops.status.invalid_source_commit",
                AgentOpsStatusTone::Danger,
            ),
            AgentOpsAvailability::EmptyArtifactSet => (
                "agent_ops.status.empty_artifacts",
                AgentOpsStatusTone::Danger,
            ),
            AgentOpsAvailability::InvalidArtifactManifest => (
                "agent_ops.status.invalid_artifact_manifest",
                AgentOpsStatusTone::Danger,
            ),
            AgentOpsAvailability::IncompleteArtifactManifest { .. } => (
                "agent_ops.status.incomplete_artifact_manifest",
                AgentOpsStatusTone::Danger,
            ),
            AgentOpsAvailability::ArtifactSetMismatch { .. } => (
                "agent_ops.status.artifact_set_mismatch",
                AgentOpsStatusTone::Danger,
            ),
            AgentOpsAvailability::ArtifactDigestMismatch { .. } => (
                "agent_ops.status.artifact_digest_mismatch",
                AgentOpsStatusTone::Danger,
            ),
            AgentOpsAvailability::ContractReady { .. } => (
                "agent_ops.status.contract_ready",
                AgentOpsStatusTone::Success,
            ),
        };
        let next_step_key = if availability.is_contract_ready() {
            "agent_ops.status.next_step_runtime"
        } else {
            "agent_ops.status.next_step_release"
        };
        Self { status_key, next_step_key, tone }
    }
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.AgentOpsPanel = #(AgentOpsPanel::register_widget(vm)) {
        ..mod.widgets.View
        width: Fill, height: Fill
        flow: Down
        padding: Inset{top: 10, left: 12, right: 12, bottom: 12}
        spacing: (SPACE_MD)
        show_bg: true
        draw_bg +: { color: (RBX_BG_SURFACE) }

        title := Label {
            width: Fill, height: Fit
            margin: Inset{top: 4, bottom: 4, left: 4}
            draw_text +: {
                color: (RBX_FG_PRIMARY)
                text_style: (RBX_TEXT_PAGE_TITLE)
            }
            text: ""
        }

        contract_card := RoundedView {
            width: Fill, height: Fit
            flow: Down
            spacing: (SPACE_SM)
            padding: Inset{
                top: (SPACE_MD), right: (SPACE_MD),
                bottom: (SPACE_MD), left: (SPACE_MD)
            }
            show_bg: true
            draw_bg +: {
                color: (RBX_WARNING_BG)
                border_radius: (RBX_RADIUS_SM)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
            }

            contract_status := Label {
                width: Fill, height: Fit
                flow: Flow.Right{wrap: true}
                draw_text +: {
                    color: (RBX_WARNING_FG)
                    text_style: (RBX_TEXT_BODY_STRONG)
                }
                text: ""
            }

            contract_next_step := Label {
                width: Fill, height: Fit
                flow: Flow.Right{wrap: true}
                draw_text +: {
                    color: (RBX_FG_SECONDARY)
                    text_style: (RBX_TEXT_BODY)
                }
                text: ""
            }
        }
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct AgentOpsPanel {
    #[deref]
    view: View,
    #[rust(AgentOpsContractGate::from_vendored_manifest())]
    contract_gate: AgentOpsContractGate,
    #[rust]
    app_language: AppLanguage,
    #[rust(false)]
    app_language_initialized: bool,
}

impl Widget for AgentOpsPanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.sync_language(cx, scope);
        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.sync_language(cx, scope);
        self.view.draw_walk(cx, scope, walk)
    }
}

impl AgentOpsPanel {
    fn sync_language(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let app_language = scope
            .data
            .get::<AppState>()
            .map(|state| state.app_language)
            .unwrap_or_default();
        if self.app_language_initialized && self.app_language == app_language {
            return;
        }

        self.app_language = app_language;
        self.app_language_initialized = true;
        self.view
            .label(cx, ids!(title))
            .set_text(cx, tr_key(app_language, "agent_ops.title"));

        let availability = self.contract_gate.availability();
        let presentation = AgentOpsPresentation::for_availability(availability);
        let diagnostic = match availability {
            AgentOpsAvailability::NoContractManifest => {
                tr_key(app_language, "agent_ops.detail.no_manifest").to_string()
            }
            AgentOpsAvailability::UnreadableManifest => {
                tr_key(app_language, "agent_ops.detail.unreadable_manifest").to_string()
            }
            AgentOpsAvailability::ContractMismatch { expected, found } => tr_fmt(
                app_language,
                "agent_ops.detail.contract_mismatch",
                &[("expected", expected), ("found", found)],
            ),
            AgentOpsAvailability::NotReleased { release_status } => tr_fmt(
                app_language,
                "agent_ops.detail.not_released",
                &[("release_status", release_status)],
            ),
            AgentOpsAvailability::UnboundSourceCommit => {
                tr_key(app_language, "agent_ops.detail.unbound_commit").to_string()
            }
            AgentOpsAvailability::InvalidSourceCommit => {
                tr_key(app_language, "agent_ops.detail.invalid_source_commit").to_string()
            }
            AgentOpsAvailability::EmptyArtifactSet => {
                tr_key(app_language, "agent_ops.detail.empty_artifacts").to_string()
            }
            AgentOpsAvailability::InvalidArtifactManifest => {
                tr_key(app_language, "agent_ops.detail.invalid_artifact_manifest").to_string()
            }
            AgentOpsAvailability::IncompleteArtifactManifest { missing_count } => {
                let missing_count = missing_count.to_string();
                tr_fmt(
                    app_language,
                    "agent_ops.detail.incomplete_artifact_manifest",
                    &[("missing_count", &missing_count)],
                )
            }
            AgentOpsAvailability::ArtifactSetMismatch {
                missing_count,
                unexpected_count,
            } => {
                let missing_count = missing_count.to_string();
                let unexpected_count = unexpected_count.to_string();
                tr_fmt(
                    app_language,
                    "agent_ops.detail.artifact_set_mismatch",
                    &[
                        ("missing_count", &missing_count),
                        ("unexpected_count", &unexpected_count),
                    ],
                )
            }
            AgentOpsAvailability::ArtifactDigestMismatch { path } => tr_fmt(
                app_language,
                "agent_ops.detail.artifact_digest_mismatch",
                &[("path", path)],
            ),
            AgentOpsAvailability::ContractReady { source_commit, artifact_count } => {
                let artifact_count = artifact_count.to_string();
                tr_fmt(
                    app_language,
                    "agent_ops.detail.contract_ready",
                    &[
                        ("source_commit", source_commit),
                        ("artifact_count", &artifact_count),
                    ],
                )
            }
        };
        let (background_color, foreground_color) = presentation.tone.colors();
        let mut contract_card = self.view.view(cx, ids!(contract_card));
        script_apply_eval!(cx, contract_card, {
            draw_bg +: { color: #(background_color) }
        });
        let mut contract_status = self.view.label(cx, ids!(contract_card.contract_status));
        script_apply_eval!(cx, contract_status, {
            draw_text +: { color: #(foreground_color) }
        });
        contract_status.set_text(cx, tr_key(app_language, presentation.status_key));

        let detail = format!(
            "{}\n\n{}",
            diagnostic,
            tr_key(app_language, presentation.next_step_key),
        );
        self.view
            .label(cx, ids!(contract_card.contract_next_step))
            .set_text(cx, &detail);
        self.redraw(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::AppState;

    #[test]
    fn panel_contains_no_backend_transport_or_credentials() {
        for (path, source) in [
            ("agent_ops/mod.rs", include_str!("mod.rs")),
            ("agent_ops/panel.rs", include_str!("panel.rs")),
            ("agent_ops/state.rs", include_str!("state.rs")),
            ("home/home_screen.rs", include_str!("../home/home_screen.rs")),
            ("home/navigation_tab_bar.rs", include_str!("../home/navigation_tab_bar.rs")),
            ("settings/app_settings.rs", include_str!("../settings/app_settings.rs")),
            ("lib.rs", include_str!("../lib.rs")),
        ] {
            let production = source.split("#[cfg(test)]").next().unwrap_or(source);
            for forbidden in [
                "HttpRequest",
                "http_request",
                "Authorization",
                "Bearer ",
                "/api/router/",
                "submit_async_request",
            ] {
                assert!(
                    !production.contains(forbidden),
                    "contract-gated source {path} must not contain {forbidden}",
                );
            }
        }
    }

    #[test]
    fn proposal_model_is_not_wired_into_runtime_panel() {
        let agent_ops_module = include_str!("mod.rs");
        assert!(
            agent_ops_module.contains("#[cfg(test)]\nmod model;"),
            "the non-canonical proposal model must compile only in tests",
        );
        assert!(
            !agent_ops_module.contains("pub mod model;"),
            "the non-canonical proposal model must not be a production API",
        );

        let panel = include_str!("panel.rs");
        let production = panel.split("#[cfg(test)]").next().unwrap_or(panel);
        for proposal_type in [
            "agent_ops::model",
            "Snapshot",
            "Invalidation",
            "OutcomeInspection",
            "ResolveOutcomeCommand",
        ] {
            assert!(
                !production.contains(proposal_type),
                "runtime panel must not consume proposal type {proposal_type}",
            );
        }
    }

    #[test]
    fn contract_status_presentation_covers_tone_and_next_step() {
        let source = include_str!("panel.rs");
        let production = source.split("#[cfg(test)]").next().unwrap_or(source);
        assert!(
            production.contains("AgentOpsPresentation::for_availability(availability)"),
            "the runtime panel must derive its text and tone from the tested presentation",
        );

        for (availability, status_key, tone) in [
            (
                AgentOpsAvailability::NoContractManifest,
                "agent_ops.status.no_manifest",
                AgentOpsStatusTone::Warning,
            ),
            (
                AgentOpsAvailability::NotReleased { release_status: "development".into() },
                "agent_ops.status.not_released",
                AgentOpsStatusTone::Warning,
            ),
            (
                AgentOpsAvailability::UnboundSourceCommit,
                "agent_ops.status.unbound_commit",
                AgentOpsStatusTone::Warning,
            ),
            (
                AgentOpsAvailability::UnreadableManifest,
                "agent_ops.status.unreadable_manifest",
                AgentOpsStatusTone::Danger,
            ),
            (
                AgentOpsAvailability::ContractMismatch {
                    expected: "expected".into(),
                    found: "found".into(),
                },
                "agent_ops.status.contract_mismatch",
                AgentOpsStatusTone::Danger,
            ),
            (
                AgentOpsAvailability::InvalidSourceCommit,
                "agent_ops.status.invalid_source_commit",
                AgentOpsStatusTone::Danger,
            ),
            (
                AgentOpsAvailability::EmptyArtifactSet,
                "agent_ops.status.empty_artifacts",
                AgentOpsStatusTone::Danger,
            ),
            (
                AgentOpsAvailability::InvalidArtifactManifest,
                "agent_ops.status.invalid_artifact_manifest",
                AgentOpsStatusTone::Danger,
            ),
            (
                AgentOpsAvailability::IncompleteArtifactManifest { missing_count: 1 },
                "agent_ops.status.incomplete_artifact_manifest",
                AgentOpsStatusTone::Danger,
            ),
            (
                AgentOpsAvailability::ArtifactSetMismatch {
                    missing_count: 1,
                    unexpected_count: 1,
                },
                "agent_ops.status.artifact_set_mismatch",
                AgentOpsStatusTone::Danger,
            ),
            (
                AgentOpsAvailability::ArtifactDigestMismatch { path: "snapshot.json".into() },
                "agent_ops.status.artifact_digest_mismatch",
                AgentOpsStatusTone::Danger,
            ),
        ] {
            assert_eq!(
                AgentOpsPresentation::for_availability(&availability),
                AgentOpsPresentation {
                    status_key,
                    next_step_key: "agent_ops.status.next_step_release",
                    tone,
                },
            );
        }

        let ready = AgentOpsAvailability::ContractReady {
            source_commit: "0123456789abcdef0123456789abcdef01234567".into(),
            artifact_count: 20,
        };
        assert_eq!(
            AgentOpsPresentation::for_availability(&ready),
            AgentOpsPresentation {
                status_key: "agent_ops.status.contract_ready",
                next_step_key: "agent_ops.status.next_step_runtime",
                tone: AgentOpsStatusTone::Success,
            },
        );
    }

    #[test]
    fn settings_contains_no_agent_ops_secret_input() {
        let settings = include_str!("../settings/app_settings.rs");
        let production = settings.split("#[cfg(test)]").next().unwrap_or(settings);
        for forbidden in [
            "agent_ops_token_input",
            "bearer_token",
            "AgentOpsConfig",
        ] {
            assert!(
                !production.contains(forbidden),
                "settings must not collect or persist {forbidden}",
            );
        }

        let stored = serde_json::to_value(AppState::default()).unwrap();
        assert!(
            stored.get("agent_ops").is_none(),
            "AppState must not contain Agent Operations credentials or configuration",
        );
    }

    #[test]
    fn legacy_agent_ops_credentials_are_not_reserialized() {
        let mut stored = serde_json::to_value(AppState::default()).unwrap();
        stored["agent_ops"] = serde_json::json!({
            "base_url": "http://127.0.0.1:8084",
            "bearer_token": "legacy-prototype-secret",
        });

        let restored: AppState = serde_json::from_value(stored).unwrap();
        let rewritten = serde_json::to_value(restored).unwrap();
        assert!(
            rewritten.get("agent_ops").is_none(),
            "obsolete Agent Operations credentials must be dropped on the next state save",
        );
    }
}
