//! The room info sliding pane (desktop overlay + mobile inline tab):
//! room identity, members list, bot/agent badge derivation, and the
//! toolbar info button.

use super::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // Floating circular button that opens the `RoomInfoSlidingPane` (desktop
    // only — mobile reaches room info via the RoomTopBar "Info" tab). Sits at
    // the rightmost slot (no right inset); the search (48px) and threads (96px)
    // buttons sit to its left.
    mod.widgets.InfoButton = #(InfoButton::register_widget(vm)) {
        width: Fill,
        height: Fill,
        flow: Overlay,
        align: Align{x: 1.0, y: 0.0},
        visible: true,

        View {
            width: 65, height: 65,
            align: Align{x: 0.5, y: 0.0},
            flow: Overlay,

            inner_button := RobrixIconButton {
                spacing: 0,
                width: 40, height: 40,
                align: Align{x: 0.5, y: 0.5},
                margin: Inset{top: 8},

                draw_icon +: {
                    svg: (ICON_INFO),
                    color: #555,
                }
                icon_walk: Walk{width: 18, height: 18}

                draw_bg +: {
                    background_color: #edededce,
                    background_color_hover: #d0d0d0ce,
                    pixel: fn() {
                        let sdf = Sdf2d.viewport(self.pos * self.rect_size);
                        let c = self.rect_size * 0.5;
                        sdf.circle(c.x, c.x, c.x);
                        sdf.fill_keep(mix(self.background_color, self.background_color_hover, self.hover));
                        return sdf.result
                    }
                }
            }
        }
    }

    mod.widgets.RoomInfoPeopleEntry = #(RoomInfoPeopleEntry::register_widget(vm)) {
        width: Fill
        height: Fit
        flow: Down
        cursor: MouseCursor.Hand

        row := View {
            width: Fill
            height: Fit
            flow: Right
            align: Align{y: 0.5}
            spacing: 12
            padding: Inset{left: 14, right: 14, top: 11, bottom: 11}

            avatar := Avatar {
                width: 38
                height: 38
            }

            name_wrap := View {
                width: Fill
                height: Fit
                flow: Flow.Right{wrap: true}
                align: Align{y: 0.5}

                display_name := Label {
                    width: Fit
                    height: Fit
                    flow: Flow.Right{wrap: true}
                    // Gap to the badge lives here (not as container `spacing`)
                    // so a wrapped bot_badge starts flush at the row's left
                    // edge instead of inheriting an extra leading offset from
                    // the wrap-flow container's spacing bookkeeping.
                    margin: Inset{right: 6}
                    draw_text +: {
                        text_style: RBX_TEXT_BODY_STRONG {}
                        color: (RBX_FG_PRIMARY)
                    }
                    text: ""
                }

                // Shown only for bot members (in place of the old " [bot]" suffix).
                bot_badge := RoundedView {
                    visible: false
                    width: Fit
                    height: Fit
                    align: Align{x: 0.5, y: 0.5}
                    padding: Inset{left: 7, right: 7, top: 2, bottom: 2}
                    show_bg: true
                    draw_bg +: { color: (RBX_ACCENT_SOFT), border_radius: (RBX_RADIUS_PILL) }
                    Label {
                        width: Fit, height: Fit
                        draw_text +: { text_style: RBX_TEXT_BADGE {}, color: (RBX_ACCENT) }
                        text: "bot"
                    }
                }
            }

            // Role chip (Creator / Admin / Moderator) — hidden for plain members.
            level_chip := RoundedView {
                visible: false
                width: Fit
                height: Fit
                align: Align{x: 0.5, y: 0.5}
                padding: Inset{left: 9, right: 9, top: 3, bottom: 3}
                show_bg: true
                draw_bg +: { color: (RBX_INFO_BG), border_radius: (RBX_RADIUS_PILL) }
                level := Label {
                    width: Fit
                    height: Fit
                    draw_text +: { text_style: RBX_TEXT_BADGE {}, color: (RBX_INFO_FG) }
                    text: ""
                }
            }
        }

        member_divider := RoundedView {
            width: Fill
            height: 1.0
            margin: Inset{left: 64}
            show_bg: true
            draw_bg +: { color: (RBX_STROKE_SOFT) }
        }
    }

    mod.widgets.RoomInfoSlidingPane = #(RoomInfoSlidingPane::register_widget(vm)) {
        visible: false,
        flow: Overlay,
        width: Fill,
        height: Fill,
        align: Align{x: 1.0, y: 0}

        bg_view := SolidView {
            width: Fill
            height: Fill
            visible: false,
            show_bg: true
            draw_bg.color: (RBX_SCRIM)
        }

        main_content := SolidView {
            width: 320,
            height: Fill
            flow: Down,
            align: Align{x: 1.0}

            show_bg: true,
            // Cool off-white page so the white cards below read as distinct
            // surfaces (the grouped-list look from the spec).
            draw_bg.color: (RBX_BG_CANVAS)

            header := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}
                padding: Inset{top: 12, right: 10, bottom: 12, left: 15}

                back_button := RobrixNeutralIconButton {
                    visible: false
                    width: Fit,
                    height: Fit,
                    spacing: 0,
                    // Left inset matches the room top bar's back icon (header pad
                    // 6 + button pad 6 = 12) so the two arrows line up vertically.
                    padding: Inset{left: 12, right: 3, top: 8, bottom: 8}
                    draw_bg +: {
                        color: #0000
                        color_hover: (RBX_BG_HOVER)
                        color_down: (RBX_BG_PRESSED)
                        border_size: 0.0
                        border_color: #0000
                        border_color_hover: #0000
                        border_color_down: #0000
                        border_radius: (RBX_RADIUS_XS)
                    }
                    draw_icon +: { svg: (ICON_JUMP), color: (RBX_FG_SECONDARY) }
                    icon_walk: Walk{width: 16, height: 16}
                    text: ""
                }

                // People sub-page count, shown inline with the back button so the
                // two are horizontally aligned (also visible in the inline tab,
                // where `title` is hidden).
                members_header_count := Label {
                    visible: false
                    width: Fit
                    height: Fit
                    margin: Inset{left: 0}
                    draw_text +: {
                        text_style: RBX_TEXT_SECTION_TITLE {}
                        color: (RBX_FG_PRIMARY)
                    }
                    text: ""
                }

                title := Label {
                    width: Fit
                    height: Fit
                    draw_text +: {
                        text_style: USERNAME_TEXT_STYLE { font_size: 12.5 }
                        color: #000
                    }
                    text: "Info"
                }

                spacer := View {
                    width: Fill
                    height: Fit
                }

                close_button := RobrixNeutralIconButton {
                    width: Fit,
                    height: Fit,
                    spacing: 0,
                    padding: 15,
                    draw_icon.svg: (ICON_CLOSE)
                    icon_walk: Walk{width: 14, height: 14}
                    text: ""
                }
            }

            content_scroll := ScrollYView {
                width: Fill
                height: Fill
                flow: Down

                info_view := View {
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 12
                    padding: Inset{left: 12, right: 12, top: 12, bottom: 12}

                    // ===== Hero / summary card =====
                    // Hero sits directly on the page (no card chrome) per spec.
                    summary_card := View {
                        width: Fill
                        height: Fit
                        flow: Down
                        spacing: 12
                        padding: Inset{left: 14, right: 14, top: 4, bottom: 4}

                        hero_row := View {
                            width: Fill
                            height: Fit
                            flow: Right
                            spacing: 13
                            align: Align{y: 0.0}

                            room_avatar := Avatar {
                                width: 56
                                height: 56
                            }

                            room_meta := View {
                                width: Fill
                                height: Fit
                                flow: Down
                                spacing: 2

                                name_row := View {
                                    width: Fill
                                    height: Fit
                                    flow: Right
                                    align: Align{y: 0.5}
                                    spacing: 6

                                    // Room name + bot pill live together in a Fill
                                    // sub-row so the pill trails the (capped,
                                    // ellipsized) name text directly, the same way
                                    // the rooms list packs its bot pill snug against
                                    // the name instead of at the row's far edge.
                                    // This sub-row keeps the same Fill role that
                                    // room_name_value used to play here, so
                                    // favorite_button's pinned-right position is
                                    // unaffected.
                                    title_wrap := View {
                                        width: Fill
                                        height: Fit
                                        flow: Right
                                        align: Align{y: 0.5}
                                        spacing: 6

                                        room_name_value := Label {
                                            width: Fit{max: FitBound.Rel{base: Base.Full, factor: 0.82}}
                                            height: Fit
                                            flow: Flow.Right{wrap: false}
                                            max_lines: 1
                                            text_overflow: Ellipsis
                                            draw_text +: {
                                                text_style: RBX_TEXT_SECTION_TITLE {}
                                                color: (RBX_FG_PRIMARY)
                                            }
                                            text: ""
                                        }

                                        // Bot indicator pill, styled to match the
                                        // rooms-list / timeline bot pill exactly.
                                        title_bot_pill := RoundedView {
                                            visible: false
                                            width: Fit
                                            height: 16.0
                                            align: Align{x: 0.5, y: 0.5}
                                            padding: Inset{left: 6.0, right: 6.0}
                                            show_bg: true
                                            new_batch: true
                                            draw_bg +: {
                                                color: (RBX_ACCENT_SOFT)
                                                border_radius: 3.0
                                            }
                                            Label {
                                                width: Fit, height: Fit, padding: 0
                                                draw_text +: {
                                                    text_style: REGULAR_TEXT { font_size: 8.5, top_drop: -0.08 }
                                                    color: (RBX_ACCENT)
                                                }
                                                text: "bot"
                                            }
                                        }
                                    }

                                    favorite_button := View {
                                        width: Fit
                                        height: Fit
                                        flow: Overlay
                                        align: Align{x: 0.5, y: 0.5}
                                        padding: 1
                                        cursor: MouseCursor.Hand

                                        star_outline := View {
                                            width: Fit
                                            height: Fit
                                            Icon {
                                                width: 17
                                                height: 17
                                                align: Align{x: 0.5, y: 0.5}
                                                draw_icon +: {
                                                    svg: (ICON_STAR)
                                                    color: (RBX_FG_TERTIARY)
                                                }
                                                icon_walk: Walk{width: 17, height: 17}
                                            }
                                        }

                                        star_filled := View {
                                            visible: false
                                            width: Fit
                                            height: Fit
                                            Icon {
                                                width: 17
                                                height: 17
                                                align: Align{x: 0.5, y: 0.5}
                                                draw_icon +: {
                                                    svg: (ICON_STAR_FILLED)
                                                    color: (RBX_WARNING_FG)
                                                }
                                                icon_walk: Walk{width: 17, height: 17}
                                            }
                                        }
                                    }
                                }

                                room_id_row := View {
                                    width: Fill
                                    height: Fit
                                    flow: Right
                                    align: Align{y: 0.5}
                                    spacing: 4

                                    room_id_value := Label {
                                        width: Fill
                                        height: Fit
                                        flow: Flow.Right{wrap: true}
                                        draw_text +: {
                                            text_style: RBX_TEXT_META {}
                                            color: (RBX_FG_TERTIARY)
                                        }
                                        text: ""
                                    }

                                    copy_room_id_button := RobrixNeutralIconButton {
                                        width: Fit
                                        height: Fit
                                        padding: 2
                                        spacing: 0
                                        margin: 0
                                        draw_bg +: {
                                            color: #00000000
                                            color_hover: #00000000
                                            color_down: #00000000
                                            border_size: 0.0
                                        }
                                        draw_icon +: { svg: (ICON_COPY), color: (RBX_FG_TERTIARY) }
                                        icon_walk: Walk{width: 13, height: 13}
                                        text: ""
                                    }
                                }

                                // visibility · members · encryption — left-aligned
                                // directly under the room id. Wraps to the next
                                // line when the meta area (constrained by the
                                // avatar on the left) is too narrow to fit all
                                // three on one row, so the encryption item is no
                                // longer clipped at the right edge.
                                meta_row := View {
                                    width: Fill
                                    height: Fit
                                    flow: Flow.Right{wrap: true}
                                    spacing: 12
                                    wrap_spacing: 6
                                    align: Align{y: 0.5}
                                    margin: Inset{top: 3}

                                    visibility_meta := View {
                                        width: Fit
                                        height: Fit
                                        flow: Right
                                        spacing: 4
                                        align: Align{y: 0.5}

                                        Icon {
                                            width: 13
                                            height: 13
                                            draw_icon +: { svg: (ICON_GLOBE), color: (RBX_FG_SECONDARY) }
                                            icon_walk: Walk{width: 13, height: 13}
                                        }
                                        visibility_value := Label {
                                            width: Fit
                                            height: Fit
                                            draw_text +: { text_style: RBX_TEXT_META {}, color: (RBX_FG_SECONDARY) }
                                            text: ""
                                        }
                                    }

                                    members_meta := View {
                                        width: Fit
                                        height: Fit
                                        flow: Right
                                        spacing: 4
                                        align: Align{y: 0.5}

                                        Icon {
                                            width: 13
                                            height: 13
                                            draw_icon +: { svg: (ICON_PEOPLE), color: (RBX_FG_SECONDARY) }
                                            icon_walk: Walk{width: 13, height: 13}
                                        }
                                        members_meta_value := Label {
                                            width: Fit
                                            height: Fit
                                            draw_text +: { text_style: RBX_TEXT_META {}, color: (RBX_FG_SECONDARY) }
                                            text: ""
                                        }
                                    }

                                    encryption_meta := View {
                                        width: Fit
                                        height: Fit
                                        flow: Right
                                        spacing: 4
                                        align: Align{y: 0.5}

                                        enc_icon_locked := View {
                                            width: Fit
                                            height: Fit
                                            Icon {
                                                width: 13
                                                height: 13
                                                draw_icon +: { svg: (ICON_LOCK), color: (RBX_SUCCESS_FG) }
                                                icon_walk: Walk{width: 13, height: 13}
                                            }
                                        }
                                        enc_icon_unlocked := View {
                                            visible: false
                                            width: Fit
                                            height: Fit
                                            Icon {
                                                width: 13
                                                height: 13
                                                draw_icon +: { svg: (ICON_LOCK_OPEN), color: (RBX_FG_TERTIARY) }
                                                icon_walk: Walk{width: 13, height: 13}
                                            }
                                        }
                                        encryption_value := Label {
                                            width: Fit
                                            height: Fit
                                            draw_text +: { text_style: RBX_TEXT_META {}, color: (RBX_FG_SECONDARY) }
                                            text: ""
                                        }
                                    }
                                }

                                badges_row := View {
                                    width: Fill
                                    height: Fit
                                    flow: Flow.Right{wrap: true}
                                    spacing: 8

                                    agent_badge_wrap := View {
                                        visible: false
                                        width: Fit
                                        height: Fit
                                        // Outdent by the pill's left padding so the
                                        // robot icon lines up with the meta-row icons
                                        // (globe / people) directly above.
                                        margin: Inset{left: -10}

                                        agent_badge := RoundedView {
                                            width: Fit
                                            height: Fit
                                            flow: Right
                                            spacing: 5
                                            align: Align{y: 0.5}
                                            padding: Inset{left: 10, right: 11, top: 5, bottom: 5}
                                            show_bg: true
                                            draw_bg +: {
                                                color: (RBX_ACCENT_SOFT)
                                                border_radius: (RBX_RADIUS_PILL)
                                            }

                                            Icon {
                                                width: 14
                                                height: 14
                                                draw_icon +: { svg: (ICON_ROBOT), color: (RBX_ACCENT) }
                                                icon_walk: Walk{width: 14, height: 14}
                                            }
                                            Label {
                                                width: Fit
                                                height: Fit
                                                draw_text +: { text_style: RBX_TEXT_BADGE {}, color: (RBX_ACCENT) }
                                                text: "Agent-enabled"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // ===== About / topic card =====
                    about_card := RoundedView {
                        width: Fill
                        height: Fit
                        flow: Right
                        spacing: 12
                        align: Align{y: 0.0}
                        padding: Inset{left: 14, right: 14, top: 13, bottom: 13}

                        show_bg: true
                        draw_bg +: {
                            color: (RBX_BG_SURFACE)
                            border_radius: (RBX_RADIUS_MD)
                            border_size: 1.0
                            border_color: (RBX_STROKE_SOFT)
                        }

                        about_icon_circle := CircleView {
                            width: 38
                            height: 38
                            align: Align{x: 0.5, y: 0.5}
                            show_bg: true
                            draw_bg +: { color: (RBX_ACCENT_SOFT) }
                            Icon {
                                width: 19
                                height: 19
                                draw_icon +: { svg: (ICON_FILE), color: (RBX_ACCENT) }
                                icon_walk: Walk{width: 19, height: 19}
                            }
                        }

                        about_col := View {
                            width: Fill
                            height: Fit
                            flow: Down
                            spacing: 2

                            about_title := Label {
                                width: Fill
                                height: Fit
                                draw_text +: { text_style: RBX_TEXT_CARD_TITLE {}, color: (RBX_FG_PRIMARY) }
                                text: "About"
                            }

                            topic_value := Label {
                                width: Fill
                                height: Fit
                                flow: Flow.Right{wrap: true}
                                draw_text +: { text_style: RBX_TEXT_META {}, color: (RBX_FG_SECONDARY) }
                                text: ""
                            }

                            topic_toggle_button := RobrixNeutralIconButton {
                                visible: false
                                width: Fit
                                height: 24
                                align: Align{x: 0.0, y: 0.5}
                                padding: Inset{left: 0, right: 0, top: 3, bottom: 0}
                                spacing: 0
                                icon_walk: Walk{width: 0, height: 0}
                                draw_bg +: {
                                    color: #00000000
                                    color_hover: #00000000
                                    color_down: #00000000
                                    border_size: 0.0
                                }
                                draw_text +: { color: (RBX_ACCENT), text_style: RBX_TEXT_META {} }
                                text: "Expand"
                            }
                        }
                    }

                    // ===== Members card =====
                    members_card := RoundedView {
                        width: Fill
                        height: Fit
                        flow: Down
                        spacing: 7
                        padding: Inset{left: 14, right: 14, top: 13, bottom: 13}

                        show_bg: true
                        draw_bg +: {
                            color: (RBX_BG_SURFACE)
                            border_radius: (RBX_RADIUS_MD)
                            border_size: 1.0
                            border_color: (RBX_STROKE_SOFT)
                        }

                        members_header := View {
                            width: Fill
                            height: Fit
                            flow: Right
                            spacing: 12
                            align: Align{y: 0.5}
                            cursor: MouseCursor.Hand

                            members_icon_circle := CircleView {
                                width: 38
                                height: 38
                                align: Align{x: 0.5, y: 0.5}
                                show_bg: true
                                draw_bg +: { color: (RBX_INFO_BG) }
                                Icon {
                                    width: 19
                                    height: 19
                                    draw_icon +: { svg: (ICON_PEOPLE), color: (RBX_INFO_FG) }
                                    icon_walk: Walk{width: 19, height: 19}
                                }
                            }

                            members_title := Label {
                                width: Fit
                                height: Fit
                                draw_text +: { text_style: RBX_TEXT_CARD_TITLE {}, color: (RBX_FG_PRIMARY) }
                                text: "Members"
                            }

                            members_header_spacer := View { width: Fill, height: Fit }

                            // count + chevron kept close together on the right edge
                            members_count_group := View {
                                width: Fit
                                height: Fit
                                flow: Right
                                spacing: 2
                                align: Align{y: 0.5}

                                members_count_value := Label {
                                    width: Fit
                                    height: Fit
                                    draw_text +: { text_style: RBX_TEXT_BODY_STRONG {}, color: (RBX_FG_SECONDARY) }
                                    text: ""
                                }

                                Icon {
                                    width: 14
                                    height: 14
                                    draw_icon +: { svg: (ICON_CHEVRON_RIGHT), color: (RBX_FG_TERTIARY) }
                                    icon_walk: Walk{width: 14, height: 14}
                                }
                            }
                        }

                        members_detail := View {
                            width: Fill
                            height: Fit
                            flow: Right
                            align: Align{y: 0.5}
                            spacing: 8
                            // Indent so the avatar row lines up under "Members"
                            // (icon width 38 + header spacing 12).
                            margin: Inset{left: 50}

                            members_stack := View {
                                width: Fit
                                height: Fit
                                flow: Right
                                align: Align{y: 0.5}

                                stack_slot_0 := View {
                                    width: Fit
                                    height: Fit
                                    ring_0 := RoundedView {
                                        width: Fit
                                        height: Fit
                                        padding: 2
                                        show_bg: true
                                        draw_bg +: { color: (RBX_BG_SURFACE), border_radius: 18.0 }
                                        stack_avatar_0 := Avatar { width: 32, height: 32 }
                                    }
                                }
                                stack_slot_1 := View {
                                    visible: false
                                    width: Fit
                                    height: Fit
                                    margin: Inset{left: -11}
                                    ring_1 := RoundedView {
                                        width: Fit
                                        height: Fit
                                        padding: 2
                                        show_bg: true
                                        draw_bg +: { color: (RBX_BG_SURFACE), border_radius: 18.0 }
                                        stack_avatar_1 := Avatar { width: 32, height: 32 }
                                    }
                                }
                                stack_slot_2 := View {
                                    visible: false
                                    width: Fit
                                    height: Fit
                                    margin: Inset{left: -11}
                                    ring_2 := RoundedView {
                                        width: Fit
                                        height: Fit
                                        padding: 2
                                        show_bg: true
                                        draw_bg +: { color: (RBX_BG_SURFACE), border_radius: 18.0 }
                                        stack_avatar_2 := Avatar { width: 32, height: 32 }
                                    }
                                }
                                stack_more_wrap := View {
                                    visible: false
                                    width: Fit
                                    height: Fit
                                    margin: Inset{left: 1}
                                    stack_more_chip := RoundedView {
                                        width: Fit
                                        height: Fit
                                        align: Align{x: 0.5, y: 0.5}
                                        padding: Inset{left: 7, right: 7, top: 5, bottom: 5}
                                        show_bg: true
                                        draw_bg +: { color: (RBX_NEUTRAL_BG), border_radius: (RBX_RADIUS_PILL) }
                                        stack_more := Label {
                                            width: Fit
                                            height: Fit
                                            draw_text +: { text_style: RBX_TEXT_BADGE {}, color: (RBX_FG_SECONDARY) }
                                            text: ""
                                        }
                                    }
                                }
                            }

                            members_detail_spacer := View { width: Fill, height: Fit }

                            my_role_wrap := View {
                                visible: false
                                width: Fit
                                height: Fit

                                my_role_chip := RoundedView {
                                    width: Fit
                                    height: Fit
                                    align: Align{x: 0.5, y: 0.5}
                                    padding: Inset{left: 10, right: 10, top: 4, bottom: 4}
                                    show_bg: true
                                    draw_bg +: {
                                        color: (RBX_ACCENT_SOFT)
                                        border_radius: (RBX_RADIUS_PILL)
                                    }
                                    my_role_label := Label {
                                        width: Fit
                                        height: Fit
                                        draw_text +: { text_style: RBX_TEXT_BADGE {}, color: (RBX_ACCENT) }
                                        text: ""
                                    }
                                }
                            }
                        }
                    }

                    // ===== Actions =====
                    actions_row := View {
                        width: Fill
                        height: Fit
                        flow: Down
                        spacing: 8

                        // Primary actions grouped into ONE card (rows + hairline
                        // divider) so they read as a settings list, not three
                        // floating boxes. Rows are transparent; the card paints
                        // the single border + radius.
                        actions_card := RoundedView {
                            width: Fill
                            height: Fit
                            flow: Down
                            clip_x: true
                            clip_y: true
                            show_bg: true
                            draw_bg +: {
                                color: (RBX_BG_SURFACE)
                                border_radius: (RBX_RADIUS_MD)
                                border_size: 1.0
                                border_color: (RBX_STROKE_SOFT)
                            }

                            invite_button := RobrixNeutralIconButton {
                                width: Fill
                                height: 50
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 13
                                padding: Inset{left: 16, right: 14, top: 0, bottom: 0}
                                draw_bg +: {
                                    color: #00000000
                                    color_hover: (RBX_BG_HOVER)
                                    color_down: (RBX_BG_PRESSED)
                                    border_size: 0.0
                                    border_radius: 0.0
                                    border_color: #0000
                                    border_color_hover: #0000
                                    border_color_down: #0000
                                }
                                draw_icon +: { svg: (ICON_ADD_USER), color: (RBX_FG_SECONDARY) }
                                icon_walk: Walk{width: 20, height: 20}
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    color_hover: (RBX_FG_PRIMARY)
                                    color_down: (RBX_FG_PRIMARY)
                                    text_style: RBX_TEXT_BODY {}
                                }
                                text: "Invite"
                            }

                            action_divider := RoundedView {
                                width: Fill
                                height: 1.0
                                margin: Inset{left: 49}
                                show_bg: true
                                draw_bg +: { color: (RBX_STROKE_SOFT) }
                            }

                            report_room_button := RobrixNeutralIconButton {
                                width: Fill
                                height: 50
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 13
                                padding: Inset{left: 16, right: 14, top: 0, bottom: 0}
                                draw_bg +: {
                                    color: #00000000
                                    color_hover: (RBX_BG_HOVER)
                                    color_down: (RBX_BG_PRESSED)
                                    border_size: 0.0
                                    border_radius: 0.0
                                    border_color: #0000
                                    border_color_hover: #0000
                                    border_color_down: #0000
                                }
                                draw_icon +: { svg: (ICON_INFO), color: (RBX_FG_SECONDARY) }
                                icon_walk: Walk{width: 20, height: 20}
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    color_hover: (RBX_FG_PRIMARY)
                                    color_down: (RBX_FG_PRIMARY)
                                    text_style: RBX_TEXT_BODY {}
                                }
                                text: "Report room"
                            }
                        }

                        // Destructive action — its own card, separated by the gap,
                        // red icon + label.
                        leave_card := RoundedView {
                            width: Fill
                            height: Fit
                            flow: Down
                            clip_x: true
                            clip_y: true
                            show_bg: true
                            draw_bg +: {
                                color: (RBX_BG_SURFACE)
                                border_radius: (RBX_RADIUS_MD)
                                border_size: 1.0
                                border_color: (RBX_STROKE_SOFT)
                            }

                            leave_room_button := RobrixNegativeIconButton {
                                width: Fill
                                height: 50
                                align: Align{x: 0.0, y: 0.5}
                                spacing: 13
                                padding: Inset{left: 16, right: 14, top: 0, bottom: 0}
                                draw_bg +: {
                                    color: #00000000
                                    color_hover: (RBX_DANGER_BG)
                                    color_down: (RBX_DANGER_BG)
                                    border_size: 0.0
                                    border_radius: 0.0
                                    border_color: #0000
                                    border_color_hover: #0000
                                    border_color_down: #0000
                                }
                                draw_icon +: { svg: (ICON_CLOSE), color: (RBX_DANGER_FG) }
                                icon_walk: Walk{width: 18, height: 18}
                                draw_text +: {
                                    color: (RBX_DANGER_FG)
                                    color_hover: (RBX_DANGER_FG)
                                    color_down: (RBX_DANGER_FG)
                                    text_style: RBX_TEXT_BODY {}
                                }
                                text: "Leave Room"
                            }
                        }
                    }
                }

            }

            people_view := View {
                visible: false
                width: Fill
                height: Fill
                flow: Down
                spacing: 10
                padding: Inset{left: 12, right: 12, top: 12, bottom: 12}

                loading_label := Label {
                    visible: false
                    width: Fill
                    height: Fit
                    draw_text +: {
                        text_style: RBX_TEXT_BODY {}
                        color: (RBX_FG_SECONDARY)
                    }
                    text: "Loading members..."
                }

                empty_label := Label {
                    visible: false
                    width: Fill
                    height: Fit
                    draw_text +: {
                        text_style: RBX_TEXT_BODY {}
                        color: (RBX_FG_SECONDARY)
                    }
                    text: "No members found."
                }

                // The member rows grouped into one card (rows + hairline dividers).
                members_list_card := RoundedView {
                    width: Fill
                    height: Fill
                    flow: Down
                    clip_x: true
                    clip_y: true
                    show_bg: true
                    draw_bg +: {
                        color: (RBX_BG_SURFACE)
                        border_radius: (RBX_RADIUS_MD)
                        border_size: 1.0
                        border_color: (RBX_STROKE_SOFT)
                    }

                    people_list := PortalList {
                        width: Fill
                        height: Fill
                        flow: Down
                        max_pull_down: 0.0

                        PersonEntry := mod.widgets.RoomInfoPeopleEntry {}
                    }
                }
            }
        }

        slide: 1.0,

        animator: Animator {
            panel: {
                default: @hide
                show: AnimatorState{
                    redraw: true,
                    from: {all: Forward {duration: 0.5}}
                    ease: Ease.ExpDecay {d1: 0.80, d2: 0.97}
                    apply: {
                        slide: 0.0
                    }
                }
                hide: AnimatorState{
                    redraw: true,
                    from: {all: Forward {duration: 0.5}}
                    ease: Ease.ExpDecay {d1: 0.80, d2: 0.97}
                    apply: {
                        slide: 1.0
                    }
                }
            }
        }
    }
}

thread_local! {
    static ROOM_INFO_ACTION_MODAL_OPEN: Cell<bool> = const { Cell::new(false) };
}

/// Set by app.rs each frame from the GLOBAL room-info-action modals
/// (report / leave-confirm), so the room info sliding pane knows not to
/// self-close on Escape / tap-outside while one of them is open over it.
pub fn set_room_info_action_modal_open(open: bool) {
    ROOM_INFO_ACTION_MODAL_OPEN.with(|state| state.set(open));
}

pub(super) fn is_room_info_action_modal_open() -> bool {
    ROOM_INFO_ACTION_MODAL_OPEN.with(|state| state.get())
}

#[derive(Clone, Default, Debug)]
pub enum InfoButtonAction {
    OpenRequested,
    #[default]
    None,
}

impl ActionDefaultRef for InfoButtonAction {
    fn default_ref() -> &'static Self {
        static DEFAULT: InfoButtonAction = InfoButtonAction::None;
        &DEFAULT
    }
}

#[derive(Clone, Default, Debug)]
pub enum RoomInfoPaneAction {
    InviteUser,
    ShowPeoplePage,
    /// Emitted ONLY by a `RoomInfoPeopleEntry` row when its person is tapped.
    /// The owning `RoomInfoSlidingPane` instance re-bubbles this as
    /// `OpenPeopleProfile` (tagged with the pane's own widget uid) so the
    /// `RoomScreen` handler can pick it up. Kept distinct from
    /// `OpenPeopleProfile` so that pane instances never react to each other's
    /// re-emitted action — otherwise the desktop overlay pane and the inline
    /// (mobile) pane, which both receive every broadcast `Event::Actions`,
    /// would ping-pong the action between themselves forever and freeze the app.
    PersonClicked(OwnedUserId),
    OpenPeopleProfile(OwnedUserId),
    ReportRoom,
    LeaveRoom,
    #[default]
    None,
}

impl ActionDefaultRef for RoomInfoPaneAction {
    fn default_ref() -> &'static Self {
        static DEFAULT: RoomInfoPaneAction = RoomInfoPaneAction::None;
        &DEFAULT
    }
}

#[derive(Clone, Debug)]
pub(super) struct RoomInfoPaneInfo {
    pub(super) room_name: String,
    pub(super) room_id: String,
    /// Owned room id kept alongside the display string so the info pane can
    /// issue room-scoped requests (e.g. toggling the favourite tag).
    pub(super) owned_room_id: OwnedRoomId,
    pub(super) topic: String,
    /// Short visibility label: "Public" / "Private" / "Unknown".
    pub(super) visibility: String,
    /// Short encryption label: "Encrypted" / "Unencrypted" / "Unknown".
    pub(super) encryption: String,
    pub(super) is_encrypted: bool,
    /// Whether the room carries the `m.favourite` tag for the current user.
    pub(super) is_favorite: bool,
    /// Whether this room has a bot/agent participating (any member detected as a
    /// bot via `is_likely_bot_member`).
    pub(super) is_agent_enabled: bool,
    /// Whether the compact "bot" pill should trail the room title. This follows
    /// the same user-facing badge semantics as the rooms list: room binding or a
    /// registered-agent DM, not merely a bot-looking member in an unbound room.
    pub(super) show_title_bot_pill: bool,
    pub(super) member_count: usize,
    /// The current user's role in this room: "Owner" / "Admin" / "Moderator" /
    /// "Member", or empty if members haven't loaded yet.
    pub(super) my_role: String,
    pub(super) room_avatar_uri: Option<OwnedMxcUri>,
    pub(super) room_avatar_fallback_text: String,
    /// The sorted member rows, shared via `Arc` so the (potentially huge) list is
    /// built/sorted at most once per member-list change and cheaply reference-
    /// counted into every subsequent Signal-driven refresh.
    pub(super) people_entries: Arc<Vec<RoomInfoPeopleEntryInfo>>,
    pub(super) people_count_text: String,
    pub(super) show_people_loading: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct RoomInfoBotIdentityFingerprint {
    pub(super) resolved_parent_bot_user_id: Option<OwnedUserId>,
    pub(super) known_bot_user_ids: Vec<OwnedUserId>,
}

pub(super) fn room_info_bot_identity_fingerprint(
    app_state: Option<&AppState>,
    my_user_id: Option<&UserId>,
) -> RoomInfoBotIdentityFingerprint {
    app_state
        .map(|app_state| {
            let resolved_parent_bot_user_id = if app_state.bot_settings.enabled {
                app_state
                    .bot_settings
                    .resolved_bot_user_id(my_user_id)
                    .ok()
            } else {
                None
            };
            RoomInfoBotIdentityFingerprint {
                resolved_parent_bot_user_id,
                known_bot_user_ids: timeline_known_bot_user_ids(app_state),
            }
        })
        .unwrap_or(RoomInfoBotIdentityFingerprint {
            resolved_parent_bot_user_id: None,
            known_bot_user_ids: Vec::new(),
        })
}

/// Delegates to the rooms-list predicate (`room_shows_agent_badge`) so the
/// Info-pane title pill and the rooms-list row pill always agree.
pub(super) fn room_info_title_shows_agent_badge<'a>(
    app_state: Option<&AppState>,
    room_id: &RoomId,
    dm_target: Option<&UserId>,
    member_user_ids: impl IntoIterator<Item = &'a UserId>,
) -> bool {
    app_state.is_some_and(|app_state|
        room_shows_agent_badge(app_state, room_id, dm_target, member_user_ids)
    )
}

pub(super) fn room_info_dm_target_from_user_ids<'a>(
    user_ids: impl IntoIterator<Item = &'a UserId>,
    my_user_id: Option<&UserId>,
) -> Option<OwnedUserId> {
    let mut dm_target = None;
    for user_id in user_ids {
        if my_user_id.is_some_and(|my_user_id| my_user_id == user_id) {
            continue;
        }
        if dm_target.is_some() {
            return None;
        }
        dm_target = Some(user_id.to_owned());
    }
    dm_target
}

/// Cache for the expensive member-row build. Keyed by the room and the identity
/// (`Arc` pointer) of `TimelineUiState::room_members`, which is replaced wholesale
/// whenever the member list changes — so a pointer match means "members unchanged,
/// reuse the prebuilt rows" and we skip rebuilding + re-sorting all members on
/// every sync Signal (critical for very large rooms). Bot identity context is part
/// of the key because registry / app-service updates can change row bot markers
/// without changing the room member list.
pub(super) struct RoomInfoMembersCache {
    pub(super) room_id: OwnedRoomId,
    /// The exact `room_members` `Arc` the cached rows were built from. Held so the
    /// allocation can't be freed and its address reused (an ABA false-hit), and so
    /// validity is a cheap `Arc::ptr_eq` against the current `room_members`.
    pub(super) members: Arc<Vec<RoomMember>>,
    pub(super) bot_identity: RoomInfoBotIdentityFingerprint,
    pub(super) entries: Arc<Vec<RoomInfoPeopleEntryInfo>>,
    pub(super) is_agent_enabled: bool,
    pub(super) my_role: String,
}

#[derive(Clone, Debug)]
pub(super) struct RoomInfoPeopleEntryInfo {
    pub(super) user_id: OwnedUserId,
    pub(super) display_name: String,
    pub(super) level: String,
    pub(super) is_bot: bool,
    pub(super) avatar_uri: Option<OwnedMxcUri>,
    pub(super) avatar_fallback_text: String,
}

#[derive(Script, ScriptHook, Widget)]
pub struct RoomInfoPeopleEntry {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,

    #[rust] user_id: Option<OwnedUserId>,
}

impl Widget for RoomInfoPeopleEntry {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        let Some(user_id) = self.user_id.clone() else { return };
        match event.hits(cx, self.view.area()) {
            Hit::FingerUp(fe) if fe.is_over && fe.is_primary_hit() && fe.was_tap() => {
                cx.widget_action(
                    self.widget_uid(),
                    RoomInfoPaneAction::PersonClicked(user_id),
                );
            }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl RoomInfoPeopleEntry {
    fn set_entry(&mut self, cx: &mut Cx, entry: &RoomInfoPeopleEntryInfo) {
        self.user_id = Some(entry.user_id.clone());
        self.label(cx, ids!(display_name)).set_text(cx, &entry.display_name);
        self.view(cx, ids!(bot_badge)).set_visible(cx, entry.is_bot);
        self.label(cx, ids!(level)).set_text(cx, &entry.level);
        self.view(cx, ids!(level_chip)).set_visible(cx, !entry.level.is_empty());

        let avatar = self.avatar(cx, ids!(avatar));
        if let Some(uri) = entry.avatar_uri.as_ref()
            && let avatar_cache::AvatarCacheEntry::Loaded(image_data) = avatar_cache::get_or_fetch_avatar(cx, uri)
        {
            let res = avatar.show_image(
                cx,
                None,
                |cx, img_ref| utils::load_png_or_jpg(&img_ref, cx, &image_data),
            );
            if res.is_err() {
                avatar.show_text(cx, None, None, &entry.avatar_fallback_text);
            }
        } else {
            avatar.show_text(cx, None, None, &entry.avatar_fallback_text);
        }
    }
}

impl RoomInfoPeopleEntryRef {
    fn set_entry(&self, cx: &mut Cx, entry: &RoomInfoPeopleEntryInfo) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_entry(cx, entry);
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct InfoButton {
    #[deref] view: View,
}

impl Widget for InfoButton {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let button_area = self.button(cx, ids!(inner_button)).area();
        match event.hits(cx, button_area) {
            Hit::FingerHoverIn(_) | Hit::FingerLongPress(_) => {
                cx.widget_action(
                    self.widget_uid(),
                    TooltipAction::HoverIn {
                        text: String::from("Room Info"),
                        widget_rect: button_area.rect(cx),
                        options: CalloutTooltipOptions {
                            position: TooltipPosition::Left,
                            ..Default::default()
                        },
                    },
                );
            }
            Hit::FingerHoverOut(_) => {
                cx.widget_action(self.widget_uid(), TooltipAction::HoverOut);
            }
            _ => {}
        }

        self.view.handle_event(cx, event, scope);

        if let Event::Actions(actions) = event {
            if self.button(cx, ids!(inner_button)).clicked(actions) {
                cx.widget_action(self.widget_uid(), InfoButtonAction::OpenRequested);
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

#[derive(Script, ScriptHook, Widget, Animator)]
pub struct RoomInfoSlidingPane {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    #[apply_default] animator: Animator,
    #[live] slide: f32,

    /// When `true`, this pane is mounted *inline* (e.g. as the room's "Info"
    /// body tab) rather than as a right-sliding overlay: the slide animation,
    /// dimmed backdrop, and tap-outside/back-to-close behavior are all skipped,
    /// and its visibility is controlled entirely by the parent (the tab).
    #[live] inline: bool,

    #[rust] info: Option<RoomInfoPaneInfo>,
    #[rust] is_animating_out: bool,
    #[rust] show_people_page: bool,
    #[rust] topic_expanded: bool,
    #[rust] people_display_count: usize,
    /// Optimistic favourite override `(room_id, is_favorite)` set when the user
    /// taps the star. `room.is_favourite()` keeps returning the old value until
    /// the async tag write syncs back, so this survives the frequent
    /// Signal-driven `set_info` rebuilds and is cleared once the room agrees.
    #[rust] pending_favorite: Option<(OwnedRoomId, bool)>,
}

/// Populate an `Avatar` with the member/room image if it's cached, otherwise
/// fall back to the text initials. Shared by the room hero avatar and the
/// members-card avatar stack.
pub(super) fn show_avatar_or_text(
    cx: &mut Cx,
    avatar: &AvatarRef,
    avatar_uri: Option<&OwnedMxcUri>,
    fallback_text: &str,
) {
    if let Some(uri) = avatar_uri
        && let avatar_cache::AvatarCacheEntry::Loaded(image_data) = avatar_cache::get_or_fetch_avatar(cx, uri)
    {
        let res = avatar.show_image(
            cx,
            None,
            |cx, img_ref| utils::load_png_or_jpg(&img_ref, cx, &image_data),
        );
        if res.is_err() {
            avatar.show_text(cx, None, None, fallback_text);
        }
    } else {
        avatar.show_text(cx, None, None, fallback_text);
    }
}

impl Widget for RoomInfoSlidingPane {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        if !self.visible { return; }

        let animator_action = self.animator_handle_event(cx, event);
        if animator_action.must_redraw() {
            self.redraw(cx);
        }

        if self.is_animating_out && !self.animator.is_track_animating(id!(panel)) {
            self.visible = false;
            self.is_animating_out = false;
            cx.revert_key_focus();
            self.view(cx, ids!(bg_view)).set_visible(cx, false);
            self.redraw(cx);
            return;
        }

        // Tap on the favourite star (in the hero) toggles the room's favourite
        // tag. The star is a plain View (not a Button), so hit-test its area
        // directly — this works in both inline and overlay modes.
        let favorite_area = self.view(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.name_row.favorite_button)).area();
        if let Hit::FingerUp(fe) = event.hits(cx, favorite_area)
            && fe.is_over && fe.is_primary_hit() && fe.was_tap()
        {
            // Flip the cached value first (ends the `self.info` borrow), then
            // record the optimistic override + fire the async tag write.
            let toggled = self.info.as_mut().map(|info| {
                let new_is_favorite = !info.is_favorite;
                info.is_favorite = new_is_favorite;
                (info.owned_room_id.clone(), new_is_favorite)
            });
            if let Some((owned_room_id, new_is_favorite)) = toggled {
                self.pending_favorite = Some((owned_room_id.clone(), new_is_favorite));
                submit_async_request(MatrixRequest::SetIsFavorite {
                    room_id: owned_room_id,
                    is_favorite: new_is_favorite,
                });
                self.redraw(cx);
            }
        }

        // Tap anywhere on the members card header opens the People sub-page.
        let members_header_area = self.view(cx, ids!(content_scroll.info_view.members_card.members_header)).area();
        if let Hit::FingerUp(fe) = event.hits(cx, members_header_area)
            && fe.is_over && fe.is_primary_hit() && fe.was_tap()
        {
            self.show_people_page = true;
            self.people_display_count = self.info.as_ref()
                .map(|info| info.people_entries.len().min(40))
                .unwrap_or(0);
            // Always open the People list scrolled to the top (the PortalList
            // otherwise keeps its previous scroll offset).
            self.portal_list(cx, ids!(people_view.members_list_card.people_list))
                .set_first_id_and_scroll(0, 0.0);
            cx.widget_action(
                self.widget_uid(),
                RoomInfoPaneAction::ShowPeoplePage,
            );
            self.redraw(cx);
        }

        // Inline (tab) mode is opened/closed by the parent tab, so it never
        // self-closes on back-press / tap-outside / Escape.
        if !self.inline {
            let area = self.view.area();
            let close_pane = if is_invite_modal_open() || is_room_info_action_modal_open() {
                matches!(
                    event,
                    Event::Actions(actions) if self.button(cx, ids!(close_button)).clicked(actions)
                )
            } else {
                matches!(
                    event,
                    Event::Actions(actions) if self.button(cx, ids!(close_button)).clicked(actions)
                )
                || event.back_pressed()
                || match event.hits_with_capture_overload(cx, area, true) {
                    Hit::KeyUp(key) => key.key_code == KeyCode::Escape,
                    Hit::FingerDown(_fde) => {
                        cx.set_key_focus(area);
                        false
                    }
                    Hit::FingerUp(fue) if fue.is_over => {
                        fue.mouse_button().is_some_and(|b| b.is_back())
                        || !self.view(cx, ids!(main_content)).area().rect(cx).contains(fue.abs)
                    }
                    _ => false,
                }
            };
            if close_pane {
                self.hide(cx);
            }
        }

        if let Event::Actions(actions) = event {
            // Re-bubble a person tap from one of OUR people rows as
            // `OpenPeopleProfile` tagged with this pane's own widget uid, so the
            // `RoomScreen` handler (which filters by pane uid) can pick it up.
            //
            // Gate on `show_people_page` so ONLY the instance currently showing
            // its People page bubbles. Both the desktop overlay pane and the
            // inline (mobile) pane receive every broadcast `Event::Actions`, and
            // `PersonClicked` is deliberately a distinct variant that no pane
            // ever emits — together this guarantees the two instances can never
            // ping-pong the action between themselves (which froze the app).
            if self.show_people_page {
                for action in actions {
                    if let RoomInfoPaneAction::PersonClicked(user_id) = action.as_widget_action().cast() {
                        cx.widget_action(
                            self.widget_uid(),
                            RoomInfoPaneAction::OpenPeopleProfile(user_id.clone()),
                        );
                        break;
                    }
                }
            }

            if self.button(cx, ids!(header.back_button)).clicked(actions) {
                self.show_people_page = false;
                self.redraw(cx);
            }
            if self.button(cx, ids!(content_scroll.info_view.about_card.about_col.topic_toggle_button)).clicked(actions) {
                self.topic_expanded = !self.topic_expanded;
                self.redraw(cx);
            }
            if self.button(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.room_id_row.copy_room_id_button)).clicked(actions)
                && let Some(info) = self.info.as_ref()
            {
                cx.copy_to_clipboard(&info.room_id);
                enqueue_popup_notification(
                    "Room ID copied.",
                    PopupKind::Success,
                    Some(2.0),
                );
            }
            if self.button(cx, ids!(content_scroll.info_view.actions_row.actions_card.invite_button)).clicked(actions) {
                cx.widget_action(
                    self.widget_uid(),
                    RoomInfoPaneAction::InviteUser,
                );
            }
            if self.button(cx, ids!(content_scroll.info_view.actions_row.actions_card.report_room_button)).clicked(actions) {
                cx.widget_action(
                    self.widget_uid(),
                    RoomInfoPaneAction::ReportRoom,
                );
            }
            if self.button(cx, ids!(content_scroll.info_view.actions_row.leave_card.leave_room_button)).clicked(actions) {
                cx.widget_action(
                    self.widget_uid(),
                    RoomInfoPaneAction::LeaveRoom,
                );
            }

            if self.show_people_page
                && let Some(info) = self.info.as_ref()
                && self.people_display_count < info.people_entries.len()
            {
                let people_list = self.portal_list(cx, ids!(people_view.members_list_card.people_list));
                if people_list.scrolled(actions) {
                    let threshold = self.people_display_count.saturating_sub(5);
                    if people_list.first_id() + people_list.visible_items() >= threshold {
                        self.people_display_count = (self.people_display_count + 40).min(info.people_entries.len());
                        self.redraw(cx);
                    }
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let Some(info) = self.info.as_ref() else {
            // In inline (tab) mode the parent controls visibility, so don't
            // force-hide just because info hasn't been populated yet.
            if !self.inline {
                self.visible = false;
            }
            return self.view.draw_walk(cx, scope, walk);
        };

        // Slide animation + dimmed backdrop only apply to the overlay variant.
        if !self.inline {
            let panel_width = 320.0;
            let right_margin = -(self.slide * panel_width);
            let mut main_content = self.view(cx, ids!(main_content));
            script_apply_eval!(cx, main_content, {
                margin.right: #(right_margin)
            });
            let bg_alpha = (1.0 - self.slide) * 0.733;
            let bg_color = vec4(0.0, 0.0, 0.0, bg_alpha);
            let mut bg_view = self.view(cx, ids!(bg_view));
            script_apply_eval!(cx, bg_view, {
                draw_bg +: { color: #(bg_color) }
            });
        }

        self.button(cx, ids!(header.back_button)).set_visible(cx, self.show_people_page);
        // On the People sub-page the header shows the member count inline next to
        // the back arrow; the plain "Info" title is blanked so they don't double up.
        self.label(cx, ids!(header.title)).set_text(cx, if self.show_people_page { "" } else { "Info" });
        self.label(cx, ids!(header.members_header_count)).set_visible(cx, self.show_people_page);
        self.view(cx, ids!(content_scroll)).set_visible(cx, !self.show_people_page);
        self.view(cx, ids!(content_scroll.info_view)).set_visible(cx, !self.show_people_page);
        self.view(cx, ids!(people_view)).set_visible(cx, self.show_people_page);

        // ----- Hero: name, room id, favourite star -----
        self.label(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.name_row.title_wrap.room_name_value)).set_text(cx, &info.room_name);
        // Bot pill trailing the room name, mirroring the rooms-list bot pill.
        self.view(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.name_row.title_wrap.title_bot_pill)).set_visible(cx, info.show_title_bot_pill);
        self.label(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.room_id_row.room_id_value)).set_text(cx, &info.room_id);
        self.view(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.name_row.favorite_button.star_outline)).set_visible(cx, !info.is_favorite);
        self.view(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.name_row.favorite_button.star_filled)).set_visible(cx, info.is_favorite);

        // ----- Meta row: visibility / members / encryption -----
        self.label(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.meta_row.visibility_meta.visibility_value)).set_text(cx, &info.visibility);
        self.label(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.meta_row.members_meta.members_meta_value)).set_text(cx, &format!("{} members", info.member_count));
        self.label(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.meta_row.encryption_meta.encryption_value)).set_text(cx, &info.encryption);
        self.view(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.meta_row.encryption_meta.enc_icon_locked)).set_visible(cx, info.is_encrypted);
        self.view(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.meta_row.encryption_meta.enc_icon_unlocked)).set_visible(cx, !info.is_encrypted);

        // ----- Agent-enabled badge -----
        self.view(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_meta.badges_row.agent_badge_wrap)).set_visible(cx, info.is_agent_enabled);

        // ----- About / topic -----
        let topic_chars_len = info.topic.chars().count();
        let topic_has_more = topic_chars_len > TOPIC_PREVIEW_CHARS;
        let topic_display_text = if topic_has_more && !self.topic_expanded {
            let mut preview: String = info.topic.chars().take(TOPIC_PREVIEW_CHARS).collect();
            preview.push_str("...");
            preview
        } else {
            info.topic.clone()
        };
        self.label(cx, ids!(content_scroll.info_view.about_card.about_col.topic_value)).set_text(cx, &topic_display_text);
        self.button(cx, ids!(content_scroll.info_view.about_card.about_col.topic_toggle_button)).set_visible(cx, topic_has_more);
        self.button(cx, ids!(content_scroll.info_view.about_card.about_col.topic_toggle_button)).set_text(
            cx,
            if self.topic_expanded { "Collapse" } else { "Expand" },
        );

        // ----- Room hero avatar -----
        let room_avatar = self.avatar(cx, ids!(content_scroll.info_view.summary_card.hero_row.room_avatar));
        show_avatar_or_text(cx, &room_avatar, info.room_avatar_uri.as_ref(), &info.room_avatar_fallback_text);

        // ----- Members card: count, avatar stack, your role -----
        self.label(cx, ids!(content_scroll.info_view.members_card.members_header.members_count_group.members_count_value)).set_text(cx, &format!("{}", info.member_count));

        let stack_shown = info.people_entries.len().min(3);
        self.view(cx, ids!(content_scroll.info_view.members_card.members_detail.members_stack.stack_slot_0)).set_visible(cx, stack_shown > 0);
        self.view(cx, ids!(content_scroll.info_view.members_card.members_detail.members_stack.stack_slot_1)).set_visible(cx, stack_shown > 1);
        self.view(cx, ids!(content_scroll.info_view.members_card.members_detail.members_stack.stack_slot_2)).set_visible(cx, stack_shown > 2);
        if let Some(entry) = info.people_entries.first() {
            let avatar = self.avatar(cx, ids!(content_scroll.info_view.members_card.members_detail.members_stack.stack_slot_0.ring_0.stack_avatar_0));
            show_avatar_or_text(cx, &avatar, entry.avatar_uri.as_ref(), &entry.avatar_fallback_text);
        }
        if let Some(entry) = info.people_entries.get(1) {
            let avatar = self.avatar(cx, ids!(content_scroll.info_view.members_card.members_detail.members_stack.stack_slot_1.ring_1.stack_avatar_1));
            show_avatar_or_text(cx, &avatar, entry.avatar_uri.as_ref(), &entry.avatar_fallback_text);
        }
        if let Some(entry) = info.people_entries.get(2) {
            let avatar = self.avatar(cx, ids!(content_scroll.info_view.members_card.members_detail.members_stack.stack_slot_2.ring_2.stack_avatar_2));
            show_avatar_or_text(cx, &avatar, entry.avatar_uri.as_ref(), &entry.avatar_fallback_text);
        }
        let stack_more = info.people_entries.len().saturating_sub(stack_shown);
        self.view(cx, ids!(content_scroll.info_view.members_card.members_detail.members_stack.stack_more_wrap)).set_visible(cx, stack_more > 0);
        self.label(cx, ids!(content_scroll.info_view.members_card.members_detail.members_stack.stack_more_wrap.stack_more_chip.stack_more)).set_text(cx, &format!("+{stack_more}"));

        self.view(cx, ids!(content_scroll.info_view.members_card.members_detail.my_role_wrap)).set_visible(cx, !info.my_role.is_empty());
        self.label(cx, ids!(content_scroll.info_view.members_card.members_detail.my_role_wrap.my_role_chip.my_role_label)).set_text(cx, &info.my_role);

        if self.show_people_page && self.people_display_count == 0 {
            self.people_display_count = info.people_entries.len().min(40);
        }
        let visible_people_count = self.people_display_count.min(info.people_entries.len());
        self.label(cx, ids!(header.members_header_count)).set_text(cx, &info.people_count_text);
        self.view(cx, ids!(people_view.loading_label)).set_visible(cx, info.show_people_loading);
        self.view(cx, ids!(people_view.empty_label)).set_visible(cx, !info.show_people_loading && info.people_entries.is_empty());
        self.view(cx, ids!(people_view.members_list_card)).set_visible(cx, visible_people_count > 0);

        while let Some(widget) = self.view.draw_walk(cx, scope, walk).step() {
            let portal_list_ref = widget.as_portal_list();
            let Some(mut list) = portal_list_ref.borrow_mut() else { continue };

            list.set_item_range(cx, 0, visible_people_count);
            while let Some(item_id) = list.next_visible_item(cx) {
                let Some(entry) = info.people_entries.get(item_id) else { continue };
                let item = list.item(cx, item_id, id!(PersonEntry));
                item.as_room_info_people_entry().set_entry(cx, entry);
                item.draw_all(cx, &mut Scope::empty());
            }
        }
        DrawStep::done()
    }
}

impl RoomInfoSlidingPane {
    pub fn is_currently_shown(&self, _cx: &mut Cx) -> bool {
        self.visible
    }

    pub(super) fn set_info(&mut self, cx: &mut Cx, mut info: RoomInfoPaneInfo) {
        // Switching to a DIFFERENT room must reset the sub-page state — otherwise
        // a reused RoomScreen/pane lands on the previous room's People list
        // (which is empty until the new room's members load). Always fall back to
        // the Info view (and clear the topic-expand / paging state) on room change.
        let room_changed = self.info.as_ref()
            .is_some_and(|current| current.owned_room_id != info.owned_room_id);
        if room_changed {
            self.show_people_page = false;
            self.people_display_count = 0;
            self.topic_expanded = false;
            self.portal_list(cx, ids!(people_view.members_list_card.people_list))
                .set_first_id_and_scroll(0, 0.0);
        }

        // Preserve an in-flight optimistic favourite toggle across the frequent
        // Signal-driven rebuilds: keep the optimistic value until the freshly
        // built `info` (which re-reads `room.is_favourite()`) catches up, then
        // drop the override. A different room invalidates any stale override.
        if let Some((pending_room, pending_val)) = self.pending_favorite.clone() {
            if pending_room == info.owned_room_id {
                if info.is_favorite == pending_val {
                    self.pending_favorite = None;
                } else {
                    info.is_favorite = pending_val;
                }
            } else {
                self.pending_favorite = None;
            }
        }
        self.info = Some(info);
        if self.show_people_page {
            if let Some(info) = self.info.as_ref() {
                self.people_display_count = self.people_display_count
                    .max(40.min(info.people_entries.len()))
                    .min(info.people_entries.len());
            }
        }
        self.redraw(cx);
    }

    pub fn show(&mut self, cx: &mut Cx) {
        self.visible = true;
        self.is_animating_out = false;
        self.show_people_page = false;
        self.topic_expanded = false;
        self.people_display_count = 0;
        cx.set_key_focus(self.view.area());
        self.animator_play(cx, ids!(panel.show));
        self.view(cx, ids!(bg_view)).set_visible(cx, true);
        self.view.button(cx, ids!(close_button)).reset_hover(cx);
        self.redraw(cx);
    }

    pub fn hide(&mut self, cx: &mut Cx) {
        if !self.visible {
            return;
        }
        self.is_animating_out = true;
        self.animator_play(cx, ids!(panel.hide));
        self.redraw(cx);
    }
}

impl RoomInfoSlidingPaneRef {
    pub fn is_currently_shown(&self, cx: &mut Cx) -> bool {
        let Some(inner) = self.borrow() else { return false };
        inner.is_currently_shown(cx)
    }

    pub(super) fn set_info(&self, cx: &mut Cx, info: RoomInfoPaneInfo) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_info(cx, info);
    }

    /// Force the inline (docked, non-sliding) presentation. Set imperatively
    /// because the DSL `inline: true` instance override is not reliably applied
    /// to this `#[live]` field on the inline `info_content` instance.
    pub fn set_inline(&self, inline: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.inline = inline;
        }
    }

    pub fn show(&self, cx: &mut Cx) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show(cx);
    }

    pub fn hide(&self, cx: &mut Cx) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.hide(cx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_room_info_bot_identity_fingerprint_tracks_registry_agents() {
        let agent_id: OwnedUserId = "@agent:example.org".try_into().unwrap();
        let mut app_state = AppState::default();
        let before = room_info_bot_identity_fingerprint(Some(&app_state), None);

        app_state
            .agent_registry
            .register(agent_id.clone(), crate::app::AgentEntry::default());
        let after = room_info_bot_identity_fingerprint(Some(&app_state), None);

        assert_ne!(before, after);
        assert!(after.known_bot_user_ids.iter().any(|id| id == &agent_id));
    }

    #[test]
    fn test_room_info_bot_identity_fingerprint_tracks_appservice_known_bots() {
        let current_user_id: OwnedUserId = "@alice:example.org".try_into().unwrap();
        let mut app_state = AppState::default();
        app_state.bot_settings.enabled = true;
        let bot_id = app_state
            .bot_settings
            .resolved_bot_user_id(Some(current_user_id.as_ref()))
            .unwrap();
        let before = room_info_bot_identity_fingerprint(
            Some(&app_state),
            Some(current_user_id.as_ref()),
        );

        app_state.bot_settings.record_known_bot_user_ids([bot_id.clone()]);
        let after = room_info_bot_identity_fingerprint(
            Some(&app_state),
            Some(current_user_id.as_ref()),
        );

        assert_ne!(before, after);
        assert_eq!(after.resolved_parent_bot_user_id.as_ref(), Some(&bot_id));
        assert!(after.known_bot_user_ids.iter().any(|id| id == &bot_id));
    }

    #[test]
    fn test_room_info_bot_marker_hidden_after_agentlab_unbind() {
        let current_user_id: OwnedUserId = "@alice:example.org".try_into().unwrap();
        let agent_id: OwnedUserId = "@octos_mac:example.org".try_into().unwrap();
        let mut app_state = AppState::default();

        app_state.agent_registry.register(agent_id.clone(), crate::app::AgentEntry {
            framework: crate::app::AgentFramework::Octos,
            ..Default::default()
        });
        app_state.bot_settings.enabled = true;
        app_state.bot_settings.botfather_user_id = agent_id.to_string();
        app_state.bot_settings.record_known_bot_user_ids([agent_id.clone()]);

        let before = room_info_bot_identity_fingerprint(
            Some(&app_state),
            Some(current_user_id.as_ref()),
        );
        assert!(is_known_or_likely_bot(
            agent_id.as_ref(),
            before.resolved_parent_bot_user_id.as_deref(),
            &before.known_bot_user_ids,
        ));

        app_state.unregister_agent_and_clear_bot_identity(
            agent_id.as_ref(),
            Some(current_user_id.as_ref()),
        );
        let after = room_info_bot_identity_fingerprint(
            Some(&app_state),
            Some(current_user_id.as_ref()),
        );

        assert!(after.resolved_parent_bot_user_id.is_none());
        assert!(after.known_bot_user_ids.is_empty());
        assert!(!is_known_or_likely_bot(
            agent_id.as_ref(),
            after.resolved_parent_bot_user_id.as_deref(),
            &after.known_bot_user_ids,
        ));
    }

    #[test]
    fn test_room_info_title_bot_pill_hidden_after_room_unbound() {
        let room_id: OwnedRoomId = "!room:example.org".try_into().unwrap();
        let bot_id: OwnedUserId = "@bot:example.org".try_into().unwrap();
        let mut app_state = AppState::default();

        app_state
            .bot_settings
            .set_room_bound(room_id.clone(), Some(bot_id.clone()), true);
        assert!(room_info_title_shows_agent_badge(
            Some(&app_state), room_id.as_ref(), None, std::iter::empty(),
        ));

        app_state
            .bot_settings
            .set_room_bound(room_id.clone(), Some(bot_id), false);
        assert!(!room_info_title_shows_agent_badge(
            Some(&app_state), room_id.as_ref(), None, std::iter::empty(),
        ));
    }

    #[test]
    fn test_room_info_title_bot_pill_shown_when_member_is_registered_agent() {
        let room_id: OwnedRoomId = "!group:example.org".try_into().unwrap();
        let agent_id: OwnedUserId = "@octos_mac:example.org".try_into().unwrap();
        let mut app_state = AppState::default();

        app_state
            .agent_registry
            .register(agent_id.clone(), crate::app::AgentEntry::default());
        assert!(room_info_title_shows_agent_badge(
            Some(&app_state), room_id.as_ref(), None, [agent_id.as_ref()],
        ));

        app_state.agent_registry.unregister(agent_id.as_ref());
        assert!(!room_info_title_shows_agent_badge(
            Some(&app_state), room_id.as_ref(), None, [agent_id.as_ref()],
        ));
    }

    #[test]
    fn test_room_info_title_bot_pill_hidden_after_agent_registry_unbind() {
        let room_id: OwnedRoomId = "!dm:example.org".try_into().unwrap();
        let agent_id: OwnedUserId = "@agent:example.org".try_into().unwrap();
        let mut app_state = AppState::default();

        app_state
            .agent_registry
            .register(agent_id.clone(), crate::app::AgentEntry::default());
        assert!(room_info_title_shows_agent_badge(
            Some(&app_state),
            room_id.as_ref(),
            Some(agent_id.as_ref()),
            std::iter::empty(),
        ));

        app_state.agent_registry.unregister(agent_id.as_ref());
        assert!(!room_info_title_shows_agent_badge(
            Some(&app_state),
            room_id.as_ref(),
            Some(agent_id.as_ref()),
            std::iter::empty(),
        ));
    }

    #[test]
    fn test_room_info_title_bot_pill_hidden_after_agentlab_unbind_clears_binding() {
        let current_user_id: OwnedUserId = "@alice:example.org".try_into().unwrap();
        let room_id: OwnedRoomId = "!room:example.org".try_into().unwrap();
        let agent_id: OwnedUserId = "@octos_mac:example.org".try_into().unwrap();
        let mut app_state = AppState::default();

        app_state.agent_registry.register(agent_id.clone(), crate::app::AgentEntry {
            framework: crate::app::AgentFramework::Octos,
            ..Default::default()
        });
        app_state.bot_settings.enabled = true;
        app_state.bot_settings.botfather_user_id = agent_id.to_string();
        app_state.bot_settings.record_known_bot_user_ids([agent_id.clone()]);
        app_state
            .bot_settings
            .set_room_bound(room_id.clone(), Some(agent_id.clone()), true);
        assert!(room_info_title_shows_agent_badge(
            Some(&app_state), room_id.as_ref(), None, std::iter::empty(),
        ));

        app_state.unregister_agent_and_clear_bot_identity(
            agent_id.as_ref(),
            Some(current_user_id.as_ref()),
        );

        assert!(!room_info_title_shows_agent_badge(
            Some(&app_state), room_id.as_ref(), None, std::iter::empty(),
        ));
    }

    #[test]
    fn test_room_info_title_bot_pill_ignores_known_bot_without_binding_or_agent_dm() {
        let room_id: OwnedRoomId = "!group:example.org".try_into().unwrap();
        let bot_id: OwnedUserId = "@bot:example.org".try_into().unwrap();
        let mut app_state = AppState::default();
        app_state.bot_settings.record_known_bot_user_ids([bot_id.clone()]);

        // A known bot that is neither bound, a DM target, nor a registered-agent
        // member must not trigger the pill — even when it is a room member.
        assert!(!room_info_title_shows_agent_badge(
            Some(&app_state), room_id.as_ref(), None, [bot_id.as_ref()],
        ));
    }

    #[test]
    fn test_room_info_dm_target_requires_single_non_self_member() {
        let me: OwnedUserId = "@me:example.org".try_into().unwrap();
        let agent_id: OwnedUserId = "@agent:example.org".try_into().unwrap();
        let human_id: OwnedUserId = "@human:example.org".try_into().unwrap();

        assert_eq!(
            room_info_dm_target_from_user_ids(
                [me.as_ref(), agent_id.as_ref()],
                Some(me.as_ref()),
            ),
            Some(agent_id.clone()),
        );
        assert_eq!(
            room_info_dm_target_from_user_ids(
                [me.as_ref(), agent_id.as_ref(), human_id.as_ref()],
                Some(me.as_ref()),
            ),
            None,
        );
    }
}
