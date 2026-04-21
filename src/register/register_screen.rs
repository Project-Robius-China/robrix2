//! RegisterScreen widget: homeserver picker + capability display.
//!
//! Phase 1 renders:
//!   - Back button (returns to login)
//!   - Screen title
//!   - Homeserver URL input
//!   - Next button (triggers capability discovery)
//!   - Three-state status area (MAS / UIAA / Disabled / errors)
//!
//! Phases 2-5 fill in OIDC launch / UIAA form / SSO buttons.

use makepad_widgets::*;

use crate::register::{HsCapabilities, RegisterAction, RegisterMode};
use crate::register::validation::{normalize_homeserver_url, HomeserverUrlError};
use crate::sliding_sync::{submit_async_request, MatrixRequest};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.RegisterScreen = #(RegisterScreen::register_widget(vm)) {
        width: Fill,
        height: Fill,
        flow: Down,
        padding: Inset { top: 24, right: 32, bottom: 24, left: 32 }
        spacing: 16
        show_bg: true
        draw_bg +: {
            color: (COLOR_SECONDARY)
        }

        back_button := RobrixIconButton {
            width: Fit,
            height: Fit,
            text: "← Back to Login"
        }

        title := Label {
            width: Fit,
            height: Fit,
            text: "Create Account"
            draw_text +: {
                color: (COLOR_TEXT)
                text_style: TITLE_TEXT {font_size: 16.0}
            }
        }

        homeserver_row := View {
            width: Fill,
            height: Fit,
            flow: Down,
            spacing: 4

            Label {
                text: "Homeserver URL"
                draw_text +: {
                    color: (COLOR_TEXT)
                    text_style: REGULAR_TEXT {font_size: 10.0}
                }
            }

            homeserver_input := RobrixTextInput {
                width: Fill,
                height: 40,
                empty_text: "matrix.org"
            }
        }

        next_button := RobrixIconButton {
            width: Fit,
            height: Fit,
            text: "Next"
        }

        status_area := View {
            width: Fill,
            height: Fit,
            flow: Down,
            spacing: 8,
            visible: false

            status_label := Label {
                width: Fill,
                height: Fit,
                text: ""
                draw_text +: {
                    color: (COLOR_TEXT)
                    text_style: REGULAR_TEXT {font_size: 12.0}
                }
            }
        }
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct RegisterScreen {
    #[deref] view: View,
    #[rust] last_discovery: Option<HsCapabilities>,
}

impl Widget for RegisterScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl WidgetMatchEvent for RegisterScreen {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        let back = self.view.button(cx, ids!(back_button));
        let next = self.view.button(cx, ids!(next_button));

        if back.clicked(actions) {
            Cx::post_action(RegisterAction::NavigateToLogin);
            return;
        }

        if next.clicked(actions) {
            let raw = self.view.text_input(cx, ids!(homeserver_input)).text();
            match normalize_homeserver_url(&raw) {
                Ok(url) => {
                    self.show_status(cx, "Checking server capabilities...");
                    submit_async_request(MatrixRequest::DiscoverHomeserverCapabilities { url });
                }
                Err(HomeserverUrlError::Empty) => {
                    self.show_status(cx, "Please enter a homeserver URL (e.g. matrix.org).");
                }
                Err(HomeserverUrlError::UnsupportedScheme(s)) => {
                    self.show_status(cx, &format!("Unsupported scheme: {s}. Only http(s) is allowed."));
                }
                Err(HomeserverUrlError::Invalid) => {
                    self.show_status(cx, "That URL looks invalid. Please check and try again.");
                }
            }
        }

        // Capability discovery results.
        for action in actions {
            match action.downcast_ref::<RegisterAction>() {
                Some(RegisterAction::CapabilitiesDiscovered(caps)) => {
                    let msg = match caps.mode() {
                        RegisterMode::MasWebOnly => "This server uses browser-based registration (MAS OAuth). Phase 2 will handle this.",
                        RegisterMode::Uiaa => "This server allows direct account creation. Phase 3 will handle the form.",
                        RegisterMode::Disabled => "This server does not allow registration. Please choose a different homeserver or sign in with an existing account.",
                    };
                    self.show_status(cx, msg);
                    self.last_discovery = Some(caps.clone());
                }
                Some(RegisterAction::DiscoveryFailed(err)) => {
                    self.show_status(cx, &format!("Could not reach that server: {err}"));
                    self.last_discovery = None;
                }
                _ => {}
            }
        }
    }
}

impl RegisterScreen {
    fn show_status(&mut self, cx: &mut Cx, message: &str) {
        self.view.view(cx, ids!(status_area)).set_visible(cx, true);
        self.view.label(cx, ids!(status_label)).set_text(cx, message);
        self.view.redraw(cx);
    }
}
