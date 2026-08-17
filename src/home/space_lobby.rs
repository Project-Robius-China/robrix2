//! Contains two widgets related to the top-level view of a space.
//!
//! 1. `SpaceLobby`: shows details about a space, including its name, avatar,
//!    members, topic, and the full list of rooms and subspaces within it.
//! 2. `SpaceLobbyEntry`: the button that can be shown in a RoomsList
//!    that allows the user to click on it to show the `SpaceLobby`.
//!

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};
use imbl::Vector;
use makepad_widgets::*;
use makepad_widgets::animator::Animate;
use matrix_sdk::{RoomDisplayName, RoomState, ruma::OwnedRoomId};
use matrix_sdk_ui::spaces::SpaceRoom;
use ruma::{OwnedRoomAliasId, room::{JoinRuleSummary, RoomType}};
use tokio::sync::mpsc::UnboundedSender;
use crate::shared::avatar::AvatarState;
use crate::shared::expand_arrow::ExpandArrow;
use crate::utils::replace_linebreaks_separators;
/// The horizontal indent width (in pixels) per tree level.
const TREE_INDENT_WIDTH: f64 = 44.0;

use crate::{
    app::{AppState, AppStateAction},
    avatar_cache::{self, AvatarCacheEntry},
    app::ConfirmDeleteAction,
    home::{
        add_existing_room_modal::AddExistingRoomModalAction,
        add_room::{CreatableSpacesAction, CreateRoomAction, CreateRoomModalAction, refresh_space_children},
        invite_modal::InviteModalAction,
        room_settings_modal::RoomSettingsAction,
        rooms_list::RoomsListRef,
    },
    i18n::{AppLanguage, tr_fmt, tr_key},
    join_leave_room_modal::{JoinLeaveModalKind, JoinLeaveRoomModalAction},
    room::BasicRoomDetails,
    shared::{
        avatar::{AvatarWidgetExt, AvatarWidgetRefExt},
        confirmation_modal::ConfirmationModalContent,
        popup_list::{PopupKind, enqueue_popup_notification},
        room_filter_input_bar::RoomFilterInputBarWidgetExt,
    },
    sliding_sync::{MatrixRequest, submit_async_request},
    space_service_sync::{SpaceRequest, SpaceRoomExt, SpaceRoomListAction},
    utils::{self, RoomNameId},
};


script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*


    // An entry in the RoomsList that will show the SpaceLobby when clicked.
    mod.widgets.SpaceLobbyEntry = #(SpaceLobbyEntry::register_widget(vm)) {
        visible: false, // only visible when a space is selected
        width: Fill,
        height: Fit,
        flow: Right,
        align: Align{y: 0.5}
        padding: 5,
        margin: Inset{top: 10, bottom: 0}
        spacing: 5
        cursor: MouseCursor.Hand

        show_bg: true
        draw_bg +: {
            hover: instance(0.0)
            active: instance(0.0)

            color: instance(RBX_BG_SURFACE)
            color_hover: instance(RBX_BG_HOVER)
            color_active: instance(RBX_BG_SELECTED)
            border_size: uniform(0.0)
            border_color: instance(RBX_TRANSPARENT)
            border_radius: uniform(4.0)
            border_inset: uniform(vec4(0.0))

            get_color: fn() -> vec4 {
                return mix(
                    mix(
                        self.color,
                        self.color_hover,
                        self.hover
                    ),
                    self.color_active,
                    self.active
                )
            }

            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                sdf.box(
                    self.border_inset.x + self.border_size,
                    self.border_inset.y + self.border_size,
                    self.rect_size.x - (self.border_inset.x + self.border_inset.z + self.border_size * 2.0),
                    self.rect_size.y - (self.border_inset.y + self.border_inset.w + self.border_size * 2.0),
                    max(1.0, self.border_radius)
                )
                sdf.fill_keep(self.get_color())
                if self.border_size > 0.0 {
                    sdf.stroke(self.border_color, self.border_size)
                }
                return sdf.result;
            }
        }

        icon := Icon {
            width: 25,
            height: 25,
            margin: Inset{left: 5, right: 3}
            align: Align{x: 0.5, y: 0.5}
            draw_icon +: {
                svg: (ICON_HIERARCHY)

                active: instance(0.0)
                hover: instance(0.0)
                down: instance(0.0)

                color: (RBX_FG_PRIMARY)
                color_hover: instance(RBX_FG_PRIMARY)
                color_active: instance(RBX_FG_PRIMARY)

                get_color: fn() -> vec4 {
                    return mix(
                        mix(
                            self.color,
                            self.color_hover,
                            self.hover
                        ),
                        self.color_active,
                        self.active
                    )
                }
            }
            icon_walk: Walk{ width: 25, height: 20, margin: Inset{top: 2} }
        }

        space_lobby_label := Label {
            width: Fill, height: Fit
            flow: Right,
            padding: 0,

            draw_text +: {
                active: instance(0.0)
                hover: instance(0.0)
                down: instance(0.0)

                color: (RBX_FG_PRIMARY)
                color_hover: instance(RBX_FG_PRIMARY)
                color_active: instance(RBX_FG_PRIMARY)

                text_style: REGULAR_TEXT {font_size: 11},

                get_color: fn() -> vec4 {
                    return mix(
                        mix(
                            self.color,
                            self.color_hover,
                            self.hover
                        ),
                        self.color_active,
                        self.active
                    )
                }
            }
            text: ""
        }

        animator: Animator{
            hover: {
                default: @off
                off: AnimatorState{
                    from: {all: Forward {duration: 0.15}}
                    apply: {
                        draw_bg: {down: [{time: 0.0, value: 0.0}], hover: 0.0}
                        space_lobby_label: { draw_text: {down: [{time: 0.0, value: 0.0}], hover: 0.0} }
                        icon: { draw_icon: {down: [{time: 0.0, value: 0.0}], hover: 0.0} }
                    }
                }
                on: AnimatorState{
                    from: {all: Snap}
                    apply: {
                        draw_bg: {down: [{time: 0.0, value: 0.0}], hover: 1.0}
                        space_lobby_label: { draw_text: {down: [{time: 0.0, value: 0.0}], hover: 1.0} }
                        icon: { draw_icon: {down: [{time: 0.0, value: 0.0}], hover: 1.0} }
                    }
                }
                down: AnimatorState{
                    from: {all: Forward {duration: 0.2}}
                    apply: {
                        draw_bg: {down: [{time: 0.0, value: 1.0}], hover: 1.0,}
                        space_lobby_label: { draw_text: {down: [{time: 0.0, value: 1.0}], hover: 1.0,} }
                        icon: { draw_icon: {down: [{time: 0.0, value: 1.0}], hover: 1.0,} }
                    }
                }
            }
            active: {
                default: @off
                off: AnimatorState{
                    from: {all: Snap}
                    apply: {
                        draw_bg: {active: 0.0}
                        space_lobby_label: { draw_text: {active: 0.0} }
                        icon: { draw_icon: {active: 0.0} }
                    }
                }
                on: AnimatorState{
                    from: {all: Snap}
                    apply: {
                        draw_bg: {active: 1.0}
                        space_lobby_label: { draw_text: {active: 1.0} }
                        icon: { draw_icon: {active: 1.0} }
                    }
                }
            }
        }
    }

    // A view that draws the hierarchical tree structure lines.
    let DrawTreeLine = set_type_default() do #(DrawTreeLine::script_shader(vm)){
        ..mod.draw.DrawQuad
    }

    mod.widgets.TreeLines = #(TreeLines::register_widget(vm)) {
        width: Fill, height: Fill { min: 32 }

        draw_bg: DrawTreeLine {
            indent_width: 44.0
            level: 0.0
            is_last: 0.0
            parent_mask: 0.0
            line_color: (RBX_DIVIDER)

            pixel: fn() {
                let pos = self.pos * self.rect_size;
                let indent = self.indent_width;
                // Offset to center each vertical line under the parent-level avatar.
                // Derived from: main_entry left padding (8) + expand_icon space (14)
                // + half avatar width (16) = 38.
                let half_indent = 38.0;
                let line_width = 1.0;
                let half_line = 0.5;

                let mut c = vec4(0.0);

                // Dumb approach, but it works.
                for i in 0..20 {
                    if f32(i) > self.level { break; }
                    
                    if f32(i) < self.level {
                        // Check mask for parent levels
                        let mask_bit = modf(floor(self.parent_mask / pow(2.0, f32(i))), 2.0);
                        if mask_bit > 0.5 {
                            // Draw full vertical line
                            if abs(pos.x - (f32(i) * indent + half_indent)) < half_line && pos.y < self.rect_size.y {
                                c = self.line_color;
                                break;
                            }
                        }
                    } else {
                        // Current level: connection to self
                        
                        // Horizontal line to content.
                        // Snap hy to the nearest pixel center (floor(y) + 0.5) so the
                        // strict abs() < 0.5 check always hits exactly one pixel regardless
                        // of whether rect_size.y is even or odd.
                        let hy = floor(self.rect_size.y * 0.5) + 0.5;
                        // Extend horizontal line to the center of the expand_icon:
                        // spacer_end + left_padding(8) - expand_margin_left(6) + expand_width(16)/2 = +10
                        if abs(pos.y - hy) < half_line && pos.x > (f32(i) * indent + half_indent) && pos.x < ((f32(i) + 1.0) * indent + 10.0) {
                            c = self.line_color;
                            break;
                        }
                        
                        // Vertical line (L shape)
                        if abs(pos.x - (f32(i) * indent + half_indent)) < half_line && pos.y < (self.rect_size.y * (1.0 - 0.5 * self.is_last)) {
                            c = self.line_color;
                            break;
                        }
                    }
                }
                return c;
            }
        }
    }

    // Entry for a child subspace (can be expanded)
    mod.widgets.SubspaceEntry = #(SubspaceEntry::register_widget(vm)) {
        width: Fill,
        height: Fit,
        flow: Overlay
        align: Align{x: 1.0, y: 0.5}

        show_bg: true
        draw_bg +: {
            hover: instance(0.0)
            color: instance(RBX_BG_SURFACE)
            color_hover: instance(RBX_BG_HOVER)
            pixel: fn() {
                return mix(self.color, self.color_hover, self.hover);
            }
        }

        main_entry := View {
            width: Fill,
            height: Fit,
            flow: Right
            align: Align{x: 0, y: 0.5}
            padding: Inset{top: 8, bottom: 8, left: 8, right: 12}
            cursor: MouseCursor.Hand

            // Invisible spacer whose width is set dynamically to match
            // the tree indent level, replacing tree_lines' layout role.
            indent_spacer := View { width: 0, height: Fit }

            // Expand/collapse arrow (animated triangle)
            expand_icon := mod.widgets.ExpandArrow {
                width: 16,
                height: 16,
                margin: Inset{ left: -6, right: 4 }
                draw_bg.color: (RBX_FG_TERTIARY)
                draw_bg.border_radius: 1.5 // less rounded
            }

            avatar := Avatar { width: 32, height: 32, margin: Inset{right: 8} }

            content := View {
                width: Fill
                height: Fit
                flow: Down
                align: Align { y: 0.5 }
                spacing: 5,

                name_label := Label {
                    width: Fill, height: Fit,
                    margin: 0
                    padding: 0
                    flow: Flow.Right{wrap: true}
                    max_lines: 2
                    text_overflow: Ellipsis
                    draw_text +: { text_style: RBX_TEXT_BODY {}, color: (RBX_FG_PRIMARY) }
                }

                suggested_tag := RoundedView {
                    visible: false
                    width: Fit, height: Fit,
                    padding: Inset { left: 9, right: 9, top: 3, bottom: 3 }
                    show_bg: true
                    draw_bg +: {
                        color: (RBX_ACCENT_SOFT)
                        border_radius: (RBX_RADIUS_PILL)
                        border_size: 0.0
                    }
                    suggested_label := Label {
                        padding: 0
                        margin: 0
                        width: Fit, height: Fit,
                        text: "Suggested"
                        draw_text +: { text_style: RBX_TEXT_BADGE {}, color: (RBX_ACCENT) }
                    }
                }

                info_label := Label {
                    width: Fill, height: Fit,
                    margin: 0
                    padding: 0
                    flow: Flow.Right{wrap: true}
                    max_lines: 2
                    text_overflow: Ellipsis
                    draw_text +: { text_style: RBX_TEXT_META {}, color: (RBX_FG_SECONDARY) }
                }
            }
        }

        buttons_view := RoundedView {
            visible: false
            width: Fit,
            height: Fit,
            flow: Right,
            spacing: 8,
            padding: Inset { left: 8, right: 8, top: 4, bottom: 4 }
            align: Align{x: 1.0, y: 0.5}
            margin: Inset{right: 16}

            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_SM)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
            }

            join_button := RobrixPositiveIconButton {
                width: Fit,
                padding: 8
                spacing: 0
                icon_walk: Walk{width: 0, height: 0}
                draw_text.text_style: REGULAR_TEXT {font_size: 9.5}
                text: ""
            }

            view_button := RobrixIconButton {
                width: Fit,
                padding: 8
                spacing: 0
                icon_walk: Walk{width: 0, height: 0}
                draw_text.text_style: REGULAR_TEXT {font_size: 9.5}
                text: ""
            }

            leave_button := RobrixNegativeIconButton {
                width: Fit,
                padding: 8
                spacing: 0
                icon_walk: Walk{width: 0, height: 0}
                draw_text.text_style: REGULAR_TEXT {font_size: 9.5}
                text: ""
            }

            // Unlinks this child from the space. Only shown to users who are
            // allowed to change the parent space's children.
            remove_from_space_button := RobrixNegativeIconButton {
                width: Fit,
                padding: 8
                spacing: 0
                icon_walk: Walk{width: 0, height: 0}
                draw_text.text_style: REGULAR_TEXT {font_size: 9.5}
                text: ""
            }
        }

        // The connecting hierarchical lines on the left, placed last in
        // the Overlay so the parent's Fit height (from main_entry) is
        // already resolved when tree_lines is laid out.
        tree_lines := mod.widgets.TreeLines {}

        animator: Animator{
            hover: {
                default: @off
                off: AnimatorState{ from: {all: Forward {duration: 0.1}}, apply: { draw_bg: {hover: 0.0} } }
                on: AnimatorState{ from: {all: Snap}, apply: { draw_bg: {hover: 1.0} } }
            }
        }
    }

    // Entry for a child room within a space, which cannot be expanded.
    mod.widgets.RoomEntry = mod.widgets.SubspaceEntry {
        main_entry +: {
            cursor: MouseCursor.Default
            expand_icon := View {
                width: 10
                height: 16
            }
        }
    }

    mod.widgets.SpaceLobbyStatusLabel = View {
        width: Fill, height: Fit,
        flow: Right,
        align: Align{ x: 0.5, y: 0.5 }
        padding: 20.0,

        loading_spinner := LoadingSpinner {
            width: 18,
            height: 18,
            draw_bg +: {
                color: (RBX_ACCENT)
                border_size: 2.5
            }
        }

        label := Label {
            padding: Inset{left: 10}
            width: Fit,
            flow: Flow.Right{wrap: true},
            align: Align{ x: 0.5, y: 0.5 }
            draw_text +: {
                color: (RBX_FG_SECONDARY),
                text_style: RBX_TEXT_BODY {}
            }
            text: ""
        }
    }

    // Small loading indicator shown inline when loading subspace children.
    // Uses the same Overlay + spacer pattern as SubspaceEntry so tree lines
    // span the full row height and the content is indented correctly.
    mod.widgets.SubspaceLoadingEntry = View {
        width: Fill, height: 36,
        flow: Overlay,
        align: Align{ x: 0, y: 0.5 }

        loading_content := View {
            width: Fill, height: Fit,
            flow: Right,
            align: Align{ x: 0, y: 0.5 }
            padding: Inset{left: 8, right: 12}

            // Spacer for tree indent (width set dynamically in draw_item)
            indent_spacer := View { width: 0, height: Fit }

            loading_spinner := LoadingSpinner {
                width: 14,
                height: 14,
                margin: Inset{left: 10, right: 4}
                draw_bg +: {
                    color: (RBX_ACCENT)
                    border_size: 2.0
                }
            }

            label := Label {
                width: Fit,
                height: Fit,
                draw_text +: {
                    text_style: RBX_TEXT_META {},
                    color: (RBX_FG_TERTIARY),
                }
                text: "Loading..."
            }
        }

        // Tree lines drawn last so parent height is resolved
        tree_lines := mod.widgets.TreeLines {}

        loading_spinner := LoadingSpinner {
            width: 14,
            height: 14,
            margin: Inset{left: 8, right: 10}
            draw_bg +: {
                color: (RBX_ACCENT)
                border_size: 2.0
            }
        }

        label := Label {
            width: Fit,
            height: Fit,
            draw_text +: {
                text_style: RBX_TEXT_META {},
                color: (RBX_FG_TERTIARY),
            }
            text: ""
        }
    }

    // The main view that shows the lobby (homepage) for a space.
    mod.widgets.SpaceLobbyScreen = set_type_default() do #(SpaceLobbyScreen::register_widget(vm)) {
        ..mod.widgets.SolidView

        width: Fill, height: Fill,
        flow: Down,

        show_bg: true
        draw_bg +: {
            color: (RBX_BG_SURFACE)
        }

        // Header with parent space info
        header := SolidView {
            width: Fill,
            height: Fit,
            flow: Down,
            padding: Inset{left: 16, right: 16, top: 16, bottom: 8}

            show_bg: true,
            draw_bg.color: (RBX_BG_SURFACE)

            space_info_row := View {
                width: Fill,
                height: Fit,
                flow: Right,
                spacing: 10,
                align: Align { y: 0.5 }

                space_info_label := Label {
                    width: Fit,
                    height: Fit,
                    flow: Right, // do not wrap
                    margin: Inset{left: 2}
                    draw_text +: {
                        text_style: RBX_TEXT_META {},
                        color: (RBX_FG_SECONDARY),
                    }
                    text: "Welcome to the space:"
                }

                // Filter input bar for searching rooms/spaces in this space
                filter_bar := mod.widgets.RoomFilterInputBar {
                    input +: {
                        empty_text: "Filter this space..."
                    }
                }
            }
            
            parent_space_row := View {
                width: Fill,
                height: Fit,
                flow: Right,
                align: Align{ y: 0.5 }
                padding: Inset{ top: 8 }
                
                parent_avatar := Avatar {
                    width: 36,
                    height: 36,
                    margin: Inset{ right: 12 }
                }
                
                parent_name := Label {
                    width: Fill,
                    height: Fit,
                    flow: Right, // do not wrap
                    margin: Inset{top: 4} // vertically center-align with the avatar
                    draw_text +: {
                        text_style: RBX_TEXT_SECTION_TITLE {},
                        color: (RBX_FG_PRIMARY),
                    }
                    text: ""
                }

                create_room_button := RobrixPositiveIconButton {
                    width: Fit
                    align: Align{x: 0.5, y: 0.5}
                    margin: Inset{left: 6}
                    padding: 12,
                    draw_icon.svg: (ICON_ADD)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: ""
                }

                // Links one of the user's existing rooms into this space.
                add_existing_room_button := RobrixNeutralIconButton {
                    width: Fit
                    align: Align{x: 0.5, y: 0.5}
                    margin: Inset{left: 6}
                    padding: 12,
                    draw_icon.svg: (ICON_LINK)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: ""
                }

                // Creates a nested space under the space this lobby is showing.
                create_subspace_button := RobrixNeutralIconButton {
                    width: Fit
                    align: Align{x: 0.5, y: 0.5}
                    margin: Inset{left: 6}
                    padding: 12,
                    draw_icon.svg: (ICON_HIERARCHY)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: ""
                }

                settings_button := RobrixNeutralIconButton {
                    width: Fit
                    align: Align{x: 0.5, y: 0.5}
                    margin: Inset{left: 6}
                    padding: 12,
                    draw_icon.svg: (ICON_SETTINGS)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: ""
                }

                invite_button := RobrixNeutralIconButton {
                    width: Fit
                    align: Align{x: 0.5, y: 0.5}
                    margin: Inset{left: 6}
                    padding: 12,
                    draw_icon.svg: (ICON_ADD_USER)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: ""
                }

                // Leaving is available to every member, not just those who can
                // administer the space — otherwise a plain member who joined has
                // no way back out. Extra left margin separates it from the
                // constructive actions so it isn't hit by accident.
                leave_space_button := RobrixNegativeIconButton {
                    width: Fit
                    align: Align{x: 0.5, y: 0.5}
                    margin: Inset{left: 16}
                    padding: 12,
                    draw_icon.svg: (ICON_LOGOUT)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1} }
                    text: ""
                }
            }
        }

        // Hairline under the header: it shares the tree's surface colour, so
        // without this the two areas bleed into one another.
        header_divider := SolidView {
            width: Fill, height: 1.0
            show_bg: true
            draw_bg.color: (RBX_DIVIDER)
        }

        // The hierarchical tree list
        tree_list := PortalList {
            keep_invisible: false,
            max_pull_down: 0.0,
            auto_tail: false,
            width: Fill, height: Fill
            flow: Down,
            spacing: 0.0

            subspace_entry := mod.widgets.SubspaceEntry {}
            room_entry := mod.widgets.RoomEntry {}
            subspace_loading := mod.widgets.SubspaceLoadingEntry {}
            status_label := mod.widgets.SpaceLobbyStatusLabel {}
            bottom_filler := View {
                width: Fill,
                height: 80.0,
            }
        }
    }
}


thread_local! {
    /// A cache of UI states for each SpaceLobbyScreen, keyed by the space's room ID.
    /// This allows preserving the expanded/collapsed state of subspaces across screen changes.
    static SPACE_LOBBY_STATES: RefCell<BTreeMap<OwnedRoomId, SpaceLobbyUiState>> = const {
        RefCell::new(BTreeMap::new())
    };
}

/// The UI-side state of a SpaceLobbyScreen that should persist across hide/show cycles.
#[derive(Default)]
struct SpaceLobbyUiState {
    /// The set of space IDs that are currently expanded (showing their children).
    expanded_spaces: HashSet<OwnedRoomId>,
}


/// A clickable entry shown in the RoomsList that will show the space lobby when clicked.
#[derive(Script, ScriptHook, Widget, Animator)]
pub struct SpaceLobbyEntry {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    #[apply_default] animator: Animator,
}

impl Widget for SpaceLobbyEntry {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, _scope: &mut Scope) {
        if self.animator_handle_event(cx, event).must_redraw() {
            self.redraw(cx);
        }

        let area = self.draw_bg.area();
        match event.hits(cx, area) {
            Hit::FingerHoverIn(_) => {
                self.animator_play(cx, ids!(hover.on));
            }
            Hit::FingerHoverOut(_) => {
                self.animator_play(cx, ids!(hover.off));
            }
            Hit::FingerDown(_fe) => {
                self.animator_play(cx, ids!(hover.down));
            }
            Hit::FingerLongPress(_lp) => {
                self.animator_play(cx, ids!(hover.down));
            }
            Hit::FingerUp(fe) if fe.is_over && fe.is_primary_hit() && fe.was_tap() => {
                self.animator_play(cx, ids!(hover.on));
                cx.action(SpaceLobbyAction::SpaceLobbyEntryClicked);
            }
            Hit::FingerUp(fe) if !fe.is_over => {
                self.animator_play(cx, ids!(hover.off));
            }
            Hit::FingerMove(_fe) => { }
            _ => {}
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        self.view.label(cx, ids!(space_lobby_label))
            .set_text(cx, tr_key(app_language, "space_lobby.entry.explore_space"));
        self.view.draw_walk(cx, scope, walk)
    }
}

impl SpaceLobbyEntry {
    fn set_selected(&mut self, cx: &mut Cx, is_selected: bool) {
        self.animator_toggle(cx, is_selected, Animate::No, ids!(active.on), ids!(active.off));
    }
}
impl SpaceLobbyEntryRef {
    pub fn set_selected(&self, cx: &mut Cx, is_selected: bool) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_selected(cx, is_selected);
    }
}


#[derive(Debug)]
pub enum SpaceLobbyAction {
    SpaceLobbyEntryClicked,
}

#[derive(Script, ScriptHook)]
#[repr(C)]
pub struct DrawTreeLine {
    #[deref] draw_super: DrawQuad,
    #[live] indent_width: f32,
    #[live] level: f32,
    #[live] is_last: f32,
    #[live] parent_mask: f32,
    /// The colour of the connector lines, so the shader doesn't hard-code grey.
    #[live] line_color: Vec4,
}

#[derive(Script, ScriptHook, Widget)]
pub struct TreeLines {
    #[uid] uid: WidgetUid,
    #[redraw] #[live] draw_bg: DrawTreeLine,
    #[walk] walk: Walk,
}

impl Widget for TreeLines {
    fn handle_event(&mut self, _cx: &mut Cx, _event: &Event, _scope: &mut Scope) { }

    fn draw_walk(&mut self, cx: &mut Cx2d, _scope: &mut Scope, walk: Walk) -> DrawStep {
        let mut walk = walk;
        // When used in a non-Overlay flow (e.g., SubspaceLoadingEntry's Right flow),
        // set the width to the indent area. When width is Fill (Overlay case),
        // the shader handles clipping via indent_width.
        if !walk.width.is_fill() {
            let indent_pixel = (self.draw_bg.level + 1.0) * self.draw_bg.indent_width;
            walk.width = Size::Fixed(indent_pixel as f64);
        }
        // Use the parent's resolved height so tree lines span the full row,
        // even when our height is Fill inside a Fit parent.
        let parent_h = cx.turtle().height();
        if parent_h.is_finite() && parent_h > 0.0 {
            walk.height = Size::Fixed(parent_h);
        }
        self.draw_bg.draw_walk(cx, walk);
        DrawStep::done()
    }
}


/// A clickable entry for a child subspace.
#[derive(Script, ScriptHook, Widget, Animator)]
pub struct SubspaceEntry {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    #[apply_default] animator: Animator,
    #[rust] room_id: Option<OwnedRoomId>,
    #[rust] is_space: bool,
    #[rust] show_buttons_view: bool,
    /// Whether `show_buttons_view` was set by a tap (touch) rather than mouse hover.
    /// On mobile (no hover events), tapping toggles button visibility;
    /// on desktop, hover handles it and taps fire the normal action.
    #[rust] buttons_shown_by_tap: bool,
    #[rust] is_expanded: bool,
}

/// The result of changing which rooms a space contains, i.e. of adding or
/// removing an `m.space.child` link.
///
/// `error` is `None` on success; these are emitted by the background Matrix task.
#[derive(Debug)]
pub enum SpaceChildAction {
    Added {
        space_id: OwnedRoomId,
        child: RoomNameId,
        error: Option<String>,
    },
    Removed {
        space_id: OwnedRoomId,
        child: RoomNameId,
        error: Option<String>,
    },
}

/// Actions emitted when a `SubspaceEntry` or its buttons are clicked.
///
/// These *are* all widget actions.
#[derive(Clone, Debug, Default)]
pub enum SubspaceEntryAction {
    SpaceClicked { space_id: OwnedRoomId },
    RoomClicked  { room_id: OwnedRoomId },
    JoinClicked  { room_id: OwnedRoomId, is_space: bool },
    LeaveClicked { room_id: OwnedRoomId, is_space: bool },
    ViewClicked  { room_id: OwnedRoomId },
    /// Unlink this child from the space whose lobby is being shown.
    /// This does not leave the room; it only removes it from the space.
    RemoveFromSpaceClicked { room_id: OwnedRoomId, is_space: bool },
    #[default]
    None,
}

impl ActionDefaultRef for SubspaceEntryAction {
    fn default_ref() -> &'static Self {
        static DEFAULT: SubspaceEntryAction = SubspaceEntryAction::None;
        &DEFAULT
    }
}

impl Widget for SubspaceEntry {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if self.animator_handle_event(cx, event).must_redraw() {
            self.redraw(cx);
        }

        // NOTE: Use child_by_path instead of widget tree-based lookups
        //       (e.g., self.view.view(), self.view.button()) because these
        //       fail for portal list items.
        let buttons_view_ref = self.view.child_by_path(ids!(buttons_view));
        let buttons_view_rect = buttons_view_ref.area().rect(cx);
        let are_buttons_visible = self.show_buttons_view;
        match event.hits_with_test(cx, self.view.area(), |abs, rect, _| {
            rect.contains(abs) && !(are_buttons_visible && buttons_view_rect.contains(abs))
        }) {
            Hit::FingerHoverIn(_) => {
                self.animator_play(cx, ids!(hover.on));
                if !self.show_buttons_view {
                    self.show_buttons_view = true;
                    self.buttons_shown_by_tap = false;
                    self.view.child_by_path(ids!(buttons_view)).set_visible(cx, true);
                    self.redraw(cx);
                }
            }
            // Occasionally there's an issue with Makepad hover events where hover in/out
            // doesn't work as expected, so we double-check here.
            Hit::FingerHoverOver(_) if !self.show_buttons_view => {
                self.animator_play(cx, ids!(hover.on));
                self.show_buttons_view = true;
                self.buttons_shown_by_tap = false;
                self.view.child_by_path(ids!(buttons_view)).set_visible(cx, true);
                self.redraw(cx);
            }
            Hit::FingerHoverOut(fe) => {
                // When the mouse moves from the main SubspaceEntry area into the buttons_view,
                // Makepad emits a HoverOut hit, but we don't want that to actually count as a hover-out
                // because the mouse is still hovering over the buttons_view.
                let entry_rect = self.view.area().rect(cx);
                let is_over_buttons_view = self.show_buttons_view && buttons_view_rect.contains(fe.abs);
                if !entry_rect.contains(fe.abs) && !is_over_buttons_view {
                    self.animator_play(cx, ids!(hover.off));
                    self.show_buttons_view = false;
                    self.buttons_shown_by_tap = false;
                    self.view.child_by_path(ids!(buttons_view)).set_visible(cx, false);
                    self.redraw(cx);
                }
            }
            Hit::FingerDown(_) => {
                cx.set_key_focus(self.view.area());
            }
            Hit::FingerUp(fe) if fe.is_over && fe.is_primary_hit() && fe.was_tap() => {
                let is_within_buttons_view = self.show_buttons_view
                    && self.view.child_by_path(ids!(buttons_view)).area().rect(cx).contains(fe.abs);
                if is_within_buttons_view {
                    // Let individual button handlers deal with taps on the buttons.
                }
                // On touch devices, tapping on the avatar or to its left
                // always expands/collapses a space (bypasses button toggle).
                else if fe.is_touch() && self.is_space {
                    let avatar_rect = self.view.child_by_path(ids!(main_entry.avatar)).area().rect(cx);
                    let tap_in_expand_region = fe.abs.x <= avatar_rect.pos.x + avatar_rect.size.x;
                    if tap_in_expand_region {
                        self.is_expanded = !self.is_expanded;
                        if let Some(mut arrow) = self.view.child_by_path(ids!(main_entry.expand_icon)).borrow_mut::<ExpandArrow>() {
                            arrow.set_is_open(cx, self.is_expanded, Animate::Yes);
                        }
                        if let Some(room_id) = self.room_id.as_ref() {
                            cx.widget_action(
                                self.widget_uid(),
                                SubspaceEntryAction::SpaceClicked { space_id: room_id.clone() },
                            );
                        }
                    } else {
                        // Touch tap on the text area: toggle buttons visibility.
                        self.toggle_buttons_for_tap(cx);
                    }
                }
                // On touch devices for rooms (not spaces): tap toggles buttons.
                else if fe.is_touch() {
                    self.toggle_buttons_for_tap(cx);
                }
                // Non-touch (desktop): fire the normal entry action,
                // since hover already handles button visibility.
                else if let Some(room_id) = self.room_id.as_ref() {
                    if self.is_space {
                        self.is_expanded = !self.is_expanded;
                        if let Some(mut arrow) = self.view.child_by_path(ids!(main_entry.expand_icon)).borrow_mut::<ExpandArrow>() {
                            arrow.set_is_open(cx, self.is_expanded, Animate::Yes);
                        }
                        cx.widget_action(
                            self.widget_uid(),
                            SubspaceEntryAction::SpaceClicked { space_id: room_id.clone() },
                        );
                    } else {
                        cx.widget_action(
                            self.widget_uid(),
                            SubspaceEntryAction::RoomClicked { room_id: room_id.clone() },
                        );
                    }
                }
            }
            _ => {}
        }

        self.view.handle_event(cx, event, scope);

        if let Event::Actions(actions) = event {
            let join_button = self.view.child_by_path(ids!(buttons_view.join_button)).as_button();
            let leave_button = self.view.child_by_path(ids!(buttons_view.leave_button)).as_button();
            let view_button = self.view.child_by_path(ids!(buttons_view.view_button)).as_button();

            if join_button.clicked(actions) {
                if let Some(room_id) = self.room_id.clone() {
                    join_button.reset_hover(cx);
                    cx.widget_action(
                        self.widget_uid(),
                        SubspaceEntryAction::JoinClicked { room_id, is_space: self.is_space },
                    );
                }
            }
            if leave_button.clicked(actions) {
                if let Some(room_id) = self.room_id.clone() {
                    leave_button.reset_hover(cx);
                    cx.widget_action(
                        self.widget_uid(),
                        SubspaceEntryAction::LeaveClicked { room_id, is_space: self.is_space },
                    );
                }
            }
            if view_button.clicked(actions) {
                if let Some(room_id) = self.room_id.clone() {
                    view_button.reset_hover(cx);
                    cx.widget_action(
                        self.widget_uid(),
                        SubspaceEntryAction::ViewClicked { room_id },
                    );
                }
            }
            let remove_from_space_button = self.view
                .child_by_path(ids!(buttons_view.remove_from_space_button))
                .as_button();
            if remove_from_space_button.clicked(actions) {
                if let Some(room_id) = self.room_id.clone() {
                    remove_from_space_button.reset_hover(cx);
                    cx.widget_action(
                        self.widget_uid(),
                        SubspaceEntryAction::RemoveFromSpaceClicked { room_id, is_space: self.is_space },
                    );
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl SubspaceEntry {
    /// Toggles the buttons_view visibility for a touch tap.
    fn toggle_buttons_for_tap(&mut self, cx: &mut Cx) {
        if self.show_buttons_view {
            self.animator_play(cx, ids!(hover.off));
            self.show_buttons_view = false;
            self.buttons_shown_by_tap = false;
            self.view.child_by_path(ids!(buttons_view)).set_visible(cx, false);
        } else {
            self.animator_play(cx, ids!(hover.on));
            self.show_buttons_view = true;
            self.buttons_shown_by_tap = true;
            self.view.child_by_path(ids!(buttons_view)).set_visible(cx, true);
        }
        self.redraw(cx);
    }
}

/// The subset of info in [`SpaceRoom`] that we display for each room/space.
#[derive(Debug)]
struct SpaceRoomInfo {
    id: OwnedRoomId,
    name: String,
    canonical_alias: Option<OwnedRoomAliasId>,
    topic: Option<String>,
    avatar: AvatarState,
    num_joined_members: u64,
    state: Option<RoomState>,
    #[allow(unused)]
    join_rule: Option<JoinRuleSummary>,
    /// If `Some`, this is a space. If `None`, it's a room.
    children_count: Option<u64>,
    /// Whether the room is suggested by the space administrators.
    suggested: bool,
}
impl SpaceRoomInfo {
    fn is_space(&self) -> bool {
        self.children_count.is_some()
    }
}
impl From<&SpaceRoom> for SpaceRoomInfo {
    fn from(space_room: &SpaceRoom) -> Self {
        SpaceRoomInfo {
            id: space_room.room_id.clone(),
            name: space_room.display_name.clone(),
            canonical_alias: space_room.canonical_alias.clone(),
            topic: space_room.topic.as_ref().map(|t| {
                replace_linebreaks_separators(t.trim(), false).into_owned()
            }),
            avatar: AvatarState::Known(space_room.avatar_url.clone()),
            num_joined_members: space_room.num_joined_members,
            state: space_room.state,
            join_rule: space_room.join_rule.clone(),
            children_count: space_room.is_space().then_some(space_room.children_count),
            suggested: space_room.suggested,
        }
    }
}
impl From<SpaceRoom> for SpaceRoomInfo {
    fn from(space_room: SpaceRoom) -> Self {
        SpaceRoomInfo {
            children_count: space_room.is_space().then_some(space_room.children_count),
            canonical_alias: space_room.canonical_alias,
            id: space_room.room_id,
            name: space_room.display_name,
            topic: space_room.topic.map(|t| {
                replace_linebreaks_separators(t.trim(), false).into_owned()
            }),
            avatar: AvatarState::Known(space_room.avatar_url),
            num_joined_members: space_room.num_joined_members,
            state: space_room.state,
            join_rule: space_room.join_rule,
            suggested: space_room.suggested,
        }
    }
}

/// An entry in the tree to be displayed.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
enum TreeEntry {
    /// A regular space or room entry.
    Item {
        /// The info needed to display this space or room.
        info: SpaceRoomInfo,
        /// The space this entry is a direct child of. Needed to unlink it from
        /// the right parent, since the tree can nest several spaces deep.
        parent_space_id: OwnedRoomId,
        /// The nesting level (0 = direct child of the displayed space).
        level: usize,
        /// Whether this entry is the last child of its parent.
        is_last: bool,
        /// Bitmask of which parent levels need continuation lines.
        parent_mask: u32,
    },
    /// A loading indicator for a subspace that's still loading.
    Loading {
        /// The nesting level for proper indentation.
        level: usize,
        /// Bitmask of which parent levels need continuation lines.
        parent_mask: u32,
    },
}

// ---------------------------------------------------------------------------------
// Pure, `&self`/Widget-free tree-building functions.
//
// These are deliberately free functions (not methods on `SpaceLobbyScreen`) that
// take only explicit inputs (the children cache, expansion/loading sets, filter
// keywords) so they can be unit-tested directly without any Makepad `Cx`/`Widget`
// machinery. `SpaceLobbyScreen::rebuild_tree_entries` is a thin wrapper around
// `build_tree_for_space`/`build_filtered_tree`.
//
// Cycle-safety: `m.space.child` relationships are legally allowed to form cycles
// per the Matrix spec, so every function below guards against infinite recursion
// (and duplicate/self-referential edges) using a `visited: HashSet<OwnedRoomId>`
// of room IDs already emitted/walked *anywhere* in the current build, rather than
// a depth limit — a depth limit would incorrectly truncate legitimately deep (but
// acyclic) space hierarchies.
// ---------------------------------------------------------------------------------

/// Returns whether the given [`SpaceRoomInfo`] matches the filter keywords.
fn matches_filter(info: &SpaceRoomInfo, keywords: &str) -> bool {
    info.name.to_lowercase().contains(keywords)
        || info.id.as_str().to_lowercase().contains(keywords)
        || info.canonical_alias.as_ref()
            .is_some_and(|a| a.as_str().to_lowercase().contains(keywords))
        || info.topic.as_ref()
            .is_some_and(|t| t.to_lowercase().contains(keywords))
}

/// Recursively build the tree of spaces and their expanded children such that they
/// can be displayed in the SpaceLobbyScreen's PortalList.
///
/// Cycle-safe: see the module-level note above.
fn build_tree_for_space(
    children_cache: &HashMap<OwnedRoomId, Vector<SpaceRoom>>,
    expanded_spaces: &HashSet<OwnedRoomId>,
    loading_subspaces: &HashSet<OwnedRoomId>,
    tree_entries: &mut Vec<TreeEntry>,
    space_id: &OwnedRoomId,
    level: usize,
    parent_mask: u32,
) {
    let mut visited = HashSet::new();
    visited.insert(space_id.clone());
    build_tree_for_space_inner(
        children_cache,
        expanded_spaces,
        loading_subspaces,
        tree_entries,
        space_id,
        level,
        parent_mask,
        &mut visited,
    );
}

#[allow(clippy::too_many_arguments)]
fn build_tree_for_space_inner(
    children_cache: &HashMap<OwnedRoomId, Vector<SpaceRoom>>,
    expanded_spaces: &HashSet<OwnedRoomId>,
    loading_subspaces: &HashSet<OwnedRoomId>,
    tree_entries: &mut Vec<TreeEntry>,
    space_id: &OwnedRoomId,
    level: usize,
    parent_mask: u32,
    visited: &mut HashSet<OwnedRoomId>,
) {
    let Some(children) = children_cache.get(space_id) else { return };

    // Preserve the SDK's spec-compliant `m.space.child` ordering: the SDK already
    // sorts children per the spec's "Ordering of children within a space"
    // (`m.space.child` `order`, then the child event's timestamp, then room ID).
    // Re-sorting here would silently discard the ordering a space's admins
    // deliberately set.
    let sorted_children: Vec<_> = children.iter().collect();
    let count = sorted_children.len();

    for (i, child) in sorted_children.into_iter().enumerate() {
        // Cycle/duplicate-edge guard: skip any room ID we've already emitted
        // anywhere in this tree (whether reached via a genuine `m.space.child`
        // cycle, a self-loop, or a duplicate edge within the same parent's list).
        if !visited.insert(child.room_id.clone()) {
            continue;
        }

        let is_last = i == count - 1;

        tree_entries.push(TreeEntry::Item {
            info: SpaceRoomInfo::from(child),
            parent_space_id: space_id.clone(),
            level,
            is_last,
            parent_mask,
        });

        // If this is an expanded space, recursively add its children or a loading indicator
        if child.is_space() && expanded_spaces.contains(&child.room_id) {
            // Calculate mask for children:
            // If we are NOT the last child, our level needs a continuation line for our children.
            // If we ARE the last child, our level does NOT need a line.
            // Parent levels are preserved.
            let child_mask = if is_last {
                parent_mask
            } else {
                parent_mask | (1 << level)
            };

            if children_cache.contains_key(&child.room_id) {
                build_tree_for_space_inner(
                    children_cache,
                    expanded_spaces,
                    loading_subspaces,
                    tree_entries,
                    &child.room_id,
                    level + 1,
                    child_mask,
                    visited,
                );
            } else if loading_subspaces.contains(&child.room_id) {
                // Show loading indicator
                tree_entries.push(TreeEntry::Loading {
                    level: level + 1,
                    parent_mask: child_mask,
                });
            }
        }
    }
}

/// Recursively build a filtered tree that includes only entries matching
/// the keywords, plus any ancestor spaces needed to preserve the hierarchy.
///
/// Returns `true` if any matching entry was added within this subtree.
///
/// Cycle-safe: see the module-level note above.
fn build_filtered_tree(
    children_cache: &HashMap<OwnedRoomId, Vector<SpaceRoom>>,
    tree_entries: &mut Vec<TreeEntry>,
    space_id: &OwnedRoomId,
    keywords: &str,
    level: usize,
    parent_mask: u32,
) -> bool {
    let mut visited = HashSet::new();
    visited.insert(space_id.clone());
    build_filtered_tree_inner(children_cache, tree_entries, space_id, keywords, level, parent_mask, &mut visited)
}

#[allow(clippy::too_many_arguments)]
fn build_filtered_tree_inner(
    children_cache: &HashMap<OwnedRoomId, Vector<SpaceRoom>>,
    tree_entries: &mut Vec<TreeEntry>,
    space_id: &OwnedRoomId,
    keywords: &str,
    level: usize,
    parent_mask: u32,
    visited: &mut HashSet<OwnedRoomId>,
) -> bool {
    let Some(children) = children_cache.get(space_id) else { return false };

    // Sort identically to the unfiltered tree: spaces first, then rooms, both alphabetically.
    // Keep the order the SpaceRoomList gave us: the SDK already sorts children
    // per the spec's "Ordering of children within a space" — `m.space.child`
    // `order`, then the child event's timestamp, then room ID. Re-sorting here
    // (e.g. spaces-first, alphabetical) would silently discard the ordering a
    // space's admins deliberately set.
    let sorted_children: Vec<_> = children.iter().collect();

    // First pass: determine which children have matches (self or descendants)
    // so we can correctly compute `is_last` for tree line drawing.
    let matched_indices: Vec<usize> = sorted_children.iter().enumerate().filter_map(|(i, child)| {
        let info = SpaceRoomInfo::from(*child);
        let self_matches = matches_filter(&info, keywords);
        let has_matching_descendants = child.is_space()
            && children_cache.contains_key(&child.room_id)
            && subtree_has_match(children_cache, &child.room_id, keywords);
        if self_matches || has_matching_descendants {
            Some(i)
        } else {
            None
        }
    }).collect();

    if matched_indices.is_empty() {
        return false;
    }

    // Second pass: emit entries for matched children, preserving hierarchy.
    for (pos, &child_idx) in matched_indices.iter().enumerate() {
        let child = sorted_children[child_idx];

        // Cycle/duplicate-edge guard, see `build_tree_for_space_inner`.
        if !visited.insert(child.room_id.clone()) {
            continue;
        }

        let is_last = pos == matched_indices.len() - 1;
        let info = SpaceRoomInfo::from(child);
        let self_matches = matches_filter(&info, keywords);

        let child_mask = if is_last {
            parent_mask
        } else {
            parent_mask | (1 << level)
        };

        if child.is_space() && children_cache.contains_key(&child.room_id) {
            // For spaces: always include if self matches or descendants match.
            tree_entries.push(TreeEntry::Item {
                info,
                parent_space_id: space_id.clone(),
                level,
                is_last,
                parent_mask,
            });
            // Recurse into child space: if the space itself matches,
            // show ALL of its children (unfiltered); otherwise show only
            // the matching descendants.
            if self_matches {
                // Show all children of a matching space (no further filtering).
                build_tree_for_space_ignoring_expansion(
                    children_cache,
                    tree_entries,
                    &child.room_id,
                    level + 1,
                    child_mask,
                );
            } else {
                // Space doesn't match, but some descendant does — recurse with filter.
                build_filtered_tree_inner(
                    children_cache,
                    tree_entries,
                    &child.room_id,
                    keywords,
                    level + 1,
                    child_mask,
                    visited,
                );
            }
        } else if self_matches {
            // Non-space room or space without cached children: include only if it matches.
            tree_entries.push(TreeEntry::Item {
                info,
                parent_space_id: space_id.clone(),
                level,
                is_last,
                parent_mask,
            });
        }
    }

    true
}

/// Returns `true` if any entry in the subtree rooted at `space_id` matches the keywords.
///
/// Cycle-safe: see the module-level note above.
fn subtree_has_match(
    children_cache: &HashMap<OwnedRoomId, Vector<SpaceRoom>>,
    space_id: &OwnedRoomId,
    keywords: &str,
) -> bool {
    let mut visited = HashSet::new();
    visited.insert(space_id.clone());
    subtree_has_match_inner(children_cache, space_id, keywords, &mut visited)
}

fn subtree_has_match_inner(
    children_cache: &HashMap<OwnedRoomId, Vector<SpaceRoom>>,
    space_id: &OwnedRoomId,
    keywords: &str,
    visited: &mut HashSet<OwnedRoomId>,
) -> bool {
    let Some(children) = children_cache.get(space_id) else { return false };
    for child in children.iter() {
        let info = SpaceRoomInfo::from(child);
        if matches_filter(&info, keywords) {
            return true;
        }
        if child.is_space()
            && visited.insert(child.room_id.clone())
            && subtree_has_match_inner(children_cache, &child.room_id, keywords, visited)
        {
            return true;
        }
    }
    false
}

/// Like [`build_tree_for_space`] but ignores expansion state — shows all children.
/// Used to display the full contents of a space that itself matched the filter.
///
/// Cycle-safe: see the module-level note above.
fn build_tree_for_space_ignoring_expansion(
    children_cache: &HashMap<OwnedRoomId, Vector<SpaceRoom>>,
    tree_entries: &mut Vec<TreeEntry>,
    space_id: &OwnedRoomId,
    level: usize,
    parent_mask: u32,
) {
    let mut visited = HashSet::new();
    visited.insert(space_id.clone());
    build_tree_for_space_ignoring_expansion_inner(children_cache, tree_entries, space_id, level, parent_mask, &mut visited);
}

fn build_tree_for_space_ignoring_expansion_inner(
    children_cache: &HashMap<OwnedRoomId, Vector<SpaceRoom>>,
    tree_entries: &mut Vec<TreeEntry>,
    space_id: &OwnedRoomId,
    level: usize,
    parent_mask: u32,
    visited: &mut HashSet<OwnedRoomId>,
) {
    let Some(children) = children_cache.get(space_id) else { return };

    // Preserve the SDK's spec-compliant `m.space.child` ordering (see above).
    let sorted_children: Vec<_> = children.iter().collect();

    let count = sorted_children.len();
    for (i, child) in sorted_children.into_iter().enumerate() {
        if !visited.insert(child.room_id.clone()) {
            continue;
        }

        let is_last = i == count - 1;
        tree_entries.push(TreeEntry::Item {
            info: SpaceRoomInfo::from(child),
            parent_space_id: space_id.clone(),
            level,
            is_last,
            parent_mask,
        });

        if child.is_space() && children_cache.contains_key(&child.room_id) {
            let child_mask = if is_last {
                parent_mask
            } else {
                parent_mask | (1 << level)
            };
            build_tree_for_space_ignoring_expansion_inner(
                children_cache,
                tree_entries,
                &child.room_id,
                level + 1,
                child_mask,
                visited,
            );
        }
    }
}

/// The view showing the lobby/homepage for a given space.
#[derive(Script, ScriptHook, Widget)]
pub struct SpaceLobbyScreen {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,

    /// The space that is currently being displayed.
    #[rust] space_name_id: Option<RoomNameId>,
    #[rust] space_avatar_state: AvatarState,

    /// The sender channel to submit space requests to the background service.
    #[rust] space_request_sender: Option<UnboundedSender<SpaceRequest>>,

    /// Cache of detailed children for each space we've fetched.
    /// Key is the space_id, value is the list of its direct children.
    #[rust] children_cache: HashMap<OwnedRoomId, Vector<SpaceRoom>>,

    /// The set of space IDs that are currently expanded (showing their children).
    #[rust] expanded_spaces: HashSet<OwnedRoomId>,

    /// The ordered list of children to display in the space tree.
    #[rust] tree_entries: Vec<TreeEntry>,

    /// The set of space IDs that are currently loading their children.
    #[rust] loading_subspaces: HashSet<OwnedRoomId>,

    /// Whether we are currently loading the initial data.
    #[rust] is_loading: bool,
    #[rust] top_level_join_rule: Option<JoinRuleSummary>,
    #[rust] top_level_member_count: Option<u64>,
    #[rust] app_language: AppLanguage,

    /// The current filter keywords entered by the user, if any.
    #[rust] filter_keywords: String,
    /// Spaces where this user may add or remove children.
    #[rust] creatable_spaces: HashSet<OwnedRoomId>,
    /// Spaces whose own settings (name/topic/avatar) this user may edit.
    #[rust] manageable_spaces: HashSet<OwnedRoomId>,
}

impl Widget for SpaceLobbyScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if self.app_language != app_language {
            self.app_language = app_language;
            self.update_space_info_label(cx, app_language);
            self.redraw(cx);
        }
        self.view.handle_event(cx, event, scope);

        // Handle Signal events for avatar cache updates
        if let Event::Signal = event {
            // Process any pending avatar updates
            avatar_cache::process_avatar_updates(cx);
            self.redraw(cx);
        }

        if let Event::Actions(actions) = event {
            for action in actions {
                if let Some(CreatableSpacesAction::Loaded { spaces, manageable_spaces }) = action.downcast_ref() {
                    self.creatable_spaces = spaces.iter()
                        .map(|space| space.room_id().clone())
                        .collect();
                    self.manageable_spaces = manageable_spaces.iter()
                        .map(|space| space.room_id().clone())
                        .collect();
                    self.sync_header_action_buttons(cx);
                    self.redraw(cx);
                }

                match action.downcast_ref() {
                    Some(SpaceRoomListAction::DetailedChildren { space_id, children, .. }) => {
                        self.update_children_in_space(cx, space_id, children);
                    }

                    // Handle receiving top-level space details (join rule, member count).
                    Some(SpaceRoomListAction::TopLevelSpaceDetails(sr))
                        if self.space_name_id.as_ref().is_some_and(|sni| sni.room_id() == &sr.room_id) => {
                        self.space_avatar_state = AvatarState::Known(sr.avatar_url.clone());
                        self.space_avatar_state.update_from_cache(cx); // prefetch the avatar image
                        self.top_level_join_rule = sr.join_rule.clone();
                        self.top_level_member_count = Some(sr.num_joined_members);
                        self.update_space_info_label(cx, app_language);
                        self.redraw(cx);
                    }
                    Some(SpaceRoomListAction::TopLevelSpaceDetails(..)) => {}

                    // Handle a change to the set of children in this space or any of its child subspaces.
                    Some(SpaceRoomListAction::UpdatedChildren { space_id, parent_chain, .. })
                        if self.space_name_id.as_ref().is_some_and(|sni|
                            sni.room_id() == space_id
                            || parent_chain.iter().any(|ancestor_id| sni.room_id() == ancestor_id)
                        ) => {
                        if let Some(sender) = &self.space_request_sender {
                            let _ = sender.send(SpaceRequest::GetDetailedChildren {
                                space_id: space_id.clone(),
                                parent_chain: parent_chain.clone(),
                            });
                        }
                    }
                    Some(SpaceRoomListAction::UpdatedChildren { .. }) => {}
                    _ => { }
                }

                // A space's children changed as a result of us adding or removing one.
                if let Some(space_child_action) = action.downcast_ref::<SpaceChildAction>() {
                    let (space_id, child, error, was_added) = match space_child_action {
                        SpaceChildAction::Added { space_id, child, error } => (space_id, child, error, true),
                        SpaceChildAction::Removed { space_id, child, error } => (space_id, child, error, false),
                    };
                    let child_name = child.to_string();
                    let (message, kind) = match error {
                        None => (
                            tr_fmt(app_language, if was_added {
                                "space_lobby.popup.added_to_space"
                            } else {
                                "space_lobby.popup.removed_from_space"
                            }, &[("child_name", child_name.as_str())]),
                            PopupKind::Success,
                        ),
                        Some(error) => (
                            tr_fmt(app_language, if was_added {
                                "space_lobby.popup.add_to_space_failed"
                            } else {
                                "space_lobby.popup.remove_from_space_failed"
                            }, &[("child_name", child_name.as_str()), ("error", error.as_str())]),
                            PopupKind::Error,
                        ),
                    };
                    enqueue_popup_notification(message, kind, Some(5.0));
                    // `Added` always refreshes, even on a reported failure: attaching a
                    // room is best-effort (the primary `m.space.child` write can succeed
                    // while the advisory `m.space.parent` backlink write fails and gets
                    // reported as an `error` here), so the UI must reconcile with
                    // whatever the server actually ended up with rather than trusting a
                    // partial-failure error to mean nothing changed. `Removed` keeps the
                    // original success-only refresh, since it has no such partial-failure
                    // case (detach is already fully best-effort at the request layer).
                    if was_added || error.is_none() {
                        // The homeserver's hierarchy view is what the tree renders, so
                        // re-fetch that space's children rather than editing it locally.
                        refresh_space_children(cx, space_id);
                    }
                    continue;
                }

                if let Some(CreateRoomAction::Created { room_name_id, parent_space_id, space_link_error, is_space, .. }) = action.downcast_ref() {
                    if space_link_error.is_none()
                        && parent_space_id.as_ref() == self.space_name_id.as_ref().map(RoomNameId::room_id)
                    {
                        self.insert_created_room_placeholder(cx, room_name_id, *is_space);
                    }
                }

                // Handle SubspaceEntry clicks
                match action.as_widget_action().cast_ref() {
                    SubspaceEntryAction::SpaceClicked { space_id } => {
                        self.toggle_space_expansion(cx, space_id);
                    }
                    SubspaceEntryAction::RoomClicked { room_id: _ } => {
                        // TODO: highlight the room, such that on mobile devices
                        //       it will behave just like we hovered-in on desktop platforms.
                    }
                    SubspaceEntryAction::JoinClicked { room_id, is_space } => {
                        cx.action(JoinLeaveRoomModalAction::Open {
                            kind: JoinLeaveModalKind::JoinRoom {
                                details: self.basic_room_details_for(room_id),
                                is_space: *is_space,
                            },
                            show_tip: false,
                        });
                    }
                    SubspaceEntryAction::LeaveClicked { room_id, is_space } => {
                        if *is_space {
                            if let Some(space_request_sender) = self.space_request_sender.clone() {
                                cx.action(JoinLeaveRoomModalAction::Open {
                                    kind: JoinLeaveModalKind::LeaveSpace {
                                        details: self.basic_room_details_for(room_id),
                                        space_request_sender,
                                    },
                                    show_tip: false,
                                });
                            }
                        } else {
                            cx.action(JoinLeaveRoomModalAction::Open {
                                kind: JoinLeaveModalKind::LeaveRoom(
                                    self.basic_room_details_for(room_id)
                                ),
                                show_tip: false,
                            });
                        }
                    }
                    SubspaceEntryAction::RemoveFromSpaceClicked { room_id, is_space } => {
                        self.confirm_remove_from_space(cx, room_id, *is_space);
                    }
                    SubspaceEntryAction::ViewClicked { room_id } => {
                        cx.action(AppStateAction::NavigateToRoom {
                            room_to_close: None,
                            destination_room: self.basic_room_details_for(room_id),
                        });
                    }
                    SubspaceEntryAction::None => { }
                }
            }

            if self.view.button(cx, ids!(header.parent_space_row.create_room_button)).clicked(actions) {
                if self.can_create_room_in_current_space()
                    && let Some(space_name_id) = self.space_name_id.as_ref()
                {
                    cx.action(CreateRoomModalAction::Open {
                        parent_space_id: Some(space_name_id.room_id().clone()),
                        create_space: false,
                    });
                }
            }

            // Adding an existing room writes the same `m.space.child` state as
            // creating one, so it is behind the same permission gate.
            if self.view.button(cx, ids!(header.parent_space_row.add_existing_room_button)).clicked(actions) {
                if self.can_create_room_in_current_space()
                    && let Some(space_name_id) = self.space_name_id.clone()
                {
                    let existing_children = self.children_cache
                        .get(space_name_id.room_id())
                        .map(|children| children.iter().map(|c| c.room_id.clone()).collect())
                        .unwrap_or_default();
                    cx.action(AddExistingRoomModalAction::Open {
                        space_name_id,
                        existing_children,
                    });
                }
            }

            // Same permission gate as creating a room: adding a subspace also
            // requires being able to send `m.space.child` in this space.
            if self.view.button(cx, ids!(header.parent_space_row.create_subspace_button)).clicked(actions) {
                if self.can_create_room_in_current_space()
                    && let Some(space_name_id) = self.space_name_id.as_ref()
                {
                    cx.action(CreateRoomModalAction::Open {
                        parent_space_id: Some(space_name_id.room_id().clone()),
                        create_space: true,
                    });
                }
            }

            // Space settings reuse the room settings modal: a space is a room
            // underneath, so name/topic/avatar/addresses are all the same state.
            if self.view.button(cx, ids!(header.parent_space_row.settings_button)).clicked(actions) {
                if self.can_manage_current_space()
                    && let Some(space_name_id) = self.space_name_id.as_ref()
                {
                    cx.action(RoomSettingsAction::Open {
                        room_id: space_name_id.room_id().clone(),
                        room_name: Some(space_name_id.to_string()),
                        is_space: true,
                    });
                }
            }

            // Leave the space (and the rooms joined inside it). Reuses the same
            // confirmation + backend path as leaving a subspace from the tree.
            if self.view.button(cx, ids!(header.parent_space_row.leave_space_button)).clicked(actions) {
                if let (Some(space_name_id), Some(space_request_sender)) =
                    (self.space_name_id.clone(), self.space_request_sender.clone())
                {
                    cx.action(JoinLeaveRoomModalAction::Open {
                        kind: JoinLeaveModalKind::LeaveSpace {
                            details: BasicRoomDetails::Name(space_name_id),
                            space_request_sender,
                        },
                        show_tip: false,
                    });
                }
            }

            // Handle the invite button being clicked in the header.
            if self.view.button(cx, ids!(header.parent_space_row.invite_button)).clicked(actions) {
                if let Some(space_name_id) = self.space_name_id.as_ref() {
                    cx.action(InviteModalAction::Open(space_name_id.clone()));
                }
            }

            // Handle changes to this screen's own filter input bar.
            if let Some(keywords) = self.view.room_filter_input_bar(cx, ids!(filter_bar)).changed(actions) {
                self.filter_keywords = keywords;
                self.rebuild_tree_entries();
                // Reset scroll to the top when filter changes.
                let portal_list = self.view.portal_list(cx, ids!(tree_list));
                portal_list.set_first_id_and_scroll(0, 0.0);
                self.redraw(cx);
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        self.app_language = app_language;

        // Draw parent avatar from the SpaceRoom's avatar URL, or show initials.
        let parent_avatar_ref = self.view.avatar(cx, ids!(parent_avatar));
        if self.space_avatar_state.update_from_cache(cx).is_none_or(|data| {
            parent_avatar_ref.show_image(
                cx,
                None,
                |cx, img| utils::load_png_or_jpg(&img, cx, data),
            ).is_err()
        }) {
            let first_char = self.space_name_id.as_ref().and_then(|sni| sni.name_for_avatar())
                .and_then(|name| utils::user_name_first_letter(name));
            parent_avatar_ref.show_text(cx, None, None, first_char.unwrap_or("S"));
        }

        self.update_space_info_label(cx, app_language);
        self.sync_header_action_buttons(cx);
        self.view.button(cx, ids!(header.parent_space_row.create_room_button))
            .set_text(cx, tr_key(app_language, "space_lobby.header.button.new_room"));
        self.view.button(cx, ids!(header.parent_space_row.add_existing_room_button))
            .set_text(cx, tr_key(app_language, "space_lobby.header.button.add_existing_room"));
        self.view.button(cx, ids!(header.parent_space_row.create_subspace_button))
            .set_text(cx, tr_key(app_language, "space_lobby.header.button.new_subspace"));
        self.view.button(cx, ids!(header.parent_space_row.settings_button))
            .set_text(cx, tr_key(app_language, "space_lobby.header.button.settings"));
        self.view.button(cx, ids!(header.parent_space_row.invite_button))
            .set_text(cx, tr_key(app_language, "space_lobby.header.button.invite"));
        self.view.button(cx, ids!(header.parent_space_row.leave_space_button))
            .set_text(cx, tr_key(app_language, "space_lobby.header.button.leave"));
        
        while let Some(widget_to_draw) = self.view.draw_walk(cx, scope, walk).step() {
            let portal_list_ref = widget_to_draw.as_portal_list();
            let Some(mut list) = portal_list_ref.borrow_mut() else { continue };

            let entry_count = self.tree_entries.len();
            let total_count = if self.is_loading || entry_count == 0 {
                2 // status label + filler
            } else {
                entry_count + 1 // entries + filler
            };

            list.set_item_range(cx, 0, total_count);

            while let Some(item_id) = list.next_visible_item(cx) {
                // NOTE: Use child_by_path instead of widget tree-based lookups
                //       (e.g., item.label(), item.avatar(), item.widget())
                //       because WidgetRef::widget() fails for portal list items.

                // Draw loading indicator
                let item = if self.is_loading && item_id == 0 {
                    let item = list.item(cx, item_id, id!(status_label));
                    item.child_by_path(ids!(label)).as_label().set_text(
                        cx,
                        tr_key(app_language, "space_lobby.status.loading_rooms_spaces"),
                    );
                    item.child_by_path(ids!(loading_spinner)).set_visible(cx, true);
                    item
                }
                // No entries found
                else if entry_count == 0 && item_id == 0 {
                    let item = list.item(cx, item_id, id!(status_label));
                    let msg = if self.filter_keywords.is_empty() {
                        tr_key(app_language, "space_lobby.status.no_rooms_spaces")
                    } else {
                        tr_key(app_language, "space_lobby.status.no_matching_rooms_spaces")
                    };
                    item.child_by_path(ids!(label)).as_label().set_text(cx, msg);
                    item.child_by_path(ids!(loading_spinner)).set_visible(cx, false);
                    item
                }
                // Draw a regular entry
                else if let Some(show_remove_button) = self.tree_entries.get(item_id).map(|entry| match entry {
                    // Unlinking a child edits the *parent* space's state, so the permission
                    // that matters is the one on that parent (which may be a nested subspace).
                    // Resolved before the mutable borrow of `tree_entries` below.
                    TreeEntry::Item { parent_space_id, .. } => self.creatable_spaces.contains(parent_space_id),
                    TreeEntry::Loading { .. } => false,
                }) && let Some(entry) = self.tree_entries.get_mut(item_id) {
                    match entry {
                        TreeEntry::Item { info, level, is_last, parent_mask, .. } => {
                            let show_join_button = !matches!(info.state, Some(RoomState::Joined));
                            let show_leave_button = !show_join_button;
                            let show_view_button = show_leave_button && !info.is_space();
                            let item = if info.is_space() {
                                let item = list.item(cx, item_id, id!(subspace_entry));
                                let is_expanded = self.expanded_spaces.contains(&info.id);
                                let mut show_buttons_view = false;
                                let mut need_snap = false;
                                if let Some(mut inner) = item.borrow_mut::<SubspaceEntry>() {
                                    let id_changed = inner.room_id.as_ref() != Some(&info.id);
                                    need_snap = id_changed || inner.is_expanded != is_expanded;
                                    inner.room_id = Some(info.id.clone());
                                    inner.is_space = true;
                                    inner.is_expanded = is_expanded;
                                    if id_changed {
                                        inner.show_buttons_view = false;
                                        inner.buttons_shown_by_tap = false;
                                    }
                                    show_buttons_view = inner.show_buttons_view;
                                }
                                item.child_by_path(ids!(buttons_view)).set_visible(cx, show_buttons_view);
                                // Snap expand arrow to correct state without animation
                                // when item is reused or state changed externally
                                if need_snap {
                                    if let Some(mut arrow) = item.child_by_path(ids!(main_entry.expand_icon)).borrow_mut::<ExpandArrow>() {
                                        arrow.set_is_open(cx, is_expanded, Animate::No);
                                    }
                                }
                                item
                            } else {
                                let item = list.item(cx, item_id, id!(room_entry));
                                let mut show_buttons_view = false;
                                if let Some(mut inner) = item.borrow_mut::<SubspaceEntry>() {
                                    let id_changed = inner.room_id.as_ref() != Some(&info.id);
                                    inner.room_id = Some(info.id.clone());
                                    inner.is_space = false;
                                    if id_changed {
                                        inner.show_buttons_view = false;
                                        inner.buttons_shown_by_tap = false;
                                    }
                                    show_buttons_view = inner.show_buttons_view;
                                }
                                item.child_by_path(ids!(buttons_view)).set_visible(cx, show_buttons_view);
                                item
                            };

                            item.child_by_path(ids!(buttons_view.join_button)).set_visible(cx, show_join_button);
                            item.child_by_path(ids!(buttons_view.leave_button)).set_visible(cx, show_leave_button);
                            item.child_by_path(ids!(buttons_view.view_button)).set_visible(cx, show_view_button);
                            item.child_by_path(ids!(buttons_view.join_button)).as_button().set_text(
                                cx,
                                tr_key(app_language, "space_lobby.item.button.join"),
                            );
                            item.child_by_path(ids!(buttons_view.leave_button)).as_button().set_text(
                                cx,
                                tr_key(app_language, "space_lobby.item.button.leave"),
                            );
                            item.child_by_path(ids!(buttons_view.view_button)).as_button().set_text(
                                cx,
                                tr_key(app_language, "space_lobby.item.button.view"),
                            );
                            item.child_by_path(ids!(buttons_view.remove_from_space_button))
                                .set_visible(cx, show_remove_button);
                            item.child_by_path(ids!(buttons_view.remove_from_space_button)).as_button().set_text(
                                cx,
                                tr_key(app_language, "space_lobby.item.button.remove_from_space"),
                            );

                            // Below, draw things that are common to child rooms and subspaces.
                            item.child_by_path(ids!(main_entry.content.name_label)).as_label().set_text(cx, &info.name);

                            // Display avatar from stored data, or fetch from cache, or show initials
                            let avatar_ref = item.child_by_path(ids!(main_entry.avatar)).as_avatar();
                            let first_char = utils::user_name_first_letter(&info.name);
                            let mut drew_avatar = false;

                            match &info.avatar {
                                AvatarState::Loaded(data) => {
                                    drew_avatar = avatar_ref.show_image(
                                        cx,
                                        None,
                                        |cx, img| utils::load_png_or_jpg(&img, cx, data),
                                    ).is_ok();
                                }
                                AvatarState::Known(Some(uri)) => {
                                    match avatar_cache::get_or_fetch_avatar(cx, uri) {
                                        AvatarCacheEntry::Loaded(data) => {
                                            drew_avatar = avatar_ref.show_image(
                                                cx,
                                                None,
                                                |cx, img| utils::load_png_or_jpg(&img, cx, &data),
                                            ).is_ok();
                                            info.avatar = AvatarState::Loaded(data);
                                        }
                                        AvatarCacheEntry::Failed => {
                                            info.avatar = AvatarState::Failed;
                                        }
                                        AvatarCacheEntry::Requested => { }
                                    }
                                }
                                _ => { }
                            };
                            // Fallback to text initials.
                            if !drew_avatar {
                                avatar_ref.show_text(cx, None, None, first_char.unwrap_or("#"));
                            }

                            let indent_width = TREE_INDENT_WIDTH as f32;
                            if let Some(mut lines) = item.child_by_path(ids!(tree_lines)).borrow_mut::<TreeLines>() {
                                lines.draw_bg.level = *level as f32;
                                lines.draw_bg.is_last = if *is_last { 1.0 } else { 0.0 };
                                lines.draw_bg.parent_mask = *parent_mask as f32;
                                lines.draw_bg.indent_width = indent_width;
                            }
                            // Set the indent spacer width to match the tree indentation.
                            let indent_pixel = (*level as f64 + 1.0) * TREE_INDENT_WIDTH;
                            if let Some(mut spacer) = item.child_by_path(ids!(main_entry.indent_spacer)).borrow_mut::<View>() {
                                spacer.walk.width = Size::Fixed(indent_pixel);
                            }

                            // Show "Suggested" tag if recommended and not already joined
                            let show_suggested = info.suggested
                                && !matches!(info.state, Some(RoomState::Joined));
                            item.child_by_path(ids!(main_entry.content.suggested_tag))
                                .set_visible(cx, show_suggested);

                            // Build the info label with join status, member count, and topic
                            // Note: Public/Private is intentionally not shown per-item to reduce clutter
                            let info_label = item.child_by_path(ids!(main_entry.content.info_label)).as_label();
                            let mut info_parts = Vec::new();

                            // Add join status for rooms we haven't joined
                            if let Some(state) = &info.state {
                                match state {
                                    RoomState::Joined => info_parts.push(tr_key(app_language, "space_lobby.item.state.joined").to_string()),
                                    RoomState::Left => info_parts.push(tr_key(app_language, "space_lobby.item.state.left").to_string()),
                                    RoomState::Invited => info_parts.push(tr_key(app_language, "space_lobby.item.state.invited").to_string()),
                                    RoomState::Knocked => info_parts.push(tr_key(app_language, "space_lobby.item.state.knocked").to_string()),
                                    RoomState::Banned => info_parts.push(tr_key(app_language, "space_lobby.item.state.banned").to_string()),
                                }
                            }

                            // Add member count
                            let member_count = info.num_joined_members.to_string();
                            info_parts.push(if info.num_joined_members == 1 {
                                tr_key(app_language, "space_lobby.item.member_one").to_string()
                            } else {
                                tr_fmt(app_language, "space_lobby.item.member_n", &[("count", member_count.as_str())])
                            });

                            // Add children count for spaces
                            if let Some(c) = info.children_count {
                                if c > 0 {
                                    let child_count = c.to_string();
                                    info_parts.push(if c == 1 {
                                        tr_fmt(app_language, "space_lobby.item.child_room_one", &[("count", child_count.as_str())])
                                    } else {
                                        tr_fmt(app_language, "space_lobby.item.child_room_n", &[("count", child_count.as_str())])
                                    });
                                }
                            }

                            // Add topic if available (Label handles truncation via flow: Flow.Right{wrap: false})
                            if let Some(topic) = &info.topic {
                                if !topic.is_empty() {
                                    info_parts.push(topic.to_string());
                                }
                            }

                            info_label.set_text(cx, &info_parts.join("  |  "));

                            item
                        }
                        TreeEntry::Loading { level, parent_mask } => {
                            // Draw loading indicator for subspace
                            let item = list.item(cx, item_id, id!(subspace_loading));
                            item.child_by_path(ids!(label)).as_label().set_text(
                                cx,
                                tr_key(app_language, "space_lobby.status.loading"),
                            );
                            let indent_width = TREE_INDENT_WIDTH as f32;
                            // Configure tree lines
                            if let Some(mut lines) = item.child_by_path(ids!(tree_lines)).borrow_mut::<TreeLines>() {
                                lines.draw_bg.level = *level as f32;
                                lines.draw_bg.is_last = 1.0;
                                lines.draw_bg.parent_mask = *parent_mask as f32;
                                lines.draw_bg.indent_width = indent_width;
                            }
                            // Set the indent spacer width to match the tree indentation.
                            let indent_pixel = (*level as f64 + 1.0) * TREE_INDENT_WIDTH;
                            if let Some(mut spacer) = item.child_by_path(ids!(loading_content.indent_spacer)).borrow_mut::<View>() {
                                spacer.walk.width = Size::Fixed(indent_pixel);
                            }
                            item
                        }
                    }
                } else {
                    list.item(cx, item_id, id!(bottom_filler))
                };
                item.draw_all(cx, scope);
            }
        }

        DrawStep::done()
    }
}

impl SpaceLobbyScreen {
    /// Finds the given room/space ID in the tree and returns its basic details (including name).
    fn basic_room_details_for(&self, room_id: &OwnedRoomId) -> BasicRoomDetails {
        let room_name = self.tree_entries.iter().find_map(|entry| match entry {
            TreeEntry::Item { info, .. } if &info.id == room_id => Some(info.name.clone()),
            _ => None,
        });
        let room_name_id: RoomNameId = room_name
            .map(|name| RoomNameId::new(RoomDisplayName::Named(name), room_id.clone()))
            .unwrap_or_else(|| RoomNameId::empty(room_id.clone()));
        BasicRoomDetails::Name(room_name_id)
    }

    fn update_space_info_label(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        let text = if self.is_loading {
            tr_key(app_language, "space_lobby.header.welcome").to_string()
        } else if let Some(member_count) = self.top_level_member_count {
            let member_count_str = member_count.to_string();
            format!(
                "{}  ·  {}",
                match self.top_level_join_rule.as_ref() {
                    Some(JoinRuleSummary::Public) => tr_key(app_language, "space_lobby.header.public_space"),
                    _ => tr_key(app_language, "space_lobby.header.private_space"),
                },
                if member_count == 1 {
                    tr_key(app_language, "space_lobby.header.member_one").to_string()
                } else {
                    tr_fmt(app_language, "space_lobby.header.member_n", &[("count", member_count_str.as_str())])
                }
            )
        } else {
            String::new()
        };
        self.view.label(cx, ids!(header.space_info_row.space_info_label)).set_text(cx, &text);
    }

    /// The space that the given child entry hangs off of, which is the space
    /// whose `m.space.child` state has to change to unlink it.
    fn parent_space_of(&self, room_id: &OwnedRoomId) -> Option<OwnedRoomId> {
        self.tree_entries.iter().find_map(|entry| match entry {
            TreeEntry::Item { info, parent_space_id, .. } if &info.id == room_id => {
                Some(parent_space_id.clone())
            }
            _ => None,
        })
    }

    /// Asks the user to confirm unlinking a child from its parent space, then submits it.
    ///
    /// This is a space-wide change (everyone sees the child disappear from the space),
    /// so it is confirmed even though it neither leaves nor deletes the room itself.
    fn confirm_remove_from_space(&mut self, cx: &mut Cx, room_id: &OwnedRoomId, is_space: bool) {
        let Some(space_id) = self.parent_space_of(room_id) else {
            error!("BUG: no parent space found for child {room_id} being removed.");
            return;
        };
        let child = match self.basic_room_details_for(room_id) {
            BasicRoomDetails::Name(room_name_id) => room_name_id,
            other => other.room_name_id().clone(),
        };
        let parent_name = self.space_name_id.as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| space_id.to_string());
        let child_name = child.to_string();
        let app_language = self.app_language;

        let content = ConfirmationModalContent {
            title_text: Cow::Owned(tr_key(app_language, if is_space {
                "space_lobby.remove_from_space.confirm.title_space"
            } else {
                "space_lobby.remove_from_space.confirm.title_room"
            }).to_owned()),
            body_text: Cow::Owned(tr_fmt(app_language, if is_space {
                "space_lobby.remove_from_space.confirm.body_space"
            } else {
                "space_lobby.remove_from_space.confirm.body_room"
            }, &[
                ("child_name", child_name.as_str()),
                ("space_name", parent_name.as_str()),
            ])),
            accept_button_text: Some(Cow::Owned(
                tr_key(app_language, "space_lobby.remove_from_space.confirm.accept").to_owned()
            )),
            cancel_button_text: Some(Cow::Owned(
                tr_key(app_language, "space_lobby.item.button.cancel").to_owned()
            )),
            on_accept_clicked: Some(Box::new(move |_cx| {
                submit_async_request(MatrixRequest::RemoveRoomFromSpace { space_id, child });
            })),
            ..Default::default()
        };
        cx.action(ConfirmDeleteAction::Show(RefCell::new(Some(content))));
    }

    fn can_create_room_in_current_space(&self) -> bool {
        self.space_name_id.as_ref()
            .is_some_and(|space_name_id| self.creatable_spaces.contains(space_name_id.room_id()))
    }

    /// Whether this user may edit the displayed space's own settings.
    ///
    /// This is a different power level than adding children, so a user can be
    /// allowed to do one but not the other.
    fn can_manage_current_space(&self) -> bool {
        self.space_name_id.as_ref()
            .is_some_and(|space_name_id| self.manageable_spaces.contains(space_name_id.room_id()))
    }

    fn sync_header_action_buttons(&mut self, cx: &mut Cx) {
        let can_create = self.can_create_room_in_current_space();
        self.view.button(cx, ids!(header.parent_space_row.create_room_button))
            .set_visible(cx, can_create);
        self.view.button(cx, ids!(header.parent_space_row.add_existing_room_button))
            .set_visible(cx, can_create);
        self.view.button(cx, ids!(header.parent_space_row.create_subspace_button))
            .set_visible(cx, can_create);
        self.view.button(cx, ids!(header.parent_space_row.settings_button))
            .set_visible(cx, self.can_manage_current_space());
    }

    fn insert_created_room_placeholder(&mut self, cx: &mut Cx, room_name_id: &RoomNameId, is_space: bool) {
        let Some(space_id) = self.space_name_id.as_ref().map(|space| space.room_id().clone()) else {
            return;
        };
        let room_id = room_name_id.room_id().clone();
        let display_name = room_name_id.to_string();
        let mut children = self.children_cache.get(&space_id).cloned().unwrap_or_default();
        // A newly-created subspace must be tagged as a space, or the tree would
        // draw it as a plain room until the homeserver's hierarchy catches up.
        let room_type = is_space.then_some(RoomType::Space);

        if let Some(existing_index) = children.iter().position(|child| child.room_id == room_id) {
            if let Some(existing_child) = children.get_mut(existing_index) {
                existing_child.name = Some(display_name.clone());
                existing_child.display_name = display_name;
                existing_child.state = Some(RoomState::Joined);
                existing_child.num_joined_members = existing_child.num_joined_members.max(1);
                existing_child.room_type = room_type;
            }
        } else {
            children.push_back(SpaceRoom {
                room_id,
                canonical_alias: None,
                name: Some(display_name.clone()),
                display_name,
                topic: None,
                avatar_url: None,
                room_type,
                num_joined_members: 1,
                join_rule: None,
                world_readable: None,
                guest_can_join: false,
                is_direct: Some(false),
                children_count: 0,
                state: Some(RoomState::Joined),
                suggested: false,
                heroes: None,
                via: Vec::new(),
            });
        }

        self.children_cache.insert(space_id.clone(), children);
        self.is_loading = false;
        self.expanded_spaces.insert(space_id);
        self.rebuild_tree_entries();
        self.redraw(cx);
    }

    /// Handle receiving detailed children for a space.
    fn update_children_in_space(&mut self, cx: &mut Cx, space_id: &OwnedRoomId, children: &Vector<SpaceRoom>) {
        self.children_cache.insert(space_id.clone(), children.clone());
        self.loading_subspaces.remove(space_id);

        // If this is for our displayed space, mark as loaded and rebuild tree
        if self.space_name_id.as_ref().is_some_and(|sni| sni.room_id() == space_id) {
            self.is_loading = false;
            // Auto-expand the top-level space (we don't show it, just its children)
            self.expanded_spaces.insert(space_id.clone());
        }

        self.rebuild_tree_entries();
        self.redraw(cx);
    }

    /// Toggle the expansion state of a space.
    fn toggle_space_expansion(&mut self, cx: &mut Cx, space_id: &OwnedRoomId) {
        if self.expanded_spaces.contains(space_id) {
            self.expanded_spaces.remove(space_id);
            self.loading_subspaces.remove(space_id);
        } else {
            self.expanded_spaces.insert(space_id.clone());

            // Request children if we don't have them yet
            if !self.children_cache.contains_key(space_id) {
                self.loading_subspaces.insert(space_id.clone());
                if let Some(sender) = &self.space_request_sender {
                    let parent_chain = cx.get_global::<RoomsListRef>()
                        .get_space_parent_chain(space_id)
                        .unwrap_or_default();
                    let _ = sender.send(SpaceRequest::GetDetailedChildren {
                        space_id: space_id.clone(),
                        parent_chain,
                    });
                }
            }
        }

        self.rebuild_tree_entries();
        self.redraw(cx);
    }

    /// Rebuild the flattened tree entries based on the current expansion state,
    /// and then apply the current filter keywords (if any).
    fn rebuild_tree_entries(&mut self) {
        let Some(space_name_id) = &self.space_name_id else { return };
        let root_space_id = space_name_id.room_id().clone();
        let mut new_tree_entries = Vec::new();

        if self.filter_keywords.is_empty() {
            // No filter: build tree respecting expansion state.
            build_tree_for_space(
                &self.children_cache,
                &self.expanded_spaces,
                &self.loading_subspaces,
                &mut new_tree_entries,
                &root_space_id,
                0,
                0,
            );
        } else {
            // Filter active: build tree showing all matching entries
            // plus their ancestor spaces to preserve hierarchy context.
            let kw = self.filter_keywords.to_lowercase();
            build_filtered_tree(
                &self.children_cache,
                &mut new_tree_entries,
                &root_space_id,
                &kw,
                0,
                0,
            );
        }

        self.tree_entries = new_tree_entries;
    }

    /// Saves the current UI state to the cache. Call this when the screen is being hidden.
    pub fn save_current_state(&mut self) {
        if let Some(current_space) = &self.space_name_id {
            SPACE_LOBBY_STATES.with_borrow_mut(|states| {
                states.insert(
                    current_space.room_id().clone(),
                    SpaceLobbyUiState {
                        expanded_spaces: self.expanded_spaces.clone(),
                    },
                );
            });
        }
    }

    pub fn set_displayed_space(&mut self, cx: &mut Cx, space_name_id: &RoomNameId) {
        let space_name = space_name_id.display();
        let parent_name = self.view.label(cx, ids!(header.parent_space_row.parent_name));
        parent_name.set_text(cx, &space_name);

        // If this space is already being displayed, then the only thing we may need to do
        // is update its name in the top-level header (already done above).
        if self.space_name_id.as_ref().is_some_and(|sni| sni.room_id() == space_name_id.room_id()) {
            return;
        }

        // Save the current UI state before switching to a new space
        self.save_current_state();

        self.space_name_id = Some(space_name_id.clone());
        self.sync_header_action_buttons(cx);
        let rooms_list_ref = cx.get_global::<RoomsListRef>();
        if let Some(sender) = rooms_list_ref.get_space_request_sender() {
            // Request detailed children for this space so we can start populating it.
            let parent_chain_opt = rooms_list_ref.get_space_parent_chain(space_name_id.room_id());
            let _ = sender.send(SpaceRequest::GetDetailedChildren {
                space_id: space_name_id.room_id().clone(),
                parent_chain: parent_chain_opt.unwrap_or_default(),
            });
            let _ = sender.send(SpaceRequest::GetTopLevelSpaceDetails {
                space_id: space_name_id.room_id().clone(),
            });
            self.space_request_sender = Some(sender);
        }
        submit_async_request(MatrixRequest::GetCreatableSpaces);

        // Clear the main content until we receive the async space info responses.
        self.tree_entries.clear();
        self.top_level_join_rule = None;
        self.top_level_member_count = None;
        self.view.label(cx, ids!(header.space_info_row.space_info_label)).set_text(cx, "");
        self.is_loading = true;

        // Clear the filter bar when switching to a new space.
        self.filter_keywords.clear();
        self.view.text_input(cx, ids!(filter_bar.input)).set_text(cx, "");
        self.view.button(cx, ids!(filter_bar.clear_button)).set_visible(cx, false);

        // Restore UI state if we've viewed this space before, otherwise start fresh
        self.expanded_spaces = SPACE_LOBBY_STATES.with_borrow(|states| {
            states
                .get(space_name_id.room_id())
                .map(|state| state.expanded_spaces.clone())
                .unwrap_or_default()
        });

        // TODO: move avatar setting to `draw_walk()`
        // Set parent avatar
        let avatar_ref = self.view.avatar(cx, ids!(header.parent_space_row.parent_avatar));
        let first_char = utils::user_name_first_letter(&space_name);
        avatar_ref.show_text(cx, None, None, first_char.unwrap_or("#"));

        self.redraw(cx);
    }
}

impl SpaceLobbyScreenRef {
    pub fn set_displayed_space(&self, cx: &mut Cx, space_name_id: &RoomNameId) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_displayed_space(cx, space_name_id);
    }

    /// Saves the current UI state. Call this when the screen is being hidden or destroyed.
    pub fn save_current_state(&self) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.save_current_state();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::owned_room_id;

    /// Builds a minimal [`SpaceRoom`] fixture for use in tree-building tests.
    ///
    /// `children_count` should be non-zero for any node that itself has an
    /// entry in the `children_cache` map (i.e. is a space), so that
    /// [`SpaceRoomExt::is_space`] reports it correctly.
    fn space_room(id: &OwnedRoomId, children_count: u64) -> SpaceRoom {
        SpaceRoom {
            room_id: id.clone(),
            canonical_alias: None,
            name: Some(id.to_string()),
            display_name: id.to_string(),
            topic: None,
            avatar_url: None,
            room_type: if children_count > 0 { Some(RoomType::Space) } else { None },
            num_joined_members: 1,
            join_rule: None,
            world_readable: None,
            guest_can_join: false,
            is_direct: None,
            children_count,
            state: Some(RoomState::Joined),
            heroes: None,
            via: Vec::new(),
            suggested: false,
        }
    }

    /// Counts how many `TreeEntry::Item`s in `entries` have the given `room_id`.
    fn count_room(entries: &[TreeEntry], room_id: &OwnedRoomId) -> usize {
        entries.iter().filter(|e| matches!(e, TreeEntry::Item { info, .. } if &info.id == room_id)).count()
    }

    #[test]
    fn space_tree_cycle_two_nodes_terminates() {
        let a = owned_room_id!("!a:example.com");
        let b = owned_room_id!("!b:example.com");

        // A contains B, B contains A: a two-node cycle.
        let mut children_cache = HashMap::new();
        children_cache.insert(a.clone(), Vector::from(vec![space_room(&b, 1)]));
        children_cache.insert(b.clone(), Vector::from(vec![space_room(&a, 1)]));

        let mut expanded = HashSet::new();
        expanded.insert(a.clone());
        expanded.insert(b.clone());

        let mut entries = Vec::new();
        // Must terminate in finite steps (this call would previously stack-overflow).
        build_tree_for_space(&children_cache, &expanded, &HashSet::new(), &mut entries, &a, 0, 0);

        assert_eq!(count_room(&entries, &b), 1, "B must appear exactly once as A's child");
    }

    #[test]
    fn space_tree_self_loop_ignored() {
        let a = owned_room_id!("!a:example.com");

        // A contains itself.
        let mut children_cache = HashMap::new();
        children_cache.insert(a.clone(), Vector::from(vec![space_room(&a, 1)]));

        let mut expanded = HashSet::new();
        expanded.insert(a.clone());

        let mut entries = Vec::new();
        build_tree_for_space(&children_cache, &expanded, &HashSet::new(), &mut entries, &a, 0, 0);

        assert!(entries.is_empty(), "A must not appear as its own child: {entries:?}");
    }

    #[test]
    fn space_tree_duplicate_edges_deduplicated() {
        let a = owned_room_id!("!a:example.com");
        let r = owned_room_id!("!r:example.com");

        // A has two edges pointing to the same child room R.
        let mut children_cache = HashMap::new();
        children_cache.insert(a.clone(), Vector::from(vec![space_room(&r, 0), space_room(&r, 0)]));

        let mut entries = Vec::new();
        build_tree_for_space(&children_cache, &HashSet::new(), &HashSet::new(), &mut entries, &a, 0, 0);

        assert_eq!(count_room(&entries, &r), 1, "R must appear exactly once despite the duplicate edge");
    }

    #[test]
    fn space_tree_deep_chain_fully_built() {
        // Build a 50-level-deep chain: root -> s1 -> s2 -> ... -> s50, no cycles.
        const DEPTH: usize = 50;
        let ids: Vec<OwnedRoomId> = (0..=DEPTH)
            .map(|i| OwnedRoomId::try_from(format!("!s{i}:example.com")).unwrap())
            .collect();

        let mut children_cache = HashMap::new();
        let mut expanded = HashSet::new();
        for i in 0..DEPTH {
            let parent = &ids[i];
            let child = &ids[i + 1];
            // Every level but the last has one child, and each such space is expanded.
            children_cache.insert(parent.clone(), Vector::from(vec![space_room(child, 1)]));
            expanded.insert(parent.clone());
        }

        let mut entries = Vec::new();
        build_tree_for_space(&children_cache, &expanded, &HashSet::new(), &mut entries, &ids[0], 0, 0);

        // All 50 descendant levels (ids[1..=50]) must appear in the tree.
        for id in &ids[1..=DEPTH] {
            assert_eq!(count_room(&entries, id), 1, "node {id} missing from the fully-expanded deep chain");
        }
        assert_eq!(entries.len(), DEPTH, "expected exactly {DEPTH} entries in the deep chain");
    }

    #[test]
    fn space_tree_filter_terminates_on_cycle() {
        let a = owned_room_id!("!a:example.com");
        let b = owned_room_id!("!b:example.com");

        // A <-> B cycle.
        let mut children_cache = HashMap::new();
        children_cache.insert(a.clone(), Vector::from(vec![space_room(&b, 1)]));
        children_cache.insert(b.clone(), Vector::from(vec![space_room(&a, 1)]));

        // Must return in finite steps regardless of the keyword or which node we start from.
        let _ = subtree_has_match(&children_cache, &a, "anything");
        let _ = subtree_has_match(&children_cache, &b, "anything");
        let _ = subtree_has_match(&children_cache, &a, "");
    }
}
