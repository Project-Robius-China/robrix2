//! The timeline DSL templates: colors, the message template family,
//! small-state rows, date dividers, and the Timeline / RoomScreen
//! screen templates. Later stages move template families next to
//! their Rust code; what remains here is the screen-level DSL.

use super::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.COLOR_BG = #xfff8ee
    mod.widgets.COLOR_OVERLAY_BG = #x000000d8
    mod.widgets.COLOR_READ_MARKER = #xeb2733

    mod.widgets.REACTION_TEXT_COLOR = #4c00b0

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
    // Shown under a message whose send is parked and will not move again on its
    // own: why it failed, and the two ways out. Declared once and instantiated
    // by both message templates, like MessageDownloadSection.
    mod.widgets.SendFailureSection = View {
        visible: false
        width: Fill,
        height: Fit
        flow: Down,
        spacing: (SPACE_XS)
        margin: Inset{ top: 4.0, bottom: 2.0 }

        send_failure_reason := Label {
            width: Fill,
            height: Fit
            flow: Flow.Right{wrap: true}
            draw_text +: {
                text_style: RBX_TEXT_META {}
                color: (RBX_DANGER_FG)
            }
            text: ""
        }

        send_failure_actions := View {
            width: Fill,
            height: Fit
            flow: Right,
            spacing: (SPACE_XS)

            send_retry_button := RobrixIconButton {
                width: Fit, height: Fit
                padding: Inset{left: 10.0, right: 10.0, top: 4.0, bottom: 4.0}
                draw_bg +: {
                    color: (RBX_ACCENT_SOFT)
                    color_hover: (RBX_ACCENT_SOFT)
                    color_down: (RBX_BG_PRESSED)
                    border_radius: (RBX_RADIUS_MD)
                    border_size: 1.0
                    border_color: (RBX_ACCENT)
                    border_color_hover: (RBX_ACCENT)
                    border_color_down: (RBX_ACCENT)
                }
                draw_text +: {
                    text_style: (RBX_TEXT_META)
                    color: (RBX_ACCENT)
                    color_hover: (RBX_ACCENT)
                    color_down: (RBX_ACCENT)
                    color_focus: (RBX_ACCENT)
                }
                draw_icon +: { svg: (ICON_SEND), color: (RBX_ACCENT) }
                icon_walk: Walk{width: 12, height: 12}
                text: "Retry"
            }

            send_discard_button := RobrixIconButton {
                width: Fit, height: Fit
                padding: Inset{left: 10.0, right: 10.0, top: 4.0, bottom: 4.0}
                draw_bg +: {
                    color: (RBX_DANGER_BG)
                    color_hover: (RBX_DANGER_BG)
                    color_down: (RBX_BG_PRESSED)
                    border_radius: (RBX_RADIUS_MD)
                    border_size: 1.0
                    border_color: (RBX_DANGER_FG)
                    border_color_hover: (RBX_DANGER_FG)
                    border_color_down: (RBX_DANGER_FG)
                }
                draw_text +: {
                    text_style: (RBX_TEXT_META)
                    color: (RBX_DANGER_FG)
                    color_hover: (RBX_DANGER_FG)
                    color_down: (RBX_DANGER_FG)
                    color_focus: (RBX_DANGER_FG)
                }
                draw_icon +: { svg: (ICON_TRASH), color: (RBX_DANGER_FG) }
                icon_walk: Walk{width: 12, height: 12}
                text: "Discard"
            }
        }
    }

    // Delivery-state pill for a message still in the send queue. Shares the
    // meta band so it costs no vertical space, and sits next to the read
    // receipts — both answer "did this message land?".
    mod.widgets.SendStatePill = RoundedView {
        visible: false
        width: Fit,
        height: Fit
        padding: Inset{ left: 6.0, right: 6.0, top: 1.0, bottom: 1.0 }
        show_bg: true
        draw_bg +: {
            color: (RBX_NEUTRAL_BG)
            border_radius: (RBX_RADIUS_PILL)
        }

        send_state_label := Label {
            width: Fit,
            height: Fit
            padding: 0
            draw_text +: {
                text_style: RBX_TEXT_META {}
                color: (RBX_NEUTRAL_FG)
            }
            text: ""
        }
    }

    mod.widgets.MessageMetaBand = View {
        width: Fill,
        height: Fit
        flow: Right,
        align: Align{y: 0.5}
        spacing: (SPACE_XS)

        send_state_pill := mod.widgets.SendStatePill {}

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
                send_failure_section := mod.widgets.SendFailureSection {}
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
                send_failure_section := mod.widgets.SendFailureSection {}
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
                send_failure_section := mod.widgets.SendFailureSection {}
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
                send_failure_section := mod.widgets.SendFailureSection {}
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


    // The view used for each state event (non-messages) in a room's timeline.
    // The timestamp, profile picture, and text are all very small.
    mod.widgets.SmallStateEvent = View {
        width: Fill,
        height: Fit,
        flow: Right,
        margin: Inset{ top: 4.0, bottom: 4.0}
        padding: Inset{ top: 1.0, bottom: 1.0, right: 10.0 }
        spacing: 0.0
        cursor: MouseCursor.Default

        body := View {
            width: Fill,
            height: Fit
            flow: Down,
            padding: Inset{ left: 7.0, top: 2.0, bottom: 2.0 }
            spacing: 4.0

            group_header := View {
                visible: false
                width: Fill,
                height: Fit
                flow: Right
                spacing: 0.0
                // 43 + the parent `body`'s 7 = the 50 that
                // `SmallStateEventsSummary` uses, so the summary text keeps its
                // left edge when the group is expanded instead of shifting 7px.
                padding: Inset{ left: 43.0, right: 10.0, bottom: 1.0 }

                group_summary_label := Label {
                    width: Fit,
                    height: Fit
                    draw_text +: {
                        text_style: SMALL_STATE_TEXT_STYLE {}
                        color: (SMALL_STATE_TEXT_COLOR)
                    }
                    text: ""
                }

                spacer := View {
                    width: Fill
                    height: Fit
                }

                state_group_toggle_button := mod.widgets.SmallStateGroupToggleButton {
                    width: Fit
                    height: Fit
                    margin: Inset{ top: 1.0 }
                    text: ""
                }
            }

            event_row := View {
                width: Fill,
                height: Fit
                flow: Right,
                spacing: 5.0

                left_container := View {
                    align: Align{x: 0.5, y: 0}
                    width: 70.0,
                    height: Fit

                    timestamp := Timestamp {
                        margin: Inset{top: 3}
                    }
                }

                avatar := Avatar {
                    width: 19.,
                    height: 19.,
                    margin: 0

                    text_view +: {
                        text +: {
                            draw_text +: {
                                text_style: TITLE_TEXT { font_size: 7.0 }
                            }
                        }
                    }
                }

                // Show an invite button only for a `Knocked` room membership change.
                // All other small state events will not show this button.
                invite_user_button := RobrixPositiveIconButton {
                    visible: false
                    margin: Inset{ top: -1.5, left: 2, right: 2}
                    padding: Inset{top: 4, bottom: 4, left: 9, right: 9}
                    draw_bg +: {
                        border_size: 0.75
                    }
                    draw_icon.svg: (ICON_ADD_USER)
                    draw_text.text_style: SMALL_STATE_TEXT_STYLE {}
                    icon_walk: Walk{width: 15, height: Fit, margin: Inset{right: -4}}
                    text: ""
                }

                content := Label {
                    width: Fill,
                    height: Fit
                    flow: Flow.Right{wrap: true},
                    margin: Inset{top: 2.5}
                    padding: Inset{ top: 0.0, bottom: 0.0, left: 0.0, right: 0.0 }
                    draw_text +: {
                        text_style: SMALL_STATE_TEXT_STYLE {},
                        color: (SMALL_STATE_TEXT_COLOR)
                    }
                    text: ""
                }

                avatar_row := mod.widgets.AvatarRow {}
            }
        }
    }

    // The summary row shown for a collapsed group of adjacent small state events.
    mod.widgets.SmallStateEventsSummary = View {
        width: Fill,
        height: Fit,
        flow: Right,
        margin: Inset{ top: 4.0, bottom: 4.0}
        padding: Inset{ left: 50.0, top: 1.0, bottom: 1.0, right: 10.0 }
        spacing: 7.0
        cursor: MouseCursor.Default

        summary_label := Label {
            width: Fit,
            height: Fit
            flow: Right
            margin: Inset{top: 1.5}
            draw_text +: {
                text_style: SMALL_STATE_TEXT_STYLE {}
                color: (SMALL_STATE_TEXT_COLOR)
            }
            text: ""
        }

        spacer := View {
            width: Fill
            height: Fit
        }

        state_group_toggle_button := mod.widgets.SmallStateGroupToggleButton {
            width: Fit
            height: Fit
            margin: Inset{ left: 2.0, top: 1.0 }
            text: ""
        }
    }


    // The view used for each day divider in a room's timeline.
    // The date text is centered between two horizontal lines.
    mod.widgets.DateDivider = View {
        width: Fill,
        height: Fit,
        margin: Inset{top: 7.0, bottom: 7.0}
        flow: Right,
        padding: Inset{left: 7.0, right: 7.0},
        spacing: 0.0,
        align: Align{x: 0.5, y: 0.5} // center horizontally and vertically

        left_line := LineH { }

        date := Label {
            padding: Inset{left: 7.0, right: 7.0}
            draw_text +: {
                text_style: TEXT_SUB {},
                color: (COLOR_DIVIDER_DARK)
            }
            text: ""
        }

        right_line := LineH { }
    }

    // The view used for the divider indicating where the user's last-viewed message is.
    // This is implemented as a DateDivider with a different color and a fixed text label.
    mod.widgets.ReadMarker = mod.widgets.DateDivider {
        left_line := LineH {
            draw_bg.color: (mod.widgets.COLOR_READ_MARKER)
        }

        date := Label {
            draw_text.color: (mod.widgets.COLOR_READ_MARKER)
            text: ""
        }

        right_line := LineH {
            draw_bg.color: (mod.widgets.COLOR_READ_MARKER)
        }
    }


    // The top space is used to display a loading message while the room is being paginated.
    mod.widgets.TopSpace = SolidView {
        visible: false,
        width: Fill,
        height: Fit,
        align: Align{x: 0.5, y: 0}
        flow: Right,
        show_bg: true,
        draw_bg.color: #xDAF5E5F0, // mostly opaque light green

        label := Label {
            width: Fill,
            height: Fit,
            align: Align{x: 0.5, y: 0.5},
            flow: Right,
            padding: Inset{ top: 10.0, bottom: 7.0, left: 15.0, right: 15.0 }
            draw_text +: {
                text_style: MESSAGE_TEXT_STYLE { font_size: 10 },
                color: (TIMESTAMP_TEXT_COLOR)
            }
            text: ""
        }
    }

    mod.widgets.Timeline = View {
        width: Fill,
        height: Fill,
        align: Align{x: 0.5, y: 0.0} // center horizontally, align to top vertically
        flow: Overlay,

        list := PortalList {
            height: Fill,
            width: Fill
            flow: Down

            auto_tail: true, // set to `true` to lock the view to the last item.
            max_pull_down: 0.0, // set to `0.0` to disable the pulldown bounce animation.
            // TODO: enable `reuse_items: true` once Makepad's Html/TextFlow widget
            //   properly resets all internal state during `script_apply(Reload)`.
            //   Currently, stale TextFlow layout state (particularly related to
            //   list items) leaks through when a widget is recycled, causing
            //   excessive whitespace in HTML messages with `<ul>`/`<ol>` lists.

            // Below, we must place all of the possible templates (views) that can be used in the portal list.
            Message := mod.widgets.Message {}
            CondensedMessage := mod.widgets.CondensedMessage {}
            ImageMessage := mod.widgets.ImageMessage {}
            CondensedImageMessage := mod.widgets.CondensedImageMessage {}
            VideoMessage := mod.widgets.VideoMessage {}
            AudioMessage := mod.widgets.AudioMessage {}
            StickerMessage := mod.widgets.StickerMessage {}
            CondensedStickerMessage := mod.widgets.CondensedStickerMessage {}
            SmallStateEvent := mod.widgets.SmallStateEvent {}
            SmallStateEventsSummary := mod.widgets.SmallStateEventsSummary {}
            Empty := mod.widgets.Empty {}
            EncryptionNotice := mod.widgets.EncryptionNotice {}
            DateDivider := mod.widgets.DateDivider {}
            ReadMarker := mod.widgets.ReadMarker {}
            AppServicePanel := mod.widgets.AppServicePanel {}
        }

        // A jump to bottom button (with an unread message badge) that is shown
        // when the timeline is not at the bottom.
        jump_to_bottom_button := JumpToBottomButton { }

        // Floating info button at the top-right, occupying the rightmost slot.
        // Clicking it opens the `room_info_sliding_pane` (desktop only).
        info_button := mod.widgets.InfoButton { }

        // Floating threads button at the top-right, sitting left of the search
        // and info buttons. Clicking it opens the `threads_sliding_pane`.
        threads_button := mod.widgets.ThreadsButton { }

        // Floating search button at the top-right (mirrors jump-to-bottom).
        // Clicking it opens the `search_messages_pane` sliding pane.
        // NOTE: the pane itself is NOT defined here — it lives as a top-level
        // overlay in `room_screen_wrapper` (next to `room_info_sliding_pane`)
        // so it composites OVER the timeline's `new_batch` cards. Defining it
        // inside this Timeline overlay let those cards z-fight through it.
        search_messages_button := mod.widgets.SearchMessagesButton { }
    }

    mod.widgets.TranslationLangPopupButton = RobrixIconButton {
        width: Fill
        height: 36
        spacing: 0
        margin: 0
        padding: Inset{left: 12, right: 12, top: 8, bottom: 8}
        icon_walk: Walk{width: 0, height: 0}
        draw_text +: {
            color: (COLOR_TEXT)
            color_hover: (COLOR_TEXT)
            color_down: (COLOR_TEXT)
            text_style: MESSAGE_TEXT_STYLE { font_size: 10.5 }
        }
        draw_bg +: {
            color: #0000
            color_hover: #xF0F4FA
            color_down: #xE8EEF8
            border_size: 0.0
            border_radius: 0.0
        }
    }

    mod.widgets.RoomScreen = #(RoomScreen::register_widget(vm)) {
        width: Fill, height: Fill,
        cursor: MouseCursor.Default,
        flow: Down,
        spacing: 0.0

        room_screen_wrapper := SolidView {
            width: Fill, height: Fill,
            flow: Overlay,

            show_bg: true
            draw_bg.color: (COLOR_PRIMARY_DARKER)

            restore_status_view := RestoreStatusView {}

            // Keyboard avoidance is already provided by the Window's built-in
            // KeyboardView (makepad `window.rs`: `body := KeyboardView`). Using a
            // second KeyboardView here nested inside it double-applies the
            // keyboard shift (content jumps / a big blank gap on Android), so
            // this is a plain View.
            keyboard_view := View {
                width: Fill, height: Fill,
                flow: Down,

                // Robrix-owned mobile room header + `Chat | Info` tab row.
                // Hidden on desktop (which uses its own dock chrome); shown and
                // populated on mobile from `draw_walk`.
                room_top_bar := mod.widgets.RoomTopBar {
                    visible: false,
                }

                // The Chat / Info bodies share the same space via an Overlay so
                // exactly one is shown at a time. (Two `height: Fill` siblings
                // in a Down flow are sized incorrectly when one is hidden — the
                // hidden one's Fill space can still be reserved, pushing the
                // visible one off-screen. An Overlay sizes BOTH children to the
                // full area, so toggling visibility just swaps which is drawn.)
                body_area := View {
                    width: Fill, height: Fill,
                    flow: Overlay,

                    // "Chat" tab body: the timeline, typing notice, and input bar.
                    chat_content := View {
                        width: Fill, height: Fill,
                        flow: Down,

                        // First, display the timeline of all messages/events.
                        timeline := mod.widgets.Timeline {
                            // margin: Inset{bottom: 10}
                        }

                        // Below that, display a typing notice when other users in the room are typing.
                        typing_notice := TypingNotice { }

                        room_input_bar := RoomInputBar {
                            // margin: Inset{top: 20}
                        }
                    }

                    // "Info" tab body: the existing room-info content reused
                    // inline (no slide animation / backdrop / close button).
                    //
                    // NOTE: the visibility toggle is on this PLAIN `View`
                    // wrapper — not on `info_content` directly — because
                    // `set_visible` is a no-op on a custom widget
                    // (`RoomInfoSlidingPane`); only plain Views honor it. The
                    // inner pane stays always-visible + inline.
                    info_tab_body := View {
                        width: Fill, height: Fill,
                        visible: false,

                        info_content := mod.widgets.RoomInfoSlidingPane {
                            // The base RoomInfoSlidingPane is `visible: false`
                            // (it's normally a hidden slide-in pane); override
                            // to always-visible here since the `info_tab_body`
                            // wrapper controls when this inline copy is shown.
                            visible: true,
                            inline: true,
                            bg_view +: { visible: false }
                            main_content +: {
                                width: Fill
                                header +: {
                                    padding: 0
                                    title +: { visible: false }
                                    close_button +: { visible: false }
                                }
                            }
                        }
                    }
                }
            }

            translation_lang_modal := Modal {
                align: Align{x: 0, y: 0}
                bg_view.draw_bg.color: #00000000
                content +: {
                    width: Fill
                    height: Fill
                    flow: Overlay
                    align: Align{x: 0, y: 0}

                    translation_lang_popup := RoundedView {
                        width: 220
                        height: Fit
                        margin: Inset{left: 0, top: 0}
                        padding: Inset{top: 4, bottom: 4}
                        show_bg: true
                        new_batch: true
                        draw_bg +: {
                            color: (COLOR_PRIMARY)
                            border_radius: 6.0
                            border_size: 1.0
                            border_color: #ddd
                            shadow_color: #0003
                            shadow_radius: 8.0
                            shadow_offset: vec2(0.0, 2.0)
                        }

                        translation_lang_scroll := ScrollYView {
                            width: Fill
                            height: 288
                            flow: Down
                            spacing: 0

                            lang_en := mod.widgets.TranslationLangPopupButton { text: "en  English" }
                            lang_zh := mod.widgets.TranslationLangPopupButton { text: "zh  简体中文" }
                            lang_zh_tw := mod.widgets.TranslationLangPopupButton { text: "zh-TW  繁體中文" }
                            lang_ja := mod.widgets.TranslationLangPopupButton { text: "ja  日本語" }
                            lang_ko := mod.widgets.TranslationLangPopupButton { text: "ko  한국어" }
                            lang_es := mod.widgets.TranslationLangPopupButton { text: "es  Español" }
                            lang_fr := mod.widgets.TranslationLangPopupButton { text: "fr  Français" }
                            lang_de := mod.widgets.TranslationLangPopupButton { text: "de  Deutsch" }
                            lang_ru := mod.widgets.TranslationLangPopupButton { text: "ru  Русский" }
                            lang_pt := mod.widgets.TranslationLangPopupButton { text: "pt  Português" }
                            lang_ar := mod.widgets.TranslationLangPopupButton { text: "ar  العربية" }
                            lang_vi := mod.widgets.TranslationLangPopupButton { text: "vi  Tiếng Việt" }
                            lang_th := mod.widgets.TranslationLangPopupButton { text: "th  ไทย" }
                            lang_id := mod.widgets.TranslationLangPopupButton { text: "id  Bahasa Indonesia" }
                            lang_ms := mod.widgets.TranslationLangPopupButton { text: "ms  Bahasa Melayu" }
                            lang_tr := mod.widgets.TranslationLangPopupButton { text: "tr  Türkçe" }
                            lang_hi := mod.widgets.TranslationLangPopupButton { text: "hi  हिन्दी" }
                        }
                    }
                }
            }

            // Note: here, we're within a View that has an Overlay flow,
            // so the order that we define the below views determines which one is on top.

            // The top space should be displayed as an overlay at the top of the timeline.
            top_space := mod.widgets.TopSpace { }

            threads_sliding_pane := mod.widgets.ThreadsSlidingPane { }
            room_info_sliding_pane := mod.widgets.RoomInfoSlidingPane { }
            // Right-sliding pane hosting the server-side message search. Lives
            // here (a top-level wrapper overlay), NOT inside the Timeline, so it
            // composites over the timeline's `new_batch` message cards — same
            // as `room_info_sliding_pane`, which has no z-order glitch.
            search_messages_pane := mod.widgets.SearchMessagesSlidingPane { }

            // The user profile sliding pane should be displayed on top of other "static" subviews
            // (on top of all other views that are always visible).
            user_profile_sliding_pane := mod.widgets.UserProfileSlidingPane { }

            // The loading pane appears while the user is waiting for something in the room screen
            // to finish loading, e.g., when loading an older replied-to message.
            loading_pane := LoadingPane { }

            create_bot_modal := Modal {
                content +: {
                    create_bot_modal_inner := mod.widgets.CreateBotModal {}
                }
            }

            delete_bot_modal := Modal {
                content +: {
                    delete_bot_modal_inner := mod.widgets.DeleteBotModal {}
                }
            }

            /*
             * TODO: add the action bar back in as a series of floating buttons.
             *
            message_action_bar_popup := PopupNotification {
                align: Align{x: 0.0, y: 0.0}
                content: {
                    height: Fit,
                    width: Fit,
                    show_bg: false,
                    align: Align{
                        x: 0.5,
                        y: 0.5
                    }

                    message_action_bar := MessageActionBar {}
                }
            }
            */
        }
    }
}
