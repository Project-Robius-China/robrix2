//! The "Add an agent" bottom-sheet modal (Agent Registry design handoff).
//!
//! Two-step flow:
//!   Step 1 — choose a framework (Hermes / OpenClaw = direct friend agents,
//!            Octos = AppService agent).
//!   Step 2 — enter the agent's Matrix ID and "Add friend & bind" (real DM via
//!            `OpenOrCreateDirectMessage`). For Octos, an additional AppService
//!            section writes the real `BotSettingsState` (enable + BotFather ID +
//!            local Octos URL) and runs the shared `OctosHealthState` health probe
//!            — so the existing App Service / slash-command binding is reused, not
//!            duplicated or broken.
//!
//! "Finish & register" writes the agent into the global `AgentRegistry`.
//!
//! NOTE: inside the `script_mod!` block, only `//` comments are allowed.

use makepad_widgets::*;
use ruma::OwnedUserId;

use crate::{
    app::{AgentFramework, AppState},
    i18n::AppLanguage,
    persistence,
    profile::user_profile::UserProfile,
    shared::avatar::AvatarState,
    settings::{
        agent_settings::{framework_label, parse_agent_user_id, register_agent_from_search},
        bot_settings::{OctosHealthState, OctosHealthStatus},
    },
    sliding_sync::{
        DirectMessageRoomAction, MatrixRequest, current_user_id, submit_async_request,
    },
};

const AGENT_OCTOS_HEALTH_REQUEST_ID: LiveId = live_id!(agent_add_octos_health);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum FriendState {
    #[default]
    Idle,
    Pending,
    Added,
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // Back chevron icon (left arrow) — drawn as SVG, not a text glyph, so it
    // never renders as a missing-glyph box in the app font.
    mod.widgets.AGENT_ICON_BACK = crate_resource("self://resources/icons/go_back.svg")

    // A selectable framework card for step 1. Root is Overlay so the transparent
    // `card_click` button covers the whole card (mirrors invite_modal result row).
    let FrameworkCard = View {
        width: Fill
        height: Fit
        flow: Overlay

        card_body := RoundedView {
            width: Fill
            height: Fit
            flow: Right
            align: Align{y: 0.5}
            spacing: 11
            padding: Inset{left: 12, right: 12, top: 9, bottom: 9}
            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_LG)
                border_size: 1.5
                border_color: (RBX_STROKE_SOFT)
            }

            card_tile := RoundedView {
                width: 40
                height: 40
                align: Align{x: 0.5, y: 0.5}
                show_bg: true
                draw_bg +: {
                    color: (RBX_BG_SURFACE_SUBTLE)
                    border_radius: (RBX_RADIUS_MD)
                }
                card_mono := Label {
                    width: Fit
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: TITLE_TEXT { font_size: 13.0 }
                    }
                    text: ""
                }
            }

            card_col := View {
                width: Fill
                height: Fit
                flow: Down
                spacing: 3
                card_name := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: TITLE_TEXT { font_size: 13.0 }
                    }
                    text: ""
                }
                card_tag_label := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_BADGE {}
                    }
                    text: ""
                }
                card_blurb := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_META {}
                    }
                    text: ""
                }
            }

            card_radio := RoundedView {
                width: 22
                height: 22
                align: Align{x: 0.5, y: 0.5}
                show_bg: true
                draw_bg +: {
                    color: (RBX_BG_SURFACE)
                    border_radius: (RBX_RADIUS_PILL)
                    border_size: 2.0
                    border_color: (RBX_STROKE_STRONG)
                }
                card_radio_check := Icon {
                    width: 12
                    height: 12
                    visible: false
                    draw_icon +: {
                        svg: (ICON_CHECKMARK)
                        color: (RBX_FG_ON_ACCENT)
                    }
                }
            }
        }

        card_click := RobrixNeutralIconButton {
            width: Fill
            height: Fill
            text: ""
            icon_walk: Walk{width: 0, height: 0}
            draw_bg +: {
                color: #0000
                color_hover: #00000008
                color_down: #00000012
                border_radius: (RBX_RADIUS_LG)
            }
        }
    }

    let AgentField = View {
        width: Fill
        height: Fit
        flow: Down
        spacing: 5
        field_label := Label {
            width: Fill
            height: Fit
            draw_text +: {
                color: (RBX_FG_SECONDARY)
                text_style: RBX_TEXT_META {}
            }
            text: ""
        }
        field_input := RobrixTextInput {
            width: Fill
            height: Fit
            padding: 11
            empty_text: ""
            draw_bg +: {
                color: (RBX_BG_SURFACE_SUBTLE)
                border_radius: (RBX_RADIUS_SM)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
            }
        }
    }

    mod.widgets.AddAgentModal = #(AddAgentModal::register_widget(vm)) {
        // Fit/Fit root — this size reliably renders inside a Modal (Fill/Fill
        // collapses the sheet). The hosting Modal content aligns it to the
        // bottom-center (align y:1.0) so it reads as a bottom sheet, and the
        // Modal's own scrim handles tap-outside-to-dismiss.
        width: Fit
        height: Fit

        sheet := RoundedView {
            width: 360
            height: Fit
            flow: Down
            // Lift the sheet clear of the bottom navigation bar behind the modal
            // (the nav is drawn above app-root modals, so its tap area would
            // otherwise overlap the sheet's footer). This margin keeps the footer
            // button above the nav's hit zone.
            margin: Inset{bottom: 96}
            // capture_overload + cursor make this view hit-test and ABSORB every
            // finger event inside the sheet's rect that a child control didn't
            // already take (e.g. taps on padding around the Continue button).
            // Without this, those taps fall through the sheet to the buttons
            // behind the modal. (A View only hit-tests when it has a cursor or an
            // animator — see widgets/src/view.rs.)
            capture_overload: true
            cursor: MouseCursor.Default
            padding: Inset{left: 18, right: 18, top: 10, bottom: 26}
            spacing: 0
            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_XL)
            }

            grip := View {
                width: 38
                height: 5
                margin: Inset{bottom: 12}
                align: Align{x: 0.5}
                show_bg: true
                draw_bg +: {
                    color: (RBX_STROKE_STRONG)
                    border_radius: (RBX_RADIUS_PILL)
                }
            }

            header := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}
                spacing: 8
                margin: Inset{bottom: 10}

                back_button := RobrixNeutralIconButton {
                    width: Fit
                    height: Fit
                    visible: false
                    padding: Inset{left: 2, right: 6, top: 6, bottom: 6}
                    spacing: 0
                    text: ""
                    draw_icon.svg: (AGENT_ICON_BACK)
                    draw_icon.color: (RBX_FG_SECONDARY)
                    icon_walk: Walk{width: 18, height: 18}
                    draw_bg +: { color: #0000, color_hover: #0000, color_down: #0000 }
                }
                header_col := View {
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 1
                    sheet_title := Label {
                        width: Fill
                        height: Fit
                        draw_text +: {
                            color: (RBX_FG_PRIMARY)
                            text_style: RBX_TEXT_PAGE_TITLE {}
                        }
                        text: "Add an agent"
                    }
                    sheet_subtitle := Label {
                        width: Fill
                        height: Fit
                        draw_text +: {
                            color: (RBX_FG_SECONDARY)
                            text_style: RBX_TEXT_META {}
                        }
                        text: "Step 1 of 2 · Choose a framework"
                    }
                }
                close_button := RobrixNeutralIconButton {
                    width: Fit
                    height: Fit
                    padding: Inset{left: 6, right: 6, top: 6, bottom: 6}
                    spacing: 0
                    text: ""
                    draw_icon.svg: (ICON_CLOSE)
                    draw_icon.color: (RBX_FG_TERTIARY)
                    icon_walk: Walk{width: 16, height: 16}
                    draw_bg +: { color: #0000, color_hover: #0000, color_down: #0000 }
                }
            }

            // ---------- STEP 1: framework picker ----------
            step1_view := View {
                width: Fill
                height: Fit
                flow: Down
                spacing: 9

                step1_intro := Label {
                    width: Fill
                    height: Fit
                    margin: Inset{bottom: 2}
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_BODY {}
                    }
                    text: "Each framework registers a little differently. Pick the one your agent runs on."
                }

                // Per-framework visuals (mono, name, tag, blurb, colors) are
                // populated from Rust in `populate_framework_cards` to avoid
                // deep DSL overrides (unreliable in this Makepad fork).
                octos_card := FrameworkCard {}
                hermes_card := FrameworkCard {}
                openclaw_card := FrameworkCard {}
            }

            // ---------- STEP 2: enter id + bind ----------
            step2_view := View {
                width: Fill
                height: Fit
                flow: Down
                spacing: 9
                visible: false

                step2_heading := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: RBX_TEXT_BODY_STRONG {}
                    }
                    text: "New agent"
                }

                id_field := AgentField {
                    field_label.text: "Agent Matrix ID"
                    field_input.empty_text: "@agent:server"
                }

                step2_helper := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_META {}
                    }
                    text: "Robrix sends a friend request to this account and records its framework."
                }

                add_friend_button := RobrixIconButton {
                    width: Fill
                    height: (RBX_CONTROL_H_LG)
                    margin: Inset{top: 2}
                    padding: Inset{top: 10, bottom: 10, left: 16, right: 16}
                    icon_walk: Walk{width: 0, height: 0}
                    spacing: 0
                    text: "＋ Add friend & bind"
                    draw_bg +: {
                        color: (RBX_BG_SURFACE)
                        color_hover: (RBX_ACCENT_SOFT)
                        color_down: (RBX_BG_SELECTED)
                        border_radius: (RBX_RADIUS_SM)
                        border_size: 1.5
                        border_color: (RBX_ACCENT)
                    }
                    draw_text +: { color: (RBX_ACCENT), color_hover: (RBX_ACCENT), color_down: (RBX_ACCENT) }
                }

                friend_added_strip := RoundedView {
                    width: Fill
                    height: Fit
                    visible: false
                    flow: Right
                    align: Align{y: 0.5}
                    spacing: 9
                    margin: Inset{top: 2}
                    padding: Inset{left: 12, right: 12, top: 11, bottom: 11}
                    show_bg: true
                    draw_bg +: {
                        color: (RBX_SUCCESS_BG)
                        border_radius: (RBX_RADIUS_SM)
                        border_size: 1.0
                        border_color: (RBX_SUCCESS_FG)
                    }
                    friend_added_label := Label {
                        width: Fill
                        height: Fit
                        draw_text +: {
                            color: (RBX_SUCCESS_FG)
                            text_style: RBX_TEXT_BODY {}
                        }
                        text: "Friend request sent."
                    }
                }

                // ---------- Octos AppService section ----------
                octos_section := View {
                    width: Fill
                    height: Fit
                    visible: false
                    flow: Down
                    spacing: 9
                    margin: Inset{top: 6}
                    padding: Inset{top: 14}
                    show_bg: true
                    draw_bg +: { color: #0000 }

                    octos_divider := View {
                        width: Fill
                        height: 1.0
                        margin: Inset{bottom: 6}
                        show_bg: true
                        draw_bg +: { color: (RBX_STROKE_SOFT) }
                    }
                    octos_heading := Label {
                        width: Fill
                        height: Fit
                        draw_text +: {
                            color: (RBX_FG_PRIMARY)
                            text_style: RBX_TEXT_BODY_STRONG {}
                        }
                        text: "AppService binding"
                    }
                    octos_blurb := Label {
                        width: Fill
                        height: Fit
                        draw_text +: {
                            color: (RBX_FG_SECONDARY)
                            text_style: RBX_TEXT_META {}
                        }
                        text: "Octos runs as a Matrix AppService. Point Robrix at the local service and confirm it responds."
                    }
                    botfather_field := AgentField {
                        field_label.text: "BotFather user ID"
                        field_input.empty_text: "octos"
                    }
                    octos_url_field := AgentField {
                        field_label.text: "Local Octos service"
                        field_input.empty_text: "http://127.0.0.1:8010"
                    }
                    octos_check_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        align: Align{y: 0.5}
                        spacing: 9
                        margin: Inset{top: 2}
                        check_now_button := RobrixIconButton {
                            width: Fit
                            height: Fit
                            padding: Inset{top: 9, bottom: 9, left: 16, right: 16}
                            icon_walk: Walk{width: 0, height: 0}
                            spacing: 0
                            text: "Check now"
                            draw_bg +: {
                                color: (RBX_FW_OCTOS_BG)
                                color_hover: (RBX_FW_OCTOS_BG)
                                color_down: (RBX_FW_OCTOS_BG)
                                border_radius: (RBX_RADIUS_SM)
                            }
                            draw_text +: { color: (RBX_FW_OCTOS_FG), color_hover: (RBX_FW_OCTOS_FG), color_down: (RBX_FW_OCTOS_FG) }
                        }
                        octos_status_pill := RoundedView {
                            width: Fit
                            height: Fit
                            padding: Inset{left: 12, right: 12, top: 6, bottom: 6}
                            show_bg: true
                            draw_bg +: {
                                color: (RBX_NEUTRAL_BG)
                                border_radius: (RBX_RADIUS_PILL)
                            }
                            octos_status_label := Label {
                                width: Fit
                                height: Fit
                                draw_text +: {
                                    color: (RBX_NEUTRAL_FG)
                                    text_style: RBX_TEXT_BADGE {}
                                }
                                text: "Unknown"
                            }
                        }
                    }
                    octos_error_label := Label {
                        width: Fill
                        height: Fit
                        visible: false
                        margin: Inset{top: 2}
                        draw_text +: {
                            color: (RBX_DANGER_FG)
                            text_style: RBX_TEXT_META {}
                        }
                        text: ""
                    }
                }
            }

            // ---------- footer ----------
            footer := View {
                width: Fill
                height: Fit
                flow: Down
                margin: Inset{top: 14}
                padding: Inset{top: 12}
                show_bg: true
                draw_bg +: { color: #0000 }

                footer_divider := View {
                    width: Fill
                    height: 1.0
                    margin: Inset{bottom: 12}
                    show_bg: true
                    draw_bg +: { color: (RBX_STROKE_SOFT) }
                }
                primary_button := RobrixIconButton {
                    width: Fill
                    height: (RBX_CONTROL_H_LG)
                    padding: Inset{top: 11, bottom: 11, left: 16, right: 16}
                    icon_walk: Walk{width: 0, height: 0}
                    spacing: 0
                    text: "Continue"
                    draw_bg +: {
                        color: (RBX_ACCENT)
                        color_hover: (RBX_ACCENT_HOVER)
                        color_down: (RBX_ACCENT_PRESSED)
                        border_radius: (RBX_RADIUS_SM)
                    }
                    draw_text +: { color: (RBX_FG_ON_ACCENT), color_hover: (RBX_FG_ON_ACCENT), color_down: (RBX_FG_ON_ACCENT) }
                }
            }
        }
    }
}

/// Actions emitted by the [`AddAgentModal`].
#[derive(Clone, Debug)]
pub enum AddAgentModalAction {
    /// The modal should be closed (cancel / scrim dismiss / after finish).
    Close,
    /// An agent was registered; carries the display name for a success toast.
    Registered(String),
}

#[derive(Script, ScriptHook, Widget)]
pub struct AddAgentModal {
    #[deref]
    view: View,
    #[rust]
    app_language: AppLanguage,
    #[rust]
    step: u8,
    #[rust]
    selected_framework: Option<AgentFramework>,
    #[rust]
    friend_state: FriendState,
    #[rust]
    target_user_id: Option<OwnedUserId>,
    #[rust]
    octos_health: OctosHealthState,
}

impl Widget for AddAgentModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if let Event::NetworkResponses(responses) = event {
            for response in responses {
                match response {
                    NetworkResponse::HttpResponse { request_id, response }
                        if *request_id == AGENT_OCTOS_HEALTH_REQUEST_ID =>
                    {
                        let url = self.octos_service_url(cx);
                        if let Some(fallback) = self.octos_health.handle_http_result(&url, response.status_code) {
                            self.send_health_request(cx, &fallback);
                        }
                        self.sync_octos_status(cx);
                    }
                    NetworkResponse::HttpError { request_id, .. }
                        if *request_id == AGENT_OCTOS_HEALTH_REQUEST_ID =>
                    {
                        let url = self.octos_service_url(cx);
                        if let Some(fallback) = self.octos_health.handle_transport_error(&url) {
                            self.send_health_request(cx, &fallback);
                        }
                        self.sync_octos_status(cx);
                    }
                    _ => {}
                }
            }
        }
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl WidgetMatchEvent for AddAgentModal {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, scope: &mut Scope) {
        // Close / dismiss.
        if self.view.button(cx, ids!(close_button)).clicked(actions)
            || actions.iter().any(|a| matches!(a.downcast_ref(), Some(ModalAction::Dismissed)))
        {
            cx.action(AddAgentModalAction::Close);
            return;
        }

        // Back to step 1.
        if self.view.button(cx, ids!(back_button)).clicked(actions) {
            self.step = 1;
            self.sync_steps(cx);
            return;
        }

        // Framework card selection (step 1).
        if self.step == 1 {
            let cards = [
                (AgentFramework::Octos, ids!(octos_card.card_click)),
                (AgentFramework::Hermes, ids!(hermes_card.card_click)),
                (AgentFramework::OpenClaw, ids!(openclaw_card.card_click)),
            ];
            for (framework, click_id) in cards {
                if self.view.button(cx, click_id).clicked(actions) {
                    self.selected_framework = Some(framework);
                    self.update_framework_cards(cx);
                    self.sync_primary_button(cx);
                }
            }
        }

        // Primary footer button: Continue (step1) / Finish & register (step2).
        if self.view.button(cx, ids!(primary_button)).clicked(actions) {
            if self.step == 1 {
                if self.selected_framework.is_some() {
                    self.step = 2;
                    self.sync_steps(cx);
                }
            } else if self.can_finish() {
                self.finish_register(cx, scope);
            }
            return;
        }

        // Step 2: add friend & bind.
        if self.step == 2 && self.view.button(cx, ids!(add_friend_button)).clicked(actions) {
            self.add_friend(cx);
            return;
        }

        // Step 2 (Octos): check service health.
        if self.step == 2 && self.view.button(cx, ids!(check_now_button)).clicked(actions) {
            let url = self.octos_service_url(cx);
            if let Some(probe) = self.octos_health.begin_check(&url) {
                self.sync_octos_status(cx);
                self.send_health_request(cx, &probe);
            }
            return;
        }

        // Friend-request (DM) result.
        if self.friend_state == FriendState::Pending {
            for action in actions {
                match action.downcast_ref() {
                    Some(DirectMessageRoomAction::NewlyCreated { user_profile, .. })
                        if self.matches_target(&user_profile.user_id) =>
                    {
                        self.on_friend_added(cx);
                    }
                    Some(DirectMessageRoomAction::FoundExisting { user_id, .. })
                        if self.matches_target(user_id) =>
                    {
                        self.on_friend_added(cx);
                    }
                    Some(DirectMessageRoomAction::FailedToCreate { user_profile, error })
                        if self.matches_target(&user_profile.user_id) =>
                    {
                        self.friend_state = FriendState::Idle;
                        let msg = format!("Friend request failed: {error}");
                        self.set_add_friend_label(cx, &msg);
                        self.sync_step2(cx);
                    }
                    _ => {}
                }
            }
        }
    }
}

impl AddAgentModal {
    fn matches_target(&self, user_id: &OwnedUserId) -> bool {
        self.target_user_id.as_ref().is_some_and(|t| t.as_str() == user_id.as_str())
    }

    fn is_octos(&self) -> bool {
        self.selected_framework == Some(AgentFramework::Octos)
    }

    fn can_finish(&self) -> bool {
        self.friend_state == FriendState::Added
            && (!self.is_octos() || self.octos_health.status == OctosHealthStatus::Reachable)
    }

    fn octos_service_url(&self, cx: &mut Cx) -> String {
        let raw = self.view.text_input(cx, ids!(octos_section.octos_url_field.field_input)).text();
        let trimmed = raw.trim();
        if trimmed.is_empty() { "http://127.0.0.1:8010".to_string() } else { trimmed.to_string() }
    }

    fn add_friend(&mut self, cx: &mut Cx) {
        let raw = self.view.text_input(cx, ids!(id_field.field_input)).text();
        match parse_agent_user_id(&raw) {
            Ok(user_id) => {
                self.target_user_id = Some(user_id.clone());
                self.friend_state = FriendState::Pending;
                let display_name = user_id.localpart().to_string();
                submit_async_request(MatrixRequest::OpenOrCreateDirectMessage {
                    user_profile: UserProfile {
                        user_id,
                        username: Some(display_name),
                        avatar_state: AvatarState::Unknown,
                    },
                    allow_create: true,
                    create_encrypted: false,
                });
                self.set_add_friend_label(cx, "Sending friend request…");
                self.sync_step2(cx);
            }
            Err(error) => {
                self.set_add_friend_label(cx, &error);
                self.view.redraw(cx);
            }
        }
    }

    fn on_friend_added(&mut self, cx: &mut Cx) {
        self.friend_state = FriendState::Added;
        let uid = self.target_user_id.as_ref().map(|u| u.as_str().to_string()).unwrap_or_default();
        let framework = self.selected_framework.map(framework_label).unwrap_or("");
        self.view.label(cx, ids!(friend_added_strip.friend_added_label))
            .set_text(cx, &format!("Friend request sent — {uid} bound to {framework}."));
        self.sync_step2(cx);
    }

    fn finish_register(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let Some(framework) = self.selected_framework else { return };
        let Some(user_id) = self.target_user_id.clone() else { return };
        let display_name = user_id.localpart().to_string();

        let Some(app_state) = scope.data.get_mut::<AppState>() else { return };
        register_agent_from_search(app_state, user_id.clone(), Some(display_name.clone()), framework);

        // Octos: write the real App Service binding state (enable + BotFather + URL),
        // so the existing slash-command / appservice path is wired, not duplicated.
        if framework == AgentFramework::Octos {
            let botfather = self.view.text_input(cx, ids!(octos_section.botfather_field.field_input)).text().trim().to_string();
            let url = self.octos_service_url(cx);
            app_state.bot_settings.enabled = true;
            if !botfather.is_empty() {
                app_state.bot_settings.botfather_user_id = botfather;
            }
            app_state.bot_settings.octos_service_url = url;
        }

        if let Some(account_user_id) = current_user_id() {
            if let Err(e) = persistence::save_app_state(app_state.clone(), account_user_id) {
                error!("Failed to persist agent registry. Error: {e}");
            }
        }

        cx.action(AddAgentModalAction::Registered(display_name));
        cx.action(AddAgentModalAction::Close);
    }

    fn send_health_request(&self, cx: &mut Cx, url: &str) {
        let req = HttpRequest::new(url.to_string(), HttpMethod::GET);
        cx.http_request(AGENT_OCTOS_HEALTH_REQUEST_ID, req);
    }

    fn set_add_friend_label(&mut self, cx: &mut Cx, text: &str) {
        self.view.button(cx, ids!(add_friend_button)).set_text(cx, text);
    }

    fn populate_framework_cards(&mut self, cx: &mut Cx) {
        // Text content.
        self.view.label(cx, ids!(octos_card.card_body.card_tile.card_mono)).set_text(cx, "Oc");
        self.view.label(cx, ids!(octos_card.card_body.card_col.card_name)).set_text(cx, "Octos");
        self.view.label(cx, ids!(octos_card.card_body.card_col.card_tag_label)).set_text(cx, "APPSERVICE");
        self.view.label(cx, ids!(octos_card.card_body.card_col.card_blurb)).set_text(cx, "Added as a friend, plus a local AppService binding.");
        self.view.label(cx, ids!(hermes_card.card_body.card_tile.card_mono)).set_text(cx, "He");
        self.view.label(cx, ids!(hermes_card.card_body.card_col.card_name)).set_text(cx, "Hermes");
        self.view.label(cx, ids!(hermes_card.card_body.card_col.card_tag_label)).set_text(cx, "DIRECT AGENT");
        self.view.label(cx, ids!(hermes_card.card_body.card_col.card_blurb)).set_text(cx, "Registered by adding it as a Matrix friend.");
        self.view.label(cx, ids!(openclaw_card.card_body.card_tile.card_mono)).set_text(cx, "Cl");
        self.view.label(cx, ids!(openclaw_card.card_body.card_col.card_name)).set_text(cx, "OpenClaw");
        self.view.label(cx, ids!(openclaw_card.card_body.card_col.card_tag_label)).set_text(cx, "DIRECT AGENT");
        self.view.label(cx, ids!(openclaw_card.card_body.card_col.card_blurb)).set_text(cx, "Registered by adding it as a Matrix friend.");

        // Per-framework colors (tile fill + mono text + tag text).
        let mut octos_tile = self.view.view(cx, ids!(octos_card.card_body.card_tile));
        script_apply_eval!(cx, octos_tile, { draw_bg +: { color: mod.widgets.RBX_FW_OCTOS_BG } });
        let mut octos_mono = self.view.label(cx, ids!(octos_card.card_body.card_tile.card_mono));
        script_apply_eval!(cx, octos_mono, { draw_text +: { color: mod.widgets.RBX_FW_OCTOS_FG } });
        let mut octos_tag = self.view.label(cx, ids!(octos_card.card_body.card_col.card_tag_label));
        script_apply_eval!(cx, octos_tag, { draw_text +: { color: mod.widgets.RBX_FW_OCTOS_FG } });

        let mut hermes_tile = self.view.view(cx, ids!(hermes_card.card_body.card_tile));
        script_apply_eval!(cx, hermes_tile, { draw_bg +: { color: mod.widgets.RBX_FW_HERMES_BG } });
        let mut hermes_mono = self.view.label(cx, ids!(hermes_card.card_body.card_tile.card_mono));
        script_apply_eval!(cx, hermes_mono, { draw_text +: { color: mod.widgets.RBX_FW_HERMES_FG } });
        let mut hermes_tag = self.view.label(cx, ids!(hermes_card.card_body.card_col.card_tag_label));
        script_apply_eval!(cx, hermes_tag, { draw_text +: { color: mod.widgets.RBX_FW_HERMES_FG } });

        let mut openclaw_tile = self.view.view(cx, ids!(openclaw_card.card_body.card_tile));
        script_apply_eval!(cx, openclaw_tile, { draw_bg +: { color: mod.widgets.RBX_FW_OPENCLAW_BG } });
        let mut openclaw_mono = self.view.label(cx, ids!(openclaw_card.card_body.card_tile.card_mono));
        script_apply_eval!(cx, openclaw_mono, { draw_text +: { color: mod.widgets.RBX_FW_OPENCLAW_FG } });
        let mut openclaw_tag = self.view.label(cx, ids!(openclaw_card.card_body.card_col.card_tag_label));
        script_apply_eval!(cx, openclaw_tag, { draw_text +: { color: mod.widgets.RBX_FW_OPENCLAW_FG } });
    }

    fn update_framework_cards(&mut self, cx: &mut Cx) {
        let cards = [
            (AgentFramework::Octos, ids!(octos_card)),
            (AgentFramework::Hermes, ids!(hermes_card)),
            (AgentFramework::OpenClaw, ids!(openclaw_card)),
        ];
        for (framework, card_id) in cards {
            let selected = self.selected_framework == Some(framework);
            self.view.widget(cx, &[card_id[0], live_id!(card_body), live_id!(card_radio), live_id!(card_radio_check)])
                .set_visible(cx, selected);
            let mut radio = self.view.view(cx, &[card_id[0], live_id!(card_body), live_id!(card_radio)]);
            let mut card = self.view.view(cx, &[card_id[0], live_id!(card_body)]);
            if selected {
                script_apply_eval!(cx, radio, { draw_bg +: { color: mod.widgets.RBX_ACCENT, border_color: mod.widgets.RBX_ACCENT } });
                script_apply_eval!(cx, card, { draw_bg +: { border_size: 2.0, border_color: mod.widgets.RBX_ACCENT, color: mod.widgets.RBX_ACCENT_SOFT } });
            } else {
                script_apply_eval!(cx, radio, { draw_bg +: { color: mod.widgets.RBX_BG_SURFACE, border_color: mod.widgets.RBX_STROKE_STRONG } });
                script_apply_eval!(cx, card, { draw_bg +: { border_size: 1.5, border_color: mod.widgets.RBX_STROKE_SOFT, color: mod.widgets.RBX_BG_SURFACE } });
            }
        }
        self.view.redraw(cx);
    }

    fn sync_steps(&mut self, cx: &mut Cx) {
        let step2 = self.step == 2;
        self.view.view(cx, ids!(step1_view)).set_visible(cx, !step2);
        self.view.view(cx, ids!(step2_view)).set_visible(cx, step2);
        self.view.button(cx, ids!(back_button)).set_visible(cx, step2);

        if step2 {
            let fw = self.selected_framework.map(framework_label).unwrap_or("agent");
            self.view.label(cx, ids!(sheet_title)).set_text(cx, &format!("Connect {fw}"));
            let sub = if self.is_octos() {
                "Step 2 of 2 · Friend + AppService binding"
            } else {
                "Step 2 of 2 · Find the Matrix friend"
            };
            self.view.label(cx, ids!(sheet_subtitle)).set_text(cx, sub);
            self.view.label(cx, ids!(step2_heading)).set_text(cx, &format!("New {fw} agent"));
            self.sync_step2(cx);
        } else {
            self.view.label(cx, ids!(sheet_title)).set_text(cx, "Add an agent");
            self.view.label(cx, ids!(sheet_subtitle)).set_text(cx, "Step 1 of 2 · Choose a framework");
        }
        self.sync_primary_button(cx);
        self.view.redraw(cx);
    }

    fn sync_step2(&mut self, cx: &mut Cx) {
        let added = self.friend_state == FriendState::Added;
        self.view.button(cx, ids!(add_friend_button)).set_visible(cx, !added);
        self.view.view(cx, ids!(friend_added_strip)).set_visible(cx, added);
        self.view.view(cx, ids!(octos_section)).set_visible(cx, self.is_octos());
        // The Matrix ID field locks once a friend request is in flight / done.
        self.view.text_input(cx, ids!(id_field.field_input)).set_is_read_only(cx, self.friend_state != FriendState::Idle);

        // Octos AppService section is GATED until the friend is added (design
        // handoff §"Octos gating"): the BotFather / URL inputs and the Check now
        // button are disabled (and render greyed) until the agent is a friend.
        if self.is_octos() {
            self.view.text_input(cx, ids!(octos_section.botfather_field.field_input)).set_is_read_only(cx, !added);
            self.view.text_input(cx, ids!(octos_section.octos_url_field.field_input)).set_is_read_only(cx, !added);
            self.view.button(cx, ids!(octos_section.octos_check_row.check_now_button)).set_enabled(cx, added);
        }
        self.sync_octos_status(cx);
        self.sync_primary_button(cx);
        self.view.redraw(cx);
    }

    fn sync_octos_status(&mut self, cx: &mut Cx) {
        let text = match self.octos_health.status {
            OctosHealthStatus::Unknown => "Unknown",
            OctosHealthStatus::Checking => "Checking",
            OctosHealthStatus::Reachable => "Online",
            OctosHealthStatus::Unreachable => "Offline",
        };
        self.view.label(cx, ids!(octos_section.octos_check_row.octos_status_pill.octos_status_label)).set_text(cx, text);
        // Check now is enabled only after the friend is added (gating) and when
        // no probe is in flight.
        let check_enabled = self.friend_state == FriendState::Added && !self.octos_health.in_flight;
        self.view.button(cx, ids!(octos_section.octos_check_row.check_now_button))
            .set_enabled(cx, check_enabled);
        let offline = self.octos_health.status == OctosHealthStatus::Unreachable;
        let err = self.view.label(cx, ids!(octos_section.octos_error_label));
        if offline {
            let url = self.view.text_input(cx, ids!(octos_section.octos_url_field.field_input)).text();
            err.set_text(cx, &format!("No response from {}. Start the local Octos service, then re-check.", url.trim()));
        }
        err.set_visible(cx, offline);
        self.sync_primary_button(cx);
        self.view.redraw(cx);
    }

    fn sync_primary_button(&mut self, cx: &mut Cx) {
        let (text, enabled) = if self.step == 1 {
            ("Continue".to_string(), self.selected_framework.is_some())
        } else {
            let enabled = self.can_finish();
            let text = if enabled {
                "Finish & register"
            } else if self.friend_state != FriendState::Added {
                "Add the agent above to continue"
            } else {
                "Service must be online"
            };
            (text.to_string(), enabled)
        };
        let button = self.view.button(cx, ids!(primary_button));
        button.set_text(cx, &text);
        // Keep the button always click-enabled and gate the ACTION in the handler
        // (`selected_framework.is_some()` / `can_finish()`) instead of disabling the
        // widget. Disabling defers the clickable hit-area to the next draw, so a fast
        // tap right after selecting a framework could be swallowed — this made
        // "Continue" feel dead. The button still *looks* disabled (grey) below.
        button.set_enabled(cx, true);
        // Visually reflect disabled state (teal when actionable, grey when not).
        let mut button = self.view.button(cx, ids!(primary_button));
        if enabled {
            script_apply_eval!(cx, button, { draw_bg +: { color: mod.widgets.RBX_ACCENT } draw_text +: { color: mod.widgets.RBX_FG_ON_ACCENT } });
        } else {
            script_apply_eval!(cx, button, { draw_bg +: { color: mod.widgets.RBX_BG_DISABLED } draw_text +: { color: mod.widgets.RBX_FG_DISABLED } });
        }
    }

    pub fn show(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        self.app_language = app_language;
        self.step = 1;
        self.selected_framework = None;
        self.friend_state = FriendState::Idle;
        self.target_user_id = None;
        self.octos_health = OctosHealthState::default();

        self.view.text_input(cx, ids!(id_field.field_input)).set_text(cx, "");
        self.view.text_input(cx, ids!(id_field.field_input)).set_is_read_only(cx, false);
        self.view.text_input(cx, ids!(octos_section.botfather_field.field_input)).set_text(cx, "");
        self.view.text_input(cx, ids!(octos_section.octos_url_field.field_input)).set_text(cx, "");
        self.set_add_friend_label(cx, "＋ Add friend & bind");
        self.populate_framework_cards(cx);
        self.update_framework_cards(cx);
        self.sync_steps(cx);
        self.view.redraw(cx);
    }
}

impl AddAgentModalRef {
    pub fn show(&self, cx: &mut Cx, app_language: AppLanguage) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show(cx, app_language);
    }
}
