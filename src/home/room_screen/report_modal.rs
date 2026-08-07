//! The report-room modal dialog.

use super::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.ReportRoomModalLabel = Label {
        width: Fill
        height: Fit
        draw_text +: {
            text_style: REGULAR_TEXT { font_size: 10.5 }
            color: #333
        }
        text: ""
    }

    mod.widgets.ReportRoomModal = #(ReportRoomModal::register_widget(vm)) {
        width: Fill { max: 430 }
        height: Fit
        margin: Inset{left: 12, right: 12}

        RoundedShadowView {
            width: Fill
            height: Fit
            align: Align{x: 0.5}
            flow: Down
            padding: Inset{top: 26, right: 22, bottom: 18, left: 22}
            spacing: 14

            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_SM)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
                shadow_color: (RBX_SHADOW_STRONG)
                shadow_radius: 10.0
                shadow_offset: vec2(0.0, 3.0)
            }

            title := Label {
                width: Fill
                height: Fit
                draw_text +: {
                    text_style: TITLE_TEXT { font_size: 13 }
                    color: (RBX_FG_PRIMARY)
                }
                text: "Report Room"
            }

            body := mod.widgets.ReportRoomModalLabel {
                text: ""
            }

            reason_input := RobrixTextInput {
                width: Fill
                height: Fit
                padding: 10
                draw_text +: {
                    text_style: REGULAR_TEXT { font_size: 11.5 }
                    color: #000
                }
                empty_text: "Describe why you are reporting this room"
            }

            status_label := Label {
                width: Fill
                height: Fit
                draw_text +: {
                    text_style: REGULAR_TEXT { font_size: 10.2 }
                    color: #000
                }
                text: ""
            }

            buttons := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{x: 1.0, y: 0.5}
                spacing: 16

                cancel_button := RobrixNeutralIconButton {
                    width: 110
                    align: Align{x: 0.5, y: 0.5}
                    padding: 12
                    draw_icon.svg: (ICON_FORBIDDEN)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1}}
                    text: "Cancel"
                }

                report_button := RobrixNegativeIconButton {
                    width: 130
                    align: Align{x: 0.5, y: 0.5}
                    padding: 12
                    draw_icon.svg: (ICON_INFO)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1}}
                    text: "Report room"
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub enum ReportRoomModalAction {
    /// Emitted by RoomScreen to open the (now global, app-root) report modal
    /// for a specific room. Carries the room so app.rs can route the result.
    Open {
        room_id: OwnedRoomId,
        room_name_id: RoomNameId,
    },
    Close,
    Submit(String),
}

#[derive(Script, ScriptHook, Widget)]
pub struct ReportRoomModal {
    #[deref]
    view: View,
    #[rust]
    is_showing_error: bool,
}

impl Widget for ReportRoomModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl WidgetMatchEvent for ReportRoomModal {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        let cancel_button = self.view.button(cx, ids!(buttons.cancel_button));
        let report_button = self.view.button(cx, ids!(buttons.report_button));
        let reason_input = self.view.text_input(cx, ids!(reason_input));
        let mut status_label = self.view.label(cx, ids!(status_label));

        if cancel_button.clicked(actions)
            || actions
                .iter()
                .any(|a| matches!(a.downcast_ref(), Some(ModalAction::Dismissed)))
        {
            cx.action(ReportRoomModalAction::Close);
            return;
        }

        if self.is_showing_error && reason_input.changed(actions).is_some() {
            self.is_showing_error = false;
            status_label.set_text(cx, "");
            self.view.redraw(cx);
        }

        if report_button.clicked(actions) || reason_input.returned(actions).is_some() {
            let reason = reason_input.text().trim().to_string();
            if reason.is_empty() {
                self.is_showing_error = true;
                script_apply_eval!(cx, status_label, {
                    text: "Please enter a reason before reporting."
                    draw_text +: {
                        color: mod.widgets.COLOR_FG_DANGER_RED
                    }
                });
                self.view.redraw(cx);
                return;
            }
            cx.action(ReportRoomModalAction::Submit(reason));
        }
    }
}

impl ReportRoomModal {
    pub fn show(&mut self, cx: &mut Cx, room_name_id: &RoomNameId) {
        self.is_showing_error = false;
        self.view
            .label(cx, ids!(title))
            .set_text(cx, "Report Room");
        self.view.label(cx, ids!(body)).set_text(
            cx,
            &format!(
                "Report {} to your homeserver administrators. Please provide a reason.",
                room_name_id
            ),
        );
        self.view
            .text_input(cx, ids!(reason_input))
            .set_text(cx, "");
        self.view.label(cx, ids!(status_label)).set_text(cx, "");
        self.view
            .button(cx, ids!(buttons.report_button))
            .set_enabled(cx, true);
        self.view
            .button(cx, ids!(buttons.cancel_button))
            .set_enabled(cx, true);
        self.view
            .button(cx, ids!(buttons.report_button))
            .reset_hover(cx);
        self.view
            .button(cx, ids!(buttons.cancel_button))
            .reset_hover(cx);
        self.view.redraw(cx);
    }
}

impl ReportRoomModalRef {
    pub fn show(&self, cx: &mut Cx, room_name_id: &RoomNameId) {
        let Some(mut inner) = self.borrow_mut() else {
            return;
        };
        inner.show(cx, room_name_id);
    }
}
