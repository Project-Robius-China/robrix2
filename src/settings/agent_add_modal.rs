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
    i18n::AppLanguage,
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

pub fn register_agent_with_modal_settings(
    app_state: &mut AppState,
    user_id: OwnedUserId,
    display_name: Option<String>,
    framework: AgentFramework,
    octos_service_url: Option<String>,
) -> bool {
    let added = register_agent_from_search(app_state, user_id.clone(), display_name, framework);

    if framework == AgentFramework::Octos {
        let url = octos_service_url
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(BotSettingsState::DEFAULT_OCTOS_SERVICE_URL)
            .to_string();

        app_state.bot_settings.enabled = true;
        app_state.bot_settings.botfather_user_id = user_id.as_str().to_string();
        app_state.bot_settings.octos_service_url = url;
        app_state.bot_settings.record_known_bot_user_ids([user_id]);
    }

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

    let MatrixIdField = View {
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
            text: "Agent Matrix ID"
        }
        field_control := RoundedView {
            width: Fill
            height: Fit
            flow: Right
            align: Align{y: 0.5}
            spacing: 0
            padding: Inset{left: 11, right: 4, top: 2, bottom: 2}
            new_batch: true
            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE_SUBTLE)
                border_radius: (RBX_RADIUS_XXS)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
            }
            at_prefix := Label {
                width: Fit
                height: Fit
                margin: Inset{right: 2}
                draw_text +: {
                    color: (RBX_FG_TERTIARY)
                    text_style: RBX_TEXT_BODY_STRONG {}
                }
                text: "@"
            }
            field_input := RobrixTextInput {
                width: Fill
                height: Fit
                padding: Inset{left: 2, right: 8, top: 8, bottom: 8}
                empty_text: "agent:server"
                draw_bg +: {
                    color: (RBX_TRANSPARENT)
                    color_hover: (RBX_TRANSPARENT)
                    color_focus: (RBX_TRANSPARENT)
                    color_down: (RBX_TRANSPARENT)
                    color_empty: (RBX_TRANSPARENT)
                    color_disabled: (RBX_TRANSPARENT)
                    border_size: 0.0
                    border_color: (RBX_TRANSPARENT)
                    border_color_hover: (RBX_TRANSPARENT)
                    border_color_focus: (RBX_TRANSPARENT)
                    border_color_down: (RBX_TRANSPARENT)
                    border_color_empty: (RBX_TRANSPARENT)
                    border_color_disabled: (RBX_TRANSPARENT)
                }
                // Dark real input vs light-grey placeholder, so the hint reads
                // as a hint rather than an entered ID.
                draw_text +: {
                    color: (RBX_FG_PRIMARY)
                    color_empty: (RBX_FG_TERTIARY)
                    color_empty_hover: (RBX_FG_TERTIARY)
                    color_empty_focus: (RBX_FG_TERTIARY)
                }
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

                    id_field := MatrixIdField {}

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
                (AgentFramework::Hermes, ids!(hermes_card.card_click)),
                (AgentFramework::OpenClaw, ids!(openclaw_card.card_click)),
                (AgentFramework::Octos, ids!(octos_card.card_click)),
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
            let id_changed = self.view.text_input(cx, ids!(id_field.field_control.field_input)).changed(actions).is_some();
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
        let raw = self.view.text_input(cx, ids!(id_field.field_control.field_input)).text();
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
        self.view.redraw(cx);
    }

    fn set_add_friend_label(&mut self, cx: &mut Cx, text: &str) {
        self.view.button(cx, ids!(add_friend_button)).set_text(cx, text);
    }

    fn sync_bind_button(&mut self, cx: &mut Cx) {
        let agent_id_raw = self.view.text_input(cx, ids!(id_field.field_control.field_input)).text();
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
        self.view.label(cx, ids!(octos_card.card_body.card_tile.card_mono)).set_text(cx, "Oc");
        self.view.label(cx, ids!(octos_card.card_body.card_col.card_name)).set_text(cx, "Octos");
        self.view.label(cx, ids!(octos_card.card_body.card_col.card_tag.card_tag_label)).set_text(cx, "APPSERVICE");
        self.view.label(cx, ids!(octos_card.card_body.card_col.card_blurb)).set_text(cx, "Friend plus local AppService.");
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
            AgentFramework::Octos => {
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
        self.view.text_input(cx, ids!(id_field.field_control.field_input)).set_is_read_only(cx, self.friend_state != FriendState::Idle);

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

        self.view.text_input(cx, ids!(id_field.field_control.field_input)).set_text(cx, "");
        self.view.text_input(cx, ids!(id_field.field_control.field_input)).set_is_read_only(cx, false);
        self.view.text_input(cx, ids!(octos_section.octos_url_field.field_input)).set_text(cx, BotSettingsState::DEFAULT_OCTOS_SERVICE_URL);
        self.view.button(cx, ids!(octos_section.save_appservice_button)).set_text(cx, "Save AppService");
        self.set_add_friend_label(cx, "Add friend & bind");
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

    pub fn is_waiting_for_direct_message_result(&self, user_id: &OwnedUserId) -> bool {
        let Some(inner) = self.borrow() else { return false };
        inner.is_waiting_for_direct_message_result(user_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::app::BotSettingsState;

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
            .find("id_field := MatrixIdField")
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
    fn test_matrix_id_field_matches_handoff_at_adornment() {
        let src = production_src(include_str!("agent_add_modal.rs"));

        assert!(src.contains("let MatrixIdField = View"));
        assert!(src.contains("at_prefix := Label"));
        assert!(src.contains("text: \"@\""));
        assert!(src.contains("empty_text: \"agent:server\""));
        assert!(!src.contains("field_input.empty_text: \"@agent:server\""));
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
