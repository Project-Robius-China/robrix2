//! The NavigationTabBar shows a bar of icon buttons that allow the user to
//! navigate or switch between various top-level views in Robrix.
//!
//! The bar is positioned either within the left side bar (in the wide "Desktop" view mode)
//! or along the bottom of the app window (in the narrow "Mobile" view mode).
//!
//! Their order in Mobile view (horizontally from left to right) is:
//! 1. Home (house icon): the main view that shows all rooms across all spaces.
//! 2. Add Room (plus sign icon): a separate view that allows adding (joining) existing rooms,
//!    exploring public rooms, or creating new rooms/spaces.
//! 3. Spaces: a button that toggles the `SpacesBar` (shows/hides it).
//!    * This is NOT a regular radio button, it's a separate toggle. 
//!    * This is only shown in Mobile view mode, because the `SpacesBar` is always shown
//!      within the NavigationTabBar itself in Desktop view mode.
//! 4. Activity (an inbox, alert bell, or notifications icon): a separate view that shows
//!    a list of notifications, mentions, invitations, etc.
//! 5. Profile/Settings (user profile avatar): the existing `ProfileIcon` with a
//!    verification badge.
//!    * Upon click, this shows the SettingsScreen as normal.
//!
//! The order in Desktop view (vertically from top to bottom) is:
//! 1. Home
//! 2. Add/Join
//! 3. ----- separator -----
//!      SpacesBar content
//!    ----- separator -----
//! 4. Activity/Inbox
//! 5. Profile/Settings
//!

use makepad_widgets::*;
use serde::{Deserialize, Serialize};
use crate::{
    app::AppState, avatar_cache::{self, AvatarCacheEntry}, i18n::{AppLanguage, tr_fmt, tr_key}, login::login_screen::LoginAction, logout::logout_confirm_modal::LogoutAction, profile::{
        user_profile::UserProfile,
        user_profile_cache::{self, UserProfileUpdate},
    }, home::spaces_bar::SpacesBarWidgetExt, shared::{
        avatar::{AvatarState, AvatarWidgetExt}, styles::*, verification_badge::VerificationBadgeWidgetExt
    }, settings::app_preferences::{effective_is_desktop, AppPreferencesGlobal, AppPreferencesAction, ViewModeOverride}, sliding_sync::{current_user_id, AccountDataAction, AccountSwitchAction}, utils::{self, RoomNameId}
};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*


    // A RadioButton styled to fit within our NavigationTabBar.
    // Use RadioButtonTab as the base to stay aligned with current widgets/studio behavior.
    mod.widgets.NavigationTabButton = RadioButtonTab {
        width: Fill,
        height: (NAVIGATION_TAB_BAR_SIZE - 5),
        padding: (SPACE_XS),
        margin: (SPACE_XS),
        align: Align{x: 0.5, y: 0.5}
        flow: Down,
        text: "",

        icon_walk: Walk{
            margin: 0,
            width: (NAVIGATION_TAB_BAR_SIZE / 2.2),
            height: (NAVIGATION_TAB_BAR_SIZE / 2.2)
        }
        // Fully hide the text with zero size, zero margin, and zero spacing
        label_walk: Walk{margin: 0, width: 0, height: 0}
        spacing: 0,

        draw_bg +: {
            // Dark navy nav-rail item (visual spec §5.6 / RBX_NAV_* tokens):
            // transparent when idle so the navy rail shows through, a navy "pill"
            // on hover/active, plus a teal accent bar on the left of the *active*
            // item to echo the app-wide teal selection language.
            color: #0000
            color_hover: (RBX_NAV_ITEM_HOVER_BG)
            color_down: (RBX_NAV_ITEM_HOVER_BG)
            color_active: (RBX_NAV_ITEM_ACTIVE_BG)
            color_focus: #0000
            accent_color: instance((RBX_ACCENT))

            border_size: 0.0
            border_radius: (RBX_RADIUS_SM)
            border_color: #0000
            border_color_hover: #0000
            border_color_down: #0000
            border_color_active: #0000
            border_color_focus: #0000

            pixel: fn() {
                let sdf = Sdf2d.viewport(self.pos * self.rect_size)
                sdf.box(
                    self.border_size,
                    self.border_size,
                    self.rect_size.x - self.border_size * 2.0,
                    self.rect_size.y - self.border_size * 2.0,
                    self.border_radius
                )
                let color_fill = self.color
                    .mix(self.color_focus, self.focus)
                    .mix(self.color_active, self.active)
                    .mix(self.color_hover, self.hover)
                    .mix(self.color_down, self.down)
                sdf.fill(color_fill)
                // Teal selection bar on the left edge, shown only when active.
                let bar_inset = 12.0
                sdf.box(
                    0.0,
                    bar_inset,
                    3.0,
                    self.rect_size.y - bar_inset * 2.0,
                    1.5
                )
                sdf.fill(mix(vec4(0.0, 0.0, 0.0, 0.0), self.accent_color, self.active))
                return sdf.result
            }
        }

        draw_text +: {
            // Labels are hidden on the desktop rail (label_walk is zero-sized),
            // but keep the colors on the nav palette for correctness.
            color: (RBX_NAV_FG)
            color_hover: (RBX_NAV_FG_ACTIVE)
            color_down: (RBX_NAV_FG_ACTIVE)
            color_active: (RBX_NAV_FG_ACTIVE)
            color_focus: (RBX_NAV_FG)

            text_style: theme.font_bold {font_size: 9}
        }

        draw_icon +: {
            // DrawSvg has no per-state color mixing, so the active/idle icon color
            // is driven by the animator's `active` apply blocks below (white when
            // selected, muted grey otherwise) — mirrors MobileTabButton.
            color: (RBX_NAV_FG)
        }

        // Custom animator: drive the pill (draw_bg amounts), the (hidden) label and
        // — crucially — the icon color from the active/hover states. Selecting a tab
        // snaps the icon to white (RBX_NAV_FG_ACTIVE); deselecting returns it to the
        // muted RBX_NAV_FG. Mirrors the base RadioButton animator + MobileTabButton's
        // draw_icon trick.
        animator: Animator {
            disabled: {
                default: @off
                off: AnimatorState { from: {all: Forward {duration: 0.0}} apply: { draw_bg: {disabled: 0.0} draw_text: {disabled: 0.0} } }
                on:  AnimatorState { from: {all: Forward {duration: 0.2}} apply: { draw_bg: {disabled: 1.0} draw_text: {disabled: 1.0} } }
            }
            hover: {
                default: @off
                off:  AnimatorState { from: {all: Forward {duration: 0.15}} apply: { draw_bg: {down: snap(0.0), hover: 0.0} draw_text: {down: snap(0.0), hover: 0.0} } }
                on:   AnimatorState { from: {all: Snap} apply: { draw_bg: {down: snap(0.0), hover: 1.0} draw_text: {down: snap(0.0), hover: 1.0} } }
                down: AnimatorState { from: {all: Forward {duration: 0.2}} apply: { draw_bg: {down: snap(1.0), hover: 1.0} draw_text: {down: snap(1.0), hover: 1.0} } }
            }
            active: {
                default: @off
                off: AnimatorState { from: {all: Forward {duration: 0.2}} apply: { draw_bg: {active: 0.0} draw_text: {active: 0.0} draw_icon: {color: (RBX_NAV_FG)} } }
                on:  AnimatorState { from: {all: Forward {duration: 0.0}} apply: { draw_bg: {active: 1.0} draw_text: {active: 1.0} draw_icon: {color: (RBX_NAV_FG_ACTIVE)} } }
            }
            focus: {
                default: @off
                off: AnimatorState { from: {all: Forward {duration: 0.2}} apply: { draw_bg: {focus: 0.0} draw_text: {focus: 0.0} } }
                on:  AnimatorState { from: {all: Forward {duration: 0.0}} apply: { draw_bg: {focus: 1.0} draw_text: {focus: 1.0} } }
            }
        }
    }

    // A bottom-bar tab for the Mobile layout: a 24px icon stacked above a small
    // label, with no active "pill" — selection is shown purely by recoloring the
    // icon + label to the teal accent (see visual spec §4.14).
    //
    // NOTE: `draw_text` recolors itself via the RadioButton animator's `active`
    // state (color_active). `draw_icon` (DrawSvg) has no per-state color, so its
    // color is set imperatively in `NavigationTabBar::sync_selected_tab()`.
    mod.widgets.MobileTabButton = RadioButtonTab {
        width: Fill,
        height: Fill,
        padding: Inset{top: (SPACE_XS), bottom: (SPACE_XS), left: (SPACE_XS), right: (SPACE_XS)}
        margin: 0,
        align: Align{x: 0.5, y: 0.5}
        flow: Down,
        spacing: (SPACE_XS),
        text: "",

        icon_walk: Walk{ margin: 0, width: (RBX_ICON_MD), height: (RBX_ICON_MD) }
        // Full-width label box with centered text, so the label sits centered
        // under the icon regardless of its length.
        label_walk: Walk{ margin: 0, width: Fill, height: Fit }
        label_align: Align{x: 0.5, y: 0.0}

        draw_bg +: {
            // Transparent in every state — no pill, no border. The bar surface
            // behind the buttons provides the background color.
            color: #00000000
            color_active: #00000000
            color_disabled: #00000000
            border_size: 0.0
            border_radius: 0.0
            border_color: #0000
            border_color_hover: #0000
            border_color_down: #0000
            border_color_active: #0000
            border_color_focus: #0000
        }

        draw_text +: {
            // Only the *active* (selected) tab is teal; every other state stays
            // the inactive grey so hovering/pressing an inactive tab does not
            // flash it teal (spec §4.14: active=accent, inactive=secondary).
            color: (RBX_FG_SECONDARY)
            color_hover: (RBX_FG_SECONDARY)
            color_down: (RBX_FG_SECONDARY)
            color_active: (RBX_ACCENT)
            color_focus: (RBX_FG_SECONDARY)
            text_style: RBX_TEXT_META {}
        }

        draw_icon +: {
            color: (RBX_FG_SECONDARY)
        }

        // Drive the icon color from the same `active` animator state as the text
        // so icon + label switch to/from the accent color together on tap (no
        // lag). DrawSvg has no per-state color, so we set draw_icon.color
        // directly in the apply blocks. This mirrors the base RadioButton
        // animator (disabled / hover / active / focus) and only adds draw_icon.
        animator: Animator {
            disabled: {
                default: @off
                off: AnimatorState { from: {all: Forward {duration: 0.0}} apply: { draw_bg: {disabled: 0.0} draw_text: {disabled: 0.0} } }
                on:  AnimatorState { from: {all: Forward {duration: 0.2}} apply: { draw_bg: {disabled: 1.0} draw_text: {disabled: 1.0} } }
            }
            // Hover must NOT recolor the text/icon: the RadioButton mixes hover
            // AFTER active, so any hover color would override the selected tab's
            // teal. We only drive draw_bg on hover (which is transparent here, so
            // no visible change) — the tab color stays purely selection-driven.
            hover: {
                default: @off
                off:  AnimatorState { from: {all: Forward {duration: 0.15}} apply: { draw_bg: {down: snap(0.0), hover: 0.0} } }
                on:   AnimatorState { from: {all: Snap} apply: { draw_bg: {down: snap(0.0), hover: 1.0} } }
                down: AnimatorState { from: {all: Forward {duration: 0.2}} apply: { draw_bg: {down: snap(1.0), hover: 1.0} } }
            }
            active: {
                default: @off
                off: AnimatorState { from: {all: Forward {duration: 0.2}} apply: { draw_bg: {active: 0.0} draw_text: {active: 0.0} draw_icon: {color: (RBX_FG_SECONDARY)} } }
                on:  AnimatorState { from: {all: Forward {duration: 0.0}} apply: { draw_bg: {active: 1.0} draw_text: {active: 1.0} draw_icon: {color: (RBX_ACCENT)} } }
            }
            focus: {
                default: @off
                off: AnimatorState { from: {all: Forward {duration: 0.2}} apply: { draw_bg: {focus: 0.0} draw_text: {focus: 0.0} } }
                on:  AnimatorState { from: {all: Forward {duration: 0.0}} apply: { draw_bg: {focus: 1.0} draw_text: {focus: 1.0} } }
            }
        }
    }

    mod.widgets.ProfileIcon = #(ProfileIcon::register_widget(vm)) {
        width: Fill,
        height: (NAVIGATION_TAB_BAR_SIZE - 8)
        flow: Overlay
        align: Align{ x: 0.5, y: 0.5 }

        our_own_avatar := Avatar {
            width: mod.widgets.NAVIGATION_TAB_BAR_AVATAR_SIZE
            height: mod.widgets.NAVIGATION_TAB_BAR_AVATAR_SIZE
            // If no avatar picture, use white text on a dark background.
            text_view +: {
                draw_bg.color: (COLOR_FG_DISABLED),
                text +: {
                    draw_text +: {
                        text_style: theme.font_regular { font_size: mod.widgets.NAVIGATION_TAB_BAR_AVATAR_FONT_SIZE },
                        color: (COLOR_PRIMARY),
                    }
                }
            }
        }

        View {
            align: Align { x: 0.5, y: 0.0 }
            margin: Inset{ left: (mod.widgets.NAVIGATION_TAB_BAR_AVATAR_SIZE * 0.9) }
            verification_badge := VerificationBadge {}
        }
    }

    mod.widgets.HomeButton = mod.widgets.NavigationTabButton {
        draw_icon +: { svg: (ICON_HOME) }
    }

    mod.widgets.AddRoomButton = mod.widgets.NavigationTabButton {
        draw_icon +: { svg: (ICON_ADD) }
    }

    mod.widgets.Separator = LineH { margin: (SPACE_SM), draw_bg.color: (RBX_NAV_DIVIDER) }

    mod.widgets.NavigationTabBar = #(NavigationTabBar::register_widget(vm)) {
        Desktop := SolidView {
            flow: Down,
            align: Align{x: 0.5}
            padding: Inset{top: (SPACE_SM), bottom: (SPACE_SM), left: (SPACE_XS), right: (SPACE_XS)}
            width: (NAVIGATION_TAB_BAR_SIZE),
            height: Fill

            show_bg: true
            // Dark navy anchor rail (visual spec §2/§5.6). SolidView fills its column
            // edge-to-edge (no rounded-SDF anti-aliased border), so the navy is
            // perfectly flush to the window's left edge AND to the rooms list — no
            // stray hairline gap on either side.
            draw_bg.color: (RBX_NAV_BG)

            CachedWidget {
                profile_icon := mod.widgets.ProfileIcon {}
            }
            CachedWidget {
                home_button := mod.widgets.HomeButton {}
            }
            CachedWidget {
                add_room_button := mod.widgets.AddRoomButton {}
            }

            mod.widgets.Separator {}

            CachedWidget {
                root_spaces_bar := mod.widgets.SpacesBar {}
            }

        }

        // Mobile bottom tab bar (visual spec §4.14): a flat white bar, ~56px
        // tall, with a 1px top hairline and three equal-width tabs. Each tab is
        // a 24px icon above a small label; the selected tab is recolored teal.
        //
        // NOTE: these tabs are intentionally NOT wrapped in `CachedWidget` (unlike
        // the Desktop rail above), so the mobile bar can carry its own look. The
        // Desktop and Mobile variants never coexist, so reusing the `home_button`
        // / `add_room_button` ids across both is safe.
        // NOTE: SolidView (not a plain View) — a plain `View` draws nothing
        // because its base DrawQuad pixel shader is transparent; SolidView/
        // RoundedView/LineH are the widget types that actually fill with a color.
        Mobile := SolidView {
            flow: Down
            width: Fill,
            height: Fit

            show_bg: true
            // White bar surface — app-bar, room list and this tab bar are all
            // white; depth comes from the clear divider lines, not bg color.
            draw_bg.color: (RBX_BG_SURFACE)

            // Top divider — LineH (a RoundedView) actually renders, unlike a plain
            // View. Clearer line (strong stroke, 1.5px) for depth over the list.
            LineH {
                width: Fill, height: 1.5
                draw_bg.color: (RBX_STROKE_STRONG)
            }

            View {
                flow: Right
                width: Fill,
                height: (RBX_BOTTOM_TAB_H)
                align: Align{y: 0.5}

                // Labels are set in the DSL (English) so they always render; the
                // runtime i18n pass in handle_event localizes them when possible.
                home_button := mod.widgets.MobileTabButton {
                    text: "Home"
                    draw_icon +: { svg: (ICON_HOME) }
                }
                add_room_button := mod.widgets.MobileTabButton {
                    text: "Add Room"
                    draw_icon +: { svg: (ICON_ADD) }
                }
                settings_button := mod.widgets.MobileTabButton {
                    text: "Settings"
                    draw_icon +: { svg: (ICON_SETTINGS) }
                }
            }
            // No per-bar safe-area spacer: the window `body` already insets all
            // content by the platform safe area (SAFE_INSET_PAD_*), so the bar is
            // a consistent height on iOS and Android. (Supersedes the spacer inset
            // clamp from #217 — there is no spacer left to inflate.)
        }
    }
}

/// The icon in the NavigationTabBar that show the user's avatar.
///
/// Clicking on this icon will open the settings screen.
#[derive(Script, Widget)]
pub struct ProfileIcon {
    #[deref] view: View,
    #[rust] own_profile: Option<UserProfile>,
    #[rust] app_language: AppLanguage,
}

#[derive(Clone, Debug, Default)]
pub enum ProfileIconAction {
    Clicked,
    #[default]
    None,
}

impl ScriptHook for ProfileIcon {
    fn on_after_reload(&mut self, vm: &mut ScriptVm) {
        vm.with_cx_mut(|cx| {
            if self.own_profile.is_none() {
                self.own_profile = get_own_profile(cx);
            }
        });
    }
}

impl Widget for ProfileIcon {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        self.app_language = app_language;

        if self.own_profile.is_none() {
            self.own_profile = get_own_profile(cx);
        }

        // A UI Signal indicates that a user profile or avatar may have been updated.
        if let Event::Signal = event {
            let mut needs_redraw = false;
            // Refetch our profile if we don't have it yet.
            if self.own_profile.is_none() {
                user_profile_cache::process_user_profile_updates(cx);
                self.own_profile = get_own_profile(cx);
                needs_redraw = true;
            }
            // If we're waiting for an avatar image, process avatar updates.
            if let Some(p) = self.own_profile.as_mut() && p.avatar_state.uri().is_some() {
                avatar_cache::process_avatar_updates(cx);
                let new_data = p.avatar_state.update_from_cache(cx);
                needs_redraw |= new_data.is_some();
                if new_data.is_some() {
                    user_profile_cache::enqueue_user_profile_update(
                        UserProfileUpdate::UserProfileOnly(p.clone())
                    );
                }
            }
            if needs_redraw {
                self.view.redraw(cx);
            }
        }

        // Handle actions related to the currently-logged-in user account,
        // such as changing their avatar, display name, etc.
        if let Event::Actions(actions) = event {
            for action in actions {
                if let Some(LoginAction::LoginSuccess) = action.downcast_ref() {
                    self.own_profile = get_own_profile(cx);
                    self.view.redraw(cx);
                    continue;
                }

                if let Some(LogoutAction::ClearAppState { .. }) = action.downcast_ref() {
                    self.own_profile = None;
                    self.view.redraw(cx);
                    continue;
                }

                // Handle account switch - refresh profile with new account's data
                if let Some(AccountSwitchAction::Switched(_new_user_id)) = action.downcast_ref() {
                    self.own_profile = get_own_profile(cx);
                    self.view.redraw(cx);
                    continue;
                }

                // Handle account data changes (e.g., avatar updated/removed)
                match action.downcast_ref() {
                    Some(AccountDataAction::AvatarChanged(None)) => {
                        // Update both this widget's local profile info and the user profile cache.
                        if let Some(p) = self.own_profile.as_mut() {
                            p.avatar_state = AvatarState::Known(None);
                            user_profile_cache::enqueue_user_profile_update(
                                UserProfileUpdate::UserProfileOnly(p.clone())
                            );
                            self.view.redraw(cx);
                        }
                        continue;
                    }
                    Some(AccountDataAction::AvatarChanged(Some(new_uri))) => {
                        if let Some(p) = self.own_profile.as_mut() {
                            p.avatar_state = AvatarState::Known(Some(new_uri.clone()));
                            p.avatar_state.update_from_cache(cx);
                            user_profile_cache::enqueue_user_profile_update(
                                UserProfileUpdate::UserProfileOnly(p.clone())
                            );
                            self.view.redraw(cx);
                        }
                        continue;
                    }
                    Some(AccountDataAction::AvatarChangeFailed(_)) => {
                        // this is only handled in the account settings screen
                        continue;
                    }
                    Some(AccountDataAction::DisplayNameChanged(new_display_name)) => {
                        if let Some(p) = self.own_profile.as_mut() {
                            p.username = new_display_name.clone();
                            user_profile_cache::enqueue_user_profile_update(
                                UserProfileUpdate::UserProfileOnly(p.clone())
                            );
                            self.view.redraw(cx);
                        }
                        continue;
                    }
                    Some(AccountDataAction::DisplayNameChangeFailed(_)) => {
                        // this is only handled in the account settings screen
                        continue;
                    }
                    _ => {}
                }
            }
        }

        let area = self.view.area();
        match event.hits(cx, area) {
            Hit::FingerUp(fe) if fe.is_over && fe.is_primary_hit() && fe.was_tap() => {
                cx.widget_action(self.widget_uid(), ProfileIconAction::Clicked);
            }
            Hit::FingerLongPress(_) | Hit::FingerHoverIn(_) => {
                let (verification_str, bg_color) = self.view
                    .verification_badge(cx, ids!(verification_badge))
                    .tooltip_content(self.app_language);
                let text = self.own_profile.as_ref().map_or_else(
                    || tr_fmt(self.app_language, "navigation_tab_bar.profile.tooltip.not_logged_in", &[
                        ("verification", verification_str.as_str()),
                    ]),
                    |p| tr_fmt(self.app_language, "navigation_tab_bar.profile.tooltip.logged_in_as", &[
                        ("display_name", p.displayable_name()),
                        ("verification", verification_str.as_str()),
                    ]),
                );
                let mut options = CalloutTooltipOptions {
                    position: if effective_is_desktop(cx) { TooltipPosition::Right} else { TooltipPosition::Top},
                    ..Default::default()
                };
                if let Some(c) = bg_color {
                    options.bg_color = c;
                }
                cx.widget_action(
                    self.widget_uid(), 
                    TooltipAction::HoverIn {
                        text,
                        widget_rect: area.rect(cx),
                        options,
                    },
                );
            }
            Hit::FingerHoverOut(_) => {
                cx.widget_action(self.widget_uid(),  TooltipAction::HoverOut);
            }
            _ => { }
        };

        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let our_own_avatar = self.view.avatar(cx, ids!(our_own_avatar));
        let Some(own_profile) = self.own_profile.as_ref() else {
            // If we don't have a profile, default to an unknown avatar.
            our_own_avatar.show_text(
                cx,
                Some(COLOR_FG_DISABLED),
                None, // don't make this avatar clickable; we handle clicks on this ProfileIcon widget directly.
                "",
            );
            return self.view.draw_walk(cx, scope, walk);
        };

        let mut drew_avatar = false;
        if let Some(avatar_img_data) = own_profile.avatar_state.data() {
            drew_avatar = our_own_avatar.show_image(
                cx,
                None, // don't make this avatar clickable; we handle clicks on this ProfileIcon widget directly.
                |cx, img| utils::load_png_or_jpg(&img, cx, avatar_img_data),
            ).is_ok();
        }
        if !drew_avatar {
            our_own_avatar.show_text(
                cx,
                Some(COLOR_ROBRIX_PURPLE),
                None, // don't make this avatar clickable; we handle clicks on this ProfileIcon widget directly.
                own_profile.displayable_name(),
            );
        }

        self.view.draw_walk(cx, scope, walk)
    }
}


/// The tab bar with buttons that navigate through top-level app pages.
///
/// * In the "desktop" (wide) layout, this is a vertical bar on the left.
/// * In the "mobile" (narrow) layout, this is a horizontal bar on the bottom.
#[derive(Script, Widget)]
pub struct NavigationTabBar {
    #[deref] view: AdaptiveView,

    #[rust] applied_view_mode: ViewModeOverride,
    /// The selected tab currently reflected in the bar's highlight. Used to
    /// re-project `AppState::selected_tab` onto the tab buttons only when it
    /// actually changes (covers programmatic navigation + view-mode switches).
    #[rust] applied_selected_tab: Option<SelectedTab>,
    /// The language the tab labels are currently rendered in.
    #[rust] applied_language: Option<AppLanguage>,
}

impl ScriptHook for NavigationTabBar {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        vm.with_cx_mut(|cx| {
            // Programmatically select the Home button as active on startup,
            // because animator default overrides in the DSL don't take effect.
            if let Some(mut rb) = self.view.radio_button(cx, ids!(home_button)).borrow_mut() {
                rb.animator_play(cx, ids!(active.on));
            }
            cx.set_global(self.view.spaces_bar(cx, ids!(root_spaces_bar)));
            let mode = cx.global::<AppPreferencesGlobal>().0.view_mode;
            self.apply_view_mode(mode);
        });
    }
}

impl NavigationTabBar {
    fn apply_view_mode(&mut self, mode: ViewModeOverride) {
        self.view.set_variant_selector(mode.variant_selector());
        self.applied_view_mode = mode;
        // Switching variants rebuilds the (non-cached) mobile tab buttons, so
        // force the labels and active highlight to re-apply.
        self.applied_selected_tab = None;
        self.applied_language = None;
    }

    /// Localize the mobile tab labels. (Harmless on Desktop, where the rail
    /// buttons hide their labels and the gear `settings_button` does not exist.)
    fn update_tab_labels(&mut self, cx: &mut Cx, language: AppLanguage) {
        self.view.radio_button(cx, ids!(home_button))
            .set_text(tr_key(language, "navigation_tab_bar.tab.home"));
        self.view.radio_button(cx, ids!(add_room_button))
            .set_text(tr_key(language, "navigation_tab_bar.tab.add_room"));
        self.view.radio_button(cx, ids!(settings_button))
            .set_text(tr_key(language, "navigation_tab_bar.tab.settings"));
        self.view.redraw(cx);
    }

    /// Project `AppState::selected_tab` onto the tab buttons' active highlight.
    /// This is the single source of truth for which tab looks selected.
    fn sync_selected_tab(&mut self, cx: &mut Cx, scope: &mut Scope) {
        let tab = {
            let Some(app_state) = scope.data.get::<AppState>() else { return };
            app_state.selected_tab.clone()
        };
        // Bail until the tab buttons exist — the Mobile variant is built lazily on
        // first draw; we retry on the next event.
        if self.view.radio_button(cx, ids!(home_button)).borrow().is_none() {
            return;
        }

        if self.applied_selected_tab.as_ref() == Some(&tab) {
            // selected_tab is unchanged, so normally there's nothing to do. BUT a
            // variant switch / script reapply rebuilds the (non-cached) tab buttons
            // and resets their active state — leaving the startup Home tab grey. So
            // re-apply when the tab that SHOULD be active isn't currently active.
            // Skip while a tap is in-flight (a tab is already active but
            // selected_tab hasn't caught up yet) so we never fight the tap.
            let any_active = self.view.radio_button(cx, ids!(home_button)).active(cx)
                || self.view.radio_button(cx, ids!(add_room_button)).active(cx)
                || self.view.radio_button(cx, ids!(settings_button)).active(cx);
            let wants_tab = matches!(
                tab,
                SelectedTab::Home | SelectedTab::AddRoom | SelectedTab::Settings
            );
            if any_active || !wants_tab {
                return;
            }
            // A tab should be active but none is → the buttons were rebuilt; fall
            // through and re-apply the highlight.
        }
        self.applied_selected_tab = Some(tab.clone());

        // Both bars drive icon + label color from the radio animator's `active`
        // state, so we only need to set which tab is active.
        self.apply_tab_active(cx, ids!(home_button), matches!(tab, SelectedTab::Home));
        self.apply_tab_active(cx, ids!(add_room_button), matches!(tab, SelectedTab::AddRoom));
        self.apply_tab_active(cx, ids!(settings_button), matches!(tab, SelectedTab::Settings));
    }

    fn apply_tab_active(&mut self, cx: &mut Cx, path: &[LiveId], active: bool) {
        let radio_button = self.view.radio_button(cx, path);
        // `set_active` plays the radio animator's `active` state, which already
        // drives both draw_text and draw_icon colors (see the MobileTabButton
        // animator). No imperative draw_icon recolor is needed — and
        // `script_apply_eval!` fails on these tab buttons anyway (pitfall #40).
        radio_button.set_active(cx, active, Animate::No);
    }
}

impl Widget for NavigationTabBar {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Keep the mobile tab labels localized. Mobile-only: the Desktop rail
        // buttons must stay icon-only (their hidden label doesn't clip text, so
        // setting it would make labels appear under the desktop icons). Only
        // commit once the tab buttons exist — the Mobile variant is built lazily
        // on first draw, so an earlier attempt would no-op and never retry. The
        // DSL sets English labels meanwhile, so the bar is never blank.
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if !effective_is_desktop(cx)
            && self.applied_language != Some(app_language)
            && self.view.radio_button(cx, ids!(home_button)).borrow().is_some()
        {
            self.applied_language = Some(app_language);
            self.update_tab_labels(cx, app_language);
        }

        if let Event::Actions(actions) = event {
            // Handle a tab being clicked (selected).
            // Note: `settings_button` only exists in the Mobile variant; on
            // Desktop the avatar `profile_icon` opens Settings instead (below).
            let radio_button_set = self.view.radio_button_set(cx, ids_array!(
                home_button,
                add_room_button,
                settings_button,
            ));
            match radio_button_set.selected(cx, actions) {
                Some(0) => cx.action(NavigationBarAction::GoToHome),
                Some(1) => cx.action(NavigationBarAction::GoToAddRoom),
                Some(2) => cx.action(NavigationBarAction::OpenSettings),
                _ => { }
            }

            for action in actions {
                // On Desktop, clicking the profile avatar opens Settings.
                if let ProfileIconAction::Clicked = action.as_widget_action().cast() {
                    cx.action(NavigationBarAction::OpenSettings);
                    continue;
                }

                if let Some(AppPreferencesAction::ViewModeChanged(new_mode)) = action.downcast_ref() {
                    if *new_mode != self.applied_view_mode {
                        self.apply_view_mode(*new_mode);
                        self.view.redraw(cx);
                    }
                    continue;
                }
            }
        }

        // Project the active highlight from the global selected-tab state. This
        // covers tab clicks (once HomeScreen updates the state), programmatic
        // navigation, and view-mode switches.
        self.sync_selected_tab(cx, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        // The bottom safe-area inset is handled globally by the window `body`
        // padding (SAFE_INSET_PAD_*), so the bar needs no per-widget spacer.
        self.view.draw_walk(cx, scope, walk)
    }
}


/// Which top-level view is currently shown, and which navigation tab is selected.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SelectedTab {
    #[default]
    Home,
    AddRoom,
    Settings,
    VoIP,
    /// The public room directory browser screen.
    /// Entered from the sidebar header's compass button; no dedicated tab in the
    /// navigation bar (so no button to keep selected here).
    Directory,
    // AlertsInbox,
    Space { space_name_id: RoomNameId },
}


/// Actions for navigating through the top-level views of the app,
/// e.g., when the user clicks/taps on a button in the NavigationTabBar.
///
/// ## Tip: you only want to handle `TabSelected`
/// The most important variant is `TabSelected`, which is most likely the action
/// that you want to handle in other widgets, if you care about which
/// top-level navigation tab is currently selected.
/// This is because the `TabSelected` variant will always occur even if the
/// other actions do not occur --- for example, if the user chooses to jump
/// to a different view (or back to a previous view) without explicitly clicking
/// a navigation tab button, e.g., via a keyboard shortcut, or programmatically.
///
/// Only one widget, the `HomeScreen`, should emit the `TabSelected` action.
/// All other widgets should handle only that action in order to ensure
/// consistent behavior.
///
/// ## More details
/// There are 3 kinds of actions within this one enum:
/// 1. "Leading-edge" ("request") actions emitted by the NavigationTabBar
///    when the user selects a particular button/space.
///    * Includes `GoToHome`, `GoToAddRoom`, `GoToSpace`, `OpenSettings`, `CloseSettings`.
/// 2. "Trailing-edge" ("response") actions that are emitted by the `HomeScreen` widget
///    in response to a leading-edge action.
///    * This includes only the `TabSelected` variant.
///    * This is what all other widgets should handle if they want/need to respond
///      to changes in the top-level app-wide navigation selection.
/// 3. Other actions that aren't requests/responses to navigate to a different view.
///    * This only includes the `ToggleSpacesBar` variant.
#[derive(Debug, PartialEq, Eq)]
pub enum NavigationBarAction {
    /// Go to the main rooms content view.
    GoToHome,
    /// Go the add/join/explore room view.
    GoToAddRoom,
    /// Go to the public room directory browser view (`DirectoryScreen`).
    GoToDirectory,
    /// Go to the Settings view (open the `SettingsScreen`).
    OpenSettings,
    /// Close the Settings view (`SettingsScreen`), returning to the previous view.
    CloseSettings,
    /// Go the space screen for the given space.
    GoToSpace { space_name_id: RoomNameId },
    // /// Go to the VoIP call screen.
    // GoToVoip,

    // TODO: add GoToAlertsInbox, once we add that button/screen

    /// The given tab was selected as the active top-level view.
    /// This is needed to ensure that the proper tab is marked as selected.
    TabSelected(SelectedTab),
    /// Toggle whether the SpacesBar is shown, i.e., show/hide it.
    /// This is only applicable in the Mobile view mode, because the SpacesBar
    /// is always shown in Desktop view mode.
    ToggleSpacesBar,
}


/// Returns the current user's profile and avatar, if available.
pub fn get_own_profile(cx: &mut Cx) -> Option<UserProfile> {
    let mut own_profile = None;
    if let Some(own_user_id) = current_user_id() {
        let avatar_uri_to_fetch = user_profile_cache::with_user_profile(
            cx,
            own_user_id,
            None,
            true,
            |new_profile, _rooms| {
                let avatar_uri_to_fetch = new_profile.avatar_state.uri().cloned();
                own_profile = Some(new_profile.clone());
                avatar_uri_to_fetch
            },
        );
        // If we have an avatar URI to fetch, try to fetch it.
        if let Some(Some(avatar_uri)) = avatar_uri_to_fetch {
            if let AvatarCacheEntry::Loaded(data) = avatar_cache::get_or_fetch_avatar(cx, &avatar_uri) {
                if let Some(p) = own_profile.as_mut() {
                    p.avatar_state = AvatarState::Loaded(data);
                    // Update the user profile cache with the new avatar data.
                    user_profile_cache::enqueue_user_profile_update(
                        UserProfileUpdate::UserProfileOnly(p.clone())
                    );
                }
            }
        }
    }

    own_profile
}
