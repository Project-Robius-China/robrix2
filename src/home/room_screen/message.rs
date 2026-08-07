//! The `Message` timeline item widget, its action enum, and the whole
//! message DSL template family (plain/condensed/sticker/image/video/
//! audio variants and their shared sub-templates).

use super::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.COLOR_THREAD_SUMMARY_BG = #FFF4E5
    mod.widgets.COLOR_THREAD_SUMMARY_BG_HOVER = #FFEACC
    mod.widgets.COLOR_THREAD_SUMMARY_BORDER = #E8C99A
    mod.widgets.COLOR_THREAD_SUMMARY_REPLY_COUNT = #A35A00
    // Aliases onto the token layer. These were seven hand-picked blue-greys that
    // belonged to no ramp — each within ~2% of a real token, which is exactly
    // the drift that makes an agent card read as slightly "off" next to the
    // approval card and every other surface nested inside it.
    mod.widgets.COLOR_BOT_CARD_BG = (mod.widgets.RBX_BG_CANVAS)
    mod.widgets.COLOR_BOT_CARD_BORDER = (mod.widgets.RBX_STROKE_STRONG)
    mod.widgets.COLOR_BOT_STATUS_BG = (mod.widgets.RBX_BG_SUNKEN)
    mod.widgets.COLOR_BOT_STATUS_TEXT = (mod.widgets.RBX_FG_SECONDARY)
    mod.widgets.COLOR_BOT_FOOTER_TEXT = (mod.widgets.RBX_FG_TERTIARY)
    mod.widgets.COLOR_BOT_CODE_BG = (mod.widgets.RBX_BG_SUNKEN)
    mod.widgets.COLOR_BOT_CODE_BORDER = (mod.widgets.RBX_STROKE_STRONG)

    mod.widgets.MessageActionPrimaryButton = RobrixPositiveIconButton {
        width: Fit
        height: Fit
        spacing: 6.0
        padding: Inset{ left: 10.0, right: 10.0, top: 7.0, bottom: 7.0 }
        draw_text +: {
            text_style: mod.widgets.MESSAGE_TEXT_STYLE { font_size: 10.0 }
        }
    }

    mod.widgets.MessageActionSecondaryButton = Button {
        width: Fit
        height: Fit
        spacing: 6.0
        padding: Inset{ left: 10.0, right: 10.0, top: 7.0, bottom: 7.0 }
        draw_text +: {
            text_style: mod.widgets.MESSAGE_TEXT_STYLE { font_size: 10.0 }
        }
        text: ""
    }

    mod.widgets.MessageActionDangerButton = RobrixNegativeIconButton {
        width: Fit
        height: Fit
        spacing: 6.0
        padding: Inset{ left: 10.0, right: 10.0, top: 7.0, bottom: 7.0 }
        draw_text +: {
            text_style: mod.widgets.MESSAGE_TEXT_STYLE { font_size: 10.0 }
        }
    }

    mod.widgets.SmallStateGroupToggleButton = Button {
        width: Fit
        height: Fit
        spacing: 0.0
        padding: Inset{ left: 0.0, right: 0.0, top: 0.0, bottom: 0.0 }
        draw_bg +: {
            border_radius: 4.0
            border_size: 0.0
            border_color: #x00000000
            border_color_hover: #x00000000
            border_color_down: #x00000000
            color: #x00000000
            color_hover: #x00000000
            color_down: #x00000000
        }
        draw_text +: {
            text_style: SMALL_STATE_TEXT_STYLE { font_size: 11.0 }
            color: #x232A31
            color_hover: #x1A1F25
            color_down: #x0E1217
        }
        text: ""
    }

    mod.widgets.MessageActionButtonSlot = View {
        visible: false
        width: Fit
        height: Fit
        flow: Overlay

        primary_button := mod.widgets.MessageActionPrimaryButton {
            visible: false
        }
        secondary_button := mod.widgets.MessageActionSecondaryButton {
            visible: false
        }
        danger_button := mod.widgets.MessageActionDangerButton {
            visible: false
        }
    }

    mod.widgets.BotTimelineMarkdown = Markdown {
        width: Fill
        height: Fit
        padding: 0.0
        font_size: (MESSAGE_FONT_SIZE)
        font_color: (MESSAGE_TEXT_COLOR)
        paragraph_spacing: 10.0
        pre_code_spacing: 8.0
        heading_base_scale: 1.45
        inline_code_padding: Inset{ top: 3, bottom: 3, left: 4, right: 4 }
        inline_code_margin: Inset{ left: 3, right: 3, bottom: 2, top: 2 }
        use_code_block_widget: true

        draw_text +: {
            color: (MESSAGE_TEXT_COLOR)
        }
        text_style_normal: mod.widgets.MESSAGE_TEXT_STYLE {
            font_size: (MESSAGE_FONT_SIZE)
            line_spacing: (MESSAGE_TEXT_LINE_SPACING)
        }
        text_style_italic: theme.font_italic {
            font_size: (MESSAGE_FONT_SIZE)
            line_spacing: (MESSAGE_TEXT_LINE_SPACING)
        }
        text_style_bold: theme.font_bold {
            font_size: (MESSAGE_FONT_SIZE)
            line_spacing: (MESSAGE_TEXT_LINE_SPACING)
        }
        text_style_bold_italic: theme.font_bold_italic {
            font_size: (MESSAGE_FONT_SIZE)
            line_spacing: (MESSAGE_TEXT_LINE_SPACING)
        }
        text_style_fixed: mod.widgets.MESSAGE_CODE_TEXT_STYLE {
            font_size: (MESSAGE_FONT_SIZE - 0.5)
            line_spacing: (MESSAGE_TEXT_LINE_SPACING)
        }
        draw_block +: {
            line_color: (MESSAGE_TEXT_COLOR)
            sep_color: (mod.widgets.COLOR_BOT_CODE_BORDER)
            quote_bg_color: (mod.widgets.RBX_BG_SUNKEN)
            quote_fg_color: (mod.widgets.RBX_FG_SECONDARY)
            code_color: (mod.widgets.COLOR_BOT_CODE_BG)
        }
        code_layout: Layout{
            flow: Flow.Right{wrap: true}
            padding: Inset{ left: 0.0, right: 0.0, top: 0.0, bottom: 0.0 }
        }
        code_walk: Walk{ width: Fill, height: Fit, margin: Inset{ top: 10.0, bottom: 10.0 } }
        quote_layout: Layout{
            flow: Flow.Right{wrap: true}
            padding: Inset{ left: 12.0, right: 12.0, top: 8.0, bottom: 8.0 }
        }
        quote_walk: Walk{ width: Fill, height: Fit, margin: Inset{ top: 6.0, bottom: 6.0 } }
        list_item_layout: Layout{
            flow: Flow.Right{wrap: true}
            padding: Inset{ left: 0.0, right: 0.0, top: 1.0, bottom: 1.0 }
        }
        list_item_walk: Walk{ width: Fill, height: Fit, margin: Inset{ top: 0.0, bottom: 1.0 } }

        code_block := RoundedView {
            width: Fill
            height: Fit
            flow: Overlay
            padding: 0.0
            show_bg: true
            draw_bg +: {
                color: (mod.widgets.COLOR_BOT_CODE_BG)
                border_radius: 10.0
                border_size: 1.0
                border_color: (mod.widgets.COLOR_BOT_CODE_BORDER)
            }

            code_view := mod.widgets.CodeView {
                keep_cursor_at_end: false
                editor +: {
                    width: Fill
                    height: Fit
                    margin: Inset{ left: 12.0, right: 12.0, top: 10.0, bottom: 10.0 }
                    draw_bg +: { color: #0000 }
                    draw_text +: {
                        text_style: mod.widgets.MESSAGE_CODE_TEXT_STYLE {
                            font_size: (MESSAGE_FONT_SIZE - 0.5)
                            line_spacing: (MESSAGE_TEXT_LINE_SPACING)
                        }
                    }
                    token_colors +: {
                        whitespace: #x6a737d
                        delimiter: #x24292e
                        delimiter_highlight: #x005cc5
                        error_decoration: #xcb2431
                        warning_decoration: #xb08800
                        unknown: #x24292e
                        branch_keyword: #xd73a49
                        constant: #x005cc5
                        identifier: #x24292e
                        loop_keyword: #xd73a49
                        number: #x005cc5
                        other_keyword: #xd73a49
                        punctuator: #x24292e
                        string: #x22863a
                        function: #x6f42c1
                        typename: #xe36209
                        comment: #x6a737d
                    }
                }
            }
        }
    }

    // An empty view that takes up no space in the portal list.
    mod.widgets.Empty = View { }

    mod.widgets.MessageDownloadSection = View {
        visible: false,
        width: Fit, height: Fit,
        flow: Right,
        margin: Inset{top: 8, bottom: 2}

        download_button := RobrixIconButton {
            height: mod.widgets.SETTINGS_BUTTON_HEIGHT,
            padding: Inset{left: 12, right: 12}
            margin: 0
            draw_icon.svg: (ICON_DOWNLOAD)
            icon_walk: Walk{width: 16, height: 16}
            text: "Download"
        }

        downloading_view := View {
            visible: false,
            width: Fit, height: mod.widgets.SETTINGS_BUTTON_HEIGHT
            flow: Right,
            align: Align{y: 0.5}
            spacing: 8,
            padding: Inset{left: 12, right: 6}

            spinner := LoadingSpinner {
                width: 16, height: 16
                draw_bg.color: (COLOR_ACTIVE_PRIMARY)
            }
            status_label := Label {
                width: Fit, height: Fit
                padding: 0
                margin: 0
                draw_text +: {
                    text_style: REGULAR_TEXT { font_size: 11 },
                    color: (COLOR_ACTIVE_PRIMARY)
                }
                text: "Downloading…"
            }
            cancel_button := RobrixNegativeIconButton {
                height: mod.widgets.SETTINGS_BUTTON_HEIGHT,
                padding: Inset{left: 12, right: 12}
                margin: 0
                draw_icon.svg: (ICON_CLOSE)
                icon_walk: Walk{width: 16, height: 16}
                text: "Cancel"
            }
        }

        success_button := RobrixPositiveIconButton {
            visible: false,
            height: mod.widgets.SETTINGS_BUTTON_HEIGHT,
            padding: Inset{left: 12, right: 12}
            margin: 0
            draw_icon.svg: (ICON_CHECKMARK)
            icon_walk: Walk{width: 16, height: 16}
            text: "Downloaded"
        }

        failure_button := RobrixNegativeIconButton {
            visible: false,
            height: mod.widgets.SETTINGS_BUTTON_HEIGHT,
            padding: Inset{left: 12, right: 12}
            margin: 0
            draw_icon.svg: (ICON_CLOSE)
            icon_walk: Walk{width: 16, height: 16}
            text: "Download Failed"
        }
    }

    // A summary at the bottom of a message that is the root of a thread.
    mod.widgets.ThreadRootSummary = RoundedView {
        visible: false
        width: Fill,
        height: Fit
        flow: Right,
        align: Align{x: 0.0, y: 0.5}
        spacing: 5.0
        margin: Inset{ top: 5.0 }
        padding: 12,
        cursor: MouseCursor.Hand

        show_bg: true
        draw_bg +: {
            color: (mod.widgets.COLOR_THREAD_SUMMARY_BG)
            border_radius: 4.0
            border_size: 1.5
            border_color: (mod.widgets.COLOR_THREAD_SUMMARY_BORDER)
        }

        thread_summary_count := Label {
            width: Fit,
            draw_text +: {
                text_style: USERNAME_TEXT_STYLE { font_size: 11 }
                color: (mod.widgets.COLOR_THREAD_SUMMARY_REPLY_COUNT)
            }
            text: ""
        }

        Icon {
            width: Fit, height: Fit,
            align: Align{x: 0.5, y: 0.5}
            draw_icon +: {
                svg: crate_resource("self://resources/icons/double_chat.svg")
                color: (mod.widgets.COLOR_THREAD_SUMMARY_REPLY_COUNT)
            }
            icon_walk: Walk{ width: 25, height: 25, margin: Inset{top: 3, right: 7} }
        }

        thread_summary_latest := MessageHtml {
            flow: Right,
            max_lines: 2
            text_overflow: Ellipsis
        }
    }

    // The view used for each text-based message event in a room's timeline.
    // The per-message meta band: copy action (left) · bot model metadata
    // (middle) · read receipts (right). Declared once and instantiated by both
    // the Message and CondensedMessage templates — CondensedMessage re-declares
    // its whole `body` subtree with `:=`, so it cannot inherit this from Message.
    // (Named MessageMetaBand to avoid colliding with the retired floating
    // MessageActionBar popup referenced in a commented-out block below.)
    // Bottom-right delivery indicator for a message still in the send queue.
    // Rides at the trailing edge of the meta band so it costs no vertical
    // space: animated dots while the local echo is in flight, a warning
    // button once the send has failed — clicking it asks whether to resend.
    mod.widgets.SendStateIndicator = View {
        visible: false
        width: Fit,
        height: Fit
        align: Align{y: 0.5}

        sending_dots := BouncingDots {
            visible: false
            width: 20,
            height: 10
            draw_bg +: {
                color: (RBX_FG_TERTIARY)
                dot_radius: 1.3
            }
        }

        send_failure_button := RobrixIconButton {
            visible: false
            width: Fit, height: Fit
            padding: Inset{left: 5.0, right: 5.0, top: 3.0, bottom: 3.0}
            draw_bg +: {
                color: (RBX_DANGER_BG)
                color_hover: (RBX_DANGER_BG)
                color_down: (RBX_BG_PRESSED)
                border_radius: (RBX_RADIUS_PILL)
                border_size: 1.0
                border_color: (RBX_DANGER_FG)
                border_color_hover: (RBX_DANGER_FG)
                border_color_down: (RBX_DANGER_FG)
            }
            draw_icon +: { svg: (ICON_WARNING), color: (RBX_DANGER_FG) }
            icon_walk: Walk{width: 12, height: 12}
            text: ""
        }
    }

    mod.widgets.MessageMetaBand = View {
        width: Fill,
        height: Fit
        flow: Right,
        align: Align{y: 0.5}
        spacing: (SPACE_XS)

        // Fold affordance for an over-long plain message. Bot replies carry
        // their own toggle inside the card footer; this is the equivalent for
        // everything else, and it lives in the meta band so no message template
        // needs a new slot.
        plain_fold_toggle := mod.widgets.SmallStateGroupToggleButton {
            visible: false
            padding: Inset{ left: 0.0, right: 6.0, top: 0.0, bottom: 0.0 }
            draw_text +: {
                text_style: RBX_TEXT_META {}
                color: (RBX_ACCENT)
                color_hover: (RBX_ACCENT_HOVER)
                color_down: (RBX_ACCENT_PRESSED)
                color_focus: (RBX_ACCENT)
            }
            text: ""
        }

        // Message-type badge (request / reply / info) for bridge-relayed agent
        // messages, derived from the leading emoji marker the bridge stamps on
        // the body — the one language-independent signal on the wire. It rides
        // in the meta band rather than on its own row so it costs no vertical
        // space. Hidden for every message without that marker.
        kind_badge := RoundedView {
            visible: false
            width: Fit,
            height: Fit
            padding: Inset{ left: 6.0, right: 6.0, top: 1.0, bottom: 1.0 }
            show_bg: true
            draw_bg +: {
                color: (RBX_ACCENT_SOFT)
                border_radius: 3.0
            }

            kind_badge_label := Label {
                width: Fit,
                height: Fit
                padding: 0
                draw_text +: {
                    text_style: RBX_TEXT_META {}
                    color: (RBX_ACCENT)
                }
                text: ""
            }
        }
        copy_button := RobrixNeutralIconButton {
            visible: false
            width: Fit,
            height: Fit,
            padding: (SPACE_XS)
            // Optical alignment: cancel the button's own SPACE_XS padding so
            // the icon glyph lines up with the message text's left edge.
            margin: Inset{ left: -4 }
            spacing: 0
            draw_bg +: {
                color: (RBX_TRANSPARENT)
                color_hover: (RBX_HIT_HOVER)
                color_down: (RBX_HIT_DOWN)
                border_size: 0.0
            }
            draw_icon +: { svg: (ICON_COPY), color: (RBX_FG_TERTIARY) }
            icon_walk: Walk{width: (RBX_ICON_SM), height: (RBX_ICON_SM)}
            text: ""
        }
        metadata_label := Label {
            visible: false
            width: Fill,
            height: Fit
            padding: 0
            max_lines: 1
            text_overflow: Ellipsis
            draw_text +: {
                text_style: RBX_TEXT_META {}
                color: (RBX_FG_TERTIARY)
            }
            text: ""
        }
        avatar_row := mod.widgets.AvatarRow {}

        send_state_indicator := mod.widgets.SendStateIndicator {}
    }

    mod.widgets.Message = set_type_default() do #(Message::register_widget(vm)) {

        width: Fill,
        height: Fit,
        margin: 0.0
        flow: Down,
        cursor: MouseCursor.Default,
        padding: 0.0,
        spacing: 0.0

        show_bg: true
        draw_bg +: {
            highlight: instance(0.0)
            hover: instance(0.0)
            color: instance((COLOR_PRIMARY)) // default color)

            mentions_bar_color: instance((COLOR_PRIMARY))
            mentions_bar_width: instance(4.0)

            pixel: fn() {
                let base_color = mix(
                    self.color,
                    #fafafa,
                    self.hover
                );

                let with_highlight = mix(
                    base_color,
                    #c5d6fa,
                    self.highlight
                );

                let sdf = Sdf2d.viewport(self.pos * self.rect_size);

                // draw bg
                sdf.rect(0., 0., self.rect_size.x, self.rect_size.y);
                sdf.fill(with_highlight);

                // draw the left vertical line
                sdf.rect(0., 0., self.mentions_bar_width, self.rect_size.y);
                sdf.fill(self.mentions_bar_color);

                return sdf.result;
            }
        }

        animator: Animator{
            highlight: {
                default: @off
                off: AnimatorState{
                    redraw: true,
                    from: { all: Forward {duration: 2.0} }
                    ease: ExpDecay {d1: 0.80, d2: 0.97}
                    apply: { draw_bg: {highlight: 0.0} }
                }
                on: AnimatorState{
                    redraw: true,
                    from: { all: Forward {duration: 0.5} }
                    ease: ExpDecay {d1: 0.80, d2: 0.97}
                    apply: { draw_bg: {highlight: 1.0} }
                }
            }
            hover: {
                default: @off
                off: AnimatorState{
                    redraw: true,
                    from: { all: Snap }
                    apply: { draw_bg: {hover: 0.0} }
                }
                on: AnimatorState{
                    redraw: true,
                    from: { all: Snap }
                    apply: { draw_bg: {hover: 1.0} }
                }
            }
        }

        // A preview of the earlier message that this message was in reply to.
        replied_to_message := mod.widgets.RepliedToMessage {
            flow: Right
            margin: Inset{ bottom: 3, top: 10 }
            replied_to_message_content +: {
                margin +: { left: 29 }
                padding +: { bottom: 10 }
            }
        }

        body := View {
            width: Fill,
            height: Fit
            flow: Right,
            padding: Inset{top: 0, bottom: 10, left: 10, right: 10},

            profile := View {
                align: Align{x: 0.5, y: 0.0} // centered horizontally, top aligned
                width: 65.0,
                height: Fit,
                margin: Inset{top: #(MESSAGE_PROFILE_TOP_MARGIN), right: 10}
                flow: Down,
                avatar := Avatar {
                    width: #(MESSAGE_PROFILE_AVATAR_SIZE),
                    height: #(MESSAGE_PROFILE_AVATAR_SIZE),
                }
                edited_indicator := EditedIndicator { }
                tsp_sign_indicator := TspSignIndicator { }
            }

            content := View {
                width: Fill,
                height: Fit
                flow: Down,
                padding: 0.0

                username_view := View {
                    flow: Right,
                    align: Align{y: 0.5},
                    width: Fit,
                    height: #(MESSAGE_USERNAME_ROW_HEIGHT),
                    margin: Inset{
                        top: #(MESSAGE_USERNAME_ROW_TOP_MARGIN),
                        bottom: #(MESSAGE_USERNAME_ROW_BOTTOM_MARGIN),
                    }
                    username := Label {
                        width: Fit,
                        flow: Right, // do not wrap
                        padding: 0,
                        margin: Inset{right: #(MESSAGE_USERNAME_RIGHT_MARGIN)}
                        max_lines: 1
                        text_overflow: Ellipsis
                        draw_text +: {
                            text_style: USERNAME_TEXT_STYLE {},
                            color: (USERNAME_TEXT_COLOR)
                        }
                        text: ""
                    }
                    bot_badge := RoundedView {
                        visible: false
                        width: Fit
                        height: #(BOT_BADGE_HEIGHT)
                        align: Align{x: 0.5, y: 0.5}
                        padding: Inset{left: #(BOT_BADGE_HORIZONTAL_PADDING), right: #(BOT_BADGE_HORIZONTAL_PADDING)}
                        show_bg: true
                        draw_bg +: {
                            color: (RBX_ACCENT_SOFT)
                            border_radius: #(BOT_BADGE_BORDER_RADIUS)
                        }
                        bot_badge_label := Label {
                            width: Fit
                            height: Fit
                            padding: 0
                            draw_text +: {
                                text_style: REGULAR_TEXT {
                                    font_size: #(BOT_BADGE_TEXT_FONT_SIZE)
                                    top_drop: #(BOT_BADGE_TEXT_TOP_DROP)
                                }
                                color: (RBX_ACCENT)
                            }
                            text: "bot"
                        }
                    }
                    timestamp := Timestamp {
                        margin: Inset{ left: (SPACE_XS) }
                    }
                }

                bot_message_card := View {
                    visible: false
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 6.0
                    margin: Inset{ top: 1.0, bottom: 3.0 }

                    bot_status_strip := RoundedView {
                        visible: false
                        width: Fit
                        height: Fit
                        padding: Inset{ left: 10.0, right: 10.0, top: 5.0, bottom: 5.0 }
                        show_bg: true
                        draw_bg +: {
                            color: (mod.widgets.COLOR_BOT_STATUS_BG)
                            border_radius: 10.0
                        }

                        bot_status_label := Label {
                            width: Fit
                            height: Fit
                            draw_text +: {
                                text_style: mod.widgets.MESSAGE_TEXT_STYLE { font_size: 9.5 }
                                color: (mod.widgets.COLOR_BOT_STATUS_TEXT)
                            }
                            text: ""
                        }
                    }

                    bot_body_card := RoundedView {
                        width: Fill
                        height: Fit
                        flow: Down
                        padding: Inset{ left: 14.0, right: 14.0, top: 12.0, bottom: 12.0 }
                        show_bg: true
                        draw_bg +: {
                            color: (mod.widgets.COLOR_BOT_CARD_BG)
                            border_radius: 6.0
                            border_size: 1.0
                            border_color: (mod.widgets.COLOR_BOT_CARD_BORDER)
                        }

                        bot_card_body := HtmlOrPlaintext { }
                        bot_card_markdown := mod.widgets.BotTimelineMarkdown {
                            body: ""
                        }
                        bot_card_markdown_plain := mod.widgets.BotTimelineMarkdown {
                            use_code_block_widget: false
                            body: ""
                        }

                        // Card footer: the fold affordance and the permalink sit
                        // on one row so both stay reachable while folded.
                        bot_card_footer_row := View {
                            width: Fill
                            height: Fit
                            flow: Right
                            align: Align{y: 0.5}
                            spacing: 10.0
                            margin: Inset{ top: 6.0 }

                            // Fold affordance for long agent replies. Only shown
                            // when the body exceeded the fold threshold.
                            bot_body_fold_toggle := mod.widgets.SmallStateGroupToggleButton {
                                visible: false
                                padding: Inset{ left: 0.0, right: 6.0, top: 0.0, bottom: 0.0 }
                                draw_text +: {
                                    text_style: mod.widgets.MESSAGE_TEXT_STYLE { font_size: 9.5 }
                                    // The base widget only defines color/hover/down,
                                    // so an unset focus colour turned the label white
                                    // the moment a click focused it, until a later
                                    // redraw happened to reset the state.
                                    color: (mod.widgets.RBX_ACCENT)
                                    color_hover: (mod.widgets.RBX_ACCENT_HOVER)
                                    color_down: (mod.widgets.RBX_ACCENT_PRESSED)
                                    color_focus: (mod.widgets.RBX_ACCENT)
                                }
                                text: ""
                            }

                            // The bridge's permalink, pinned here so folding the
                            // body never hides it.
                            bot_permalink_link := LinkLabel {
                                visible: false
                                width: Fit
                                height: Fit
                                padding: 0
                                margin: 0
                                spacing: 0
                                draw_text +: {
                                    text_style: mod.widgets.MESSAGE_TEXT_STYLE { font_size: 9.5 }
                                    // The base widget only defines color/hover/down,
                                    // so an unset focus colour turned the label white
                                    // the moment a click focused it, until a later
                                    // redraw happened to reset the state.
                                    color: (mod.widgets.RBX_ACCENT)
                                    color_hover: (mod.widgets.RBX_ACCENT_HOVER)
                                    color_down: (mod.widgets.RBX_ACCENT_PRESSED)
                                    color_focus: (mod.widgets.RBX_ACCENT)
                                }
                                text: ""
                            }
                        }
                    }

                }

                message := HtmlOrPlaintext { }
                splash_card := Splash { }
                action_buttons := View {
                    visible: false
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 6.0
                    margin: Inset{ top: 8.0, bottom: 2.0 }

                    approval_request_view := mod.widgets.AgentApprovalCard {}

                    action_button_row := View {
                        visible: false
                        width: Fill
                        height: Fit
                        flow: Flow.Right{wrap: true}
                        spacing: 8.0

                        action_button_slot_0 := mod.widgets.MessageActionButtonSlot {}
                        action_button_slot_1 := mod.widgets.MessageActionButtonSlot {}
                        action_button_slot_2 := mod.widgets.MessageActionButtonSlot {}
                        action_button_slot_3 := mod.widgets.MessageActionButtonSlot {}
                        action_button_slot_4 := mod.widgets.MessageActionButtonSlot {}
                        action_button_slot_5 := mod.widgets.MessageActionButtonSlot {}
                    }
                }
                link_preview_view := mod.widgets.LinkPreview {}
                download_section := mod.widgets.MessageDownloadSection {}
                message_action_bar := mod.widgets.MessageMetaBand {}
                View {
                    width: Fill,
                    height: Fit
                    flow: Right,
                    reaction_list := mod.widgets.ReactionList { }
                }
                thread_root_summary := mod.widgets.ThreadRootSummary {}
            }
        }
    }

    // The view used for a condensed message that came right after another message
    // from the same sender, and thus doesn't need to display the sender's profile again.
    mod.widgets.CondensedMessage = mod.widgets.Message {
        padding: Inset{ top: 2.0, bottom: 2.0 }
        replied_to_message +: {
            replied_to_message_content +: {
                margin: Inset{ left: 74, bottom: 5.0 }
            }
        }
        body := View {
            width: Fill,
            height: Fit
            flow: Right,
            padding: Inset{ top: 0, bottom: 2.5, left: 10.0, right: 10.0 },
            profile := View {
                align: Align{x: 0.5, y: 0.0} // centered horizontally, top aligned
                width: 65.0,
                height: Fit,
                flow: Down,
                timestamp := Timestamp {
                    margin: Inset{top: 2.5}
                }
                edited_indicator := EditedIndicator { }
                tsp_sign_indicator := TspSignIndicator { }
            }
            content := View {
                width: Fill,
                height: Fit,
                flow: Down,
                padding: Inset{ left: 10.0 }

                bot_message_card := View {
                    visible: false
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 6.0
                    margin: Inset{ top: 1.0, bottom: 3.0 }

                    bot_status_strip := RoundedView {
                        visible: false
                        width: Fit
                        height: Fit
                        padding: Inset{ left: 10.0, right: 10.0, top: 5.0, bottom: 5.0 }
                        show_bg: true
                        draw_bg +: {
                            color: (mod.widgets.COLOR_BOT_STATUS_BG)
                            border_radius: 10.0
                        }

                        bot_status_label := Label {
                            width: Fit
                            height: Fit
                            draw_text +: {
                                text_style: mod.widgets.MESSAGE_TEXT_STYLE { font_size: 9.5 }
                                color: (mod.widgets.COLOR_BOT_STATUS_TEXT)
                            }
                            text: ""
                        }
                    }

                    bot_body_card := RoundedView {
                        width: Fill
                        height: Fit
                        flow: Down
                        padding: Inset{ left: 14.0, right: 14.0, top: 12.0, bottom: 12.0 }
                        show_bg: true
                        draw_bg +: {
                            color: (mod.widgets.COLOR_BOT_CARD_BG)
                            border_radius: 6.0
                            border_size: 1.0
                            border_color: (mod.widgets.COLOR_BOT_CARD_BORDER)
                        }

                        bot_card_body := HtmlOrPlaintext { }
                        bot_card_markdown := mod.widgets.BotTimelineMarkdown {
                            body: ""
                        }
                        bot_card_markdown_plain := mod.widgets.BotTimelineMarkdown {
                            use_code_block_widget: false
                            body: ""
                        }

                        // Card footer: the fold affordance and the permalink sit
                        // on one row so both stay reachable while folded.
                        bot_card_footer_row := View {
                            width: Fill
                            height: Fit
                            flow: Right
                            align: Align{y: 0.5}
                            spacing: 10.0
                            margin: Inset{ top: 6.0 }

                            // Fold affordance for long agent replies. Only shown
                            // when the body exceeded the fold threshold.
                            bot_body_fold_toggle := mod.widgets.SmallStateGroupToggleButton {
                                visible: false
                                padding: Inset{ left: 0.0, right: 6.0, top: 0.0, bottom: 0.0 }
                                draw_text +: {
                                    text_style: mod.widgets.MESSAGE_TEXT_STYLE { font_size: 9.5 }
                                    // The base widget only defines color/hover/down,
                                    // so an unset focus colour turned the label white
                                    // the moment a click focused it, until a later
                                    // redraw happened to reset the state.
                                    color: (mod.widgets.RBX_ACCENT)
                                    color_hover: (mod.widgets.RBX_ACCENT_HOVER)
                                    color_down: (mod.widgets.RBX_ACCENT_PRESSED)
                                    color_focus: (mod.widgets.RBX_ACCENT)
                                }
                                text: ""
                            }

                            // The bridge's permalink, pinned here so folding the
                            // body never hides it.
                            bot_permalink_link := LinkLabel {
                                visible: false
                                width: Fit
                                height: Fit
                                padding: 0
                                margin: 0
                                spacing: 0
                                draw_text +: {
                                    text_style: mod.widgets.MESSAGE_TEXT_STYLE { font_size: 9.5 }
                                    // The base widget only defines color/hover/down,
                                    // so an unset focus colour turned the label white
                                    // the moment a click focused it, until a later
                                    // redraw happened to reset the state.
                                    color: (mod.widgets.RBX_ACCENT)
                                    color_hover: (mod.widgets.RBX_ACCENT_HOVER)
                                    color_down: (mod.widgets.RBX_ACCENT_PRESSED)
                                    color_focus: (mod.widgets.RBX_ACCENT)
                                }
                                text: ""
                            }
                        }
                    }

                }

                message := HtmlOrPlaintext { }
                action_buttons := View {
                    visible: false
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 6.0
                    margin: Inset{ top: 8.0, bottom: 2.0 }

                    approval_request_view := mod.widgets.AgentApprovalCard {}

                    action_button_row := View {
                        visible: false
                        width: Fill
                        height: Fit
                        flow: Flow.Right{wrap: true}
                        spacing: 8.0

                        action_button_slot_0 := mod.widgets.MessageActionButtonSlot {}
                        action_button_slot_1 := mod.widgets.MessageActionButtonSlot {}
                        action_button_slot_2 := mod.widgets.MessageActionButtonSlot {}
                        action_button_slot_3 := mod.widgets.MessageActionButtonSlot {}
                        action_button_slot_4 := mod.widgets.MessageActionButtonSlot {}
                        action_button_slot_5 := mod.widgets.MessageActionButtonSlot {}
                    }
                }
                link_preview_view := mod.widgets.LinkPreview {}
                download_section := mod.widgets.MessageDownloadSection {}
                message_action_bar := mod.widgets.MessageMetaBand {}
                View {
                    width: Fill,
                    height: Fit
                    flow: Right,
                    reaction_list := mod.widgets.ReactionList { }
                }
                thread_root_summary := mod.widgets.ThreadRootSummary {}
            }
        }
    }

    mod.widgets.IMG_MSG_FIT = Fit{max: FitBound.Abs(200.0)}
    mod.widgets.STICKER_HEIGHT = 150.0

    // Sticker message templates: fixed height, width determined by aspect ratio.
    mod.widgets.StickerMessage = mod.widgets.Message {
        body +: {
            content +: {
                width: Fill,
                height: Fit
                padding: Inset{ left: 10.0 }

                message := TextOrImage {
                    width: Fit, height: Fit,
                    image_view +: { width: Fit, height: Fit, image +: {
                        height: (mod.widgets.STICKER_HEIGHT)
                        width: (mod.widgets.STICKER_HEIGHT)
                        fit: ImageFit.Smallest
                    } }
                    default_image_view +: { width: Fit, height: Fit, image +: {
                        height: (mod.widgets.STICKER_HEIGHT)
                        width: (mod.widgets.STICKER_HEIGHT)
                        fit: ImageFit.Smallest
                    } }
                }
                View {
                    width: Fill,
                    height: Fit,
                    flow: Right,
                    reaction_list := mod.widgets.ReactionList { }
                }
                thread_root_summary := mod.widgets.ThreadRootSummary {}
            }
        }
    }

    mod.widgets.CondensedStickerMessage = mod.widgets.CondensedMessage {
        body +: {
            content +: {
                message := TextOrImage {
                    width: Fit, height: Fit,
                    image_view +: { width: Fit, height: Fit, image +: {
                        height: (mod.widgets.STICKER_HEIGHT)
                        width: (mod.widgets.STICKER_HEIGHT)
                        fit: ImageFit.Smallest
                    } }
                    default_image_view +: { width: Fit, height: Fit, image +: {
                        height: (mod.widgets.STICKER_HEIGHT)
                        width: (mod.widgets.STICKER_HEIGHT)
                        fit: ImageFit.Smallest
                    } }
                }
                View {
                    width: Fill,
                    height: Fit,
                    flow: Right,
                    reaction_list := mod.widgets.ReactionList { }
                }
                thread_root_summary := mod.widgets.ThreadRootSummary {}
            }
        }
    }

    // The view used for each static image-based message event in a room's timeline.
    // This excludes stickers and other animated GIFs, video clips, audio clips, etc.
    mod.widgets.ImageMessage = mod.widgets.Message {
        body +: {
            content +: {
                width: Fill,
                height: Fit
                padding: Inset{ left: 10.0 }

                message := TextOrImage {
                    image_view +: { image +: {
                        height: (mod.widgets.IMG_MSG_FIT)
                    } }
                    default_image_view +: { image +: {
                        height: (mod.widgets.IMG_MSG_FIT)
                    } }
                }
                download_section := mod.widgets.MessageDownloadSection {}
                animated_message := mod.widgets.AnimatedImage { visible: false }
                View {
                    width: Fill,
                    height: Fit,
                    flow: Right,
                    reaction_list := mod.widgets.ReactionList { }
                }
                thread_root_summary := mod.widgets.ThreadRootSummary {}
            }

        }
    }

    // The view used for a condensed image message that came right after another message
    // from the same sender, and thus doesn't need to display the sender's profile again.
    // This excludes stickers and other animated GIFs, video clips, audio clips, etc.
    mod.widgets.CondensedImageMessage = mod.widgets.CondensedMessage {
        body +: {
            content +: {
                message := TextOrImage {
                    image_view +: { image +: {
                        height: (mod.widgets.IMG_MSG_FIT)
                    } }
                    default_image_view +: { image +: {
                        height: (mod.widgets.IMG_MSG_FIT)
                    } }
                }
                download_section := mod.widgets.MessageDownloadSection {}
                animated_message := mod.widgets.AnimatedImage { visible: false }
                View {
                    width: Fill,
                    height: Fit,
                    flow: Right,
                    reaction_list := mod.widgets.ReactionList { }
                }
                thread_root_summary := mod.widgets.ThreadRootSummary {}
            }
        }
    }

    // Video message template. Embeds the inline `VideoMessagePlayer` widget
    // above the existing html caption/metadata. The player is populated by
    // `populate_video_message_content` in this file.
    mod.widgets.VideoMessage = mod.widgets.Message {
        body +: {
            content +: {
                video_player := mod.widgets.VideoMessagePlayer {}
            }
        }
    }

    mod.widgets.AudioMessage = mod.widgets.Message {
        body +: {
            content +: {
                audio_player := mod.widgets.AudioMessagePlayer {}
            }
        }
    }
}

pub(super) const MESSAGE_PROFILE_TOP_MARGIN: f64 = 4.5;
pub(super) const MESSAGE_PROFILE_AVATAR_SIZE: f64 = 48.0;
pub(super) const MESSAGE_USERNAME_ROW_HEIGHT: f64 = 18.0;
pub(super) const MESSAGE_USERNAME_ROW_BOTTOM_MARGIN: f64 = 9.0;
pub(super) const MESSAGE_USERNAME_RIGHT_MARGIN: f64 = 4.0;
pub(super) const BOT_BADGE_HEIGHT: f64 = 16.0;
pub(super) const BOT_BADGE_HORIZONTAL_PADDING: f64 = 6.0;
pub(super) const BOT_BADGE_BORDER_RADIUS: f64 = 3.0;
pub(super) const BOT_BADGE_TEXT_FONT_SIZE: f64 = 8.5;
pub(super) const BOT_BADGE_TEXT_TOP_DROP: f64 = -0.08;

pub(super) const fn centered_top_margin(outer_top_margin: f64, outer_height: f64, inner_height: f64) -> f64 {
    outer_top_margin + ((outer_height - inner_height) * 0.5)
}

#[cfg(test)]
pub(super) const fn center_y(top_margin: f64, height: f64) -> f64 {
    top_margin + (height * 0.5)
}

pub(super) const MESSAGE_USERNAME_ROW_TOP_MARGIN: f64 = centered_top_margin(
    MESSAGE_PROFILE_TOP_MARGIN,
    MESSAGE_PROFILE_AVATAR_SIZE,
    MESSAGE_USERNAME_ROW_HEIGHT,
);

#[cfg(test)]
pub(super) fn message_profile_avatar_center_y() -> f64 {
    center_y(MESSAGE_PROFILE_TOP_MARGIN, MESSAGE_PROFILE_AVATAR_SIZE)
}

#[cfg(test)]
pub(super) fn message_username_row_center_y() -> f64 {
    center_y(MESSAGE_USERNAME_ROW_TOP_MARGIN, MESSAGE_USERNAME_ROW_HEIGHT)
}

#[cfg(test)]
pub(super) fn bot_badge_center_y_within_username_row() -> f64 {
    let bot_badge_top_margin = MESSAGE_USERNAME_ROW_TOP_MARGIN
        + ((MESSAGE_USERNAME_ROW_HEIGHT - BOT_BADGE_HEIGHT) * 0.5);
    center_y(bot_badge_top_margin, BOT_BADGE_HEIGHT)
}

#[cfg(test)]
pub(super) fn bot_badge_label_center_y() -> f64 {
    let bot_badge_label_top_margin = (BOT_BADGE_HEIGHT - BOT_BADGE_TEXT_FONT_SIZE) * 0.5
        + (BOT_BADGE_TEXT_FONT_SIZE * BOT_BADGE_TEXT_TOP_DROP);
    center_y(bot_badge_label_top_margin, BOT_BADGE_TEXT_FONT_SIZE)
}

#[derive(Clone, Default, Debug)]
pub enum MessageAction {
    /// The user clicked the "react" button on a message
    /// and wants to send the given `reaction` to that message.
    React {
        details: MessageDetails,
        reaction: String,
    },
    /// The user clicked the "reply" button on a message.
    Reply(MessageDetails),
    /// The user clicked the "edit" button on a message.
    Edit(MessageDetails),
    /// The user requested to edit their latest message in this room.
    EditLatest,
    /// The user submitted a new local message and the timeline should follow the live tail.
    MessageSubmittedLocally,
    /// The user clicked the "pin" button on a message.
    Pin(MessageDetails),
    /// The user clicked the "unpin" button on a message.
    Unpin(MessageDetails),
    /// The user clicked the "copy text" button on a message.
    CopyText(MessageDetails),
    /// The user clicked the "copy HTML" button on a message.
    CopyHtml(MessageDetails),
    /// The user clicked the "copy link" button on a message.
    CopyLink(MessageDetails),
    /// The user clicked the "forward message" button on a message.
    Forward(MessageDetails),
    /// The user clicked the "view source" button on a message.
    ViewSource(MessageDetails),
    /// The user clicked the "jump to related" button on a message,
    /// indicating that they want to auto-scroll back to the related message,
    /// e.g., a replied-to message.
    JumpToRelated(MessageDetails),
    /// The user clicked the thread summary on a thread-root message.
    OpenThread(OwnedEventId),
    /// The user requested to jump to a specific event in this room.
    JumpToEvent(OwnedEventId),
    /// The user clicked the "delete" button on a message.
    #[doc(alias("delete"))]
    Redact {
        details: MessageDetails,
        reason: Option<String>,
    },

    // /// The user clicked the "report" button on a message.
    // Report(MessageDetails),

    /// The user clicked the "Download" button on a media/file message.
    DownloadAttachment(DownloadableAttachment),
    /// The user clicked "Cancel" on an in-progress attachment download.
    CancelDownload(OwnedMxcUri),
    /// The message at the given item index in the timeline should be highlighted.
    HighlightMessage(usize),
    /// The user requested that we show a context menu with actions
    /// that can be performed on a given message.
    OpenMessageContextMenu {
        details: MessageDetails,
        /// The absolute position where we should show the context menu,
        /// in which the (0,0) origin coordinate is the top left corner of the app window.
        abs_pos: DVec2,
        opening_gesture: ContextMenuOpenGesture,
    },
    ToggleTranslationLangPopup {
        button_rect: Rect,
    },
    /// The user requested opening the message action bar
    ActionBarOpen {
        /// At the given timeline item index
        item_id: usize,
        /// The message rect, so the action bar can be positioned relative to it
        message_rect: Rect,
    },
    /// The user requested closing the message action bar
    ActionBarClose,
    /// The user requested toggling the in-room app service quick actions card.
    ToggleAppServiceActions,
    ShowThreadsPane,
    ShowRoomInfoPane,
    #[default]
    None,
}

impl ActionDefaultRef for MessageAction {
    fn default_ref() -> &'static Self {
        static DEFAULT: MessageAction = MessageAction::None;
        &DEFAULT
    }
}

/// A widget representing a single message of any kind within a room timeline.
#[derive(Script, ScriptHook, Widget, Animator)]
pub struct Message {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    #[apply_default] animator: Animator,

    #[rust] details: Option<MessageDetails>,
    /// Set on file/image/audio/video messages so the download button knows
    /// what to save when the user clicks it. `None` for plain text messages,
    /// which hide the download button entirely.
    #[rust] download_info: Option<DownloadableAttachment>,
    /// Cached so `set_data` can reset_hover only on the button that just
    /// transitioned into visibility, not on every redraw.
    #[rust] download_state: DownloadDisplayState,
    /// Whether the meta band's copy button is currently shown. Tracked so
    /// `set_data` only touches the widget when the value flips, and so
    /// `handle_event` can skip the per-Actions `clicked()` lookup entirely
    /// while the button is hidden.
    #[rust] show_copy_button: bool,
    /// The meta band's model-metadata line currently displayed (None = hidden).
    /// Tracked so recycled items only touch the label when the text changes.
    #[rust] band_metadata: Option<String>,
}

impl Widget for Message {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.animator_handle_event(cx, event).must_redraw() {
            self.redraw(cx);
        }

        if !self.animator.is_track_animating(id!(highlight))
            && self.animator_in_state(cx, ids!(highlight.on))
        {
            self.animator_play(cx, ids!(highlight.off));
        }

        let Some(details) = self.details.clone() else { return };

        // We first handle a click on the replied-to message preview, if present,
        // because we don't want any widgets within the replied-to message to be
        // clickable or otherwise interactive.
        match event.hits(cx, self.view(cx, ids!(replied_to_message)).area()) {
            Hit::FingerDown(fe) if fe.device.mouse_button().is_some_and(|b| b.is_secondary()) => {
                cx.widget_action(
                    details.room_screen_widget_uid,
                    MessageAction::OpenMessageContextMenu {
                        details: details.clone(),
                        abs_pos: fe.abs,
                        opening_gesture: ContextMenuOpenGesture::from_finger_down(&fe),
                    }
                );
            }
            Hit::FingerDown(_) => {}
            Hit::FingerLongPress(lp) => {
                cx.widget_action(
                    details.room_screen_widget_uid, 
                    MessageAction::OpenMessageContextMenu {
                        details: details.clone(),
                        abs_pos: lp.abs,
                        opening_gesture: ContextMenuOpenGesture::from_long_press(&lp),
                    }
                );
            }
            // If the hit occurred on the replied-to message preview, jump to it.
            Hit::FingerUp(fe) if fe.is_over && fe.is_primary_hit() && fe.was_tap() => {
                cx.widget_action(
                    details.room_screen_widget_uid, 
                    MessageAction::JumpToRelated(details.clone()),
                );
            }
            _ => { }
        }

        // Handle clicks on the thread summary shown beneath a thread-root message.
        if let Some(thread_root_event_id) = details.thread_root_event_id.as_ref() {
            let thread_root_summary = self.view(cx, ids!(thread_root_summary));
            let apply_hover = |cx: &mut Cx, bg_color: Vec4| {
                let mut thread_root_summary_ref = thread_root_summary.clone();
                script_apply_eval!(cx, thread_root_summary_ref, {
                    draw_bg.color: #(bg_color)
                });
            };
            match event.hits(cx, thread_root_summary.area()) {
                Hit::FingerDown(fe) => {
                    apply_hover(cx, COLOR_THREAD_SUMMARY_BG_HOVER);
                    if fe.device.mouse_button().is_some_and(|b| b.is_secondary()) {
                        cx.widget_action(
                            details.room_screen_widget_uid, 
                            MessageAction::OpenMessageContextMenu {
                                details: details.clone(),
                                abs_pos: fe.abs,
                                opening_gesture: ContextMenuOpenGesture::from_finger_down(&fe),
                            }
                        );
                    }
                }
                Hit::FingerHoverIn(_) => {
                    apply_hover(cx, COLOR_THREAD_SUMMARY_BG_HOVER);
                }
                Hit::FingerHoverOut(_) => {
                    apply_hover(cx, COLOR_THREAD_SUMMARY_BG);
                }
                Hit::FingerLongPress(lp) => {
                    cx.widget_action(
                        details.room_screen_widget_uid, 
                        MessageAction::OpenMessageContextMenu {
                            details: details.clone(),
                            abs_pos: lp.abs,
                            opening_gesture: ContextMenuOpenGesture::from_long_press(&lp),
                        }
                    );
                }
                Hit::FingerUp(fe) => {
                    apply_hover(cx, COLOR_THREAD_SUMMARY_BG);
                    if fe.is_over && fe.is_primary_hit() && fe.was_tap() {
                        cx.widget_action(
                            details.room_screen_widget_uid, 
                            MessageAction::OpenThread(thread_root_event_id.clone()),
                        );
                    }
                }
                _ => { }
            }
        }

        // Next, we forward the event to the child view such that it has the chance
        // to handle it before the Message widget handles it.
        // This ensures that events like right-clicking/long-pressing a reaction button
        // or a link within a message will be treated as an action upon that child view
        // rather than an action upon the message itself.
        self.view.handle_event(cx, event, scope);

        // Finally, handle any hits on the rest of the message body itself.
        let message_view_area = self.view.area();
        match event.hits(cx, message_view_area) {
            Hit::FingerDown(fe) => {
                cx.set_key_focus(message_view_area);
                // A right click means we should display the context menu.
                if fe.device.mouse_button().is_some_and(|b| b.is_secondary()) {
                    cx.widget_action(
                        details.room_screen_widget_uid, 
                        MessageAction::OpenMessageContextMenu {
                            details: details.clone(),
                            abs_pos: fe.abs,
                            opening_gesture: ContextMenuOpenGesture::from_finger_down(&fe),
                        }
                    );
                }
            }
            Hit::FingerLongPress(lp) => {
                cx.widget_action(
                    details.room_screen_widget_uid, 
                    MessageAction::OpenMessageContextMenu {
                        details: details.clone(),
                        abs_pos: lp.abs,
                        opening_gesture: ContextMenuOpenGesture::from_long_press(&lp),
                    }
                );
            }
            Hit::FingerHoverIn(..) => {
                self.animator_play(cx, ids!(hover.on));
                // TODO: here, show the "action bar" buttons upon hover-in
            }
            Hit::FingerHoverOut(_fho) => {
                self.animator_play(cx, ids!(hover.off));
                // TODO: here, hide the "action bar" buttons upon hover-out
            }
            _ => { }
        }

        if let Event::Actions(actions) = event {
            if let Some(info) = self.download_info.as_ref()
                && self.view.button(cx, ids!(content.download_section.download_button)).clicked(actions)
            {
                cx.widget_action(
                    details.room_screen_widget_uid,
                    MessageAction::DownloadAttachment(info.clone()),
                );
            }
            if let Some(info) = self.download_info.as_ref()
                && self.view.button(cx, ids!(content.download_section.downloading_view.cancel_button)).clicked(actions)
            {
                cx.widget_action(
                    details.room_screen_widget_uid,
                    MessageAction::CancelDownload(media_source_mxc(&info.media_source).clone()),
                );
            }
            if self.show_copy_button
                && self.view.button(cx, ids!(content.message_action_bar.copy_button)).clicked(actions)
            {
                cx.widget_action(
                    details.room_screen_widget_uid,
                    MessageAction::CopyText(details.clone()),
                );
            }
            for action in actions {
                match action.as_widget_action().widget_uid_eq(details.room_screen_widget_uid).cast_ref() {
                    MessageAction::HighlightMessage(id) if id == &details.item_id => {
                        self.animator_play(cx, ids!(highlight.on));
                        self.redraw(cx);
                    }
                    _ => {}
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if self.details.as_ref().is_some_and(|d| d.should_be_highlighted) {
            script_apply_eval!(cx, self, {
                draw_bg +: {
                    color: #ffffd1,
                    mentions_bar_color: #ffd54f
                }
            });
        }

        self.view.draw_walk(cx, scope, walk)
    }
}

impl Message {
    pub(super) fn set_data(
        &mut self,
        cx: &mut Cx,
        details: MessageDetails,
        download_info: Option<DownloadableAttachment>,
        download_state: DownloadDisplayState,
        show_copy_button: bool,
    ) {
        let prev_section_visible = self.download_info.is_some();
        let prev_state = self.download_state;

        self.details = Some(details);
        self.download_info = download_info;
        let section_visible = self.download_info.is_some();
        self.view.view(cx, ids!(content.download_section))
            .set_visible(cx, section_visible);
        if self.show_copy_button != show_copy_button {
            let copy_button = self.view.button(cx, ids!(content.message_action_bar.copy_button));
            copy_button.set_visible(cx, show_copy_button);
            if show_copy_button {
                copy_button.reset_hover(cx);
            }
            self.show_copy_button = show_copy_button;
        }
        if let Some(info) = self.download_info.as_ref() {
            let download_button = self.view.button(cx, ids!(content.download_section.download_button));
            let downloading_view = self.view.view(cx, ids!(content.download_section.downloading_view));
            let cancel_button = self.view.button(cx, ids!(content.download_section.downloading_view.cancel_button));
            let success_button = self.view.button(cx, ids!(content.download_section.success_button));
            let failure_button = self.view.button(cx, ids!(content.download_section.failure_button));
            download_button.set_text(cx, info.kind.button_text());
            download_button.set_visible(cx, matches!(download_state, DownloadDisplayState::Idle));
            downloading_view.set_visible(cx, matches!(download_state, DownloadDisplayState::InProgress));
            success_button.set_visible(cx, matches!(download_state, DownloadDisplayState::Succeeded));
            failure_button.set_visible(cx, matches!(download_state, DownloadDisplayState::Failed));
            let newly_visible = !prev_section_visible || prev_state != download_state;
            if newly_visible {
                match download_state {
                    DownloadDisplayState::Idle => download_button.reset_hover(cx),
                    DownloadDisplayState::InProgress => cancel_button.reset_hover(cx),
                    DownloadDisplayState::Succeeded => success_button.reset_hover(cx),
                    DownloadDisplayState::Failed => failure_button.reset_hover(cx),
                }
            }
        }
        self.download_state = download_state;
    }

    /// Sets the meta band's model-metadata line (None hides it).
    ///
    /// Only touches the label widget when the value actually changes, so
    /// recycled PortalList items and the timeline-majority case (human
    /// messages, metadata None → None) cost nothing beyond the comparison.
    pub(super) fn set_band_metadata(&mut self, cx: &mut Cx, band_metadata: Option<String>) {
        if self.band_metadata == band_metadata {
            return;
        }
        let label = self.view.label(cx, ids!(content.message_action_bar.metadata_label));
        label.set_visible(cx, band_metadata.is_some());
        label.set_text(cx, band_metadata.as_deref().unwrap_or(""));
        self.band_metadata = band_metadata;
    }
}

impl MessageRef {
    pub(super) fn set_data(
        &self,
        cx: &mut Cx,
        details: MessageDetails,
        download_info: Option<DownloadableAttachment>,
        download_state: DownloadDisplayState,
        show_copy_button: bool,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_data(cx, details, download_info, download_state, show_copy_button);
    }

    pub(super) fn set_band_metadata(&self, cx: &mut Cx, band_metadata: Option<String>) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_band_metadata(cx, band_metadata);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn center_username_row_aligns_with_avatar_center() {
        assert_eq!(
            message_profile_avatar_center_y(),
            message_username_row_center_y(),
        );
    }

    #[test]
    fn center_bot_badge_aligns_with_username_row_center() {
        assert_eq!(
            message_username_row_center_y(),
            bot_badge_center_y_within_username_row(),
        );
    }

    #[test]
    fn bot_badge_text_is_centered_within_badge() {
        assert!(bot_badge_label_center_y() < (BOT_BADGE_HEIGHT * 0.5));
    }
}
