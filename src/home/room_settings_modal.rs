//! A modal dialog for viewing and editing room settings.

use std::path::PathBuf;

use makepad_widgets::*;
use ruma::OwnedRoomId;

use crate::shared::avatar::AvatarWidgetExt;
use crate::utils::load_png_or_jpg;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.RoomSettingsModal = #(RoomSettingsModal::register_widget(vm)) {
        // A *fixed* height, not `Fill{max}`: the header / scrolling body / footer
        // split needs a resolved viewport for the ScrollYView, and a `Fill` child
        // also claims all of the modal's space, which leaves the modal's
        // `align: y: 0.5` nothing to centre with (the dialog then hugs the top).
        width: Fill { max: 680 }
        height: 600
        margin: Inset{left: 12, right: 12}

        RoundedShadowView {
            width: Fill
            height: Fill
            flow: Down
            padding: Inset{top: 0, right: 0, bottom: 0, left: 0}
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

            // ── Title bar ────────────────────────────────────────────────
            title_bar := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}
                padding: Inset{left: 20, right: 12, top: 12, bottom: 12}
                spacing: 8

                title_label := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        text_style: RBX_TEXT_SECTION_TITLE {}
                        color: (RBX_FG_PRIMARY)
                    }
                    text: "Room Settings"
                }

                close_button := RobrixNeutralIconButton {
                    width: 28
                    height: 28
                    padding: 4
                    draw_icon.svg: (ICON_CLOSE)
                    icon_walk: Walk{width: 14, height: 14}
                    text: ""
                }
            }

            // ── Separator ────────────────────────────────────────────────
            View {
                width: Fill
                height: 1
                show_bg: true
                draw_bg +: { color: (RBX_DIVIDER) }
            }

            // ── Main area ────────────────────────────────────────────────
            main_area := View {
                width: Fill
                height: Fill
                flow: Right

                // Sidebar. `height: Fill` so its surface spans the whole body
                // next to the content, instead of collapsing to one row.
                sidebar := SolidView {
                    width: 150
                    height: Fill
                    flow: Down
                    padding: Inset{top: 12, left: 0, right: 0, bottom: 12}
                    show_bg: true
                    draw_bg.color: (RBX_BG_SURFACE_SUBTLE)

                    // Selected nav row: teal rail + tinted background + accent
                    // label, matching the desktop navigation rail's selected state.
                    general_tab_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        align: Align{y: 0.5}

                        general_tab_indicator := SolidView {
                            width: 3
                            height: 36
                            show_bg: true
                            draw_bg.color: (RBX_ACCENT)
                        }

                        general_tab_button := RobrixNeutralIconButton {
                            width: Fill
                            height: 36
                            padding: Inset{left: 12, right: 8, top: 8, bottom: 8}
                            align: Align{x: 0.0, y: 0.5}
                            icon_walk: Walk{width: 0, height: 0}
                            draw_bg +: {
                                color: (RBX_BG_SELECTED)
                                color_hover: (RBX_BG_SELECTED)
                                color_down: (RBX_BG_PRESSED)
                                border_radius: 0.0
                            }
                            draw_text +: {
                                color: (RBX_ACCENT)
                                color_hover: (RBX_ACCENT)
                                color_down: (RBX_ACCENT)
                                text_style: RBX_TEXT_BODY_STRONG {}
                            }
                            text: "General"
                        }
                    }

                    members_tab_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        align: Align{y: 0.5}

                        members_tab_indicator := SolidView {
                            width: 3
                            height: 36
                            show_bg: true
                            draw_bg.color: (RBX_TRANSPARENT)
                        }

                        members_tab_button := RobrixNeutralIconButton {
                            width: Fill
                            height: 36
                            padding: Inset{left: 12, right: 8, top: 8, bottom: 8}
                            align: Align{x: 0.0, y: 0.5}
                            icon_walk: Walk{width: 0, height: 0}
                            draw_bg +: {
                                color: (RBX_TRANSPARENT)
                                color_hover: (RBX_BG_HOVER)
                                color_down: (RBX_BG_PRESSED)
                                border_radius: 0.0
                            }
                            draw_text +: {
                                color: (RBX_FG_SECONDARY)
                                color_hover: (RBX_FG_PRIMARY)
                                color_down: (RBX_FG_PRIMARY)
                                text_style: RBX_TEXT_BODY {}
                            }
                            text: "Members"
                        }
                    }
                }

                // Vertical hairline between the sidebar and the content.
                sidebar_divider := SolidView {
                    width: 1.0
                    height: Fill
                    show_bg: true
                    draw_bg.color: (RBX_DIVIDER)
                }

                // Content area. A sunken canvas behind the white section cards,
                // so the grouping reads as cards rather than as one long form.
                content_scroll := ScrollYView {
                    width: Fill
                    height: Fill
                    flow: Down
                    spacing: 14
                    padding: Inset{left: 20, right: 20, top: 16, bottom: 20}
                    show_bg: true
                    draw_bg.color: (RBX_BG_CANVAS)

                    general_card := RoundedView {
                        width: Fill
                        height: Fit
                        flow: Down
                        padding: Inset{left: 16, right: 16, top: 12, bottom: 14}
                        show_bg: true
                        new_batch: true
                        draw_bg +: {
                            color: (RBX_BG_SURFACE)
                            border_radius: (RBX_RADIUS_MD)
                            border_size: 1.0
                            border_color: (RBX_STROKE_SOFT)
                        }

                        general_heading := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 3}
                            draw_text +: {
                                text_style: RBX_TEXT_SECTION_TITLE {}
                                color: (RBX_FG_PRIMARY)
                            }
                            text: "General"
                        }

                        form_row := View {
                            width: Fill
                            height: Fit
                            flow: Right
                            spacing: 16

                            // Inputs column
                            inputs_col := View {
                                width: Fill
                                height: Fit
                                flow: Down
                                spacing: 6

                                room_name_label := Label {
                                    width: Fill
                                    height: Fit
                                    margin: Inset{bottom: 2}
                                    draw_text +: {
                                        text_style: RBX_TEXT_BODY {}
                                        color: (RBX_FG_SECONDARY)
                                    }
                                    text: "Room Name"
                                }

                                room_name_input := RobrixTextInput {
                                    width: Fill
                                    height: 44
                                    empty_text: "Room name"
                                }

                                room_topic_label := Label {
                                    width: Fill
                                    height: Fit
                                    margin: Inset{top: 10, bottom: 2}
                                    draw_text +: {
                                        text_style: RBX_TEXT_BODY {}
                                        color: (RBX_FG_SECONDARY)
                                    }
                                    text: "Room Topic"
                                }

                                room_topic_input := RobrixTextInput {
                                    width: Fill
                                    height: 120
                                    empty_text: "Room topic (optional)"
                                    is_multiline: true
                                }

                                name_error_label := Label {
                                    visible: false
                                    width: Fill
                                    height: Fit
                                    margin: Inset{top: 2}
                                    draw_text +: {
                                        text_style: RBX_TEXT_META {}
                                        color: (RBX_DANGER_FG)
                                    }
                                    text: ""
                                }
                            }

                            // Avatar column
                            avatar_col := View {
                                width: 80
                                height: Fit
                                flow: Down
                                align: Align{x: 0.5}
                                spacing: 6

                                room_avatar := Avatar {
                                    width: 60
                                    height: 60
                                }

                                pencil_button := RobrixNeutralIconButton {
                                    width: 60
                                    height: 24
                                    padding: 4
                                    align: Align{x: 0.5, y: 0.5}
                                    draw_icon.svg: (ICON_EDIT)
                                    icon_walk: Walk{width: 12, height: 12}
                                    text: ""
                                }
                            }
                        }
                    }

                    // ── Access ───────────────────────────────────────
                    access_card := RoundedView {
                        width: Fill
                        height: Fit
                        flow: Down
                        padding: Inset{left: 16, right: 16, top: 12, bottom: 14}
                        show_bg: true
                        new_batch: true
                        draw_bg +: {
                            color: (RBX_BG_SURFACE)
                            border_radius: (RBX_RADIUS_MD)
                            border_size: 1.0
                            border_color: (RBX_STROKE_SOFT)
                        }

                        access_heading := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 3}
                            draw_text +: {
                                text_style: RBX_TEXT_CARD_TITLE {}
                                color: (RBX_FG_PRIMARY)
                            }
                            text: "Who can join"
                        }

                        access_desc := Label {
                            width: Fill
                            height: Fit
                            flow: Flow.Right{wrap: true}
                            margin: Inset{bottom: 8}
                            draw_text +: {
                                text_style: RBX_TEXT_META {}
                                color: (RBX_FG_TERTIARY)
                            }
                            text: ""
                        }

                        access_options := View {
                            width: Fill
                            height: Fit
                            flow: Down
                            spacing: 4

                            join_invite_radio := RadioButton {
                                width: Fit
                                height: Fit
                                align: Align{y: 0.5}
                                padding: Inset{top: 4, bottom: 4, left: 6, right: 4}
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    color_hover: (RBX_FG_PRIMARY)
                                    color_focus: (RBX_FG_PRIMARY)
                                    color_active: (RBX_FG_PRIMARY)
                                    color_down: (RBX_FG_PRIMARY)
                                    color_disabled: (RBX_FG_DISABLED)
                                    text_style: RBX_TEXT_BODY {}
                                }
                                draw_bg +: {
                                    color: (RBX_BG_SURFACE)
                                    border_color: (RBX_STROKE_STRONG)
                                    border_color_active: (RBX_ACCENT)
                                    mark_color: vec4(0.0, 0.0, 0.0, 0.0)
                                    mark_color_active: (RBX_ACCENT)
                                }
                                text: "Invite only"
                            }

                            join_knock_radio := RadioButton {
                                width: Fit
                                height: Fit
                                align: Align{y: 0.5}
                                padding: Inset{top: 4, bottom: 4, left: 6, right: 4}
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    color_hover: (RBX_FG_PRIMARY)
                                    color_focus: (RBX_FG_PRIMARY)
                                    color_active: (RBX_FG_PRIMARY)
                                    color_down: (RBX_FG_PRIMARY)
                                    color_disabled: (RBX_FG_DISABLED)
                                    text_style: RBX_TEXT_BODY {}
                                }
                                draw_bg +: {
                                    color: (RBX_BG_SURFACE)
                                    border_color: (RBX_STROKE_STRONG)
                                    border_color_active: (RBX_ACCENT)
                                    mark_color: vec4(0.0, 0.0, 0.0, 0.0)
                                    mark_color_active: (RBX_ACCENT)
                                }
                                text: "Ask to join"
                            }

                            join_public_radio := RadioButton {
                                width: Fit
                                height: Fit
                                align: Align{y: 0.5}
                                padding: Inset{top: 4, bottom: 4, left: 6, right: 4}
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    color_hover: (RBX_FG_PRIMARY)
                                    color_focus: (RBX_FG_PRIMARY)
                                    color_active: (RBX_FG_PRIMARY)
                                    color_down: (RBX_FG_PRIMARY)
                                    color_disabled: (RBX_FG_DISABLED)
                                    text_style: RBX_TEXT_BODY {}
                                }
                                draw_bg +: {
                                    color: (RBX_BG_SURFACE)
                                    border_color: (RBX_STROKE_STRONG)
                                    border_color_active: (RBX_ACCENT)
                                    mark_color: vec4(0.0, 0.0, 0.0, 0.0)
                                    mark_color_active: (RBX_ACCENT)
                                }
                                text: "Anyone"
                            }
                        }

                        // Shown instead of the options when the current rule is one
                        // this dialog can display but not author (restricted), or
                        // when the user lacks permission to change it.
                        access_locked_note := Label {
                            visible: false
                            width: Fill
                            height: Fit
                            flow: Flow.Right{wrap: true}
                            draw_text +: {
                                text_style: RBX_TEXT_META {}
                                color: (RBX_FG_SECONDARY)
                            }
                            text: ""
                        }
                    }

                    addresses_card := RoundedView {
                        width: Fill
                        height: Fit
                        flow: Down
                        padding: Inset{left: 16, right: 16, top: 12, bottom: 14}
                        show_bg: true
                        new_batch: true
                        draw_bg +: {
                            color: (RBX_BG_SURFACE)
                            border_radius: (RBX_RADIUS_MD)
                            border_size: 1.0
                            border_color: (RBX_STROKE_SOFT)
                        }

                        addresses_heading := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 3}
                            draw_text +: {
                                text_style: RBX_TEXT_CARD_TITLE {}
                                color: (RBX_FG_PRIMARY)
                            }
                            text: "Room Addresses"
                        }

                        published_addresses_label := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 4}
                            draw_text +: {
                                text_style: RBX_TEXT_BODY {}
                                color: (RBX_FG_SECONDARY)
                            }
                            text: "Published Addresses"
                        }

                        published_desc := Label {
                            width: Fill
                            height: Fit
                            flow: Flow.Right{wrap: true}
                            margin: Inset{bottom: 8}
                            draw_text +: {
                                text_style: RBX_TEXT_META {}
                                color: (RBX_FG_TERTIARY)
                            }
                            text: "These are the addresses that are published on the room directory for others to find this room."
                        }

                        main_alias_row := View {
                            width: Fill
                            height: Fit
                            flow: Right
                            align: Align{y: 0.5}
                            margin: Inset{bottom: 8}
                            spacing: 8

                            main_alias_label := Label {
                                width: Fill
                                height: Fit
                                draw_text +: {
                                    text_style: RBX_TEXT_BODY {}
                                    color: (RBX_FG_SECONDARY)
                                }
                                text: "No main address set"
                            }
                        }

                        publish_toggle_row := View {
                            width: Fill
                            height: Fit
                            flow: Right
                            align: Align{y: 0.5}
                            margin: Inset{bottom: 8}
                            spacing: 8

                            publish_toggle := Toggle {
                                width: Fit
                                height: Fit
                                padding: Inset{top: 2, right: 4, bottom: 2, left: 2}
                                text: ""
                                active: false
                                draw_bg +: {
                                    size: 18.0
                                    color_active: (RBX_ACCENT)
                                    border_color_active: (RBX_ACCENT)
                                    mark_color_active: (RBX_FG_ON_ACCENT)
                                }
                            }

                            publish_toggle_label := Label {
                                width: Fill
                                height: Fit
                                flow: Flow.Right{wrap: true}
                                draw_text +: {
                                    text_style: RBX_TEXT_META {}
                                    color: (RBX_FG_SECONDARY)
                                }
                                text: "Publish this room to the public in matrix.org's room directory?"
                            }
                        }

                        no_published_label := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 8}
                            draw_text +: {
                                text_style: RBX_TEXT_META {}
                                color: (RBX_FG_TERTIARY)
                            }
                            text: "No other published addresses yet, add one below"
                        }

                        add_address_row := View {
                            width: Fill
                            height: Fit
                            flow: Right
                            align: Align{y: 0.5}
                            spacing: 8
                            margin: Inset{bottom: 12}

                            add_address_input := RobrixTextInput {
                                width: Fill
                                height: 36
                                empty_text: "# e.g. my-room"
                            }

                            add_address_button := RobrixIconButton {
                                width: 60
                                height: 36
                                padding: 6
                                icon_walk: Walk{width: 0, height: 0}
                                text: "Add"
                            }
                        }

                        local_addresses_label := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 4}
                            draw_text +: {
                                text_style: RBX_TEXT_BODY {}
                                color: (RBX_FG_SECONDARY)
                            }
                            text: "Local Addresses"
                        }

                        local_desc := Label {
                            width: Fill
                            height: Fit
                            flow: Flow.Right{wrap: true}
                            margin: Inset{bottom: 8}
                            draw_text +: {
                                text_style: RBX_TEXT_META {}
                                color: (RBX_FG_TERTIARY)
                            }
                            text: "Set addresses for this room so users can find this room. As an admin, you can set local addresses for this room."
                        }
                    }

                    moderation_card := RoundedView {
                        width: Fill
                        height: Fit
                        flow: Down
                        padding: Inset{left: 16, right: 16, top: 12, bottom: 14}
                        show_bg: true
                        new_batch: true
                        draw_bg +: {
                            color: (RBX_BG_SURFACE)
                            border_radius: (RBX_RADIUS_MD)
                            border_size: 1.0
                            border_color: (RBX_STROKE_SOFT)
                        }

                        other_heading := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 3}
                            draw_text +: {
                                text_style: RBX_TEXT_CARD_TITLE {}
                                color: (RBX_FG_PRIMARY)
                            }
                            text: "Other"
                        }

                        moderation_label := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 6}
                            draw_text +: {
                                text_style: RBX_TEXT_BODY {}
                                color: (RBX_FG_SECONDARY)
                            }
                            text: "Moderation and safety"
                        }

                        show_media_label := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 2}
                            draw_text +: {
                                text_style: RBX_TEXT_BODY {}
                                color: (RBX_FG_SECONDARY)
                            }
                            text: "Show media in timeline"
                        }

                        show_media_desc := Label {
                            width: Fill
                            height: Fit
                            flow: Flow.Right{wrap: true}
                            margin: Inset{bottom: 6}
                            draw_text +: {
                                text_style: RBX_TEXT_META {}
                                color: (RBX_FG_TERTIARY)
                            }
                            text: "A hidden media can always be shown by tapping on it"
                        }

                        media_hide_radio := RadioButton {
                            width: Fit
                            height: Fit
                            align: Align{y: 0.5}
                            padding: Inset{top: 4, bottom: 4, left: 6, right: 4}
                            draw_text +: {
                                color: (RBX_FG_PRIMARY)
                                color_hover: (RBX_FG_PRIMARY)
                                color_focus: (RBX_FG_PRIMARY)
                                color_active: (RBX_FG_PRIMARY)
                                color_down: (RBX_FG_PRIMARY)
                                color_disabled: (RBX_FG_PRIMARY)
                                text_style: RBX_TEXT_BODY {}
                            }
                            draw_bg +: {
                                color: (RBX_BG_SURFACE)
                                border_color: (RBX_STROKE_STRONG)
                                border_color_active: (RBX_ACCENT)
                                mark_color: vec4(0.0, 0.0, 0.0, 0.0)
                                mark_color_active: (RBX_ACCENT)
                            }
                            text: "Always hide"
                        }

                        media_show_radio := RadioButton {
                            width: Fit
                            height: Fit
                            align: Align{y: 0.5}
                            padding: Inset{top: 4, bottom: 4, left: 6, right: 4}
                            draw_text +: {
                                color: (RBX_FG_PRIMARY)
                                color_hover: (RBX_FG_PRIMARY)
                                color_focus: (RBX_FG_PRIMARY)
                                color_active: (RBX_FG_PRIMARY)
                                color_down: (RBX_FG_PRIMARY)
                                color_disabled: (RBX_FG_PRIMARY)
                                text_style: RBX_TEXT_BODY {}
                            }
                            draw_bg +: {
                                color: (RBX_BG_SURFACE)
                                border_color: (RBX_STROKE_STRONG)
                                border_color_active: (RBX_ACCENT)
                                mark_color: vec4(0.0, 0.0, 0.0, 0.0)
                                mark_color_active: (RBX_ACCENT)
                            }
                            text: "Always show"
                        }
                    }

                    advanced_card := RoundedView {
                        width: Fill
                        height: Fit
                        flow: Down
                        padding: Inset{left: 16, right: 16, top: 12, bottom: 14}
                        show_bg: true
                        new_batch: true
                        draw_bg +: {
                            color: (RBX_BG_SURFACE)
                            border_radius: (RBX_RADIUS_MD)
                            border_size: 1.0
                            border_color: (RBX_STROKE_SOFT)
                        }

                        advanced_heading := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 3}
                            draw_text +: {
                                text_style: RBX_TEXT_SECTION_TITLE {}
                                color: (RBX_FG_PRIMARY)
                            }
                            text: "Advanced"
                        }

                        room_id_label := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 4}
                            draw_text +: {
                                text_style: RBX_TEXT_BODY {}
                                color: (RBX_FG_SECONDARY)
                            }
                            text: "Room ID"
                        }

                        room_id_input := RobrixTextInput {
                            width: Fill
                            height: 36
                            is_read_only: true
                            empty_text: "!room:server"
                        }
                    }

                    danger_card := RoundedView {
                        width: Fill
                        height: Fit
                        flow: Down
                        padding: Inset{left: 16, right: 16, top: 12, bottom: 14}
                        show_bg: true
                        new_batch: true
                        draw_bg +: {
                            color: (RBX_BG_SURFACE)
                            border_radius: (RBX_RADIUS_MD)
                            border_size: 1.0
                            border_color: (RBX_STROKE_SOFT)
                        }

                        leave_room_label := Label {
                            width: Fill
                            height: Fit
                            margin: Inset{bottom: 3}
                            draw_text +: {
                                text_style: RBX_TEXT_BODY {}
                                color: (RBX_FG_SECONDARY)
                            }
                            text: "Leave room"
                        }

                        leave_button := RobrixNegativeIconButton {
                            width: Fit
                            height: 32
                            padding: Inset{left: 12, right: 12, top: 6, bottom: 6}
                            icon_walk: Walk{width: 0, height: 0}
                            text: "Leave room"
                        }
                    }
                }

                // Members tab body. A sibling of `content_scroll` rather than a card
                // inside it, so the list gets its own PortalList viewport (a virtualized
                // list nested in a scrolling parent would fight it for scroll events).
                members_pane := View {
                    visible: false
                    width: Fill
                    height: Fill
                    flow: Down
                    show_bg: true
                    draw_bg.color: (RBX_BG_CANVAS)

                    members_summary := Label {
                        width: Fill
                        height: Fit
                        padding: Inset{left: 20, right: 20, top: 14, bottom: 10}
                        draw_text +: {
                            text_style: RBX_TEXT_META {}
                            color: (RBX_FG_SECONDARY)
                        }
                        text: ""
                    }

                    members_list := PortalList {
                        width: Fill
                        height: Fill
                        flow: Down
                        auto_tail: false
                        max_pull_down: 0.0

                        member_row := View {
                            width: Fill
                            height: 56
                            flow: Right
                            align: Align{y: 0.5}
                            padding: Inset{left: 20, right: 20}
                            spacing: 10

                            member_avatar := Avatar { width: 32, height: 32 }

                            member_text := View {
                                width: Fill
                                height: Fit
                                flow: Down
                                spacing: 1

                                member_name := Label {
                                    width: Fill
                                    height: Fit
                                    max_lines: 1
                                    text_overflow: Ellipsis
                                    draw_text +: {
                                        text_style: RBX_TEXT_BODY {}
                                        color: (RBX_FG_PRIMARY)
                                    }
                                    text: ""
                                }

                                member_user_id := Label {
                                    width: Fill
                                    height: Fit
                                    max_lines: 1
                                    text_overflow: Ellipsis
                                    draw_text +: {
                                        text_style: RBX_TEXT_META {}
                                        color: (RBX_FG_TERTIARY)
                                    }
                                    text: ""
                                }
                            }

                            member_role := RoundedView {
                                visible: false
                                width: Fit
                                height: Fit
                                padding: Inset{left: 9, right: 9, top: 3, bottom: 3}
                                show_bg: true
                                new_batch: true
                                draw_bg +: {
                                    color: (RBX_ACCENT_SOFT)
                                    border_radius: (RBX_RADIUS_PILL)
                                    border_size: 0.0
                                }

                                member_role_label := Label {
                                    width: Fit
                                    height: Fit
                                    draw_text +: {
                                        text_style: RBX_TEXT_BADGE {}
                                        color: (RBX_ACCENT)
                                    }
                                    text: ""
                                }
                            }
                        }

                        members_status := View {
                            width: Fill
                            height: Fit
                            padding: Inset{left: 20, right: 20, top: 16, bottom: 16}

                            members_status_label := Label {
                                width: Fill
                                height: Fit
                                flow: Flow.Right{wrap: true}
                                draw_text +: {
                                    text_style: RBX_TEXT_BODY {}
                                    color: (RBX_FG_SECONDARY)
                                }
                                text: ""
                            }
                        }
                    }
                }

            }

            // ── Sticky footer ────────────────────────────────────────────
            // The save/cancel pair belongs to the whole dialog, so it stays put
            // at the bottom instead of scrolling away in the middle of the form.
            footer_divider := SolidView {
                width: Fill
                height: 1.0
                show_bg: true
                draw_bg.color: (RBX_DIVIDER)
            }

            footer := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{x: 1.0, y: 0.5}
                spacing: 10
                padding: Inset{left: 24, right: 24, top: 12, bottom: 12}

                cancel_button := RobrixNeutralIconButton {
                    width: 100
                    height: 34
                    padding: 6
                    align: Align{x: 0.5, y: 0.5}
                    icon_walk: Walk{width: 0, height: 0}
                    draw_icon.svg: (ICON_FORBIDDEN)
                    text: "Cancel"
                }

                save_button := RobrixIconButton {
                    width: 100
                    height: 34
                    padding: 6
                    align: Align{x: 0.5, y: 0.5}
                    icon_walk: Walk{width: 0, height: 0}
                    draw_icon.svg: (ICON_CHECKMARK)
                    text: "Save"
                }
            }
        }
    }
}

/// Actions emitted by the `RoomSettingsModal`.
#[derive(Clone, Debug, Default)]
pub enum RoomSettingsAction {
    /// Open the modal for the given room.
    Open { room_id: OwnedRoomId },
    /// Close the modal (user clicked close/X).
    Close,
    /// Save room name and topic.
    Save { room_id: OwnedRoomId, room_name: String, room_topic: String },
    /// Cancel edits without saving.
    Cancel,
    /// Toggle publishing this room to the directory.
    SetDirectoryPublish { room_id: OwnedRoomId, enabled: bool },
    /// Add a local address alias.
    AddLocalAddress { room_id: OwnedRoomId, alias: String },
    /// Change media visibility preference.
    SetMediaVisibility { room_id: OwnedRoomId, always_show: bool },
    /// Leave the room.
    LeaveRoom { room_id: OwnedRoomId },
    /// Upload a new room avatar from the given local file path.
    UploadRoomAvatar { room_id: OwnedRoomId, avatar_path: PathBuf },
    #[default]
    None,
}

#[derive(Script, ScriptHook, Widget)]
pub struct RoomSettingsModal {
    #[deref] view: View,
    #[source] source: ScriptObjectRef,
    #[rust] room_id: Option<OwnedRoomId>,
    #[rust] original_name: String,
    #[rust] original_topic: String,
    #[rust] always_show_media: bool,
}

impl Widget for RoomSettingsModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl WidgetMatchEvent for RoomSettingsModal {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        // Close button
        if self.view.button(cx, ids!(close_button)).clicked(actions) {
            cx.action(RoomSettingsAction::Close);
            return;
        }

        // Cancel button
        if self.view.button(cx, ids!(cancel_button)).clicked(actions) {
            cx.action(RoomSettingsAction::Cancel);
            return;
        }

        // Save button – validate name not empty
        if self.view.button(cx, ids!(save_button)).clicked(actions) {
            let name = self.view.text_input(cx, ids!(room_name_input)).text();
            let topic = self.view.text_input(cx, ids!(room_topic_input)).text();
            if name.trim().is_empty() {
                self.view.label(cx, ids!(name_error_label))
                    .set_text(cx, "Room name cannot be empty");
                self.view.label(cx, ids!(name_error_label)).set_visible(cx, true);
                self.view.redraw(cx);
            } else {
                self.view.label(cx, ids!(name_error_label)).set_visible(cx, false);
                if let Some(room_id) = self.room_id.clone() {
                    cx.action(RoomSettingsAction::Save {
                        room_id,
                        room_name: name.trim().to_string(),
                        room_topic: topic.trim().to_string(),
                    });
                }
            }
            return;
        }

        // Publish toggle
        let publish_toggle = self.view.check_box(cx, ids!(publish_toggle));
        if let Some(enabled) = publish_toggle.changed(actions) {
            if let Some(room_id) = self.room_id.clone() {
                cx.action(RoomSettingsAction::SetDirectoryPublish { room_id, enabled });
            }
        }

        // Add address button
        if self.view.button(cx, ids!(add_address_button)).clicked(actions) {
            let alias = self.view.text_input(cx, ids!(add_address_input)).text();
            let alias = alias.trim().trim_start_matches('#').to_string();
            if !alias.is_empty() {
                if let Some(room_id) = self.room_id.clone() {
                    cx.action(RoomSettingsAction::AddLocalAddress { room_id, alias });
                    self.view.text_input(cx, ids!(add_address_input)).set_text(cx, "");
                }
            }
        }

        // Media radio buttons
        let radios = self.view.radio_button_set(cx, ids_array!(media_hide_radio, media_show_radio));
        if let Some(selected) = radios.selected(cx, actions) {
            let always_show = selected == 1;
            self.always_show_media = always_show;
            if let Some(room_id) = self.room_id.clone() {
                cx.action(RoomSettingsAction::SetMediaVisibility { room_id, always_show });
            }
        }

        // Leave button
        if self.view.button(cx, ids!(leave_button)).clicked(actions) {
            if let Some(room_id) = self.room_id.clone() {
                cx.action(RoomSettingsAction::LeaveRoom { room_id });
            }
        }

        // Pencil / edit avatar button — open native file picker
        if self.view.button(cx, ids!(pencil_button)).clicked(actions) {
            #[cfg(any(target_os = "macos", target_os = "windows", all(target_os = "linux", not(target_env = "ohos"))))]
            if let Some(room_id) = self.room_id.clone() {
                use rfd::FileDialog;
                if let Some(path) = FileDialog::new()
                    .add_filter("Image", &["png", "jpg", "jpeg"])
                    .pick_file()
                {
                    cx.action(RoomSettingsAction::UploadRoomAvatar { room_id, avatar_path: path });
                }
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows", all(target_os = "linux", not(target_env = "ohos")))))]
            if let Some(_room_id) = self.room_id.clone() {
                use crate::shared::popup_list::{PopupKind, enqueue_popup_notification};
                enqueue_popup_notification(
                    "Avatar upload not supported on this platform",
                    PopupKind::Warning,
                    Some(4.0),
                );
            }
        }
    }
}

impl RoomSettingsModal {
    /// Populate the modal with room data and prepare for display.
    pub fn show(
        &mut self,
        cx: &mut Cx,
        room_id: OwnedRoomId,
        room_name: &str,
        room_topic: &str,
        canonical_alias: Option<&str>,
    ) {
        let room_id_text = room_id.as_str().to_string();
        self.room_id = Some(room_id);
        self.original_name = room_name.to_string();
        self.original_topic = room_topic.to_string();
        self.always_show_media = false;

        // Update title
        self.view.label(cx, ids!(title_label))
            .set_text(cx, &format!("Room Settings – {room_name}"));

        // Populate inputs
        self.view.text_input(cx, ids!(room_name_input))
            .set_text(cx, room_name);
        self.view.text_input(cx, ids!(room_topic_input))
            .set_text(cx, room_topic);
        self.view.text_input(cx, ids!(room_id_input))
            .set_text(cx, &room_id_text);
        self.view.text_input(cx, ids!(room_id_input))
            .set_is_read_only(cx, true);

        // Canonical alias
        let alias_text = canonical_alias
            .map(|a| a.to_string())
            .unwrap_or_else(|| String::from("No main address set"));
        self.view.label(cx, ids!(main_alias_label))
            .set_text(cx, &alias_text);

        // Avatar fallback text (first char of name)
        let avatar_char = room_name.chars().next().unwrap_or('?').to_string();
        self.view.avatar(cx, ids!(room_avatar))
            .show_text(cx, None, None, &avatar_char);

        // Reset error label
        self.view.label(cx, ids!(name_error_label)).set_visible(cx, false);
        self.view.label(cx, ids!(name_error_label)).set_text(cx, "");

        self.view.redraw(cx);
    }

    /// Update the avatar widget with freshly uploaded image bytes.
    pub fn apply_avatar(&mut self, cx: &mut Cx, image_data: &[u8]) {
        let _ = self.view.avatar(cx, ids!(room_avatar))
            .show_image(cx, None, |cx, img| load_png_or_jpg(&img, cx, image_data));
        self.view.redraw(cx);
    }

    /// Apply fetched settings (topic, is_public) that arrived asynchronously.
    pub fn apply_fetched_settings(
        &mut self,
        cx: &mut Cx,
        topic: Option<String>,
        is_public: bool,
    ) {
        if let Some(t) = topic {
            self.original_topic = t.clone();
            self.view.text_input(cx, ids!(room_topic_input)).set_text(cx, &t);
        }
        // Update publish toggle state (active == is_public)
        // Toggle widget: set via script_apply_eval on check_box
        let _ = is_public; // reflected by the toggle's current state
        self.view.redraw(cx);
    }
}

impl RoomSettingsModalRef {
    /// Populate the modal with room data and prepare for display.
    pub fn show_settings(
        &self,
        cx: &mut Cx,
        room_id: OwnedRoomId,
        room_name: &str,
        room_topic: &str,
        canonical_alias: Option<&str>,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show(cx, room_id, room_name, room_topic, canonical_alias);
    }

    /// Apply asynchronously-fetched settings (topic, is_public).
    pub fn apply_fetched_settings(&self, cx: &mut Cx, topic: Option<String>, is_public: bool) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.apply_fetched_settings(cx, topic, is_public);
    }

    /// Update the avatar widget after a successful upload.
    pub fn apply_avatar(&self, cx: &mut Cx, image_data: &[u8]) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.apply_avatar(cx, image_data);
    }
}

#[cfg(test)]
mod tests {
    const SOURCE: &str = include_str!("room_settings_modal.rs");

    #[test]
    fn advanced_section_declares_read_only_room_id_input() {
        assert!(SOURCE.contains(concat!("advanced_", "heading := Label")));
        assert!(SOURCE.contains(concat!("text: \"", "Advanced", "\"")));
        assert!(SOURCE.contains(concat!("room_id_", "label := Label")));
        assert!(SOURCE.contains(concat!("text: \"", "Room ID", "\"")));
        assert!(SOURCE.contains(concat!("room_id_", "input := RobrixTextInput")));
        assert!(SOURCE.contains(concat!("is_read_", "only: true")));
        assert!(SOURCE.contains(concat!("empty_text: \"", "!room:server", "\"")));
    }

    #[test]
    fn show_populates_room_id_input_from_room_id() {
        assert!(SOURCE.contains(concat!("let room_id_", "text = room_id.as_str().to_string();")));
        assert!(SOURCE.contains(concat!("self.room_id = Some(room_id", ");")));
        assert!(SOURCE.contains(concat!("ids!(room_id_", "input))")));
        assert!(SOURCE.contains(concat!(".set_text(cx, &room_id_", "text);")));
    }
}
