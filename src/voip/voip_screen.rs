//! VoIP Screen - Voice/Video call interface
//!
//! This module provides a VoIP screen widget that can be used for voice/video calls.
//! It uses the Matrix client from the room screen for authentication and signaling.

use makepad_widgets::*;
use makepad_widgets::makepad_platform::permission::{Permission, PermissionStatus};
use matrix_sdk::Client;
use ruma::OwnedRoomId;
use tokio::sync::mpsc;

use crate::sliding_sync::{get_client, submit_async_request, MatrixRequest};
use super::{VoipGlobalState, VoipAction, CallMember};

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
                height: 120
                margin: Inset{bottom: 8}
                draw_bg.color: #3a3a5a
                draw_bg.radius: 8.0
                flow: Overlay

                // Video view (shown when video is on)
                participant_video_host := View {
                    width: Fill
                    height: Fill
                    visible: false

                    participant_video := Video {
                        width: Fill
                        height: Fill
                        autoplay: false
                        show_controls: false
                    }
                }

                // Avatar view (shown when video is off)
                avatar_container := View {
                    width: Fill
                    height: Fill
                    align: Center

                    avatar := RoundedView {
                        width: 48
                        height: 48
                        draw_bg.color: #a0d0a0
                        draw_bg.radius: 24.0
                        align: Center

                        avatar_letter := Label {
                            text: "?"
                            draw_text.text_style.font_size: 20
                            draw_text.color: #2a6a2a
                        }
                    }
                }

                // Info overlay at bottom
                View {
                    width: Fill
                    height: Fill
                    align: Align{x: 0.0 y: 1.0}
                    padding: 8

                    RoundedView {
                        width: Fit
                        height: Fit
                        padding: Inset{left: 8 right: 8 top: 4 bottom: 4}
                        draw_bg.color: #000000aa
                        draw_bg.radius: 4.0
                        flow: Right
                        spacing: 6

                        mute_icon := Label {
                            text: ""
                            draw_text.text_style.font_size: 10
                            draw_text.color: #fff
                        }

                        name_label := Label {
                            text: "Participant"
                            draw_text.text_style.font_size: 10
                            draw_text.color: #fff
                        }

                        status_label := Label {
                            text: ""
                            draw_text.text_style.font_size: 10
                            draw_text.color: #4CAF50
                        }
                    }
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

            // Main content area (participants + video)
            call_content := View {
                width: Fill
                height: Fill
                flow: Right
                spacing: 0

                // Participants list on the left
                participants_panel := View {
                    width: 220
                    height: Fill
                    padding: 12
                    show_bg: true
                    draw_bg.color: #1e1e3a
                    flow: Down
                    spacing: 8

                    Label {
                        text: "Participants"
                        draw_text.text_style.font_size: 14
                        draw_text.color: #fff
                        margin: Inset{bottom: 8}
                    }

                    participants_list := mod.widgets.VoipParticipantsList {}
                }

                // Local video container (takes remaining space)
                local_video_container := View {
                    width: Fill
                    height: Fill
                    flow: Overlay

                    // Speaking indicator border
                    local_speaking_border := RoundedView {
                        width: Fill
                        height: Fill
                        draw_bg.color: #4CAF50
                        draw_bg.radius: 0.0
                        visible: false
                    }

                    // Avatar placeholder (shown when camera is off)
                    local_avatar_view := View {
                        width: Fill
                        height: Fill
                        align: Center
                        show_bg: true
                        draw_bg.color: #2a2a4a

                        RoundedView {
                            width: 120
                            height: 120
                            draw_bg.color: #a0d0a0
                            draw_bg.radius: 60.0
                            align: Center

                            local_avatar_letter := Label {
                                text: "Y"
                                draw_text.text_style.font_size: 48
                                draw_text.color: #2a6a2a
                            }
                        }
                    }

                    // Camera video (shown when camera is on)
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

                    // Name badge overlay at bottom left
                    View {
                        width: Fill
                        height: Fill
                        align: Align{x: 0.0 y: 1.0}
                        padding: 16

                        RoundedView {
                            width: Fit
                            height: Fit
                            padding: Inset{left: 12 right: 12 top: 6 bottom: 6}
                            draw_bg.color: #000000aa
                            draw_bg.radius: 6.0
                            flow: Right
                            spacing: 6

                            local_mute_icon := Label {
                                text: ""
                                draw_text.text_style.font_size: 14
                                draw_text.color: #fff
                            }

                            local_name_label := Label {
                                text: "You"
                                draw_text.text_style.font_size: 14
                                draw_text.color: #fff
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
                    hangup_button := Button { text: "End" width: 60 }
                }
            }
        }

        // Lobby view
        lobby_view := View {
            width: Fill
            height: Fill
            flow: Down
            visible: true
            show_bg: true
            draw_bg.color: #2a2a4a

            // Top bar with close button
            lobby_header := View {
                width: Fill
                height: Fit
                flow: Right
                padding: Inset{top: 12, right: 12, bottom: 0, left: 12}
                align: Align{x: 1.0, y: 0.0}

                close_button := RobrixIconButton {
                    width: 40
                    height: 40
                    padding: 8
                    draw_icon.svg: (ICON_CLOSE)
                    icon_walk: Walk{width: 20, height: 20}
                    draw_bg +: {
                        color: #ffffff20
                        border_radius: 20.0
                    }
                    draw_icon +: {
                        color: #fff
                    }
                }
            }

            // Main content area with camera preview - use flex to fill remaining space
            lobby_content := View {
                width: Fill
                height: Fill
                flow: Overlay
                margin: 0
                show_bg: true
                draw_bg.color: #2a2a4a

                // Camera preview background
                lobby_camera_container := View {
                    width: Fill
                    height: Fill
                    flow: Overlay

                    lobby_camera_placeholder := View {
                        width: Fill
                        height: Fill
                        align: Center

                        // Placeholder logo/icon
                        RoundedView {
                            width: 120
                            height: 120
                            draw_bg.color: #1a1a2e
                            draw_bg.radius: 60.0
                            align: Center

                            Label {
                                text: "VoIP"
                                draw_text.text_style.font_size: 24
                                draw_text.color: #fff
                            }
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

                // Status label at bottom
                View {
                    width: Fill
                    height: Fill
                    align: Align{x: 0.5, y: 0.85}

                    lobby_status := Label {
                        text: ""
                        draw_text.text_style.font_size: 12
                        draw_text.color: #aaa
                    }
                }
            }
            join_call_button_view := View {
                width: Fill
                height: Fit
                align: Align{x: 0.5, y: 0.7}

                join_call_button := Button {
                    text: "Join call"
                    width: 100
                    height: 48
                    draw_bg +: {
                        color: #4CAF50
                        border_radius: 20.0
                    }
                    draw_text +: {
                        color: #fff
                        text_style.font_size: 16
                    }
                }
            }
            // Bottom control bar with icons
            lobby_controls := View {
                width: Fill
                height: Fit
                padding: Inset{top: 16 bottom: 24}
                align: Center
                show_bg: true
                draw_bg.color: #fff

                View {
                    width: Fit
                    height: Fit
                    flow: Right
                    spacing: 16
                    align: Center

                    lobby_mic_button := RobrixIconButton {
                        width: 48
                        height: 48
                        padding: Inset{top: 10, bottom: 10, left: 10, right: 10}
                        margin: 0,
                        draw_icon.svg: (ICON_MICROPHONE)
                        icon_walk: Walk{width: 24, height: 24}
                        draw_bg +: {
                            color: #fff
                            border_radius: 0.0
                            border_size: 1.5
                            border_color: #ccc
                        }
                    }

                    // Video icon button
                    lobby_camera_button :=  RobrixIconButton {
                        width: 48
                        height: 48
                        padding: Inset{top: 10, bottom: 10, left: 10, right: 10}
                        margin: 0,
                        draw_icon.svg: (ICON_VIDEO)
                        icon_walk: Walk{width: 24, height: 24}
                        draw_bg +: {
                            color: #fff
                            border_radius: 0.0
                            border_size: 1.5
                            border_color: #ccc
                        }
                    }

                    // Settings icon button
                    lobby_settings_button :=  RobrixIconButton {
                        width: 48
                        height: 48
                        padding: Inset{top: 10, bottom: 10, left: 10, right: 10}
                        margin: 0,
                        draw_icon.svg: (ICON_SETTINGS)
                        icon_walk: Walk{width: 24, height: 24}
                        draw_bg +: {
                            color: #000000
                            border_radius: 0.0
                            border_size: 1.5
                            border_color: #ccc
                        }
                    }
                }
            }

            // Hidden buttons for compatibility
            video_call_button := Button { visible: false text: "Video Call" }
            voice_call_button := Button { visible: false text: "Voice Call" }
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
    /// Whether the user navigated here from a call notification (to join an existing call)
    #[rust] from_notification: bool,
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

    // Timer for refreshing call members from Matrix
    #[rust] call_members_refresh_timer: Timer,
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
            _ => {
                if self.speaking_check_timer.is_event(event).is_some() {
                    self.check_speaking_state(cx);
                }
                if self.video_publish_timer.is_event(event).is_some() {
                    // Video publishing handled here if needed
                }
                if self.call_members_refresh_timer.is_event(event).is_some() {
                    // Refresh call members from Matrix (only when in a call)
                    if !self.in_lobby {
                        if let Some(room_id) = self.room_id.clone() {
                            submit_async_request(MatrixRequest::GetCallMembers { room_id });
                        }
                    }
                }
            }
        }

        // Poll LiveKit messages
        if self.poll_livekit_messages(cx) {
            self.update_ui(cx);
        }

        // Let the view process events first (including button clicks)
        self.view.handle_event(cx, event, scope);

        // Then handle actions AFTER the view has processed them
        if let Event::Actions(actions) = event {
            self.handle_actions(cx, actions);

            // Handle VoipActions
            for action in actions {
                if let Some(voip_action) = action.downcast_ref::<VoipAction>() {
                    match voip_action {
                        VoipAction::JoinCall => {
                            if self.in_lobby {
                                log!("VoipScreen: Received VoipAction::JoinCall, triggering join call");
                                self.start_call(cx, super::call_state::CallType::Video);
                            } else {
                                log!("VoipScreen: VoipAction::JoinCall ignored - not in lobby");
                            }
                        }
                        VoipAction::CallMemberStateSent { room_id, success } => {
                            if self.room_id.as_ref() == Some(room_id) {
                                if *success {
                                    log!("VoipScreen: Call member state sent successfully");
                                    self.call.connection_state = ConnectionState::Connecting;
                                    self.in_lobby = false;
                                    self.call_start_time = Some(Cx::time_now());

                                    // Stop lobby camera and prepare for call camera
                                    CameraManager::stop_lobby_camera(&self.view, cx);
                                    self.pending_call_camera_start = true;
                                    self.camera_active = false;

                                    // Fetch call members immediately after joining
                                    submit_async_request(MatrixRequest::GetCallMembers { room_id: room_id.clone() });

                                    // Start LiveKit connection flow: fetch OpenID token
                                    log!("VoipScreen: Fetching OpenID token for LiveKit auth");
                                    submit_async_request(MatrixRequest::FetchOpenIdToken { room_id: room_id.clone() });
                                } else {
                                    log!("VoipScreen: Failed to send call member state");
                                    self.call.connection_state = ConnectionState::Disconnected;
                                }
                                self.update_ui(cx);
                            }
                        }
                        VoipAction::OpenIdTokenFetched { room_id, access_token, token_type, matrix_server_name, expires_in } => {
                            if self.room_id.as_ref() == Some(room_id) {
                                log!("VoipScreen: OpenID token fetched, now fetching LiveKit JWT");
                                log!("  server_name: {}", matrix_server_name);
                                log!("  expires_in: {} seconds", expires_in);

                                // Next step: fetch LiveKit JWT from SFU
                                // POST https://livekit-jwt.call.matrix.org/sfu/get
                                submit_async_request(MatrixRequest::FetchLiveKitJwt {
                                    room_id: room_id.clone(),
                                    access_token: access_token.clone(),
                                    token_type: token_type.clone(),
                                    matrix_server_name: matrix_server_name.clone(),
                                    expires_in: *expires_in,
                                });
                            }
                        }
                        VoipAction::LiveKitJwtFetched { room_id, url, jwt } => {
                            if self.room_id.as_ref() == Some(room_id) {
                                log!("VoipScreen: LiveKit JWT fetched, connecting to LiveKit");
                                log!("  url: {}", url);
                                self.connect_livekit(cx, url, jwt);
                            }
                        }
                        VoipAction::LiveKitConnectionFailed { room_id, error } => {
                            if self.room_id.as_ref() == Some(room_id) {
                                log!("VoipScreen: LiveKit connection failed: {}", error);
                                self.call.connection_state = ConnectionState::Disconnected;
                                self.update_ui(cx);
                            }
                        }
                        VoipAction::CallMembersUpdated { room_id, members } => {
                            if self.room_id.as_ref() == Some(room_id) {
                                log!("VoipScreen: CallMembersUpdated - {} members", members.len());
                                self.update_participants_from_call_members(cx, members);
                                self.update_ui(cx);
                                self.redraw(cx);
                            }
                        }
                        VoipAction::TestAddParticipant { name, is_video_on } => {
                            log!("VoipScreen: TestAddParticipant - name={}, video={}", name, is_video_on);
                            self.add_participant(cx, name, *is_video_on);
                            self.update_ui(cx);
                        }
                        VoipAction::TestToggleParticipantVideo { id } => {
                            log!("VoipScreen: TestToggleParticipantVideo - id={}", id);
                            self.toggle_participant_video(cx, id);
                            self.update_ui(cx);
                        }
                        VoipAction::TestRemoveParticipant { id } => {
                            log!("VoipScreen: TestRemoveParticipant - id={}", id);
                            self.remove_participant(cx, id);
                            self.update_ui(cx);
                        }
                        VoipAction::TestClearParticipants => {
                            log!("VoipScreen: TestClearParticipants");
                            self.clear_participants(cx);
                            self.update_ui(cx);
                        }
                        VoipAction::TestToggleParticipantsSidebar => {
                            log!("VoipScreen: TestToggleParticipantsSidebar");
                            self.show_participants = !self.show_participants;
                            self.update_ui(cx);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl VoipScreen {
    /// Initialize the VoIP screen
    pub fn initialize(&mut self, cx: &mut Cx, room_id: OwnedRoomId) {
        log!("VoipScreen: Initializing for room {}", room_id);
        self.in_lobby = true;
        self.lobby_mic_enabled = true;
        self.lobby_camera_enabled = true;
        self.call = Call::default();
        self.speaking_detector = SpeakingDetector::new();

        // Initialize LiveKit client
        let mut client = LiveKitClient::new();
        let rx = client.start();
        self.livekit_client = Some(client);
        self.livekit_rx = Some(rx);

        // Timer for speaking detection
        self.speaking_check_timer = cx.start_interval(0.1);

        // Timer for video frames (~30fps)
        self.video_publish_timer = cx.start_interval(1.0 / 30.0);

        // Timer for refreshing call members (every 5 seconds)
        self.call_members_refresh_timer = cx.start_interval(5.0);

        // Read camera permission and choice from global state (captured at app startup)
        self.camera_permission = VoipGlobalState::get_camera_permission(cx);
        self.camera_choice = VoipGlobalState::get_camera_choice(cx);

        // Try to start camera if we already have permission and camera choice
        self.try_start_camera(cx);

        // Set default room
        self.set_room(cx, room_id.clone());

        // Fetch initial call members
        submit_async_request(MatrixRequest::GetCallMembers { room_id });

        self.update_ui(cx);
    }

    /// Set the room for this VoIP call (uses Matrix client from room screen)
    /// When called from a call notification, this shows the "Join Call" button.
    pub fn set_room(&mut self, cx: &mut Cx, room_id: OwnedRoomId) {
        self.room_id = Some(room_id.clone());
        self.from_notification = true;  // Show "Join Call" button

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

        // Send call member state event via Matrix (MSC3401)
        if let Some(room_id) = self.room_id.clone() {
            log!("Submitting SendCallMemberState request for room {}", room_id);
            submit_async_request(MatrixRequest::SendCallMemberState {
                room_id,
                call_type,
            });
        } else {
            log!("Error: No room_id set, cannot start call");
            self.call.connection_state = ConnectionState::Disconnected;
        }

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
                LiveKitMessage::RemoteVideoFrame { participant_id, y, u, v, width, height, pts_ms } => {
                    // TODO: Update participant's video texture with the I420 frame data
                    // This would use RemoteVideoSession to push frames to a Video widget
                    log!("Remote video frame from {}: {}x{} (Y:{} U:{} V:{} bytes) pts={}ms",
                        participant_id, width, height, y.len(), u.len(), v.len(), pts_ms);
                    // For now, just mark needs_update to trigger UI refresh
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
    fn toggle_camera(&mut self, cx: &mut Cx) {
        self.call.local_video_muted = !self.call.local_video_muted;
        if let Some(client) = &self.livekit_client {
            client.set_camera_muted(self.call.local_video_muted);
        }

        if self.call.local_video_muted {
            // Camera off - stop the call camera
            log!("Camera off - stopping call camera");
            CameraManager::stop_call_camera(&self.view, cx);
            self.camera_active = false;
        } else {
            // Camera on - start the call camera
            log!("Camera on - starting call camera");
            if let Some(choice) = self.camera_choice.clone() {
                if CameraManager::start_call_camera(&self.view, cx, &choice) {
                    self.camera_active = true;
                }
            }
        }
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

        // Send end call state event via Matrix (MSC3401)
        if let Some(room_id) = self.room_id.clone() {
            log!("Submitting SendEndCallState request for room {}", room_id);
            submit_async_request(MatrixRequest::SendEndCallState { room_id });
        }

        // Reset state
        self.call.connection_state = ConnectionState::Disconnected;
        self.in_lobby = true;
        self.call_start_time = None;
        CameraManager::stop_lobby_camera(&self.view, cx);
        self.pending_call_camera_start = true;
        self.camera_active = false;

        self.update_ui(cx);
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

        // Participants panel is now always visible on the left (no toggle needed)
        self.view.view(cx, ids!(debug_panel)).set_visible(cx, self.show_debug);

        if let Some(start) = self.call_start_time {
            let elapsed = (Cx::time_now() - start) as u64;
            let mins = elapsed / 60;
            let secs = elapsed % 60;
            self.view.label(cx, ids!(call_duration))
                .set_text(cx, &format!("{:02}:{:02}", mins, secs));
        }

        // Update lobby icon button styles based on state
        // When disabled, show different border color
        if self.in_lobby {
            let mut mic_btn = self.view.button(cx, ids!(lobby_mic_button));
            let mut cam_btn = self.view.button(cx, ids!(lobby_camera_button));

            if self.lobby_mic_enabled {
                script_apply_eval!(cx, mic_btn, {
                    draw_bg +: { border_color: #ccc }
                    draw_icon +: { color: #333 }
                });
            } else {
                script_apply_eval!(cx, mic_btn, {
                    draw_bg +: { border_color: #f00 }
                    draw_icon +: { color: #f00 }
                });
            }

            if self.lobby_camera_enabled {
                script_apply_eval!(cx, cam_btn, {
                    draw_bg +: { border_color: #ccc }
                    draw_icon +: { color: #333 }
                });
            } else {
                script_apply_eval!(cx, cam_btn, {
                    draw_bg +: { border_color: #f00 }
                    draw_icon +: { color: #f00 }
                });
            }
        }

        // Show "Join Call" button always in lobby (it's the main action button now)
        self.view.view(cx, ids!(join_call_button_view)).set_visible(cx, self.in_lobby);
        self.view.button(cx, ids!(join_call_button)).set_visible(cx, self.in_lobby);

        // Force redraw to ensure all visibility changes take effect
        self.view.redraw(cx);
    }

    /// Try to start camera
    fn try_start_camera(&mut self, cx: &mut Cx) {
        if !matches!(self.camera_permission, Some(PermissionStatus::Granted)) {
            self.view.label(cx, ids!(lobby_status)).set_text(cx, "Waiting for camera permission...");
            return;
        }

        let Some(choice) = self.camera_choice.clone() else {
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
                self.view.view(cx, ids!(join_call_button_view)).set_visible(cx, true);
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

    /// Handle video resources released (camera handoff from lobby to call)
    fn handle_video_resources_released(&mut self, cx: &mut Cx) {
        if self.pending_call_camera_start {
            log!("Lobby camera released, starting call camera...");
            self.pending_call_camera_start = false;
            if let Some(choice) = self.camera_choice.clone() {
                if CameraManager::start_call_camera(&self.view, cx, &choice) {
                    log!("Call camera started successfully");
                    self.camera_active = true;
                }
            }
        } else {
            log!("Video resources released");
        }
    }

    /// Handle UI actions
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions) {
        
        // Lobby buttons
        // Close button - exit VoIP screen
        if self.view.button(cx, ids!(close_button)).clicked(actions) {
            log!("Close button clicked, exiting VoIP screen");
            if let Some(room_id) = self.room_id.clone() {
                log!("Emitting VoipAction::Close for room {}", room_id);
                
                self.hangup(cx);
                cx.action(VoipAction::Close(room_id));
            }
        }
        // Join Call button - main action to start the call
        if self.view.button(cx, ids!(join_call_button)).clicked(actions) {
            log!("Join call button clicked for room: {:?}", self.room_id);
            self.from_notification = false;
            self.start_call(cx, CallType::Video);
        }
        // Microphone toggle (icon button)
        if self.view.button(cx, ids!(lobby_mic_button)).clicked(actions) {
            self.lobby_mic_enabled = !self.lobby_mic_enabled;
            log!("Lobby mic toggled: {}", self.lobby_mic_enabled);
            self.update_ui(cx);
        }
        // Camera toggle (icon button)
        if self.view.button(cx, ids!(lobby_camera_button)).clicked(actions) {
            self.lobby_camera_enabled = !self.lobby_camera_enabled;
            log!("Lobby camera toggled: {}", self.lobby_camera_enabled);
            self.update_ui(cx);
        }
        // Settings button (icon button)
        if self.view.button(cx, ids!(lobby_settings_button)).clicked(actions) {
            log!("Settings button clicked");
            // Toggle debug panel for now
            self.show_debug = !self.show_debug;
            self.update_ui(cx);
        }
        // Legacy buttons (hidden, for compatibility)
        if self.view.button(cx, ids!(video_call_button)).clicked(actions) {
            self.start_call(cx, CallType::Video);
        }
        if self.view.button(cx, ids!(voice_call_button)).clicked(actions) {
            self.start_call(cx, CallType::Voice);
        }

        // Call controls
        if self.view.button(cx, ids!(mic_button)).clicked(actions) {
            self.toggle_microphone();
            self.update_ui(cx);
        }
        if self.view.button(cx, ids!(camera_button)).clicked(actions) {
            self.toggle_camera(cx);
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

    /// Add a test participant (with optional video on)
    pub fn add_participant(&mut self, cx: &mut Cx, name: &str, is_video_on: bool) {
        self.participant_counter += 1;
        let letter = name.chars().next().unwrap_or('?').to_uppercase().to_string();
        let participant = Participant {
            id: format!("{}", self.participant_counter),
            name: name.to_string(),
            avatar_letter: letter,
            is_muted: false,
            is_speaking: false,
            is_video_on,
        };
        log!("Adding participant: {} (id={}, video={})", name, self.participant_counter, is_video_on);

        let list = self.view.participants_list(cx, ids!(participants_list));
        list.add_participant(cx, participant);
    }

    /// Toggle participant video state
    pub fn toggle_participant_video(&mut self, cx: &mut Cx, id: &str) {
        log!("Toggling video for participant id={}", id);
        let list = self.view.participants_list(cx, ids!(participants_list));
        list.update_participant(cx, id, |p| {
            p.is_video_on = !p.is_video_on;
            log!("Participant {} video is now {}", p.name, if p.is_video_on { "on" } else { "off" });
        });
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

    /// Update participants list from Matrix call member state events
    fn update_participants_from_call_members(&mut self, cx: &mut Cx, members: &[CallMember]) {
        log!("update_participants_from_call_members: received {} members", members.len());

        // Clear existing participants and rebuild from call members
        let list = self.view.participants_list(cx, ids!(participants_list));
        list.clear(cx);
        self.participant_counter = 0;

        // Get current user to exclude self from participants list
        let current_user_id = get_client()
            .and_then(|c| c.session_meta().map(|m| m.user_id.to_string()));
        log!("Current user ID: {:?}", current_user_id);

        for member in members {
            // Skip self
            if current_user_id.as_ref() == Some(&member.user_id) {
                continue;
            }

            self.participant_counter += 1;
            let name = member.display_name.clone()
                .unwrap_or_else(|| member.user_id.clone());
            let letter = name.chars().next().unwrap_or('?').to_uppercase().to_string();

            let participant = Participant {
                id: format!("{}_{}", member.user_id, member.device_id),
                name,
                avatar_letter: letter,
                is_muted: false,  // We don't have this info from state events
                is_speaking: false,
                is_video_on: false,  // We don't have this info from state events
            };

            log!("Adding call member: {} (user={}, device={})",
                participant.name, member.user_id, member.device_id);
            list.add_participant(cx, participant);
        }

        // Update participant count display
        let count = members.len();
        self.view.label(cx, ids!(participant_count))
            .set_text(cx, &format!("{} participant{}", count, if count == 1 { "" } else { "s" }));

        log!("Updated participants panel with {} other participants", self.participant_counter);
    }

    /// Connect to LiveKit with the given URL and JWT token
    fn connect_livekit(&mut self, cx: &mut Cx, url: &str, jwt: &str) {
        log!("connect_livekit: url={}", url);

        if let Some(client) = &self.livekit_client {
            // Connect to LiveKit
            client.connect(url.to_string(), jwt.to_string());

            // Update connection state
            self.call.connection_state = ConnectionState::Connected;
            log!("LiveKit connection initiated");
        } else {
            log!("Error: LiveKit client not initialized");
            self.call.connection_state = ConnectionState::Disconnected;
        }

        self.update_ui(cx);
    }
}

impl VoipScreenRef {
    /// Initialize the VoIP screen
    pub fn initialize(&self, cx: &mut Cx, room_id: OwnedRoomId) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.initialize(cx, room_id);
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
    pub fn add_participant(&self, cx: &mut Cx, name: &str, is_video_on: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.add_participant(cx, name, is_video_on);
        }
    }

    /// Toggle participant video state
    pub fn toggle_participant_video(&self, cx: &mut Cx, id: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.toggle_participant_video(cx, id);
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

    pub fn hangup(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.hangup(cx);
        }
    }
}
