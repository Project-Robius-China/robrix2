
use makepad_widgets::*;
use url::Url;

use crate::{app::{AppState, AppUpdateAction, BotSettingsState}, home::navigation_tab_bar::{NavigationBarAction, get_own_profile}, i18n::{AppLanguage, I18nKey, language_dropdown_labels, tr, tr_fmt, tr_key}, persistence, proxy_config::{validate_proxy_url_for_user_input, ProxyInputError}, profile::user_profile::UserProfile, settings::{account_settings::AccountSettingsWidgetExt, app_preferences::AppPreferences, app_settings::AppSettingsWidgetExt, bot_settings::BotSettingsWidgetExt, translation_settings::TranslationSettingsWidgetExt}, shared::{expand_arrow::ExpandArrow, popup_list::{PopupKind, enqueue_popup_notification}, styles::{apply_neutral_button_style, apply_primary_button_style, apply_segment_selected_style, apply_segment_idle_style}}, sliding_sync::current_user_id, updater::{UpdateCheckOutcome, check_for_updates}};

const CONTRIBUTE_REPO_URL: &str = "https://github.com/Project-Robius-China/robrix2";

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.ICO_CHEVRON_RIGHT = crate_resource("self://resources/icons/chevron_right.svg")
    mod.widgets.ICO_CHEVRON_DOWN = crate_resource("self://resources/icons/chevron_down.svg")

    // A mobile settings segmented tab. Selection is driven by the `selected`
    // animator (teal text when active, gray when idle) instead of
    // `script_apply_eval!`, because these tabs are instantiated by an
    // AdaptiveView and eval-based restyling silently fails on such widgets
    // (their ScriptObject is zero — Makepad pitfall #40). The animator writes
    // the shader instance directly, so it works.
    let SettingsSegmentTab = RobrixNeutralIconButton {
        width: Fit, height: Fit,
        padding: Inset{top: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_MD), right: (SPACE_MD)}
        spacing: 0, margin: 0,
        icon_walk: Walk{width: 0, height: 0, margin: 0}
        draw_bg +: { color: #0000, color_hover: #0000, color_down: #0000, border_size: 0.0 }
        draw_text +: { color: (RBX_FG_SECONDARY) }
        text: ""
        animator +: {
            selected: {
                default: @off
                off: AnimatorState {
                    from: {all: Forward {duration: 0.0}}
                    apply: { draw_text: { color: (RBX_FG_SECONDARY) } }
                }
                on: AnimatorState {
                    from: {all: Forward {duration: 0.0}}
                    apply: { draw_text: { color: (RBX_ACCENT) } }
                }
            }
        }
    }

    // The main, top-level settings screen widget.
    mod.widgets.SettingsScreen = #(SettingsScreen::register_widget(vm)) {
        width: Fill, height: Fill,
        flow: Overlay

        // The settings screen is the SAME CachedWidget instance on desktop and
        // mobile (see home_screen.rs). To redesign ONLY the mobile layout without
        // touching desktop, the whole screen is wrapped in an AdaptiveView whose
        // `Desktop` variant is the original layout verbatim and whose `Mobile`
        // variant is the redesigned one. The variant is chosen by the same
        // `ViewModeOverride::variant_selector()` the NavigationTabBar uses.
        settings_adaptive := AdaptiveView {
            width: Fill, height: Fill

            Desktop := View {
            padding: Inset{top: (SPACE_SM), left: (SETTINGS_CONTENT_PADDING), right: (SETTINGS_CONTENT_PADDING), bottom: (SETTINGS_CONTENT_PADDING)},
            flow: Down

            // The settings header shows a title, with a close button to the right.
            settings_header := View {
                flow: Right,
                width: Fill, height: Fit
                margin: Inset{top: (SPACE_SM), left: (SPACE_XS), right: (SPACE_XS)}
                spacing: (SPACE_SM),

                settings_header_title := TitleLabel {
                    padding: 0,
                    margin: Inset{ left: 0, top: (SPACE_SM) },
                    text: "Add/Explore Rooms"
                    draw_text +: {
                        text_style: theme.font_regular {font_size: 18},
                    }
                }

                // The "X" close button on the top right
                close_button := RobrixNeutralIconButton {
                    width: Fit,
                    height: Fit,
                    spacing: 0,
                    margin: 0,
                    padding: (SPACE_LG),
                    draw_icon.svg: (ICON_CLOSE)
                    icon_walk: Walk{width: 12, height: 12}
                }
            }

            // Make sure the dividing line is aligned with the close_button
            LineH { padding: 0, margin: Inset{top: (SPACE_SM), bottom: (SPACE_SM)} }

            settings_category_cards := View {
                width: Fill, height: Fit
                flow: Flow.Right{wrap: true}
                align: Align{y: 0.5}
                spacing: (SPACE_SM)
                margin: Inset{left: (SPACE_XS), right: (SPACE_XS), bottom: (SPACE_SM)}

                category_account_button := RobrixNeutralIconButton {
                    width: Fit, height: Fit,
                    padding: Inset{top: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_MD), right: (SPACE_MD)}
                    spacing: 0,
                    icon_walk: Walk{width: 0, height: 0, margin: 0}
                    draw_bg +: { border_radius: (RADIUS_MD) }
                    text: "Account"
                }

                category_preferences_button := RobrixNeutralIconButton {
                    width: Fit, height: Fit,
                    padding: Inset{top: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_MD), right: (SPACE_MD)}
                    spacing: 0,
                    icon_walk: Walk{width: 0, height: 0, margin: 0}
                    draw_bg +: { border_radius: (RADIUS_MD) }
                    text: "Preferences"
                }

                category_devices_button := RobrixNeutralIconButton {
                    width: Fit, height: Fit,
                    padding: Inset{top: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_MD), right: (SPACE_MD)}
                    spacing: 0,
                    icon_walk: Walk{width: 0, height: 0, margin: 0}
                    draw_bg +: { border_radius: (RADIUS_MD) }
                    text: "Devices"
                }

                category_labs_button := RobrixNeutralIconButton {
                    width: Fit, height: Fit,
                    padding: Inset{top: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_MD), right: (SPACE_MD)}
                    spacing: 0,
                    icon_walk: Walk{width: 0, height: 0, margin: 0}
                    draw_bg +: { border_radius: (RADIUS_MD) }
                    text: "Labs"
                }

                category_contribute_button := RobrixNeutralIconButton {
                    width: Fit, height: Fit,
                    padding: Inset{top: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_MD), right: (SPACE_MD)}
                    spacing: 0,
                    icon_walk: Walk{width: 0, height: 0, margin: 0}
                    draw_bg +: { border_radius: (RADIUS_MD) }
                    text: "Contribute"
                }
            }

            settings_sections := PageFlip {
                width: Fill, height: Fill
                lazy_init: true,
                active_page: @account_settings_page

                account_settings_page := ScrollXYView {
                    width: Fill, height: Fill
                    flow: Down

                    account_settings_section := View {
                        width: Fill, height: Fit
                        flow: Down
                        account_settings := AccountSettings {}
                    }
                }

                preferences_settings_page := ScrollXYView {
                    width: Fill, height: Fill
                    flow: Down

                    preferences_settings_section := View {
                        width: Fill, height: Fit
                        flow: Down
                        spacing: (SPACE_SM)

                        app_settings := AppSettings {}

                        preferences_language_title := TitleLabel {
                            text: "Language"
                        }

                        // --- Language card ---
                        RoundedView {
                            width: Fill, height: Fit
                            flow: Down
                            padding: Inset{left: (SPACE_MD), right: (SPACE_MD), top: (SPACE_SM), bottom: (SPACE_MD)}
                            margin: Inset{top: (SPACE_XS)}
                            show_bg: true
                            draw_bg +: {
                                color: #F8F8FA
                                border_radius: (RADIUS_LG)
                            }

                            preferences_application_language_label := SubsectionLabel {
                                margin: Inset{top: 0, bottom: (SPACE_XS)}
                                text: "Application language"
                            }

                            // Custom language selector: button + popup list
                            // (replaces DropDown which has unsolvable arrow shader artifact)
                            language_selector_button := RoundedView {
                                width: 200, height: Fit
                                flow: Right
                                align: Align{y: 0.5}
                                padding: Inset{left: (SPACE_MD), right: (SPACE_SM), top: (SPACE_SM), bottom: (SPACE_SM)}
                                margin: Inset{left: (SPACE_XS), top: 2, bottom: 2}
                            cursor: MouseCursor.Hand
                            show_bg: true
                            draw_bg +: {
                                color: (COLOR_PRIMARY)
                                border_radius: (RADIUS_SM)
                                border_size: 1.0
                                border_color: (COLOR_DROPDOWN_BORDER)
                            }

                            language_selector_label := Label {
                                width: Fill, height: Fit
                                draw_text +: {
                                    color: (COLOR_DROPDOWN_TEXT)
                                    text_style: REGULAR_TEXT { font_size: 11 }
                                }
                                text: "English"
                            }

                            language_arrow := ExpandArrow {
                                width: 14, height: 14
                                draw_bg +: {
                                    color: instance((COLOR_DROPDOWN_ARROW))
                                }
                            }
                        }

                        language_popup := RoundedView {
                            visible: false
                            width: 200, height: Fit
                            flow: Down
                            padding: Inset{top: (SPACE_XS), bottom: (SPACE_XS)}
                            show_bg: true
                            new_batch: true
                            draw_bg +: {
                                color: (COLOR_PRIMARY)
                                border_radius: (RADIUS_MD)
                                border_size: 1.0
                                border_color: (COLOR_DROPDOWN_POPUP_BORDER)
                            }

                            lang_option_en := View {
                                width: Fill, height: 36
                                flow: Right
                                align: Align{y: 0.5}
                                padding: Inset{left: (SPACE_MD), right: (SPACE_MD)}
                                cursor: MouseCursor.Hand
                                show_bg: true
                                draw_bg +: { color: #0000 }
                                Label {
                                    width: Fit, height: Fit
                                    draw_text +: {
                                        color: (COLOR_DROPDOWN_TEXT)
                                        text_style: REGULAR_TEXT { font_size: 11 }
                                    }
                                    text: "English"
                                }
                            }
                            lang_option_zh := View {
                                width: Fill, height: 36
                                flow: Right
                                align: Align{y: 0.5}
                                padding: Inset{left: (SPACE_MD), right: (SPACE_MD)}
                                cursor: MouseCursor.Hand
                                show_bg: true
                                draw_bg +: { color: #0000 }
                                Label {
                                    width: Fit, height: Fit
                                    draw_text +: {
                                        color: (COLOR_DROPDOWN_TEXT)
                                        text_style: REGULAR_TEXT { font_size: 11 }
                                    }
                                    text: "简体中文"
                                }
                            }
                        }

                            preferences_language_hint_label := Label {
                                width: Fill
                                height: Fit
                                margin: Inset{right: (SPACE_SM), top: (SPACE_XS), bottom: (SPACE_XS)}
                                draw_text +: {
                                    color: (MESSAGE_TEXT_COLOR)
                                    text_style: REGULAR_TEXT { font_size: 10.5 }
                                }
                                text: "The app will reload after selecting another language"
                            }
                        }

                        preferences_proxy_title := TitleLabel {
                            text: "Proxy"
                        }

                        preferences_proxy_use_card := RoundedView {
                            width: Fill, height: Fit,
                            flow: Down
                            show_bg: true
                            draw_bg +: {
                                color: #F8F8FA
                                border_radius: (RADIUS_LG)
                            }
                            padding: Inset{left: (SPACE_MD), right: (SPACE_MD), top: (SPACE_SM), bottom: (SPACE_SM)}
                            margin: Inset{top: (SPACE_XS)}

                            View {
                                width: Fill, height: Fit
                                flow: Right
                                align: Align{x: 1.0, y: 0.5}

                                preferences_proxy_use_label := SubsectionLabel {
                                    margin: Inset{top: 0, bottom: 0}
                                    text: "Use proxy"
                                }

                                preferences_proxy_use_toggle := Toggle {
                                    width: Fit
                                    height: Fit
                                    padding: Inset{top: (SPACE_SM), right: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_SM)}
                                    text: ""
                                    active: false
                                    draw_bg +: {
                                        size: 20.0
                                        color_active: (COLOR_ACTIVE_PRIMARY)
                                        border_color_active: (COLOR_ACTIVE_PRIMARY)
                                        mark_color_active: #fff
                                    }
                                }
                            }

                            preferences_proxy_fields_section := View {
                                visible: false
                                width: Fill, height: Fit,
                                flow: Down
                                spacing: 0

                                preferences_proxy_address_row := View {
                                width: Fill, height: Fit,
                                flow: Right
                                align: Align{y: 0.5}
                                spacing: 8.0
                                padding: Inset{top: 8, bottom: 8}

                                preferences_proxy_address_label := Label {
                                    width: 90, height: Fit
                                    draw_text +: {
                                        color: (COLOR_TEXT)
                                        text_style: REGULAR_TEXT {font_size: 12}
                                    }
                                    text: "Address"
                                }

                                preferences_proxy_address_input := RobrixTextInput {
                                    width: Fill, height: Fit,
                                    flow: Right,
                                    empty_text: "127.0.0.1"
                                    padding: Inset{top: 5, bottom: 5, left: 10, right: 10}
                                }
                            }

                            preferences_proxy_port_row := View {
                                width: Fill, height: Fit,
                                flow: Right
                                align: Align{y: 0.5}
                                spacing: 8.0
                                padding: Inset{top: 8, bottom: 8}

                                preferences_proxy_port_label := Label {
                                    width: 90, height: Fit
                                    draw_text +: {
                                        color: (COLOR_TEXT)
                                        text_style: REGULAR_TEXT {font_size: 12}
                                    }
                                    text: "Port"
                                }

                                preferences_proxy_port_input := RobrixTextInput {
                                    width: Fill, height: Fit,
                                    flow: Right,
                                    empty_text: "7890"
                                    padding: Inset{top: 5, bottom: 5, left: 10, right: 10}
                                }
                            }

                            preferences_proxy_account_row := View {
                                width: Fill, height: Fit,
                                flow: Right
                                align: Align{y: 0.5}
                                spacing: 8.0
                                padding: Inset{top: 8, bottom: 8}

                                preferences_proxy_account_label := Label {
                                    width: 90, height: Fit
                                    draw_text +: {
                                        color: (COLOR_TEXT)
                                        text_style: REGULAR_TEXT {font_size: 12}
                                    }
                                    text: "Account"
                                }

                                preferences_proxy_account_input := RobrixTextInput {
                                    width: Fill, height: Fit,
                                    flow: Right,
                                    empty_text: ""
                                    padding: Inset{top: 5, bottom: 5, left: 10, right: 10}
                                }
                            }

                            preferences_proxy_password_row := View {
                                width: Fill, height: Fit,
                                flow: Right
                                align: Align{y: 0.5}
                                spacing: 8.0
                                padding: Inset{top: 8, bottom: 8}

                                preferences_proxy_password_label := Label {
                                    width: 90, height: Fit
                                    draw_text +: {
                                        color: (COLOR_TEXT)
                                        text_style: REGULAR_TEXT {font_size: 12}
                                    }
                                    text: "Password"
                                }

                                preferences_proxy_password_input := RobrixTextInput {
                                    width: Fill, height: Fit,
                                    flow: Right,
                                    empty_text: ""
                                    is_password: true,
                                    padding: Inset{top: 5, bottom: 5, left: 10, right: 10}
                                }
                            }
                            }

                            preferences_proxy_error_label := Label {
                                visible: false
                                width: Fill, height: Fit
                                margin: Inset{top: (SPACE_SM)}
                                draw_text +: {
                                    color: (COLOR_TEXT_WARNING_NOT_FOUND)
                                    text_style: REGULAR_TEXT {font_size: 11}
                                    wrap: Words
                                }
                                text: ""
                            }

                            preferences_proxy_save_button_row := View {
                                width: Fill, height: Fit
                                flow: Right
                                align: Align{x: 0.0, y: 0.5}
                                margin: Inset{top: (SPACE_SM)}

                                preferences_proxy_save_button := RobrixIconButton {
                                    width: Fit, height: Fit
                                    padding: Inset{left: (SPACE_MD), right: (SPACE_MD), top: (SPACE_SM), bottom: (SPACE_SM)}
                                    align: Align{x: 0.5, y: 0.5}
                                    text: "Save Proxy"
                                }
                            }
                        }
                    }
                }

                devices_settings_page := ScrollXYView {
                    width: Fill, height: Fill
                    flow: Down

                    devices_settings_section := View {
                        width: Fill, height: Fill
                        flow: Down
                        spacing: (SPACE_SM)
                        devices_settings := DevicesScreen {}
                    }
                }

                labs_settings_page := ScrollXYView {
                    width: Fill, height: Fill
                    flow: Down

                    labs_settings_section := View {
                        width: Fill, height: Fit
                        flow: Down
                        spacing: (SPACE_SM)

                        // --- Agents card (Agent Registry; Octos AppService config
                        // lives inside the "Add an agent" → Octos flow) ---
                        agent_settings := AgentSettings {}

                        // --- Translation card ---
                        RoundedView {
                            width: Fill, height: Fit
                            flow: Down
                            padding: Inset{left: (SPACE_MD), right: (SPACE_MD), top: (SPACE_SM), bottom: (SPACE_MD)}
                            show_bg: true
                            draw_bg +: {
                                color: #F8F8FA
                                border_radius: (RADIUS_LG)
                            }
                            translation_settings := TranslationSettings {}
                        }

                        // --- TSP card ---
                        tsp_settings_card := RoundedView {
                            width: Fill, height: Fit
                            flow: Down
                            padding: Inset{left: (SPACE_MD), right: (SPACE_MD), top: (SPACE_SM), bottom: (SPACE_MD)}
                            show_bg: true
                            draw_bg +: {
                                color: #F8F8FA
                                border_radius: (RADIUS_LG)
                            }
                            // The TSP wallet settings section.
                            tsp_settings_screen := TspSettingsScreen {}
                        }
                    }
                }

                contribute_settings_page := ScrollXYView {
                    width: Fill, height: Fill
                    flow: Down

                    contribute_settings_section := View {
                        width: Fill, height: Fit
                        flow: Down
                        spacing: (SPACE_SM)

                        RoundedView {
                            width: Fill, height: Fit
                            flow: Down
                            padding: Inset{left: (SPACE_MD), right: (SPACE_MD), top: (SPACE_SM), bottom: (SPACE_MD)}
                            margin: Inset{top: (SPACE_XS)}
                            show_bg: true
                            draw_bg +: {
                                color: #F8F8FA
                                border_radius: (RADIUS_LG)
                            }

                            contribute_title := SubsectionLabel {
                                margin: Inset{top: 0, bottom: (SPACE_XS)}
                                text: "Contribute"
                            }

                            contribute_description := Label {
                                width: Fill
                                height: Fit
                                margin: Inset{top: 0, bottom: 2}
                                draw_text +: {
                                    color: (COLOR_DESCRIPTION_TEXT)
                                    text_style: REGULAR_TEXT { font_size: 10.5 }
                                }
                                text: "Contribute to Robrix on GitHub:"
                            }

                            contribute_repo_link := LinkLabel {
                                width: Fit, height: Fit,
                                padding: Inset{left: (LINK_LABEL_LEFT_PAD)}
                                margin: 0
                                spacing: 0,
                                align: Align{x: 0.0}
                                icon_walk: Walk{width: 0, height: 0}
                                draw_text +: {
                                    text_style: REGULAR_TEXT { font_size: 10.5 }
                                    color: #x0000EE,
                                    color_hover: (COLOR_LINK_HOVER),
                                }
                                text: "https://github.com/Project-Robius-China/robrix2"
                            }
                        }

                        RoundedView {
                            width: Fill, height: Fit
                            flow: Down
                            padding: Inset{left: (SPACE_MD), right: (SPACE_MD), top: (SPACE_SM), bottom: (SPACE_MD)}
                            show_bg: true
                            draw_bg +: {
                                color: #F8F8FA
                                border_radius: (RADIUS_LG)
                            }

                            about_title := SubsectionLabel {
                                margin: Inset{top: 0, bottom: (SPACE_XS)}
                                text: "About Robrix"
                            }

                            about_description := Label {
                                width: Fill
                                height: Fit
                                margin: Inset{top: 0, bottom: 2}
                                draw_text +: {
                                    color: (COLOR_DESCRIPTION_TEXT)
                                    text_style: REGULAR_TEXT { font_size: 10.5 }
                                }
                                text: "Robrix is a multi-platform Matrix chat client built with Makepad and Robius."
                            }

                            LineH { margin: Inset{top: (SPACE_SM), bottom: (SPACE_XS)} }

                            contribute_current_version_label := Label {
                                width: Fill
                                height: Fit
                                margin: Inset{top: 0, bottom: 4}
                                draw_text +: {
                                    color: (MESSAGE_TEXT_COLOR)
                                    text_style: REGULAR_TEXT { font_size: 10.5 }
                                }
                                text: "Current version: 0.0.0"
                            }

                            contribute_check_update_button := RobrixIconButton {
                                width: Fit, height: Fit,
                                margin: Inset{left: (ICON_BUTTON_LEFT_PAD)}
                                padding: Inset{top: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_MD), right: (SPACE_MD)}
                                spacing: 0,
                                icon_walk: Walk{width: 0, height: 0, margin: 0}
                                draw_bg +: { border_radius: (RADIUS_MD) }
                                text: "Check for Updates"
                            }
                        }
                    }
                }
            }
        }

            // =================================================================
            // MOBILE variant — the redesigned settings screen (spec §5.1).
            // Page canvas + page title + segmented category tabs + a body
            // PageFlip that reuses the SAME page / sub-widget / control ids as
            // the Desktop variant, so all existing Rust logic drives it
            // unchanged (only the active AdaptiveView variant is instantiated).
            // =================================================================
            Mobile := View {
                width: Fill, height: Fill
                flow: Down
                show_bg: true
                draw_bg +: { color: (RBX_BG_CANVAS) }

                // ---- Header: page title + close button ----
                View {
                    width: Fill, height: Fit
                    flow: Right
                    align: Align{y: 0.5}
                    padding: Inset{top: (SPACE_LG), left: (SPACE_LG), right: (SPACE_MD), bottom: (SPACE_SM)}
                    spacing: (SPACE_SM)

                    m_settings_title := Label {
                        width: Fill, height: Fit
                        draw_text +: {
                            color: (RBX_FG_PRIMARY)
                            text_style: RBX_TEXT_PAGE_TITLE {}
                        }
                        text: "Settings"
                    }

                    // Reuses the `close_button` id so the existing close handler
                    // (click / back / Escape -> CloseSettings) works on mobile too.
                    close_button := RobrixNeutralIconButton {
                        width: Fit, height: Fit,
                        spacing: 0, margin: 0,
                        padding: (SPACE_SM),
                        draw_bg +: { border_radius: (RBX_RADIUS_PILL) }
                        draw_icon.svg: (ICON_CLOSE)
                        icon_walk: Walk{width: 14, height: 14}
                    }
                }

                // ---- Category tabs: text-only (transparent bg; selected = teal
                //      text, idle = gray text; recolored at runtime after the
                //      variant swap). Wraps on narrow widths. ----
                m_tabs_row := View {
                    width: Fill, height: Fit
                    flow: Flow.Right{wrap: true}
                    align: Align{y: 0.5}
                    spacing: (SPACE_XS)
                    padding: Inset{left: (SPACE_LG), right: (SPACE_LG), top: 0, bottom: (SPACE_SM)}

                    m_tab_account := SettingsSegmentTab { text: "Account" }
                    m_tab_preferences := SettingsSegmentTab { text: "Preferences" }
                    m_tab_devices := SettingsSegmentTab { text: "Devices" }
                    m_tab_labs := SettingsSegmentTab { text: "Labs" }
                    m_tab_contribute := SettingsSegmentTab { text: "Contribute" }
                }

                // hairline under the tabs
                View {
                    width: Fill, height: 1.0
                    show_bg: true
                    draw_bg +: { color: (RBX_STROKE_SOFT) }
                }

                // ---- Body: one page per category (same ids as Desktop) ----
                settings_sections := PageFlip {
                    width: Fill, height: Fill
                    lazy_init: true,
                    active_page: @account_settings_page

                    account_settings_page := ScrollXYView {
                        width: Fill, height: Fill
                        flow: Down
                        padding: Inset{left: (SPACE_LG), right: (SPACE_LG), top: (SPACE_MD), bottom: (SPACE_XXL)}
                        account_settings := AccountSettings {}
                    }

                    preferences_settings_page := ScrollXYView {
                        width: Fill, height: Fill
                        flow: Down
                        spacing: (SPACE_MD)
                        padding: Inset{left: (SPACE_LG), right: (SPACE_LG), top: (SPACE_MD), bottom: (SPACE_XXL)}
                        app_settings := AppSettings {}
                    }

                    devices_settings_page := ScrollXYView {
                        width: Fill, height: Fill
                        flow: Down
                        padding: Inset{left: (SPACE_LG), right: (SPACE_LG), top: (SPACE_MD), bottom: (SPACE_XXL)}
                        devices_settings := DevicesScreen {}
                    }

                    labs_settings_page := ScrollXYView {
                        width: Fill, height: Fill
                        flow: Down
                        spacing: (SPACE_MD)
                        padding: Inset{left: (SPACE_LG), right: (SPACE_LG), top: (SPACE_MD), bottom: (SPACE_XXL)}
                        agent_settings := AgentSettings {}
                        translation_settings := TranslationSettings {}
                    }

                    contribute_settings_page := ScrollXYView {
                        width: Fill, height: Fill
                        flow: Down
                        spacing: (SPACE_MD)
                        padding: Inset{left: (SPACE_LG), right: (SPACE_LG), top: (SPACE_MD), bottom: (SPACE_XXL)}

                        // About card (SectionCard recipe, spec §4.1)
                        RoundedView {
                            width: Fill, height: Fit
                            flow: Down
                            spacing: (SPACE_XS)
                            padding: (SPACE_LG)
                            show_bg: true
                            draw_bg +: {
                                color: (RBX_BG_SURFACE)
                                border_radius: (RBX_RADIUS_MD)
                                border_size: 1.0
                                border_color: (RBX_STROKE_SOFT)
                            }

                            about_title := Label {
                                width: Fill, height: Fit
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    text_style: RBX_TEXT_CARD_TITLE {}
                                }
                                text: "About Robrix"
                            }
                            about_description := Label {
                                width: Fill, height: Fit
                                draw_text +: {
                                    color: (RBX_FG_SECONDARY)
                                    text_style: RBX_TEXT_BODY {}
                                }
                                text: "Robrix is a multi-platform Matrix chat client built with Makepad and Robius."
                            }
                            contribute_current_version_label := Label {
                                width: Fill, height: Fit
                                margin: Inset{top: (SPACE_XS)}
                                draw_text +: {
                                    color: (RBX_FG_TERTIARY)
                                    text_style: RBX_TEXT_META {}
                                }
                                text: "Current version: 0.0.0"
                            }
                            contribute_check_update_button := RobrixIconButton {
                                width: Fit, height: Fit,
                                margin: Inset{top: (SPACE_XS)}
                                padding: Inset{top: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_MD), right: (SPACE_MD)}
                                spacing: 0,
                                icon_walk: Walk{width: 0, height: 0, margin: 0}
                                draw_bg +: {
                                    color: (RBX_ACCENT)
                                    color_hover: (RBX_ACCENT_HOVER)
                                    color_down: (RBX_ACCENT_PRESSED)
                                    border_radius: (RBX_RADIUS_SM)
                                }
                                draw_text +: {
                                    color: (RBX_FG_ON_ACCENT)
                                    color_hover: (RBX_FG_ON_ACCENT)
                                    color_down: (RBX_FG_ON_ACCENT)
                                }
                                text: "Check for Updates"
                            }
                        }

                        // Contribute card
                        RoundedView {
                            width: Fill, height: Fit
                            flow: Down
                            spacing: (SPACE_XS)
                            padding: (SPACE_LG)
                            show_bg: true
                            draw_bg +: {
                                color: (RBX_BG_SURFACE)
                                border_radius: (RBX_RADIUS_MD)
                                border_size: 1.0
                                border_color: (RBX_STROKE_SOFT)
                            }

                            contribute_title := Label {
                                width: Fill, height: Fit
                                draw_text +: {
                                    color: (RBX_FG_PRIMARY)
                                    text_style: RBX_TEXT_CARD_TITLE {}
                                }
                                text: "Contribute"
                            }
                            contribute_description := Label {
                                width: Fill, height: Fit
                                draw_text +: {
                                    color: (RBX_FG_SECONDARY)
                                    text_style: RBX_TEXT_BODY {}
                                }
                                text: "Contribute to Robrix on GitHub:"
                            }
                            contribute_repo_link := LinkLabel {
                                width: Fit, height: Fit,
                                margin: Inset{top: (SPACE_XS)}
                                spacing: 0,
                                align: Align{x: 0.0}
                                icon_walk: Walk{width: 0, height: 0}
                                draw_text +: {
                                    text_style: RBX_TEXT_BODY {}
                                    color: (RBX_LINK),
                                    color_hover: (RBX_ACCENT),
                                }
                                text: "https://github.com/Project-Robius-China/robrix2"
                            }
                        }
                    }
                }
            }
        }

        // We want all modals to appear in front of the settings screen.
        create_wallet_modal := Modal {
            content +: {
                create_wallet_modal_inner := CreateWalletModal {}
            }
        }

        create_did_modal := Modal {
            content +: {
                create_did_modal_inner := CreateDidModal {}
            }
        }

        add_agent_modal := Modal {
            content +: {
                add_agent_modal_inner := mod.widgets.AddAgentModal {}
            }
        }
    }
}


/// The top-level widget showing all app and user settings/preferences.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SettingsCategory {
    #[default]
    Account,
    Preferences,
    Devices,
    Labs,
    Contribute,
}

#[derive(Debug)]
enum SettingsUpdateAction {
    CheckFinished(UpdateCheckOutcome),
}

/// The top-level widget showing all app and user settings/preferences.
#[derive(Script, ScriptHook, Widget)]
pub struct SettingsScreen {
    #[deref] view: View,

    #[rust] selected_category: SettingsCategory,
    #[rust] app_language: AppLanguage,
    /// Fires the frame AFTER the Desktop/Mobile AdaptiveView swaps in its variant,
    /// so we can (re)apply tab styling + labels onto the now-instantiated widgets
    /// (set_variant_selector only takes effect on the next draw — see AdaptiveView).
    #[rust] resync_frame: NextFrame,
    #[rust] preferences_use_proxy_enabled: bool,
    #[rust] preferences_proxy_layout_width: f64,
    #[rust] language_popup_visible: bool,
    #[rust] is_update_checking: bool,
}

impl Widget for SettingsScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.sync_preferences_proxy_layout(cx);
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }
        self.sync_update_widgets_text(cx);
        self.view.handle_event(cx, event, scope);

        // After the AdaptiveView swaps Desktop<->Mobile, the new variant's
        // widgets exist only now — (re)apply their labels and tab styling.
        if self.resync_frame.is_event(event).is_some() {
            self.sync_app_language(cx);
            self.sync_selected_category(cx);
        }

        // Close the pane if:
        // 1. The close button is clicked,
        // 2. The back navigational gesture/action occurs (e.g., Back on Android),
        // 3. The escape key is pressed if this pane has key focus,
        // 4. The back mouse button is clicked within this view.
        let area = self.view.area();
        let close_pane = {
            matches!(
                event,
                Event::Actions(actions) if self.button(cx, ids!(close_button)).clicked(actions)
            )
            || event.back_pressed()
            || match event.hits(cx, area) {
                Hit::KeyUp(key) => key.key_code == KeyCode::Escape,
                Hit::FingerDown(_fde) => {
                    cx.set_key_focus(area);
                    false
                }
                _ => false,
            }
        };
        if close_pane {
            cx.action(NavigationBarAction::CloseSettings);
        }

        // Handle language selector button click
        {
            let selector = self.view.view(cx, ids!(language_selector_button));
            if let Hit::FingerUp(fe) = event.hits(cx, selector.area()) {
                if fe.is_over && fe.was_tap() {
                    self.language_popup_visible = !self.language_popup_visible;
                    self.view.view(cx, ids!(language_popup)).set_visible(cx, self.language_popup_visible);
                    self.update_language_button_text(cx);
                    self.redraw(cx);
                }
            }
        }

        // Handle language popup item selection via finger_up
        if self.language_popup_visible {
            let lang_options: &[(&[LiveId], usize)] = &[
                (&[live_id!(lang_option_en)], 0),
                (&[live_id!(lang_option_zh)], 1),
            ];
            for &(id_path, index) in lang_options {
                let item_view = self.view.view(cx, id_path);
                if let Hit::FingerUp(fe) = event.hits(cx, item_view.area()) {
                    if fe.is_over && fe.was_tap() {
                        self.language_popup_visible = false;
                        self.view.view(cx, &[live_id!(language_popup)]).set_visible(cx, false);
                        self.update_language_button_text(cx);

                        let selected_language = AppLanguage::from_dropdown_index(index);
                        if self.app_language != selected_language {
                            self.set_app_language(cx, selected_language);
                            if let Some(app_state) = scope.data.get_mut::<AppState>() {
                                if app_state.app_language != selected_language {
                                    app_state.app_language = selected_language;
                                    persist_app_state(app_state);
                                    enqueue_popup_notification(
                                        tr(selected_language, I18nKey::LanguageReloadHint),
                                        PopupKind::Info,
                                        Some(4.0),
                                    );
                                }
                            }
                        }
                        self.redraw(cx);
                        break;
                    }
                }
            }
        }

        if let Event::Actions(actions) = event {
            // Handle language selector click — moved to finger_up below

            if let Some(enabled) = self.view.check_box(cx, ids!(preferences_proxy_use_toggle)).changed(actions) {
                self.set_preferences_use_proxy_enabled(cx, enabled);
            }

            if self.view.button(cx, ids!(preferences_proxy_save_button)).clicked(actions) {
                let error_label = self.view.label(cx, ids!(preferences_proxy_error_label));
                match self.build_proxy_url_from_preferences(cx) {
                    Ok(proxy_url) => {
                        match crate::proxy_config::save_proxy_url(proxy_url.as_deref()) {
                            Ok(_) => {
                                error_label.set_visible(cx, false);
                                enqueue_popup_notification(
                                    tr_key(self.app_language, "settings.preferences.proxy.popup.saved").to_string(),
                                    PopupKind::Success,
                                    Some(4.0),
                                );
                            }
                            Err(proxy_error) => {
                                error_label.set_text(cx, &format!(
                                    "{}\n{}",
                                    tr_key(self.app_language, "settings.preferences.proxy.popup.invalid"),
                                    proxy_error,
                                ));
                                error_label.set_visible(cx, true);
                            }
                        }
                    }
                    Err(proxy_error) => {
                        error_label.set_text(cx, &format!(
                            "{}\n{}",
                            tr_key(self.app_language, "settings.preferences.proxy.popup.invalid"),
                            proxy_error,
                        ));
                        error_label.set_visible(cx, true);
                    }
                }
                self.redraw(cx);
            }

            if self.view.button(cx, ids!(category_account_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Account);
            }
            else if self.view.button(cx, ids!(category_preferences_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Preferences);
            }
            else if self.view.button(cx, ids!(category_devices_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Devices);
            }
            else if self.view.button(cx, ids!(category_labs_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Labs);
            }
            else if self.view.button(cx, ids!(category_contribute_button)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Contribute);
            }

            // Mobile segmented tabs (only the active AdaptiveView variant's
            // buttons exist, so the desktop & mobile handlers can coexist).
            if self.view.button(cx, ids!(m_tab_account)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Account);
            }
            else if self.view.button(cx, ids!(m_tab_preferences)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Preferences);
            }
            else if self.view.button(cx, ids!(m_tab_devices)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Devices);
            }
            else if self.view.button(cx, ids!(m_tab_labs)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Labs);
            }
            else if self.view.button(cx, ids!(m_tab_contribute)).clicked(actions) {
                self.set_selected_category(cx, SettingsCategory::Contribute);
            }

            if !self.is_update_checking && (
                self.view.button(cx, ids!(contribute_check_update_button)).clicked(actions)
            ) {
                self.set_update_checking(cx, true);
                cx.spawn_thread(move || {
                    let result = check_for_updates();
                    Cx::post_action(SettingsUpdateAction::CheckFinished(result));
                });
            }

            for action in actions {
                if let Some(crate::settings::app_preferences::AppPreferencesAction::ViewModeChanged(new_mode)) = action.downcast_ref() {
                    self.apply_settings_view_mode(cx, *new_mode);
                }
                if let HtmlLinkAction::Clicked { url, .. } = action.as_widget_action().cast() {
                    if url == CONTRIBUTE_REPO_URL {
                        if let Err(e) = robius_open::Uri::new(&url).open() {
                            error!("Failed to open URL {:?}. Error: {:?}", url, e);
                            enqueue_popup_notification(
                                tr_fmt(self.app_language, "room_screen.popup.open_url_failed", &[("url", url.as_str())]),
                                PopupKind::Error,
                                Some(10.0),
                            );
                        }
                    }
                }
                match action.downcast_ref() {
                    Some(SettingsUpdateAction::CheckFinished(result)) => {
                        self.set_update_checking(cx, false);
                        self.show_update_check_result(cx, result);
                    }
                    None => { }
                }
            }

            #[cfg(feature = "tsp")]
            {
                use crate::tsp::{
                    create_did_modal::CreateDidModalAction,
                    create_wallet_modal::CreateWalletModalAction,
                };

                for action in actions {
                    // Handle the create wallet modal being opened or closed.
                    match action.downcast_ref() {
                        Some(CreateWalletModalAction::Open) => {
                            use crate::tsp::create_wallet_modal::CreateWalletModalWidgetExt;
                            self.view.create_wallet_modal(cx, ids!(create_wallet_modal_inner)).show(cx);
                            self.view.modal(cx, ids!(create_wallet_modal)).open(cx);
                        }
                        Some(CreateWalletModalAction::Close) => {
                            self.view.modal(cx, ids!(create_wallet_modal)).close(cx);
                        }
                        None => { }
                    }

                    // Handle the create DID modal being opened or closed.
                    match action.downcast_ref() {
                        Some(CreateDidModalAction::Open) => {
                            use crate::tsp::create_did_modal::CreateDidModalWidgetExt;
                            self.view.create_did_modal(cx, ids!(create_did_modal_inner)).show(cx);
                            self.view.modal(cx, ids!(create_did_modal)).open(cx);
                        }
                        Some(CreateDidModalAction::Close) => {
                            self.view.modal(cx, ids!(create_did_modal)).close(cx);
                        }
                        None => { }
                    }
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

impl SettingsScreen {
    fn sync_preferences_proxy_layout(&mut self, cx: &mut Cx) {
        let rect = self.view.area().rect(cx);
        if rect.size.x <= 1.0 {
            return;
        }
        let available_width = (rect.size.x - 42.0).max(260.0);
        let card_width = available_width.min(360.0);
        if (self.preferences_proxy_layout_width - card_width).abs() <= 0.5 {
            return;
        }
        self.preferences_proxy_layout_width = card_width;
        if let Some(mut proxy_use_card) = self.view
            .child_by_path(ids!(preferences_proxy_use_card))
            .borrow_mut::<View>()
        {
            proxy_use_card.walk.width = Size::Fixed(card_width);
            proxy_use_card.redraw(cx);
        }
        if let Some(mut proxy_fields_section) = self.view
            .child_by_path(ids!(preferences_proxy_fields_section))
            .borrow_mut::<View>()
        {
            proxy_fields_section.walk.width = Size::Fixed(card_width);
            proxy_fields_section.redraw(cx);
        }
    }

    fn set_app_language(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        self.app_language = app_language;
        self.sync_app_language(cx);
    }

    fn sync_tsp_settings_card_visibility(&mut self, cx: &mut Cx) {
        self.view
            .view(cx, ids!(tsp_settings_card))
            .set_visible(cx, cfg!(feature = "tsp"));
    }

    fn sync_app_language(&mut self, cx: &mut Cx) {
        self.view
            .label(cx, ids!(settings_header_title))
            .set_text(cx, tr(self.app_language, I18nKey::AllSettingsTitle));
        self.view
            .button(cx, ids!(category_account_button))
            .set_text(cx, tr(self.app_language, I18nKey::SettingsCategoryAccount));
        self.view
            .button(cx, ids!(category_preferences_button))
            .set_text(cx, tr(self.app_language, I18nKey::SettingsCategoryPreferences));
        self.view
            .button(cx, ids!(category_labs_button))
            .set_text(cx, tr(self.app_language, I18nKey::SettingsCategoryLabs));
        self.view
            .button(cx, ids!(category_contribute_button))
            .set_text(cx, tr(self.app_language, I18nKey::SettingsCategoryContribute));
        // Mobile variant: page title + segmented category tabs (no-ops on the
        // desktop variant, where these ids don't exist). Devices keeps its
        // static label, matching the desktop category button.
        self.view
            .label(cx, ids!(m_settings_title))
            .set_text(cx, tr(self.app_language, I18nKey::AllSettingsTitle));
        self.view
            .button(cx, ids!(m_tab_account))
            .set_text(cx, tr(self.app_language, I18nKey::SettingsCategoryAccount));
        self.view
            .button(cx, ids!(m_tab_preferences))
            .set_text(cx, tr(self.app_language, I18nKey::SettingsCategoryPreferences));
        self.view
            .button(cx, ids!(m_tab_labs))
            .set_text(cx, tr(self.app_language, I18nKey::SettingsCategoryLabs));
        self.view
            .button(cx, ids!(m_tab_contribute))
            .set_text(cx, tr(self.app_language, I18nKey::SettingsCategoryContribute));
        self.view
            .label(cx, ids!(preferences_language_title))
            .set_text(cx, tr(self.app_language, I18nKey::LanguageTitle));
        self.view
            .label(cx, ids!(preferences_application_language_label))
            .set_text(cx, tr(self.app_language, I18nKey::ApplicationLanguageLabel));
        self.view
            .label(cx, ids!(preferences_language_hint_label))
            .set_text(cx, tr(self.app_language, I18nKey::LanguageReloadHint));
        self.view
            .label(cx, ids!(preferences_proxy_title))
            .set_text(cx, tr_key(self.app_language, "settings.preferences.proxy.title"));
        self.view
            .label(cx, ids!(preferences_proxy_use_label))
            .set_text(cx, tr_key(self.app_language, "settings.preferences.proxy.use_proxy"));
        self.view
            .label(cx, ids!(preferences_proxy_address_label))
            .set_text(cx, tr_key(self.app_language, "settings.preferences.proxy.address"));
        self.view
            .label(cx, ids!(preferences_proxy_port_label))
            .set_text(cx, tr_key(self.app_language, "settings.preferences.proxy.port"));
        self.view
            .label(cx, ids!(preferences_proxy_account_label))
            .set_text(cx, tr_key(self.app_language, "settings.preferences.proxy.account"));
        self.view
            .label(cx, ids!(preferences_proxy_password_label))
            .set_text(cx, tr_key(self.app_language, "settings.preferences.proxy.password"));
        self.view
            .text_input(cx, ids!(preferences_proxy_address_input))
            .set_empty_text(cx, tr_key(self.app_language, "settings.preferences.proxy.input.address").to_string());
        self.view
            .text_input(cx, ids!(preferences_proxy_port_input))
            .set_empty_text(cx, tr_key(self.app_language, "settings.preferences.proxy.input.port").to_string());
        self.view
            .text_input(cx, ids!(preferences_proxy_account_input))
            .set_empty_text(cx, tr_key(self.app_language, "settings.preferences.proxy.input.account").to_string());
        self.view
            .text_input(cx, ids!(preferences_proxy_password_input))
            .set_empty_text(cx, tr_key(self.app_language, "settings.preferences.proxy.input.password").to_string());
        self.view
            .button(cx, ids!(preferences_proxy_save_button))
            .set_text(cx, tr_key(self.app_language, "settings.preferences.proxy.button.save"));
        self.update_language_button_text(cx);
        self.view
            .account_settings(cx, ids!(account_settings))
            .set_app_language(cx, self.app_language);
        self.view
            .bot_settings(cx, ids!(bot_settings))
            .set_app_language(cx, self.app_language);
        self.view
            .translation_settings(cx, ids!(translation_settings))
            .set_app_language(cx, self.app_language);
        self.sync_tsp_settings_card_visibility(cx);
        self.view
            .label(cx, ids!(contribute_title))
            .set_text(cx, tr_key(self.app_language, "settings.contribute.title"));
        self.view
            .label(cx, ids!(contribute_description))
            .set_text(cx, tr_key(self.app_language, "settings.contribute.description"));
        let contribute_repo_link = self.view.link_label(cx, ids!(contribute_repo_link));
        contribute_repo_link.set_text(cx, CONTRIBUTE_REPO_URL);
        if let Some(mut contribute_repo_link) = contribute_repo_link.borrow_mut() {
            contribute_repo_link.url = CONTRIBUTE_REPO_URL.to_string();
        }
        self.view
            .label(cx, ids!(about_title))
            .set_text(cx, tr_key(self.app_language, "settings.about.title"));
        self.view
            .label(cx, ids!(about_description))
            .set_text(cx, tr_key(self.app_language, "settings.about.description"));
        self.sync_update_widgets_text(cx);
        self.view.redraw(cx);
    }

    fn set_preferences_use_proxy_enabled(&mut self, cx: &mut Cx, enabled: bool) {
        self.preferences_use_proxy_enabled = enabled;
        self.view
            .check_box(cx, ids!(preferences_proxy_use_toggle))
            .set_active(cx, enabled, Animate::No);
        self.view
            .view(cx, ids!(preferences_proxy_fields_section))
            .set_visible(cx, enabled);
        self.view
            .label(cx, ids!(preferences_proxy_error_label))
            .set_visible(cx, false);
        self.view.redraw(cx);
    }

    fn load_saved_proxy_to_preferences_form(&mut self, cx: &mut Cx) {
        let saved_proxy = crate::proxy_config::load_saved_proxy_url();
        let Some(saved_proxy) = saved_proxy else {
            self.set_preferences_use_proxy_enabled(cx, false);
            self.view.text_input(cx, ids!(preferences_proxy_address_input)).set_text(cx, "");
            self.view.text_input(cx, ids!(preferences_proxy_port_input)).set_text(cx, "");
            self.view.text_input(cx, ids!(preferences_proxy_account_input)).set_text(cx, "");
            self.view.text_input(cx, ids!(preferences_proxy_password_input)).set_text(cx, "");
            return;
        };

        let Ok(parsed_url) = Url::parse(&saved_proxy) else {
            self.set_preferences_use_proxy_enabled(cx, true);
            self.view.text_input(cx, ids!(preferences_proxy_address_input)).set_text(cx, &saved_proxy);
            self.view.text_input(cx, ids!(preferences_proxy_port_input)).set_text(cx, "");
            self.view.text_input(cx, ids!(preferences_proxy_account_input)).set_text(cx, "");
            self.view.text_input(cx, ids!(preferences_proxy_password_input)).set_text(cx, "");
            return;
        };

        self.set_preferences_use_proxy_enabled(cx, true);
        self.view
            .text_input(cx, ids!(preferences_proxy_address_input))
            .set_text(cx, parsed_url.host_str().unwrap_or_default());
        self.view
            .text_input(cx, ids!(preferences_proxy_port_input))
            .set_text(cx, &parsed_url.port().map(|p| p.to_string()).unwrap_or_default());
        self.view
            .text_input(cx, ids!(preferences_proxy_account_input))
            .set_text(cx, parsed_url.username());
        self.view
            .text_input(cx, ids!(preferences_proxy_password_input))
            .set_text(cx, parsed_url.password().unwrap_or_default());
    }

    fn build_proxy_url_from_preferences(&mut self, cx: &mut Cx) -> Result<Option<String>, String> {
        if !self.preferences_use_proxy_enabled {
            return Ok(None);
        }

        let address = self.view.text_input(cx, ids!(preferences_proxy_address_input)).text();
        let port_text = self.view.text_input(cx, ids!(preferences_proxy_port_input)).text();
        let account = self.view.text_input(cx, ids!(preferences_proxy_account_input)).text();
        let password = self.view.text_input(cx, ids!(preferences_proxy_password_input)).text();

        let address = address.trim().to_owned();
        let port_text = port_text.trim().to_owned();
        let account = account.trim().to_owned();
        let password = password.trim().to_owned();

        if address.is_empty() {
            return Err(tr_key(self.app_language, "settings.preferences.proxy.error.missing_address").to_string());
        }
        if port_text.is_empty() {
            return Err(tr_key(self.app_language, "settings.preferences.proxy.error.missing_port").to_string());
        }
        let port: u16 = port_text
            .parse()
            .map_err(|_| tr_key(self.app_language, "settings.preferences.proxy.error.invalid_port").to_string())?;

        let mut proxy_url = if address.contains("://") {
            Url::parse(&address)
                .map_err(|e| format!("Invalid proxy URL: {e}"))?
        } else {
            let mut url = Url::parse("http://127.0.0.1")
                .map_err(|e| format!("Failed to initialize proxy URL builder: {e}"))?;
            url.set_host(Some(&address))
                .map_err(|e| format!("Invalid proxy address `{address}`: {e}"))?;
            url
        };

        proxy_url
            .set_port(Some(port))
            .map_err(|()| format!("Invalid proxy port `{port}`"))?;

        if account.is_empty() {
            proxy_url
                .set_username("")
                .map_err(|()| String::from("Invalid proxy account value"))?;
            proxy_url
                .set_password(None)
                .map_err(|()| String::from("Invalid proxy password value"))?;
        } else {
            proxy_url
                .set_username(&account)
                .map_err(|()| String::from("Invalid proxy account value"))?;
            if password.is_empty() {
                proxy_url
                    .set_password(None)
                    .map_err(|()| String::from("Invalid proxy password value"))?;
            } else {
                proxy_url
                    .set_password(Some(&password))
                    .map_err(|()| String::from("Invalid proxy password value"))?;
            }
        }

        let proxy_url = proxy_url.to_string();
        validate_proxy_url_for_user_input(&proxy_url).map_err(|e| match e {
            ProxyInputError::InvalidHost(host) => tr_fmt(
                self.app_language,
                "settings.preferences.proxy.error.invalid_host",
                &[("host", host.as_str())],
            ),
            other => other.to_string(),
        })?;
        Ok(Some(proxy_url))
    }

    fn update_language_button_text(&mut self, cx: &mut Cx) {
        let labels = language_dropdown_labels(self.app_language);
        let selected_idx = self.app_language.dropdown_index();
        let selected_label = labels.get(selected_idx).cloned().unwrap_or_else(|| "English".to_string());
        self.view.label(cx, ids!(language_selector_label)).set_text(cx, &selected_label);

        // Toggle expand arrow direction
        if let Some(mut arrow) = self.view.child_by_path(ids!(language_arrow)).borrow_mut::<ExpandArrow>() {
            arrow.set_is_open(cx, self.language_popup_visible, Animate::Yes);
        }
    }

    /// Drives the `settings_adaptive` AdaptiveView's Desktop/Mobile choice from
    /// the app's view-mode override (mirrors `NavigationTabBar::apply_view_mode`),
    /// so a forced wide/narrow layout also flips the settings screen.
    fn apply_settings_view_mode(&mut self, cx: &mut Cx, mode: crate::settings::app_preferences::ViewModeOverride) {
        if let Some(mut adaptive) = self.view
            .child_by_path(ids!(settings_adaptive))
            .borrow_mut::<AdaptiveView>()
        {
            adaptive.set_variant_selector(mode.variant_selector());
        }
        // The variant only swaps in on the next draw; re-sync labels + tab
        // styling onto the freshly-instantiated variant on the frame after that.
        self.resync_frame = cx.new_next_frame();
        self.view.redraw(cx);
    }

    fn set_selected_category(&mut self, cx: &mut Cx, category: SettingsCategory) {
        self.selected_category = category;
        self.sync_selected_category(cx);
    }

    fn sync_selected_category(&mut self, cx: &mut Cx) {
        let show_account = self.selected_category == SettingsCategory::Account;
        let show_preferences = self.selected_category == SettingsCategory::Preferences;
        let show_devices = self.selected_category == SettingsCategory::Devices;
        let show_labs = self.selected_category == SettingsCategory::Labs;
        let show_contribute = self.selected_category == SettingsCategory::Contribute;

        self.view
            .page_flip(cx, ids!(settings_sections))
            .set_active_page(
                cx,
                if show_account {
                    id!(account_settings_page)
                } else if show_preferences {
                    id!(preferences_settings_page)
                } else if show_devices {
                    id!(devices_settings_page)
                } else if show_labs {
                    id!(labs_settings_page)
                } else {
                    id!(contribute_settings_page)
                },
            );
        self.sync_tsp_settings_card_visibility(cx);

        // The preferences page is lazy-init: its widgets don't exist until the
        // user first switches to it, so the saved proxy populated during
        // SettingsScreen::populate misses the input refs. Re-load here once the
        // page's widget tree is live so the proxy form reflects whatever was
        // saved from the login modal.
        if show_preferences {
            self.load_saved_proxy_to_preferences_form(cx);
            // `app_settings` lives in this same lazy-init page. The initial
            // `SettingsScreen::populate` ran while the preferences page wasn't
            // instantiated yet, so `app_settings(...).populate(...)` was a silent
            // no-op (empty ref). Re-populate now that the widget tree is live —
            // mirroring the proxy re-load above — so saved values are applied and
            // the (feature-gated) agent-chat section is revealed.
            let prefs = cx.global::<crate::settings::app_preferences::AppPreferencesGlobal>().0.clone();
            self.view.app_settings(cx, ids!(app_settings)).populate(cx, &prefs, self.app_language);
        }

        // Style only the active variant's tabs. effective_is_desktop matches the
        // AdaptiveView's Desktop/Mobile choice; the inactive variant's buttons
        // don't exist, so we skip them rather than styling empty refs.
        let is_desktop = crate::settings::app_preferences::effective_is_desktop(cx);
        if is_desktop {
            let mut category_account_button = self.view.button(cx, ids!(category_account_button));
            let mut category_preferences_button = self.view.button(cx, ids!(category_preferences_button));
            let mut category_devices_button = self.view.button(cx, ids!(category_devices_button));
            let mut category_labs_button = self.view.button(cx, ids!(category_labs_button));
            let mut category_contribute_button = self.view.button(cx, ids!(category_contribute_button));

            if show_account { apply_primary_button_style(cx, &mut category_account_button); } else { apply_neutral_button_style(cx, &mut category_account_button); }
            if show_preferences { apply_primary_button_style(cx, &mut category_preferences_button); } else { apply_neutral_button_style(cx, &mut category_preferences_button); }
            if show_devices { apply_primary_button_style(cx, &mut category_devices_button); } else { apply_neutral_button_style(cx, &mut category_devices_button); }
            if show_labs { apply_primary_button_style(cx, &mut category_labs_button); } else { apply_neutral_button_style(cx, &mut category_labs_button); }
            if show_contribute { apply_primary_button_style(cx, &mut category_contribute_button); } else { apply_neutral_button_style(cx, &mut category_contribute_button); }

            category_account_button.reset_hover(cx);
            category_preferences_button.reset_hover(cx);
            category_devices_button.reset_hover(cx);
            category_labs_button.reset_hover(cx);
            category_contribute_button.reset_hover(cx);
        } else {
            let mut m_tab_account = self.view.button(cx, ids!(m_tab_account));
            let mut m_tab_preferences = self.view.button(cx, ids!(m_tab_preferences));
            let mut m_tab_devices = self.view.button(cx, ids!(m_tab_devices));
            let mut m_tab_labs = self.view.button(cx, ids!(m_tab_labs));
            let mut m_tab_contribute = self.view.button(cx, ids!(m_tab_contribute));

            if show_account { apply_segment_selected_style(cx, &mut m_tab_account); } else { apply_segment_idle_style(cx, &mut m_tab_account); }
            if show_preferences { apply_segment_selected_style(cx, &mut m_tab_preferences); } else { apply_segment_idle_style(cx, &mut m_tab_preferences); }
            if show_devices { apply_segment_selected_style(cx, &mut m_tab_devices); } else { apply_segment_idle_style(cx, &mut m_tab_devices); }
            if show_labs { apply_segment_selected_style(cx, &mut m_tab_labs); } else { apply_segment_idle_style(cx, &mut m_tab_labs); }
            if show_contribute { apply_segment_selected_style(cx, &mut m_tab_contribute); } else { apply_segment_idle_style(cx, &mut m_tab_contribute); }

            m_tab_account.reset_hover(cx);
            m_tab_preferences.reset_hover(cx);
            m_tab_devices.reset_hover(cx);
            m_tab_labs.reset_hover(cx);
            m_tab_contribute.reset_hover(cx);
        }
        self.view.redraw(cx);
    }

    fn set_update_checking(&mut self, cx: &mut Cx, is_update_checking: bool) {
        self.is_update_checking = is_update_checking;
        self.sync_update_widgets_text(cx);
        self.view.redraw(cx);
    }

    fn sync_update_widgets_text(&mut self, cx: &mut Cx) {
        let current_version_text = tr_fmt(self.app_language, "settings.update.current_version", &[
            ("version", env!("CARGO_PKG_VERSION")),
        ]);
        self.view
            .label(cx, ids!(contribute_current_version_label))
            .set_text(cx, &current_version_text);
        let check_button_text = if self.is_update_checking {
            tr_key(self.app_language, "settings.update.button.checking")
        } else {
            tr_key(self.app_language, "settings.update.button.check")
        };
        self.view
            .button(cx, ids!(contribute_check_update_button))
            .set_text(cx, check_button_text);
    }

    fn show_update_check_result(&mut self, cx: &mut Cx, result: &UpdateCheckOutcome) {
        match result {
            UpdateCheckOutcome::UpToDate { current_version } => {
                enqueue_popup_notification(
                    tr_fmt(self.app_language, "settings.update.popup.latest", &[
                        ("version", current_version.as_str()),
                    ]),
                    PopupKind::Info,
                    Some(4.0),
                );
            }
            UpdateCheckOutcome::UpdateAvailable { current_version, latest_version } => {
                cx.action(AppUpdateAction::ShowUpdatePrompt {
                    current_version: current_version.clone(),
                    latest_version: latest_version.clone(),
                    from_auto_check: false,
                });
            }
            UpdateCheckOutcome::NotConfigured => {
                enqueue_popup_notification(
                    tr_key(self.app_language, "settings.update.popup.not_configured"),
                    PopupKind::Warning,
                    Some(4.0),
                );
            }
            UpdateCheckOutcome::UnsupportedPlatform => {
                enqueue_popup_notification(
                    tr_key(self.app_language, "settings.update.popup.unsupported"),
                    PopupKind::Warning,
                    Some(4.0),
                );
            }
            UpdateCheckOutcome::Error(error) => {
                enqueue_popup_notification(
                    tr_fmt(self.app_language, "settings.update.popup.failed", &[
                        ("error", error.as_str()),
                    ]),
                    PopupKind::Error,
                    Some(6.0),
                );
            }
        }
    }

    /// Fetches the current user's profile and uses it to populate the settings screen.
    pub fn populate(&mut self, cx: &mut Cx, own_profile: Option<UserProfile>, bot_settings: &BotSettingsState, translation_config: &crate::room::translation::TranslationConfig, app_prefs: &AppPreferences, app_language: AppLanguage) {
        // Ensure the AdaptiveView has selected the right Desktop/Mobile variant
        // before we populate it, so populate targets the live variant's widgets.
        self.apply_settings_view_mode(cx, app_prefs.view_mode);
        if let Some(profile) = own_profile.or_else(|| get_own_profile(cx)) {
            self.view.account_settings(cx, ids!(account_settings)).populate(cx, profile);
        } else {
            error!("Failed to get own profile for settings screen.");
        }
        self.view.app_settings(cx, ids!(app_settings)).populate(cx, app_prefs, app_language);
        self.view.bot_settings(cx, ids!(bot_settings)).populate(cx, bot_settings);
        self.load_saved_proxy_to_preferences_form(cx);
        self.view.translation_settings(cx, ids!(translation_settings)).populate(cx, translation_config);
        #[cfg(feature = "tsp")]
        if let Some(mut tsp_settings_screen) = self.view.child_by_path(ids!(tsp_settings_screen)).borrow_mut::<crate::tsp::tsp_settings_screen::TspSettingsScreen>() {
            tsp_settings_screen.prepare_for_display(cx, app_language);
        }
        self.set_app_language(cx, app_language);
        self.set_update_checking(cx, false);
        self.set_selected_category(cx, SettingsCategory::Account);
        self.sync_preferences_proxy_layout(cx);
        self.view.button(cx, ids!(close_button)).reset_hover(cx);
        cx.set_key_focus(self.view.area());
        self.redraw(cx);
    }
}

impl SettingsScreenRef {
    /// See [`SettingsScreen::populate()`].
    pub fn populate(&self, cx: &mut Cx, own_profile: Option<UserProfile>, bot_settings: &BotSettingsState, translation_config: &crate::room::translation::TranslationConfig, app_prefs: &AppPreferences, app_language: AppLanguage) {
        let Some(mut inner) = self.borrow_mut() else { return; };
        inner.populate(cx, own_profile, bot_settings, translation_config, app_prefs, app_language);
    }
}

fn persist_app_state(app_state: &AppState) {
    if let Some(user_id) = current_user_id() {
        if let Err(e) = persistence::save_app_state(app_state.clone(), user_id) {
            error!("Failed to persist app state after updating language setting. Error: {e}");
        }
    }
}
