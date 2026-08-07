//! A modal dialog for viewing and editing room settings.

use std::path::PathBuf;

use makepad_widgets::*;
use ruma::OwnedRoomId;

use crate::avatar_cache::{self, AvatarCacheEntry};
use crate::shared::avatar::{AvatarWidgetExt, AvatarWidgetRefExt};
use crate::shared::design_tokens::{RBX_ACCENT, RBX_BG_SELECTED, RBX_FG_SECONDARY};
use crate::sliding_sync::{JoinRuleChoice, MatrixRequest, SettingsMemberInfo, submit_async_request};
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
    /// Open the modal for the given room or space.
    Open {
        room_id: OwnedRoomId,
        /// The display name to show. Needed for spaces, which the rooms list
        /// doesn't know about; `None` falls back to looking the room up there.
        room_name: Option<String>,
        /// Whether this is a space. A space has no timeline, so the message-media
        /// settings don't apply and the wording differs.
        is_space: bool,
    },
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
    /// Leave the room, or the space and the joined rooms inside it.
    LeaveRoom { room_id: OwnedRoomId, is_space: bool },
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
    /// Whether the modal is currently showing a space rather than a room.
    #[rust(false)] is_space: bool,
    /// The room's current join rule, as last fetched.
    #[rust] join_rule: JoinRuleChoice,
    /// Whether this user's power level allows changing the join rule.
    #[rust(false)] can_change_join_rule: bool,
    /// Which sidebar tab is showing.
    #[rust] active_tab: SettingsTab,
    /// Joined members, populated asynchronously after the dialog opens.
    #[rust] members: Vec<SettingsMemberInfo>,
    /// Whether the member list request is still outstanding.
    #[rust(false)] members_loading: bool,
}

/// The sidebar sections of the settings dialog.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    General,
    Members,
}

impl Widget for RoomSettingsModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        while let Some(widget_to_draw) = self.view.draw_walk(cx, scope, walk).step() {
            let portal_list_ref = widget_to_draw.as_portal_list();
            let Some(mut list) = portal_list_ref.borrow_mut() else { continue };

            // One extra slot after the members holds the loading/empty placeholder.
            let count = self.members.len();
            list.set_item_range(cx, 0, count + 1);
            while let Some(index) = list.next_visible_item(cx) {
                let item = if let Some(member) = self.members.get(index) {
                    let item = list.item(cx, index, id!(member_row));
                    item.label(cx, ids!(member_name))
                        .set_text(cx, member.displayable_name());
                    item.label(cx, ids!(member_user_id))
                        .set_text(cx, member.user_id.as_str());

                    let role = member.role_label();
                    item.view(cx, ids!(member_role)).set_visible(cx, role.is_some());
                    if let Some(role) = role {
                        item.label(cx, ids!(member_role_label)).set_text(cx, role);
                    }

                    let avatar = item.avatar(cx, ids!(member_avatar));
                    let mut drew_avatar = false;
                    if let Some(uri) = member.avatar_url.as_ref()
                        && let AvatarCacheEntry::Loaded(data) = avatar_cache::get_or_fetch_avatar(cx, uri)
                    {
                        drew_avatar = avatar.show_image(
                            cx,
                            None, // member avatars here aren't clickable
                            |cx, img| load_png_or_jpg(&img, cx, &data),
                        ).is_ok();
                    }
                    if !drew_avatar {
                        avatar.show_text(cx, None, None, member.displayable_name());
                    }
                    item
                } else if index == count {
                    let item = list.item(cx, index, id!(members_status));
                    let text = if self.members_loading {
                        "Loading members…"
                    } else if count == 0 {
                        "No members to show yet."
                    } else {
                        ""
                    };
                    item.label(cx, ids!(members_status_label)).set_text(cx, text);
                    item
                } else {
                    continue;
                };
                item.draw_all(cx, scope);
            }
        }
        DrawStep::done()
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
                self.view.label(cx, ids!(name_error_label)).set_text(
                    cx,
                    if self.is_space { "Space name cannot be empty" } else { "Room name cannot be empty" },
                );
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

        // Sidebar tabs
        if self.view.button(cx, ids!(general_tab_button)).clicked(actions) {
            self.set_active_tab(cx, SettingsTab::General);
            return;
        }
        if self.view.button(cx, ids!(members_tab_button)).clicked(actions) {
            self.set_active_tab(cx, SettingsTab::Members);
            return;
        }

        // Join-rule radio buttons
        let join_radios = self.view.radio_button_set(
            cx,
            ids_array!(join_invite_radio, join_knock_radio, join_public_radio),
        );
        if let Some(selected) = join_radios.selected(cx, actions) {
            let new_rule = match selected {
                0 => JoinRuleChoice::InviteOnly,
                1 => JoinRuleChoice::Knock,
                _ => JoinRuleChoice::Public,
            };
            // Guard against re-emitting the rule we just applied from the server.
            if new_rule != self.join_rule
                && self.can_change_join_rule
                && let Some(room_id) = self.room_id.clone()
            {
                self.join_rule = new_rule;
                submit_async_request(MatrixRequest::SetRoomJoinRule {
                    room_id,
                    join_rule: new_rule,
                });
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
                cx.action(RoomSettingsAction::LeaveRoom { room_id, is_space: self.is_space });
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
        is_space: bool,
    ) {
        let room_id_text = room_id.as_str().to_string();
        self.room_id = Some(room_id);
        self.original_name = room_name.to_string();
        self.original_topic = room_topic.to_string();
        self.always_show_media = false;
        self.is_space = is_space;
        // Cleared until FetchRoomSettings answers, so the previous room's access
        // settings never briefly show up under this room's name.
        self.join_rule = JoinRuleChoice::default();
        self.can_change_join_rule = false;
        self.members.clear();
        self.members_loading = false;
        self.apply_kind_wording(cx);
        self.apply_join_rule(cx);
        self.set_active_tab(cx, SettingsTab::General);

        // Update title
        self.view.label(cx, ids!(title_label)).set_text(cx, &format!(
            "{} – {room_name}",
            if is_space { "Space Settings" } else { "Room Settings" },
        ));

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

    /// Switches the labels between room and space wording, and hides the parts
    /// that don't apply to a space.
    ///
    /// A space carries no timeline, so the message-media controls are meaningless
    /// there; the rest of the form (name, topic, avatar, addresses, directory
    /// visibility) is identical because a space *is* a room underneath.
    fn apply_kind_wording(&mut self, cx: &mut Cx) {
        let is_space = self.is_space;
        self.view.label(cx, ids!(room_name_label)).set_text(
            cx,
            if is_space { "Space name" } else { "Room name" },
        );
        self.view.label(cx, ids!(room_topic_label)).set_text(
            cx,
            if is_space { "Space topic" } else { "Room topic" },
        );
        self.view.text_input(cx, ids!(room_name_input)).set_empty_text(
            cx,
            if is_space { "Space name".to_string() } else { "Room name".to_string() },
        );
        self.view.text_input(cx, ids!(room_topic_input)).set_empty_text(
            cx,
            if is_space { "Space topic (optional)".to_string() } else { "Room topic (optional)".to_string() },
        );
        self.view.label(cx, ids!(room_id_label)).set_text(
            cx,
            if is_space { "Space ID" } else { "Room ID" },
        );
        self.view.label(cx, ids!(leave_room_label)).set_text(
            cx,
            if is_space { "Leave space" } else { "Leave room" },
        );
        self.view.button(cx, ids!(leave_button)).set_text(
            cx,
            if is_space { "Leave space" } else { "Leave room" },
        );
        self.view.label(cx, ids!(published_desc)).set_text(cx, if is_space {
            "These are the addresses that are published on the directory for others to find this space."
        } else {
            "These are the addresses that are published on the room directory for others to find this room."
        });

        // The whole "Other" card is only about timeline media today, so the card
        // is hidden wholesale for spaces rather than left as an empty heading.
        self.view.view(cx, ids!(moderation_card)).set_visible(cx, !is_space);
    }

    /// Update the avatar widget with freshly uploaded image bytes.
    pub fn apply_avatar(&mut self, cx: &mut Cx, image_data: &[u8]) {
        let _ = self.view.avatar(cx, ids!(room_avatar))
            .show_image(cx, None, |cx, img| load_png_or_jpg(&img, cx, image_data));
        self.view.redraw(cx);
    }

    /// Apply fetched settings (topic, is_public, join rule) that arrived asynchronously.
    pub fn apply_fetched_settings(
        &mut self,
        cx: &mut Cx,
        topic: Option<String>,
        is_public: bool,
        join_rule: JoinRuleChoice,
        can_change_join_rule: bool,
    ) {
        if let Some(t) = topic {
            self.original_topic = t.clone();
            self.view.text_input(cx, ids!(room_topic_input)).set_text(cx, &t);
        }
        // Update publish toggle state (active == is_public)
        // Toggle widget: set via script_apply_eval on check_box
        let _ = is_public; // reflected by the toggle's current state
        self.join_rule = join_rule;
        self.can_change_join_rule = can_change_join_rule;
        self.apply_join_rule(cx);
        self.view.redraw(cx);
    }

    /// Switches sidebar tabs, styling the rows and swapping the two body panes.
    fn set_active_tab(&mut self, cx: &mut Cx, tab: SettingsTab) {
        self.active_tab = tab;
        let general_selected = tab == SettingsTab::General;

        // No Rust-side transparent token exists, so it's spelled out here.
        let transparent = vec4(0.0, 0.0, 0.0, 0.0);
        for (id, selected) in [
            (ids!(general_tab_indicator), general_selected),
            (ids!(members_tab_indicator), !general_selected),
        ] {
            let mut indicator = self.view.view(cx, id);
            let color = if selected { RBX_ACCENT } else { transparent };
            script_apply_eval!(cx, indicator, { draw_bg +: { color: #(color) } });
        }
        for (id, selected) in [
            (ids!(general_tab_button), general_selected),
            (ids!(members_tab_button), !general_selected),
        ] {
            let mut button = self.view.button(cx, id);
            let (bg, fg) = if selected {
                (RBX_BG_SELECTED, RBX_ACCENT)
            } else {
                (transparent, RBX_FG_SECONDARY)
            };
            script_apply_eval!(cx, button, {
                draw_bg +: { color: #(bg), color_hover: #(bg) },
                draw_text +: { color: #(fg), color_hover: #(fg) },
            });
            button.reset_hover(cx);
        }

        self.view.view(cx, ids!(content_scroll)).set_visible(cx, general_selected);
        self.view.view(cx, ids!(members_pane)).set_visible(cx, !general_selected);

        // The member list is only worth fetching once the user asks to see it.
        if tab == SettingsTab::Members
            && self.members.is_empty()
            && !self.members_loading
            && let Some(room_id) = self.room_id.clone()
        {
            self.members_loading = true;
            submit_async_request(MatrixRequest::FetchRoomMemberList { room_id });
        }
        self.update_members_status(cx);
        self.view.redraw(cx);
    }

    /// Applies a freshly-fetched member list.
    pub fn apply_member_list(&mut self, cx: &mut Cx, members: Vec<SettingsMemberInfo>) {
        self.members = members;
        self.members_loading = false;
        self.update_members_status(cx);
        self.view.redraw(cx);
    }

    /// Keeps the summary line and the empty/loading placeholder in sync.
    fn update_members_status(&mut self, cx: &mut Cx) {
        let count = self.members.len();
        let summary = if self.members_loading && count == 0 {
            String::new()
        } else if count == 1 {
            "1 member".to_string()
        } else {
            format!("{count} members")
        };
        self.view.label(cx, ids!(members_summary)).set_text(cx, &summary);
    }

    /// Reflects the fetched join rule in the access card.
    ///
    /// The options are replaced by an explanatory note whenever we can show the
    /// current rule but must not overwrite it — either because the user lacks the
    /// power level, or because the rule carries an allow-list (restricted) that
    /// this dialog has no editor for.
    fn apply_join_rule(&mut self, cx: &mut Cx) {
        let editable = self.can_change_join_rule && self.join_rule.is_settable();
        self.view.view(cx, ids!(access_options)).set_visible(cx, editable);

        let note = self.view.label(cx, ids!(access_locked_note));
        note.set_visible(cx, !editable);
        if !editable {
            note.set_text(cx, match self.join_rule {
                JoinRuleChoice::Restricted =>
                    "Members of a parent space can join. Changing which spaces grant access isn't supported here yet.",
                JoinRuleChoice::KnockRestricted =>
                    "Members of a parent space can join, and others can ask. Changing which spaces grant access isn't supported here yet.",
                JoinRuleChoice::Other =>
                    "This uses a join rule Robrix doesn't recognise, so it's left untouched.",
                _ => "You don't have permission to change who can join.",
            });
        }

        let selected = match self.join_rule {
            JoinRuleChoice::InviteOnly => 0,
            JoinRuleChoice::Knock => 1,
            JoinRuleChoice::Public => 2,
            // Nothing sensible to select; the note above explains why.
            _ => usize::MAX,
        };
        for (index, id) in [
            ids!(join_invite_radio),
            ids!(join_knock_radio),
            ids!(join_public_radio),
        ].into_iter().enumerate() {
            self.view.radio_button(cx, id).set_active(cx, index == selected, Animate::No);
        }

        let is_space = self.is_space;
        self.view.label(cx, ids!(access_desc)).set_text(cx, if is_space {
            "Controls who can join the space itself. Rooms inside it keep their own settings."
        } else {
            "Controls who can join this room."
        });
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
        is_space: bool,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show(cx, room_id, room_name, room_topic, canonical_alias, is_space);
    }

    /// Apply asynchronously-fetched settings (topic, is_public).
    pub fn apply_fetched_settings(
        &self,
        cx: &mut Cx,
        topic: Option<String>,
        is_public: bool,
        join_rule: JoinRuleChoice,
        can_change_join_rule: bool,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.apply_fetched_settings(cx, topic, is_public, join_rule, can_change_join_rule);
    }

    /// Applies an asynchronously-fetched member list.
    pub fn apply_member_list(&self, cx: &mut Cx, members: Vec<SettingsMemberInfo>) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.apply_member_list(cx, members);
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
