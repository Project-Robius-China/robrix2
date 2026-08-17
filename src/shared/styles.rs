use makepad_widgets::*;

script_mod! {

    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.ICON_ADD              = crate_resource("self://resources/icons/add.svg")

    // Agent framework brand logos (Settings > Labs > Agent Access + Add-agent modal).
    // Bundled high-res transparent PNGs; the GPU downscales them into the small
    // tiles, so they stay crisp on high-DPI Android/iOS without @2x/@3x variants.
    mod.widgets.IMG_FW_OCTOS          = crate_resource("self://resources/img/agent_octos.png")
    mod.widgets.IMG_FW_HERMES         = crate_resource("self://resources/img/agent_hermes.png")
    mod.widgets.IMG_FW_OPENCLAW       = crate_resource("self://resources/img/agent_openclaw.png")
    mod.widgets.ICON_ADD_REACTION     = crate_resource("self://resources/icons/add_reaction.svg")
    mod.widgets.ICON_ADD_USER         = crate_resource("self://resources/icons/add_user.svg") // TODO: FIX
    mod.widgets.ICON_ADD_WALLET       = crate_resource("self://resources/icons/add_wallet.svg")
    mod.widgets.ICON_FORBIDDEN        = crate_resource("self://resources/icons/forbidden.svg")
    mod.widgets.ICON_CHECKMARK        = crate_resource("self://resources/icons/checkmark.svg")
    mod.widgets.ICON_CLOSE            = crate_resource("self://resources/icons/close.svg")
    mod.widgets.ICON_CLOUD_CHECKMARK  = crate_resource("self://resources/icons/cloud_checkmark.svg")
    mod.widgets.ICON_CLOUD_OFFLINE    = crate_resource("self://resources/icons/cloud_offline.svg")
    mod.widgets.ICON_ROTATE_CW        = crate_resource("self://resources/icons/rotate_right_fa.svg")
    mod.widgets.ICON_ROTATE_CCW       = crate_resource("self://resources/icons/rotate_left_fa.svg")
    mod.widgets.ICON_COPY             = crate_resource("self://resources/icons/copy.svg")
    mod.widgets.ICON_DOWNLOAD         = crate_resource("self://resources/icons/download.svg")
    mod.widgets.ICON_EDIT             = crate_resource("self://resources/icons/edit.svg")
    mod.widgets.ICON_EXTERNAL_LINK    = crate_resource("self://resources/icons/external_link.svg")
    mod.widgets.ICON_IMPORT           = crate_resource("self://resources/icons/import.svg") // TODO: FIX
    mod.widgets.ICON_GLOBE            = crate_resource("self://resources/icons/globe.svg")
    mod.widgets.ICON_HIERARCHY        = crate_resource("self://resources/icons/hierarchy.svg")
    mod.widgets.ICON_LOCK             = crate_resource("self://resources/icons/lock.svg")
    mod.widgets.ICON_HOME             = crate_resource("self://resources/icons/home.svg")
    mod.widgets.ICON_HTML_FILE        = crate_resource("self://resources/icons/html_file.svg")
    mod.widgets.ICON_INFO             = crate_resource("self://resources/icons/info.svg")
    mod.widgets.ICON_INVITE           = crate_resource("self://resources/icons/invite.svg")
    mod.widgets.ICON_JOIN_ROOM        = crate_resource("self://resources/icons/join_room.svg")
    mod.widgets.ICON_JUMP             = crate_resource("self://resources/icons/go_back.svg")
    mod.widgets.ICON_LOCK_FILLED      = crate_resource("self://resources/icons/lock_filled.svg")
    mod.widgets.ICON_LOCK_OPEN        = crate_resource("self://resources/icons/lock_open.svg")
    mod.widgets.ICON_LOGOUT           = crate_resource("self://resources/icons/logout.svg")
    mod.widgets.ICON_LINK             = crate_resource("self://resources/icons/link.svg")
    mod.widgets.ICON_PIN              = crate_resource("self://resources/icons/pin.svg")
    mod.widgets.ICON_REPLY            = crate_resource("self://resources/icons/reply.svg")
    mod.widgets.ICON_SEARCH           = crate_resource("self://resources/icons/search.svg")
    mod.widgets.ICON_THREADS          = crate_resource("self://resources/icons/double_chat.svg")
    mod.widgets.ICON_SEND             = crate_resource("self://resources/icon_send.svg")
    mod.widgets.ICON_SETTINGS         = crate_resource("self://resources/icons/settings.svg")
    mod.widgets.ICON_SQUARES          = crate_resource("self://resources/icons/squares_filled.svg")
    mod.widgets.ICON_TOMBSTONE        = crate_resource("self://resources/icons/tombstone.svg")
    mod.widgets.ICON_TRASH            = crate_resource("self://resources/icons/trash.svg")
    mod.widgets.ICON_TRIANGLE_DOWN    = crate_resource("self://resources/icons/triangle_down_fill.svg")
    mod.widgets.ICON_TRIANGLE_UP      = crate_resource("self://resources/icons/triangle_up_fill.svg")
    mod.widgets.ICON_UPLOAD           = crate_resource("self://resources/icons/upload.svg")
    mod.widgets.ICON_VIEW_SOURCE      = crate_resource("self://resources/icons/view_source.svg")
    mod.widgets.ICON_WARNING          = crate_resource("self://resources/icons/warning.svg")
    mod.widgets.ICON_ZOOM_IN          = crate_resource("self://resources/icons/zoom_in.svg")
    mod.widgets.ICON_ZOOM_OUT         = crate_resource("self://resources/icons/zoom_out.svg")
    mod.widgets.ICON_ADD_ATTACHMENT   = crate_resource("self://resources/icons/add_attachment.svg")
    mod.widgets.ICON_FILE             = crate_resource("self://resources/icons/file.svg")
    mod.widgets.ICON_ARROW_BACK       = crate_resource("self://resources/icons/arrow_back.svg")
    mod.widgets.ICON_SHIELD           = crate_resource("self://resources/icons/shield.svg")
    mod.widgets.ICON_MORE_VERT        = crate_resource("self://resources/icons/more_vert.svg")
    mod.widgets.ICON_CHEVRON_RIGHT    = crate_resource("self://resources/icons/chevron_right.svg")
    mod.widgets.ICON_STAR             = crate_resource("self://resources/icons/star.svg")
    mod.widgets.ICON_STAR_FILLED      = crate_resource("self://resources/icons/star_filled.svg")
    mod.widgets.ICON_ROBOT            = crate_resource("self://resources/icons/robot.svg")
    mod.widgets.ICON_PEOPLE           = crate_resource("self://resources/icons/people.svg")
    mod.widgets.ICON_DEVICE           = crate_resource("self://resources/icons/device.svg")

    // App-owned base text styles, so no text font ships in the bundle.
    //
    // These replace `theme.font_regular` / `theme.font_bold` everywhere in
    // Robrix. They are *new* styles rather than assignments back into the
    // `theme` namespace: assigning to `theme.font_*` from an app `script_mod!`
    // compiles and runs but leaves the font members unresolved, and all text
    // renders as nothing. Declaring a fresh `TextStyle` with an explicit
    // `FontFamily` into `mod.widgets` is the form that works — it is how the
    // code-block styles below already load their fonts. (A bare
    // `APP_FONT_REGULAR = ...` is an assignment to a nonexistent variable,
    // not a declaration, and fails scope resolution entirely.)
    //
    // `system_latin.ttf` and `system_cjk.ttc` are symlinks that build.rs
    // resolves per platform (San Francisco + PingFang on macOS — including
    // macOS 26+'s hvgl-only PingFang, which makepad renders through its
    // CoreText outline fallback), so the bytes stay the operating system's
    // and nothing here is redistributed. Emoji stays on the bundled Noto,
    // which is already in this repo.
    mod.widgets.APP_FONT_REGULAR = TextStyle{
        font_family: FontFamily{
            latin := FontMember{
                res: crate_resource("self://resources/fonts/system_latin.ttf")
                asc: -0.1
                desc: 0.0
            }
            chinese := FontMember{
                res: crate_resource("self://resources/fonts/system_cjk.ttc")
                asc: 0.0
                desc: 0.0
            }
            emoji := FontMember{
                res: crate_resource("self://resources/fonts/NotoColorEmoji.ttf")
                asc: 0.0
                desc: 0.0
            }
        }
    }

    // Both system faces are variable, so bold comes from the `wght` axis of
    // the same files (`weight: 700` below) — San Francisco natively via
    // ttf_parser, PingFang via makepad's CoreText fallback which carries the
    // variation onto its CTFont.
    mod.widgets.APP_FONT_BOLD = TextStyle{
        font_family: FontFamily{
            latin := FontMember{
                res: crate_resource("self://resources/fonts/system_latin.ttf")
                asc: -0.1
                desc: 0.0
                weight: 700
            }
            chinese := FontMember{
                res: crate_resource("self://resources/fonts/system_cjk.ttc")
                asc: 0.0
                desc: 0.0
                weight: 700
            }
            emoji := FontMember{
                res: crate_resource("self://resources/fonts/NotoColorEmoji.ttf")
                asc: 0.0
                desc: 0.0
            }
        }
    }

    mod.widgets.TITLE_TEXT = mod.widgets.APP_FONT_REGULAR {
        font_size: (13),
    }

    mod.widgets.REGULAR_TEXT = mod.widgets.APP_FONT_REGULAR {
        font_size: (10),
    }

    mod.widgets.BOLD_TEXT = mod.widgets.APP_FONT_BOLD {
        font_size: (13),
    }

    mod.widgets.TEXT_SUB = mod.widgets.APP_FONT_REGULAR {
        font_size: (10),
    }

    mod.widgets.USERNAME_FONT_SIZE = 11

    mod.widgets.USERNAME_TEXT_COLOR = #x2
    mod.widgets.USERNAME_TEXT_STYLE = mod.widgets.APP_FONT_BOLD {
        font_size: (mod.widgets.USERNAME_FONT_SIZE),
    }

    mod.widgets.COLOR_ROBRIX_PURPLE = #572DCC; // the purple color from the Robrix logo

    mod.widgets.COLOR_ROBRIX_CYAN = #05CDC7; // the cyan color from the Robrix logo

    mod.widgets.TYPING_NOTICE_TEXT_COLOR = #121570


    mod.widgets.MESSAGE_FONT_SIZE = 11
    mod.widgets.REDACTED_MESSAGE_FONT_SIZE = 10

    mod.widgets.MESSAGE_TEXT_COLOR = #x333
    // notices (automated messages from bots) use a lighter color
    mod.widgets.COLOR_MESSAGE_NOTICE_TEXT = #x888
    mod.widgets.MESSAGE_TEXT_LINE_SPACING = 1.3
    // This font should only be used for plaintext labels. Don't use this for Html content,
    // as the Html widget sets different fonts for different text styles (e.g., bold, italic).
    mod.widgets.MESSAGE_TEXT_STYLE = mod.widgets.APP_FONT_REGULAR {
        font_size: (mod.widgets.MESSAGE_FONT_SIZE),
        line_spacing: (mod.widgets.MESSAGE_TEXT_LINE_SPACING),
    }

    // Code blocks need a real monospace latin font for CodeView layout,
    // plus a Chinese fallback so mixed CJK comments remain readable.
    mod.widgets.MESSAGE_CODE_TEXT_STYLE = TextStyle {
        font_family: FontFamily{
            latin := FontMember{
                res: crate_resource("self://resources/fonts/LiberationMono-Regular.ttf")
                asc: 0.0
                desc: 0.0
            }
            chinese := FontMember{
                res: crate_resource("self://resources/fonts/system_cjk.ttc")
                asc: 0.0
                desc: 0.0
            }
            emoji := FontMember{
                res: crate_resource("self://resources/fonts/NotoColorEmoji.ttf")
                asc: 0.0
                desc: 0.0
            }
        }
        font_size: (mod.widgets.MESSAGE_FONT_SIZE),
        line_spacing: (mod.widgets.MESSAGE_TEXT_LINE_SPACING),
        top_drop: 0.21,
    }

    // Event source JSON benefits from a slightly looser code style than
    // bot markdown blocks, especially when CJK glyph fallback is involved.
    mod.widgets.EVENT_SOURCE_CODE_TEXT_STYLE = TextStyle {
        font_family: FontFamily{
            latin := FontMember{
                res: crate_resource("self://resources/fonts/LiberationMono-Regular.ttf")
                asc: 0.0
                desc: 0.0
            }
            chinese := FontMember{
                res: crate_resource("self://resources/fonts/system_cjk.ttc")
                asc: 0.0
                desc: 0.0
            }
            emoji := FontMember{
                res: crate_resource("self://resources/fonts/NotoColorEmoji.ttf")
                asc: 0.0
                desc: 0.0
            }
        }
        font_size: 11.0,
        line_spacing: 1.58,
        top_drop: 0.18,
    }

    mod.widgets.MESSAGE_REPLY_PREVIEW_FONT_SIZE = 9.5



    mod.widgets.SMALL_STATE_FONT_SIZE = 9.0


    mod.widgets.SMALL_STATE_TEXT_COLOR = #x888
    mod.widgets.SMALL_STATE_TEXT_STYLE = mod.widgets.APP_FONT_REGULAR {
        font_size: (mod.widgets.SMALL_STATE_FONT_SIZE),
    }

    mod.widgets.TIMESTAMP_FONT_SIZE = 8.5

    mod.widgets.TIMESTAMP_TEXT_COLOR = #x999
    mod.widgets.TIMESTAMP_TEXT_STYLE = mod.widgets.APP_FONT_REGULAR {
        font_size: (mod.widgets.TIMESTAMP_FONT_SIZE),
    }

    mod.widgets.ROOM_NAME_TEXT_COLOR = #x0

    mod.widgets.COLOR_META = #xccc

    mod.widgets.COLOR_DIVIDER = #00000018

    mod.widgets.COLOR_DIVIDER_DARK = #00000044

    mod.widgets.COLOR_FG_ACCEPT_GREEN = #138808
    mod.widgets.COLOR_BG_ACCEPT_GREEN = #F0FFF0
    mod.widgets.COLOR_FG_DANGER_RED = #DC0005
    mod.widgets.COLOR_BG_DANGER_RED = #FFF0F0
    mod.widgets.COLOR_FG_DISABLED = #B3B3B3
    mod.widgets.COLOR_BG_DISABLED = #E0E0E0
    // Informational accent — it only ever shared a value with the retired legacy
    // primary. Now the system's own info blue (literal mirroring RBX_INFO_FG;
    // see the registration-order note further down), so it stays blue while the
    // primary moves to teal.
    mod.widgets.COLOR_INFO_BLUE = #1C67B0
    mod.widgets.COLOR_WARNING_YELLOW = #fcdb03
    mod.widgets.COLOR_TEXT_WARNING_NOT_FOUND = #953800

    // mod.widgets.COLOR_SELECT_TEXT = #A6CDFE
    // mod.widgets.COLOR_SELECT_TEXT = #B5D8FE
    // mod.widgets.COLOR_SELECT_TEXT = #6BB1FD88 // results in #B5D8FE when mixed halfway with white
    // mod.widgets.COLOR_SELECT_TEXT = #57A3FB44
    // 0x4C is ~30% opacity , which results in #B5D8FE when atop pure white
    // But i like the look of 0x33 20% opacity a little better.
    mod.widgets.COLOR_SELECT_TEXT = #087DFC33
    // mod.widgets.COLOR_SELECT_TEXT = #4D9BFD88 // results in #A6CDFE when mixed halfway with white

    mod.widgets.COLOR_PRIMARY = #ffffff

    // Was a stray `#fefefe` that belonged to no ramp; every use is a near-white
    // panel background or border, so it takes the surface value instead (a 0.4%
    // shift, invisible in place).
    // NOTE: this file is registered BEFORE design_tokens.rs (see shared/mod.rs),
    // so it cannot reference `RBX_*` — doing so silently fails to resolve at
    // runtime and the property falls back to a grey default. Keep the literal,
    // mirroring RBX_BG_SURFACE.
    mod.widgets.COLOR_PRIMARY_DARKER = #ffffff
    mod.widgets.COLOR_SECONDARY = #E3E3E3
    mod.widgets.COLOR_SECONDARY_DARKER = #C8C8C8

    // The primary/CTA/focus colour. Was the legacy bright blue `#0f88fe`; now the
    // accent teal, completing the migration design_tokens.rs describes. Every one
    // of the ~40 call sites means "primary", "active" or "focus", which is
    // exactly what the accent is defined to be, so they all move together —
    // migrating a screen at a time would have left blue and teal side by side for
    // as long as the migration ran.
    // Literals mirroring RBX_ACCENT / RBX_ACCENT_HOVER: this file is registered
    // before design_tokens.rs, so `RBX_*` is not resolvable here (see the note on
    // COLOR_PRIMARY_DARKER above).
    mod.widgets.COLOR_ACTIVE_PRIMARY = #0D7988

    mod.widgets.COLOR_ACTIVE_PRIMARY_DARKER = #0A6675

    mod.widgets.COLOR_BG_PREVIEW = #F0F5FF

    mod.widgets.COLOR_BG_PREVIEW_HOVER = #CDEDDF

    mod.widgets.COLOR_AVATAR_BG = #52b2ac

    mod.widgets.COLOR_AVATAR_BG_IDLE = #d8d8d8


    // Unread badge fills. A mention keeps the conventional red, but the
    // system's red rather than pure #FF0000 — the latter was the most saturated
    // pixel in the app and appeared nowhere else. "Marked unread" is a user
    // action, so it takes the functional accent instead of the logo cyan (the
    // brand ramp is reserved for brand entry points, per design_tokens.rs).
    // Literals mirroring RBX_DANGER_FG / RBX_ACCENT / RBX_FG_TERTIARY — this file
    // is registered before design_tokens.rs, so `RBX_*` is not resolvable here
    // (see the note on COLOR_PRIMARY_DARKER above). The Rust consts below do
    // reference the tokens directly, which keeps the two sides tied together.
    mod.widgets.COLOR_UNREAD_BADGE_MENTIONS = #B93429;
    mod.widgets.COLOR_UNREAD_BADGE_MARKED = #0D7988;
    mod.widgets.COLOR_UNREAD_BADGE_MESSAGES = #687283


    mod.widgets.COLOR_TEXT_IDLE = #d8d8d8


    mod.widgets.COLOR_TEXT = #1C274C
    mod.widgets.COLOR_TEXT_INPUT_IDLE = #d8d8d8

    mod.widgets.COLOR_TRANSPARENT = #00000000

    mod.widgets.COLOR_WARNING = #fcdb03

    mod.widgets.COLOR_LINK_HOVER = #21B070


    // This is chosen to nicely fit the 3 window chrome buttons on macOS
    mod.widgets.NAVIGATION_TAB_BAR_SIZE = 76
    mod.widgets.NAVIGATION_TAB_BAR_AVATAR_SIZE = (mod.widgets.NAVIGATION_TAB_BAR_SIZE * 0.65)
    mod.widgets.NAVIGATION_TAB_BAR_AVATAR_FONT_SIZE = (mod.widgets.NAVIGATION_TAB_BAR_AVATAR_SIZE * 0.4)


    mod.widgets.COLOR_NAVIGATION_TAB_FG = (mod.widgets.COLOR_TEXT)
    mod.widgets.COLOR_NAVIGATION_TAB_FG_HOVER = (mod.widgets.COLOR_TEXT)
    mod.widgets.COLOR_NAVIGATION_TAB_FG_ACTIVE = (mod.widgets.COLOR_TEXT)
    mod.widgets.COLOR_NAVIGATION_TAB_BG = (mod.widgets.COLOR_SECONDARY)
    mod.widgets.COLOR_NAVIGATION_TAB_BG_HOVER = (mod.widgets.COLOR_SECONDARY * 0.85)
    mod.widgets.COLOR_NAVIGATION_TAB_BG_ACTIVE = #9

    // Layout spacing constants (4px grid)
    mod.widgets.SPACE_XS  = 4
    mod.widgets.SPACE_SM  = 8
    mod.widgets.SPACE_MD  = 12
    mod.widgets.SPACE_LG  = 16
    mod.widgets.SPACE_XL  = 20
    mod.widgets.SPACE_XXL = 24

    // Border radius constants
    mod.widgets.RADIUS_SM = 4.0
    mod.widgets.RADIUS_MD = 6.0
    mod.widgets.RADIUS_LG = 8.0

    // Settings screen colors
    mod.widgets.COLOR_ACCOUNT_ACTIVE_BG = #3B8CFF  // softer blue for active account bar
    mod.widgets.COLOR_DROPDOWN_TEXT = #x333333        // text in dropdown selectors
    mod.widgets.COLOR_DROPDOWN_BORDER = #xC8D9F2      // dropdown border (light blue-gray)
    mod.widgets.COLOR_DROPDOWN_POPUP_BORDER = #xD3E1F6 // popup border (slightly lighter)
    mod.widgets.COLOR_DROPDOWN_ARROW = #x888888        // dropdown arrow icon
    mod.widgets.COLOR_INACTIVE_BORDER = #xBBBBBB       // inactive account entry border
    mod.widgets.COLOR_DESCRIPTION_TEXT = #x7A7A7A      // secondary description text
    mod.widgets.COLOR_FIELD_LABEL = #x555555           // form field labels
    mod.widgets.COLOR_DISABLED_TEXT = #x999999          // disabled/inactive state text

    // Settings screen layout
    mod.widgets.SETTINGS_CONTENT_PADDING = 16
    mod.widgets.SETTINGS_BUTTON_HEIGHT = 36

    // The font size used for regular (non-title, non-subsection) text
    // within any settings screen (e.g., dropdown labels, radio/toggle
    // labels, inline helper text inside a control).
    mod.widgets.SETTINGS_REGULAR_FONT_SIZE = 11
    mod.widgets.SETTINGS_REGULAR_TEXT_STYLE = mod.widgets.APP_FONT_REGULAR {
        font_size: (mod.widgets.SETTINGS_REGULAR_FONT_SIZE),
    }

    // Text alignment compensation for non-Label widgets (LinkLabel, IconButton)
    // whose internal rendering origin differs from plain Label.
    mod.widgets.LINK_LABEL_LEFT_PAD = 6
    mod.widgets.ICON_BUTTON_LEFT_PAD = 4

    mod.widgets.COLOR_IMAGE_VIEWER_BACKGROUND = #333333CC // 80% Opacity

    mod.widgets.COLOR_IMAGE_VIEWER_META_BACKGROUND = #E8E8E8

    // A text input widget styled for Robrix.
    mod.widgets.RobrixTextInput = TextInput {
        width: Fill, height: Fit
        flow: Flow.Right{wrap: true},
        align: Align{y: 0.5}
        margin: 0,
        padding: 10,

        // For multiline text inputs, we want to show a light-colored scroll bar.
        scroll_bar +: {
            draw_bg +: {
                color: #00000040
                color_hover: #00000060
                color_drag: #00000080
            }
        }

        draw_bg +: {
            border_radius: 4.0 // was previously 2.0
            border_size: 1.0

            color: (mod.widgets.COLOR_PRIMARY)
            color_hover: (mod.widgets.COLOR_PRIMARY)
            color_focus: (mod.widgets.COLOR_PRIMARY)
            color_down: (mod.widgets.COLOR_PRIMARY)
            color_empty: (mod.widgets.COLOR_PRIMARY)
            color_disabled: (mod.widgets.COLOR_BG_DISABLED)

            border_color: (mod.widgets.COLOR_SECONDARY_DARKER)
            border_color_hover: (mod.widgets.COLOR_ACTIVE_PRIMARY)
            border_color_focus: (mod.widgets.COLOR_ACTIVE_PRIMARY_DARKER)
            border_color_down: (mod.widgets.COLOR_ACTIVE_PRIMARY_DARKER)
            border_color_empty: (mod.widgets.COLOR_SECONDARY_DARKER)
            border_color_disabled: (mod.widgets.COLOR_FG_DISABLED)

            color_2: vec4(-1.0, -1.0, -1.0, -1.0) // don't use color_2*
            border_color_2: vec4(-1.0, -1.0, -1.0, -1.0) // don't use border_color_2*
        }

        draw_selection +: {
            color: mod.widgets.COLOR_SELECT_TEXT
            // color: mix(mod.widgets.COLOR_BG_DISABLED, mod.widgets.COLOR_SELECT_TEXT, 0.5)
            color_hover:  (mod.widgets.COLOR_SELECT_TEXT)
            color_focus:  (mod.widgets.COLOR_SELECT_TEXT)
            color_down:  (mod.widgets.COLOR_SELECT_TEXT)
            color_empty:  (mod.widgets.COLOR_SELECT_TEXT)
            color_disabled: (mod.widgets.COLOR_SELECT_TEXT)
        }

        draw_cursor +: {
            color: (mod.widgets.MESSAGE_TEXT_COLOR)
        }

        draw_text +: {
            color: (mod.widgets.MESSAGE_TEXT_COLOR),
            color_hover: (mod.widgets.MESSAGE_TEXT_COLOR),
            color_focus: (mod.widgets.MESSAGE_TEXT_COLOR),
            color_down: (mod.widgets.MESSAGE_TEXT_COLOR),
            color_disabled: (mod.widgets.COLOR_FG_DISABLED),
            color_empty: #B,
            color_empty_hover: #9,
            color_empty_focus: #9,

            text_style: mod.widgets.MESSAGE_TEXT_STYLE {},
        }
    }
}


/// #FFFFFF
pub const COLOR_PRIMARY:               Vec4 = vec4(1.0, 1.0, 1.0, 1.0);
/// The primary/CTA/focus colour, now the accent teal (was the legacy `#0F88FE`).
/// The Rust side has no registration-order constraint, so it names the tokens
/// directly and stays tied to the DSL literals above by construction.
pub const COLOR_ACTIVE_PRIMARY:        Vec4 = crate::shared::design_tokens::RBX_ACCENT;
pub const COLOR_ACTIVE_PRIMARY_DARKER: Vec4 = crate::shared::design_tokens::RBX_ACCENT_HOVER;
/// #138808
pub const COLOR_FG_ACCEPT_GREEN:       Vec4 = vec4(0.074, 0.533, 0.031, 1.0);
/// #F0FFF0
pub const COLOR_BG_ACCEPT_GREEN:       Vec4 = vec4(0.941, 1.0, 0.941, 1.0);
/// #B3B3B3
pub const COLOR_FG_DISABLED:           Vec4 = vec4(0.7, 0.7, 0.7, 1.0);
/// #E0E0E0
pub const COLOR_BG_DISABLED:           Vec4 = vec4(0.878, 0.878, 0.878, 1.0);
/// #DC0005
pub const COLOR_FG_DANGER_RED:         Vec4 = vec4(0.863, 0.0, 0.02, 1.0);
/// #FFF0F0
pub const COLOR_BG_DANGER_RED:         Vec4 = vec4(1.0, 0.941, 0.941, 1.0);
/// #572DCC
pub const COLOR_ROBRIX_PURPLE:         Vec4 = vec4(0.341, 0.176, 0.8, 1.0);
/// #05CDC7
pub const COLOR_ROBRIX_CYAN:           Vec4 = vec4(0.031, 0.804, 0.78, 1.0);
// Keep these in sync with the DSL definitions above.
/// #B93429 — mention badge (`RBX_DANGER_FG`).
pub const COLOR_UNREAD_BADGE_MENTIONS: Vec4 = crate::shared::design_tokens::RBX_DANGER_FG;
/// #0D7988 — marked-unread badge (`RBX_ACCENT`).
pub const COLOR_UNREAD_BADGE_MARKED:   Vec4 = crate::shared::design_tokens::RBX_ACCENT;
/// #687283 — plain unread-count badge (`RBX_FG_TERTIARY`).
pub const COLOR_UNREAD_BADGE_MESSAGES: Vec4 = crate::shared::design_tokens::RBX_FG_TERTIARY;
/// #FF6e00
pub const COLOR_UNKNOWN_ROOM_AVATAR:   Vec4 = vec4(1.0, 0.431, 0.0, 1.0);
/// #888888
pub const COLOR_MESSAGE_NOTICE_TEXT:   Vec4 = vec4(0.5, 0.5, 0.5, 1.0);
/// #953800
pub const COLOR_TEXT_WARNING_NOT_FOUND: Vec4 = vec4(0.584, 0.219, 0.0, 1.0);
/// #F0F5FF
pub const COLOR_BG_PREVIEW:            Vec4 = vec4(0.941, 0.961, 1.0, 1.0);
/// #CDEDDF
pub const COLOR_BG_PREVIEW_HOVER:      Vec4 = vec4(0.804, 0.929, 0.875, 1.0);

/// Applies positive (green) button styling to the given button.
pub fn apply_positive_button_style(cx: &mut Cx, button: &mut ButtonRef) {
    script_apply_eval!(cx, button, {
        draw_bg +: {
            border_color: mod.widgets.COLOR_FG_ACCEPT_GREEN,
            color: mod.widgets.COLOR_BG_ACCEPT_GREEN,
            color_hover: #D4EED4,
            color_down: #B8E0B8,
        }
        draw_text +: {
            color: mod.widgets.COLOR_FG_ACCEPT_GREEN,
            color_hover: mod.widgets.COLOR_FG_ACCEPT_GREEN,
            color_down: mod.widgets.COLOR_FG_ACCEPT_GREEN,
        }
        draw_icon +: {
            color: mod.widgets.COLOR_FG_ACCEPT_GREEN,
        }
    });
}

/// Applies negative (red) button styling to the given button.
pub fn apply_negative_button_style(cx: &mut Cx, button: &mut ButtonRef) {
    script_apply_eval!(cx, button, {
        draw_bg +: {
            border_color: mod.widgets.COLOR_FG_DANGER_RED,
            color: mod.widgets.COLOR_BG_DANGER_RED,
            color_hover: #F0D4D4,
            color_down: #E0B8B8,
        }
        draw_text +: {
            color: mod.widgets.COLOR_FG_DANGER_RED,
            color_hover: mod.widgets.COLOR_FG_DANGER_RED,
            color_down: mod.widgets.COLOR_FG_DANGER_RED,
        }
        draw_icon +: {
            color: mod.widgets.COLOR_FG_DANGER_RED,
        }
    });
}

/// Applies neutral (gray) button styling to the given button.
pub fn apply_neutral_button_style(cx: &mut Cx, button: &mut ButtonRef) {
    script_apply_eval!(cx, button, {
        draw_bg +: {
            border_color: mod.widgets.COLOR_BG_DISABLED,
            color: mod.widgets.COLOR_SECONDARY,
            color_hover: #D0D0D0,
            color_down: #C0C0C0,
        }
        draw_text +: {
            color: mod.widgets.COLOR_TEXT,
            color_hover: mod.widgets.COLOR_TEXT,
            color_down: mod.widgets.COLOR_TEXT,
        }
        draw_icon +: {
            color: mod.widgets.COLOR_TEXT,
        }
    });
}

/// Applies the primary (blue) button styling to the given button.
pub fn apply_primary_button_style(cx: &mut Cx, button: &mut ButtonRef) {
    script_apply_eval!(cx, button, {
        draw_bg +: {
            color: mod.widgets.COLOR_ACTIVE_PRIMARY,
            color_hover: mod.widgets.COLOR_ACTIVE_PRIMARY_DARKER,
            color_down: #0C5DAA,
            border_color: #0000,
            border_color_hover: #0000,
            border_color_down: #0000,
        }
        draw_text +: {
            color: mod.widgets.COLOR_PRIMARY,
            color_hover: mod.widgets.COLOR_PRIMARY,
            color_down: mod.widgets.COLOR_PRIMARY,
        }
        draw_icon +: {
            color: mod.widgets.COLOR_PRIMARY,
        }
    });
}

#[cfg(test)]
mod cjk_font_tests {
    use std::path::Path;

    /// The single path the DSL's `chinese` font members point at. `build.rs`
    /// links it to a system CJK font where one is found (PingFang on macOS,
    /// YaHei on Windows, fontconfig's answer on Linux), or to the bundled
    /// LXGWWenKai in `bundled_fonts/` otherwise — and always to the bundled
    /// face when `ROBRIX_BUNDLED_FONTS=1` was set at build time.
    const CJK_FONT: &str = "resources/fonts/system_cjk.ttc";
    const CJK_BUNDLED: &str = "bundled_fonts/LXGWWenKaiRegular.ttf";
    const LATIN_BUNDLED: &str = "resources/fonts/LiberationMono-Regular.ttf";

    /// Same for the Latin UI face (`system_latin.ttf`).
    const LATIN_FONT: &str = "resources/fonts/system_latin.ttf";

    /// `path` must exist (following symlinks) and start with a font magic
    /// `ttf_parser::Face::parse` accepts: a collection (`ttcf`), TrueType
    /// outlines (0x00010000), or CFF (`OTTO`).
    fn assert_is_real_font(path: &str) {
        let p = Path::new(path);
        assert!(
            p.exists(),
            "{path} missing or dangling — build.rs should have linked it; the DSL \
             references this path and font loading would panic",
        );
        let data = std::fs::read(p).expect("font should be readable");
        assert!(data.len() > 4, "{path} is empty");
        let magic = &data[..4];
        assert!(
            magic == b"ttcf" || magic == b"OTTO" || magic == [0x00, 0x01, 0x00, 0x00],
            "unrecognised font magic {magic:?} at {path}",
        );
    }

    /// Font loading in makepad is lazy and panics on a bad face
    /// (`.expect("font face should load")`), so a broken link would not
    /// surface until the first Chinese glyph is drawn — potentially in front
    /// of a user. Check the wiring here instead.
    #[test]
    fn cjk_font_link_resolves_to_a_real_font() {
        assert_is_real_font(CJK_FONT);
    }

    /// The Latin face is used by every label, so a dangling link here is a
    /// fully blank UI (seen on Arch/Omarchy: build.rs once wrote a
    /// crate-relative fallback path into the symlink, which then resolved
    /// relative to `resources/fonts/` and pointed nowhere).
    #[test]
    fn latin_font_link_resolves_to_a_real_font() {
        assert_is_real_font(LATIN_FONT);
    }

    /// On unix these are symlinks, and their targets must be absolute: a
    /// relative target is resolved against the link's *own* directory, so
    /// `resources/fonts/X.ttf` would silently become
    /// `resources/fonts/resources/fonts/X.ttf`. This is exactly the bug that
    /// blanked the UI on distros whose system-font paths didn't match the
    /// hardcoded candidates.
    #[cfg(unix)]
    #[test]
    fn font_symlink_targets_are_absolute() {
        for link in [CJK_FONT, LATIN_FONT] {
            let target = std::fs::read_link(link)
                .unwrap_or_else(|e| panic!("{link} should be a symlink: {e}"));
            assert!(
                target.is_absolute(),
                "{link} -> {} : symlink target must be absolute",
                target.display(),
            );
            assert!(
                target.is_file(),
                "{link} -> {} : symlink target does not exist",
                target.display(),
            );
        }
    }

    /// `ROBRIX_BUNDLED_FONTS=1` is the packaging switch: packagers copy
    /// `resources/` by value (dereferencing symlinks), so a build meant for
    /// distribution must never resolve these to a host font — that would ship
    /// PingFang / Microsoft YaHei inside the package. build.rs re-exports the
    /// switch via `cargo:rustc-env` so this test sees the same value the
    /// build saw. Run with `ROBRIX_BUNDLED_FONTS=1 cargo test` to exercise it.
    #[test]
    fn bundled_mode_never_resolves_to_a_host_font() {
        if env!("ROBRIX_BUNDLED_FONTS") != "1" {
            eprintln!("ROBRIX_BUNDLED_FONTS not set for this build, skipping");
            return;
        }
        for (link, bundled) in [(CJK_FONT, CJK_BUNDLED), (LATIN_FONT, LATIN_BUNDLED)] {
            let resolved = std::fs::canonicalize(link)
                .unwrap_or_else(|e| panic!("{link} should resolve: {e}"));
            let expected = std::fs::canonicalize(bundled)
                .unwrap_or_else(|e| panic!("{bundled} should exist: {e}"));
            // On unix the link is a symlink to the bundled file; on Windows it
            // is a copy, so compare bytes as well as the resolved path.
            let same_path = resolved == expected;
            let link_bytes = std::fs::read(link)
                .unwrap_or_else(|e| panic!("{link} should be readable: {e}"));
            let bundled_bytes = std::fs::read(bundled)
                .unwrap_or_else(|e| panic!("{bundled} should be readable: {e}"));
            let same_bytes = link_bytes == bundled_bytes;
            assert!(
                same_path || same_bytes,
                "{link} resolves to {} but bundled mode requires {bundled}",
                resolved.display(),
            );
        }
    }

    /// On macOS the whole point is that this resolves to a *system* Chinese
    /// sans rather than the bundled fallback — the system PingFang where
    /// present. Symlinked, never copied: the bytes stay Apple's and nothing
    /// here redistributes them.
    ///
    /// Face 0 must carry outlines the text stack can render: `glyf`/`CFF`/
    /// `CFF2` via ttf_parser, or Apple's `hvgl` (PingFang on macOS 26+),
    /// which makepad decodes on macOS through its CoreText outline fallback.
    #[cfg(target_os = "macos")]
    #[test]
    fn macos_links_to_a_system_cjk_font_with_renderable_outlines() {
        if env!("ROBRIX_BUNDLED_FONTS") == "1" {
            eprintln!("ROBRIX_BUNDLED_FONTS set: system fonts intentionally not linked, skipping");
            return;
        }
        let target = std::fs::read_link(CJK_FONT)
            .expect("on macOS the CJK font should be a symlink, not a copy");
        assert!(
            target.starts_with("/System/Library/"),
            "CJK font should point at a system font, got {}",
            target.display(),
        );
        let data = std::fs::read(&target).expect("system CJK font should be readable");
        let face_off = if &data[..4] == b"ttcf" {
            u32::from_be_bytes([data[12], data[13], data[14], data[15]]) as usize
        } else {
            0
        };
        let num_tables = u16::from_be_bytes([data[face_off + 4], data[face_off + 5]]) as usize;
        let has_outlines = (0..num_tables).any(|i| {
            let rec = face_off + 12 + 16 * i;
            matches!(&data[rec..rec + 4], b"glyf" | b"CFF " | b"CFF2" | b"hvgl")
        });
        assert!(
            has_outlines,
            "{} has no renderable outline table — build.rs should have skipped it",
            target.display(),
        );
    }
}
