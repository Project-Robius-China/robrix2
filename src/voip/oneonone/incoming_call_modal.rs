//! Full-screen incoming-call modal for 1:1 voice calls.
//!
//! Mounted at the app root (see `src/app.rs`). Becomes visible when the
//! orchestrator emits [`OneOnOneUiAction::ShowIncomingModal`] and hides
//! itself on Accept / Decline / timeout. The Accept and Decline buttons
//! feed [`OneOnOneEvent::UserAccept`] / [`OneOnOneEvent::UserDecline`]
//! back into the orchestrator, which drives the rest of the flow.

use makepad_widgets::*;
use matrix_sdk::ruma::{OwnedRoomId, OwnedUserId};

use crate::voip::VoipGlobalState;
use crate::voip::oneonone::{OneOnOneEvent, OneOnOneUiAction};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.IncomingCallModal = #(IncomingCallModal::register_widget(vm)) {
        width: Fit
        height: Fit

        RoundedView {
            flow: Down
            width: 340
            height: Fit
            padding: Inset{top: 30, right: 30, bottom: 25, left: 30}
            spacing: 16
            align: Align{x: 0.5}

            show_bg: true
            draw_bg.color: (COLOR_PRIMARY)
            draw_bg.border_radius: 6.0

            ringing_label := Label {
                draw_text +: {
                    text_style: REGULAR_TEXT {font_size: 10}
                    color: #888
                }
                text: "Incoming voice call…"
            }

            caller_name := Label {
                draw_text +: {
                    text_style: TITLE_TEXT {font_size: 16}
                    color: #000
                }
                text: ""
            }

            caller_user_id := Label {
                draw_text +: {
                    text_style: REGULAR_TEXT {font_size: 11}
                    color: #666
                }
                text: ""
            }

            View {
                width: Fill, height: Fit
                flow: Right
                padding: Inset{top: 12}
                align: Align{x: 0.5, y: 0.5}
                spacing: 24

                decline_button := RobrixNegativeIconButton {
                    width: 130,
                    align: Align{x: 0.5, y: 0.5}
                    padding: 15,
                    draw_icon.svg: (ICON_FORBIDDEN)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: "Decline"
                }

                accept_button := RobrixPositiveIconButton {
                    width: 130,
                    align: Align{x: 0.5, y: 0.5}
                    padding: 15,
                    draw_icon.svg: (ICON_PHONE)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: "Accept"
                }
            }
        }
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct IncomingCallModal {
    #[deref] view: View,
    /// True while we're showing an actual incoming call (vs the default
    /// hidden state). When false, button clicks are ignored — defensive
    /// guard against late actions reaching us after the orchestrator
    /// already moved past `Incoming`.
    #[rust] active: bool,
    /// Set when a ring is in flight so the orchestrator knows which
    /// call we're acting on. Not strictly necessary (the FSM has the
    /// same info) but useful for logging.
    #[rust] current_call_id: Option<String>,
    #[rust] current_room_id: Option<OwnedRoomId>,
    #[rust] current_caller: Option<OwnedUserId>,
}

impl Widget for IncomingCallModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl WidgetMatchEvent for IncomingCallModal {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        // React to orchestrator UI actions.
        for action in actions {
            if let Some(ui) = action.downcast_ref::<OneOnOneUiAction>() {
                match ui {
                    OneOnOneUiAction::ShowIncomingModal { call_id, room_id, caller } => {
                        self.active = true;
                        self.current_call_id = Some(call_id.clone());
                        self.current_room_id = Some(room_id.clone());
                        self.current_caller = Some(caller.clone());

                        // Caller display name: fall back to the bare
                        // mxid until profile cache resolves. The full
                        // avatar/displayname integration belongs to a
                        // later round; for v1 the mxid is enough to
                        // distinguish callers.
                        let caller_str = caller.as_str();
                        let display_name = caller.localpart().to_owned();
                        self.view.label(cx, ids!(caller_name))
                            .set_text(cx, &display_name);
                        self.view.label(cx, ids!(caller_user_id))
                            .set_text(cx, caller_str);
                        self.view.redraw(cx);
                    }
                    OneOnOneUiAction::HideIncomingModal => {
                        self.active = false;
                        self.current_call_id = None;
                        self.current_room_id = None;
                        self.current_caller = None;
                        self.view.redraw(cx);
                    }
                    _ => {}
                }
            }
        }

        if !self.active { return; }

        if self.view.button(cx, ids!(accept_button)).clicked(actions) {
            log!("IncomingCallModal: user accepted call");
            VoipGlobalState::apply_call_event(cx, OneOnOneEvent::UserAccept);
        }
        if self.view.button(cx, ids!(decline_button)).clicked(actions) {
            log!("IncomingCallModal: user declined call");
            VoipGlobalState::apply_call_event(cx, OneOnOneEvent::UserDecline);
        }
    }
}

impl IncomingCallModal {
    /// Whether the modal currently has an active call to show. Used by
    /// the app to gate the parent Modal's open/close calls.
    pub fn is_active(&self) -> bool { self.active }
}
