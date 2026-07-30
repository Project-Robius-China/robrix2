//! Picture-in-Picture (PiP) overlay for VoIP calls
//!
//! This module provides a floating PiP window that appears when the user switches
//! to a different room tab during an active VoIP call. It shows participant info,
//! call status, and control buttons.

use makepad_widgets::*;
use makepad_widgets::video::VideoCameraPreviewMode;
use matrix_sdk::ruma::OwnedRoomId;
use super::{VoipGlobalState, VoipAction, CameraChoice};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.PipVoipOverlay = #(PipVoipOverlay::register_widget(vm)) {
        width: Fit
        height: Fit
        flow: Overlay
        visible: false

        // Position in top-right corner
        margin: Inset { top: 60, right: 16, left: 0, bottom: 0 }
        align: Align { x: 1.0, y: 0.0 }

        // Main container with click area
        pip_container := RoundedView {
            width: 280
            height: Fit
            padding: 0
            draw_bg.color: #2a2a4a
            draw_bg.radius: 12.0
            flow: Down
            spacing: 0

            // Video preview area
            pip_video_container := View {
                width: Fill
                height: 160
                flow: Overlay

                // Avatar placeholder (shown when camera is off)
                pip_avatar_view := View {
                    width: Fill
                    height: Fill
                    align: Center
                    show_bg: true
                    draw_bg.color: #1a1a2e

                    RoundedView {
                        width: 60
                        height: 60
                        draw_bg.color: #a0d0a0
                        draw_bg.radius: 30.0
                        align: Center

                        pip_avatar_letter := Label {
                            text: "?"
                            draw_text.text_style.font_size: 24
                            draw_text.color: #2a6a2a
                        }
                    }
                }

                // Camera video (shown when camera is on)
                pip_video_host := View {
                    width: Fill
                    height: Fill
                    visible: false

                    pip_camera_video := Video {
                        width: Fill
                        height: Fill
                        autoplay: false
                        show_controls: false
                    }
                }

                // Back button overlay at top-left
                View {
                    width: Fill
                    height: Fill
                    align: Align { x: 0.0, y: 0.0 }
                    padding: 8

                    pip_back_button := RobrixIconButton {
                        width: 32
                        height: 32
                        padding: 6
                        draw_icon.svg: (ICON_JUMP)
                        icon_walk: Walk { width: 16, height: 16 }
                        draw_bg +: {
                            color: #1a1a3aCC
                            border_radius: 16.0
                        }
                        draw_icon +: {
                            color: #fff
                        }
                    }
                }

                // Name badge overlay at bottom
                View {
                    width: Fill
                    height: Fill
                    align: Align { x: 0.5, y: 1.0 }
                    padding: 8

                    RoundedView {
                        width: Fit
                        height: Fit
                        padding: Inset { left: 8, right: 10, top: 4, bottom: 4 }
                        draw_bg.color: #1a1a3a
                        draw_bg.radius: 10.0
                        flow: Right
                        spacing: 4
                        align: Center

                        participant_name := Label {
                            text: "User"
                            draw_text.text_style.font_size: 11
                            draw_text.color: #ddd
                        }

                        status_label := Label {
                            text: ""
                            draw_text.text_style.font_size: 10
                            draw_text.color: #aaa
                            margin: Inset { left: 4 }
                        }
                    }
                }
            }

            // Control buttons row
            controls_row := View {
                width: Fill
                height: Fit
                flow: Right
                spacing: 8
                align: Center
                padding: 10

                pip_mic_button := RobrixIconButton {
                    width: 36
                    height: 36
                    padding: 6
                    draw_icon.svg: (ICON_MICROPHONE)
                    icon_walk: Walk { width: 18, height: 18 }
                    draw_bg +: {
                        color: #3a3a5a
                        border_radius: 18.0
                    }
                    draw_icon +: {
                        color: #fff
                    }
                }

                pip_camera_button := RobrixIconButton {
                    width: 36
                    height: 36
                    padding: 6
                    draw_icon.svg: (ICON_VIDEO)
                    icon_walk: Walk { width: 18, height: 18 }
                    draw_bg +: {
                        color: #3a3a5a
                        border_radius: 18.0
                    }
                    draw_icon +: {
                        color: #fff
                    }
                }

                pip_screenshare_button := RobrixIconButton {
                    width: 36
                    height: 36
                    padding: 6
                    draw_icon.svg: (ICON_SQUARES)
                    icon_walk: Walk { width: 18, height: 18 }
                    draw_bg +: {
                        color: #3a3a5a
                        border_radius: 18.0
                    }
                    draw_icon +: {
                        color: #fff
                    }
                }

                pip_hangup_button := RobrixIconButton {
                    width: 36
                    height: 36
                    padding: 6
                    draw_icon.svg: (ICON_CLOSE)
                    icon_walk: Walk { width: 18, height: 18 }
                    draw_bg +: {
                        color: #e53935
                        border_radius: 18.0
                    }
                    draw_icon +: {
                        color: #fff
                    }
                }
            }
        }
    }
}

/// PiP overlay widget for VoIP calls
#[derive(Script, ScriptHook, Widget)]
pub struct PipVoipOverlay {
    #[deref]
    view: View,

    /// The room ID of the active call being displayed
    #[rust]
    room_id: Option<OwnedRoomId>,

    /// Whether the PiP is currently visible
    #[rust]
    is_visible: bool,

    /// Whether the camera is active in PiP
    #[rust]
    camera_active: bool,

    /// Stored camera choice for starting camera
    #[rust]
    camera_choice: Option<CameraChoice>,
}

impl Widget for PipVoipOverlay {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);

        // Handle video events
        match event {
            Event::VideoPlaybackPrepared(_) => {
                if self.is_visible && self.camera_active {
                    self.show_video(cx);
                }
            }
            Event::VideoTextureUpdated(_) => {
                if self.is_visible && self.camera_active {
                    self.show_video(cx);
                }
            }
            _ => {}
        }

        if !self.is_visible {
            return;
        }

        // Handle button clicks
        if let Event::Actions(actions) = event {
            self.handle_actions(cx, actions);
        }

        // Handle click anywhere on the PiP to return to VoIP tab
        if let Hit::FingerUp(fe) = event.hits(cx, self.view.area()) {
            if fe.was_tap() {
                // Check if the click was NOT on a button (buttons handle their own clicks)
                let back_area = self.view.button(cx, ids!(pip_back_button)).area();
                let mic_area = self.view.button(cx, ids!(pip_mic_button)).area();
                let cam_area = self.view.button(cx, ids!(pip_camera_button)).area();
                let share_area = self.view.button(cx, ids!(pip_screenshare_button)).area();
                let hangup_area = self.view.button(cx, ids!(pip_hangup_button)).area();

                let click_pos = fe.abs;
                let on_button = back_area.rect(cx).contains(click_pos)
                    || mic_area.rect(cx).contains(click_pos)
                    || cam_area.rect(cx).contains(click_pos)
                    || share_area.rect(cx).contains(click_pos)
                    || hangup_area.rect(cx).contains(click_pos);

                if !on_button {
                    if let Some(room_id) = self.room_id.clone() {
                        log!("PipVoipOverlay: Clicked on PiP, returning to VoIP tab");
                        cx.action(VoipAction::ReturnToVoipTab { room_id });
                    }
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        if self.is_visible {
            // Update the display from global state before drawing
            self.update_from_global_state(cx);
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

impl PipVoipOverlay {
    /// Show the PiP overlay for the given room
    pub fn show(&mut self, cx: &mut Cx, room_id: OwnedRoomId) {
        log!("PipVoipOverlay: Showing for room {}", room_id);
        self.room_id = Some(room_id);
        self.is_visible = true;
        self.view.set_visible(cx, true);
        self.update_from_global_state(cx);

        // Start camera if available and not muted
        if let Some(active_call) = VoipGlobalState::get_active_call(cx) {
            if !active_call.camera_muted {
                self.start_camera(cx);
            }
        }

        self.redraw(cx);
    }

    /// Hide the PiP overlay
    pub fn hide(&mut self, cx: &mut Cx) {
        log!("PipVoipOverlay: Hiding");
        self.is_visible = false;

        // Stop camera before hiding
        self.stop_camera(cx);

        self.view.set_visible(cx, false);
        self.redraw(cx);
    }

    /// Start the camera in PiP
    fn start_camera(&mut self, cx: &mut Cx) {
        // Get camera choice from global state
        let Some(choice) = VoipGlobalState::get_camera_choice(cx) else {
            log!("PipVoipOverlay: No camera choice available");
            return;
        };

        let video = self.view.video(cx, &[live_id!(pip_camera_video)]);

        if !video.is_unprepared() {
            log!("PipVoipOverlay: Camera already running or preparing");
            return;
        }

        log!("PipVoipOverlay: Starting camera: {} ({}x{} {:?})",
            choice.name, choice.width, choice.height, choice.pixel_format);

        self.camera_choice = Some(choice.clone());
        self.camera_active = true;

        video.set_camera_preview_mode(cx, VideoCameraPreviewMode::Native);
        video.set_source_camera(cx, choice.input_id, choice.format_id);
        video.begin_playback(cx);
    }

    /// Stop the camera in PiP
    fn stop_camera(&mut self, cx: &mut Cx) {
        if !self.camera_active {
            return;
        }

        log!("PipVoipOverlay: Stopping camera");
        let video = self.view.video(cx, &[live_id!(pip_camera_video)]);
        if !video.is_unprepared() && !video.is_cleaning_up() {
            video.stop_and_cleanup_resources(cx);
        }

        self.view.view(cx, ids!(pip_video_host)).set_visible(cx, false);
        self.view.view(cx, ids!(pip_avatar_view)).set_visible(cx, true);
        self.camera_active = false;
    }

    /// Show video view
    fn show_video(&mut self, cx: &mut Cx) {
        self.view.view(cx, ids!(pip_video_host)).set_visible(cx, true);
        self.view.view(cx, ids!(pip_avatar_view)).set_visible(cx, false);
    }

    /// Update the display from global VoIP state
    fn update_from_global_state(&mut self, cx: &mut Cx) {
        if let Some(active_call) = VoipGlobalState::get_active_call(cx) {
            // Update participant info
            self.view.label(cx, ids!(pip_avatar_letter))
                .set_text(cx, &active_call.local_participant.avatar_letter);
            self.view.label(cx, ids!(participant_name))
                .set_text(cx, &active_call.local_participant.display_name);
            self.view.label(cx, ids!(status_label))
                .set_text(cx, &format!(" - {}", active_call.status_text));

            // Update button styles based on state
            let mut mic_btn = self.view.button(cx, ids!(pip_mic_button));
            let mut cam_btn = self.view.button(cx, ids!(pip_camera_button));
            let mut share_btn = self.view.button(cx, ids!(pip_screenshare_button));

            // Mic button - red when muted
            if active_call.mic_muted {
                script_apply_eval!(cx, mic_btn, {
                    draw_bg +: { color: #e53935 }
                });
            } else {
                script_apply_eval!(cx, mic_btn, {
                    draw_bg +: { color: #3a3a5a }
                });
            }

            // Camera button - red when off
            if active_call.camera_muted {
                script_apply_eval!(cx, cam_btn, {
                    draw_bg +: { color: #e53935 }
                });
                // Hide video, show avatar when camera is muted
                if self.camera_active {
                    self.stop_camera(cx);
                }
            } else {
                script_apply_eval!(cx, cam_btn, {
                    draw_bg +: { color: #3a3a5a }
                });
                // Start camera if not already active
                if !self.camera_active && self.is_visible {
                    self.start_camera(cx);
                }
            }

            // Screen share button - green when sharing
            if active_call.screen_sharing {
                script_apply_eval!(cx, share_btn, {
                    draw_bg +: { color: #4CAF50 }
                });
            } else {
                script_apply_eval!(cx, share_btn, {
                    draw_bg +: { color: #3a3a5a }
                });
            }

            // If the call ended, hide the PiP
            if !active_call.in_call {
                self.hide(cx);
            }
        } else {
            // No active call, hide PiP
            self.hide(cx);
        }
    }

    /// Handle UI actions (button clicks)
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        let Some(room_id) = self.room_id.clone() else {
            return;
        };

        // Back button - return to VoIP tab
        if self.view.button(cx, ids!(pip_back_button)).clicked(actions) {
            log!("PipVoipOverlay: Back button clicked, returning to VoIP tab");
            cx.action(VoipAction::ReturnToVoipTab { room_id: room_id.clone() });
            return;
        }

        // Mic button
        if self.view.button(cx, ids!(pip_mic_button)).clicked(actions) {
            log!("PipVoipOverlay: Mic button clicked");
            cx.action(VoipAction::PipMicToggle { room_id: room_id.clone() });
        }

        // Camera button
        if self.view.button(cx, ids!(pip_camera_button)).clicked(actions) {
            log!("PipVoipOverlay: Camera button clicked");
            cx.action(VoipAction::PipCameraToggle { room_id: room_id.clone() });
        }

        // Screen share button
        if self.view.button(cx, ids!(pip_screenshare_button)).clicked(actions) {
            log!("PipVoipOverlay: Screen share button clicked");
            cx.action(VoipAction::PipScreenShareToggle { room_id: room_id.clone() });
        }

        // Hangup button
        if self.view.button(cx, ids!(pip_hangup_button)).clicked(actions) {
            log!("PipVoipOverlay: Hangup button clicked");
            cx.action(VoipAction::PipHangup { room_id: room_id.clone() });
            // Hide PiP after hangup
            self.hide(cx);
        }
    }
}

impl PipVoipOverlayRef {
    /// Show the PiP overlay for the given room
    pub fn show(&self, cx: &mut Cx, room_id: OwnedRoomId) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.show(cx, room_id);
        }
    }

    /// Hide the PiP overlay
    pub fn hide(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.hide(cx);
        }
    }

    /// Check if the PiP is currently visible
    pub fn is_visible(&self) -> bool {
        if let Some(inner) = self.borrow() {
            inner.is_visible
        } else {
            false
        }
    }

    /// Get the room ID of the active call being displayed
    pub fn get_room_id(&self) -> Option<OwnedRoomId> {
        if let Some(inner) = self.borrow() {
            inner.room_id.clone()
        } else {
            None
        }
    }
}
