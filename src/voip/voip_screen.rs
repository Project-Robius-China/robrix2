//! VoIP Screen - Voice/Video call interface
//!
//! This module provides a VoIP screen widget that can be used for voice/video calls.
//! It uses the Matrix client from the room screen for authentication and signaling.

use makepad_widgets::*;
use makepad_widgets::makepad_platform::permission::{Permission, PermissionStatus};
use matrix_sdk::Client;
use ruma::OwnedRoomId;
use tokio::sync::mpsc;

use crate::sliding_sync::get_client;
use super::VoipGlobalState;

use super::call_state::{Call, CallType, ConnectionState};
use super::camera::{CameraChoice, CameraManager};
use super::livekit_client::{LiveKitClient, LiveKitCommand, LiveKitMessage};
use super::speaking::SpeakingDetector;
use super::participants_list::{Participant, ParticipantsListWidgetExt};

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // ParticipantsList widget definition
    let ParticipantsListBase = #(super::participants_list::ParticipantsList::register_widget(vm))
    mod.widgets.VoipParticipantsList = set_type_default() do ParticipantsListBase {
        width: Fill
        height: Fill

        list := FlatList {
            width: Fill
            height: Fill
            flow: Down
            grab_key_focus: true

            ParticipantItem := RoundedView {
                width: Fill
                height: Fit
                padding: 8
                margin: Inset{bottom: 4}
                draw_bg.color: #3a3a5a
                draw_bg.radius: 6.0
                flow: Right
                spacing: 8
                align: Align{y: 0.5}

                avatar := RoundedView {
                    width: 32
                    height: 32
                    draw_bg.color: #a0d0a0
                    draw_bg.radius: 16.0
                    align: Center

                    avatar_letter := Label {
                        text: "?"
                        draw_text.text_style.font_size: 14
                        draw_text.color: #2a6a2a
                    }
                }

                info_container := View {
                    width: Fill
                    height: Fit
                    flow: Down
                    spacing: 2

                    name_label := Label {
                        text: "Participant"
                        draw_text.text_style.font_size: 12
                        draw_text.color: #fff
                    }

                    status_label := Label {
                        text: ""
                        draw_text.text_style.font_size: 10
                        draw_text.color: #888
                    }
                }

                mute_icon := Label {
                    text: ""
                    draw_text.text_style.font_size: 12
                    draw_text.color: #888
                }
            }
        }
    }

    // VoIP Screen widget
    mod.widgets.VoipScreen = #(VoipScreen::register_widget(vm)) {
        width: Fill
        height: Fill
        flow: Overlay

        // Main call view
        call_view := View {
            width: Fill
            height: Fill
            flow: Down
            show_bg: true
            draw_bg.color: #1a1a2e
            visible: false

            // Call header
            call_header := View {
                width: Fill
                height: Fit
                padding: 16
                flow: Right
                spacing: 12

                View {
                    width: Fit
                    height: Fit
                    flow: Down
                    spacing: 4

                    room_name := Label {
                        text: "Call Room"
                        draw_text.text_style.font_size: 18
                        draw_text.color: #fff
                    }

                    call_status := Label {
                        text: "Not connected"
                        draw_text.text_style.font_size: 12
                        draw_text.color: #888
                    }
                }

                View { width: Fill height: 1 }

                call_duration := Label {
                    text: ""
                    draw_text.text_style.font_size: 14
                    draw_text.color: #888
                }

                participant_count := Label {
                    text: "0 participants"
                    draw_text.text_style.font_size: 12
                    draw_text.color: #888
                }
            }

            // Participants grid
            participants_grid := View {
                width: Fill
                height: Fill
                flow: Right
                spacing: 16
                padding: 16
                align: Center

                // Local user card wrapper
                local_card_wrapper := View {
                    width: Fit
                    height: Fit
                    flow: Overlay

                    // Speaking indicator border
                    local_speaking_border := RoundedView {
                        width: 286
                        height: 216
                        draw_bg.color: #4CAF50
                        draw_bg.radius: 15.0
                        visible: false
                    }

                    // Main card
                    local_participant_card := RoundedView {
                        width: 280
                        height: 210
                        margin: 3
                        draw_bg.color: #e8e8e8
                        draw_bg.radius: 12.0
                        flow: Overlay

                        // Video container
                        local_video_container := View {
                            width: Fill
                            height: Fill
                            flow: Overlay

                            // Avatar placeholder
                            local_avatar_view := View {
                                width: Fill
                                height: Fill
                                align: Center

                                RoundedView {
                                    width: 80
                                    height: 80
                                    draw_bg.color: #a0d0a0
                                    draw_bg.radius: 40.0
                                    align: Center

                                    local_avatar_letter := Label {
                                        text: "Y"
                                        draw_text.text_style.font_size: 32
                                        draw_text.color: #2a6a2a
                                    }
                                }
                            }

                            // Camera video
                            local_video_host := View {
                                width: Fill
                                height: Fill
                                visible: false

                                local_camera_video := Video {
                                    width: Fill
                                    height: Fill
                                    autoplay: false
                                    show_controls: false
                                }
                            }
                        }

                        // Name badge
                        View {
                            width: Fill
                            height: Fit
                            align: Align{x: 0.0 y: 1.0}
                            padding: 8

                            RoundedView {
                                width: Fit
                                height: Fit
                                padding: Inset{left: 8 right: 8 top: 4 bottom: 4}
                                draw_bg.color: #fff
                                draw_bg.radius: 4.0
                                flow: Right
                                spacing: 4

                                local_mute_icon := Label {
                                    text: ""
                                    draw_text.text_style.font_size: 12
                                    draw_text.color: #666
                                }

                                local_name_label := Label {
                                    text: "You"
                                    draw_text.text_style.font_size: 12
                                    draw_text.color: #333
                                }
                            }
                        }
                    }
                }

                // Remote participant card
                remote_participant_card := RoundedView {
                    width: 280
                    height: 210
                    draw_bg.color: #e8e8e8
                    draw_bg.radius: 12.0
                    flow: Overlay
                    visible: false

                    View {
                        width: Fill
                        height: Fill
                        align: Center

                        RoundedView {
                            width: 80
                            height: 80
                            draw_bg.color: #d0a0d0
                            draw_bg.radius: 40.0
                            align: Center

                            remote_avatar_letter := Label {
                                text: "R"
                                draw_text.text_style.font_size: 32
                                draw_text.color: #6a2a6a
                            }
                        }
                    }

                    View {
                        width: Fill
                        height: Fit
                        align: Align{x: 0.0 y: 1.0}
                        padding: 8

                        RoundedView {
                            width: Fit
                            height: Fit
                            padding: Inset{left: 8 right: 8 top: 4 bottom: 4}
                            draw_bg.color: #fff
                            draw_bg.radius: 4.0

                            remote_name_label := Label {
                                text: "Remote"
                                draw_text.text_style.font_size: 12
                                draw_text.color: #333
                            }
                        }
                    }
                }
            }

            // Call controls
            call_controls := View {
                width: Fill
                height: Fit
                padding: Inset{bottom: 20 top: 10}
                align: Center

                RoundedView {
                    width: Fit
                    height: Fit
                    padding: 12
                    draw_bg.color: #2a2a4a
                    draw_bg.radius: 24.0
                    flow: Right
                    spacing: 8

                    mic_button := Button { text: "Mic" width: 60 }
                    camera_button := Button { text: "Cam" width: 60 }
                    screenshare_button := Button { text: "Share" width: 60 }
                    participants_button := Button { text: "Users" width: 60 }
                    hangup_button := Button { text: "End" width: 60 }
                }
            }
        }

        // Participants sidebar
        participants_sidebar := View {
            width: 200
            height: Fill
            margin: Inset{top: 60 bottom: 80}
            padding: 12
            show_bg: true
            draw_bg.color: #2a2a4a
            flow: Down
            spacing: 8
            visible: false
            align: Align{x: 1.0 y: 0.0}

            Label {
                text: "Participants"
                draw_text.text_style.font_size: 14
                draw_text.color: #fff
            }

            participants_list := mod.widgets.VoipParticipantsList {}
        }

        // Lobby view
        lobby_view := View {
            width: Fill
            height: Fill
            flow: Down
            spacing: 20
            padding: 40
            align: Center
            show_bg: true
            draw_bg.color: #1a1a2e
            visible: true

            Label {
                text: "Join Call"
                draw_text.text_style.font_size: 24
                draw_text.color: #fff
            }

            lobby_camera_container := View {
                width: 320
                height: 240

                lobby_camera_placeholder := RoundedView {
                    width: Fill
                    height: Fill
                    draw_bg.color: #2a2a4a
                    draw_bg.radius: 12.0
                    align: Center

                    Label {
                        text: "Camera Preview"
                        draw_text.text_style.font_size: 14
                        draw_text.color: #666
                    }
                }

                lobby_video_host := View {
                    width: Fill
                    height: Fill
                    visible: false

                    lobby_camera_video := Video {
                        width: Fill
                        height: Fill
                        autoplay: false
                        show_controls: false
                    }
                }
            }

            View {
                width: Fit
                height: Fit
                flow: Down
                spacing: 12
                align: Center

                View {
                    width: Fit
                    height: Fit
                    flow: Right
                    spacing: 8
                    align: Center

                    Label {
                        text: "Microphone:"
                        draw_text.text_style.font_size: 12
                        draw_text.color: #888
                    }

                    lobby_mic_button := Button {
                        text: "Mic On"
                        width: 80
                        draw_text.color: #333
                    }
                }

                View {
                    width: Fit
                    height: Fit
                    flow: Right
                    spacing: 8
                    align: Center

                    Label {
                        text: "Camera:"
                        draw_text.text_style.font_size: 12
                        draw_text.color: #888
                    }

                    lobby_camera_button := Button {
                        text: "Cam On"
                        width: 80
                        draw_text.color: #333
                    }
                }
            }

            View {
                width: Fit
                height: Fit
                flow: Right
                spacing: 12

                video_call_button := Button {
                    text: "Video Call"
                    width: 120
                    draw_text.color: #333
                }

                voice_call_button := Button {
                    text: "Voice Call"
                    width: 120
                    draw_text.color: #333
                }
            }

            lobby_status := Label {
                text: ""
                draw_text.text_style.font_size: 12
                draw_text.color: #888
            }
        }

        // Debug panel
        debug_panel := View {
            width: 300
            height: 200
            margin: Inset{right: 10 bottom: 10}
            padding: 10
            align: Align{x: 1.0 y: 1.0}
            show_bg: true
            draw_bg.color: #000000
            visible: false

            message_log := Label {
                width: Fill
                height: Fill
                text: ""
                draw_text.text_style.font_size: 9
                draw_text.color: #0f0
            }
        }
    }
}

/// VoIP Screen widget for voice/video calls
#[derive(Script, ScriptHook, Widget)]
pub struct VoipScreen {
    #[deref]
    view: View,

    // Call state
    #[rust] call: Call,
    #[rust] in_lobby: bool,
    #[rust] lobby_mic_enabled: bool,
    #[rust] lobby_camera_enabled: bool,
    #[rust] show_participants: bool,
    #[rust] show_debug: bool,

    // LiveKit client
    #[rust] livekit_client: Option<LiveKitClient>,
    #[rust] livekit_rx: Option<mpsc::UnboundedReceiver<LiveKitMessage>>,

    // Call timing
    #[rust] call_start_time: Option<f64>,

    // Matrix auth (from room screen client)
    #[rust] room_id: Option<OwnedRoomId>,

    // Camera
    #[rust] camera_permission: Option<PermissionStatus>,
    #[rust] camera_choice: Option<CameraChoice>,
    #[rust] camera_active: bool,

    // Speaking detection
    #[rust] speaking_detector: SpeakingDetector,
    #[rust] speaking_check_timer: Timer,

    // Video publish timer
    #[rust] video_publish_timer: Timer,

    // Flag to start call camera after lobby camera releases
    #[rust] pending_call_camera_start: bool,

    // Participant counter
    #[rust] participant_counter: usize,
}

impl Widget for VoipScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        match event {
            Event::PermissionResult(result) => {
                if result.permission == Permission::Camera {
                    log!("VoipScreen: Camera permission result: {:?}", result.status);
                    // Sync from global state (App already updated it)
                    self.camera_permission = VoipGlobalState::get_camera_permission(cx);
                    self.try_start_camera(cx);
                }
            }
            Event::VideoInputs(ev) => {
                log!("VoipScreen: VideoInputs event received with {} cameras", ev.descs.len());
                // Sync from global state (App already updated it)
                self.camera_choice = VoipGlobalState::get_camera_choice(cx);
                if let Some(ref choice) = self.camera_choice {
                    log!("VoipScreen: Got camera from global: {} ({}x{} {:?})",
                        choice.name, choice.width, choice.height, choice.pixel_format);
                }
                self.try_start_camera(cx);
            }
            Event::AudioDevices(ev) => {
                self.speaking_detector.handle_audio_devices(cx, ev);
            }
            Event::VideoPlaybackPrepared(ev) => {
                log!("VideoPlaybackPrepared: {:?}", ev.video_id);
                self.handle_video_prepared(cx);
            }
            Event::VideoTextureUpdated(ev) => {
                log!("VideoTextureUpdated: {:?}", ev.video_id);
                self.handle_video_texture_updated(cx);
            }
            Event::VideoPlaybackResourcesReleased(_) => {
                self.handle_video_resources_released(cx);
            }
            Event::Actions(actions) => {
                self.handle_actions(cx, actions);
            }
            _ => {
                if self.speaking_check_timer.is_event(event).is_some() {
                    self.check_speaking_state(cx);
                }
                if self.video_publish_timer.is_event(event).is_some() {
                    // Video publishing handled here if needed
                }
            }
        }

        // Poll LiveKit messages
        if self.poll_livekit_messages(cx) {
            self.update_ui(cx);
        }

        self.view.handle_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl VoipScreen {
    /// Initialize the VoIP screen
    pub fn initialize(&mut self, cx: &mut Cx) {
        self.in_lobby = true;
        self.lobby_mic_enabled = true;
        self.lobby_camera_enabled = true;
        self.call = Call::default();
        self.speaking_detector = SpeakingDetector::new();

        log!("VoipScreen initialized");

        // Initialize LiveKit client
        let mut client = LiveKitClient::new();
        let rx = client.start();
        self.livekit_client = Some(client);
        self.livekit_rx = Some(rx);

        // Timer for speaking detection
        self.speaking_check_timer = cx.start_interval(0.1);

        // Timer for video frames (~30fps)
        self.video_publish_timer = cx.start_interval(1.0 / 30.0);

        // Read camera permission and choice from global state (captured at app startup)
        self.camera_permission = VoipGlobalState::get_camera_permission(cx);
        self.camera_choice = VoipGlobalState::get_camera_choice(cx);

        log!("VoipScreen: Read from global state - permission={:?}, choice={:?}",
            self.camera_permission, self.camera_choice.as_ref().map(|c| &c.name));

        // Try to start camera if we already have permission and camera choice
        self.try_start_camera(cx);

        // Set default room
        let room_id: OwnedRoomId = "!rTeTgZzSYKoeJEVosH:matrix.org".try_into().unwrap();
        self.set_room(cx, room_id);

        self.update_ui(cx);
    }

    /// Set the room for this VoIP call (uses Matrix client from room screen)
    pub fn set_room(&mut self, cx: &mut Cx, room_id: OwnedRoomId) {
        self.room_id = Some(room_id.clone());

        // Get room name from client
        if let Some(client) = get_client() {
            if let Some(room) = client.get_room(&room_id) {
                let room_name = room.name().unwrap_or_else(|| room_id.to_string());
                self.view.label(cx, ids!(room_name)).set_text(cx, &room_name);
            }
        }

        self.update_ui(cx);
    }

    /// Get the Matrix client from the sliding_sync module
    #[allow(dead_code)]
    fn get_matrix_client(&self) -> Option<Client> {
        get_client()
    }

    /// Start a call of the given type
    fn start_call(&mut self, cx: &mut Cx, call_type: CallType) {
        self.call.call_type = call_type;
        self.call.local_audio_muted = !self.lobby_mic_enabled;
        self.call.local_video_muted = !self.lobby_camera_enabled;
        self.call.connection_state = ConnectionState::Connecting;

        log!("Starting {:?} call...", call_type);

        // In a full implementation, this would send call member state via Matrix
        // For now, we simulate connection
        self.call.connection_state = ConnectionState::Connected;
        self.in_lobby = false;
        self.call_start_time = Some(Cx::time_now());

        // Stop lobby camera
        CameraManager::stop_lobby_camera(&self.view, cx);
        self.pending_call_camera_start = true;
        self.camera_active = false;

        self.update_ui(cx);
    }

    /// Poll for LiveKit messages
    fn poll_livekit_messages(&mut self, _cx: &mut Cx) -> bool {
        let messages: Vec<LiveKitMessage> = if let Some(rx) = &mut self.livekit_rx {
            let mut msgs = Vec::new();
            while let Ok(msg) = rx.try_recv() {
                msgs.push(msg);
            }
            msgs
        } else {
            Vec::new()
        };

        let mut needs_update = false;
        for msg in messages {
            match msg {
                LiveKitMessage::Connected => {
                    self.call.connection_state = ConnectionState::Connected;
                    self.in_lobby = false;
                    self.call_start_time = Some(Cx::time_now());
                    log!("LiveKit connected");
                    needs_update = true;
                }
                LiveKitMessage::Disconnected => {
                    self.call.connection_state = ConnectionState::Disconnected;
                    log!("LiveKit disconnected");
                    needs_update = true;
                }
                LiveKitMessage::ParticipantJoined(p) => {
                    self.call.participants.insert(p.user_id.clone(), p);
                    log!("Participant joined");
                    needs_update = true;
                }
                LiveKitMessage::ParticipantLeft(id) => {
                    self.call.participants.remove(&id);
                    log!("Participant left");
                    needs_update = true;
                }
                LiveKitMessage::Error(e) => {
                    log!("LiveKit error: {}", e);
                    needs_update = true;
                }
            }
        }

        needs_update
    }

    /// Toggle microphone
    fn toggle_microphone(&mut self) {
        self.call.local_audio_muted = !self.call.local_audio_muted;
        if let Some(client) = &self.livekit_client {
            client.set_microphone_muted(self.call.local_audio_muted);
        }
        log!("Microphone {}", if self.call.local_audio_muted { "muted" } else { "unmuted" });
    }

    /// Toggle camera
    fn toggle_camera(&mut self) {
        self.call.local_video_muted = !self.call.local_video_muted;
        if let Some(client) = &self.livekit_client {
            client.set_camera_muted(self.call.local_video_muted);
        }
        log!("Camera {}", if self.call.local_video_muted { "off" } else { "on" });
    }

    /// Toggle screen sharing
    fn toggle_screenshare(&mut self) {
        self.call.is_screen_sharing = !self.call.is_screen_sharing;
        if let Some(client) = &self.livekit_client {
            if self.call.is_screen_sharing {
                client.send_command(LiveKitCommand::StartScreenShare);
            } else {
                client.send_command(LiveKitCommand::StopScreenShare);
            }
        }
        log!("Screen sharing {}", if self.call.is_screen_sharing { "started" } else { "stopped" });
    }

    /// Hangup the call
    fn hangup(&mut self, cx: &mut Cx) {
        self.call.connection_state = ConnectionState::Disconnecting;
        if let Some(client) = &self.livekit_client {
            client.disconnect();
        }
        CameraManager::stop_call_camera(&self.view, cx);
        self.camera_active = false;

        log!("Ending call...");

        // Reset state
        self.call.connection_state = ConnectionState::Disconnected;
        self.in_lobby = true;
        self.call_start_time = None;
        self.try_start_camera(cx);
    }

    /// Update UI to reflect current state
    fn update_ui(&mut self, cx: &mut Cx) {
        
        self.view.view(cx, ids!(lobby_view)).set_visible(cx, self.in_lobby);
        self.view.view(cx, ids!(call_view)).set_visible(cx, !self.in_lobby);

        let status = match self.call.connection_state {
            ConnectionState::Disconnected => "Not connected",
            ConnectionState::Connecting => "Connecting...",
            ConnectionState::Connected => "Connected",
            ConnectionState::Disconnecting => "Disconnecting...",
        };
        self.view.label(cx, ids!(call_status)).set_text(cx, status);

        let count = self.call.participants.len() + 1;
        self.view.label(cx, ids!(participant_count))
            .set_text(cx, &format!("{} participant{}", count, if count == 1 { "" } else { "s" }));

        let mic_text = if self.call.local_audio_muted { "Muted" } else { "Mic" };
        let cam_text = if self.call.local_video_muted { "Cam Off" } else { "Cam" };
        let screen_text = if self.call.is_screen_sharing { "Stop" } else { "Share" };

        self.view.button(cx, ids!(mic_button)).set_text(cx, mic_text);
        self.view.button(cx, ids!(camera_button)).set_text(cx, cam_text);
        self.view.button(cx, ids!(screenshare_button)).set_text(cx, screen_text);

        let mute_icon = if self.call.local_audio_muted { "M" } else { "" };
        self.view.label(cx, ids!(local_mute_icon)).set_text(cx, mute_icon);

        self.view.view(cx, ids!(participants_sidebar)).set_visible(cx, self.show_participants);
        self.view.view(cx, ids!(debug_panel)).set_visible(cx, self.show_debug);

        if let Some(start) = self.call_start_time {
            let elapsed = (Cx::time_now() - start) as u64;
            let mins = elapsed / 60;
            let secs = elapsed % 60;
            self.view.label(cx, ids!(call_duration))
                .set_text(cx, &format!("{:02}:{:02}", mins, secs));
        }

        // Update lobby buttons
        let mic_text = if self.lobby_mic_enabled { "Mic On" } else { "Mic Off" };
        let cam_text = if self.lobby_camera_enabled { "Cam On" } else { "Cam Off" };
        self.view.button(cx, ids!(lobby_mic_button)).set_text(cx, mic_text);
        self.view.button(cx, ids!(lobby_camera_button)).set_text(cx, cam_text);
    }

    /// Try to start camera
    fn try_start_camera(&mut self, cx: &mut Cx) {
        log!("try_start_camera: permission={:?}, choice={:?}",
            self.camera_permission, self.camera_choice.as_ref().map(|c| &c.name));

        if !matches!(self.camera_permission, Some(PermissionStatus::Granted)) {
            log!("Waiting for camera permission...");
            self.view.label(cx, ids!(lobby_status)).set_text(cx, "Waiting for camera permission...");
            return;
        }

        let Some(choice) = self.camera_choice.clone() else {
            log!("Waiting for camera device...");
            self.view.label(cx, ids!(lobby_status)).set_text(cx, "Waiting for camera device...");
            return;
        };

        if self.in_lobby {
            log!("Starting lobby camera: {}", choice.name);
            self.view.label(cx, ids!(lobby_status))
                .set_text(cx, &format!("Camera: {} ({}x{})", choice.name, choice.width, choice.height));
            if CameraManager::start_lobby_camera(&self.view, cx, &choice) {
                log!("Lobby camera started successfully");
                self.camera_active = true;
            } else {
                log!("Failed to start lobby camera (already running?)");
            }
        } else if CameraManager::start_call_camera(&self.view, cx, &choice) {
            log!("Call camera started successfully");
            self.camera_active = true;
        }
    }

    /// Check speaking state
    fn check_speaking_state(&mut self, cx: &mut Cx) {
        if self.in_lobby {
            if self.speaking_detector.is_speaking {
                self.speaking_detector.is_speaking = false;
                SpeakingDetector::update_indicator(&self.view, cx, false);
            }
            return;
        }

        if self.speaking_detector.check_speaking(self.call.local_audio_muted) {
            SpeakingDetector::update_indicator(&self.view, cx, self.speaking_detector.is_speaking);
        }
    }

    /// Handle camera permission result
    pub fn handle_camera_permission(&mut self, cx: &mut Cx, status: PermissionStatus) {
        self.camera_permission = Some(status);
        match status {
            PermissionStatus::Granted => {
                log!("Camera permission granted");
                self.try_start_camera(cx);
            }
            PermissionStatus::DeniedPermanent => {
                log!("Camera permission denied permanently");
                self.view.label(cx, ids!(lobby_status)).set_text(cx, "Camera permission denied");
            }
            _ => {
                log!("Camera permission: {:?}", status);
            }
        }
    }

    /// Handle video playback prepared
    fn handle_video_prepared(&mut self, cx: &mut Cx) {
        if self.camera_active {
            if self.in_lobby {
                CameraManager::show_lobby_video(&self.view, cx);
            } else {
                CameraManager::show_call_video(&self.view, cx);
            }
        }
    }

    /// Handle video texture updated
    fn handle_video_texture_updated(&mut self, cx: &mut Cx) {
        if self.camera_active {
            if self.in_lobby {
                CameraManager::show_lobby_video(&self.view, cx);
            } else {
                CameraManager::show_call_video(&self.view, cx);
            }
        }
    }

    /// Handle video resources released
    fn handle_video_resources_released(&mut self, cx: &mut Cx) {
        log!("Video resources released");
        if self.pending_call_camera_start {
            self.pending_call_camera_start = false;
            if let Some(choice) = self.camera_choice.clone() {
                if CameraManager::start_call_camera(&self.view, cx, &choice) {
                    self.camera_active = true;
                }
            }
        }
    }

    /// Handle UI actions
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        
        // Lobby buttons
        if self.view.button(cx, ids!(video_call_button)).clicked(actions) {
            log!("Video call button clicked");
            self.start_call(cx, CallType::Video);
        }
        if self.view.button(cx, ids!(voice_call_button)).clicked(actions) {
            self.start_call(cx, CallType::Voice);
        }
        if self.view.button(cx, ids!(lobby_mic_button)).clicked(actions) {
            self.lobby_mic_enabled = !self.lobby_mic_enabled;
            self.update_ui(cx);
        }
        if self.view.button(cx, ids!(lobby_camera_button)).clicked(actions) {
            self.lobby_camera_enabled = !self.lobby_camera_enabled;
            self.update_ui(cx);
        }

        // Call controls
        if self.view.button(cx, ids!(mic_button)).clicked(actions) {
            self.toggle_microphone();
            self.update_ui(cx);
        }
        if self.view.button(cx, ids!(camera_button)).clicked(actions) {
            self.toggle_camera();
            self.update_ui(cx);
        }
        if self.view.button(cx, ids!(screenshare_button)).clicked(actions) {
            self.toggle_screenshare();
            self.update_ui(cx);
        }
        if self.view.button(cx, ids!(participants_button)).clicked(actions) {
            self.show_participants = !self.show_participants;
            self.update_ui(cx);
        }
        if self.view.button(cx, ids!(hangup_button)).clicked(actions) {
            self.hangup(cx);
            self.update_ui(cx);
        }
    }

    /// Add a test participant
    pub fn add_participant(&mut self, cx: &mut Cx, name: &str) {
        self.participant_counter += 1;
        let letter = name.chars().next().unwrap_or('?').to_uppercase().to_string();
        let participant = Participant {
            id: format!("{}", self.participant_counter),
            name: name.to_string(),
            avatar_letter: letter,
            is_muted: false,
            is_speaking: false,
        };
        log!("Adding participant: {} (id={})", name, self.participant_counter);

        let list = self.view.participants_list(cx, ids!(participants_list));
        list.add_participant(cx, participant);
    }

    /// Remove a participant
    pub fn remove_participant(&mut self, cx: &mut Cx, id: &str) {
        log!("Removing participant with id={}", id);
        let list = self.view.participants_list(cx, ids!(participants_list));
        list.remove_participant(cx, id);
    }

    /// Clear all participants
    pub fn clear_participants(&mut self, cx: &mut Cx) {
        log!("Clearing all participants");
        let list = self.view.participants_list(cx, ids!(participants_list));
        list.clear(cx);
        self.participant_counter = 0;
    }
}

impl VoipScreenRef {
    /// Initialize the VoIP screen
    pub fn initialize(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.initialize(cx);
        }
    }

    /// Set the room for this VoIP call
    pub fn set_room(&self, cx: &mut Cx, room_id: OwnedRoomId) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.set_room(cx, room_id);
        }
    }

    /// Handle camera permission
    pub fn handle_camera_permission(&self, cx: &mut Cx, status: PermissionStatus) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.handle_camera_permission(cx, status);
        }
    }

    /// Add a participant
    pub fn add_participant(&self, cx: &mut Cx, name: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.add_participant(cx, name);
        }
    }

    /// Remove a participant
    pub fn remove_participant(&self, cx: &mut Cx, id: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.remove_participant(cx, id);
        }
    }

    /// Clear all participants
    pub fn clear_participants(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.clear_participants(cx);
        }
    }
}
