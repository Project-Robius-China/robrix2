//! Centered message toasts + richer notification cards, on one overlay host.
//!
//! Two presentational widgets, one manager:
//!   * **Message toast** (`MessageToastDesktop` / `MessageToastMobile`) — a small,
//!     top-centered white card with a colored circular icon and one short line.
//!     For lightweight status blips ("Room ID copied", "Saving…").
//!   * **Notification card** (`NotificationCard*`) — a white card with a title,
//!     description, a colored circular icon, an optional custom icon and up to
//!     three custom action buttons. Top-right stacked on desktop; a full-width
//!     top banner on mobile (one at a time, the rest queued).
//!
//! Call sites do not pick a widget. `enqueue_popup_notification` auto-routes by
//! message volume (short → toast, long/multiline → notification); the richer
//! `enqueue_notification` is for explicit title/actions/custom-icon cases.
//! All routing (toast vs card, desktop vs mobile, mobile demotion) happens in
//! [`RobrixPopupNotification::enqueue_internal`], which runs with a `Cx`.

use std::borrow::Cow;
use crossbeam_queue::SegQueue;
use makepad_widgets::*;
use crate::{LivePtr, view_from_live_ptr};
use crate::settings::app_preferences::effective_is_desktop;

/// Messages whose length is at or below this (in characters) and that fit on a
/// single line render as a lightweight toast; longer ones are promoted to a
/// notification card. Derived from a survey of all call sites (~60% toast).
const TOAST_MAX_CHARS: usize = 48;

/// Max concurrently-visible toasts; oldest is dropped past this.
const TOAST_CAP_DESKTOP: usize = 4;
const TOAST_CAP_MOBILE: usize = 3;

/// Max concurrently-visible notification cards; extras queue in `notif_backlog`.
/// Mobile shows one at a time so a banner never dominates the small screen.
const NOTIF_CAP_DESKTOP: usize = 5;
const NOTIF_CAP_MOBILE: usize = 1;

static PENDING_POPUP_NOTIFICATIONS: SegQueue<PendingItem> = SegQueue::new();

/// Internal queue payload: either a legacy popup (auto-routed) or an explicit
/// rich notification.
enum PendingItem {
    Popup(PopupItem),
    Notification(NotificationItem),
}

/// Displays a new popup notification with a popup item.
///
/// This is the legacy, fire-and-forget entry point used across the app. It can
/// be called without a Makepad widget context (e.g. from async tasks). Short
/// messages render as a centered toast; long or multi-line messages are
/// automatically promoted to a notification card (with a kind-derived title).
///
/// Notifications are shown in the order they were enqueued and dismiss either
/// manually (close button / action) or automatically. Maximum auto-dismissal
/// duration is 3 minutes.
pub fn enqueue_popup_notification(
    message: impl Into<Cow<'static, str>>,
    kind: PopupKind,
    auto_dismissal_duration: Option<f64>,
) {
    let popup_item = PopupItem {
        message: message.into(),
        kind,
        // Limit auto dismiss duration to 180 seconds.
        auto_dismissal_duration: auto_dismissal_duration.map(|d| d.min(3. * 60.)),
    };
    PENDING_POPUP_NOTIFICATIONS.push(PendingItem::Popup(popup_item));
    SignalToUI::set_ui_signal();
}

/// Displays a rich notification card with a title, body, optional custom icon,
/// and up to three custom action buttons.
///
/// Use this when a message needs a button (Retry / Open / Undo …) or a custom
/// title/icon. On mobile, a notification with no actions and no custom icon is
/// automatically demoted to a lightweight toast to save space.
///
/// ```no_run
/// # use std::borrow::Cow;
/// # use makepad_widgets::Cx;
/// # use robrix::shared::popup_list::*;
/// enqueue_notification(NotificationItem {
///     kind: PopupKind::Error,
///     title: Some(Cow::Borrowed("Couldn't switch account")),
///     message: Cow::Borrowed("The request timed out. Check your connection."),
///     actions: vec![
///         NotificationAction::new("Retry", NotifActionStyle::Primary, |_cx: &mut Cx| {
///             // re-submit the request…
///         }),
///     ],
///     ..Default::default()
/// });
/// ```
pub fn enqueue_notification(mut item: NotificationItem) {
    item.auto_dismissal_duration = item.auto_dismissal_duration.map(|d| d.min(3. * 60.));
    PENDING_POPUP_NOTIFICATIONS.push(PendingItem::Notification(item));
    SignalToUI::set_ui_signal();
}

/// Retrieves a mutable reference to the global `RobrixPopupNotificationRef`.
pub fn get_global_popup_list(cx: &mut Cx) -> &mut RobrixPopupNotificationRef {
    cx.get_global::<RobrixPopupNotificationRef>()
}

/// Sets the global popup list notification widget reference.
pub fn set_global_popup_list(cx: &mut Cx, parent_ref: &WidgetRef) {
    Cx::set_global(
        cx,
        parent_ref.robrix_popup_notification(cx, ids!(popup_notification)),
    );
}

/// Kind of a notification — defines the severity color (and the default icon).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PopupKind {
    /// No icon; a neutral card.
    #[default]
    Blank,
    /// Red circle + forbidden icon. Failed / rejected / error.
    Error,
    /// Blue circle + info icon. Capability / neutral status.
    Info,
    /// Green circle + checkmark icon. Connected / done / synced.
    Success,
    /// Amber circle + warning icon. Approval required / pending.
    Warning,
}

/// The glyph drawn inside the colored circle.
///
/// `Auto` derives the glyph from [`PopupKind`]; `Hidden` removes the circle.
/// The remaining variants let a caller override just the glyph while keeping the
/// kind's circle color (the "custom icon" feature). Each maps to a registered
/// `ICON_*` resource; dynamic per-call SVG paths are intentionally not supported
/// because runtime `script_apply_eval!` cannot resolve arbitrary dependencies.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NotificationIcon {
    #[default]
    Auto,
    Hidden,
    Info,
    Success,
    Warning,
    Error,
    Forbidden,
    Checkmark,
    CloudCheckmark,
    Refresh,
    Close,
}

/// Visual style of a notification action button.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum NotifActionStyle {
    /// Solid accent (teal). The recommended/primary action.
    #[default]
    Primary,
    /// Outlined neutral. Secondary / dismiss-style action.
    Neutral,
    /// Solid red. Destructive action.
    Danger,
}

/// A custom button shown in the footer of a notification card.
///
/// `on_click` runs (with `&mut Cx`) when the button is tapped; the card then
/// dismisses itself. Put the follow-up here — `submit_async_request(...)`,
/// `Cx::post_action(MyAction::…)`, navigation, etc.
pub struct NotificationAction {
    pub label: Cow<'static, str>,
    pub style: NotifActionStyle,
    pub on_click: Box<dyn FnMut(&mut Cx) + Send>,
}

impl NotificationAction {
    pub fn new(
        label: impl Into<Cow<'static, str>>,
        style: NotifActionStyle,
        on_click: impl FnMut(&mut Cx) + Send + 'static,
    ) -> Self {
        Self { label: label.into(), style, on_click: Box::new(on_click) }
    }
}

/// Lightweight popup item (legacy API). Renders as a toast, or is promoted to a
/// notification card when long.
#[derive(Default, Debug, Clone)]
pub struct PopupItem {
    /// Text to be displayed.
    pub message: Cow<'static, str>,
    /// Auto-close duration in seconds (max 3 minutes). `None` = manual close.
    pub auto_dismissal_duration: Option<f64>,
    /// Severity, see [`PopupKind`].
    pub kind: PopupKind,
}

/// Rich notification item — title + body + optional icon + actions.
pub struct NotificationItem {
    /// Severity (defines the circle color and default icon).
    pub kind: PopupKind,
    /// Bold title line. `None` derives one from `kind` (e.g. "Error").
    pub title: Option<Cow<'static, str>>,
    /// Body / description text.
    pub message: Cow<'static, str>,
    /// Glyph override; defaults to the kind's icon.
    pub icon: NotificationIcon,
    /// Up to three action buttons (extras are ignored).
    pub actions: Vec<NotificationAction>,
    /// Auto-close duration in seconds (max 3 minutes). `None` = manual close.
    pub auto_dismissal_duration: Option<f64>,
}

impl Default for NotificationItem {
    fn default() -> Self {
        Self {
            kind: PopupKind::Info,
            title: None,
            message: Cow::Borrowed(""),
            icon: NotificationIcon::Auto,
            actions: Vec::new(),
            auto_dismissal_duration: Some(8.0),
        }
    }
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // Colored circle behind the kind glyph. Small (toast) variant.
    mod.widgets.NotifIconCircleSm = CircleView {
        width: 26, height: 26,
        align: Align{ x: 0.5, y: 0.5 }
        show_bg: true,
        draw_bg +: { color: (RBX_INFO_FG) }
        popup_icon := Icon {
            width: Fit, height: Fit,
            draw_icon +: {
                svg: (ICON_INFO),
                color: (COLOR_PRIMARY),
            }
            icon_walk: Walk{ width: 15, height: 15 }
        }
    }

    // Colored circle behind the kind glyph. Large (notification) variant.
    mod.widgets.NotifIconCircleLg = CircleView {
        width: 34, height: 34,
        align: Align{ x: 0.5, y: 0.5 }
        show_bg: true,
        draw_bg +: { color: (RBX_INFO_FG) }
        popup_icon := Icon {
            width: Fit, height: Fit,
            draw_icon +: {
                svg: (ICON_INFO),
                color: (COLOR_PRIMARY),
            }
            icon_walk: Walk{ width: 20, height: 20 }
        }
    }

    // Minimal close affordance (thin grey ×, hover wash, no border).
    mod.widgets.NotifCloseButton = ButtonFlat {
        width: 24, height: 24,
        text: "",
        spacing: 0,
        margin: 0,
        padding: 0,
        align: Align{ x: 0.5, y: 0.5 }
        label_walk: Walk{ width: 0, height: 0 }
        icon_walk: Walk{ width: 13, height: 13, margin: 0 }
        draw_bg +: {
            border_size: 0.0
            border_radius: 6.0
            color: #00000000
            color_hover: #x00000012
            color_down: #x0000001E
            border_color: #00000000
            border_color_hover: #00000000
            border_color_down: #00000000
        }
        draw_icon +: {
            svg: (ICON_CLOSE),
            color: (RBX_FG_SECONDARY),
        }
    }

    // Footer action button base. Recolored per-style at runtime.
    mod.widgets.NotifActionButton = mod.widgets.RobrixIconButton {
        height: 30,
        padding: Inset{ left: 14, right: 14, top: 7, bottom: 7 }
        align: Align{ x: 0.5, y: 0.5 }
        icon_walk: Walk{ width: 0, height: 0 }
        draw_bg +: {
            border_size: 0.0
            border_radius: (RBX_RADIUS_SM)
        }
        draw_text +: {
            text_style: RBX_TEXT_BODY {},
        }
        text: ""
    }

    // ---- Message toast (lightweight) ----

    mod.widgets.MessageToastDesktop = View {
        width: Fill, height: Fit,
        clip_x: false, clip_y: false,
        align: Align{ x: 0.5, y: 0.0 }
        toast_card := RoundedShadowView {
            width: Fit, height: Fit,
            flow: Right,
            align: Align{ y: 0.5 }
            spacing: 10,
            padding: Inset{ left: 14, right: 10, top: 10, bottom: 10 }
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_MD)
                border_size: 1.0
                border_color: (RBX_STROKE_OVERLAY)
                shadow_color: (RBX_SHADOW)
                shadow_radius: 11.0
                shadow_offset: vec2(0.0, 4.0)
            }
            icon_circle := mod.widgets.NotifIconCircleSm {}
            toast_label := Label {
                width: Fit, height: Fit,
                draw_text +: {
                    color: (RBX_FG_PRIMARY)
                    text_style: RBX_TEXT_BODY {},
                }
                text: ""
            }
            close_button := mod.widgets.NotifCloseButton {}
        }
    }

    mod.widgets.MessageToastMobile = View {
        width: Fill, height: Fit,
        clip_x: false, clip_y: false,
        align: Align{ x: 0.5, y: 0.0 }
        padding: Inset{ left: 12, right: 12 }
        toast_card := RoundedShadowView {
            width: Fill, height: Fit,
            flow: Right,
            align: Align{ y: 0.5 }
            spacing: 10,
            padding: Inset{ left: 14, right: 10, top: 11, bottom: 11 }
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_MD)
                border_size: 1.0
                border_color: (RBX_STROKE_OVERLAY)
                shadow_color: (RBX_SHADOW)
                shadow_radius: 11.0
                shadow_offset: vec2(0.0, 4.0)
            }
            icon_circle := mod.widgets.NotifIconCircleSm {}
            toast_label := Label {
                width: Fill, height: Fit,
                draw_text +: {
                    color: (RBX_FG_PRIMARY)
                    text_style: RBX_TEXT_BODY {},
                }
                text: ""
            }
            close_button := mod.widgets.NotifCloseButton {}
        }
    }

    // ---- Notification card (rich) ----
    // Shared body; the wrapper sets the width (fixed on desktop, Fill on mobile).
    mod.widgets.NotificationCardBody = RoundedShadowView {
        width: Fill, height: Fit,
        flow: Down,
        padding: Inset{ left: 14, right: 12, top: 13, bottom: 12 }
        draw_bg +: {
            color: (RBX_BG_SURFACE)
            border_radius: (RBX_RADIUS_MD)
            border_size: 1.0
            border_color: (RBX_STROKE_OVERLAY)
            shadow_color: (RBX_SHADOW)
            shadow_radius: 13.0
            shadow_offset: vec2(0.0, 5.0)
        }
        header := View {
            width: Fill, height: Fit,
            flow: Right,
            spacing: 11,
            align: Align{ y: 0.0 }
            icon_circle := mod.widgets.NotifIconCircleLg {}
            text_col := View {
                width: Fill, height: Fit,
                flow: Down,
                spacing: 3,
                padding: Inset{ top: 1 }
                notif_title := Label {
                    width: Fill, height: Fit,
                    draw_text +: {
                        color: (RBX_FG_PRIMARY)
                        text_style: RBX_TEXT_CARD_TITLE {},
                    }
                    text: ""
                }
                notif_label := Label {
                    width: Fill, height: Fit,
                    draw_text +: {
                        color: (RBX_FG_SECONDARY)
                        text_style: RBX_TEXT_BODY {},
                    }
                    text: ""
                }
            }
            close_button := mod.widgets.NotifCloseButton {}
        }
        actions_row := View {
            width: Fill, height: Fit,
            flow: Right,
            spacing: 8,
            align: Align{ x: 1.0, y: 0.5 }
            margin: Inset{ top: 11 }
            visible: false,
            action_btn_0 := mod.widgets.NotifActionButton { visible: false }
            action_btn_1 := mod.widgets.NotifActionButton { visible: false }
            action_btn_2 := mod.widgets.NotifActionButton { visible: false }
        }
    }

    mod.widgets.NotificationCardDesktop = View {
        width: Fill, height: Fit,
        clip_x: false, clip_y: false,
        align: Align{ x: 1.0, y: 0.0 }
        padding: Inset{ left: 14, right: 14 }
        notif_card := mod.widgets.NotificationCardBody { width: 330 }
    }

    mod.widgets.NotificationCardMobile = View {
        width: Fill, height: Fit,
        clip_x: false, clip_y: false,
        align: Align{ x: 0.5, y: 0.0 }
        padding: Inset{ left: 12, right: 12 }
        notif_card := mod.widgets.NotificationCardBody { width: Fill }
    }

    // ---- Overlay host ----
    // Full-screen, transparent, non-capturing. Stacks toasts/cards top-down;
    // each entry's wrapper aligns itself (center for toasts, right/full for
    // notifications), so the host just lays them out vertically.
    mod.widgets.RobrixPopupNotification = set_type_default() do #(RobrixPopupNotification::register_widget(vm)) {
        ..mod.widgets.SolidView

        width: Fill, height: Fill,
        flow: Down,
        spacing: 10,
        padding: Inset{ top: 16, left: 0, right: 0, bottom: 0 }
        align: Align{ x: 0.0, y: 0.0 }
        draw_bg +: {
            color: (COLOR_TRANSPARENT)
        }
        toast_desktop: mod.widgets.MessageToastDesktop {}
        toast_mobile:  mod.widgets.MessageToastMobile {}
        notif_desktop: mod.widgets.NotificationCardDesktop {}
        notif_mobile:  mod.widgets.NotificationCardMobile {}
    }

    // Full-screen overlay container placed once near the root of the app.
    mod.widgets.PopupList = View {
        width: Fill, height: Fill,
        popup_notification := mod.widgets.RobrixPopupNotification {}
    }
}

/// A single live popup (toast or notification card) plus its dismissal state.
struct PopupEntry {
    view: View,
    close_timer: Timer,
    is_notification: bool,
    /// Click handlers for action buttons 0..N, parallel to the visible buttons.
    /// Empty for toasts and for action-less notifications.
    actions: Vec<Box<dyn FnMut(&mut Cx) + Send>>,
}

/// The overlay host that owns and draws all live popups.
#[derive(Script, Widget)]
pub struct RobrixPopupNotification {
    #[uid] uid: WidgetUid,
    #[source] source: ScriptObjectRef,
    #[live] toast_desktop: Option<LivePtr>,
    #[live] toast_mobile: Option<LivePtr>,
    #[live] notif_desktop: Option<LivePtr>,
    #[live] notif_mobile: Option<LivePtr>,

    #[rust] draw_list: Option<DrawList2d>,
    #[redraw] #[live] draw_bg: DrawQuad,
    #[layout] layout: Layout,
    #[walk] walk: Walk,

    /// Currently-visible popups, oldest first.
    #[rust] popups: Vec<PopupEntry>,
    /// Notification cards waiting for a free slot (mobile shows one at a time).
    #[rust] notif_backlog: Vec<NotificationItem>,
    /// Cached layout mode, refreshed whenever we process the queue. Drives the
    /// top inset at draw time (extra room for the mobile status bar / notch).
    #[rust(true)] is_desktop: bool,
}

impl ScriptHook for RobrixPopupNotification {
    fn on_after_new(&mut self, vm: &mut ScriptVm) {
        self.draw_list = Some(DrawList2d::script_new(vm));
    }

    fn on_after_apply(
        &mut self,
        vm: &mut ScriptVm,
        _apply: &Apply,
        _scope: &mut Scope,
        _value: ScriptValue,
    ) {
        vm.with_cx_mut(|cx| {
            if let Some(draw_list) = &self.draw_list {
                draw_list.redraw(cx);
            }
        });
    }
}

impl Widget for RobrixPopupNotification {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        if matches!(event, Event::Signal) {
            self.is_desktop = effective_is_desktop(cx);
            while let Some(pending) = PENDING_POPUP_NOTIFICATIONS.pop() {
                self.enqueue_internal(cx, pending);
            }
        }
        if self.popups.is_empty() {
            return;
        }
        // Keep the layout-mode cache fresh while popups are visible so the
        // overlay's top inset tracks window resize / device rotation.
        self.is_desktop = effective_is_desktop(cx);

        let mut remove_index = None;
        for (index, popup) in self.popups.iter_mut().enumerate() {
            popup.view.handle_event(cx, event, scope);
            if remove_index.is_none() && popup.close_timer.is_event(event).is_some() {
                remove_index = Some(index);
            }
        }
        if let Some(index) = remove_index {
            self.popups.remove(index);
            self.fill_backlog(cx);
            self.redraw_overlay(cx);
        }

        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, _walk: Walk) -> DrawStep {
        let draw_list = self.draw_list.as_mut().unwrap();
        draw_list.begin_overlay_reuse(cx);

        // Only establish the full-screen overlay turtle while something is
        // showing, so the transparent host quad never covers the (interactive)
        // app when idle.
        if !self.popups.is_empty() {
            let size = cx.current_pass_size();
            // Leave room for the mobile status bar / notch above the first card,
            // and keep a comfortable gap from the top on desktop.
            self.layout.padding.top = if self.is_desktop { 30.0 } else { 60.0 };
            cx.begin_root_turtle(size, self.layout);
            self.draw_bg.begin(cx, self.walk, self.layout);
            for popup in self.popups.iter_mut() {
                let _ = popup.view.draw_all(cx, scope);
            }
            self.draw_bg.end(cx);
            cx.end_pass_sized_turtle();
        }

        self.draw_list.as_mut().unwrap().end(cx);
        DrawStep::done()
    }
}

impl RobrixPopupNotification {
    fn redraw_overlay(&self, cx: &mut Cx) {
        if let Some(draw_list) = &self.draw_list {
            draw_list.redraw(cx);
        }
        self.draw_bg.redraw(cx);
    }

    /// Routes a queued item to a toast or a notification card, applying the
    /// length heuristic and the mobile demotion rule.
    fn enqueue_internal(&mut self, cx: &mut Cx, pending: PendingItem) {
        let desktop = effective_is_desktop(cx);
        self.is_desktop = desktop;
        match pending {
            PendingItem::Popup(item) => {
                if item.message.chars().count() > TOAST_MAX_CHARS
                    || item.message.contains('\n')
                {
                    self.add_notification(cx, notification_from_popup(item), desktop);
                } else {
                    self.add_toast(cx, item, desktop);
                }
            }
            PendingItem::Notification(ni) => {
                // On mobile a card with nothing to interact with (no actions,
                // no custom glyph) wastes space — show it as a toast instead.
                let demote = !desktop
                    && ni.actions.is_empty()
                    && matches!(ni.icon, NotificationIcon::Auto | NotificationIcon::Hidden);
                if demote {
                    self.add_toast(cx, popup_from_notification(ni), desktop);
                } else {
                    self.add_notification(cx, ni, desktop);
                }
            }
        }
    }

    fn add_toast(&mut self, cx: &mut Cx, item: PopupItem, desktop: bool) {
        let ptr = if desktop { self.toast_desktop } else { self.toast_mobile };
        let mut view = view_from_live_ptr(cx, ptr);
        view.label(cx, ids!(toast_label)).set_text(cx, &item.message);
        apply_kind_visuals(cx, &mut view, item.kind, NotificationIcon::Auto);

        self.enforce_toast_cap(cx, desktop);

        let close_timer = match item.auto_dismissal_duration {
            Some(duration) => cx.start_timeout(duration),
            None => Timer::empty(),
        };
        self.popups.push(PopupEntry {
            view,
            close_timer,
            is_notification: false,
            actions: Vec::new(),
        });
        self.redraw_overlay(cx);
    }

    /// Drop the oldest toast(s) so a newly-arriving one stays within the cap.
    fn enforce_toast_cap(&mut self, cx: &mut Cx, desktop: bool) {
        let max = if desktop { TOAST_CAP_DESKTOP } else { TOAST_CAP_MOBILE };
        while self.popups.iter().filter(|p| !p.is_notification).count() >= max {
            if let Some(pos) = self.popups.iter().position(|p| !p.is_notification) {
                let entry = self.popups.remove(pos);
                cx.stop_timer(entry.close_timer);
            } else {
                break;
            }
        }
    }

    /// Show a notification card now, or queue it if the cap is reached.
    fn add_notification(&mut self, cx: &mut Cx, ni: NotificationItem, desktop: bool) {
        let cap = if desktop { NOTIF_CAP_DESKTOP } else { NOTIF_CAP_MOBILE };
        let active = self.popups.iter().filter(|p| p.is_notification).count();
        if active >= cap {
            self.notif_backlog.push(ni);
            return;
        }
        self.instantiate_notification(cx, ni, desktop);
    }

    fn instantiate_notification(&mut self, cx: &mut Cx, mut ni: NotificationItem, desktop: bool) {
        let ptr = if desktop { self.notif_desktop } else { self.notif_mobile };
        let mut view = view_from_live_ptr(cx, ptr);

        let title = ni
            .title
            .clone()
            .unwrap_or_else(|| Cow::Borrowed(default_title(ni.kind)));
        view.label(cx, ids!(notif_title)).set_text(cx, &title);
        view.label(cx, ids!(notif_label)).set_text(cx, &ni.message);
        apply_kind_visuals(cx, &mut view, ni.kind, ni.icon);

        let mut actions: Vec<NotificationAction> = ni.actions.drain(..).collect();
        actions.truncate(3);
        let has_actions = !actions.is_empty();
        view.view(cx, ids!(actions_row)).set_visible(cx, has_actions);

        let mut closures: Vec<Box<dyn FnMut(&mut Cx) + Send>> = Vec::new();
        for (i, action) in actions.into_iter().enumerate() {
            let btn = match i {
                0 => view.button(cx, ids!(action_btn_0)),
                1 => view.button(cx, ids!(action_btn_1)),
                _ => view.button(cx, ids!(action_btn_2)),
            };
            btn.set_text(cx, &action.label);
            btn.set_visible(cx, true);
            style_action_button(cx, btn, action.style);
            closures.push(action.on_click);
        }
        if closures.len() < 1 {
            view.widget(cx, ids!(action_btn_0)).set_visible(cx, false);
        }
        if closures.len() < 2 {
            view.widget(cx, ids!(action_btn_1)).set_visible(cx, false);
        }
        if closures.len() < 3 {
            view.widget(cx, ids!(action_btn_2)).set_visible(cx, false);
        }

        let close_timer = match ni.auto_dismissal_duration {
            Some(duration) => cx.start_timeout(duration),
            None => Timer::empty(),
        };
        self.popups.push(PopupEntry {
            view,
            close_timer,
            is_notification: true,
            actions: closures,
        });
        self.redraw_overlay(cx);
    }

    /// After a notification dismisses, promote queued ones into freed slots.
    fn fill_backlog(&mut self, cx: &mut Cx) {
        let desktop = effective_is_desktop(cx);
        self.is_desktop = desktop;
        let cap = if desktop { NOTIF_CAP_DESKTOP } else { NOTIF_CAP_MOBILE };
        while !self.notif_backlog.is_empty()
            && self.popups.iter().filter(|p| p.is_notification).count() < cap
        {
            let ni = self.notif_backlog.remove(0);
            self.instantiate_notification(cx, ni, desktop);
        }
    }
}

impl WidgetMatchEvent for RobrixPopupNotification {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        // Find the first popup whose close button or an action button was hit.
        // `usize::MAX` in the second slot means "close button".
        let mut hit: Option<(usize, usize)> = None;
        for (i, entry) in self.popups.iter().enumerate() {
            if entry.view.button(cx, ids!(close_button)).clicked(actions) {
                hit = Some((i, usize::MAX));
                break;
            }
            if !entry.actions.is_empty()
                && entry.view.button(cx, ids!(action_btn_0)).clicked(actions)
            {
                hit = Some((i, 0));
                break;
            }
            if entry.actions.len() > 1
                && entry.view.button(cx, ids!(action_btn_1)).clicked(actions)
            {
                hit = Some((i, 1));
                break;
            }
            if entry.actions.len() > 2
                && entry.view.button(cx, ids!(action_btn_2)).clicked(actions)
            {
                hit = Some((i, 2));
                break;
            }
        }

        if let Some((i, action_index)) = hit {
            let mut entry = self.popups.remove(i);
            cx.stop_timer(entry.close_timer);
            if action_index != usize::MAX {
                if let Some(on_click) = entry.actions.get_mut(action_index) {
                    on_click(cx);
                }
            }
            self.fill_backlog(cx);
            self.redraw_overlay(cx);
        }
    }
}

impl RobrixPopupNotificationRef {
    /// Enqueue a lightweight popup directly (with a `Cx` in hand). Most callers
    /// should use the free function [`enqueue_popup_notification`] instead.
    pub fn push(&self, cx: &mut Cx, popup_item: PopupItem) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.enqueue_internal(cx, PendingItem::Popup(popup_item));
        } else {
            log!("RobrixPopupNotificationRef is not initialized.");
        }
    }
}

/// Title shown when a notification doesn't carry an explicit one.
fn default_title(kind: PopupKind) -> &'static str {
    match kind {
        PopupKind::Error => "Error",
        PopupKind::Warning => "Warning",
        PopupKind::Success => "Success",
        PopupKind::Info => "Info",
        PopupKind::Blank => "Notice",
    }
}

fn notification_from_popup(item: PopupItem) -> NotificationItem {
    NotificationItem {
        kind: item.kind,
        title: None,
        message: item.message,
        icon: NotificationIcon::Auto,
        actions: Vec::new(),
        auto_dismissal_duration: item.auto_dismissal_duration,
    }
}

fn popup_from_notification(ni: NotificationItem) -> PopupItem {
    let message = match ni.title {
        Some(t) if !t.is_empty() => Cow::Owned(format!("{}: {}", t, ni.message)),
        _ => ni.message,
    };
    PopupItem {
        message,
        kind: ni.kind,
        auto_dismissal_duration: ni.auto_dismissal_duration,
    }
}

/// Sets the circle color (by kind) and glyph (by icon, falling back to kind),
/// or hides the circle entirely when there is nothing to show.
fn apply_kind_visuals(cx: &mut Cx, view: &View, kind: PopupKind, icon: NotificationIcon) {
    let mut circle = view.view(cx, ids!(icon_circle));
    let show = !matches!(icon, NotificationIcon::Hidden)
        && !(kind == PopupKind::Blank && matches!(icon, NotificationIcon::Auto));
    circle.set_visible(cx, show);
    if !show {
        return;
    }

    match kind {
        PopupKind::Info => script_apply_eval!(cx, circle, { draw_bg.color: mod.widgets.RBX_INFO_FG }),
        PopupKind::Success => script_apply_eval!(cx, circle, { draw_bg.color: mod.widgets.RBX_SUCCESS_FG }),
        PopupKind::Warning => script_apply_eval!(cx, circle, { draw_bg.color: mod.widgets.RBX_WARNING_FG }),
        PopupKind::Error => script_apply_eval!(cx, circle, { draw_bg.color: mod.widgets.RBX_DANGER_FG }),
        PopupKind::Blank => script_apply_eval!(cx, circle, { draw_bg.color: mod.widgets.RBX_NEUTRAL_FG }),
    }

    let glyph = match icon {
        NotificationIcon::Auto => kind_glyph(kind),
        other => other,
    };
    let mut popup_icon = view.widget(cx, ids!(popup_icon));
    match glyph {
        NotificationIcon::Info => script_apply_eval!(cx, popup_icon, {
            draw_icon.svg: mod.widgets.ICON_INFO,
            draw_icon.color: mod.widgets.COLOR_PRIMARY,
        }),
        NotificationIcon::Success | NotificationIcon::Checkmark => script_apply_eval!(cx, popup_icon, {
            draw_icon.svg: mod.widgets.ICON_CHECKMARK,
            draw_icon.color: mod.widgets.COLOR_PRIMARY,
        }),
        NotificationIcon::Warning => script_apply_eval!(cx, popup_icon, {
            draw_icon.svg: mod.widgets.ICON_WARNING,
            draw_icon.color: mod.widgets.COLOR_PRIMARY,
        }),
        NotificationIcon::Error | NotificationIcon::Forbidden => script_apply_eval!(cx, popup_icon, {
            draw_icon.svg: mod.widgets.ICON_FORBIDDEN,
            draw_icon.color: mod.widgets.COLOR_PRIMARY,
        }),
        NotificationIcon::CloudCheckmark => script_apply_eval!(cx, popup_icon, {
            draw_icon.svg: mod.widgets.ICON_CLOUD_CHECKMARK,
            draw_icon.color: mod.widgets.COLOR_PRIMARY,
        }),
        NotificationIcon::Refresh => script_apply_eval!(cx, popup_icon, {
            draw_icon.svg: mod.widgets.ICON_ROTATE_CW,
            draw_icon.color: mod.widgets.COLOR_PRIMARY,
        }),
        NotificationIcon::Close => script_apply_eval!(cx, popup_icon, {
            draw_icon.svg: mod.widgets.ICON_CLOSE,
            draw_icon.color: mod.widgets.COLOR_PRIMARY,
        }),
        // Auto/Hidden are resolved above.
        NotificationIcon::Auto | NotificationIcon::Hidden => {}
    }
}

fn kind_glyph(kind: PopupKind) -> NotificationIcon {
    match kind {
        PopupKind::Info => NotificationIcon::Info,
        PopupKind::Success => NotificationIcon::Success,
        PopupKind::Warning => NotificationIcon::Warning,
        PopupKind::Error => NotificationIcon::Error,
        PopupKind::Blank => NotificationIcon::Info,
    }
}

fn style_action_button(cx: &mut Cx, mut btn: ButtonRef, style: NotifActionStyle) {
    match style {
        NotifActionStyle::Primary => script_apply_eval!(cx, btn, {
            draw_bg +: {
                color: mod.widgets.RBX_ACCENT,
                color_hover: mod.widgets.RBX_ACCENT_HOVER,
                color_down: mod.widgets.RBX_ACCENT_PRESSED,
                border_size: 0.0,
                border_color: #00000000,
            },
            draw_text +: {
                color: mod.widgets.RBX_FG_ON_ACCENT,
                color_hover: mod.widgets.RBX_FG_ON_ACCENT,
                color_down: mod.widgets.RBX_FG_ON_ACCENT,
            },
        }),
        NotifActionStyle::Danger => script_apply_eval!(cx, btn, {
            draw_bg +: {
                color: mod.widgets.RBX_DANGER_FG,
                color_hover: mod.widgets.RBX_DANGER_FG,
                color_down: mod.widgets.RBX_DANGER_FG,
                border_size: 0.0,
                border_color: #00000000,
            },
            draw_text +: {
                color: mod.widgets.COLOR_PRIMARY,
                color_hover: mod.widgets.COLOR_PRIMARY,
                color_down: mod.widgets.COLOR_PRIMARY,
            },
        }),
        NotifActionStyle::Neutral => script_apply_eval!(cx, btn, {
            draw_bg +: {
                color: #00000000,
                color_hover: mod.widgets.RBX_NEUTRAL_BG,
                color_down: mod.widgets.RBX_NEUTRAL_BG,
                border_size: 1.0,
                border_color: mod.widgets.RBX_STROKE_SOFT,
                border_color_hover: mod.widgets.RBX_STROKE_STRONG,
                border_color_down: mod.widgets.RBX_STROKE_STRONG,
            },
            draw_text +: {
                color: mod.widgets.RBX_FG_PRIMARY,
                color_hover: mod.widgets.RBX_FG_PRIMARY,
                color_down: mod.widgets.RBX_FG_PRIMARY,
            },
        }),
    }
}
