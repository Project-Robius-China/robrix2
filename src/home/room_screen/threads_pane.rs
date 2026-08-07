//! The threads sliding pane: the per-room thread list with its entry
//! widgets, and the toolbar button that opens it.

use super::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.ThreadsPaneEntry = #(ThreadsPaneEntry::register_widget(vm)) {
        ..mod.widgets.RoundedView

        width: Fill
        height: Fit
        flow: Down
        spacing: 5
        padding: Inset{top: 12, right: 12, bottom: 12, left: 12}
        margin: Inset{left: 12, right: 12, top: 6, bottom: 0}
        cursor: MouseCursor.Hand

        show_bg: true
        draw_bg +: {
            color: #F8FAFD
            border_radius: 4.0
            border_size: 1.0
            border_color: #D8E0EA
        }

        title_row := View {
            width: Fill
            height: Fit
            flow: Right
            spacing: 8

            title := HtmlOrPlaintext {
                width: Fill
                height: Fit
            }

            time := Label {
                width: Fit
                height: Fit
                draw_text +: {
                    text_style: TIMESTAMP_TEXT_STYLE { font_size: 7.5 }
                    color: (TIMESTAMP_TEXT_COLOR)
                }
                text: ""
            }
        }

        subtitle := Label {
            width: Fill
            height: Fit
            flow: Flow.Right{wrap: true}
            draw_text +: {
                text_style: MESSAGE_TEXT_STYLE { font_size: 9.8 }
                color: #7B7B7B
            }
            text: ""
        }

        preview := HtmlOrPlaintext {
            width: Fill
            height: Fit
        }
    }

    // Floating circular button that opens the `ThreadsSlidingPane`.
    // Mirrors `SearchMessagesButton`'s layout (Fill/Fill overlay aligned
    // top-right) but reserves 96px on the right so it sits to the left of the
    // search (48px) and info (rightmost) buttons.
    mod.widgets.ThreadsButton = #(ThreadsButton::register_widget(vm)) {
        width: Fill,
        height: Fill,
        flow: Overlay,
        align: Align{x: 1.0, y: 0.0},
        padding: Inset{right: 96},
        visible: true,

        View {
            width: 65, height: 65,
            align: Align{x: 0.5, y: 0.0},
            flow: Overlay,

            inner_button := RobrixIconButton {
                spacing: 0,
                width: 40, height: 40,
                align: Align{x: 0.5, y: 0.5},
                margin: Inset{top: 8},

                draw_icon +: {
                    svg: (ICON_THREADS),
                    color: #555,
                }
                icon_walk: Walk{width: 18, height: 18}

                draw_bg +: {
                    background_color: #edededce,
                    background_color_hover: #d0d0d0ce,
                    pixel: fn() {
                        let sdf = Sdf2d.viewport(self.pos * self.rect_size);
                        let c = self.rect_size * 0.5;
                        sdf.circle(c.x, c.x, c.x);
                        sdf.fill_keep(mix(self.background_color, self.background_color_hover, self.hover));
                        return sdf.result
                    }
                }
            }
        }
    }

    mod.widgets.ThreadsSlidingPane = #(ThreadsSlidingPane::register_widget(vm)) {
        visible: false,
        flow: Overlay,
        width: Fill,
        height: Fill,
        align: Align{x: 1.0, y: 0}

        bg_view := SolidView {
            width: Fill
            height: Fill
            visible: false,
            show_bg: true
            draw_bg.color: (RBX_SCRIM)
        }

        main_content := SolidView {
            width: 320,
            height: Fill
            flow: Down,
            align: Align{x: 1.0}

            show_bg: true,
            draw_bg.color: (COLOR_PRIMARY)

            header := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}
                padding: Inset{top: 12, right: 10, bottom: 12, left: 15}

                title := Label {
                    width: Fit
                    height: Fit
                    draw_text +: {
                        text_style: USERNAME_TEXT_STYLE { font_size: 12.5 }
                        color: #000
                    }
                    text: "Threads"
                }

                spacer := View {
                    width: Fill
                    height: Fit
                }

                close_button := RobrixNeutralIconButton {
                    width: Fit,
                    height: Fit,
                    spacing: 0,
                    padding: 15,
                    draw_icon.svg: (ICON_CLOSE)
                    icon_walk: Walk{width: 14, height: 14}
                    text: ""
                }
            }

            room_name := Label {
                width: Fill
                height: Fit
                flow: Flow.Right{wrap: true}
                padding: Inset{left: 15, right: 15, bottom: 10}
                draw_text +: {
                    text_style: MESSAGE_TEXT_STYLE { font_size: 10.5 }
                    color: #6E6E6E
                }
                text: ""
            }

            loading_indicator := View {
                visible: false
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}
                spacing: 8
                padding: Inset{left: 15, right: 15, top: 6, bottom: 10}

                spinner := LoadingSpinner {
                    width: 18
                    height: 18
                }

                loading_label := Label {
                    width: Fit
                    height: Fit
                    draw_text +: {
                        text_style: MESSAGE_TEXT_STYLE { font_size: 10.5 }
                        color: #7B7B7B
                    }
                    text: "Loading threads..."
                }
            }

            empty_state := Label {
                visible: false
                width: Fill
                height: Fit
                flow: Flow.Right{wrap: true}
                padding: Inset{left: 15, right: 15, top: 20, bottom: 20}
                draw_text +: {
                    text_style: MESSAGE_TEXT_STYLE { font_size: 10.5 }
                    color: #7B7B7B
                }
                text: "No threads yet."
            }

            threads_list := PortalList {
                width: Fill
                height: Fill
                flow: Down
                max_pull_down: 0.0

                ThreadEntry := mod.widgets.ThreadsPaneEntry {}
            }
        }

        slide: 1.0,

        animator: Animator {
            panel: {
                default: @hide
                show: AnimatorState{
                    redraw: true,
                    from: {all: Forward {duration: 0.5}}
                    ease: Ease.ExpDecay {d1: 0.80, d2: 0.97}
                    apply: {
                        slide: 0.0
                    }
                }
                hide: AnimatorState{
                    redraw: true,
                    from: {all: Forward {duration: 0.5}}
                    ease: Ease.ExpDecay {d1: 0.80, d2: 0.97}
                    apply: {
                        slide: 1.0
                    }
                }
            }
        }
    }
}

#[derive(Clone, Default, Debug)]
pub enum ThreadsPaneAction {
    OpenThread(OwnedEventId),
    LoadMoreRequested,
    /// The pane's close button (or Esc / back / click-outside) was triggered.
    /// The room screen should call `hide_threads_pane` to animate the pane
    /// out and re-show the floating threads button.
    CloseRequested,
    #[default]
    None,
}

impl ActionDefaultRef for ThreadsPaneAction {
    fn default_ref() -> &'static Self {
        static DEFAULT: ThreadsPaneAction = ThreadsPaneAction::None;
        &DEFAULT
    }
}

#[derive(Clone, Default, Debug)]
pub enum ThreadsButtonAction {
    OpenRequested,
    #[default]
    None,
}

impl ActionDefaultRef for ThreadsButtonAction {
    fn default_ref() -> &'static Self {
        static DEFAULT: ThreadsButtonAction = ThreadsButtonAction::None;
        &DEFAULT
    }
}

#[derive(Clone, Debug)]
pub(super) struct ThreadsPaneEntryInfo {
    pub(super) thread_root_event_id: OwnedEventId,
    pub(super) title: String,
    pub(super) subtitle: String,
    pub(super) time: String,
    pub(super) preview: String,
}

#[derive(Clone, Debug)]
pub(super) struct ThreadsPaneInfo {
    pub(super) room_name: String,
    pub(super) entries: Vec<ThreadsPaneEntryInfo>,
    pub(super) status_text: String,
    pub(super) show_entries: bool,
    pub(super) loading_text: String,
    pub(super) show_loading: bool,
}

#[derive(Default)]
pub(super) struct ThreadsPaneState {
    pub(super) room_id: Option<OwnedRoomId>,
    pub(super) entries: Vec<FetchedRoomThread>,
    pub(super) prev_batch_token: Option<String>,
    pub(super) is_loading: bool,
    pub(super) initialized: bool,
    pub(super) status_text: String,
}

#[derive(Script, ScriptHook, Widget)]
pub struct ThreadsPaneEntry {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,

    #[rust] thread_root_event_id: Option<OwnedEventId>,
}

impl Widget for ThreadsPaneEntry {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        // Hit-test the parent area BEFORE propagating to children, so the inner
        // HtmlOrPlaintext (and its TextFlow / HtmlLink children) don't steal
        // FingerDown/Up — mirrors the pattern in rooms_list_entry.rs.
        if let Some(thread_root_event_id) = self.thread_root_event_id.clone() {
            let area = self.view.area();
            match event.hits(cx, area) {
                Hit::FingerDown(_) => {
                    cx.set_key_focus(area);
                }
                Hit::FingerUp(fe) if fe.is_over && fe.is_primary_hit() && fe.was_tap() => {
                    log!("ThreadsPaneEntry: tap detected, emitting OpenThread({})", thread_root_event_id);
                    cx.widget_action(
                        self.widget_uid(),
                        ThreadsPaneAction::OpenThread(thread_root_event_id),
                    );
                }
                _ => {}
            }
        }

        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl ThreadsPaneEntry {
    fn set_entry(&mut self, cx: &mut Cx, entry: &ThreadsPaneEntryInfo) {
        self.thread_root_event_id = Some(entry.thread_root_event_id.clone());
        self.html_or_plaintext(cx, ids!(title)).show_html(cx, &entry.title);
        self.label(cx, ids!(time)).set_text(cx, &entry.time);
        self.label(cx, ids!(subtitle)).set_text(cx, &entry.subtitle);
        self.html_or_plaintext(cx, ids!(preview)).show_html(cx, &entry.preview);
    }
}

impl ThreadsPaneEntryRef {
    fn set_entry(&self, cx: &mut Cx, entry: &ThreadsPaneEntryInfo) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_entry(cx, entry);
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct ThreadsButton {
    #[deref] view: View,
}

impl Widget for ThreadsButton {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let button_area = self.button(cx, ids!(inner_button)).area();
        match event.hits(cx, button_area) {
            Hit::FingerHoverIn(_) | Hit::FingerLongPress(_) => {
                cx.widget_action(
                    self.widget_uid(),
                    TooltipAction::HoverIn {
                        text: String::from("Threads"),
                        widget_rect: button_area.rect(cx),
                        options: CalloutTooltipOptions {
                            position: TooltipPosition::Left,
                            ..Default::default()
                        },
                    },
                );
            }
            Hit::FingerHoverOut(_) => {
                cx.widget_action(self.widget_uid(), TooltipAction::HoverOut);
            }
            _ => {}
        }

        self.view.handle_event(cx, event, scope);

        if let Event::Actions(actions) = event {
            if self.button(cx, ids!(inner_button)).clicked(actions) {
                cx.widget_action(self.widget_uid(), ThreadsButtonAction::OpenRequested);
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

#[derive(Script, ScriptHook, Widget, Animator)]
pub struct ThreadsSlidingPane {
    #[source] source: ScriptObjectRef,
    #[deref] view: View,
    #[apply_default] animator: Animator,
    #[live] slide: f32,

    #[rust] info: Option<ThreadsPaneInfo>,
    #[rust] is_animating_out: bool,
}

impl Widget for ThreadsSlidingPane {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        if !self.visible { return; }

        let animator_action = self.animator_handle_event(cx, event);
        if animator_action.must_redraw() {
            self.redraw(cx);
        }

        if self.is_animating_out && !self.animator.is_track_animating(id!(panel)) {
            self.visible = false;
            self.is_animating_out = false;
            cx.revert_key_focus();
            self.view(cx, ids!(bg_view)).set_visible(cx, false);
            self.redraw(cx);
            return;
        }

        let area = self.view.area();
        let close_pane = {
            matches!(
                event,
                Event::Actions(actions) if self.button(cx, ids!(close_button)).clicked(actions)
            )
            || event.back_pressed()
            || match event.hits_with_capture_overload(cx, area, true) {
                Hit::KeyUp(key) => key.key_code == KeyCode::Escape,
                Hit::FingerDown(_fde) => {
                    cx.set_key_focus(area);
                    false
                }
                Hit::FingerUp(fue) if fue.is_over => {
                    fue.mouse_button().is_some_and(|b| b.is_back())
                    || !self.view(cx, ids!(main_content)).area().rect(cx).contains(fue.abs)
                }
                _ => false,
            }
        };
        if close_pane {
            cx.widget_action(self.widget_uid(), ThreadsPaneAction::CloseRequested);
        }

        if let Event::Actions(actions) = event {
            let threads_list = self.portal_list(cx, ids!(threads_list));
            if threads_list.scrolled(actions)
                && threads_list.first_id() == 0
                && threads_list.scroll_position() >= -0.5
            {
                cx.widget_action(
                    self.widget_uid(),
                    ThreadsPaneAction::LoadMoreRequested,
                );
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let Some(info) = self.info.as_ref() else {
            self.visible = false;
            return self.view.draw_walk(cx, scope, walk);
        };

        let container_width = self.view.area().rect(cx).size.x as f32;
        let panel_width = if container_width > 1.0 && container_width < ROOM_INFO_PANE_MOBILE_BREAKPOINT {
            container_width
        } else {
            ROOM_INFO_PANE_DESKTOP_WIDTH
        };
        let right_margin = -(self.slide * panel_width);
        let mut main_content = self.view(cx, ids!(main_content));
        script_apply_eval!(cx, main_content, {
            width: #(panel_width)
            margin.right: #(right_margin)
        });
        let bg_alpha = (1.0 - self.slide) * 0.733;
        let bg_color = vec4(0.0, 0.0, 0.0, bg_alpha);
        let mut bg_view = self.view(cx, ids!(bg_view));
        script_apply_eval!(cx, bg_view, {
            draw_bg +: { color: #(bg_color) }
        });

        self.label(cx, ids!(room_name)).set_text(cx, &info.room_name);
        self.label(cx, ids!(loading_label)).set_text(cx, &info.loading_text);
        self.view(cx, ids!(loading_indicator)).set_visible(cx, info.show_loading);
        self.label(cx, ids!(empty_state)).set_text(cx, &info.status_text);
        self.view(cx, ids!(empty_state)).set_visible(cx, !info.show_entries && !info.show_loading);
        self.view(cx, ids!(threads_list)).set_visible(cx, info.show_entries);

        while let Some(widget) = self.view.draw_walk(cx, scope, walk).step() {
            let portal_list_ref = widget.as_portal_list();
            let Some(mut list) = portal_list_ref.borrow_mut() else { continue };

            list.set_item_range(cx, 0, info.entries.len());
            while let Some(item_id) = list.next_visible_item(cx) {
                let Some(entry) = info.entries.get(item_id) else { continue };
                let item = list.item(cx, item_id, id!(ThreadEntry));
                item.as_threads_pane_entry().set_entry(cx, entry);
                item.draw_all(cx, &mut Scope::empty());
            }
        }
        DrawStep::done()
    }
}

impl ThreadsSlidingPane {
    pub fn is_currently_shown(&self, _cx: &mut Cx) -> bool {
        self.visible
    }

    pub(super) fn set_info(&mut self, _cx: &mut Cx, info: ThreadsPaneInfo) {
        self.info = Some(info);
    }

    pub fn show(&mut self, cx: &mut Cx) {
        self.visible = true;
        self.is_animating_out = false;
        cx.set_key_focus(self.view.area());
        self.animator_play(cx, ids!(panel.show));
        self.view(cx, ids!(bg_view)).set_visible(cx, true);
        self.view.button(cx, ids!(close_button)).reset_hover(cx);
        self.redraw(cx);
    }

    pub fn hide(&mut self, cx: &mut Cx) {
        if !self.visible {
            return;
        }
        self.is_animating_out = true;
        self.animator_play(cx, ids!(panel.hide));
        self.redraw(cx);
    }
}

impl ThreadsSlidingPaneRef {
    pub fn is_currently_shown(&self, cx: &mut Cx) -> bool {
        let Some(inner) = self.borrow() else { return false };
        inner.is_currently_shown(cx)
    }

    pub(super) fn set_info(&self, cx: &mut Cx, info: ThreadsPaneInfo) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_info(cx, info);
    }

    pub fn show(&self, cx: &mut Cx) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show(cx);
    }

    pub fn hide(&self, cx: &mut Cx) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.hide(cx);
    }
}

pub(super) const ROOM_INFO_PANE_DESKTOP_WIDTH: f32 = 320.0;
pub(super) const ROOM_INFO_PANE_MOBILE_BREAKPOINT: f32 = 700.0;

impl RoomScreen {
    pub(super) fn show_threads_pane(&mut self, cx: &mut Cx) {
        self.hide_room_info_pane(cx);
        self.ensure_threads_state_for_current_room();
        if !self.threads_pane_state.initialized && !self.threads_pane_state.is_loading {
            self.request_more_threads(cx, false);
        }
        self.refresh_threads_pane(cx);
        self.threads_sliding_pane(cx, ids!(threads_sliding_pane)).show(cx);
        self.threads_button(cx, ids!(timeline.threads_button)).set_visible(cx, false);
        self.redraw(cx);
    }

    pub(super) fn refresh_threads_pane(&mut self, cx: &mut Cx) {
        let Some(room_name_id) = self.room_name_id.as_ref() else { return };
        self.threads_sliding_pane(cx, ids!(threads_sliding_pane)).set_info(
            cx,
            ThreadsPaneInfo {
                room_name: room_name_id.to_string(),
                entries: self.threads_pane_state.entries.iter()
                    .map(|entry| ThreadsPaneEntryInfo {
                        thread_root_event_id: entry.thread_root_event_id.clone(),
                        title: entry.title.clone(),
                        subtitle: match entry.reply_count {
                            1 => String::from("1 reply"),
                            n => format!("{n} replies"),
                        },
                        time: utils::relative_format(entry.timestamp)
                            .unwrap_or_else(|| String::from("")),
                        preview: entry.latest_reply_preview.clone().unwrap_or_else(|| String::from("Tap to open thread")),
                    })
                    .collect(),
                status_text: self.threads_pane_state.status_text.clone(),
                show_entries: !self.threads_pane_state.entries.is_empty(),
                loading_text: if self.threads_pane_state.entries.is_empty() {
                    String::from("Loading threads...")
                } else {
                    String::from("Loading more threads...")
                },
                show_loading: self.threads_pane_state.is_loading,
            },
        );
    }

    pub(super) fn hide_threads_pane(&mut self, cx: &mut Cx) {
        self.threads_sliding_pane(cx, ids!(threads_sliding_pane)).hide(cx);
        let show_threads_button = effective_is_desktop(cx);
        self.threads_button(cx, ids!(timeline.threads_button))
            .set_visible(cx, show_threads_button);
    }

    pub(super) fn ensure_threads_state_for_current_room(&mut self) {
        let Some(room_id) = self.room_id().cloned() else { return };
        if self.threads_pane_state.room_id.as_ref().is_some_and(|current| current == &room_id) {
            return;
        }
        self.threads_pane_state = ThreadsPaneState {
            room_id: Some(room_id),
            status_text: String::from("Loading threads..."),
            ..Default::default()
        };
    }

    pub(super) fn request_more_threads(&mut self, _cx: &mut Cx, load_more: bool) {
        self.ensure_threads_state_for_current_room();
        let Some(room_id) = self.threads_pane_state.room_id.clone() else { return };
        if self.threads_pane_state.is_loading {
            return;
        }
        let from = if load_more {
            let Some(from) = self.threads_pane_state.prev_batch_token.clone() else { return };
            Some(from)
        } else {
            None
        };
        self.threads_pane_state.is_loading = true;
        if !self.threads_pane_state.initialized {
            self.threads_pane_state.status_text = String::from("Loading threads...");
        }
        submit_async_request(MatrixRequest::ListRoomThreads {
            room_id,
            from,
        });
    }

    pub(super) fn on_threads_loaded(
        &mut self,
        cx: &mut Cx,
        _from: Option<&String>,
        threads: &[FetchedRoomThread],
        prev_batch_token: Option<String>,
    ) {
        self.threads_pane_state.is_loading = false;
        self.threads_pane_state.initialized = true;
        self.threads_pane_state.prev_batch_token = prev_batch_token;
        self.threads_pane_state.entries.extend_from_slice(threads);
        self.threads_pane_state.entries.sort_by_key(|entry| u64::from(entry.timestamp.0));
        self.threads_pane_state.entries.dedup_by(|a, b| a.thread_root_event_id == b.thread_root_event_id);
        self.threads_pane_state.status_text = if self.threads_pane_state.entries.is_empty() {
            String::from("No threads yet.")
        } else {
            String::new()
        };
        self.refresh_threads_pane(cx);
        self.redraw(cx);
    }

    pub(super) fn on_threads_failed(&mut self, cx: &mut Cx, error: &str) {
        self.threads_pane_state.is_loading = false;
        self.threads_pane_state.initialized = true;
        if self.threads_pane_state.entries.is_empty() {
            self.threads_pane_state.status_text = format!("Failed to load threads.\n\nError: {error}");
        } else {
            let error_display = error.to_string();
            let room_id_retry = self.threads_pane_state.room_id.clone();
            let from_retry = self.threads_pane_state.prev_batch_token.clone();
            enqueue_notification(NotificationItem {
                kind: PopupKind::Error,
                title: Some("Load threads failed".into()),
                message: format!("Failed to load more threads.\n\nError: {error}").into(),
                actions: vec![
                    NotificationAction::new("Retry", NotifActionStyle::Primary, move |_cx| {
                        if let Some(room_id) = room_id_retry.clone() {
                            submit_async_request(MatrixRequest::ListRoomThreads {
                                room_id,
                                from: from_retry.clone(),
                            });
                        }
                    }),
                    NotificationAction::new("Copy details", NotifActionStyle::Neutral, move |cx| {
                        cx.copy_to_clipboard(&error_display);
                    }),
                ],
                auto_dismissal_duration: Some(5.0),
                ..Default::default()
            });
        }
        self.refresh_threads_pane(cx);
        self.redraw(cx);
    }
}
