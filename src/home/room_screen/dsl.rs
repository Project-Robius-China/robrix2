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
