use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.AgentApprovalPrimaryButton = Button {
        width: Fit
        height: (RBX_CONTROL_H_MD)
        padding: Inset{left: 12.0, right: 12.0, top: 7.0, bottom: 7.0}
        draw_bg +: {
            color: (RBX_SUCCESS_BG)
            color_hover: (RBX_SUCCESS_BG)
            color_down: (RBX_BG_PRESSED)
            color_disabled: (RBX_BG_DISABLED)
            border_radius: (RBX_RADIUS_MD)
            border_size: 1.0
            border_color: (RBX_SUCCESS_FG)
            border_color_hover: (RBX_SUCCESS_FG)
            border_color_down: (RBX_SUCCESS_FG)
        }
        draw_text +: {
            text_style: (RBX_TEXT_BODY_STRONG)
            color: (RBX_SUCCESS_FG)
            color_hover: (RBX_SUCCESS_FG)
            color_down: (RBX_SUCCESS_FG)
            color_disabled: (RBX_FG_DISABLED)
        }
        text: ""
    }

    mod.widgets.AgentApprovalDangerButton = Button {
        width: Fit
        height: (RBX_CONTROL_H_MD)
        padding: Inset{left: 12.0, right: 12.0, top: 7.0, bottom: 7.0}
        draw_bg +: {
            color: (RBX_DANGER_BG)
            color_hover: (RBX_DANGER_BG)
            color_down: (RBX_BG_PRESSED)
            color_disabled: (RBX_BG_DISABLED)
            border_radius: (RBX_RADIUS_MD)
            border_size: 1.0
            border_color: (RBX_DANGER_FG)
            border_color_hover: (RBX_DANGER_FG)
            border_color_down: (RBX_DANGER_FG)
        }
        draw_text +: {
            text_style: (RBX_TEXT_BODY_STRONG)
            color: (RBX_DANGER_FG)
            color_hover: (RBX_DANGER_FG)
            color_down: (RBX_DANGER_FG)
            color_disabled: (RBX_FG_DISABLED)
        }
        text: ""
    }

    mod.widgets.AgentApprovalSecondaryButton = Button {
        width: Fit
        height: (RBX_CONTROL_H_MD)
        padding: Inset{left: 12.0, right: 12.0, top: 7.0, bottom: 7.0}
        draw_bg +: {
            color: (RBX_BG_SURFACE)
            color_hover: (RBX_BG_HOVER)
            color_down: (RBX_BG_PRESSED)
            color_disabled: (RBX_BG_DISABLED)
            border_radius: (RBX_RADIUS_MD)
            border_size: 1.0
            border_color: (RBX_STROKE_STRONG)
            border_color_hover: (RBX_STROKE_STRONG)
            border_color_down: (RBX_STROKE_STRONG)
        }
        draw_text +: {
            text_style: (RBX_TEXT_BODY_STRONG)
            color: (RBX_FG_SECONDARY)
            color_hover: (RBX_FG_PRIMARY)
            color_down: (RBX_FG_PRIMARY)
            color_disabled: (RBX_FG_DISABLED)
        }
        text: ""
    }

    mod.widgets.AgentApprovalButtonSlot = View {
        visible: false
        width: Fit
        height: Fit
        flow: Overlay

        primary_button := mod.widgets.AgentApprovalPrimaryButton { visible: false }
        secondary_button := mod.widgets.AgentApprovalSecondaryButton { visible: false }
        danger_button := mod.widgets.AgentApprovalDangerButton { visible: false }
    }

    mod.widgets.AgentApprovalCard = RoundedView {
        visible: false
        width: Fill
        height: Fit
        flow: Down
        spacing: 8.0
        padding: Inset{left: 16.0, right: 16.0, top: 12.0, bottom: 16.0}
        show_bg: true
        draw_bg +: {
            color: (RBX_WARNING_BG)
            border_radius: (RBX_RADIUS_SM)
            border_size: 1.0
            border_color: (RBX_WARNING_FG)
        }

        approval_header := View {
            width: Fill
            height: Fit
            flow: Right
            spacing: 8.0
            align: Align{y: 0.5}

            approval_title_label := Label {
                width: Fill
                height: Fit
                flow: Flow.Right{wrap: true}
                draw_text +: {
                    text_style: (RBX_TEXT_CARD_TITLE)
                    color: (RBX_WARNING_FG)
                }
                text: ""
            }

            pending_badge := RoundedView {
                width: Fit
                height: Fit
                padding: Inset{left: 8.0, right: 8.0, top: 4.0, bottom: 4.0}
                show_bg: true
                draw_bg +: {
                    color: (RBX_BG_SURFACE)
                    border_radius: (RBX_RADIUS_PILL)
                }
                pending_label := Label {
                    width: Fit
                    height: Fit
                    draw_text +: {
                        text_style: (RBX_TEXT_BADGE)
                        color: (RBX_WARNING_FG)
                    }
                    text: ""
                }
            }
        }

        approval_summary_label := Label {
            width: Fill
            height: Fit
            flow: Flow.Right{wrap: true}
            draw_text +: {
                text_style: (RBX_TEXT_BODY)
                color: (RBX_FG_PRIMARY)
            }
            text: ""
        }

        approval_action_button_row := View {
            visible: false
            width: Fill
            height: Fit
            flow: Flow.Right{wrap: true}
            spacing: 8.0

            approval_button_slot_0 := mod.widgets.AgentApprovalButtonSlot {}
            approval_button_slot_1 := mod.widgets.AgentApprovalButtonSlot {}
            approval_button_slot_2 := mod.widgets.AgentApprovalButtonSlot {}
            approval_button_slot_3 := mod.widgets.AgentApprovalButtonSlot {}
            approval_button_slot_4 := mod.widgets.AgentApprovalButtonSlot {}
            approval_button_slot_5 := mod.widgets.AgentApprovalButtonSlot {}
        }
    }
}
