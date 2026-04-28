use makepad_widgets::*;
use url::Url;

use crate::{
    app::{AppState, BotSettingsState},
    i18n::{AppLanguage, tr_fmt, tr_key},
    persistence,
    shared::popup_list::{PopupKind, enqueue_popup_notification},
    sliding_sync::current_user_id,
};

const OCTOS_SERVICE_HEALTH_REQUEST_ID: LiveId = live_id!(octos_service_health);
const MATRIX_GATEWAY_HEALTH_REQUEST_ID: LiveId = live_id!(matrix_gateway_health);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OctosHealthStatus {
    #[default]
    Unknown,
    Checking,
    Reachable,
    Unreachable,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum OctosHealthProbeStage {
    #[default]
    Idle,
    Health,
    ApiStatus,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct OctosHealthState {
    status: OctosHealthStatus,
    probe_stage: OctosHealthProbeStage,
    in_flight: bool,
    current_base_url: Option<String>,
    alternate_base_url: Option<String>,
}

impl OctosHealthState {
    fn begin_check(&mut self, base_url: &str) -> Option<String> {
        if self.in_flight {
            return None;
        }
        let normalized_base_url = normalize_octos_base_url(base_url);
        self.status = OctosHealthStatus::Checking;
        self.probe_stage = OctosHealthProbeStage::Health;
        self.in_flight = true;
        self.current_base_url = Some(normalized_base_url.clone());
        self.alternate_base_url = local_octos_alternate_base_url(&normalized_base_url);
        Some(normalize_octos_probe_url(&normalized_base_url, "/health"))
    }

    fn handle_http_result(&mut self, status_code: u16) -> Option<String> {
        if status_code == 200 {
            self.finish(OctosHealthStatus::Reachable);
            None
        } else {
            self.handle_failure()
        }
    }

    fn handle_transport_error(&mut self) -> Option<String> {
        self.handle_failure()
    }

    fn handle_failure(&mut self) -> Option<String> {
        let Some(current_base_url) = self.current_base_url.clone() else {
            self.finish(OctosHealthStatus::Unreachable);
            return None;
        };
        match self.probe_stage {
            OctosHealthProbeStage::Health => {
                self.probe_stage = OctosHealthProbeStage::ApiStatus;
                Some(normalize_octos_probe_url(&current_base_url, "/api/status"))
            }
            OctosHealthProbeStage::ApiStatus => {
                if let Some(alternate_base_url) = self.alternate_base_url.clone()
                    && alternate_base_url != current_base_url
                {
                    self.current_base_url = Some(alternate_base_url.clone());
                    self.probe_stage = OctosHealthProbeStage::Health;
                    Some(normalize_octos_probe_url(&alternate_base_url, "/health"))
                } else {
                    self.finish(OctosHealthStatus::Unreachable);
                    None
                }
            }
            OctosHealthProbeStage::Idle => {
                self.finish(OctosHealthStatus::Unreachable);
                None
            }
        }
    }

    fn finish(&mut self, status: OctosHealthStatus) {
        self.status = status;
        self.probe_stage = OctosHealthProbeStage::Idle;
        self.in_flight = false;
        self.current_base_url = None;
        self.alternate_base_url = None;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct MatrixGatewayHealthState {
    status: OctosHealthStatus,
    in_flight: bool,
}

impl MatrixGatewayHealthState {
    fn begin_check(&mut self, base_url: &str) -> Option<String> {
        let probe_url = matrix_gateway_probe_url(base_url);
        self.status = if probe_url.is_some() {
            OctosHealthStatus::Checking
        } else {
            OctosHealthStatus::Unreachable
        };
        self.in_flight = probe_url.is_some();
        probe_url
    }

    fn handle_http_result(&mut self, status_code: u16) -> Option<String> {
        if matches!(status_code, 200 | 401 | 405) {
            self.finish(OctosHealthStatus::Reachable);
        } else {
            self.finish(OctosHealthStatus::Unreachable);
        }
        None
    }

    fn handle_transport_error(&mut self) -> Option<String> {
        self.finish(OctosHealthStatus::Unreachable);
        None
    }

    fn finish(&mut self, status: OctosHealthStatus) {
        self.status = status;
        self.in_flight = false;
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct AppServiceHealthState {
    octos_service: OctosHealthState,
    matrix_gateway: MatrixGatewayHealthState,
}

impl AppServiceHealthState {
    fn begin_check(&mut self, base_url: &str) -> Vec<(LiveId, String)> {
        if self.in_flight() {
            return Vec::new();
        }
        let mut requests = Vec::with_capacity(2);
        if let Some(url) = self.octos_service.begin_check(base_url) {
            requests.push((OCTOS_SERVICE_HEALTH_REQUEST_ID, url));
        }
        if let Some(url) = self.matrix_gateway.begin_check(base_url) {
            requests.push((MATRIX_GATEWAY_HEALTH_REQUEST_ID, url));
        }
        requests
    }

    fn in_flight(&self) -> bool {
        self.octos_service.in_flight || self.matrix_gateway.in_flight
    }
}

fn normalize_octos_base_url(base_url: &str) -> String {
    base_url.trim().trim_end_matches('/').to_string()
}

fn local_octos_alternate_base_url(base_url: &str) -> Option<String> {
    let mut parsed_url = Url::parse(base_url).ok()?;
    let host = parsed_url.host_str()?;
    if !matches!(host, "127.0.0.1" | "localhost" | "0.0.0.0") {
        return None;
    }
    let alternate_port = match parsed_url.port_or_known_default()? {
        8010 => 8080,
        8080 => 8010,
        _ => return None,
    };
    parsed_url.set_port(Some(alternate_port)).ok()?;
    Some(normalize_octos_base_url(parsed_url.as_str()))
}

fn normalize_octos_probe_url(base_url: &str, path: &str) -> String {
    format!("{}/{}", normalize_octos_base_url(base_url), path.trim_start_matches('/'))
}

fn matrix_gateway_probe_url(base_url: &str) -> Option<String> {
    let mut parsed_url = Url::parse(&normalize_octos_base_url(base_url)).ok()?;
    parsed_url.set_port(Some(8009)).ok()?;
    parsed_url.set_path("/_matrix/app/v1/transactions/test");
    parsed_url.set_query(None);
    parsed_url.set_fragment(None);
    Some(parsed_url.to_string())
}

fn build_app_service_health_request(request_id: LiveId, url: &str) -> HttpRequest {
    if request_id == MATRIX_GATEWAY_HEALTH_REQUEST_ID {
        let mut req = HttpRequest::new(url.to_string(), HttpMethod::PUT);
        req.set_header("Content-Type".into(), "application/json".into());
        req.set_body(br#"{}"#.to_vec());
        req
    } else {
        HttpRequest::new(url.to_string(), HttpMethod::GET)
    }
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.BotSettingsInfoLabel = Label {
        width: Fill
        height: Fit
        margin: Inset{top: 2, bottom: 2}
        draw_text +: {
            color: (MESSAGE_TEXT_COLOR)
            text_style: REGULAR_TEXT { font_size: 10.5 }
        }
        text: ""
    }

    mod.widgets.BotSettings = #(BotSettings::register_widget(vm)) {
        width: Fill
        height: Fit
        flow: Down
        spacing: (SPACE_SM)

        app_service_header := View {
            width: Fill
            height: Fit
            flow: Down
            spacing: (SPACE_XS)
            margin: Inset{bottom: 2}

            app_service_title := TitleLabel {
                width: Fit
                text: "App Service"
            }

            description := mod.widgets.BotSettingsInfoLabel {
                width: Fill
                margin: 0
                draw_text +: {
                    color: (COLOR_DESCRIPTION_TEXT)
                    text_style: REGULAR_TEXT { font_size: 9.5 }
                }
                text: "Enable Matrix app service support here. Robrix stays a normal Matrix client: it binds BotFather to a room and sends the matching slash commands."
            }
        }

        toggle_row := View {
            width: Fill
            height: Fit
            flow: Right
            align: Align{x: 0.0, y: 0.5}
            spacing: (SPACE_XS)
            padding: Inset{left: 6}
            margin: Inset{bottom: 2}

            app_service_switch := Toggle {
                width: Fit
                height: Fit
                padding: Inset{top: (SPACE_SM), right: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_SM)}
                text: ""
                active: false
                draw_bg +: {
                    size: 20.0
                    color_active: (COLOR_ACTIVE_PRIMARY)
                    border_color_active: (COLOR_ACTIVE_PRIMARY)
                    mark_color_active: #fff
                }
            }

            switch_state_label := Label {
                width: Fit
                height: Fit
                draw_text +: {
                    color: (COLOR_DISABLED_TEXT)
                    text_style: REGULAR_TEXT { font_size: 10.5 }
                }
                text: "Disabled"
            }
        }

        manage_hint := mod.widgets.BotSettingsInfoLabel {
            width: Fill
            padding: Inset{left: 6}
            margin: Inset{top: -2, bottom: 4}
            draw_text +: {
                color: (COLOR_DESCRIPTION_TEXT)
                text_style: REGULAR_TEXT { font_size: 9.5 }
            }
            text: "Manage BotFather and child bots in DM and room bind dialogs. Settings here only control whether App Service features are enabled."
        }

        botfather_section := View {
            width: Fill
            height: Fit
            flow: Down
            spacing: 4
            padding: Inset{left: 6}

            botfather_user_id_label := Label {
                width: Fit
                height: Fit
                draw_text +: {
                    color: (COLOR_FIELD_LABEL)
                    text_style: REGULAR_TEXT { font_size: 10 }
                }
                text: "BotFather User ID:"
            }

            botfather_user_id_input := RobrixTextInput {
                width: Fill
                height: Fit
                padding: 8
                empty_text: "bot or @bot:server"
            }
        }

        octos_health_section := View {
            width: Fill
            height: Fit
            flow: Down
            spacing: 4
            padding: Inset{left: 6}

            octos_service_label := Label {
                width: Fit
                height: Fit
                draw_text +: {
                    color: (COLOR_FIELD_LABEL)
                    text_style: REGULAR_TEXT { font_size: 10 }
                }
                text: "Local Octos Service"
            }

            octos_service_input := RobrixTextInput {
                width: Fill
                height: Fit
                padding: 8
                empty_text: "http://127.0.0.1:8010"
            }

            octos_health_controls := View {
                width: Fill
                height: Fit
                flow: Down
                spacing: 6
                margin: Inset{top: 2}

                octos_health_button_row := View {
                    width: Fill
                    height: Fit
                    flow: Right
                    spacing: (SPACE_SM)
                    align: Align{y: 0.5}

                    save_octos_service_button := RobrixIconButton {
                        padding: Inset{top: 8, bottom: 8, left: 16, right: 16}
                        icon_walk: Walk{width: 0, height: 0}
                        spacing: 0
                        text: "Save"
                    }

                    check_now_button := RobrixNeutralIconButton {
                        padding: Inset{top: 8, bottom: 8, left: 16, right: 16}
                        icon_walk: Walk{width: 0, height: 0}
                        spacing: 0
                        text: "Check Now"
                    }
                }

                octos_health_statuses := View {
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 4

                    octos_service_status_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        spacing: 8
                        align: Align{y: 0.5}

                        octos_service_status_name := Label {
                            width: Fit
                            height: Fit
                            draw_text +: {
                                color: (COLOR_FIELD_LABEL)
                                text_style: REGULAR_TEXT { font_size: 10 }
                            }
                            text: "Octos Service"
                        }

                        octos_service_status_label := Label {
                            width: Fit
                            height: Fit
                            draw_text +: {
                                color: (COLOR_DISABLED_TEXT)
                                text_style: REGULAR_TEXT { font_size: 10.5 }
                            }
                            text: "Unknown"
                        }
                    }

                    matrix_gateway_status_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        spacing: 8
                        align: Align{y: 0.5}

                        matrix_gateway_status_name := Label {
                            width: Fit
                            height: Fit
                            draw_text +: {
                                color: (COLOR_FIELD_LABEL)
                                text_style: REGULAR_TEXT { font_size: 10 }
                            }
                            text: "Matrix Gateway"
                        }

                        matrix_gateway_status_label := Label {
                            width: Fit
                            height: Fit
                            draw_text +: {
                                color: (COLOR_DISABLED_TEXT)
                                text_style: REGULAR_TEXT { font_size: 10.5 }
                            }
                            text: "Unknown"
                        }
                    }
                }
            }
        }
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct BotSettings {
    #[deref]
    view: View,
    #[rust]
    app_language: AppLanguage,
    #[rust]
    app_service_health: AppServiceHealthState,
}

impl Widget for BotSettings {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }

        if let Event::NetworkResponses(responses) = event {
            for response in responses {
                match response {
                    NetworkResponse::HttpResponse { request_id, response }
                        if *request_id == OCTOS_SERVICE_HEALTH_REQUEST_ID =>
                    {
                        if let Some(fallback_url) = self
                            .app_service_health
                            .octos_service
                            .handle_http_result(response.status_code)
                        {
                            self.send_app_service_health_request(
                                cx,
                                OCTOS_SERVICE_HEALTH_REQUEST_ID,
                                &fallback_url,
                            );
                        }
                        self.sync_app_service_health_ui(cx);
                    }
                    NetworkResponse::HttpResponse { request_id, response }
                        if *request_id == MATRIX_GATEWAY_HEALTH_REQUEST_ID =>
                    {
                        self.app_service_health
                            .matrix_gateway
                            .handle_http_result(response.status_code);
                        self.sync_app_service_health_ui(cx);
                    }
                    NetworkResponse::HttpError { request_id, .. }
                        if *request_id == OCTOS_SERVICE_HEALTH_REQUEST_ID =>
                    {
                        if let Some(fallback_url) = self
                            .app_service_health
                            .octos_service
                            .handle_transport_error()
                        {
                            self.send_app_service_health_request(
                                cx,
                                OCTOS_SERVICE_HEALTH_REQUEST_ID,
                                &fallback_url,
                            );
                        }
                        self.sync_app_service_health_ui(cx);
                    }
                    NetworkResponse::HttpError { request_id, .. }
                        if *request_id == MATRIX_GATEWAY_HEALTH_REQUEST_ID =>
                    {
                        self.app_service_health.matrix_gateway.handle_transport_error();
                        self.sync_app_service_health_ui(cx);
                    }
                    _ => {}
                }
            }
        }

        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

impl WidgetMatchEvent for BotSettings {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        let app_service_switch = self.view.check_box(cx, ids!(app_service_switch));
        let botfather_user_id_input = self.view.text_input(cx, ids!(botfather_user_id_input));
        let octos_service_input = self.view.text_input(cx, ids!(octos_service_input));

        let Some(app_state) = _scope.data.get_mut::<AppState>() else {
            return;
        };

        if let Some(enabled) = app_service_switch.changed(actions) {
            app_state.bot_settings.enabled = enabled;
            persist_bot_settings(app_state);
            self.sync_ui(cx, &app_state.bot_settings);
            self.view.redraw(cx);
        }

        if self.view.button(cx, ids!(save_octos_service_button)).clicked(actions)
            || botfather_user_id_input.returned(actions).is_some()
            || octos_service_input.returned(actions).is_some()
        {
            match self.save_app_service_settings(cx, app_state) {
                Ok(_) => {
                    enqueue_popup_notification(
                        tr_key(self.app_language, "settings.labs.app_service.popup.saved").to_string(),
                        PopupKind::Success,
                        Some(3.0),
                    );
                }
                Err(error) => {
                    enqueue_popup_notification(
                        tr_fmt(
                            self.app_language,
                            "settings.labs.app_service.health.validation.invalid_url",
                            &[("error", &error)],
                        ),
                        PopupKind::Error,
                        Some(4.0),
                    );
                }
            }
        }

        if self.view.button(cx, ids!(check_now_button)).clicked(actions) {
            let service_url = match self.save_app_service_settings(cx, app_state) {
                Ok(service_url) => service_url,
                Err(error) => {
                    enqueue_popup_notification(
                        error,
                        PopupKind::Error,
                        Some(4.0),
                    );
                    return;
                }
            };
            let requests = self.app_service_health.begin_check(&service_url);
            if !requests.is_empty() {
                self.sync_app_service_health_ui(cx);
                for (request_id, url) in requests {
                    self.send_app_service_health_request(cx, request_id, &url);
                }
            }
        }
    }
}

impl BotSettings {
    fn save_app_service_settings(&mut self, cx: &mut Cx, app_state: &mut AppState) -> Result<String, String> {
        let botfather_user_id = self.view
            .text_input(cx, ids!(botfather_user_id_input))
            .text()
            .trim()
            .to_string();
        BotSettingsState::validate_botfather_user_id(
            &botfather_user_id,
            current_user_id().as_deref(),
        ).map_err(|error|
            tr_fmt(
                self.app_language,
                "settings.labs.app_service.validation.invalid_botfather_user_id",
                &[("error", &error)],
            )
        )?;

        let service_url = self.view
            .text_input(cx, ids!(octos_service_input))
            .text()
            .trim()
            .to_string();
        BotSettingsState::validate_octos_service_url(&service_url).map_err(|error|
            tr_fmt(
                self.app_language,
                "settings.labs.app_service.health.validation.invalid_url",
                &[("error", &error)],
            )
        )?;
        app_state.bot_settings.botfather_user_id = botfather_user_id.clone();
        app_state.bot_settings.octos_service_url = service_url.clone();
        persist_bot_settings(app_state);
        self.view
            .text_input(cx, ids!(botfather_user_id_input))
            .set_text(cx, &botfather_user_id);
        self.view
            .text_input(cx, ids!(octos_service_input))
            .set_text(cx, &service_url);
        Ok(service_url)
    }

    fn send_app_service_health_request(&self, cx: &mut Cx, request_id: LiveId, url: &str) {
        let req = build_app_service_health_request(request_id, url);
        cx.http_request(request_id, req);
    }

    fn set_switch_state_label(&mut self, cx: &mut Cx, enabled: bool) {
        let mut switch_state_label = self.view.label(cx, ids!(switch_state_label));
        if enabled {
            script_apply_eval!(cx, switch_state_label, {
                text: #(tr_key(self.app_language, "settings.labs.app_service.status.enabled")),
                draw_text +: {
                    color: mod.widgets.COLOR_ACTIVE_PRIMARY
                }
            });
        } else {
            script_apply_eval!(cx, switch_state_label, {
                text: #(tr_key(self.app_language, "settings.labs.app_service.status.disabled")),
                draw_text +: {
                    color: #999
                }
            });
        }
    }

    fn app_service_status_text_and_color(
        status: OctosHealthStatus,
    ) -> (&'static str, Vec4) {
        match status {
            OctosHealthStatus::Unknown => (
                "settings.labs.app_service.health.status.unknown",
                vec4(0.6, 0.6, 0.6, 1.0),
            ),
            OctosHealthStatus::Checking => (
                "settings.labs.app_service.health.status.checking",
                vec4(0.6, 0.6, 0.6, 1.0),
            ),
            OctosHealthStatus::Reachable => (
                "settings.labs.app_service.health.status.reachable",
                vec4(0.0, 0.6666667, 0.0, 1.0),
            ),
            OctosHealthStatus::Unreachable => (
                "settings.labs.app_service.health.status.unreachable",
                vec4(0.8, 0.0, 0.0, 1.0),
            ),
        }
    }

    fn set_health_status_label(&mut self, cx: &mut Cx, label_id: &[LiveId], status: OctosHealthStatus) {
        let (text_key, color) = Self::app_service_status_text_and_color(status);
        let mut label = self.view.label(cx, label_id);
        script_apply_eval!(cx, label, {
            text: #(tr_key(self.app_language, text_key)),
            draw_text +: {
                color: #(color)
            }
        });
    }

    fn sync_app_service_health_ui(&mut self, cx: &mut Cx) {
        self.set_health_status_label(
            cx,
            ids!(octos_service_status_label),
            self.app_service_health.octos_service.status,
        );
        self.set_health_status_label(
            cx,
            ids!(matrix_gateway_status_label),
            self.app_service_health.matrix_gateway.status,
        );
        self.view.button(cx, ids!(save_octos_service_button))
            .set_enabled(cx, !self.app_service_health.in_flight());
        self.view.button(cx, ids!(check_now_button))
            .set_enabled(cx, !self.app_service_health.in_flight());
        self.view.redraw(cx);
    }

    fn set_app_language(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        self.app_language = app_language;
        self.sync_app_language(cx);
    }

    fn sync_app_language(&mut self, cx: &mut Cx) {
        self.view
            .label(cx, ids!(app_service_title))
            .set_text(cx, tr_key(self.app_language, "settings.labs.app_service.title"));
        self.view
            .label(cx, ids!(description))
            .set_text(cx, tr_key(self.app_language, "settings.labs.app_service.description"));
        self.view
            .label(cx, ids!(manage_hint))
            .set_text(cx, tr_key(self.app_language, "settings.labs.app_service.manage_hint"));
        self.view
            .label(cx, ids!(botfather_user_id_label))
            .set_text(cx, tr_key(self.app_language, "settings.labs.app_service.botfather_user_id"));
        self.view
            .text_input(cx, ids!(botfather_user_id_input))
            .set_empty_text(cx, tr_key(self.app_language, "settings.labs.app_service.botfather_placeholder").to_string());
        self.view
            .label(cx, ids!(octos_service_label))
            .set_text(cx, tr_key(self.app_language, "settings.labs.app_service.health.service_label"));
        self.view
            .text_input(cx, ids!(octos_service_input))
            .set_empty_text(cx, tr_key(self.app_language, "settings.labs.app_service.health.placeholder").to_string());
        self.view
            .button(cx, ids!(save_octos_service_button))
            .set_text(cx, tr_key(self.app_language, "settings.labs.app_service.health.button.save"));
        self.view
            .button(cx, ids!(check_now_button))
            .set_text(cx, tr_key(self.app_language, "settings.labs.app_service.health.button.check_now"));
        self.view
            .label(cx, ids!(octos_service_status_name))
            .set_text(cx, tr_key(self.app_language, "settings.labs.app_service.health.service_label"));
        self.view
            .label(cx, ids!(matrix_gateway_status_name))
            .set_text(cx, tr_key(self.app_language, "settings.labs.app_service.health.gateway_label"));
        self.set_switch_state_label(
            cx,
            self.view.check_box(cx, ids!(app_service_switch)).active(cx),
        );
        self.sync_app_service_health_ui(cx);
        self.view.redraw(cx);
    }

    fn sync_ui(&mut self, cx: &mut Cx, bot_settings: &BotSettingsState) {
        self.view
            .check_box(cx, ids!(app_service_switch))
            .set_active(cx, bot_settings.enabled, Animate::No);
        self.view
            .text_input(cx, ids!(botfather_user_id_input))
            .set_text(cx, bot_settings.botfather_user_id.trim());
        self.view
            .text_input(cx, ids!(octos_service_input))
            .set_text(cx, bot_settings.resolved_octos_service_url());
        self.set_switch_state_label(cx, bot_settings.enabled);
        self.sync_app_service_health_ui(cx);
        self.view.redraw(cx);
    }

    /// Populates the bot settings UI from the current persisted app state.
    pub fn populate(&mut self, cx: &mut Cx, bot_settings: &BotSettingsState) {
        self.sync_app_language(cx);
        self.sync_ui(cx, bot_settings);
    }
}

impl BotSettingsRef {
    /// See [`BotSettings::populate()`].
    pub fn populate(&self, cx: &mut Cx, bot_settings: &BotSettingsState) {
        let Some(mut inner) = self.borrow_mut() else {
            return;
        };
        inner.populate(cx, bot_settings);
    }

    pub fn set_app_language(&self, cx: &mut Cx, app_language: AppLanguage) {
        let Some(mut inner) = self.borrow_mut() else {
            return;
        };
        inner.set_app_language(cx, app_language);
    }
}

fn persist_bot_settings(app_state: &AppState) {
    if let Some(user_id) = current_user_id() {
        if let Err(e) = persistence::save_app_state(app_state.clone(), user_id) {
            error!("Failed to persist bot settings. Error: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_app_service_health_request, matrix_gateway_probe_url, AppServiceHealthState,
        OctosHealthProbeStage, OctosHealthState, OctosHealthStatus,
        MATRIX_GATEWAY_HEALTH_REQUEST_ID, OCTOS_SERVICE_HEALTH_REQUEST_ID,
    };
    use crate::app::BotSettingsState;
    use makepad_widgets::HttpMethod;

    #[test]
    fn test_app_service_health_check_starts_octos_service_and_matrix_gateway_probes() {
        let mut state = AppServiceHealthState::default();
        let bot_settings = BotSettingsState::default();

        let requests = state.begin_check(bot_settings.resolved_octos_service_url());

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].1, "http://127.0.0.1:8010/health");
        assert_eq!(
            requests[1].1,
            "http://127.0.0.1:8009/_matrix/app/v1/transactions/test"
        );
        assert_eq!(state.octos_service.status, OctosHealthStatus::Checking);
        assert_eq!(state.matrix_gateway.status, OctosHealthStatus::Checking);
        assert!(state.in_flight());
    }

    #[test]
    fn test_matrix_gateway_probe_uses_local_port_8009() {
        assert_eq!(
            matrix_gateway_probe_url("http://127.0.0.1:8010").as_deref(),
            Some("http://127.0.0.1:8009/_matrix/app/v1/transactions/test")
        );
        assert_eq!(
            matrix_gateway_probe_url("http://localhost:8080/").as_deref(),
            Some("http://localhost:8009/_matrix/app/v1/transactions/test")
        );
    }

    #[test]
    fn test_matrix_gateway_401_counts_as_reachable() {
        let mut state = AppServiceHealthState::default();
        let bot_settings = BotSettingsState::default();

        state.begin_check(bot_settings.resolved_octos_service_url());
        assert_eq!(state.matrix_gateway.handle_http_result(401), None);
        assert_eq!(state.matrix_gateway.status, OctosHealthStatus::Reachable);
    }

    #[test]
    fn test_app_service_health_remains_in_flight_until_both_checks_finish() {
        let mut state = AppServiceHealthState::default();
        let bot_settings = BotSettingsState::default();

        state.begin_check(bot_settings.resolved_octos_service_url());
        state.matrix_gateway.handle_http_result(405);
        assert!(state.in_flight());

        assert_eq!(state.octos_service.handle_http_result(200), None);
        assert!(!state.in_flight());
    }

    #[test]
    fn test_matrix_gateway_probe_uses_put_json_request_shape() {
        let req = build_app_service_health_request(
            MATRIX_GATEWAY_HEALTH_REQUEST_ID,
            "http://127.0.0.1:8009/_matrix/app/v1/transactions/test",
        );

        assert_eq!(req.method, HttpMethod::PUT);
        assert_eq!(
            req.headers.get("Content-Type"),
            Some(&vec!["application/json".to_string()])
        );
        assert_eq!(req.body.as_deref(), Some(br#"{}"#.as_slice()));
    }

    #[test]
    fn test_octos_service_probe_stays_get_without_body() {
        let req = build_app_service_health_request(
            OCTOS_SERVICE_HEALTH_REQUEST_ID,
            "http://127.0.0.1:8010/health",
        );

        assert_eq!(req.method, HttpMethod::GET);
        assert!(req.body.is_none());
    }

    #[test]
    fn test_app_service_health_defaults_to_unknown_with_editable_local_url() {
        let state = OctosHealthState::default();
        let bot_settings = BotSettingsState::default();

        assert_eq!(
            bot_settings.resolved_octos_service_url(),
            BotSettingsState::DEFAULT_OCTOS_SERVICE_URL,
        );
        assert_eq!(state.status, OctosHealthStatus::Unknown);
        assert!(!state.in_flight);
        assert_eq!(state.probe_stage, OctosHealthProbeStage::Idle);
    }

    #[test]
    fn test_app_service_health_uses_custom_octos_service_url_when_configured() {
        let mut bot_settings = BotSettingsState::default();
        bot_settings.octos_service_url = "https://octos.example.com:9443".into();

        assert_eq!(
            bot_settings.resolved_octos_service_url(),
            "https://octos.example.com:9443",
        );
    }

    #[test]
    fn test_app_service_health_validates_octos_service_url() {
        assert!(BotSettingsState::validate_octos_service_url("http://127.0.0.1:8010").is_ok());
        assert!(BotSettingsState::validate_octos_service_url("https://octos.example.com").is_ok());
        assert!(BotSettingsState::validate_octos_service_url("127.0.0.1:8010").is_err());
        assert!(BotSettingsState::validate_octos_service_url("notaurl").is_err());
    }

    #[test]
    fn test_app_service_health_check_uses_health_endpoint_first() {
        let mut state = OctosHealthState::default();
        let bot_settings = BotSettingsState::default();

        assert_eq!(state.begin_check(bot_settings.resolved_octos_service_url()).as_deref(), Some("http://127.0.0.1:8010/health"));
        assert_eq!(state.status, OctosHealthStatus::Checking);
        assert!(state.in_flight);
        assert_eq!(state.probe_stage, OctosHealthProbeStage::Health);

        assert_eq!(state.handle_http_result(200), None);
        assert_eq!(state.status, OctosHealthStatus::Reachable);
        assert!(!state.in_flight);
        assert_eq!(state.probe_stage, OctosHealthProbeStage::Idle);
    }

    #[test]
    fn test_app_service_health_check_falls_back_to_api_status() {
        let mut state = OctosHealthState::default();
        let bot_settings = BotSettingsState::default();

        state.begin_check(bot_settings.resolved_octos_service_url());
        assert_eq!(
            state.handle_transport_error().as_deref(),
            Some("http://127.0.0.1:8010/api/status")
        );
        assert_eq!(state.status, OctosHealthStatus::Checking);
        assert!(state.in_flight);
        assert_eq!(state.probe_stage, OctosHealthProbeStage::ApiStatus);

        assert_eq!(state.handle_http_result(200), None);
        assert_eq!(state.status, OctosHealthStatus::Reachable);
        assert!(!state.in_flight);
        assert_eq!(state.probe_stage, OctosHealthProbeStage::Idle);
    }

    #[test]
    fn test_app_service_health_check_tries_native_local_octos_port_after_8010_fails() {
        let mut state = OctosHealthState::default();
        let bot_settings = BotSettingsState::default();

        state.begin_check(bot_settings.resolved_octos_service_url());
        assert_eq!(
            state.handle_transport_error().as_deref(),
            Some("http://127.0.0.1:8010/api/status")
        );
        assert_eq!(
            state.handle_transport_error().as_deref(),
            Some("http://127.0.0.1:8080/health")
        );
        assert_eq!(state.status, OctosHealthStatus::Checking);
        assert!(state.in_flight);
        assert_eq!(state.probe_stage, OctosHealthProbeStage::Health);
        assert_eq!(state.handle_http_result(200), None);
        assert_eq!(state.status, OctosHealthStatus::Reachable);
        assert!(!state.in_flight);
        assert_eq!(state.probe_stage, OctosHealthProbeStage::Idle);
    }

    #[test]
    fn test_app_service_health_check_sets_unreachable_when_both_local_ports_fail() {
        let mut state = OctosHealthState::default();
        let bot_settings = BotSettingsState::default();

        state.begin_check(bot_settings.resolved_octos_service_url());
        state.handle_transport_error();
        state.handle_transport_error();
        state.handle_transport_error();
        assert_eq!(state.handle_transport_error(), None);
        assert_eq!(state.status, OctosHealthStatus::Unreachable);
        assert!(!state.in_flight);
        assert_eq!(state.probe_stage, OctosHealthProbeStage::Idle);
    }

    #[test]
    fn test_app_service_health_check_does_not_try_local_alt_port_for_remote_url() {
        let mut state = OctosHealthState::default();
        let mut bot_settings = BotSettingsState::default();
        bot_settings.octos_service_url = "https://octos.example.com:9443".into();

        state.begin_check(bot_settings.resolved_octos_service_url());
        assert_eq!(
            state.handle_transport_error().as_deref(),
            Some("https://octos.example.com:9443/api/status")
        );
        assert_eq!(state.handle_transport_error(), None);
        assert_eq!(state.status, OctosHealthStatus::Unreachable);
        assert!(!state.in_flight);
        assert_eq!(state.probe_stage, OctosHealthProbeStage::Idle);
    }

    #[test]
    fn test_app_service_health_check_disables_check_now_while_checking() {
        let mut state = OctosHealthState::default();
        let bot_settings = BotSettingsState::default();

        state.begin_check(bot_settings.resolved_octos_service_url());
        assert_eq!(state.begin_check(bot_settings.resolved_octos_service_url()), None);
        assert_eq!(state.status, OctosHealthStatus::Checking);
        assert!(state.in_flight);
    }

    #[test]
    fn test_app_service_health_does_not_auto_probe_on_open() {
        let state = OctosHealthState::default();

        assert_eq!(state.status, OctosHealthStatus::Unknown);
        assert!(!state.in_flight);
        assert_eq!(state.probe_stage, OctosHealthProbeStage::Idle);
    }

    #[test]
    fn test_app_service_health_check_is_ui_only() {
        let mut bot_settings = BotSettingsState::default();
        bot_settings.enabled = true;
        let before = bot_settings.clone();

        let mut state = OctosHealthState::default();
        state.begin_check(bot_settings.resolved_octos_service_url());
        state.handle_transport_error();
        state.handle_transport_error();
        state.handle_transport_error();
        state.handle_transport_error();

        assert_eq!(bot_settings, before);
        assert_eq!(state.status, OctosHealthStatus::Unreachable);
    }
}
