//! Devices settings page: lists every device this user has signed in with,
//! lets them remove individual sessions.
//!
//! Visually this follows the shared "AI workspace" design system (see
//! `docs/ui-visual-spec-zh.md` + `src/shared/design_tokens.rs`): a page title +
//! session count + Refresh action, then a vertical list of white `RBX_*` cards.
//! Each card has a device glyph in a soft-accent tile, the display_name
//! (fallback "Unknown device"), the raw device_id, an optional "This device"
//! badge for the current session, and a "Last active" / "IP address" detail row,
//! plus a destructive "Remove" button.
//!
//! **Lazy rendering**: the list is a `PortalList`, so only the cards currently
//! on screen are laid out and drawn (`set_item_range` + `next_visible_item`).
//! The homeserver's `/devices` endpoint returns the full list in one response —
//! the Matrix spec / ruma `get_devices` request has no `from`/`limit`/`next_batch`
//! params, so there is no server-side pagination to lean on; virtualization at
//! the UI layer is where the win is.
//!
//! Click → fires `ConfirmDeleteAction::Show(…)` which opens the global
//! delete-confirmation modal (defined in `app.rs`). On confirmation the
//! `on_accept_clicked` callback submits `MatrixRequest::DeleteDevice`. The
//! UIA-fallback popup is handled at app level in response to
//! `AccountDataAction::DeviceDeleteResult { outcome: NeedsAuth }`.

use std::borrow::Cow;
use std::cell::RefCell;

use chrono::{DateTime, Local};
use makepad_widgets::*;

use crate::app::{ConfirmDeleteAction, PositiveConfirmationModalAction};
use crate::shared::confirmation_modal::ConfirmationModalContent;
use crate::shared::popup_list::{PopupKind, enqueue_notification, NotificationItem, NotificationAction, NotifActionStyle};
use crate::sliding_sync::{
    AccountDataAction, DeviceInfo, MatrixRequest, submit_async_request,
};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // ─────────────────────────── DeviceCard ────────────────────────────
    // One card in the device list. The DeviceCard root is a transparent layout
    // wrapper (a plain custom-widget root does NOT reliably paint its own
    // draw_bg in this Makepad fork — see the "plain View draws nothing" pitfall),
    // so the visible white surface + border lives on the inner `device_card_body`
    // RoundedView. The wrapper's bottom margin gives the gap between cards.
    mod.widgets.DeviceCard = #(DeviceCard::register_widget(vm)) {
        width: Fill, height: Fit
        flow: Down
        margin: Inset{top: 0, bottom: (SPACE_MD)}

        device_card_body := RoundedView {
            width: Fill, height: Fit
            flow: Down
            padding: Inset{top: (SPACE_MD), bottom: (SPACE_MD), left: (SPACE_MD), right: (SPACE_MD)}
            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_SM)
                border_size: 1.0
                border_color: (RBX_STROKE_STRONG)
            }
            spacing: (SPACE_SM)

            // Top row: icon tile + name/id column + remove button.
            device_card_top_row := View {
                width: Fill, height: Fit
                flow: Right
                spacing: (SPACE_SM)
                align: Align{y: 0.5}

                // Generic device glyph in a soft-accent tile. Matrix's `/devices`
                // endpoint doesn't expose a device type, so we don't try to guess
                // laptop vs phone vs browser — one icon for all.
                device_card_icon_circle := SettingsIconCircle {
                    width: 40, height: 40
                    draw_bg +: { color: (RBX_ACCENT_SOFT) }
                    Icon {
                        width: (RBX_ICON_MD), height: (RBX_ICON_MD)
                        draw_icon +: { svg: (ICON_DEVICE), color: (RBX_ACCENT) }
                        icon_walk: Walk{width: (RBX_ICON_MD), height: (RBX_ICON_MD)}
                    }
                }

                device_card_name_col := View {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 2

                    device_card_name_row := View {
                        width: Fill, height: Fit
                        flow: Right
                        align: Align{y: 0.5}
                        spacing: (SPACE_XS)

                        device_card_display_name := Label {
                            width: Fit, height: Fit
                            text: "Unknown device"
                            draw_text +: {
                                color: (RBX_FG_PRIMARY)
                                text_style: RBX_TEXT_CARD_TITLE {}
                            }
                        }

                        // Accent pill flagging the session Robrix is running as.
                        // Toggled per-card in DeviceCard::draw_walk.
                        device_card_current_badge := RoundedView {
                            visible: false
                            width: Fit, height: Fit
                            align: Align{y: 0.5}
                            padding: Inset{left: 9, right: 9, top: 3, bottom: 3}
                            show_bg: true
                            draw_bg +: {
                                color: (RBX_ACCENT_SOFT)
                                border_radius: (RBX_RADIUS_PILL)
                            }
                            Label {
                                width: Fit, height: Fit
                                draw_text +: {
                                    text_style: RBX_TEXT_BADGE {}
                                    color: (RBX_ACCENT)
                                }
                                text: "This device"
                            }
                        }
                    }
                    device_card_device_id := Label {
                        width: Fill, height: Fit
                        text: ""
                        draw_text +: {
                            color: (RBX_FG_TERTIARY)
                            text_style: RBX_TEXT_META {}
                        }
                    }
                }

                // Destructive action — red text on the standard "negative"
                // outlined background.
                device_card_remove_button := RobrixNegativeIconButton {
                    width: Fit, height: (RBX_CONTROL_H_SM)
                    padding: Inset{top: 6, bottom: 6, left: 12, right: 12}
                    spacing: 5
                    text: "Remove"
                    draw_icon.svg: (ICON_TRASH)
                    icon_walk: Walk{width: 14, height: 14}
                    draw_bg +: { border_radius: (RBX_RADIUS_XS) }
                }
            }

            device_card_divider := LineH {
                height: 1.0
                margin: Inset{top: (SPACE_XS), bottom: (SPACE_XS)}
                draw_bg.color: (RBX_STROKE_SOFT)
            }

            // Detail row: Last active + IP address.
            device_card_detail_row := View {
                width: Fill, height: Fit
                flow: Right
                spacing: (SPACE_LG)

                device_card_last_active_col := View {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 2

                    Label {
                        text: "Last active"
                        draw_text +: {
                            color: (RBX_FG_SECONDARY)
                            text_style: RBX_TEXT_META {}
                        }
                    }
                    device_card_last_active_value := Label {
                        width: Fill, height: Fit
                        text: "—"
                        draw_text +: {
                            color: (RBX_FG_PRIMARY)
                            text_style: RBX_TEXT_BODY {}
                        }
                    }
                }
                device_card_ip_col := View {
                    width: Fill, height: Fit
                    flow: Down
                    spacing: 2

                    Label {
                        text: "IP address"
                        draw_text +: {
                            color: (RBX_FG_SECONDARY)
                            text_style: RBX_TEXT_META {}
                        }
                    }
                    device_card_ip_value := Label {
                        width: Fill, height: Fit
                        text: "—"
                        draw_text +: {
                            color: (RBX_FG_PRIMARY)
                            text_style: RBX_TEXT_BODY {}
                        }
                    }
                }
            }
        }
    }

    // ────────────────────────── DevicesScreen ──────────────────────────
    mod.widgets.DevicesScreen = #(DevicesScreen::register_widget(vm)) {
        width: Fill, height: Fill
        flow: Down
        padding: Inset{top: (SPACE_SM), bottom: (SPACE_SM)}
        spacing: (SPACE_LG)

        // Header: title + session count on the left, Refresh on the right.
        devices_header_row := View {
            width: Fill, height: Fit
            flow: Right
            align: Align{y: 0.5}
            spacing: (SPACE_SM)

            devices_header_col := View {
                width: Fill, height: Fit
                flow: Down
                spacing: 2

                devices_header_label := Label {
                    width: Fill, height: Fit
                    text: "Where you're signed in"
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: RBX_TEXT_PAGE_TITLE {}
                    }
                }
                devices_count_label := Label {
                    width: Fill, height: Fit
                    text: "Checking your sessions…"
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_META {}
                    }
                }
            }

            devices_refresh_button := RobrixIconButton {
                width: Fit, height: (RBX_CONTROL_H_MD)
                padding: Inset{top: 8, bottom: 8, left: 12, right: 12}
                spacing: 5
                text: "Refresh"
                draw_icon.svg: (ICON_ROTATE_CW)
                draw_icon.color: (RBX_ACCENT)
                icon_walk: Walk{width: 14, height: 14}
                draw_bg +: {
                    color: (RBX_BG_SURFACE)
                    color_hover: (RBX_BG_HOVER)
                    color_down: (RBX_BG_PRESSED)
                    border_radius: (RBX_RADIUS_XS)
                    border_size: 1.0
                    border_color: (RBX_STROKE_SOFT)
                }
                draw_text +: {
                    color: (RBX_ACCENT)
                    color_hover: (RBX_ACCENT)
                    color_down: (RBX_ACCENT)
                }
            }
        }

        // The list itself — a PortalList so off-screen cards are never laid out
        // or drawn (true UI-level virtualization). Loading / empty / error are
        // rendered as list entries so they sit inside the scroll region.
        devices_list := PortalList {
            width: Fill, height: Fill
            keep_invisible: false
            max_pull_down: 0.0
            auto_tail: false
            flow: Down
            grab_key_focus: false

            device_item := mod.widgets.DeviceCard {}

            devices_loading_entry := View {
                width: Fill, height: 90
                flow: Right
                align: Align{x: 0.5, y: 0.5}
                LoadingSpinner {
                    width: 26, height: 26
                    draw_bg +: {
                        color: (RBX_ACCENT)
                        border_size: 3.0
                    }
                }
            }

            devices_status_entry := View {
                width: Fill, height: Fit
                padding: Inset{top: 28, bottom: 28, left: 16, right: 16}
                flow: Down
                align: Align{x: 0.5, y: 0.5}
                spacing: (SPACE_XS)

                devices_status_label := Label {
                    width: Fit, height: Fit
                    align: Align{x: 0.5}
                    draw_text +: {
                        color: (RBX_FG_TERTIARY)
                        text_style: RBX_TEXT_BODY {}
                    }
                    text: "No devices found."
                }
            }
        }
    }
}

// ────────────────────────────── actions ──────────────────────────────

/// Emitted by a `DeviceCard` when the user clicks its Remove button. The
/// parent `DevicesScreen` listens and translates this into a
/// `ConfirmDeleteAction::Show`.
#[derive(Clone, Debug, Default)]
pub enum DeviceRowAction {
    #[default]
    None,
    RemoveClicked {
        device_id: String,
        display_label: String,
    },
}

// ───────────────────────────── DeviceCard ───────────────────────────────

#[derive(Script, ScriptHook, Widget)]
pub struct DeviceCard {
    #[deref] view: View,
    /// Set on every redraw from the parent's scope props.
    #[rust] device: Option<DeviceInfo>,
}

impl Widget for DeviceCard {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        if let Event::Actions(actions) = event {
            if self
                .view
                .button(cx, ids!(device_card_remove_button))
                .clicked(actions)
            {
                if let Some(device) = self.device.as_ref() {
                    let label = device
                        .display_name
                        .clone()
                        .unwrap_or_else(|| "Unknown device".to_string());
                    cx.action(DeviceRowAction::RemoveClicked {
                        device_id: device.device_id.clone(),
                        display_label: label,
                    });
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if let Some(device) = scope.props.get::<DeviceInfo>() {
            self.device = Some(device.clone());
            self.view
                .label(cx, ids!(device_card_display_name))
                .set_text(
                    cx,
                    device.display_name.as_deref().unwrap_or("Unknown device"),
                );
            self.view
                .label(cx, ids!(device_card_device_id))
                .set_text(cx, &device.device_id);
            self.view
                .view(cx, ids!(device_card_current_badge))
                .set_visible(cx, device.is_current);
            self.view
                .label(cx, ids!(device_card_last_active_value))
                .set_text(cx, &format_last_active(device));
            self.view
                .label(cx, ids!(device_card_ip_value))
                .set_text(cx, device.last_seen_ip.as_deref().unwrap_or("—"));
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

// ──────────────────────────── DevicesScreen ─────────────────────────────

#[derive(Script, ScriptHook, Widget)]
pub struct DevicesScreen {
    #[deref] view: View,
    /// The current device list, current-session-first then freshest-first (the
    /// homeserver returns them in arbitrary order; we sort on update).
    #[rust] devices: Vec<DeviceInfo>,
    /// One-shot init flag so we only auto-fetch on first draw.
    #[rust] initialized: bool,
    /// `true` while we have a `GetDeviceList` in flight.
    #[rust] fetching: bool,
    /// Message shown in the empty / error list entry.
    #[rust] status_text: String,
}

impl Widget for DevicesScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        if let Event::Actions(actions) = event {
            // Refresh button
            if self
                .view
                .button(cx, ids!(devices_refresh_button))
                .clicked(actions)
            {
                self.request_fetch(cx);
            }

            for action in actions {
                // DeviceCard Remove clicked
                if let Some(DeviceRowAction::RemoveClicked {
                    device_id,
                    display_label,
                }) = action.downcast_ref()
                {
                    self.open_remove_confirmation(cx, device_id.clone(), display_label.clone());
                    continue;
                }
                // Async results from the worker
                if let Some(account_action) = action.downcast_ref::<AccountDataAction>() {
                    match account_action {
                        AccountDataAction::DeviceListFetched(list) => {
                            self.apply_device_list(cx, list.clone());
                        }
                        AccountDataAction::DeviceListFetchFailed(err) => {
                            self.fetching = false;
                            self.status_text = format!("Couldn't load devices: {err}");
                            self.update_count_label(cx);
                            self.view.redraw(cx);
                        }
                        AccountDataAction::DeviceDeleteResult { device_id, outcome } => {
                            use crate::sliding_sync::DeviceDeleteOutcome::*;
                            match outcome {
                                Removed => {
                                    // Drop the device from local state instantly
                                    // and trigger a background refresh to confirm.
                                    self.devices.retain(|d| &d.device_id != device_id);
                                    self.update_count_label(cx);
                                    self.view.redraw(cx);
                                    self.request_fetch(cx);
                                }
                                NeedsAuth { fallback_url } => {
                                    self.prompt_browser_reauth(cx, fallback_url.clone());
                                }
                                Error(msg) => {
                                    let msg_for_notification = msg.clone();
                                    let device_id_for_retry = device_id.clone();
                                    enqueue_notification(NotificationItem {
                                        kind: PopupKind::Error,
                                        title: Some("Failed to remove device".into()),
                                        message: format!("Error: {msg}").into(),
                                        actions: vec![
                                            NotificationAction::new("Retry", NotifActionStyle::Primary, move |_cx| {
                                                submit_async_request(MatrixRequest::DeleteDevice {
                                                    device_id: device_id_for_retry.clone(),
                                                });
                                            }),
                                            NotificationAction::new("Copy error", NotifActionStyle::Neutral, move |cx| {
                                                cx.copy_to_clipboard(&format!("Device deletion failed: {msg_for_notification}"));
                                            }),
                                        ],
                                        auto_dismissal_duration: Some(8.0),
                                        ..Default::default()
                                    });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if !self.initialized {
            self.initialized = true;
            self.request_fetch(cx);
        }

        while let Some(widget_to_draw) = self.view.draw_walk(cx, scope, walk).step() {
            let plist = widget_to_draw.as_portal_list();
            let Some(mut list) = plist.borrow_mut() else {
                continue;
            };

            let n = self.devices.len();
            // First load with nothing yet → spinner. Empty after a completed
            // fetch (or a failure) → status message. Otherwise the cards. A
            // refresh over an already-populated list keeps the cards visible.
            let show_loading = self.fetching && n == 0;
            let show_status = !self.fetching && n == 0;
            let total = if show_loading || show_status { 1 } else { n };
            list.set_item_range(cx, 0, total);

            while let Some(item_id) = list.next_visible_item(cx) {
                if show_loading {
                    let item = list.item(cx, item_id, id!(devices_loading_entry));
                    item.draw_all(cx, scope);
                } else if show_status {
                    let item = list.item(cx, item_id, id!(devices_status_entry));
                    item.child_by_path(ids!(devices_status_label))
                        .as_label()
                        .set_text(cx, &self.status_text);
                    item.draw_all(cx, scope);
                } else if item_id < n {
                    let device = self.devices[item_id].clone();
                    let item = list.item(cx, item_id, id!(device_item));
                    let mut props_scope = Scope::with_props(&device);
                    item.draw_all(cx, &mut props_scope);
                }
            }
        }
        DrawStep::done()
    }
}

impl DevicesScreen {
    fn request_fetch(&mut self, cx: &mut Cx) {
        self.fetching = true;
        self.update_count_label(cx);
        self.view.redraw(cx);
        submit_async_request(MatrixRequest::GetDeviceList);
    }

    fn apply_device_list(&mut self, cx: &mut Cx, mut list: Vec<DeviceInfo>) {
        // Current session first, then freshest-last-active first.
        list.sort_by(|a, b| {
            b.is_current
                .cmp(&a.is_current)
                .then_with(|| {
                    b.last_seen_ts_ms
                        .unwrap_or(0)
                        .cmp(&a.last_seen_ts_ms.unwrap_or(0))
                })
        });
        self.devices = list;
        self.fetching = false;
        if self.devices.is_empty() {
            self.status_text = "No devices found.".to_string();
        }
        self.update_count_label(cx);
        self.view.redraw(cx);
    }

    /// Update the subtitle under the page title to reflect the current state.
    fn update_count_label(&self, cx: &mut Cx) {
        let text = if self.fetching && self.devices.is_empty() {
            "Checking your sessions…".to_string()
        } else {
            match self.devices.len() {
                0 => "No active sessions".to_string(),
                1 => "1 active session".to_string(),
                n => format!("{n} active sessions"),
            }
        };
        self.view
            .label(cx, ids!(devices_count_label))
            .set_text(cx, &text);
    }

    /// Show the user a "your server wants you to re-authenticate in the
    /// browser" prompt. On Open they're sent to the homeserver's UIA
    /// fallback page; the UIA session is valid for ~10 min, so after they
    /// come back they can re-click Remove and it'll succeed.
    fn prompt_browser_reauth(&self, cx: &mut Cx, fallback_url: String) {
        let url_for_callback = fallback_url.clone();
        let content = ConfirmationModalContent {
            title_text: Cow::Borrowed("Re-authenticate to remove this device"),
            body_text: Cow::Owned(format!(
                "Your homeserver requires you to re-authenticate before deleting \
                 this device. We'll open the authentication page in your browser. \
                 After you finish there, come back here and click Remove device \
                 again to complete the removal.\n\n{fallback_url}"
            )),
            accept_button_text: Some(Cow::Borrowed("Open in browser")),
            cancel_button_text: Some(Cow::Borrowed("Cancel")),
            on_accept_clicked: Some(Box::new(move |_cx| {
                if let Err(e) = robius_open::Uri::new(&url_for_callback).open() {
                    let url_for_copy = url_for_callback.clone();
                    let error_msg = format!("{e:?}");
                    let error_for_copy = error_msg.clone();
                    enqueue_notification(NotificationItem {
                        kind: PopupKind::Error,
                        title: Some("Couldn't open browser".into()),
                        message: error_msg.into(),
                        actions: vec![
                            NotificationAction::new("Retry", NotifActionStyle::Primary, move |_cx| {
                                let _ = robius_open::Uri::new(&url_for_callback).open();
                            }),
                            NotificationAction::new("Copy details", NotifActionStyle::Neutral, move |cx| {
                                cx.copy_to_clipboard(&format!("Failed to open: {}\nError: {}", url_for_copy, error_for_copy));
                            }),
                        ],
                        auto_dismissal_duration: Some(8.0),
                        ..Default::default()
                    });
                }
            })),
            on_cancel_clicked: None,
        };
        cx.action(PositiveConfirmationModalAction::Show(RefCell::new(Some(
            content,
        ))));
    }

    fn open_remove_confirmation(&self, cx: &mut Cx, device_id: String, display_label: String) {
        let body = format!(
            "Make sure you always have access to another verified device or your \
             recovery key to avoid losing your encrypted chat history.\n\n{display_label}\n{device_id}"
        );
        let device_id_for_callback = device_id.clone();
        let content = ConfirmationModalContent {
            title_text: Cow::Borrowed("Are you sure you want to remove this device?"),
            body_text: Cow::Owned(body),
            accept_button_text: Some(Cow::Borrowed("Remove device")),
            cancel_button_text: Some(Cow::Borrowed("Cancel")),
            on_accept_clicked: Some(Box::new(move |_cx| {
                submit_async_request(MatrixRequest::DeleteDevice {
                    device_id: device_id_for_callback.clone(),
                });
            })),
            on_cancel_clicked: None,
        };
        cx.action(ConfirmDeleteAction::Show(RefCell::new(Some(content))));
    }
}

// ────────────────────────────── helpers ──────────────────────────────

fn format_last_active(device: &DeviceInfo) -> String {
    let Some(ms) = device.last_seen_ts_ms else {
        return "—".to_string();
    };
    let Some(dt) = DateTime::from_timestamp_millis(ms) else {
        return "—".to_string();
    };
    let local = dt.with_timezone(&Local);
    // e.g. "Tue, May 19, 2026 at 5:17 PM"
    local.format("%a, %b %-d, %Y at %-I:%M %p").to_string()
}
