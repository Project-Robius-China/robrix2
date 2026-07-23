//! A small popup menu, anchored at the account-switcher button in the bottom-left of the
//! desktop navigation rail, that lets you quickly switch between logged-in accounts and
//! add another account — the "Feishu-style" account switcher.
//!
//! Structure of the anchored card (top → bottom; it opens *upward* from the bottom of the
//! rail):
//!   * Active account header  → avatar + display name + user ID + an "Active" marker
//!   * One row per *other* logged-in account → click to switch to it
//!   * A divider
//!   * "Log Into More Accounts" → opens the login screen in add-account mode
//!   * "Account Settings"       → opens Settings
//!   * "Log Out"                → opens the LogoutConfirmModal
//!
//! It reuses the same anchored-overlay pattern as [`crate::home::add_menu::AddMenu`]:
//! a full-screen scrim whose inner `main_content` card is positioned by the App
//! (which clamps it to the overlay container and sets its `margin` via
//! `script_apply_eval!`). Unlike AddMenu, the number of account rows is dynamic, so a
//! fixed pool of [`MAX_SWITCH_ROWS`] row buttons is shown/hidden at `show()` time, and
//! `show()` returns the *computed* card height so the App can anchor it upward.
//!
//! Desktop-only: opened from the rail's account-switcher button, and auto-closes if the
//! layout crosses the desktop/mobile breakpoint while open. Mobile keeps its existing
//! "switch account in Settings" flow.

use makepad_widgets::*;
use matrix_sdk::ruma::OwnedUserId;

use crate::{
    account_manager,
    app::AppState,
    home::navigation_tab_bar::{get_own_profile, NavigationBarAction},
    i18n::{tr_key, AppLanguage},
    login::login_screen::LoginAction,
    logout::logout_confirm_modal::LogoutConfirmModalAction,
    settings::app_preferences::effective_is_desktop,
    shared::avatar::AvatarWidgetExt,
    sliding_sync::{current_user_id, request_switch_account},
    utils,
};

/// The fixed width of the account menu card, in DIPs.
pub const ACCOUNT_MENU_WIDTH: f64 = 272.0;

/// The maximum number of *other* accounts shown as switchable rows. Realistically
/// nobody logs into more accounts than this at once; if they do, the extras are hidden
/// (and logged). Settings → Account applies the same cap, so the only case affected is
/// 10+ simultaneous logins — to reach a hidden account, log out one of the visible ones.
pub const MAX_SWITCH_ROWS: usize = 8;

// --- Height constants, kept in sync with the DSL below so `show()` can compute the
// card's total height for upward anchoring + clamping (main_content is height:Fit, so
// there is no measurable height until it has been drawn). ---
const CARD_VPAD: f64 = 6.0; // main_content padding (top == bottom == this)
const ROW_SPACING: f64 = 2.0; // main_content `spacing` between children
const HEADER_H: f64 = 58.0; // active-account header (avatar 40 + 9+9 padding)
const SWITCH_ROW_H: f64 = 40.0; // one switchable account row
const DIVIDER_H: f64 = 10.0; // LineH (shared height 2) + its 4 + 4 vertical margins
const ACTION_H: f64 = 40.0; // one action item (add / settings / logout)
const ACTION_COUNT: f64 = 3.0;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*


    // A single action row (add account / settings): left-aligned icon + label, with an
    // RBX surface/hover/pressed background and a soft rounded highlight. Mirrors
    // `AddMenuItem`.
    mod.widgets.AccountMenuItem = RobrixIconButton {
        height: 40,
        width: Fill,
        margin: 0,
        padding: Inset{left: 12, right: 12, top: 8, bottom: 8}
        spacing: 12,
        align: Align{x: 0.0, y: 0.5}
        icon_walk: Walk{width: 18, height: 18, margin: Inset{right: 2}}

        draw_bg +: {
            color: (RBX_BG_SURFACE)
            color_hover: (RBX_BG_HOVER)
            color_down: (RBX_BG_PRESSED)
            border_size: 0.0
            border_radius: (RBX_RADIUS_SM)
        }
        draw_icon.color: (RBX_ACCENT)
        draw_text +: {
            color: (RBX_FG_PRIMARY)
            color_hover: (RBX_FG_PRIMARY)
            color_down: (RBX_FG_PRIMARY)
            text_style: RBX_TEXT_BODY {}
        }
    }

    // Danger-styled action row (log out): red icon/text, red-tinted hover.
    mod.widgets.AccountMenuDangerItem = mod.widgets.AccountMenuItem {
        draw_bg +: {
            color: (RBX_BG_SURFACE)
            color_hover: (RBX_DANGER_BG)
            color_down: (RBX_DANGER_BG)
        }
        draw_icon.color: (RBX_DANGER_FG)
        draw_text +: {
            color: (RBX_DANGER_FG)
            color_hover: (RBX_DANGER_FG)
            color_down: (RBX_DANGER_FG)
        }
    }

    // A switchable "other account" row: a generic user icon + the account's user ID. A
    // button (not a custom avatar row) so it integrates with the overlay's standard
    // button-click event flow, exactly like the AddMenu items.
    mod.widgets.AccountSwitchItem = RobrixIconButton {
        height: 40,
        width: Fill,
        margin: 0,
        visible: false,
        padding: Inset{left: 10, right: 12, top: 6, bottom: 6}
        spacing: 10,
        align: Align{x: 0.0, y: 0.5}
        icon_walk: Walk{width: 22, height: 22, margin: Inset{right: 2}}

        draw_bg +: {
            color: (RBX_BG_SURFACE)
            color_hover: (RBX_BG_HOVER)
            color_down: (RBX_BG_PRESSED)
            border_size: 0.0
            border_radius: (RBX_RADIUS_SM)
        }
        // ICON_PEOPLE, not ICON_ADD_USER: an "add user" glyph on a *switch* row reads as
        // "add this account" right above the real "Log Into More Accounts" item.
        draw_icon +: { svg: (ICON_PEOPLE), color: (RBX_FG_SECONDARY) }
        draw_text +: {
            color: (RBX_FG_PRIMARY)
            color_hover: (RBX_FG_PRIMARY)
            color_down: (RBX_FG_PRIMARY)
            text_style: RBX_TEXT_BODY {}
        }
        text: "@other:server"
    }

    mod.widgets.AccountMenu = set_type_default() do #(AccountMenu::register_widget(vm)) {
        ..mod.widgets.SolidView

        visible: false,
        flow: Overlay,
        width: Fill,
        height: Fill,
        cursor: MouseCursor.Default,
        align: Align{x: 0, y: 0}

        show_bg: true
        draw_bg +: {
            color: (RBX_SCRIM)
        }

        main_content := RoundedView {
            flow: Down
            width: 272,
            height: Fit,
            padding: 6,
            spacing: 2,

            show_bg: true
            // Flat card: tight corners, a defined border, no drop shadow (the scrim
            // already separates it from the content behind).
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_SM)
                border_size: 1.0
                border_color: (RBX_STROKE_STRONG)
            }

            // --- Active account header ---
            // height: Fit so the two-line name + user id is never clipped by the card.
            active_account_header := RoundedView {
                width: Fill,
                height: Fit,
                flow: Right,
                align: Align{y: 0.5}
                padding: Inset{left: 8, right: 8, top: 9, bottom: 9}
                spacing: 10,
                show_bg: true
                draw_bg +: {
                    color: (RBX_ACCENT_SOFT)
                    border_radius: (RBX_RADIUS_SM)
                }

                active_avatar := Avatar {
                    width: 40, height: 40
                }

                View {
                    width: Fill, height: Fit,
                    flow: Down,
                    spacing: 0,

                    active_name := Label {
                        width: Fill, height: Fit,
                        margin: 0, padding: 0,
                        text_overflow: Ellipsis
                        draw_text +: {
                            color: (RBX_FG_PRIMARY)
                            text_style: RBX_TEXT_BODY_STRONG {}
                        }
                        text: "Display Name"
                    }
                    active_user_id := Label {
                        width: Fill, height: Fit,
                        margin: 0, padding: 0,
                        text_overflow: Ellipsis
                        draw_text +: {
                            color: (RBX_FG_SECONDARY)
                            text_style: RBX_TEXT_META {}
                        }
                        text: "@user:server"
                    }
                }

                // "Current account" marker on the right.
                //
                // Deliberately plain bold ACCENT TEXT rather than a filled pill: the
                // pill's rounded-box fill did not render here, which left white-on-pale
                // text that was almost unreadable. Text always renders, and teal-on-pale
                // has ample contrast, so this is unambiguous without depending on a
                // background being drawn.
                active_badge_label := Label {
                    width: Fit, height: Fit,
                    margin: 0, padding: 0,
                    draw_text +: {
                        color: (RBX_ACCENT)
                        text_style: theme.font_bold { font_size: 10.5 }
                    }
                    text: "Active"
                }
            }

            // --- Other accounts (fixed pool, populated dynamically) ---
            account_switch_item_0 := mod.widgets.AccountSwitchItem {}
            account_switch_item_1 := mod.widgets.AccountSwitchItem {}
            account_switch_item_2 := mod.widgets.AccountSwitchItem {}
            account_switch_item_3 := mod.widgets.AccountSwitchItem {}
            account_switch_item_4 := mod.widgets.AccountSwitchItem {}
            account_switch_item_5 := mod.widgets.AccountSwitchItem {}
            account_switch_item_6 := mod.widgets.AccountSwitchItem {}
            account_switch_item_7 := mod.widgets.AccountSwitchItem {}

            divider := LineH {
                margin: Inset{top: 4, bottom: 4, left: 8, right: 8}
                draw_bg.color: (RBX_DIVIDER)
            }

            // --- Actions ---
            add_account_item := mod.widgets.AccountMenuItem {
                draw_icon +: { svg: (ICON_ADD) }
                text: "Log Into More Accounts"
            }
            settings_item := mod.widgets.AccountMenuItem {
                draw_icon +: { svg: (ICON_SETTINGS) }
                text: "Account Settings"
            }
            logout_item := mod.widgets.AccountMenuDangerItem {
                draw_icon +: { svg: (ICON_LOGOUT) }
                text: "Log Out"
            }
        }
    }
}


/// Action to request showing the account menu, anchored so the card's *bottom-left*
/// corner sits at `pos` (already computed by the emitter — the bottom-right of the
/// account-switcher button, so the menu opens upward). The App clamps this into the
/// overlay container using the height returned by [`AccountMenu::show`].
#[derive(Clone, Debug, Default)]
pub enum AccountMenuAction {
    Open { pos: DVec2 },
    #[default]
    None,
}


#[derive(Script, ScriptHook, Widget)]
pub struct AccountMenu {
    #[deref] view: View,
    #[source] source: ScriptObjectRef,
    #[rust] app_language: AppLanguage,
    /// The effective layout (desktop vs mobile) at the moment the menu was opened. If
    /// it changes while the menu is open, the menu auto-closes — its anchored position
    /// is only valid for the layout it was opened in.
    #[rust(true)] opened_is_desktop: bool,
    /// The user IDs of the *other* accounts, index-aligned with the visible switch rows,
    /// so a row click maps back to the account to switch to.
    #[rust] switch_targets: Vec<OwnedUserId>,
}

impl Widget for AccountMenu {
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let step = self.view.draw_walk(cx, scope, walk);
        if self.visible {
            let main_content_area = self.view(cx, ids!(main_content)).area();
            cx.block_scrolling_except_within(main_content_area);
        }
        step
    }

    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if !self.visible {
            return;
        }
        // Close if the layout switched between desktop and mobile while open, so the
        // menu can't linger at a now-wrong anchor in the other layout.
        if effective_is_desktop(cx) != self.opened_is_desktop {
            self.close(cx);
            return;
        }
        if let Some(app_state) = scope.data.get::<AppState>()
            && self.app_language != app_state.app_language
        {
            self.set_app_language(cx, app_state.app_language);
        }
        self.view.handle_event(cx, event, scope);

        // Close on backdrop click, Escape, or a system back gesture. Opened from a
        // button *click* (FingerUp) which is fully consumed by the time we become
        // visible, so no stray FingerUp lands on the scrim.
        let area = self.view.area();
        let close_menu = event.back_pressed()
            || match event.hits_with_capture_overload(cx, area, true) {
                Hit::KeyUp(key) => key.key_code == KeyCode::Escape,
                Hit::FingerUp(fue) if fue.is_over => {
                    !self.view(cx, ids!(main_content)).area().rect(cx).contains(fue.abs)
                }
                _ => false,
            };
        if close_menu {
            self.close(cx);
            return;
        }

        self.widget_match_event(cx, event, scope);
    }
}

impl WidgetMatchEvent for AccountMenu {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        // A switchable account row was clicked → switch to that account.
        let switch_ids = [
            ids!(account_switch_item_0),
            ids!(account_switch_item_1),
            ids!(account_switch_item_2),
            ids!(account_switch_item_3),
            ids!(account_switch_item_4),
            ids!(account_switch_item_5),
            ids!(account_switch_item_6),
            ids!(account_switch_item_7),
        ];
        for (i, id) in switch_ids.into_iter().enumerate() {
            if self.button(cx, id).clicked(actions) {
                if let Some(user_id) = self.switch_targets.get(i).cloned() {
                    log!("AccountMenu: switching to account {user_id}");
                    request_switch_account(user_id);
                }
                self.close(cx);
                return;
            }
        }

        if self.button(cx, ids!(add_account_item)).clicked(actions) {
            cx.action(LoginAction::ShowAddAccountScreen);
            self.close(cx);
        } else if self.button(cx, ids!(settings_item)).clicked(actions) {
            cx.action(NavigationBarAction::OpenSettings);
            self.close(cx);
        } else if self.button(cx, ids!(logout_item)).clicked(actions) {
            cx.action(LogoutConfirmModalAction::Open);
            self.close(cx);
        }
    }
}

impl AccountMenu {
    fn set_app_language(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        self.app_language = app_language;
        self.label(cx, ids!(active_badge_label))
            .set_text(cx, tr_key(self.app_language, "account_menu.badge.active"));
        self.button(cx, ids!(add_account_item))
            .set_text(cx, tr_key(self.app_language, "account_menu.item.add_account"));
        self.button(cx, ids!(settings_item))
            .set_text(cx, tr_key(self.app_language, "account_menu.item.settings"));
        self.button(cx, ids!(logout_item))
            .set_text(cx, tr_key(self.app_language, "account_menu.item.logout"));
    }

    /// Populates the menu from the current account state and shows it, returning its
    /// expected `(width, height)` so the App can anchor + clamp it. `height` is computed
    /// from the number of visible rows (the card is height:Fit).
    fn show(&mut self, cx: &mut Cx, app_language: AppLanguage) -> DVec2 {
        self.opened_is_desktop = effective_is_desktop(cx);
        self.set_app_language(cx, app_language);

        // Determine the active account and the list of other accounts. Sorted, because the
        // AccountManager stores accounts in a HashMap — without this the rows would appear
        // in a different order on every launch.
        let active_id = account_manager::get_active_user_id().or_else(current_user_id);
        self.switch_targets = account_manager::get_all_user_ids()
            .into_iter()
            .filter(|id| Some(id) != active_id.as_ref())
            .collect();
        self.switch_targets.sort();

        // Populate the active-account header from our own profile (real avatar + display
        // name when available), mirroring how ProfileIcon draws its avatar.
        let own_profile = get_own_profile(cx);
        let active_avatar = self.view.avatar(cx, ids!(active_avatar));
        let (name_text, user_id_text) = if let Some(profile) = own_profile.as_ref() {
            let mut drew_image = false;
            if let Some(avatar_img_data) = profile.avatar_state.data() {
                drew_image = active_avatar
                    .show_image(cx, None, |cx, img| utils::load_png_or_jpg(&img, cx, avatar_img_data))
                    .is_ok();
            }
            if !drew_image {
                active_avatar.show_text(cx, None, None, profile.displayable_name());
            }
            (profile.displayable_name().to_string(), profile.user_id.to_string())
        } else if let Some(active) = active_id.as_ref() {
            active_avatar.show_text(cx, None, None, active.as_str());
            (active.to_string(), active.to_string())
        } else {
            active_avatar.show_text(cx, None, None, "");
            (
                tr_key(self.app_language, "settings.account.user_id.not_logged_in").to_string(),
                String::new(),
            )
        };
        self.label(cx, ids!(active_name)).set_text(cx, &name_text);
        self.label(cx, ids!(active_user_id)).set_text(cx, &user_id_text);
        // Hide the user-id line when it would just duplicate the name (no display name).
        self.label(cx, ids!(active_user_id))
            .set_visible(cx, !user_id_text.is_empty() && user_id_text != name_text);

        // Populate / toggle the fixed pool of other-account rows.
        let switch_ids = [
            ids!(account_switch_item_0),
            ids!(account_switch_item_1),
            ids!(account_switch_item_2),
            ids!(account_switch_item_3),
            ids!(account_switch_item_4),
            ids!(account_switch_item_5),
            ids!(account_switch_item_6),
            ids!(account_switch_item_7),
        ];
        let shown = self.switch_targets.len().min(MAX_SWITCH_ROWS);
        if self.switch_targets.len() > MAX_SWITCH_ROWS {
            log!(
                "AccountMenu: {} other accounts exceed the {} switchable rows; the extras \
                 are hidden (Settings applies the same cap — log out a visible account to \
                 reach them).",
                self.switch_targets.len(),
                MAX_SWITCH_ROWS,
            );
        }
        for (i, id) in switch_ids.into_iter().enumerate() {
            if i < shown {
                // Extract the text first: `self.switch_targets[i]` borrows self, which
                // would conflict with the `&mut self` from `self.button(...)`.
                let text = self.switch_targets[i].to_string();
                self.button(cx, id).set_text(cx, &text);
                self.button(cx, id).set_visible(cx, true);
                self.button(cx, id).reset_hover(cx);
            } else {
                self.button(cx, id).set_visible(cx, false);
            }
        }
        for id in [ids!(add_account_item), ids!(settings_item), ids!(logout_item)] {
            self.button(cx, id).reset_hover(cx);
        }

        self.visible = true;
        cx.set_key_focus(self.view.area());
        self.redraw(cx);

        let visible_rows = shown as f64;
        let height = 2.0 * CARD_VPAD
            + HEADER_H
            + visible_rows * SWITCH_ROW_H
            + DIVIDER_H
            + ACTION_COUNT * ACTION_H
            + (visible_rows + 4.0) * ROW_SPACING;
        dvec2(ACCOUNT_MENU_WIDTH, height)
    }

    fn close(&mut self, cx: &mut Cx) {
        self.visible = false;
        cx.revert_key_focus();
        cx.unblock_scrolling();
        self.redraw(cx);
    }
}

impl AccountMenuRef {
    pub fn show(&self, cx: &mut Cx, app_language: AppLanguage) -> DVec2 {
        let Some(mut inner) = self.borrow_mut() else { return DVec2::default() };
        inner.show(cx, app_language)
    }

    pub fn is_currently_shown(&self) -> bool {
        self.borrow().is_some_and(|inner| inner.visible)
    }
}
