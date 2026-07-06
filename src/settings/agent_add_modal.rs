//! The "Add an agent" bottom-sheet modal (Agent Registry design handoff).
//!
//! Two-step flow:
//!   Step 1 — choose a framework (Hermes / OpenClaw = direct friend agents,
//!            Octos = AppService agent).
//!   Step 2 — enter the agent's Matrix ID and bind it by creating/reusing the
//!            DM (`OpenOrCreateDirectMessage`). Octos adds a compact AppService
//!            URL check/save block before the same Matrix ID binding.
//!
//! A successful bind writes the agent into the global `AgentRegistry`.
//!
//! NOTE: inside the `script_mod!` block, only `//` comments are allowed.

use makepad_widgets::*;
use ruma::OwnedUserId;

use crate::{
    app::{AgentFramework, AppState, BotSettingsState},
    i18n::{AppLanguage, tr_key},
    persistence,
    profile::user_profile::UserProfile,
    shared::avatar::AvatarState,
    settings::{
        agent_settings::{framework_label, framework_mono, parse_agent_user_id, register_agent_from_search},
        bot_settings::{OctosHealthState, OctosHealthStatus},
    },
    sliding_sync::{
        DirectMessageRoomAction, MatrixRequest, current_user_id, submit_async_request,
    },
};

const AGENT_OCTOS_HEALTH_REQUEST_ID: LiveId = live_id!(agent_add_octos_health);

/// The sheet's resting bottom margin (lifts it clear of the nav). Must match the
/// `margin: Inset{bottom: ...}` on the `sheet` view; keyboard avoidance overrides
/// it with the keyboard height while the soft keyboard is shown.
const SHEET_BASE_BOTTOM_MARGIN: f64 = 96.0;

/// The sheet's fixed height (layout points). Must match `height:` on the `sheet`
/// view. Used to cap the keyboard lift so the sheet's top never rises above
/// `SHEET_TOP_INSET` (header stays on-screen).
const SHEET_HEIGHT: f64 = 592.0;

/// Minimum gap kept between the window top (status bar) and the sheet top when
/// the keyboard lift is at its maximum.
const SHEET_TOP_INSET: f64 = 24.0;

pub fn register_agent_with_modal_settings(
    app_state: &mut AppState,
    user_id: OwnedUserId,
    display_name: Option<String>,
    framework: AgentFramework,
    octos_service_url: Option<String>,
) -> bool {
    let added = register_agent_from_search(app_state, user_id.clone(), display_name, framework);

    if framework != AgentFramework::Octos {
        return added;
    }

    app_state.bot_settings.enabled = true;
    let current_botfather = app_state.bot_settings.botfather_user_id.trim();
    let should_claim_botfather =
        current_botfather == BotSettingsState::DEFAULT_BOTFATHER_LOCALPART
        || current_botfather == user_id.as_str();

    if should_claim_botfather {
        let url = octos_service_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(BotSettingsState::DEFAULT_OCTOS_SERVICE_URL)
            .to_string();
        app_state.bot_settings.botfather_user_id = user_id.as_str().to_string();
        app_state.bot_settings.octos_service_url = url;
    }
    app_state.bot_settings.record_known_bot_user_ids([user_id]);

    added
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum FriendState {
    #[default]
    Idle,
    Pending,
    Added,
}

fn bind_button_state(
    friend_state: FriendState,
    is_octos: bool,
    _octos_status: OctosHealthStatus,
    agent_id_raw: &str,
) -> (&'static str, bool) {
    let valid_matrix_id = parse_agent_user_id(agent_id_raw).is_ok();

    match friend_state {
        FriendState::Idle if !valid_matrix_id => ("Enter Matrix ID", false),
        FriendState::Idle if is_octos => ("Bind Octos agent", true),
        FriendState::Idle => ("Bind agent", true),
        FriendState::Pending => ("Sending friend request...", false),
        FriendState::Added => ("Bound", false),
    }
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // Back chevron icon (left arrow) — drawn as SVG, not a text glyph, so it
    // never renders as a missing-glyph box in the app font.
    mod.widgets.AGENT_ICON_BACK = crate_resource("self://resources/icons/chevron_left.svg")

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
            spacing: 12
            new_batch: true
            padding: Inset{left: 14, right: 14, top: 8, bottom: 8}
            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_XXS)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
            }

            card_tile := RoundedView {
                width: 48
                height: 48
                align: Align{x: 0.5, y: 0.5}
                new_batch: true
                show_bg: true
                draw_bg +: {
                    color: (RBX_BG_SURFACE_SUBTLE)
                    border_radius: (RBX_RADIUS_XXS)
                }
                card_mono := Label {
                    width: Fit
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: TITLE_TEXT { font_size: 15.0 }
                    }
                    text: ""
                }
            }

            card_col := View {
                width: Fill
                height: Fit
                flow: Down
                spacing: 2
                card_name := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: TITLE_TEXT { font_size: 13.0 }
                    }
                    text: ""
                }
                // Tag sits on its own line (Fit-width pill) so it never clips,
                // however long the agent name is.
                card_tag := RoundedView {
                    width: Fit
                    height: Fit
                    padding: Inset{left: 6, right: 6, top: 1, bottom: 1}
                    new_batch: true
                    show_bg: true
                    draw_bg +: {
                        color: (RBX_BG_SURFACE_SUBTLE)
                        border_radius: (RBX_RADIUS_XXS)
                    }
                    card_tag_label := Label {
                        width: Fit
                        height: Fit
                        draw_text +: {
                            color: (RBX_FG_SECONDARY)
                            text_style: RBX_TEXT_BADGE { font_size: 7.5 }
                        }
                        text: ""
                    }
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
                new_batch: true
                show_bg: true
                draw_bg +: {
                    color: (RBX_BG_SURFACE)
                    border_radius: (RBX_RADIUS_PILL)
                    border_size: 2.0
                    border_color: (RBX_STROKE_STRONG)
                }
                card_radio_check := View {
                    width: Fit
                    height: Fit
                    visible: false
                    Icon {
                        width: 12
                        height: 12
                        draw_icon +: {
                            svg: (ICON_CHECKMARK)
                            color: (RBX_FG_ON_ACCENT)
                        }
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
                color: (RBX_TRANSPARENT)
                color_hover: (RBX_HIT_HOVER)
                color_down: (RBX_HIT_DOWN)
                    border_radius: (RBX_RADIUS_XXS)
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
                border_radius: (RBX_RADIUS_XXS)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
            }
            // Real input is dark (PRIMARY); the placeholder/hint is a light grey
            // (TERTIARY) so it reads clearly as a hint, not an entered value.
            draw_text +: {
                color: (RBX_FG_PRIMARY)
                color_empty: (RBX_FG_TERTIARY)
                color_empty_hover: (RBX_FG_TERTIARY)
                color_empty_focus: (RBX_FG_TERTIARY)
            }
        }
    }

    mod.widgets.AddAgentModal = #(AddAgentModal::register_widget(vm)) {
        // Fill/Fit root: the hosting Modal content provides a real width and
        // bottom alignment. The sheet is capped below, so narrow phones get
        // side padding instead of horizontal overflow.
        width: Fill
        height: Fit
        flow: Down
        align: Align{x: 0.5, y: 1.0}
        padding: Inset{left: 8, right: 8}

        sheet := RoundedView {
            width: Fill{min: 0 max: 352}
            height: 592
            flow: Down
            new_batch: true
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
            // View handles child events first, then its own hit-test. Keep the
            // sheet from taking keyboard focus back after a TextInput receives it.
            grab_key_focus: false
            capture_overload: true
            cursor: MouseCursor.Default
            padding: Inset{left: 16, right: 16, top: 10, bottom: 16}
            spacing: 0
            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_XXS)
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
                flow: Down
                spacing: 1
                margin: Inset{bottom: 10}

                // Back ‹ · title · close ✕ all share one baseline (the subtitle
                // drops to its own line below) — keeps the header two lines tall,
                // same as before, but the nav icons sit on the title line.
                header_top := View {
                    width: Fill
                    height: Fit
                    flow: Right
                    align: Align{y: 0.5}
                    spacing: 8
                    back_button := RobrixNeutralIconButton {
                        width: Fit
                        height: Fit
                        visible: false
                        padding: Inset{left: 2, right: 6, top: 4, bottom: 4}
                        spacing: 0
                        text: ""
                        draw_icon.svg: (mod.widgets.AGENT_ICON_BACK)
                        draw_icon.color: (RBX_FG_SECONDARY)
                        icon_walk: Walk{width: 18, height: 18}
                        draw_bg +: { color: (RBX_TRANSPARENT), color_hover: (RBX_TRANSPARENT), color_down: (RBX_TRANSPARENT) }
                    }
                    sheet_title := Label {
                        width: Fill
                        height: Fit
                        draw_text +: {
                            color: (RBX_FG_PRIMARY)
                            text_style: RBX_TEXT_PAGE_TITLE {}
                        }
                        text: "Add an agent"
                    }
                    close_button := RobrixNeutralIconButton {
                        width: Fit
                        height: Fit
                        padding: Inset{left: 6, right: 6, top: 4, bottom: 4}
                        spacing: 0
                        text: ""
                        draw_icon.svg: (ICON_CLOSE)
                        draw_icon.color: (RBX_FG_TERTIARY)
                        icon_walk: Walk{width: 16, height: 16}
                        draw_bg +: { color: (RBX_TRANSPARENT), color_hover: (RBX_TRANSPARENT), color_down: (RBX_TRANSPARENT) }
                    }
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

            body_scroll := ScrollYView {
                width: Fill
                height: 382
                flow: Down
                spacing: 0

                // ---------- STEP 1: framework picker ----------
                step1_view := View {
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 6

                    step1_intro := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 1}
                        draw_text +: {
                            color: (RBX_FG_SECONDARY)
                            text_style: RBX_TEXT_BODY {}
                        }
                        text: "Pick the agent framework."
                    }

                    // Per-framework visuals (mono, name, tag, blurb, colors) are
                    // populated from Rust in `populate_framework_cards` to avoid
                    // deep DSL overrides (unreliable in this Makepad fork).
                    hermes_card := FrameworkCard {}
                    openclaw_card := FrameworkCard {}
                    octos_card := FrameworkCard {}
                    octos_direct_card := FrameworkCard {}
                }

                // ---------- STEP 2: detect/bind ----------
                step2_view := View {
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 8
                    visible: false

                    step2_agent_header := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        align: Align{y: 0.5}
                        spacing: 9

                        step2_framework_tile := RoundedView {
                            width: 26
                            height: 26
                            align: Align{x: 0.5, y: 0.5}
                            new_batch: true
                            show_bg: true
                            draw_bg +: {
                                color: (RBX_BG_SURFACE_SUBTLE)
                                border_radius: (RBX_RADIUS_XXS)
                            }
                            step2_framework_mono := Label {
                                width: Fit
                                height: Fit
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    text_style: TITLE_TEXT { font_size: 10.5 }
                                }
                                text: ""
                            }
                        }
                        step2_heading := Label {
                            width: Fill
                            height: Fit
                            draw_text +: {
                                color: (RBX_FG_PRIMARY)
                                text_style: RBX_TEXT_BODY_STRONG {}
                            }
                            text: "New agent"
                        }
                    }

                    octos_section := RoundedView {
                        width: Fill
                        height: Fit
                        visible: false
                        flow: Down
                        new_batch: true
                        spacing: 7
                        margin: Inset{bottom: 4}
                        padding: Inset{left: 12, right: 12, top: 11, bottom: 12}
                        show_bg: true
                        draw_bg +: {
                            color: (RBX_FW_OCTOS_BG)
                            border_radius: (RBX_RADIUS_XXS)
                            border_size: 1.0
                            border_color: (RBX_STROKE_SOFT)
                        }

                        octos_heading := Label {
                            width: Fill
                            height: Fit
                            draw_text +: {
                                color: (RBX_FW_OCTOS_FG)
                                text_style: RBX_TEXT_BODY_STRONG {}
                            }
                            text: "Octos AppService"
                        }
                        octos_url_field := AgentField {
                            field_label.text: "AppService URL"
                            field_input.empty_text: "http://127.0.0.1:8010"
                        }
                        octos_check_row := View {
                            width: Fill
                            height: Fit
                            flow: Right
                            align: Align{y: 0.5}
                            spacing: 8
                            margin: Inset{top: 1}
                            check_now_button := RobrixIconButton {
                                width: Fit
                                height: Fit
                                padding: Inset{top: 7, bottom: 7, left: 12, right: 12}
                                icon_walk: Walk{width: 0, height: 0}
                                spacing: 0
                                text: "Check"
                                draw_bg +: {
                                    color: (RBX_BG_SURFACE)
                                    color_hover: (RBX_BG_HOVER)
                                    color_down: (RBX_BG_PRESSED)
                                    border_radius: (RBX_RADIUS_XXS)
                                    border_size: 1.0
                                    border_color: (RBX_FW_OCTOS_FG)
                                }
                                draw_text +: { color: (RBX_FW_OCTOS_FG), color_hover: (RBX_FW_OCTOS_FG), color_down: (RBX_FW_OCTOS_FG) }
                            }
                            octos_check_spinner := LoadingSpinner {
                                visible: false
                                width: 16
                                height: 16
                                draw_bg +: {
                                    color: (RBX_FW_OCTOS_FG)
                                    border_size: 2.0
                                }
                            }
                            octos_status_pill := RoundedView {
                                width: Fit
                                height: Fit
                                padding: Inset{left: 10, right: 10, top: 5, bottom: 5}
                                new_batch: true
                                show_bg: true
                                draw_bg +: {
                                    color: (RBX_NEUTRAL_BG)
                                    border_radius: (RBX_RADIUS_XXS)
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
                            margin: Inset{top: 1}
                            draw_text +: {
                                color: (RBX_DANGER_FG)
                                text_style: RBX_TEXT_META {}
                            }
                            text: ""
                        }
                        save_appservice_button := RobrixIconButton {
                            width: Fill
                            height: (RBX_CONTROL_H_MD)
                            align: Align{x: 0.5, y: 0.5}
                            margin: Inset{top: 2}
                            padding: Inset{top: 8, bottom: 8, left: 12, right: 12}
                            icon_walk: Walk{width: 0, height: 0}
                            spacing: 0
                            text: "Save AppService"
                            draw_bg +: {
                                color: (RBX_ACCENT)
                                color_hover: (RBX_ACCENT_HOVER)
                                color_down: (RBX_ACCENT_PRESSED)
                                border_radius: (RBX_RADIUS_XXS)
                            }
                            draw_text +: { color: (RBX_FG_ON_ACCENT), color_hover: (RBX_FG_ON_ACCENT), color_down: (RBX_FG_ON_ACCENT) }
                        }
                    }

                    id_field := AgentField {
                        field_label.text: "Agent Matrix ID"
                        field_input.empty_text: "@agent:server or agent:server"
                    }

                    add_friend_button := RobrixIconButton {
                        width: Fill
                        height: (RBX_CONTROL_H_MD)
                        align: Align{x: 0.5, y: 0.5}
                        margin: Inset{top: 2}
                        padding: Inset{top: 8, bottom: 8, left: 14, right: 14}
                        draw_icon.svg: (ICON_ADD_USER)
                        draw_icon.color: (RBX_FG_ON_ACCENT)
                        icon_walk: Walk{width: 16, height: 16, margin: Inset{right: 7}}
                        spacing: 0
                        text: "Add friend & bind"
                        draw_bg +: {
                            color: (RBX_ACCENT)
                            color_hover: (RBX_ACCENT_HOVER)
                            color_down: (RBX_ACCENT_PRESSED)
                            border_radius: (RBX_RADIUS_XXS)
                        }
                        draw_text +: { color: (RBX_FG_ON_ACCENT), color_hover: (RBX_FG_ON_ACCENT), color_down: (RBX_FG_ON_ACCENT) }
                    }

                    bind_progress_row := View {
                        visible: false
                        width: Fill
                        height: Fit
                        flow: Right
                        align: Align{x: 0.5, y: 0.5}
                        spacing: 7
                        margin: Inset{top: 2}

                        bind_progress_spinner := LoadingSpinner {
                            width: 16
                            height: 16
                            draw_bg +: {
                                color: (RBX_ACCENT)
                                border_size: 2.0
                            }
                        }
                        bind_progress_label := Label {
                            width: Fit
                            height: Fit
                            draw_text +: {
                                color: (RBX_FG_SECONDARY)
                                text_style: RBX_TEXT_META {}
                            }
                            text: "Sending friend request..."
                        }
                    }

                    friend_added_strip := RoundedView {
                        width: Fill
                        height: Fit
                        visible: false
                        flow: Right
                        align: Align{y: 0.5}
                        new_batch: true
                        spacing: 9
                        margin: Inset{top: 2}
                        padding: Inset{left: 12, right: 12, top: 10, bottom: 10}
                        show_bg: true
                        draw_bg +: {
                            color: (RBX_SUCCESS_BG)
                            border_radius: (RBX_RADIUS_XXS)
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
                draw_bg +: { color: (RBX_TRANSPARENT) }

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
                    align: Align{x: 0.5, y: 0.5}
                    padding: Inset{top: 11, bottom: 11, left: 16, right: 16}
                    icon_walk: Walk{width: 0, height: 0}
                    spacing: 0
                    text: "Continue"
                    draw_bg +: {
                        color: (RBX_ACCENT)
                        color_hover: (RBX_ACCENT_HOVER)
                        color_down: (RBX_ACCENT_PRESSED)
                        border_radius: (RBX_RADIUS_XXS)
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
    #[rust]
    octos_probe_base_url: Option<String>,
    /// On-screen keyboard occlusion height (Makepad layout points), 0 when hidden.
    /// Read from `VirtualKeyboardEvent` — NOT from `cx.keyboard_shift`, which is
    /// derived from the focused IME position and would feed back into a jump loop
    /// once we move the sheet (this modal draws in its own overlay pass, outside
    /// the window KeyboardView's content shift).
    #[rust]
    keyboard_height: f64,
}

impl Widget for AddAgentModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if let Event::NetworkResponses(responses) = event {
            for response in responses {
                match response {
                    NetworkResponse::HttpResponse { request_id, response }
                        if *request_id == AGENT_OCTOS_HEALTH_REQUEST_ID =>
                    {
                        let Some(probe_base_url) = self.octos_probe_base_url.clone() else { continue };
                        if let Some(fallback) = self.octos_health.handle_http_result(&probe_base_url, response.status_code) {
                            self.send_health_request(cx, &fallback);
                        } else if !self.octos_health.in_flight {
                            self.octos_probe_base_url = None;
                        }
                        self.sync_octos_status(cx);
                    }
                    NetworkResponse::HttpError { request_id, .. }
                        if *request_id == AGENT_OCTOS_HEALTH_REQUEST_ID =>
                    {
                        let Some(probe_base_url) = self.octos_probe_base_url.clone() else { continue };
                        if let Some(fallback) = self.octos_health.handle_transport_error(&probe_base_url) {
                            self.send_health_request(cx, &fallback);
                        } else if !self.octos_health.in_flight {
                            self.octos_probe_base_url = None;
                        }
                        self.sync_octos_status(cx);
                    }
                    _ => {}
                }
            }
        }
        // Track the on-screen keyboard height so `draw_walk` can lift the sheet
        // clear of it. Driven purely by the keyboard's own show/hide events, so
        // moving the sheet never changes this value (no jump loop).
        if let Event::VirtualKeyboard(vk) = event {
            let new_height = match vk {
                VirtualKeyboardEvent::WillShow { height, .. }
                | VirtualKeyboardEvent::DidShow { height, .. } => *height,
                VirtualKeyboardEvent::WillHide { .. } | VirtualKeyboardEvent::DidHide { .. } => 0.0,
            };
            if (new_height - self.keyboard_height).abs() > 0.5 {
                self.keyboard_height = new_height;
                self.redraw(cx);
            }
        }
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // Keyboard avoidance: raise the bottom-anchored sheet so its lower
        // controls (Matrix ID field, Add friend button) stay above the on-screen
        // keyboard. When hidden, fall back to the resting margin that clears the
        // nav bar. The height comes from VirtualKeyboardEvent, so this never
        // ping-pongs the way reading `cx.keyboard_shift` did.
        //
        // Cap the lift so the sheet top never rises above SHEET_TOP_INSET: the
        // sheet (SHEET_HEIGHT) is taller than the gap above the keyboard, so a
        // full-height lift would push the header off-screen. Clamping pins the
        // header just below the status bar; the sheet's own bottom padding then
        // absorbs the keyboard and lands the Matrix ID field just above it.
        let margin_bottom = if self.keyboard_height > 0.0 {
            let window_h = cx.current_pass_size().y;
            let max_lift = (window_h - SHEET_HEIGHT - SHEET_TOP_INSET).max(SHEET_BASE_BOTTOM_MARGIN);
            (self.keyboard_height + 8.0).min(max_lift)
        } else {
            SHEET_BASE_BOTTOM_MARGIN
        };
        if let Some(mut sheet) = self.view.view(cx, ids!(sheet)).borrow_mut() {
            sheet.walk.margin.bottom = margin_bottom;
        }
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
                (AgentFramework::Hermes, ids!(hermes_card.card_click)),
                (AgentFramework::OpenClaw, ids!(openclaw_card.card_click)),
                (AgentFramework::Octos, ids!(octos_card.card_click)),
                (AgentFramework::OctosDirect, ids!(octos_direct_card.card_click)),
            ];
            for (framework, click_id) in cards {
                if self.view.button(cx, click_id).clicked(actions) {
                    self.selected_framework = Some(framework);
                    self.update_framework_cards(cx);
                    self.sync_primary_button(cx);
                }
            }
        }

        if self.step == 2 {
            let id_changed = self.view.text_input(cx, ids!(id_field.field_input)).changed(actions).is_some();
            let octos_url_changed = self.is_octos()
                && self.view.text_input(cx, ids!(octos_section.octos_url_field.field_input)).changed(actions).is_some();
            if octos_url_changed {
                self.octos_health = OctosHealthState::default();
                self.octos_probe_base_url = None;
                self.view.button(cx, ids!(octos_section.save_appservice_button))
                    .set_text(cx, "Save AppService");
            }
            if id_changed || octos_url_changed {
                self.sync_step2(cx);
            }
        }

        // Primary footer button: Continue on step 1. Step 2 uses the bind button.
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
            if self.friend_state != FriendState::Idle {
                return;
            }
            self.add_friend(cx);
            return;
        }

        // Step 2 (Octos): check service health.
        if self.step == 2 && self.view.button(cx, ids!(check_now_button)).clicked(actions) {
            let url = self.octos_service_url(cx);
            if let Some(probe) = self.octos_health.begin_check(&url) {
                self.octos_probe_base_url = Some(url);
                self.sync_octos_status(cx);
                self.send_health_request(cx, &probe);
            }
            return;
        }

        // Step 2 (Octos): save the AppService URL independently from friend binding.
        if self.step == 2 && self.is_octos()
            && self.view.button(cx, ids!(save_appservice_button)).clicked(actions)
        {
            self.save_octos_appservice_settings(cx, scope);
            return;
        }

        // Friend-request (DM) result.
        if self.friend_state == FriendState::Pending {
            for action in actions {
                match action.downcast_ref() {
                    Some(DirectMessageRoomAction::NewlyCreated { user_profile, .. })
                        if self.matches_target(&user_profile.user_id) =>
                    {
                        self.on_friend_added(cx, scope);
                    }
                    Some(DirectMessageRoomAction::FoundExisting { user_id, .. })
                        if self.matches_target(user_id) =>
                    {
                        self.on_friend_added(cx, scope);
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

    fn is_waiting_for_direct_message_result(&self, user_id: &OwnedUserId) -> bool {
        // The widget tree may process the DM result before App's global handler,
        // so keep ownership through Added until the modal close action is handled.
        self.friend_state != FriendState::Idle && self.matches_target(user_id)
    }

    fn is_octos(&self) -> bool {
        self.selected_framework == Some(AgentFramework::Octos)
    }

    fn can_finish(&self) -> bool {
        self.friend_state == FriendState::Added
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
                self.set_add_friend_label(cx, "Sending friend request...");
                self.sync_step2(cx);
            }
            Err(error) => {
                self.set_add_friend_label(cx, &error);
                self.view.redraw(cx);
            }
        }
    }

    fn on_friend_added(&mut self, cx: &mut Cx, scope: &mut Scope) {
        self.friend_state = FriendState::Added;
        let uid = self.target_user_id.as_ref().map(|u| u.as_str().to_string()).unwrap_or_default();
        let framework = self.selected_framework.map(framework_label).unwrap_or("");
        self.view.label(cx, ids!(friend_added_strip.friend_added_label))
            .set_text(cx, &format!("Friend request sent. {uid} is bound to {framework}."));
        self.sync_step2(cx);
        if self.can_finish() {
            self.finish_register(cx, scope);
        }
    }

    fn finish_register(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let Some(framework) = self.selected_framework else { return };
        let Some(user_id) = self.target_user_id.clone() else { return };
        let display_name = user_id.localpart().to_string();

        let Some(app_state) = scope.data.get_mut::<AppState>() else { return };
        let url = self.octos_service_url(cx);
        register_agent_with_modal_settings(
            app_state,
            user_id.clone(),
            Some(display_name.clone()),
            framework,
            Some(url),
        );

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

    fn save_octos_appservice_settings(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let url = self.octos_service_url(cx);
        let err = self.view.label(cx, ids!(octos_section.octos_error_label));
        if let Err(error) = BotSettingsState::validate_octos_service_url(&url) {
            err.set_text(cx, &error);
            err.set_visible(cx, true);
            self.view.button(cx, ids!(octos_section.save_appservice_button))
                .set_text(cx, "Save AppService");
            self.view.redraw(cx);
            return;
        }

        let Some(app_state) = scope.data.get_mut::<AppState>() else { return };
        app_state.bot_settings.enabled = true;
        app_state.bot_settings.octos_service_url = url.clone();
        if let Some(account_user_id) = current_user_id() {
            if let Err(e) = persistence::save_app_state(app_state.clone(), account_user_id) {
                error!("Failed to persist Octos AppService URL. Error: {e}");
            }
        }

        self.view.text_input(cx, ids!(octos_section.octos_url_field.field_input)).set_text(cx, &url);
        err.set_visible(cx, false);
        self.view.button(cx, ids!(octos_section.save_appservice_button))
            .set_text(cx, "Saved");
        if self.friend_state == FriendState::Idle {
            let id_input = self.view.text_input(cx, ids!(id_field.field_input));
            id_input.set_is_read_only(cx, false);
            id_input.set_key_focus(cx);
        }
        self.sync_bind_button(cx);
        self.view.redraw(cx);
    }

    fn set_add_friend_label(&mut self, cx: &mut Cx, text: &str) {
        self.view.button(cx, ids!(add_friend_button)).set_text(cx, text);
    }

    fn sync_bind_button(&mut self, cx: &mut Cx) {
        let agent_id_raw = self.view.text_input(cx, ids!(id_field.field_input)).text();
        let (text, enabled) = bind_button_state(
            self.friend_state,
            self.is_octos(),
            self.octos_health.status,
            &agent_id_raw,
        );

        let mut button = self.view.button(cx, ids!(add_friend_button));
        button.set_text(cx, text);
        // Keep the control hit-testable on Android; disabled RobrixIconButton
        // variants can draw as effectively invisible. The action is still gated
        // by `friend_state` and Matrix ID parsing in `add_friend`.
        button.set_enabled(cx, true);
        self.view.view(cx, ids!(bind_progress_row))
            .set_visible(cx, self.friend_state == FriendState::Pending);
        if enabled {
            script_apply_eval!(cx, button, {
                draw_bg +: {
                    color: mod.widgets.RBX_ACCENT,
                    color_hover: mod.widgets.RBX_ACCENT_HOVER,
                    color_down: mod.widgets.RBX_ACCENT_PRESSED,
                }
                draw_text +: { color: mod.widgets.RBX_FG_ON_ACCENT }
                draw_icon +: { color: mod.widgets.RBX_FG_ON_ACCENT }
            });
        } else {
            script_apply_eval!(cx, button, {
                draw_bg +: {
                    color: mod.widgets.RBX_BG_DISABLED,
                    color_hover: mod.widgets.RBX_BG_DISABLED,
                    color_down: mod.widgets.RBX_BG_DISABLED,
                }
                draw_text +: { color: mod.widgets.RBX_FG_DISABLED }
                draw_icon +: { color: mod.widgets.RBX_FG_DISABLED }
            });
        }
    }

    fn populate_framework_cards(&mut self, cx: &mut Cx) {
        // Text content.
        let text = |key| tr_key(self.app_language, key);
        self.view.label(cx, ids!(octos_card.card_body.card_tile.card_mono)).set_text(cx, "Oc");
        self.view.label(cx, ids!(octos_card.card_body.card_col.card_name)).set_text(cx, "Octos");
        self.view.label(cx, ids!(octos_card.card_body.card_col.card_tag.card_tag_label)).set_text(cx, "APPSERVICE");
        self.view.label(cx, ids!(octos_card.card_body.card_col.card_blurb)).set_text(cx, "Friend plus local AppService.");
        self.view.label(cx, ids!(octos_direct_card.card_body.card_tile.card_mono))
            .set_text(cx, text("settings.labs.agents.framework.octos_direct.mono"));
        self.view.label(cx, ids!(octos_direct_card.card_body.card_col.card_name))
            .set_text(cx, text("settings.labs.agents.framework.octos_direct.name"));
        self.view.label(cx, ids!(octos_direct_card.card_body.card_col.card_tag.card_tag_label))
            .set_text(cx, text("settings.labs.agents.framework.octos_direct.tag"));
        self.view.label(cx, ids!(octos_direct_card.card_body.card_col.card_blurb))
            .set_text(cx, text("settings.labs.agents.framework.octos_direct.blurb"));
        self.view.label(cx, ids!(hermes_card.card_body.card_tile.card_mono)).set_text(cx, "He");
        self.view.label(cx, ids!(hermes_card.card_body.card_col.card_name)).set_text(cx, "Hermes");
        self.view.label(cx, ids!(hermes_card.card_body.card_col.card_tag.card_tag_label)).set_text(cx, "DIRECT AGENT");
        self.view.label(cx, ids!(hermes_card.card_body.card_col.card_blurb)).set_text(cx, "Registered as a Matrix friend.");
        self.view.label(cx, ids!(openclaw_card.card_body.card_tile.card_mono)).set_text(cx, "Cl");
        self.view.label(cx, ids!(openclaw_card.card_body.card_col.card_name)).set_text(cx, "OpenClaw");
        self.view.label(cx, ids!(openclaw_card.card_body.card_col.card_tag.card_tag_label)).set_text(cx, "DIRECT AGENT");
        self.view.label(cx, ids!(openclaw_card.card_body.card_col.card_blurb)).set_text(cx, "Registered as a Matrix friend.");

        // Per-framework colors (tile fill + mono text + tag text).
        let mut octos_tile = self.view.view(cx, ids!(octos_card.card_body.card_tile));
        script_apply_eval!(cx, octos_tile, { draw_bg +: { color: mod.widgets.RBX_FW_OCTOS_BG } });
        let mut octos_mono = self.view.label(cx, ids!(octos_card.card_body.card_tile.card_mono));
        script_apply_eval!(cx, octos_mono, { draw_text +: { color: mod.widgets.RBX_FW_OCTOS_FG } });
        let mut octos_tag = self.view.label(cx, ids!(octos_card.card_body.card_col.card_tag.card_tag_label));
        script_apply_eval!(cx, octos_tag, { draw_text +: { color: mod.widgets.RBX_FW_OCTOS_FG } });
        let mut octos_tag_pill = self.view.view(cx, ids!(octos_card.card_body.card_col.card_tag));
        script_apply_eval!(cx, octos_tag_pill, { draw_bg +: { color: mod.widgets.RBX_FW_OCTOS_BG } });

        // OctosDirect reuses the Octos framework palette (it is an Octos agent,
        // just added directly instead of via App Service).
        let mut octos_direct_tile = self.view.view(cx, ids!(octos_direct_card.card_body.card_tile));
        script_apply_eval!(cx, octos_direct_tile, { draw_bg +: { color: mod.widgets.RBX_FW_OCTOS_BG } });
        let mut octos_direct_mono = self.view.label(cx, ids!(octos_direct_card.card_body.card_tile.card_mono));
        script_apply_eval!(cx, octos_direct_mono, { draw_text +: { color: mod.widgets.RBX_FW_OCTOS_FG } });
        let mut octos_direct_tag = self.view.label(cx, ids!(octos_direct_card.card_body.card_col.card_tag.card_tag_label));
        script_apply_eval!(cx, octos_direct_tag, { draw_text +: { color: mod.widgets.RBX_FW_OCTOS_FG } });
        let mut octos_direct_tag_pill = self.view.view(cx, ids!(octos_direct_card.card_body.card_col.card_tag));
        script_apply_eval!(cx, octos_direct_tag_pill, { draw_bg +: { color: mod.widgets.RBX_FW_OCTOS_BG } });

        let mut hermes_tile = self.view.view(cx, ids!(hermes_card.card_body.card_tile));
        script_apply_eval!(cx, hermes_tile, { draw_bg +: { color: mod.widgets.RBX_FW_HERMES_BG } });
        let mut hermes_mono = self.view.label(cx, ids!(hermes_card.card_body.card_tile.card_mono));
        script_apply_eval!(cx, hermes_mono, { draw_text +: { color: mod.widgets.RBX_FW_HERMES_FG } });
        let mut hermes_tag = self.view.label(cx, ids!(hermes_card.card_body.card_col.card_tag.card_tag_label));
        script_apply_eval!(cx, hermes_tag, { draw_text +: { color: mod.widgets.RBX_FW_HERMES_FG } });
        let mut hermes_tag_pill = self.view.view(cx, ids!(hermes_card.card_body.card_col.card_tag));
        script_apply_eval!(cx, hermes_tag_pill, { draw_bg +: { color: mod.widgets.RBX_FW_HERMES_BG } });

        let mut openclaw_tile = self.view.view(cx, ids!(openclaw_card.card_body.card_tile));
        script_apply_eval!(cx, openclaw_tile, { draw_bg +: { color: mod.widgets.RBX_FW_OPENCLAW_BG } });
        let mut openclaw_mono = self.view.label(cx, ids!(openclaw_card.card_body.card_tile.card_mono));
        script_apply_eval!(cx, openclaw_mono, { draw_text +: { color: mod.widgets.RBX_FW_OPENCLAW_FG } });
        let mut openclaw_tag = self.view.label(cx, ids!(openclaw_card.card_body.card_col.card_tag.card_tag_label));
        script_apply_eval!(cx, openclaw_tag, { draw_text +: { color: mod.widgets.RBX_FW_OPENCLAW_FG } });
        let mut openclaw_tag_pill = self.view.view(cx, ids!(openclaw_card.card_body.card_col.card_tag));
        script_apply_eval!(cx, openclaw_tag_pill, { draw_bg +: { color: mod.widgets.RBX_FW_OPENCLAW_BG } });
    }

    fn update_framework_cards(&mut self, cx: &mut Cx) {
        let cards = [
            (AgentFramework::Hermes, ids!(hermes_card)),
            (AgentFramework::OpenClaw, ids!(openclaw_card)),
            (AgentFramework::Octos, ids!(octos_card)),
            (AgentFramework::OctosDirect, ids!(octos_direct_card)),
        ];
        for (framework, card_id) in cards {
            let selected = self.selected_framework == Some(framework);
            self.view.widget(cx, &[card_id[0], live_id!(card_body), live_id!(card_radio), live_id!(card_radio_check)])
                .set_visible(cx, selected);
            let mut radio = self.view.view(cx, &[card_id[0], live_id!(card_body), live_id!(card_radio)]);
            let mut card = self.view.view(cx, &[card_id[0], live_id!(card_body)]);
            if selected {
                script_apply_eval!(cx, radio, { draw_bg +: { color: mod.widgets.RBX_ACCENT, border_color: mod.widgets.RBX_ACCENT } });
                script_apply_eval!(cx, card, { draw_bg +: { border_size: 1.5, border_color: mod.widgets.RBX_ACCENT, color: mod.widgets.RBX_ACCENT_SOFT } });
            } else {
                script_apply_eval!(cx, radio, { draw_bg +: { color: mod.widgets.RBX_BG_SURFACE, border_color: mod.widgets.RBX_STROKE_STRONG } });
                script_apply_eval!(cx, card, { draw_bg +: { border_size: 1.0, border_color: mod.widgets.RBX_STROKE_SOFT, color: mod.widgets.RBX_BG_SURFACE } });
            }
        }
        self.view.redraw(cx);
    }

    fn sync_steps(&mut self, cx: &mut Cx) {
        let step2 = self.step == 2;
        self.view.view(cx, ids!(step1_view)).set_visible(cx, !step2);
        self.view.view(cx, ids!(step2_view)).set_visible(cx, step2);
        self.view.button(cx, ids!(back_button)).set_visible(cx, step2);
        self.view.view(cx, ids!(footer)).set_visible(cx, !step2);

        if step2 {
            let fw = self.selected_framework.map(framework_label).unwrap_or("agent");
            self.view.label(cx, ids!(sheet_title)).set_text(cx, &format!("Connect {fw}"));
            let sub = if self.is_octos() {
                "Step 2 of 2 · AppService + Matrix ID"
            } else {
                "Step 2 of 2 · Matrix ID"
            };
            self.view.label(cx, ids!(sheet_subtitle)).set_text(cx, sub);
            self.view.label(cx, ids!(step2_heading)).set_text(cx, &format!("New {fw} agent"));
            if let Some(framework) = self.selected_framework {
                self.sync_step2_framework_header(cx, framework);
            }
            self.sync_step2(cx);
        } else {
            self.view.label(cx, ids!(sheet_title)).set_text(cx, "Add an agent");
            self.view.label(cx, ids!(sheet_subtitle)).set_text(cx, "Step 1 of 2 · Choose a framework");
        }
        self.sync_primary_button(cx);
        self.view.redraw(cx);
    }

    fn sync_step2_framework_header(&mut self, cx: &mut Cx, framework: AgentFramework) {
        self.view.label(cx, ids!(step2_agent_header.step2_framework_tile.step2_framework_mono))
            .set_text(cx, framework_mono(framework));

        let mut tile = self.view.view(cx, ids!(step2_agent_header.step2_framework_tile));
        let mut mono = self.view.label(cx, ids!(step2_agent_header.step2_framework_tile.step2_framework_mono));
        match framework {
            AgentFramework::Octos | AgentFramework::OctosDirect => {
                script_apply_eval!(cx, tile, { draw_bg +: { color: mod.widgets.RBX_FW_OCTOS_BG } });
                script_apply_eval!(cx, mono, { draw_text +: { color: mod.widgets.RBX_FW_OCTOS_FG } });
            }
            AgentFramework::Hermes => {
                script_apply_eval!(cx, tile, { draw_bg +: { color: mod.widgets.RBX_FW_HERMES_BG } });
                script_apply_eval!(cx, mono, { draw_text +: { color: mod.widgets.RBX_FW_HERMES_FG } });
            }
            AgentFramework::OpenClaw => {
                script_apply_eval!(cx, tile, { draw_bg +: { color: mod.widgets.RBX_FW_OPENCLAW_BG } });
                script_apply_eval!(cx, mono, { draw_text +: { color: mod.widgets.RBX_FW_OPENCLAW_FG } });
            }
            AgentFramework::Unknown => {
                script_apply_eval!(cx, tile, { draw_bg +: { color: mod.widgets.RBX_NEUTRAL_BG } });
                script_apply_eval!(cx, mono, { draw_text +: { color: mod.widgets.RBX_NEUTRAL_FG } });
            }
        }
    }

    fn sync_step2(&mut self, cx: &mut Cx) {
        let added = self.friend_state == FriendState::Added;
        self.view.button(cx, ids!(add_friend_button)).set_visible(cx, !added);
        self.view.view(cx, ids!(friend_added_strip)).set_visible(cx, added);
        self.view.view(cx, ids!(octos_section)).set_visible(cx, self.is_octos());
        // The Matrix ID field locks once a friend request is in flight / done.
        self.view.text_input(cx, ids!(id_field.field_input)).set_is_read_only(cx, self.friend_state != FriendState::Idle);

        if self.is_octos() {
            self.view.text_input(cx, ids!(octos_section.octos_url_field.field_input)).set_is_read_only(cx, false);
            self.view.button(cx, ids!(octos_section.octos_check_row.check_now_button))
                .set_enabled(cx, !self.octos_health.in_flight);
        }
        self.sync_octos_status(cx);
        self.sync_bind_button(cx);
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
        let check_enabled = !self.octos_health.in_flight;
        let check_button = self.view.button(cx, ids!(octos_section.octos_check_row.check_now_button));
        check_button.set_enabled(cx, check_enabled);
        if self.octos_health.in_flight {
            check_button.set_text(cx, "Checking...");
        } else {
            check_button.set_text(cx, "Check");
        }
        self.view.view(cx, ids!(octos_section.octos_check_row.octos_check_spinner))
            .set_visible(cx, self.octos_health.in_flight);
        let offline = self.octos_health.status == OctosHealthStatus::Unreachable;
        let err = self.view.label(cx, ids!(octos_section.octos_error_label));
        if offline {
            let url = self.view.text_input(cx, ids!(octos_section.octos_url_field.field_input)).text();
            err.set_text(cx, &format!("No response from {}.", url.trim()));
        }
        err.set_visible(cx, offline);

        let mut pill = self.view.view(cx, ids!(octos_section.octos_check_row.octos_status_pill));
        let mut label = self.view.label(cx, ids!(octos_section.octos_check_row.octos_status_pill.octos_status_label));
        match self.octos_health.status {
            OctosHealthStatus::Unknown => {
                script_apply_eval!(cx, pill, { draw_bg +: { color: mod.widgets.RBX_NEUTRAL_BG } });
                script_apply_eval!(cx, label, { draw_text +: { color: mod.widgets.RBX_NEUTRAL_FG } });
            }
            OctosHealthStatus::Checking => {
                script_apply_eval!(cx, pill, { draw_bg +: { color: mod.widgets.RBX_WARNING_BG } });
                script_apply_eval!(cx, label, { draw_text +: { color: mod.widgets.RBX_WARNING_FG } });
            }
            OctosHealthStatus::Reachable => {
                script_apply_eval!(cx, pill, { draw_bg +: { color: mod.widgets.RBX_SUCCESS_BG } });
                script_apply_eval!(cx, label, { draw_text +: { color: mod.widgets.RBX_SUCCESS_FG } });
            }
            OctosHealthStatus::Unreachable => {
                script_apply_eval!(cx, pill, { draw_bg +: { color: mod.widgets.RBX_DANGER_BG } });
                script_apply_eval!(cx, label, { draw_text +: { color: mod.widgets.RBX_DANGER_FG } });
            }
        }
        self.sync_bind_button(cx);
        self.sync_primary_button(cx);
        self.view.redraw(cx);
    }

    fn sync_primary_button(&mut self, cx: &mut Cx) {
        let (text, enabled) = if self.step == 1 {
            ("Continue".to_string(), self.selected_framework.is_some())
        } else {
            ("Continue".to_string(), self.can_finish())
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
        self.octos_probe_base_url = None;

        self.view.text_input(cx, ids!(id_field.field_input)).set_text(cx, "");
        self.view.text_input(cx, ids!(id_field.field_input)).set_is_read_only(cx, false);
        self.view.text_input(cx, ids!(octos_section.octos_url_field.field_input)).set_text(cx, BotSettingsState::DEFAULT_OCTOS_SERVICE_URL);
        self.view.button(cx, ids!(octos_section.save_appservice_button)).set_text(cx, "Save AppService");
        self.set_add_friend_label(cx, "Add friend & bind");
        self.populate_framework_cards(cx);
        self.update_framework_cards(cx);
        self.sync_steps(cx);
        self.view.redraw(cx);
    }

    pub fn show_octos(
        &mut self,
        cx: &mut Cx,
        app_language: AppLanguage,
        octos_service_url: &str,
        existing_octos_agent_user_id: Option<&str>,
    ) {
        self.show(cx, app_language);
        let trimmed = octos_service_url.trim();
        let url = if trimmed.is_empty() {
            BotSettingsState::DEFAULT_OCTOS_SERVICE_URL.to_string()
        } else {
            trimmed.to_string()
        };

        self.selected_framework = Some(AgentFramework::Octos);
        self.step = 2;
        self.view.text_input(cx, ids!(octos_section.octos_url_field.field_input))
            .set_text(cx, &url);
        self.view.button(cx, ids!(octos_section.save_appservice_button))
            .set_text(cx, "Save AppService");
        self.update_framework_cards(cx);
        self.sync_steps(cx);
        let id_input = self.view.text_input(cx, ids!(id_field.field_input));
        if let Some(existing_octos_agent_user_id) = existing_octos_agent_user_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            id_input.set_text(cx, existing_octos_agent_user_id);
        }
        id_input.set_is_read_only(cx, false);
        id_input.set_key_focus(cx);
        self.sync_bind_button(cx);
        self.view.redraw(cx);
    }

    /// Clears the friend-binding state once the modal closes, so it no longer
    /// "owns" DM results for its last target. Without this, App's
    /// `suppress_add_agent_direct_message_action` keeps swallowing the navigation
    /// when you later tap "Open chat" on an already-bound agent.
    pub fn clear_waiting_state(&mut self) {
        self.friend_state = FriendState::Idle;
        self.target_user_id = None;
    }
}

impl AddAgentModalRef {
    pub fn show(&self, cx: &mut Cx, app_language: AppLanguage) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show(cx, app_language);
    }

    pub fn show_octos(
        &self,
        cx: &mut Cx,
        app_language: AppLanguage,
        octos_service_url: &str,
        existing_octos_agent_user_id: Option<&str>,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show_octos(cx, app_language, octos_service_url, existing_octos_agent_user_id);
    }

    pub fn clear_waiting_state(&self) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.clear_waiting_state();
        }
    }

    pub fn is_waiting_for_direct_message_result(&self, user_id: &OwnedUserId) -> bool {
        let Some(inner) = self.borrow() else { return false };
        inner.is_waiting_for_direct_message_result(user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::BotSettingsState;
    use crate::i18n::tr_key;

    fn production_src(src: &'static str) -> &'static str {
        src.split("#[cfg(test)]").next().unwrap_or(src)
    }

    #[test]
    fn test_octos_modal_registration_uses_agent_mxid_as_botfather_default() {
        let mut app_state = AppState::default();
        let user_id: OwnedUserId = "@octos:example.org".parse().unwrap();

        let added = register_agent_with_modal_settings(
            &mut app_state,
            user_id.clone(),
            Some("Octos".to_string()),
            AgentFramework::Octos,
            Some("http://127.0.0.1:8787".to_string()),
        );

        assert!(added);
        assert!(app_state.bot_settings.enabled);
        assert_eq!(app_state.bot_settings.botfather_user_id, user_id.as_str());
        assert_eq!(app_state.bot_settings.octos_service_url, "http://127.0.0.1:8787");
        assert!(app_state.bot_settings.known_bot_user_ids.contains(&user_id));
    }

    #[test]
    fn test_register_octos_direct_does_not_touch_botfather() {
        let mut app_state = AppState::default();
        assert!(!app_state.bot_settings.enabled);
        let default_botfather = app_state.bot_settings.botfather_user_id.clone();
        let user_id: OwnedUserId = "@myagent:example.org".parse().unwrap();

        let added = register_agent_with_modal_settings(
            &mut app_state,
            user_id.clone(),
            Some("MyAgent".to_string()),
            AgentFramework::OctosDirect,
            None,
        );

        assert!(added);
        assert_ne!(app_state.bot_settings.botfather_user_id, user_id.as_str());
        assert_eq!(app_state.bot_settings.botfather_user_id, default_botfather);
        assert!(!app_state.bot_settings.enabled);
        assert!(!app_state.bot_settings.known_bot_user_ids.contains(&user_id));
        // Still registered as an agent in the registry.
        assert!(app_state.agent_registry.contains(user_id.as_ref()));
    }

    #[test]
    fn test_register_octos_appservice_first_time_sets_botfather() {
        let mut app_state = AppState::default();
        assert_eq!(
            app_state.bot_settings.botfather_user_id,
            BotSettingsState::DEFAULT_BOTFATHER_LOCALPART,
        );
        let user_id: OwnedUserId = "@octos:example.org".parse().unwrap();

        register_agent_with_modal_settings(
            &mut app_state,
            user_id.clone(),
            None,
            AgentFramework::Octos,
            None,
        );

        assert_eq!(app_state.bot_settings.botfather_user_id, user_id.as_str());
        assert!(app_state.bot_settings.enabled);
    }

    #[test]
    fn test_register_octos_child_does_not_clobber_botfather() {
        let mut app_state = AppState::default();
        let parent: OwnedUserId = "@octos:example.org".parse().unwrap();
        register_agent_with_modal_settings(
            &mut app_state,
            parent.clone(),
            None,
            AgentFramework::Octos,
            None,
        );
        assert_eq!(app_state.bot_settings.botfather_user_id, parent.as_str());

        let child: OwnedUserId = "@octos_weather:example.org".parse().unwrap();
        register_agent_with_modal_settings(
            &mut app_state,
            child.clone(),
            None,
            AgentFramework::Octos,
            None,
        );

        // BotFather untouched by the child registration.
        assert_eq!(app_state.bot_settings.botfather_user_id, parent.as_str());
        // The child is still recorded as a known bot and in the registry.
        assert!(app_state.bot_settings.known_bot_user_ids.contains(&child));
        assert!(app_state.agent_registry.contains(child.as_ref()));
    }

    #[test]
    fn test_register_octos_child_preserves_existing_appservice_url() {
        let mut app_state = AppState::default();
        let parent: OwnedUserId = "@octos:example.org".parse().unwrap();
        let custom_url = "http://10.0.0.5:8010";
        register_agent_with_modal_settings(
            &mut app_state,
            parent.clone(),
            None,
            AgentFramework::Octos,
            Some(custom_url.to_string()),
        );
        assert_eq!(app_state.bot_settings.botfather_user_id, parent.as_str());
        assert_eq!(app_state.bot_settings.octos_service_url, custom_url);

        let child: OwnedUserId = "@octos_weather:example.org".parse().unwrap();
        register_agent_with_modal_settings(
            &mut app_state,
            child.clone(),
            None,
            AgentFramework::Octos,
            Some(BotSettingsState::DEFAULT_OCTOS_SERVICE_URL.to_string()),
        );

        assert_eq!(app_state.bot_settings.botfather_user_id, parent.as_str());
        assert_eq!(app_state.bot_settings.octos_service_url, custom_url);
        assert!(app_state.agent_registry.contains(child.as_ref()));
    }

    #[test]
    fn test_octos_modal_registration_uses_same_id_for_botfather_and_friend() {
        let mut app_state = AppState::default();
        let user_id: OwnedUserId = "@octos:example.org".parse().unwrap();

        let added = register_agent_with_modal_settings(
            &mut app_state,
            user_id.clone(),
            None,
            AgentFramework::Octos,
            None,
        );

        assert!(added);
        assert_eq!(app_state.bot_settings.botfather_user_id, user_id.as_str());
        assert_eq!(
            app_state.bot_settings.octos_service_url,
            BotSettingsState::DEFAULT_OCTOS_SERVICE_URL,
        );
    }

    #[test]
    fn test_add_agent_dm_result_is_suppressed_before_global_navigation() {
        let app_src = include_str!("../app.rs");
        let suppress_pos = app_src
            .find("suppress_add_agent_direct_message_action")
            .expect("app should suppress add-agent DM results before global navigation");
        let global_pos = app_src
            .find("// Handle DirectMessageRoomActions")
            .expect("global direct message handler should keep its marker");

        assert!(suppress_pos < global_pos);
    }

    #[test]
    fn test_octos_appservice_controls_precede_matrix_binding() {
        let src = production_src(include_str!("agent_add_modal.rs"));
        let octos_pos = src
            .find("octos_section := RoundedView")
            .expect("Octos step should expose an AppService section");
        let matrix_pos = src
            .find("id_field := AgentField")
            .expect("Octos step should still include Matrix ID binding");

        assert!(octos_pos < matrix_pos, "Octos AppService URL should be configured before Matrix ID binding");
        assert!(src.contains("save_appservice_button"));
        assert!(!src.contains("Unlocks after the friend is added."));
    }

    #[test]
    fn test_octos_appservice_controls_are_not_friend_locked() {
        let src = production_src(include_str!("agent_add_modal.rs"));

        assert!(!src.contains("let appservice_unlocked = self.friend_state == FriendState::Added;"));
        assert!(!src.contains("if self.friend_state != FriendState::Added {\n                return;\n            }"));
        assert!(src.contains("fn save_octos_appservice_settings"));
    }

    #[test]
    fn test_saving_octos_appservice_focuses_matrix_id_binding_field() {
        let src = production_src(include_str!("agent_add_modal.rs"));
        let save_fn_pos = src
            .find("fn save_octos_appservice_settings")
            .expect("Octos AppService save handler should exist");
        let save_fn = &src[save_fn_pos..];

        assert!(
            save_fn.contains("let id_input = self.view.text_input(cx, ids!(id_field.field_input));"),
            "after saving the AppService URL, the handler should select the Matrix ID field",
        );
        assert!(
            save_fn.contains("id_input.set_key_focus(cx);"),
            "after saving the AppService URL, focus should move to the Matrix ID field for the bind step",
        );
    }

    #[test]
    fn test_saving_octos_appservice_does_not_steal_matrix_id_focus_while_friend_request_in_flight() {
        let src = production_src(include_str!("agent_add_modal.rs"));
        let save_fn_pos = src
            .find("fn save_octos_appservice_settings")
            .expect("Octos AppService save handler should exist");
        let save_fn = &src[save_fn_pos..];

        assert!(
            save_fn.contains("if self.friend_state == FriendState::Idle {"),
            "saving the AppService URL must not unlock or steal focus into the Matrix ID field while a friend/bind request is in flight or already done, since sync_step2 locks that field on FriendState::Pending/Added",
        );
    }

    #[test]
    fn test_add_modal_can_open_directly_to_octos_setup_with_saved_url() {
        let src = production_src(include_str!("agent_add_modal.rs"));
        let show_fn_pos = src
            .find("pub fn show_octos")
            .expect("AddAgentModal should expose a direct Octos setup entry point");
        let show_fn = &src[show_fn_pos..];

        assert!(show_fn.contains("self.selected_framework = Some(AgentFramework::Octos);"));
        assert!(show_fn.contains("self.step = 2;"));
        assert!(
            show_fn.contains("octos_service_url")
                && show_fn.contains("octos_section.octos_url_field.field_input")
                && show_fn.contains("set_text(cx, &url);"),
            "direct Octos setup should prefill the saved AppService URL instead of resetting to localhost",
        );
    }

    #[test]
    fn test_direct_octos_setup_focuses_matrix_id_input() {
        let src = production_src(include_str!("agent_add_modal.rs"));
        let show_fn_pos = src
            .find("pub fn show_octos")
            .expect("AddAgentModal should expose a direct Octos setup entry point");
        let show_fn = &src[show_fn_pos..];

        assert!(
            show_fn.contains("let id_input = self.view.text_input(cx, ids!(id_field.field_input));")
                && show_fn.contains("id_input.set_is_read_only(cx, false);")
                && show_fn.contains("id_input.set_key_focus(cx);"),
            "opening the saved Octos setup directly should focus the Matrix ID field so the user can start typing the bot id",
        );
    }

    #[test]
    fn test_direct_octos_setup_prefills_existing_octos_agent_id() {
        let src = production_src(include_str!("agent_add_modal.rs"));
        let show_fn_pos = src
            .find("pub fn show_octos")
            .expect("AddAgentModal should expose a direct Octos setup entry point");
        let show_fn = &src[show_fn_pos..];

        assert!(show_fn.contains("existing_octos_agent_user_id"));
        assert!(
            show_fn.contains("if let Some(existing_octos_agent_user_id) = existing_octos_agent_user_id")
                && show_fn.contains("id_input.set_text(cx, existing_octos_agent_user_id);"),
            "opening Change Octos Bot should show the currently registered Octos bot instead of an empty Matrix ID field",
        );
        assert!(
            show_fn.contains("self.sync_bind_button(cx);"),
            "programmatic Matrix ID prefill should refresh the bind button state",
        );
    }

    #[test]
    fn test_app_routes_octos_setup_action_with_saved_appservice_url() {
        let app_src = include_str!("../app.rs");

        assert!(app_src.contains("AgentSettingsAction::OpenOctosSetup"));
        assert!(app_src.contains("resolved_octos_service_url().to_string()"));
        assert!(app_src.contains("find(|user_id|"));
        assert!(app_src.contains("entry.framework == AgentFramework::Octos"));
        assert!(app_src.contains(".show_octos("));
        assert!(app_src.contains("&octos_service_url"));
        assert!(app_src.contains("existing_octos_agent_user_id"));
    }

    #[test]
    fn test_octos_direct_card_text_uses_i18n_keys() {
        let src = production_src(include_str!("agent_add_modal.rs"));

        assert_eq!(
            tr_key(AppLanguage::English, "settings.labs.agents.framework.octos_direct.name"),
            "Octos (Direct)",
        );
        assert_eq!(
            tr_key(AppLanguage::ChineseSimplified, "settings.labs.agents.framework.octos_direct.blurb"),
            "Octos, 作为 Matrix 好友直接添加。",
        );
        for key in [
            "settings.labs.agents.framework.octos_direct.mono",
            "settings.labs.agents.framework.octos_direct.name",
            "settings.labs.agents.framework.octos_direct.tag",
            "settings.labs.agents.framework.octos_direct.blurb",
        ] {
            assert!(
                src.contains(&format!("text(\"{key}\")")),
                "OctosDirect card text should read {key} from i18n resources",
            );
        }
        assert!(src.contains("let text = |key| tr_key(self.app_language, key);"));
        assert!(
            !src.contains(".set_text(cx, \"Octos (Direct)\")"),
            "OctosDirect card name should not be hardcoded in AddAgentModal",
        );
    }

    #[test]
    fn test_octos_flow_removed_local_binding_browser_step() {
        let src = production_src(include_str!("agent_add_modal.rs"));

        assert!(!src.contains("open_local_binding_button"));
        assert!(!src.contains("robius_open::Uri::new(&url).open()"));
        assert!(!src.contains("BotFather"));
        assert!(!src.contains("botfather_locked_strip"));
    }

    #[test]
    fn test_friend_result_finishes_registration_without_extra_finish_step() {
        let src = include_str!("agent_add_modal.rs");
        let signature = [
            "fn on_friend_added(&mut self, cx: &mut Cx, scope",
            ": &mut Scope)",
        ].concat();
        let finish_gate = ["if self.can", "_finish() {"].concat();
        let finish_call = ["self.finish", "_register(cx, scope);"].concat();

        assert!(src.contains(&signature));
        assert!(src.contains(&finish_gate));
        assert!(src.contains(&finish_call));
    }

    #[test]
    fn test_bind_button_requires_valid_matrix_id() {
        assert_eq!(
            bind_button_state(FriendState::Idle, false, OctosHealthStatus::Unknown, ""),
            ("Enter Matrix ID", false),
        );
        assert_eq!(
            bind_button_state(FriendState::Idle, false, OctosHealthStatus::Unknown, "@agent"),
            ("Enter Matrix ID", false),
        );
        assert_eq!(
            bind_button_state(FriendState::Idle, false, OctosHealthStatus::Unknown, "@agent:example.org"),
            ("Bind agent", true),
        );
        assert_eq!(
            bind_button_state(FriendState::Idle, true, OctosHealthStatus::Unknown, "@octos:example.org"),
            ("Bind Octos agent", true),
        );
        assert_eq!(
            bind_button_state(FriendState::Idle, true, OctosHealthStatus::Reachable, ""),
            ("Enter Matrix ID", false),
        );
        assert_eq!(
            bind_button_state(FriendState::Idle, true, OctosHealthStatus::Reachable, "@octos:example.org"),
            ("Bind Octos agent", true),
        );
    }

    #[test]
    fn test_octos_url_change_resets_health_and_drops_stale_probe() {
        let src = include_str!("agent_add_modal.rs");
        let probe_field = ["octos_probe_base", "_url: Option<String>"].concat();
        let url_changed = ["octos_section.octos_url_field.field", "_input)).changed(actions)"].concat();
        let clear_probe = ["self.octos_probe_base", "_url = None;"].concat();
        let ignore_stale = ["let Some(probe_base_url) = self.octos_probe_base", "_url.clone() else { continue };"].concat();

        assert!(src.contains(&probe_field));
        assert!(src.contains(&url_changed));
        assert!(src.contains(&clear_probe));
        assert!(src.contains(&ignore_stale));
    }

    #[test]
    fn test_add_modal_uses_responsive_sheet_width_and_batched_surfaces() {
        let src = production_src(include_str!("agent_add_modal.rs"));
        let responsive_width = ["width: Fill{min: 0 max: ", "352}"].concat();
        let sheet_batch = ["sheet := RoundedView {", "\n            width: Fill{min: 0 max: 352}", "\n            height: 592", "\n            flow: Down", "\n            new_batch: true"].concat();
        let root_start = src
            .find("mod.widgets.AddAgentModal = #(AddAgentModal::register_widget(vm)) {")
            .expect("expected AddAgentModal definition");
        let sheet_start = src[root_start..]
            .find("sheet := RoundedView {")
            .expect("expected sheet inside AddAgentModal");
        let root_prefix = &src[root_start..root_start + sheet_start];

        assert!(root_prefix.contains("width: Fill"));
        assert!(root_prefix.contains("height: Fit"));
        assert!(src.contains(&responsive_width));
        assert!(src.contains(&sheet_batch));
        let block_start = src.find("octos_section := RoundedView {").expect("expected Octos section");
        let next_draw = src[block_start..]
            .find("draw_bg +:")
            .expect("expected Octos section colored surface");
        let block_prefix = &src[block_start..block_start + next_draw];
        assert!(block_prefix.contains("new_batch: true"));
    }

    #[test]
    fn test_add_modal_uses_scrollable_body_with_sticky_footer() {
        let src = production_src(include_str!("agent_add_modal.rs"));

        assert!(src.contains("height: 592"));
        assert!(src.contains("body_scroll := ScrollYView"));
        assert!(
            src.contains("body_scroll := ScrollYView {\n                width: Fill\n                height: 382"),
            "body_scroll must not Fill over the sticky footer hit area on Android",
        );
        let body_pos = src.find("body_scroll := ScrollYView").expect("modal should have a scroll body");
        let footer_pos = src.find("footer := View").expect("modal should keep a footer");
        assert!(body_pos < footer_pos, "footer should be outside and after the scroll body");
    }

    #[test]
    fn test_add_modal_sheet_does_not_steal_text_input_focus() {
        let src = production_src(include_str!("agent_add_modal.rs"));
        let sheet_pos = src.find("sheet := RoundedView").expect("modal should have a sheet");
        let body_pos = src.find("body_scroll := ScrollYView").expect("modal should have a body");
        let sheet_block = &src[sheet_pos..body_pos];

        assert!(sheet_block.contains("cursor: MouseCursor.Default"));
        assert!(sheet_block.contains("capture_overload: true"));
        assert!(
            sheet_block.contains("grab_key_focus: false"),
            "sheet absorbs pointer hits but must not take keyboard focus back from TextInput children",
        );
    }

    #[test]
    fn test_framework_picker_matches_handoff_order_and_tile_size() {
        let src = production_src(include_str!("agent_add_modal.rs"));
        let hermes_pos = src.find("hermes_card := FrameworkCard").expect("Hermes card should exist");
        let openclaw_pos = src.find("openclaw_card := FrameworkCard").expect("OpenClaw card should exist");
        let octos_pos = src.find("octos_card := FrameworkCard").expect("Octos card should exist");

        assert!(hermes_pos < openclaw_pos);
        assert!(openclaw_pos < octos_pos);
        assert!(src.contains("card_tile := RoundedView {\n                width: 48\n                height: 48"));
    }

    #[test]
    fn test_step2_header_shows_framework_identity_tile() {
        let src = production_src(include_str!("agent_add_modal.rs"));

        assert!(src.contains("step2_agent_header := View"));
        assert!(src.contains("step2_framework_tile := RoundedView"));
        assert!(src.contains("step2_framework_mono := Label"));
        assert!(src.contains("fn sync_step2_framework_header"));
        assert!(src.contains("framework_mono(framework)"));
    }


    #[test]
    fn test_matrix_id_field_uses_plain_text_input_without_at_adornment() {
        let src = production_src(include_str!("agent_add_modal.rs"));

        assert!(
            src.contains("id_field := AgentField"),
            "Matrix ID should use the same plain RobrixTextInput field pattern as working user/bot id inputs",
        );
        assert!(!src.contains("let MatrixIdField = View"));
        assert!(!src.contains("at_prefix := Label"));
        assert!(src.contains("field_input.empty_text: \"@agent:server or agent:server\""));
    }

    #[test]
    fn test_step2_primary_button_copy_matches_handoff() {
        let src = production_src(include_str!("agent_add_modal.rs"));

        assert!(src.contains("self.view.view(cx, ids!(footer)).set_visible(cx, !step2);"));
        assert!(src.contains("Bind Octos agent"));
        assert!(src.contains("save_appservice_button"));
        assert!(!src.contains("\"Finish & register\""));
        assert!(!src.contains("\"Add the agent above to continue\""));
        assert!(!src.contains("\"Service must be online\""));
        assert!(!src.contains("\"Ready\""));
        assert!(!src.contains("\"Bind above to continue\""));
    }

    #[test]
    fn test_pending_states_show_spinners_and_handoff_copy() {
        let src = production_src(include_str!("agent_add_modal.rs"));

        assert!(src.contains("bind_progress_row := View"));
        assert!(src.contains("bind_progress_spinner := LoadingSpinner"));
        assert!(src.contains("\"Sending friend request...\""));
        assert!(src.contains("octos_check_spinner := LoadingSpinner"));
        assert!(src.contains("text: \"Check\""));
        assert!(src.contains("set_text(cx, \"Checking...\")"));
    }

    #[test]
    fn test_back_button_uses_plain_left_chevron() {
        let src = production_src(include_str!("agent_add_modal.rs"));

        assert!(src.contains("resources/icons/chevron_left.svg"));
        assert!(!src.contains("resources/icons/go_back.svg"));
    }
}
