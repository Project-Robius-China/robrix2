use std::ops::Not;

use makepad_widgets::*;
use url::Url;

use crate::{app::AppState, homeserver::{login_mode, CapabilityProbeAction, LoginMode}, i18n::{AppLanguage, tr_fmt, tr_key}, proxy_config::{validate_proxy_url_for_user_input, ProxyInputError}, sliding_sync::{submit_async_request, AccountSwitchAction, LoginByPassword, LoginRequest, MatrixRequest}};
use crate::register::{validation::normalize_homeserver_url, RegisterAction};

use super::login_status_modal::{LoginStatusModalAction, LoginStatusModalWidgetExt};

fn should_show_login_failure_modal(
    suppress_login_failure_modal: bool,
    last_failure_message_shown: Option<&str>,
    error: &str,
) -> bool {
    !suppress_login_failure_modal && last_failure_message_shown != Some(error)
}

/// Whether the login_button click should trigger a homeserver capability
/// probe before attempting to log in.
///
/// Pure predicate so the decision can be unit-tested without driving a
/// LoginScreen instance: we probe whenever we haven't yet classified this
/// homeserver into Password vs MasOidc, and no OIDC flow is already in
/// flight (re-probing mid-OAuth would clobber the session we're building).
fn should_probe_homeserver(login_mode: Option<LoginMode>, oidc_in_flight: bool) -> bool {
    login_mode.is_none() && !oidc_in_flight
}

const MOBILE_LOGIN_MAX_WIDTH: f64 = 700.0;

fn is_mobile_login_layout(width: f64, mobile_target: bool) -> bool {
    mobile_target || width <= MOBILE_LOGIN_MAX_WIDTH
}

fn is_mobile_runtime_target() -> bool {
    cfg!(any(target_os = "android", target_os = "ios"))
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.IMG_APP_LOGO = crate_resource("self://resources/robrix_logo_alpha.png")
    mod.widgets.ICON_EYE_OPEN   = crate_resource("self://resources/icons/eye_open.svg")
    mod.widgets.ICON_EYE_CLOSED = crate_resource("self://resources/icons/eye_closed.svg")
    mod.widgets.ICON_LOGIN_USER = crate_resource("self://resources/icon_user.svg")

    mod.widgets.SsoButton = RoundedView {
        width: 46,
        height: 40,
        new_batch: true,
        cursor: MouseCursor.Hand,
        visible: true,
        padding: 8,
        margin: Inset{ left: 3, right: 3, top: 0, bottom: 0}
        align: Align{x: 0.5, y: 0.5}
        flow: Right
        spacing: 8
        show_bg: true,
        draw_bg +: {
            border_size: 1.0
            border_color: (RBX_STROKE_SOFT)
            color: (RBX_BG_SURFACE)
            border_radius: (RBX_RADIUS_SM)
        }
    }

    mod.widgets.SsoImage = Image {
        width: 22, height: 22,
        draw_bg +: {
            mask: instance(0.0)
            pixel: fn() {
                let color = self.get_color()
                let gray = dot(color.rgb, vec3(0.299, 0.587, 0.114))
                let grayed = mix(color, vec4(gray, gray, gray, color.a), self.mask)
                return Pal.premul(vec4(grayed.xyz, grayed.w * self.opacity))
            }
        }
    }

    mod.widgets.LoginTextInput = RobrixTextInput {
        height: (RBX_CONTROL_H_LG)
        clip_x: true
        clip_y: true
        padding: Inset{top: 10, bottom: 10, left: 14, right: 14}
        draw_bg +: {
            color: (RBX_BG_SURFACE)
            color_hover: (RBX_BG_SURFACE)
            color_focus: (RBX_BG_SURFACE)
            color_down: (RBX_BG_SURFACE)
            color_empty: (RBX_BG_SURFACE)
            border_radius: (RBX_RADIUS_SM)
            border_color: (RBX_STROKE_SOFT)
            border_color_hover: (RBX_STROKE_STRONG)
            border_color_focus: (RBX_ACCENT)
            border_color_down: (RBX_STROKE_STRONG)
            border_color_empty: (RBX_STROKE_SOFT)
        }
        draw_cursor +: {
            color: (RBX_ACCENT)
        }
    }


    mod.widgets.LoginScreen = set_type_default() do #(LoginScreen::register_widget(vm)) {
        ..mod.widgets.SolidView

        width: Fill, height: Fill,
        flow: Overlay
        align: Align{x: 0.5, y: 0.5}
        show_bg: true,
        draw_bg +: {
            color: (RBX_BG_CANVAS)
        }

        login_scroll := ScrollYView {
            width: Fill, height: Fill,
            flow: Down, // Required for vertical scrolling to work.
            align: Align{x: 0.5, y: 0.5}
            show_bg: true,
            draw_bg.color: (RBX_BG_CANVAS)

            // allow the view to be scrollable but hide the actual scroll bar
            scroll_bars: {
                show_scroll_x: false, show_scroll_y: true,
                scroll_bar_y: {
                    bar_size: 0.0
                    min_handle_size: 0.0
                    drag_scrolling: true
                }
            }

            RoundedView {
                margin: Inset{top: 50, bottom: 50}
                width: Fill
                height: Fit
                align: Align{x: 0.5, y: 0.5}
                flow: Overlay,

                login_card := RoundedView {
                    width: Fill{max: 494}
                    height: Fit
                    margin: Inset{left: 16, right: 16}
                    new_batch: true
                    flow: Down
                    align: Align{x: 0.5, y: 0.5}
                    padding: Inset{top: 24, bottom: 24, left: 36, right: 36}
                    show_bg: true,
                    draw_bg +: {
                        color: (RBX_BG_SURFACE)
                        border_size: 1.0
                        border_color: (RBX_STROKE_SOFT)
                        border_radius: (RBX_RADIUS_LG)
                    }

                    // Top-right brand badge in normal card flow; avoid card-level
                    // overlay around the input stack.
                    View {
                        width: Fill, height: Fit
                        align: Align{x: 1.0, y: 0.0}
                        agent_badge := RoundedView {
                            width: Fit, height: Fit
                            new_batch: true
                            padding: Inset{left: 9, right: 9, top: 4, bottom: 4}
                            show_bg: true,
                            draw_bg +: {
                                color: (RBX_ACCENT_SOFT)
                                border_radius: (RBX_RADIUS_PILL)
                            }
                            agent_badge_label := Label {
                                width: Fit, height: Fit
                                draw_text +: {
                                    color: (RBX_ACCENT)
                                    text_style: RBX_TEXT_BADGE {}
                                }
                                text: "Agent-ready workspace"
                            }
                        }
                    }

                    // Centered content column.
                    form_column := View {
                    width: Fill, height: Fit
                    flow: Down
                    align: Align{x: 0.5, y: 0.5}
                    spacing: 10.0

                    logo_image := Image {
                        fit: ImageFit.Smallest,
                        width: 60
                        src: (mod.widgets.IMG_APP_LOGO),
                    }

                    brand_wordmark := Label {
                        width: Fit, height: Fit
                        padding: 0,
                        draw_text +: {
                            color: (RBX_FG_PRIMARY)
                            text_style: RBX_TEXT_PAGE_TITLE {font_size: 22.0}
                        }
                        text: "Robrix2"
                    }

                    title := Label {
                        width: Fit, height: Fit
                        margin: Inset{ bottom: 6 }
                        padding: 0,
                        draw_text +: {
                            color: (RBX_FG_SECONDARY)
                            text_style: REGULAR_TEXT {font_size: 10.5}
                        }
                        text: "Agent-native collaboration client"
                    }

                    user_id_field_label := Label {
                        visible: false
                        width: Fill{max: 422}, height: Fit
                        draw_text +: {
                            color: (RBX_NAV_FG)
                            text_style: REGULAR_TEXT {font_size: 11.0}
                        }
                        text: "User ID"
                    }

                    user_id_input := mod.widgets.LoginTextInput {
                        width: Fill{max: 422}, height: (RBX_CONTROL_H_LG)
                        flow: Right, // do not wrap
                        padding: Inset{top: 10, bottom: 10, left: 14, right: 14}
                        empty_text: "User ID"
                    }

                    password_field_label := Label {
                        visible: false
                        width: Fill{max: 422}, height: Fit
                        draw_text +: {
                            color: (RBX_NAV_FG)
                            text_style: REGULAR_TEXT {font_size: 11.0}
                        }
                        text: "Password"
                    }

                    View {
                        width: Fill{max: 422}, height: (RBX_CONTROL_H_LG)
                        flow: Overlay
                        align: Align{x: 1.0, y: 0.5}

                        password_input := mod.widgets.LoginTextInput {
                            width: Fill, height: (RBX_CONTROL_H_LG)
                            flow: Right, // do not wrap
                            padding: Inset{top: 10, bottom: 10, left: 14, right: 42}
                            empty_text: "Password"
                            is_password: true,
                        }

                        View {
                            width: 38, height: Fill
                            align: Align{x: 0.5, y: 0.5}

                            show_password_button := RobrixNeutralIconButton {
                                width: Fit, height: Fit,
                                align: Align{x: 0.5, y: 0.5}
                                padding: 5
                                spacing: 0
                                margin: 0
                                draw_bg +: {
                                    color: (RBX_BG_SURFACE)
                                }
                                draw_icon +: {
                                    svg: (mod.widgets.ICON_EYE_CLOSED),
                                    color: (RBX_FG_TERTIARY),
                                }
                                icon_walk: Walk{width: 18, height: 18, margin: 0}
                                text: ""
                            }

                            hide_password_button := RobrixNeutralIconButton {
                                visible: false,
                                align: Align{x: 0.5, y: 0.5}
                                width: Fit, height: Fit,
                                padding: 5
                                spacing: 0
                                margin: 0
                                draw_bg +: {
                                    color: (RBX_BG_SURFACE)
                                }
                                draw_icon +: {
                                    svg: (mod.widgets.ICON_EYE_OPEN),
                                    color: (RBX_FG_TERTIARY),
                                }
                                icon_walk: Walk{width: 18, height: 18, margin: 0}
                                text: ""
                            }
                        }
                    }

                    confirm_password_wrapper := View {
                        width: Fill{max: 422}, height: (RBX_CONTROL_H_LG),
                        visible: false,
                        flow: Overlay,

                        confirm_password_input := mod.widgets.LoginTextInput {
                            width: Fill, height: (RBX_CONTROL_H_LG)
                            flow: Right, // do not wrap
                            padding: Inset{top: 10, bottom: 10, left: 14, right: 42}
                            empty_text: "Confirm password"
                            is_password: true,
                        }

                        View {
                            width: Fill, height: Fill
                            align: Align{x: 1.0, y: 0.5}

                            show_confirm_password_button := Button {
                                width: 36, height: 36,
                                padding: 6,
                                draw_bg +: {
                                    color: (COLOR_TRANSPARENT)
                                    color_hover: (COLOR_TRANSPARENT)
                                    color_down: (COLOR_TRANSPARENT)
                                    border_size: 0.0
                                }
                                draw_icon +: {
                                    svg: (mod.widgets.ICON_EYE_CLOSED),
                                    color: (RBX_FG_TERTIARY),
                                }
                                icon_walk: Walk{width: 20, height: 20}
                                text: ""
                            }

                            hide_confirm_password_button := Button {
                                visible: false,
                                width: 36, height: 36,
                                padding: 6,
                                draw_bg +: {
                                    color: (COLOR_TRANSPARENT)
                                    color_hover: (COLOR_TRANSPARENT)
                                    color_down: (COLOR_TRANSPARENT)
                                    border_size: 0.0
                                }
                                draw_icon +: {
                                    svg: (mod.widgets.ICON_EYE_OPEN),
                                    color: (RBX_FG_TERTIARY),
                                }
                                icon_walk: Walk{width: 20, height: 20}
                                text: ""
                            }
                        }
                    }

                    homeserver_field_label := Label {
                        visible: false
                        width: Fill{max: 422}, height: Fit
                        draw_text +: {
                            color: (RBX_NAV_FG)
                            text_style: REGULAR_TEXT {font_size: 11.0}
                        }
                        text: "Homeserver URL"
                    }

                    View {
                        width: Fill{max: 422}, height: Fit,
                        flow: Down,
                        spacing: 5.0

                        homeserver_input := mod.widgets.LoginTextInput {
                            width: Fill, height: (RBX_CONTROL_H_LG),
                            flow: Right, // do not wrap
                            padding: Inset{top: 10, bottom: 10, left: 14, right: 14}
                            empty_text: "matrix.org"
                            draw_text +: {
                                text_style: TITLE_TEXT {font_size: 10.0}
                            }
                        }

                        homeserver_hint_row := View {
                            width: 422,
                            height: 16,
                            flow: Right,
                            padding: 0
                            spacing: 0.0
                            align: Align{x: 0.5, y: 0.5} // center horizontally and vertically

                            homeserver_hint_label := Label {
                                width: Fit, height: Fit
                                padding: 0
                                draw_text +: {
                                    color: (RBX_FG_TERTIARY)
                                    text_style: REGULAR_TEXT {font_size: 9}
                                }
                                text: "Homeserver URL (optional)"
                            }
                        }
                    }

                    login_button := RobrixIconButton {
                        width: Fill{max: 422},
                        height: (RBX_CONTROL_H_LG)
                        padding: 10
                        margin: Inset{top: 8, bottom: 8}
                        align: Align{x: 0.5, y: 0.5}
                        draw_bg +: {
                            color: (RBX_ACCENT)
                            color_hover: (RBX_ACCENT_HOVER)
                            color_down: (RBX_ACCENT_PRESSED)
                            border_radius: (RBX_RADIUS_SM)
                            // Same-color 1px border smooths the rounded outer edge
                            // (a fill-only SDF edge aliases against the white card).
                            border_size: 1.0
                            border_color: (RBX_ACCENT)
                            border_color_hover: (RBX_ACCENT_HOVER)
                            border_color_down: (RBX_ACCENT_PRESSED)
                        }
                        draw_icon +: {
                            svg: (ICON_LOCK)
                            color: (RBX_FG_ON_ACCENT)
                        }
                        icon_walk: Walk{width: 15, height: 15, margin: Inset{right: 5}}
                        draw_text +: {
                            color: (RBX_FG_ON_ACCENT)
                            color_hover: (RBX_FG_ON_ACCENT)
                            color_down: (RBX_FG_ON_ACCENT)
                            text_style: TITLE_TEXT {font_size: 12.0}
                        }
                        text: "Sign in securely"
                    }

                    // MAS (OIDC) login branch. Hidden by default; the Rust
                    // side flips visibility on CapabilityProbeAction::Discovered
                    // when login_mode resolves to MasOidc. Mirrors the register
                    // screen's "browser sign-in" affordance so the two entry
                    // points feel consistent.
                    oidc_card := View {
                        visible: false
                        width: Fill{max: 422}, height: Fit,
                        flow: Down,
                        spacing: 8,
                        margin: Inset{top: 5, bottom: 10}

                        oidc_info_title := Label {
                            width: Fill, height: Fit
                            draw_text +: {
                                color: (COLOR_TEXT)
                                text_style: TITLE_TEXT {font_size: 11.0}
                            }
                            text: "Browser sign-in required"
                        }

                        oidc_info_body := Label {
                            width: Fill, height: Fit
                            draw_text +: {
                                color: (RBX_FG_TERTIARY)
                                text_style: REGULAR_TEXT {font_size: 10.0}
                            }
                            text: ""
                        }

                        oidc_continue_button := RobrixIconButton {
                            width: Fill,
                            height: (RBX_CONTROL_H_LG)
                            padding: 10
                            align: Align{x: 0.5, y: 0.5}
                            text: "Continue in browser"
                        }

                        oidc_status_label := Label {
                            visible: false
                            width: Fill, height: Fit
                            draw_text +: {
                                color: (COLOR_TEXT)
                                text_style: REGULAR_TEXT {font_size: 10.0}
                            }
                            text: ""
                        }

                        oidc_cancel_button := RobrixIconButton {
                            visible: false
                            width: Fill,
                            height: (RBX_CONTROL_H_LG)
                            padding: 10
                            align: Align{x: 0.5, y: 0.5}
                            text: "Cancel sign-in"
                        }
                    }

                    View {
                        width: Fill{max: 422}, height: Fit,
                        flow: Right,
                        spacing: 8.0,
                        margin: Inset{top: 6}
                        align: Align{x: 0.5, y: 0.5}

                        LineH { draw_bg.color: (RBX_STROKE_SOFT) }

                        sso_prompt_label := Label {
                            width: Fit, height: Fit
                            padding: 0,
                            draw_text +: {
                                color: (RBX_FG_SECONDARY)
                                text_style: REGULAR_TEXT {font_size: 10.0}
                            }
                            text: "Or continue with"
                        }

                        LineH { draw_bg.color: (RBX_STROKE_SOFT) }
                    }

                    sso_view := View {
                        width: Fill{max: 170}, height: Fit,
                        margin: Inset{top: 8}
                        align: Align{x: 0.5, y: 0.5}
                        flow: Flow.Right{wrap: true},
                        spacing: 0.0,
                        apple_button := mod.widgets.SsoButton {
                            image := mod.widgets.SsoImage {
                                src: crate_resource("self://resources/img/apple.png")
                            }
                            sso_label := Label {
                                visible: false
                                width: Fit, height: Fit
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    text_style: RBX_TEXT_BODY_STRONG {font_size: 10.0}
                                }
                                text: "Apple"
                            }
                        }
                        facebook_button := mod.widgets.SsoButton {
                            image := mod.widgets.SsoImage {
                                src: crate_resource("self://resources/img/facebook.png")
                            }
                            sso_label := Label {
                                visible: false
                                width: Fit, height: Fit
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    text_style: RBX_TEXT_BODY_STRONG {font_size: 10.0}
                                }
                                text: "Facebook"
                            }
                        }
                        github_button := mod.widgets.SsoButton {
                            image := mod.widgets.SsoImage {
                                src: crate_resource("self://resources/img/github.png")
                            }
                            sso_label := Label {
                                visible: false
                                width: Fit, height: Fit
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    text_style: RBX_TEXT_BODY_STRONG {font_size: 10.0}
                                }
                                text: "GitHub"
                            }
                        }
                        gitlab_button := mod.widgets.SsoButton {
                            image := mod.widgets.SsoImage {
                                src: crate_resource("self://resources/img/gitlab.png")
                            }
                            sso_label := Label {
                                visible: false
                                width: Fit, height: Fit
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    text_style: RBX_TEXT_BODY_STRONG {font_size: 10.0}
                                }
                                text: "GitLab"
                            }
                        }
                        google_button := mod.widgets.SsoButton {
                            image := mod.widgets.SsoImage {
                                src: crate_resource("self://resources/img/google.png")
                            }
                            sso_label := Label {
                                visible: false
                                width: Fit, height: Fit
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    text_style: RBX_TEXT_BODY_STRONG {font_size: 10.0}
                                }
                                text: "Google"
                            }
                        }
                        twitter_button := mod.widgets.SsoButton {
                            image := mod.widgets.SsoImage {
                                src: crate_resource("self://resources/img/x.png")
                            }
                            sso_label := Label {
                                visible: false
                                width: Fit, height: Fit
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    text_style: RBX_TEXT_BODY_STRONG {font_size: 10.0}
                                }
                                text: "X"
                            }
                        }
                    }

                    View {
                        width: Fill{max: 422},
                        height: Fit,
                        flow: Right,
                        spacing: 4.0,
                        margin: Inset{top: 8}
                        align: Align{x: 0.5, y: 0.5} // center horizontally and vertically

                        account_prompt_label := Label {
                            width: Fit, height: Fit
                            padding: 0,
                            draw_text +: {
                                color: (RBX_FG_SECONDARY)
                                text_style: REGULAR_TEXT {font_size: 10.0}
                            }
                            text: "New to Robrix?"
                        }

                        mode_toggle_button := RobrixIconButton {
                            width: Fit, height: Fit
                            padding: Inset{left: 4, right: 4, top: 4, bottom: 4}
                            align: Align{x: 0.5, y: 0.5}
                            draw_bg +: {
                                color: (COLOR_TRANSPARENT)
                                color_hover: (COLOR_TRANSPARENT)
                                color_down: (COLOR_TRANSPARENT)
                                border_color: (COLOR_TRANSPARENT)
                                border_color_hover: (COLOR_TRANSPARENT)
                                border_color_down: (COLOR_TRANSPARENT)
                            }
                            draw_text +: {
                                color: (RBX_ACCENT)
                                color_hover: (RBX_ACCENT_HOVER)
                                color_down: (RBX_ACCENT_PRESSED)
                                text_style: TITLE_TEXT {font_size: 11.0}
                            }
                            text: "Create an account"
                        }
                    }

                    // Cancel button for add-account mode (hidden by default)
                    cancel_button := RobrixIconButton {
                        width: Fit, height: Fit,
                        padding: Inset{left: 15, right: 15, top: 10, bottom: 10}
                        margin: Inset{top: 10, bottom: 5}
                        align: Align{x: 0.5, y: 0.5}
                        text: "Cancel"
                        visible: false
                    }

                    // Reassuring status footer (decorative; mirrors the reference design).
                    desktop_status_divider := LineH {
                        width: Fill{max: 422}
                        margin: Inset{top: 12, bottom: 8}
                        draw_bg.color: (RBX_DIVIDER)
                    }

                    desktop_status_footer := View {
                        width: Fill{max: 422}, height: Fit
                        flow: Right
                        align: Align{x: 0.5, y: 0.5}
                        spacing: 8.0

                        footer_secure_label := Label {
                            width: Fit, height: Fit
                            draw_text +: {
                                color: (RBX_FG_TERTIARY)
                                text_style: REGULAR_TEXT {font_size: 9.0}
                            }
                            text: "Secure session"
                        }
                        Label {
                            width: Fit, height: Fit
                            draw_text +: {
                                color: (RBX_FG_TERTIARY)
                                text_style: REGULAR_TEXT {font_size: 9.0}
                            }
                            text: "·"
                        }
                        footer_selfhost_label := Label {
                            width: Fit, height: Fit
                            draw_text +: {
                                color: (RBX_FG_TERTIARY)
                                text_style: REGULAR_TEXT {font_size: 9.0}
                            }
                            text: "Self-host ready"
                        }
                        Label {
                            width: Fit, height: Fit
                            draw_text +: {
                                color: (RBX_FG_TERTIARY)
                                text_style: REGULAR_TEXT {font_size: 9.0}
                            }
                            text: "·"
                        }
                        footer_matrix_label := Label {
                            width: Fit, height: Fit
                            draw_text +: {
                                color: (RBX_FG_TERTIARY)
                                text_style: REGULAR_TEXT {font_size: 9.0}
                            }
                            text: "Matrix connected"
                        }
                    }
                    } // end centered content column
                }

                mobile_status_footer := View {
                    visible: false
                    width: Fill{max: 390}, height: Fit
                    margin: Inset{top: 18, left: 18, right: 18}
                    flow: Right
                    align: Align{x: 0.5, y: 0.5}
                    spacing: 8.0

                    mobile_footer_secure_label := Label {
                        width: Fit, height: Fit
                        draw_text +: {
                            color: (RBX_FG_TERTIARY)
                            text_style: REGULAR_TEXT {font_size: 9.5}
                        }
                        text: "Secure session"
                    }
                    Label {
                        width: Fit, height: Fit
                        draw_text +: {
                            color: (RBX_FG_TERTIARY)
                            text_style: REGULAR_TEXT {font_size: 9.5}
                        }
                        text: "·"
                    }
                    mobile_footer_selfhost_label := Label {
                        width: Fit, height: Fit
                        draw_text +: {
                            color: (RBX_FG_TERTIARY)
                            text_style: REGULAR_TEXT {font_size: 9.5}
                        }
                        text: "Self-host ready"
                    }
                    Label {
                        width: Fit, height: Fit
                        draw_text +: {
                            color: (RBX_FG_TERTIARY)
                            text_style: REGULAR_TEXT {font_size: 9.5}
                        }
                        text: "·"
                    }
                    mobile_footer_matrix_label := Label {
                        width: Fit, height: Fit
                        draw_text +: {
                            color: (RBX_FG_TERTIARY)
                            text_style: REGULAR_TEXT {font_size: 9.5}
                        }
                        text: "Matrix connected"
                    }
                }

                mobile_version_label := Label {
                    visible: false
                    width: Fit, height: Fit
                    margin: Inset{top: 20}
                    draw_text +: {
                        color: (RBX_FG_TERTIARY)
                        text_style: REGULAR_TEXT {font_size: 12.0}
                    }
                    text: "v2.0.0"
                }

                // The modal that pops up to display login status messages,
                // such as when the user is logging in or when there is an error.
                login_status_modal := Modal {
                    can_dismiss: false,
                    content +: {
                        login_status_modal_inner := mod.widgets.LoginStatusModal {}
                    }
                }

                proxy_settings_modal := Modal {
                    can_dismiss: true,
                    content +: {
                        proxy_settings_modal_inner := RoundedView {
                            width: 380, height: Fit,
                            flow: Down
                            spacing: 14.0
                            padding: Inset{top: 20, left: 20, right: 20, bottom: 20}
                            show_bg: true
                            draw_bg +: {
                                color: (COLOR_PRIMARY)
                                border_radius: 8.0
                                border_size: 1.0
                                border_color: (RBX_STROKE_SOFT)
                            }

                            proxy_settings_header := View {
                                width: Fill, height: Fit,
                                flow: Right,
                                align: Align{x: 1.0, y: 0.5}
                                spacing: 8.0

                                proxy_settings_title := TitleLabel {
                                    width: Fill, height: Fit
                                    margin: Inset{top: 0}
                                    text: "Network proxy settings"
                                }

                                proxy_settings_close_button := RobrixNeutralIconButton {
                                    width: Fit, height: Fit
                                    padding: Inset{left: 6, right: 6, top: 6, bottom: 6}
                                    spacing: 0
                                    text: ""
                                    icon_walk: Walk{width: 14, height: 14, margin: 0}
                                    label_walk: Walk{width: 0, height: 0, margin: 0}
                                    draw_icon.svg: (ICON_CLOSE)
                                }
                            }

                            proxy_use_card := RoundedView {
                                width: Fill, height: Fit,
                                flow: Down
                                show_bg: true
                                draw_bg +: {
                                    color: (RBX_BG_SURFACE_SUBTLE)
                                    border_radius: (RADIUS_LG)
                                }
                                padding: Inset{left: (SPACE_MD), right: (SPACE_MD), top: (SPACE_SM), bottom: (SPACE_SM)}

                                View {
                                    width: Fill, height: Fit
                                    flow: Right
                                    align: Align{x: 1.0, y: 0.5}

                                    proxy_use_label := SubsectionLabel {
                                        margin: Inset{top: 0, bottom: 0}
                                        text: "Use proxy"
                                    }

                                    proxy_use_toggle := Toggle {
                                        width: Fit
                                        height: Fit
                                        padding: Inset{top: (SPACE_SM), right: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_SM)}
                                        text: ""
                                        active: false
                                        draw_bg +: {
                                            size: 20.0
                                            color_active: (RBX_ACCENT)
                                            border_color_active: (RBX_ACCENT)
                                            mark_color_active: (RBX_FG_ON_ACCENT)
                                        }
                                    }
                                }

                                proxy_fields_section := View {
                                    visible: false
                                    width: Fill, height: Fit,
                                    flow: Down
                                    spacing: 0

                                    proxy_address_row := View {
                                    width: Fill, height: Fit,
                                    flow: Right
                                    align: Align{y: 0.5}
                                    spacing: 8.0
                                    padding: Inset{top: 8, bottom: 8}

                                    proxy_address_label := Label {
                                        width: 90, height: Fit
                                        draw_text +: {
                                            color: (COLOR_TEXT)
                                            text_style: REGULAR_TEXT {font_size: 12}
                                        }
                                        text: "Address"
                                    }

                                    proxy_address_input := RobrixTextInput {
                                        width: Fill, height: Fit,
                                        flow: Right,
                                        empty_text: "127.0.0.1"
                                        padding: Inset{top: 5, bottom: 5, left: 10, right: 10}
                                    }
                                }

                                proxy_port_row := View {
                                    width: Fill, height: Fit,
                                    flow: Right
                                    align: Align{y: 0.5}
                                    spacing: 8.0
                                    padding: Inset{top: 8, bottom: 8}

                                    proxy_port_label := Label {
                                        width: 90, height: Fit
                                        draw_text +: {
                                            color: (COLOR_TEXT)
                                            text_style: REGULAR_TEXT {font_size: 12}
                                        }
                                        text: "Port"
                                    }

                                    proxy_port_input := RobrixTextInput {
                                        width: Fill, height: Fit,
                                        flow: Right,
                                        empty_text: "7890"
                                        padding: Inset{top: 5, bottom: 5, left: 10, right: 10}
                                    }
                                }

                                proxy_account_row := View {
                                    width: Fill, height: Fit,
                                    flow: Right
                                    align: Align{y: 0.5}
                                    spacing: 8.0
                                    padding: Inset{top: 8, bottom: 8}

                                    proxy_account_label := Label {
                                        width: 90, height: Fit
                                        draw_text +: {
                                            color: (COLOR_TEXT)
                                            text_style: REGULAR_TEXT {font_size: 12}
                                        }
                                        text: "Account"
                                    }

                                    proxy_account_input := RobrixTextInput {
                                        width: Fill, height: Fit,
                                        flow: Right,
                                        empty_text: ""
                                        padding: Inset{top: 5, bottom: 5, left: 10, right: 10}
                                    }
                                }

                                proxy_password_row := View {
                                    width: Fill, height: Fit,
                                    flow: Right
                                    align: Align{y: 0.5}
                                    spacing: 8.0
                                    padding: Inset{top: 8, bottom: 8}

                                    proxy_password_label := Label {
                                        width: 90, height: Fit
                                        draw_text +: {
                                            color: (COLOR_TEXT)
                                            text_style: REGULAR_TEXT {font_size: 12}
                                        }
                                        text: "Password"
                                    }

                                    proxy_password_input := RobrixTextInput {
                                        width: Fill, height: Fit,
                                        flow: Right,
                                        empty_text: ""
                                        is_password: true,
                                        padding: Inset{top: 5, bottom: 5, left: 10, right: 10}
                                    }
                                }
                            }
                            }

                            proxy_settings_error_label := Label {
                                visible: false
                                width: Fill, height: Fit
                                margin: Inset{top: 0, bottom: 0, left: 2, right: 2}
                                draw_text +: {
                                    color: (COLOR_TEXT_WARNING_NOT_FOUND)
                                    text_style: REGULAR_TEXT {font_size: 11}
                                    wrap: Words
                                }
                                text: ""
                            }

                            proxy_settings_save_button_row := View {
                                width: Fill, height: Fit
                                flow: Right
                                align: Align{x: 0.5, y: 0.5}
                                margin: Inset{top: 2}

                                proxy_settings_save_button := RobrixIconButton {
                                    width: 160, height: 42
                                    align: Align{x: 0.5, y: 0.5}
                                    text: "Save Proxy"
                                }
                            }
                        }
                    }
                }
            }

        }

        proxy_settings_button_anchor := View {
            width: Fill, height: Fill
            flow: Down
            align: Align{x: 0.0, y: 0.0}

            View {
                width: Fill, height: Fit
                flow: Right
                padding: Inset{top: 10, right: 10}

                View {
                    width: Fill, height: Fit
                }

                proxy_settings_button := RobrixNeutralIconButton {
                    width: Fit, height: Fit
                    spacing: 0
                    padding: 8
                    text: ""
                    label_walk: Walk{width: 0, height: 0, margin: 0}
                    icon_walk: Walk{width: 14, height: 14, margin: 0}
                    draw_icon.svg: (ICON_SETTINGS)
                }
            }
        }
    }
}

#[derive(Script, Widget)]
pub struct LoginScreen {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    /// Whether the password field is currently showing plaintext.
    #[rust] password_visible: bool,
    /// Whether the confirm password field is currently showing plaintext.
    #[rust] confirm_password_visible: bool,
    /// Boolean to indicate if the SSO login process is still in flight
    #[rust] sso_pending: bool,
    /// The URL to redirect to after logging in with SSO.
    #[rust] sso_redirect_url: Option<String>,
    /// The most recent login failure message shown to the user.
    #[rust] last_failure_message_shown: Option<String>,
    /// Register flow owns login/setup failures while the login screen is hidden.
    #[rust] suppress_login_failure_modal: bool,
    #[rust] app_language: AppLanguage,
    /// Boolean to indicate if we're in "add account" mode (adding another Matrix account).
    #[rust] adding_account: bool,
    #[rust] use_proxy_enabled: bool,
    /// Classified login flavor for the current homeserver input, once a
    /// capability probe has completed. None while unresolved or after the
    /// user edits the homeserver field.
    #[rust] login_mode: Option<LoginMode>,
    /// Normalized URL we last dispatched a probe for. Used to (a) drop
    /// out-of-order probe responses from superseded clicks, and (b) detect
    /// when the user has edited the homeserver field since the last probe.
    #[rust] last_discovery_input_url: Option<String>,
    /// True between probe-dispatch and probe-result. Blocks duplicate probes
    /// from rapid clicking and keeps the button's "Checking..." label honest.
    #[rust] discovery_pending: bool,
    /// True while the OIDC browser flow is in flight. Blocks re-probes and
    /// re-entry into start_oidc_login from duplicate clicks.
    #[rust] oidc_in_flight: bool,
    /// Cached responsive layout mode. None means the first window geometry
    /// event still needs to apply either desktop or mobile styling.
    #[rust] mobile_layout_active: Option<bool>,
}

impl LoginScreen {
    fn sync_proxy_settings_modal_layout(&mut self, cx: &mut Cx) {
        let rect = self.view.area().rect(cx);
        let available_width = (rect.size.x - 24.0).max(260.0);
        let modal_width = available_width.min(380.0);
        let mut proxy_settings_modal_inner = self.view.view(cx, ids!(proxy_settings_modal_inner));
        script_apply_eval!(cx, proxy_settings_modal_inner, {
            width: #(modal_width)
        });
    }

    fn set_login_input_copy(&mut self, cx: &mut Cx) {
        let mobile = self.mobile_layout_active.unwrap_or(false);
        let user_id_key = if mobile { "login.input.mobile.user_id" } else { "login.input.user_id" };
        let password_key = if mobile { "login.input.mobile.password" } else { "login.input.password" };
        let homeserver_key = if mobile { "login.input.mobile.homeserver" } else { "login.input.homeserver" };

        self.view.text_input(cx, ids!(user_id_input))
            .set_empty_text(cx, tr_key(self.app_language, user_id_key).to_string());
        self.view.text_input(cx, ids!(password_input))
            .set_empty_text(cx, tr_key(self.app_language, password_key).to_string());
        self.view.text_input(cx, ids!(homeserver_input))
            .set_empty_text(cx, tr_key(self.app_language, homeserver_key).to_string());
        self.view.label(cx, ids!(user_id_field_label))
            .set_text(cx, tr_key(self.app_language, "login.input.user_id"));
        self.view.label(cx, ids!(password_field_label))
            .set_text(cx, tr_key(self.app_language, "login.input.password"));
        self.view.label(cx, ids!(homeserver_field_label))
            .set_text(cx, tr_key(self.app_language, "login.label.homeserver_url"));
    }

    fn sync_login_responsive_layout(&mut self, cx: &mut Cx) {
        let mobile_runtime_target = is_mobile_runtime_target();
        let rect = self.view.area().rect(cx);
        let width = rect.size.x;
        if width <= 0.0 && !mobile_runtime_target {
            return;
        }
        let mobile = is_mobile_login_layout(width, mobile_runtime_target);
        if self.mobile_layout_active == Some(mobile) {
            self.apply_login_layout(cx, mobile);
            return;
        }
        self.mobile_layout_active = Some(mobile);
        self.set_login_input_copy(cx);
        self.apply_login_layout(cx, mobile);
    }

    fn apply_login_layout(&mut self, cx: &mut Cx, mobile: bool) {
        self.view.view(cx, ids!(agent_badge)).set_visible(cx, !mobile);
        self.view.view(cx, ids!(user_id_field_label)).set_visible(cx, mobile);
        self.view.view(cx, ids!(password_field_label)).set_visible(cx, mobile);
        self.view.view(cx, ids!(homeserver_field_label)).set_visible(cx, mobile);
        self.view.view(cx, ids!(homeserver_hint_row)).set_visible(cx, !mobile);
        self.view.view(cx, ids!(desktop_status_divider)).set_visible(cx, !mobile);
        self.view.view(cx, ids!(desktop_status_footer)).set_visible(cx, !mobile);
        self.view.view(cx, ids!(mobile_status_footer)).set_visible(cx, mobile);
        self.view.view(cx, ids!(mobile_version_label)).set_visible(cx, mobile);

        self.apply_login_surface_style(cx, mobile);
        self.apply_login_text_style(cx, mobile);
        self.apply_login_input_style(cx, mobile);
        self.apply_sso_layout_style(cx, mobile);
    }

    fn apply_login_surface_style(&mut self, cx: &mut Cx, mobile: bool) {
        let mut login_scroll = self.view.view(cx, ids!(login_scroll));
        let mut login_card = self.view.view(cx, ids!(login_card));
        if mobile {
            script_apply_eval!(cx, login_scroll, {
                draw_bg +: {
                    color: mod.widgets.RBX_BG_CANVAS
                }
            });
            script_apply_eval!(cx, login_card, {
                margin: Inset{left: 28, right: 28}
                padding: Inset{top: 24, bottom: 24, left: 22, right: 22}
                draw_bg +: {
                    color: mod.widgets.RBX_BG_SURFACE
                    border_color: mod.widgets.RBX_STROKE_SOFT
                    border_radius: mod.widgets.RBX_RADIUS_XL
                }
            });
        } else {
            script_apply_eval!(cx, login_scroll, {
                draw_bg +: {
                    color: mod.widgets.RBX_BG_CANVAS
                }
            });
            script_apply_eval!(cx, login_card, {
                margin: Inset{left: 16, right: 16}
                padding: Inset{top: 24, bottom: 24, left: 36, right: 36}
                draw_bg +: {
                    color: mod.widgets.RBX_BG_SURFACE
                    border_color: mod.widgets.RBX_STROKE_SOFT
                    border_radius: mod.widgets.RBX_RADIUS_LG
                }
            });
        }
    }

    fn apply_login_text_style(&mut self, cx: &mut Cx, mobile: bool) {
        let mut brand_wordmark = self.view.label(cx, ids!(brand_wordmark));
        let mut title = self.view.label(cx, ids!(title));
        let mut sso_prompt_label = self.view.label(cx, ids!(sso_prompt_label));
        let mut account_prompt_label = self.view.label(cx, ids!(account_prompt_label));
        let mut mode_toggle_button = self.view.button(cx, ids!(mode_toggle_button));
        if mobile {
            script_apply_eval!(cx, brand_wordmark, {
                draw_text +: {
                    color: mod.widgets.RBX_FG_PRIMARY
                    text_style: mod.widgets.RBX_TEXT_PAGE_TITLE {font_size: 34.0}
                }
            });
            script_apply_eval!(cx, title, {
                draw_text +: {
                    color: mod.widgets.RBX_FG_SECONDARY
                    text_style: mod.widgets.RBX_TEXT_BODY {font_size: 13.0}
                }
            });
            script_apply_eval!(cx, sso_prompt_label, {
                draw_text +: {
                    color: mod.widgets.RBX_FG_SECONDARY
                    text_style: mod.widgets.RBX_TEXT_BODY {font_size: 11.0}
                }
            });
            script_apply_eval!(cx, account_prompt_label, {
                draw_text +: {
                    color: mod.widgets.RBX_FG_SECONDARY
                    text_style: mod.widgets.RBX_TEXT_BODY {font_size: 10.5}
                }
            });
            script_apply_eval!(cx, mode_toggle_button, {
                draw_text +: {
                    color: mod.widgets.RBX_ACCENT
                    color_hover: mod.widgets.RBX_ACCENT_HOVER
                    color_down: mod.widgets.RBX_ACCENT_PRESSED
                    text_style: mod.widgets.RBX_TEXT_BODY_STRONG {font_size: 14.0}
                }
            });
        } else {
            script_apply_eval!(cx, brand_wordmark, {
                draw_text +: {
                    color: mod.widgets.RBX_FG_PRIMARY
                    text_style: mod.widgets.RBX_TEXT_PAGE_TITLE {font_size: 22.0}
                }
            });
            script_apply_eval!(cx, title, {
                draw_text +: {
                    color: mod.widgets.RBX_FG_SECONDARY
                    text_style: mod.widgets.RBX_TEXT_BODY {font_size: 10.5}
                }
            });
            script_apply_eval!(cx, sso_prompt_label, {
                draw_text +: {
                    color: mod.widgets.RBX_FG_SECONDARY
                    text_style: mod.widgets.RBX_TEXT_BODY {font_size: 10.0}
                }
            });
            script_apply_eval!(cx, account_prompt_label, {
                draw_text +: {
                    color: mod.widgets.RBX_FG_SECONDARY
                    text_style: mod.widgets.RBX_TEXT_BODY {font_size: 10.0}
                }
            });
            script_apply_eval!(cx, mode_toggle_button, {
                draw_text +: {
                    color: mod.widgets.RBX_ACCENT
                    color_hover: mod.widgets.RBX_ACCENT_HOVER
                    color_down: mod.widgets.RBX_ACCENT_PRESSED
                    text_style: mod.widgets.RBX_TEXT_BODY_STRONG {font_size: 11.0}
                }
            });
        }
    }

    fn apply_login_input_style(&mut self, cx: &mut Cx, mobile: bool) {
        let input_ids: &[&[LiveId]] = ids_array!(
            user_id_input,
            password_input,
            confirm_password_input,
            homeserver_input
        );
        for input_ref in self.view_set(cx, input_ids).iter() {
            let Some(mut input) = input_ref.borrow_mut() else { continue };
            if mobile {
                script_apply_eval!(cx, input, {
                    padding: Inset{top: 10, bottom: 10, left: 16, right: 16}
                    draw_bg +: {
                        color: mod.widgets.RBX_BG_SURFACE
                        color_hover: mod.widgets.RBX_BG_SURFACE
                        color_focus: mod.widgets.RBX_BG_SURFACE
                        color_down: mod.widgets.RBX_BG_SURFACE
                        color_empty: mod.widgets.RBX_BG_SURFACE
                        border_color: mod.widgets.RBX_STROKE_SOFT
                        border_color_hover: mod.widgets.RBX_STROKE_STRONG
                        border_color_focus: mod.widgets.RBX_ACCENT
                        border_color_down: mod.widgets.RBX_STROKE_STRONG
                        border_color_empty: mod.widgets.RBX_STROKE_SOFT
                    }
                    draw_cursor +: {
                        color: mod.widgets.RBX_ACCENT
                    }
                    draw_text +: {
                        color: mod.widgets.RBX_FG_PRIMARY
                        color_hover: mod.widgets.RBX_FG_PRIMARY
                        color_focus: mod.widgets.RBX_FG_PRIMARY
                        color_down: mod.widgets.RBX_FG_PRIMARY
                        color_empty: mod.widgets.RBX_FG_TERTIARY
                        color_empty_hover: mod.widgets.RBX_FG_TERTIARY
                        color_empty_focus: mod.widgets.RBX_FG_TERTIARY
                    }
                });
            } else {
                script_apply_eval!(cx, input, {
                    padding: Inset{top: 10, bottom: 10, left: 14, right: 14}
                    draw_bg +: {
                        color: mod.widgets.RBX_BG_SURFACE
                        color_hover: mod.widgets.RBX_BG_SURFACE
                        color_focus: mod.widgets.RBX_BG_SURFACE
                        color_down: mod.widgets.RBX_BG_SURFACE
                        color_empty: mod.widgets.RBX_BG_SURFACE
                        border_color: mod.widgets.RBX_STROKE_SOFT
                        border_color_hover: mod.widgets.RBX_STROKE_STRONG
                        border_color_focus: mod.widgets.RBX_ACCENT
                        border_color_down: mod.widgets.RBX_STROKE_STRONG
                        border_color_empty: mod.widgets.RBX_STROKE_SOFT
                    }
                    draw_cursor +: {
                        color: mod.widgets.RBX_ACCENT
                    }
                    draw_text +: {
                        color: mod.widgets.RBX_FG_PRIMARY
                        color_hover: mod.widgets.RBX_FG_PRIMARY
                        color_focus: mod.widgets.RBX_FG_PRIMARY
                        color_down: mod.widgets.RBX_FG_PRIMARY
                        color_empty: mod.widgets.RBX_FG_TERTIARY
                        color_empty_hover: mod.widgets.RBX_FG_TERTIARY
                        color_empty_focus: mod.widgets.RBX_FG_TERTIARY
                    }
                });
            }
        }
    }

    fn apply_sso_layout_style(&mut self, cx: &mut Cx, mobile: bool) {
        let mut sso_view = self.view.view(cx, ids!(sso_view));
        let sso_width = if mobile { 170.0 } else { 322.0 };
        if mobile {
            script_apply_eval!(cx, sso_view, {
                width: #(sso_width)
            });
        } else {
            script_apply_eval!(cx, sso_view, {
                width: #(sso_width)
            });
        }
    }

    fn set_sso_pending_state(&mut self, cx: &mut Cx, pending: bool) {
        let mask = if pending { 1.0 } else { 0.0 };
        let cursor = if pending { MouseCursor::NotAllowed } else { MouseCursor::Hand };
        let button_set: &[&[LiveId]] = ids_array!(
            apple_button,
            facebook_button,
            github_button,
            gitlab_button,
            google_button,
            twitter_button
        );
        for view_ref in self.view_set(cx, button_set).iter() {
            let Some(mut view_mut) = view_ref.borrow_mut() else { continue };
            let mut image = view_mut.image(cx, ids!(image));
            script_apply_eval!(cx, image, {
                draw_bg.mask: #(mask)
            });
            view_mut.cursor = Some(cursor);
        }
        self.sso_pending = pending;
    }

    fn reset_sso_state(&mut self, cx: &mut Cx) {
        self.sso_redirect_url = None;
        self.set_sso_pending_state(cx, false);
    }

    fn set_app_language(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        self.app_language = app_language;
        self.set_login_input_copy(cx);
        self.view.text_input(cx, ids!(proxy_address_input))
            .set_empty_text(cx, tr_key(self.app_language, "login.proxy_settings.input.address").to_string());
        self.view.text_input(cx, ids!(proxy_port_input))
            .set_empty_text(cx, tr_key(self.app_language, "login.proxy_settings.input.port").to_string());
        self.view.text_input(cx, ids!(proxy_account_input))
            .set_empty_text(cx, tr_key(self.app_language, "login.proxy_settings.input.account").to_string());
        self.view.text_input(cx, ids!(proxy_password_input))
            .set_empty_text(cx, tr_key(self.app_language, "login.proxy_settings.input.password").to_string());
        self.view.label(cx, ids!(homeserver_hint_label))
            .set_text(cx, tr_key(self.app_language, "login.label.homeserver_optional"));
        self.view.label(cx, ids!(proxy_settings_title))
            .set_text(cx, tr_key(self.app_language, "login.proxy_settings.title"));
        self.view.label(cx, ids!(proxy_use_label))
            .set_text(cx, tr_key(self.app_language, "login.proxy_settings.use_proxy"));
        self.view.label(cx, ids!(proxy_address_label))
            .set_text(cx, tr_key(self.app_language, "login.proxy_settings.address"));
        self.view.label(cx, ids!(proxy_port_label))
            .set_text(cx, tr_key(self.app_language, "login.proxy_settings.port"));
        self.view.label(cx, ids!(proxy_account_label))
            .set_text(cx, tr_key(self.app_language, "login.proxy_settings.account"));
        self.view.label(cx, ids!(proxy_password_label))
            .set_text(cx, tr_key(self.app_language, "login.proxy_settings.password"));
        self.view.button(cx, ids!(proxy_settings_save_button))
            .set_text(cx, tr_key(self.app_language, "login.proxy_settings.save"));
        self.view.label(cx, ids!(sso_prompt_label))
            .set_text(cx, tr_key(self.app_language, "login.divider.or_continue_with"));
        let login_status_modal_inner = self.view.login_status_modal(cx, ids!(login_status_modal_inner));
        login_status_modal_inner.set_title(cx, tr_key(self.app_language, "login_status_modal.title"));
        login_status_modal_inner.button_ref(cx).set_text(cx, tr_key(self.app_language, "login_status_modal.button.cancel"));
        self.view.label(cx, ids!(title))
            .set_text(cx, tr_key(self.app_language, "login.subtitle.tagline"));
        self.view.button(cx, ids!(login_button))
            .set_text(cx, tr_key(self.app_language, "login.button.sign_in_securely"));
        self.view.label(cx, ids!(account_prompt_label))
            .set_text(cx, tr_key(self.app_language, "login.account_prompt.new_to_robrix"));
        self.view.button(cx, ids!(mode_toggle_button))
            .set_text(cx, tr_key(self.app_language, "login.mode_toggle.create_account"));
        self.view.label(cx, ids!(footer_secure_label))
            .set_text(cx, tr_key(self.app_language, "login.footer.secure_session"));
        self.view.label(cx, ids!(footer_selfhost_label))
            .set_text(cx, tr_key(self.app_language, "login.footer.self_host_ready"));
        self.view.label(cx, ids!(footer_matrix_label))
            .set_text(cx, tr_key(self.app_language, "login.footer.matrix_connected"));
        self.view.label(cx, ids!(mobile_footer_secure_label))
            .set_text(cx, tr_key(self.app_language, "login.footer.secure_session"));
        self.view.label(cx, ids!(mobile_footer_selfhost_label))
            .set_text(cx, tr_key(self.app_language, "login.footer.self_host_ready"));
        self.view.label(cx, ids!(mobile_footer_matrix_label))
            .set_text(cx, tr_key(self.app_language, "login.footer.matrix_connected"));
        self.view.label(cx, ids!(agent_badge_label))
            .set_text(cx, tr_key(self.app_language, "login.badge.agent_ready"));
    }

    fn set_use_proxy_enabled(&mut self, cx: &mut Cx, enabled: bool) {
        self.use_proxy_enabled = enabled;
        self.view
            .check_box(cx, ids!(proxy_use_toggle))
            .set_active(cx, enabled, Animate::No);
        self.view
            .view(cx, ids!(proxy_fields_section))
            .set_visible(cx, enabled);
        self.view
            .label(cx, ids!(proxy_settings_error_label))
            .set_visible(cx, false);
        self.redraw(cx);
    }

    fn load_saved_proxy_to_form(&mut self, cx: &mut Cx) {
        let saved_proxy = crate::proxy_config::load_saved_proxy_url();
        let Some(saved_proxy) = saved_proxy else {
            self.set_use_proxy_enabled(cx, false);
            self.view.text_input(cx, ids!(proxy_address_input)).set_text(cx, "");
            self.view.text_input(cx, ids!(proxy_port_input)).set_text(cx, "");
            self.view.text_input(cx, ids!(proxy_account_input)).set_text(cx, "");
            self.view.text_input(cx, ids!(proxy_password_input)).set_text(cx, "");
            return;
        };

        let Ok(parsed_url) = Url::parse(&saved_proxy) else {
            self.set_use_proxy_enabled(cx, true);
            self.view.text_input(cx, ids!(proxy_address_input)).set_text(cx, &saved_proxy);
            self.view.text_input(cx, ids!(proxy_port_input)).set_text(cx, "");
            self.view.text_input(cx, ids!(proxy_account_input)).set_text(cx, "");
            self.view.text_input(cx, ids!(proxy_password_input)).set_text(cx, "");
            return;
        };

        self.set_use_proxy_enabled(cx, true);
        self.view
            .text_input(cx, ids!(proxy_address_input))
            .set_text(cx, parsed_url.host_str().unwrap_or_default());
        self.view
            .text_input(cx, ids!(proxy_port_input))
            .set_text(cx, &parsed_url.port().map(|p| p.to_string()).unwrap_or_default());
        self.view
            .text_input(cx, ids!(proxy_account_input))
            .set_text(cx, parsed_url.username());
        self.view
            .text_input(cx, ids!(proxy_password_input))
            .set_text(cx, parsed_url.password().unwrap_or_default());
    }

    fn build_proxy_url_from_form(&mut self, cx: &mut Cx) -> Result<Option<String>, String> {
        if !self.use_proxy_enabled {
            return Ok(None);
        }

        let address = self.view.text_input(cx, ids!(proxy_address_input)).text();
        let port_text = self.view.text_input(cx, ids!(proxy_port_input)).text();
        let account = self.view.text_input(cx, ids!(proxy_account_input)).text();
        let password = self.view.text_input(cx, ids!(proxy_password_input)).text();

        let address = address.trim().to_owned();
        let port_text = port_text.trim().to_owned();
        let account = account.trim().to_owned();
        let password = password.trim().to_owned();

        if address.is_empty() {
            return Err(tr_key(self.app_language, "login.proxy_settings.error.missing_address").to_string());
        }

        if port_text.is_empty() {
            return Err(tr_key(self.app_language, "login.proxy_settings.error.missing_port").to_string());
        }

        let port: u16 = port_text
            .parse()
            .map_err(|_| tr_key(self.app_language, "login.proxy_settings.error.invalid_port").to_string())?;

        let mut proxy_url = if address.contains("://") {
            Url::parse(&address)
                .map_err(|e| format!("Invalid proxy URL: {e}"))?
        } else {
            let mut url = Url::parse("http://127.0.0.1")
                .map_err(|e| format!("Failed to initialize proxy URL builder: {e}"))?;
            url.set_host(Some(&address))
                .map_err(|e| format!("Invalid proxy address `{address}`: {e}"))?;
            url
        };

        proxy_url
            .set_port(Some(port))
            .map_err(|()| format!("Invalid proxy port `{port}`"))?;

        if account.is_empty() {
            proxy_url
                .set_username("")
                .map_err(|()| String::from("Invalid proxy account value"))?;
            proxy_url
                .set_password(None)
                .map_err(|()| String::from("Invalid proxy password value"))?;
        } else {
            proxy_url
                .set_username(&account)
                .map_err(|()| String::from("Invalid proxy account value"))?;
            if password.is_empty() {
                proxy_url
                    .set_password(None)
                    .map_err(|()| String::from("Invalid proxy password value"))?;
            } else {
                proxy_url
                    .set_password(Some(&password))
                    .map_err(|()| String::from("Invalid proxy password value"))?;
            }
        }

        let proxy_url = proxy_url.to_string();
        validate_proxy_url_for_user_input(&proxy_url).map_err(|e| match e {
            ProxyInputError::InvalidHost(host) => tr_fmt(
                self.app_language,
                "login.proxy_settings.error.invalid_host",
                &[("host", host.as_str())],
            ),
            other => other.to_string(),
        })?;
        Ok(Some(proxy_url))
    }

    fn clear_homeserver_classification(&mut self) {
        self.login_mode = None;
        self.last_discovery_input_url = None;
        self.discovery_pending = false;
    }

    fn show_password_login_branch(&mut self, cx: &mut Cx) {
        self.view.view(cx, ids!(oidc_card)).set_visible(cx, false);
        self.view.text_input(cx, ids!(user_id_input)).set_visible(cx, true);
        self.view.text_input(cx, ids!(password_input)).set_visible(cx, true);
        self.view.button(cx, ids!(login_button)).set_visible(cx, true);
        self.view.view(cx, ids!(sso_view)).set_visible(cx, true);
        self.view.label(cx, ids!(sso_prompt_label)).set_visible(cx, true);
        self.view.label(cx, ids!(oidc_info_title))
            .set_text(cx, tr_key(self.app_language, "login.oidc.info_title"));
        self.view.label(cx, ids!(oidc_info_body))
            .set_text(cx, tr_key(self.app_language, "login.oidc.info_body"));
        self.view.button(cx, ids!(oidc_continue_button))
            .set_text(cx, tr_key(self.app_language, "login.button.continue_in_browser"));
        self.view.button(cx, ids!(oidc_continue_button)).set_visible(cx, true);
        self.view.label(cx, ids!(oidc_status_label)).set_visible(cx, false);
        self.view.button(cx, ids!(oidc_cancel_button)).set_visible(cx, false);
    }

    fn show_oidc_login_branch(&mut self, cx: &mut Cx) {
        self.view.button(cx, ids!(login_button)).set_visible(cx, false);
        self.view.text_input(cx, ids!(user_id_input)).set_visible(cx, false);
        self.view.text_input(cx, ids!(password_input)).set_visible(cx, false);
        self.view.view(cx, ids!(sso_view)).set_visible(cx, false);
        self.view.label(cx, ids!(sso_prompt_label)).set_visible(cx, false);
        self.view.label(cx, ids!(oidc_info_title))
            .set_text(cx, tr_key(self.app_language, "login.oidc.info_title"));
        self.view.label(cx, ids!(oidc_info_body))
            .set_text(cx, tr_key(self.app_language, "login.oidc.info_body"));
        self.view.button(cx, ids!(oidc_continue_button))
            .set_text(cx, tr_key(self.app_language, "login.button.continue_in_browser"));
        self.view.button(cx, ids!(oidc_continue_button)).set_visible(cx, true);
        self.view.label(cx, ids!(oidc_status_label)).set_visible(cx, false);
        self.view.button(cx, ids!(oidc_cancel_button)).set_visible(cx, false);
        self.view.view(cx, ids!(oidc_card)).set_visible(cx, true);
    }

    fn reset_oidc_screen_state(&mut self, cx: &mut Cx) {
        self.oidc_in_flight = false;
        self.clear_homeserver_classification();
        self.show_password_login_branch(cx);
    }

}

impl ScriptHook for LoginScreen {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        vm.with_cx_mut(|cx| {
            self.load_saved_proxy_to_form(cx);
            self.set_app_language(cx, self.app_language);
            self.sync_proxy_settings_modal_layout(cx);
            self.sync_login_responsive_layout(cx);
        });
    }
}


impl Widget for LoginScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }
        self.view.handle_event(cx, event, scope);
        if matches!(event, Event::WindowGeomChange(_)) {
            self.sync_proxy_settings_modal_layout(cx);
            self.sync_login_responsive_layout(cx);
        }
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

impl WidgetMatchEvent for LoginScreen {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        let login_button = self.view.button(cx, ids!(login_button));
        let mode_toggle_button = self.view.button(cx, ids!(mode_toggle_button));
        let cancel_button = self.view.button(cx, ids!(cancel_button));
        let user_id_input = self.view.text_input(cx, ids!(user_id_input));
        let password_input = self.view.text_input(cx, ids!(password_input));
        let homeserver_input = self.view.text_input(cx, ids!(homeserver_input));

        let login_status_modal = self.view.modal(cx, ids!(login_status_modal));
        let login_status_modal_inner = self.view.login_status_modal(cx, ids!(login_status_modal_inner));
        let proxy_settings_modal = self.view.modal(cx, ids!(proxy_settings_modal));

        if self.view.button(cx, ids!(proxy_settings_button)).clicked(actions) {
            self.sync_proxy_settings_modal_layout(cx);
            self.view.label(cx, ids!(proxy_settings_error_label)).set_visible(cx, false);
            proxy_settings_modal.open(cx);
            self.redraw(cx);
        }

        if self.view.button(cx, ids!(proxy_settings_close_button)).clicked(actions) {
            self.view.label(cx, ids!(proxy_settings_error_label)).set_visible(cx, false);
            proxy_settings_modal.close(cx);
            self.redraw(cx);
        }

        if let Some(enabled) = self.view.check_box(cx, ids!(proxy_use_toggle)).changed(actions) {
            self.set_use_proxy_enabled(cx, enabled);
        }

        if homeserver_input.changed(actions).is_some() {
            self.clear_homeserver_classification();
            if !self.oidc_in_flight {
                self.show_password_login_branch(cx);
                self.redraw(cx);
            }
        }

        if self.view.button(cx, ids!(proxy_settings_save_button)).clicked(actions) {
            let error_label = self.view.label(cx, ids!(proxy_settings_error_label));
            match self.build_proxy_url_from_form(cx) {
                Ok(proxy_url) => {
                    if let Err(e) = crate::proxy_config::save_proxy_url(proxy_url.as_deref()) {
                        warning!("Failed to persist proxy configuration from proxy settings modal: {e}");
                        let error_text = tr_fmt(self.app_language, "login.status.invalid_proxy.body", &[
                            ("error", e.as_str()),
                        ]);
                        error_label.set_text(cx, &error_text);
                        error_label.set_visible(cx, true);
                    } else {
                        error_label.set_visible(cx, false);
                        proxy_settings_modal.close(cx);
                        login_status_modal_inner.set_title(cx, tr_key(self.app_language, "login.proxy_settings.saved.title"));
                        login_status_modal_inner.set_status(cx, tr_key(self.app_language, "login.proxy_settings.saved.body"));
                        login_status_modal_inner.button_ref(cx).set_text(cx, tr_key(self.app_language, "login.status.okay"));
                        login_status_modal.open(cx);
                    }
                    self.redraw(cx);
                }
                Err(proxy_validation_error) => {
                    let error_text = tr_fmt(self.app_language, "login.status.invalid_proxy.body", &[
                        ("error", proxy_validation_error.as_str()),
                    ]);
                    error_label.set_text(cx, &error_text);
                    error_label.set_visible(cx, true);
                    self.redraw(cx);
                }
            }
        }

        // Handle cancel button for add-account mode
        if cancel_button.clicked(actions) {
            self.adding_account = false;
            self.reset_sso_state(cx);
            self.reset_oidc_screen_state(cx);
            // Reset the UI back to normal login mode
            self.view.label(cx, ids!(title)).set_text(cx, tr_key(self.app_language, "login.subtitle.tagline"));
            cancel_button.set_visible(cx, false);
            mode_toggle_button.set_visible(cx, true);
            cx.action(LoginAction::CancelAddAccount);
            self.redraw(cx);
        }

        // Handle toggling password visibility
        let show_pw_button = self.view.button(cx, ids!(show_password_button));
        let hide_pw_button = self.view.button(cx, ids!(hide_password_button));
        if show_pw_button.clicked(actions) || hide_pw_button.clicked(actions) {
            self.password_visible = !self.password_visible;
            password_input.toggle_is_password(cx);
            show_pw_button.set_visible(cx, !self.password_visible);
            hide_pw_button.set_visible(cx, self.password_visible);
            password_input.set_key_focus(cx);
            self.redraw(cx);
        }

        // Handle toggling confirm password visibility
        let confirm_password_input = self.view.text_input(cx, ids!(confirm_password_input));
        let show_confirm_pw_button = self.view.button(cx, ids!(show_confirm_password_button));
        let hide_confirm_pw_button = self.view.button(cx, ids!(hide_confirm_password_button));
        if show_confirm_pw_button.clicked(actions) || hide_confirm_pw_button.clicked(actions) {
            self.confirm_password_visible = !self.confirm_password_visible;
            confirm_password_input.toggle_is_password(cx);
            show_confirm_pw_button.set_visible(cx, !self.confirm_password_visible);
            hide_confirm_pw_button.set_visible(cx, self.confirm_password_visible);
            self.redraw(cx);
        }

        if mode_toggle_button.clicked(actions) {
            self.suppress_login_failure_modal = true;
            self.last_failure_message_shown = None;
            login_status_modal.close(cx);
            Cx::post_action(LoginAction::NavigateToRegister);
        }

        if login_button.clicked(actions)
            || user_id_input.returned(actions).is_some()
            || password_input.returned(actions).is_some()
            || homeserver_input.returned(actions).is_some()
        {
            let user_id = user_id_input.text().trim().to_owned();
            let password = password_input.text();
            let homeserver = homeserver_input.text().trim().to_owned();

            // Defensive backstop for cases where the homeserver field was
            // updated programmatically rather than through a Changed action.
            // Compare normalized URLs so `matrix.org` and
            // `https://matrix.org` count as the same probe target.
            let normalized_homeserver = homeserver
                .is_empty()
                .not()
                .then(|| normalize_homeserver_url(&homeserver).ok())
                .flatten();
            if self.last_discovery_input_url.as_deref() != normalized_homeserver.as_deref() {
                self.clear_homeserver_classification();
            }

            // Defensive guard: in MAS mode the login_button should be hidden
            // and Continue-in-browser is the active CTA. If a stale click
            // reaches here, drop it rather than submit password-auth to a
            // server that rejects it.
            if matches!(self.login_mode, Some(LoginMode::MasOidc)) {
                return;
            }

            // If the user typed a homeserver we haven't classified yet, run a
            // capability probe before deciding between password and OIDC paths.
            // An empty input means "use the default (matrix-client.matrix.org)" —
            // preserve the existing zero-latency password path there rather than
            // adding a probe round-trip.
            if !homeserver.is_empty()
                && !self.discovery_pending
                && should_probe_homeserver(self.login_mode, self.oidc_in_flight)
            {
                if let Ok(normalized) = normalize_homeserver_url(&homeserver) {
                    self.discovery_pending = true;
                    self.last_discovery_input_url = Some(normalized.clone());
                    self.view.button(cx, ids!(login_button)).set_text(
                        cx,
                        tr_key(self.app_language, "login.status.checking_homeserver.title"),
                    );
                    login_status_modal_inner.set_title(
                        cx,
                        tr_key(self.app_language, "login.status.checking_homeserver.title"),
                    );
                    login_status_modal_inner.set_status(
                        cx,
                        tr_key(self.app_language, "login.status.checking_homeserver.body"),
                    );
                    login_status_modal_inner.button_ref(cx)
                        .set_text(cx, tr_key(self.app_language, "login.status.cancel"));
                    login_status_modal.open(cx);
                    let proxy = match self.build_proxy_url_from_form(cx) {
                        Ok(proxy) => proxy,
                        Err(proxy_validation_error) => {
                            login_status_modal_inner.set_title(
                                cx,
                                tr_key(self.app_language, "login.status.invalid_proxy.title"),
                            );
                            let error_text = tr_fmt(self.app_language, "login.status.invalid_proxy.body", &[
                                ("error", proxy_validation_error.as_str()),
                            ]);
                            login_status_modal_inner.set_status(cx, &error_text);
                            login_status_modal_inner.button_ref(cx)
                                .set_text(cx, tr_key(self.app_language, "login.status.okay"));
                            login_status_modal.open(cx);
                            self.redraw(cx);
                            return;
                        }
                    };
                    if let Err(e) = crate::proxy_config::save_proxy_url(proxy.as_deref()) {
                        warning!("Failed to persist proxy configuration from homeserver probe: {e}");
                    }
                    submit_async_request(MatrixRequest::DiscoverHomeserverCapabilities {
                        url: normalized,
                        proxy,
                    });
                    self.redraw(cx);
                    return;
                }
                // normalize failed: fall through so the existing password path
                // surfaces a usable error to the user.
            }

            if user_id.is_empty() {
                login_status_modal_inner.set_title(cx, tr_key(self.app_language, "login.status.missing_user_id.title"));
                login_status_modal_inner.set_status(cx, tr_key(self.app_language, "login.status.missing_user_id.body"));
                login_status_modal_inner.button_ref(cx).set_text(cx, tr_key(self.app_language, "login.status.okay"));
            } else if password.is_empty() {
                login_status_modal_inner.set_title(cx, tr_key(self.app_language, "login.status.missing_password.title"));
                login_status_modal_inner.set_status(cx, tr_key(self.app_language, "login.status.missing_password.body"));
                login_status_modal_inner.button_ref(cx).set_text(cx, tr_key(self.app_language, "login.status.okay"));
            } else {
                let proxy = match self.build_proxy_url_from_form(cx) {
                    Ok(proxy) => proxy,
                    Err(proxy_validation_error) => {
                        login_status_modal_inner.set_title(cx, tr_key(self.app_language, "login.status.invalid_proxy.title"));
                        let error_text = tr_fmt(self.app_language, "login.status.invalid_proxy.body", &[
                            ("error", proxy_validation_error.as_str()),
                        ]);
                        login_status_modal_inner.set_status(cx, &error_text);
                        login_status_modal_inner.button_ref(cx).set_text(cx, tr_key(self.app_language, "login.status.okay"));
                        login_status_modal.open(cx);
                        self.redraw(cx);
                        return;
                    }
                };
                if let Err(e) = crate::proxy_config::save_proxy_url(proxy.as_deref()) {
                    warning!("Failed to persist proxy configuration from login screen: {e}");
                }
                self.last_failure_message_shown = None;
                login_status_modal_inner.set_title(cx, tr_key(self.app_language, "login.status.logging_in.title"));
                login_status_modal_inner.set_status(
                    cx,
                    tr_key(self.app_language, "login.status.logging_in.body"),
                );
                login_status_modal_inner.button_ref(cx).set_text(cx, tr_key(self.app_language, "login.status.cancel"));
                submit_async_request(MatrixRequest::Login(LoginRequest::LoginByPassword(LoginByPassword {
                    user_id,
                    password,
                    homeserver: homeserver.is_empty().not().then_some(homeserver),
                    proxy: proxy.clone(),
                    is_add_account: self.adding_account,
                })));
            }
            login_status_modal.open(cx);
            self.redraw(cx);
        }

        // "Continue in browser" click — only reachable when login_mode resolved
        // to MasOidc (otherwise the card is hidden). Kick off the worker's
        // OAuth flow via StartOidcLogin; oidc_in_flight will flip on when the
        // worker posts LoginAction::OidcLoginStarted.
        if self.view.button(cx, ids!(oidc_continue_button)).clicked(actions)
            && !self.oidc_in_flight
        {
            if !matches!(self.login_mode, Some(LoginMode::MasOidc)) {
                return;
            }
            let homeserver = homeserver_input.text().trim().to_owned();
            match self.build_proxy_url_from_form(cx) {
                Ok(proxy) => {
                    if let Err(e) = crate::proxy_config::save_proxy_url(proxy.as_deref()) {
                        warning!("Failed to persist proxy configuration from login screen: {e}");
                    }
                    submit_async_request(MatrixRequest::StartOidcLogin {
                        homeserver_url: homeserver,
                        proxy,
                        is_add_account: self.adding_account,
                    });
                    self.redraw(cx);
                }
                Err(proxy_validation_error) => {
                    login_status_modal_inner.set_title(
                        cx,
                        tr_key(self.app_language, "login.status.invalid_proxy.title"),
                    );
                    let error_text = tr_fmt(self.app_language, "login.status.invalid_proxy.body", &[
                        ("error", proxy_validation_error.as_str()),
                    ]);
                    login_status_modal_inner.set_status(cx, &error_text);
                    login_status_modal_inner.button_ref(cx)
                        .set_text(cx, tr_key(self.app_language, "login.status.okay"));
                    login_status_modal.open(cx);
                    self.redraw(cx);
                }
            }
        }

        // Cancel an in-flight OIDC flow. Worker drops the cancel branch of
        // its tokio::select!, calls abort_login, and posts OidcLoginCancelled
        // which we handle below to restore the oidc_card to ready state.
        if self.view.button(cx, ids!(oidc_cancel_button)).clicked(actions) {
            submit_async_request(MatrixRequest::CancelOidcLogin);
        }

        let provider_brands = ["apple", "facebook", "github", "gitlab", "google", "twitter"];
        let button_set: &[&[LiveId]] = ids_array!(
            apple_button, 
            facebook_button, 
            github_button, 
            gitlab_button, 
            google_button, 
            twitter_button
        );
        for action in actions {
            if let LoginStatusModalAction::Close = action.as_widget_action().cast() {
                login_status_modal.close(cx);
            }

            if let Some(RegisterAction::NavigateToLogin) = action.downcast_ref() {
                self.suppress_login_failure_modal = false;
                self.last_failure_message_shown = None;
                self.reset_oidc_screen_state(cx);
                login_status_modal.close(cx);
            }

            // Capability probe result for the homeserver input. We share this
            // action with RegisterScreen via crate::homeserver; filter on
            // requested_url so a probe fired from Register doesn't drive us
            // and vice versa.
            match action.downcast_ref::<CapabilityProbeAction>() {
                Some(CapabilityProbeAction::Discovered { requested_url, caps }) => {
                    if self.last_discovery_input_url.as_deref() != Some(requested_url.as_str()) {
                        continue;
                    }
                    self.discovery_pending = false;
                    self.view.button(cx, ids!(login_button))
                        .set_text(cx, tr_key(self.app_language, "login.button.sign_in_securely"));
                    let resolved = login_mode(caps.as_ref());
                    self.login_mode = Some(resolved);
                    match resolved {
                        LoginMode::MasOidc => {
                            self.show_oidc_login_branch(cx);
                            login_status_modal.close(cx);
                        }
                        LoginMode::Password => {
                            self.show_password_login_branch(cx);
                            login_status_modal.close(cx);
                        }
                    }
                    self.redraw(cx);
                    continue;
                }
                Some(CapabilityProbeAction::Failed { requested_url, error }) => {
                    if self.last_discovery_input_url.as_deref() != Some(requested_url.as_str()) {
                        continue;
                    }
                    self.clear_homeserver_classification();
                    self.show_password_login_branch(cx);
                    self.view.button(cx, ids!(login_button))
                        .set_text(cx, tr_key(self.app_language, "login.button.sign_in_securely"));
                    login_status_modal_inner.set_title(
                        cx,
                        tr_key(self.app_language, "login.status.login_failed"),
                    );
                    login_status_modal_inner.set_status(cx, error);
                    login_status_modal_inner.button_ref(cx)
                        .set_text(cx, tr_key(self.app_language, "login.status.okay"));
                    login_status_modal.open(cx);
                    self.redraw(cx);
                    continue;
                }
                Some(CapabilityProbeAction::None) | None => {}
            }

            // Handle login-related actions received from background async tasks.
            match action.downcast_ref() {
                Some(LoginAction::ShowLoginScreen) => {
                    self.suppress_login_failure_modal = false;
                    self.last_failure_message_shown = None;
                    self.reset_oidc_screen_state(cx);
                    login_status_modal.close(cx);
                    self.redraw(cx);
                }
                Some(LoginAction::CliAutoLogin { user_id, homeserver }) => {
                    self.last_failure_message_shown = None;
                    user_id_input.set_text(cx, user_id);
                    password_input.set_text(cx, "");
                    homeserver_input.set_text(cx, homeserver.as_deref().unwrap_or_default());
                    login_status_modal_inner.set_title(cx, tr_key(self.app_language, "login.status.logging_in_cli.title"));
                    login_status_modal_inner.set_status(
                        cx,
                        &tr_fmt(self.app_language, "login.status.auto_logging_in_as_user", &[
                            ("user_id", user_id.as_str()),
                        ])
                    );
                    let login_status_modal_button = login_status_modal_inner.button_ref(cx);
                    login_status_modal_button.set_text(cx, tr_key(self.app_language, "login.status.cancel"));
                    login_status_modal_button.set_enabled(cx, false); // Login cancel not yet supported
                    login_status_modal.open(cx);
                }
                Some(LoginAction::Status { title, status }) => {
                    self.last_failure_message_shown = None;
                    login_status_modal_inner.set_title(cx, title);
                    login_status_modal_inner.set_status(cx, status);
                    let login_status_modal_button = login_status_modal_inner.button_ref(cx);
                    login_status_modal_button.set_text(cx, tr_key(self.app_language, "login.status.cancel"));
                    login_status_modal_button.set_enabled(cx, true);
                    login_status_modal.open(cx);
                    self.redraw(cx);
                }
                Some(LoginAction::LoginSuccess) => {
                    // The main `App` component handles showing the main screen
                    // and hiding the login screen & login status modal.
                    self.suppress_login_failure_modal = false;
                    self.last_failure_message_shown = None;
                    self.adding_account = false;
                    self.reset_oidc_screen_state(cx);
                    user_id_input.set_text(cx, "");
                    password_input.set_text(cx, "");
                    homeserver_input.set_text(cx, "");
                    // Reset title and buttons in case we were in add-account mode
                    self.view.label(cx, ids!(title)).set_text(cx, tr_key(self.app_language, "login.subtitle.tagline"));
                    cancel_button.set_visible(cx, false);
                    mode_toggle_button.set_visible(cx, true);
                    login_status_modal.close(cx);
                    self.redraw(cx);
                }
                Some(LoginAction::ClearFailureState) => {
                    self.last_failure_message_shown = None;
                    login_status_modal.close(cx);
                    self.redraw(cx);
                }
                Some(LoginAction::LoginFailure(error)) => {
                    if !should_show_login_failure_modal(
                        self.suppress_login_failure_modal,
                        self.last_failure_message_shown.as_deref(),
                        error,
                    ) {
                        continue;
                    }
                    self.last_failure_message_shown = Some(error.clone());
                    login_status_modal_inner.set_title(cx, tr_key(self.app_language, "login.status.login_failed"));
                    login_status_modal_inner.set_status(cx, error);
                    let login_status_modal_button = login_status_modal_inner.button_ref(cx);
                    login_status_modal_button.set_text(cx, tr_key(self.app_language, "login.status.okay"));
                    login_status_modal_button.set_enabled(cx, true);
                    login_status_modal.open(cx);
                    self.redraw(cx);
                }
                Some(LoginAction::SsoPending(pending)) => {
                    self.set_sso_pending_state(cx, *pending);
                    self.redraw(cx);
                }
                Some(LoginAction::SsoSetRedirectUrl(url)) => {
                    self.sso_redirect_url = Some(url.to_string());
                }
                Some(LoginAction::ShowAddAccountScreen) => {
                    self.suppress_login_failure_modal = false;
                    self.adding_account = true;
                    self.reset_sso_state(cx);
                    self.reset_oidc_screen_state(cx);
                    // Update UI to "add account" mode
                    self.view.label(cx, ids!(title)).set_text(cx, tr_key(self.app_language, "settings.account.button.add_another_account"));
                    cancel_button.set_visible(cx, true);
                    // Hide signup button in add-account mode (user already has an account)
                    mode_toggle_button.set_visible(cx, false);
                    self.redraw(cx);
                }
                Some(LoginAction::AddAccountSuccess) => {
                    // Reset the login screen state
                    self.suppress_login_failure_modal = false;
                    self.adding_account = false;
                    self.reset_sso_state(cx);
                    self.reset_oidc_screen_state(cx);
                    user_id_input.set_text(cx, "");
                    password_input.set_text(cx, "");
                    homeserver_input.set_text(cx, "");
                    // Reset title and buttons
                    self.view.label(cx, ids!(title)).set_text(cx, tr_key(self.app_language, "login.subtitle.tagline"));
                    cancel_button.set_visible(cx, false);
                    mode_toggle_button.set_visible(cx, true);
                    login_status_modal.close(cx);
                    self.redraw(cx);
                }
                Some(LoginAction::OidcLoginStarted) => {
                    // Worker has launched the browser; flip the oidc_card to
                    // its waiting state and expose Cancel.
                    self.oidc_in_flight = true;
                    self.show_oidc_login_branch(cx);
                    self.view.button(cx, ids!(oidc_continue_button)).set_visible(cx, false);
                    let status = self.view.label(cx, ids!(oidc_status_label));
                    status.set_text(cx, tr_key(self.app_language, "login.oidc.waiting_body"));
                    status.set_visible(cx, true);
                    let cancel = self.view.button(cx, ids!(oidc_cancel_button));
                    cancel.set_text(cx, tr_key(self.app_language, "login.button.cancel_oidc"));
                    cancel.set_visible(cx, true);
                    login_status_modal.close(cx);
                    self.redraw(cx);
                }
                Some(LoginAction::OidcLoginCancelled) => {
                    // Restore the idle MAS branch so the user can retry. Use
                    // login.oidc.cancelled as a soft hint in the info body so
                    // they know why they're back here without a modal popup.
                    self.oidc_in_flight = false;
                    self.show_oidc_login_branch(cx);
                    self.view.label(cx, ids!(oidc_info_body))
                        .set_text(cx, tr_key(self.app_language, "login.oidc.cancelled"));
                    self.redraw(cx);
                }
                Some(LoginAction::OidcLoginFailed(error)) => {
                    // Same idle reset as cancel, but surface the error via
                    // the login_status_modal so it's unmissable.
                    self.oidc_in_flight = false;
                    self.show_oidc_login_branch(cx);
                    login_status_modal_inner.set_title(
                        cx,
                        tr_key(self.app_language, "login.status.login_failed"),
                    );
                    login_status_modal_inner.set_status(cx, error);
                    login_status_modal_inner.button_ref(cx)
                        .set_text(cx, tr_key(self.app_language, "login.status.okay"));
                    login_status_modal.open(cx);
                    self.redraw(cx);
                }
                _ => { }
            }

            // Handle account switch actions - close modal when switch completes or fails
            match action.downcast_ref() {
                Some(AccountSwitchAction::Switched(_)) => {
                    login_status_modal.close(cx);
                    self.redraw(cx);
                }
                Some(AccountSwitchAction::Failed(error)) => {
                    login_status_modal_inner.set_title(cx, tr_key(self.app_language, "login.status.account_switch_failed"));
                    login_status_modal_inner.set_status(cx, error);
                    let login_status_modal_button = login_status_modal_inner.button_ref(cx);
                    login_status_modal_button.set_text(cx, tr_key(self.app_language, "login.status.okay"));
                    login_status_modal_button.set_enabled(cx, true);
                    self.redraw(cx);
                }
                _ => { }
            }
        }

        // If the Login SSO screen's "cancel" button was clicked, send a http request to gracefully shutdown the SSO server
        if let Some(sso_redirect_url) = &self.sso_redirect_url {
            let login_status_modal_button = login_status_modal_inner.button_ref(cx);
            if login_status_modal_button.clicked(actions) {
                let request_id = id!(SSO_CANCEL_BUTTON);
                let request = HttpRequest::new(format!("{}/?login_token=",sso_redirect_url), HttpMethod::GET);
                cx.http_request(request_id, request);
                self.reset_sso_state(cx);
                self.redraw(cx);
            }
        }

        // Handle any of the SSO login buttons being clicked
        for (view_ref, brand) in self.view_set(cx, button_set).iter().zip(&provider_brands) {
            if view_ref.finger_up(actions).is_some() && !self.sso_pending {
                let proxy = match self.build_proxy_url_from_form(cx) {
                    Ok(proxy) => proxy,
                    Err(proxy_validation_error) => {
                        login_status_modal_inner.set_title(cx, tr_key(self.app_language, "login.status.invalid_proxy.title"));
                        let error_text = tr_fmt(self.app_language, "login.status.invalid_proxy.body", &[
                            ("error", proxy_validation_error.as_str()),
                        ]);
                        login_status_modal_inner.set_status(cx, &error_text);
                        let login_status_modal_button = login_status_modal_inner.button_ref(cx);
                        login_status_modal_button.set_text(cx, tr_key(self.app_language, "login.status.okay"));
                        login_status_modal_button.set_enabled(cx, true);
                        login_status_modal.open(cx);
                        self.redraw(cx);
                        continue;
                    }
                };
                if let Err(e) = crate::proxy_config::save_proxy_url(proxy.as_deref()) {
                    warning!("Failed to persist proxy configuration from SSO login flow: {e}");
                }
                submit_async_request(MatrixRequest::SpawnSSOServer{
                    identity_provider_id: format!("oidc-{}",brand),
                    brand: brand.to_string(),
                    homeserver_url: homeserver_input.text(),
                    proxy,
                });
            }
        }
    }

}

/// Actions sent to or from the login screen.
#[derive(Clone, Default, Debug)]
pub enum LoginAction {
    /// Request to show the login screen because no reusable session is available.
    ShowLoginScreen,
    /// A positive response from the backend Matrix task to the login screen.
    LoginSuccess,
    /// A positive response when adding an additional account (multi-account mode).
    /// The login was successful but we should add this as a new account, not replace the existing one.
    AddAccountSuccess,
    /// A negative response from the backend Matrix task to the login screen.
    LoginFailure(String),
    /// Clear any hidden login failure UI/state that should not leak across flows.
    ClearFailureState,
    /// A login-related status message to display to the user.
    Status {
        title: String,
        status: String,
    },
    /// The given login info was specified on the command line (CLI),
    /// and the login process is underway.
    CliAutoLogin {
        user_id: String,
        homeserver: Option<String>,
    },
    /// An acknowledgment that is sent from the backend Matrix task to the login screen
    /// informing it that the SSO login process is either still in flight (`true`) or has finished (`false`).
    ///
    /// Note that an inner value of `false` does *not* imply that the login request has
    /// successfully finished.
    /// The login screen can use this to prevent the user from submitting
    /// additional SSO login requests while a previous request is in flight.
    SsoPending(bool),
    /// Set the SSO redirect URL in the LoginScreen.
    ///
    /// When an SSO-based login is pendng, pressing the cancel button will send
    /// an HTTP request to this SSO server URL to gracefully shut it down.
    SsoSetRedirectUrl(Url),
    /// Request to show the login screen in "add account" mode.
    /// This is used when the user wants to add another Matrix account.
    ShowAddAccountScreen,
    /// User clicked "Sign up here"; the main App should hide the
    /// login screen and show RegisterScreen.
    NavigateToRegister,
    /// Request to cancel adding an account and return to the previous screen.
    CancelAddAccount,
    /// Posted by the OIDC worker once the browser-based auth flow has been
    /// launched and robrix2 is waiting for the loopback callback.
    /// LoginScreen uses this to swap the "Continue in browser" button for the
    /// "Waiting for callback..." + Cancel affordance.
    OidcLoginStarted,
    /// Posted when the OIDC flow was aborted — either via in-app Cancel, via
    /// the browser's `error=access_denied` redirect, or via the 3-minute
    /// total timeout. LoginScreen returns to the MAS branch ready-for-retry.
    OidcLoginCancelled,
    /// Posted when OIDC failed at any post-click stage (metadata discovery,
    /// dynamic registration, authorize build, browser open, token exchange).
    /// Payload is user-displayable; technical details go to logs.
    OidcLoginFailed(String),
    #[default]
    None,
}

#[cfg(test)]
mod tests {
    use super::{is_mobile_login_layout, should_probe_homeserver, should_show_login_failure_modal};
    use crate::homeserver::LoginMode;

    #[test]
    fn login_failure_modal_is_suppressed_while_register_flow_is_active() {
        assert!(!should_show_login_failure_modal(true, None, "boom"));
    }

    #[test]
    fn duplicate_login_failure_message_is_suppressed() {
        assert!(!should_show_login_failure_modal(false, Some("boom"), "boom"));
    }

    #[test]
    fn fresh_login_failure_message_is_shown_when_not_suppressed() {
        assert!(should_show_login_failure_modal(false, Some("old"), "boom"));
    }

    #[test]
    fn capability_probe_is_required_when_login_mode_is_unknown() {
        assert!(should_probe_homeserver(None, false));
    }

    #[test]
    fn capability_probe_is_not_required_when_mode_already_classified() {
        assert!(!should_probe_homeserver(Some(LoginMode::Password), false));
        assert!(!should_probe_homeserver(Some(LoginMode::MasOidc), false));
    }

    #[test]
    fn capability_probe_is_not_required_while_oidc_login_is_in_flight() {
        assert!(!should_probe_homeserver(None, true));
    }

    /// User-visible copy keys added by the login redesign. Every key must
    /// resolve in both dictionaries (see the two tests below).
    const LOGIN_REDESIGN_KEYS: &[&str] = &[
        "login.subtitle.tagline",
        "login.button.sign_in_securely",
        "login.divider.or_continue_with",
        "login.account_prompt.new_to_robrix",
        "login.mode_toggle.create_account",
        "login.footer.secure_session",
        "login.footer.self_host_ready",
        "login.footer.matrix_connected",
        "login.badge.agent_ready",
    ];

    #[test]
    fn test_login_redesign_i18n_keys_exist_en() {
        use crate::i18n::{tr_key, AppLanguage};
        for key in LOGIN_REDESIGN_KEYS {
            let v = tr_key(AppLanguage::English, key);
            assert!(!v.is_empty(), "missing EN i18n for {key}");
            // tr_key falls back to the key name when the entry is absent.
            assert_ne!(v, *key, "EN i18n for {key} is missing (fell back to the key name)");
        }
    }

    #[test]
    fn test_login_redesign_i18n_keys_exist_zh() {
        use crate::i18n::{tr_key, AppLanguage};
        for key in LOGIN_REDESIGN_KEYS {
            let zh = tr_key(AppLanguage::ChineseSimplified, key);
            let en = tr_key(AppLanguage::English, key);
            assert!(!zh.is_empty(), "missing ZH i18n for {key}");
            assert_ne!(zh, *key, "ZH i18n for {key} is missing (fell back to the key name)");
            // tr_key falls back to EN when the zh-CN entry is missing, so a
            // distinct zh-CN string is the real proof the key was translated.
            assert_ne!(zh, en, "ZH i18n for {key} is missing (fell back to the English string)");
        }
    }

    #[test]
    fn test_login_screen_source_wires_accent_token() {
        let src = include_str!("login_screen.rs");
        assert!(src.contains("RBX_ACCENT"), "the primary CTA should use the teal RBX_ACCENT token");
        assert!(src.contains("login_button"), "the login_button widget id must be preserved");
        // Build the legacy-blue needle from parts so this assertion's own source
        // text does not contain the contiguous token and self-trip include_str!.
        let legacy_primary = concat!("COLOR_ACTIVE", "_PRIMARY");
        assert!(
            !src.contains(legacy_primary),
            "login screen must not reference the deprecated legacy primary-blue token",
        );
    }

    #[test]
    fn test_login_screen_source_uses_desktop_card_contract() {
        let src = include_str!("login_screen.rs");
        assert!(src.contains("max: 494"), "desktop login card should keep the wider reference-card max width");
        assert!(src.contains("border_radius: (RBX_RADIUS_LG)"), "login card should use RBX_RADIUS_LG");
        assert!(src.contains("login_button := RobrixIconButton"));
        assert!(src.contains("max: 422"), "primary form controls should align to the desktop content column max width");

        for forbidden in [
            concat!("#", "0000"),
            concat!("#", "x00000000"),
            concat!("#", "fff"),
        ] {
            assert!(
                !src.contains(forbidden),
                "login screen should not contain bare color literal {forbidden}",
            );
        }
    }

    #[test]
    fn test_login_inputs_do_not_inherit_legacy_blue_focus_border() {
        let src = include_str!("login_screen.rs");
        let input_style = src
            .split("mod.widgets.LoginTextInput = RobrixTextInput")
            .nth(1)
            .expect("LoginTextInput style should exist")
            .split("mod.widgets.LoginScreen")
            .next()
            .expect("LoginTextInput style should precede LoginScreen");
        assert!(input_style.contains("border_color_hover: (RBX_STROKE_STRONG)"));
        assert!(input_style.contains("border_color_focus: (RBX_ACCENT)"));
        assert!(input_style.contains("border_color_down: (RBX_STROKE_STRONG)"));
        assert!(input_style.contains("clip_y: true"));
        assert!(input_style.contains("draw_cursor +:"));
        assert!(src.contains("homeserver_input := mod.widgets.LoginTextInput"));
        let legacy_homeserver_input = concat!("homeserver_input := ", "RobrixTextInput");
        assert!(
            !src.contains(legacy_homeserver_input),
            "homeserver input should use the login-specific input style, not the legacy-blue global input",
        );
    }

    #[test]
    fn test_login_form_uses_non_overlay_layout_around_inputs() {
        let src = include_str!("login_screen.rs");
        let card_needle = concat!("login_card", " := RoundedView");
        let column_needle = concat!("form_column", " := View");
        let hint_row_needle = concat!("homeserver_hint_row", " := View");
        let old_hint_needle = concat!(
            "LineH { draw_bg.color: (RBX_STROKE_SOFT) }",
            "\n\n                            ",
            "homeserver_hint_label",
        );
        assert!(src.contains(card_needle));
        assert!(src.contains(column_needle));
        assert!(src.contains(hint_row_needle));
        assert!(
            !src.contains(old_hint_needle),
            "homeserver hint should not be sandwiched between divider lines directly under the input",
        );
    }

    #[test]
    fn test_mobile_login_layout_target_detection() {
        assert!(is_mobile_login_layout(700.0, false));
        assert!(is_mobile_login_layout(390.0, false));
        assert!(!is_mobile_login_layout(701.0, false));
        assert!(is_mobile_login_layout(1272.0, true));
    }

    #[test]
    fn test_mobile_login_reapplies_layout_for_late_bound_children() {
        let src = include_str!("login_screen.rs");
        let same_mode_branch = concat!(
            "if self.mobile_layout_active == Some(mobile) {\n",
            "            self.apply_login_layout(cx, mobile);"
        );
        assert!(
            src.contains(same_mode_branch),
            "mobile layout must reapply on repeated geometry events so nested controls receive runtime style",
        );
    }

    #[test]
    fn test_login_screen_source_contains_mobile_light_contract() {
        let src = include_str!("login_screen.rs");
        assert!(src.contains("RBX_BG_CANVAS"), "mobile and desktop login should use the light canvas token until global theme switching exists");
        assert!(src.contains("RBX_BG_SURFACE"), "mobile and desktop login should use the light surface token until global theme switching exists");
        assert!(
            !src.contains(concat!("RBX_", "LOGIN_BG")),
            "mobile login should not introduce a standalone dark background before global theme switching",
        );
        assert!(
            !src.contains(concat!("RBX_", "LOGIN_SURFACE")),
            "mobile login should not introduce a standalone dark surface before global theme switching",
        );

        for forbidden in [
            concat!("#", "0000"),
            concat!("#", "x00000000"),
            concat!("#", "fff"),
        ] {
            assert!(
                !src.contains(forbidden),
                "login screen should not contain bare color literal {forbidden}",
            );
        }
    }

    #[test]
    fn test_mobile_login_form_labels_and_placeholders_are_i18n_bound() {
        use crate::i18n::{tr_key, AppLanguage};

        let src = include_str!("login_screen.rs");
        for needle in [
            "user_id_field_label",
            "password_field_label",
            "homeserver_field_label",
            "login.input.mobile.user_id",
            "login.input.mobile.password",
            "login.input.mobile.homeserver",
        ] {
            assert!(src.contains(needle), "missing mobile login source contract: {needle}");
        }

        for key in [
            "login.input.mobile.user_id",
            "login.input.mobile.password",
            "login.input.mobile.homeserver",
        ] {
            let en = tr_key(AppLanguage::English, key);
            let zh = tr_key(AppLanguage::ChineseSimplified, key);
            assert_ne!(en, key, "missing EN i18n for {key}");
            assert_ne!(zh, key, "missing ZH i18n for {key}");
            if key != "login.input.mobile.homeserver" {
                assert_ne!(zh, en, "ZH i18n for {key} fell back to English");
            }
        }
    }

    #[test]
    fn test_mobile_sso_grid_preserves_provider_ids() {
        let src = include_str!("login_screen.rs");
        assert!(
            src.contains(concat!("width: Fill{max: ", "170}")),
            "mobile SSO grid should use a constrained width so six icon buttons align as 3x2",
        );
        assert!(
            src.contains("let sso_width = if mobile { 170.0 } else { 322.0 };"),
            "runtime layout should only switch the direct SSO container width",
        );
        for id in [
            "apple_button",
            "facebook_button",
            "github_button",
            "gitlab_button",
            "google_button",
            "twitter_button",
        ] {
            assert!(src.contains(id), "missing existing SSO provider id {id}");
        }
        assert!(!src.contains(concat!("microsoft", "_button")));
        assert!(!src.contains(concat!("more", "_button")));
    }

    #[test]
    fn test_mobile_login_footer_contract() {
        let src = include_str!("login_screen.rs");
        assert!(src.contains("mobile_status_footer"));
        assert!(src.contains("mobile_version_label"));
        assert!(src.contains("desktop_status_footer"));
    }
}
