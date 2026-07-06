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
use super::{VoipGlobalState, VoipAction, CallMember, ActiveCallState, ParticipantInfo};

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
                height: 100
                margin: Inset{bottom: 4}
                draw_bg.color: #3a3a5a
                draw_bg.radius: 6.0
                flow: Overlay

                // Video view (shown when video is on)
                participant_video := WebRtcVideo {
                    width: Fill
                    height: Fill
                    visible: false
                }

                // Avatar view (shown when video is off)
                avatar_container := View {
                    width: Fill
                    height: Fill
                    align: Center

                    avatar := RoundedView {
                        width: 40
                        height: 40
                        draw_bg.color: #a0d0a0
                        draw_bg.radius: 20.0
                        align: Center

                        avatar_letter := Label {
                            text: "?"
                            draw_text.text_style.font_size: 18
                            draw_text.color: #2a6a2a
                        }
                    }
                }

                // Info overlay at bottom
                View {
                    width: Fill
                    height: Fill
                    align: Align{x: 0.5 y: 1.0}
                    padding: 6

                    RoundedView {
                        width: Fit
                        height: Fit
                        padding: Inset{left: 8 right: 10 top: 4 bottom: 4}
                        draw_bg.color: #1a1a3a
                        draw_bg.radius: 12.0
                        flow: Right
                        spacing: 4
                        align: Center

                        // The card shows whichever of these two icons
                        // matches the participant's current mute state.
                        // We swap visibility at draw time rather than
                        // re-binding the SVG at runtime, because
                        // script_apply_eval cannot reliably re-resolve
                        // resource references (CLAUDE.md pitfall #41).
                        mute_icon := RobrixIconButton {
                            width: 14
                            height: 14
                            padding: 0
                            draw_icon.svg: (ICON_MICROPHONE)
                            icon_walk: Walk{width: 12, height: 12}
                            draw_bg +: {
                                color: #00000000
                                border_radius: 0.0
                            }
                            draw_icon +: {
                                color: #aaa
                            }
                        }
                        mute_off_icon := RobrixIconButton {
                            width: 14
                            height: 14
                            padding: 0
                            draw_icon.svg: (ICON_MICROPHONE_OFF)
                            icon_walk: Walk{width: 12, height: 12}
                            visible: false
                            draw_bg +: {
                                color: #00000000
                                border_radius: 0.0
                            }
                            draw_icon +: {
                                color: #e53935
                            }
                        }

                        name_label := Label {
                            text: "Participant"
                            draw_text.text_style.font_size: 10
                            draw_text.color: #ddd
                        }

                        status_label := Label {
                            text: ""
                            draw_text.text_style.font_size: 10
                            draw_text.color: #4CAF50
                            visible: false
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
                spacing: 8
                align: Center

                room_name := Label {
                    text: "Call Room"
                    draw_text.text_style.font_size: 18
                    draw_text.color: #888
                }

                call_status := Label {
                    text: "Not connected"
                    draw_text.text_style.font_size: 12
                    draw_text.color: #888
                    margin: Inset{left: 4}
                }

                View { width: Fill height: 1 }

                call_duration := Label {
                    text: ""
                    draw_text.text_style.font_size: 14
                    draw_text.color: #888
                    margin: Inset{right: 8}
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
                    width: 200
                    height: Fill
                    padding: 0
                    show_bg: true
                    draw_bg.color: #1e1e3a
                    flow: Down
                    spacing: 0

                    Label {
                        text: "Participants"
                        draw_text.text_style.font_size: 13
                        draw_text.color: #888
                        margin: Inset{bottom: 4}
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

                    // Name badge overlay at bottom center (always visible)
                    View {
                        width: Fill
                        height: Fill
                        align: Align{x: 0.5 y: 1.0}
                        padding: 12

                        RoundedView {
                            width: Fit
                            height: Fit
                            padding: Inset{left: 10 right: 14 top: 6 bottom: 6}
                            draw_bg.color: #1a1a3a
                            draw_bg.radius: 14.0
                            flow: Right
                            spacing: 6
                            align: Center

                            local_mute_icon := RobrixIconButton {
                                width: 16
                                height: 16
                                padding: 0
                                draw_icon.svg: (ICON_MICROPHONE)
                                icon_walk: Walk{width: 14, height: 14}
                                draw_bg +: {
                                    color: #00000000
                                    border_radius: 0.0
                                }
                                draw_icon +: {
                                    color: #aaa
                                }
                            }

                            local_name_label := Label {
                                text: "You"
                                draw_text.text_style.font_size: 12
                                draw_text.color: #ddd
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
                    spacing: 12

                    mic_button := RobrixIconButton {
                        width: 48
                        height: 48
                        padding: 10
                        draw_icon.svg: (ICON_MICROPHONE)
                        icon_walk: Walk{width: 24, height: 24}
                        draw_bg +: {
                            color: #3a3a5a
                            border_radius: 24.0
                        }
                        draw_icon +: {
                            color: #fff
                        }
                    }

                    camera_button := RobrixIconButton {
                        width: 48
                        height: 48
                        padding: 10
                        draw_icon.svg: (ICON_VIDEO)
                        icon_walk: Walk{width: 24, height: 24}
                        draw_bg +: {
                            color: #3a3a5a
                            border_radius: 24.0
                        }
                        draw_icon +: {
                            color: #fff
                        }
                    }

                    screenshare_button := RobrixIconButton {
                        width: 48
                        height: 48
                        padding: 10
                        draw_icon.svg: (ICON_SQUARES)
                        icon_walk: Walk{width: 24, height: 24}
                        draw_bg +: {
                            color: #3a3a5a
                            border_radius: 24.0
                        }
                        draw_icon +: {
                            color: #fff
                        }
                    }

                    participants_button := RobrixIconButton {
                        width: 48
                        height: 48
                        padding: 10
                        draw_icon.svg: (ICON_ADD_USER)
                        icon_walk: Walk{width: 24, height: 24}
                        draw_bg +: {
                            color: #3a3a5a
                            border_radius: 24.0
                        }
                        draw_icon +: {
                            color: #fff
                        }
                    }

                    hangup_button := RobrixIconButton {
                        width: 48
                        height: 48
                        padding: 10
                        draw_icon.svg: (ICON_CLOSE)
                        icon_walk: Walk{width: 24, height: 24}
                        draw_bg +: {
                            color: #e53935
                            border_radius: 24.0
                        }
                        draw_icon +: {
                            color: #fff
                        }
                    }
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
                    padding: Inset{left: 24, right: 24, top: 12, bottom: 12}
                    text: "Join call"
                    width: Fit
                    height: Fit
                    draw_bg +: {
                        color: #4CAF50
                        border_radius: 4.0
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
    /// Whether this is a voice-only call. When true, the camera is
    /// never started, the lobby hides the camera preview, and remote
    /// video tracks are ignored — only the avatar is shown.
    #[rust] voice_only: bool,

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

    // Timer for refreshing call members from Matrix
    #[rust] call_members_refresh_timer: Timer,

    // Timer for updating call duration display
    #[rust] call_duration_timer: Timer,

    // Test mode: timer for pushing test video frames
    #[rust] test_video_frame_timer: Timer,
    // Test mode: participant ID to push frames to
    #[rust] test_video_participant_id: Option<String>,
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
                if self.call_duration_timer.is_event(event).is_some() {
                    // Update call duration display
                    self.update_call_duration(cx);
                }
                if self.call_members_refresh_timer.is_event(event).is_some() {
                    // Refresh call members from Matrix (only when in a call)
                    if !self.in_lobby {
                        if let Some(room_id) = self.room_id.clone() {
                            submit_async_request(MatrixRequest::GetCallMembers { room_id });
                        }
                    }
                }
                // Test mode: push video frames at ~30fps
                if self.test_video_frame_timer.is_event(event).is_some() {
                    if let Some(ref participant_id) = self.test_video_participant_id.clone() {
                        self.push_test_video_frame(cx, participant_id);
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

                                    // Start call duration timer (updates every second)
                                    self.call_duration_timer = cx.start_interval(1.0);

                                    // Stop lobby camera and prepare for call camera
                                    CameraManager::stop_lobby_camera(&self.view, cx);
                                    self.pending_call_camera_start = true;
                                    self.camera_active = false;

                                    // Fetch call members immediately after joining
                                    submit_async_request(MatrixRequest::GetCallMembers { room_id: room_id.clone() });

                                    // Start LiveKit connection flow with token caching
                                    // Check if we have a valid cached LiveKit JWT for this room
                                    if let Some(cached_jwt) = VoipGlobalState::get_valid_livekit_jwt(cx, room_id) {
                                        log!("VoipScreen: Using cached LiveKit JWT ({} seconds remaining)", cached_jwt.remaining_seconds());
                                        self.connect_livekit(cx, &cached_jwt.url, &cached_jwt.jwt);
                                    } else if let Some(cached_openid) = VoipGlobalState::get_valid_openid_token(cx) {
                                        // Have valid OpenID token, skip to JWT fetch
                                        log!("VoipScreen: Using cached OpenID token ({} seconds remaining), fetching LiveKit JWT", cached_openid.remaining_seconds());
                                        submit_async_request(MatrixRequest::FetchLiveKitJwt {
                                            room_id: room_id.clone(),
                                            access_token: cached_openid.access_token.clone(),
                                            token_type: cached_openid.token_type.clone(),
                                            matrix_server_name: cached_openid.matrix_server_name.clone(),
                                            expires_in: cached_openid.expires_in,
                                        });
                                    } else {
                                        // No cached tokens, start fresh
                                        log!("VoipScreen: No cached tokens, fetching OpenID token for LiveKit auth");
                                        submit_async_request(MatrixRequest::FetchOpenIdToken { room_id: room_id.clone() });
                                    }
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

                                // Cache the OpenID token for future use
                                let cached_token = super::CachedOpenIdToken::new(
                                    access_token.clone(),
                                    token_type.clone(),
                                    matrix_server_name.clone(),
                                    *expires_in,
                                );
                                VoipGlobalState::store_openid_token(cx, cached_token);

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

                                // Cache the LiveKit JWT for future use
                                let cached_jwt = super::CachedLiveKitJwt::new(
                                    jwt.clone(),
                                    url.clone(),
                                    room_id.clone(),
                                );
                                VoipGlobalState::store_livekit_jwt(cx, cached_jwt);

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
                        VoipAction::TestPushVideoFrame { participant_id } => {
                            log!("VoipScreen: TestPushVideoFrame - participant_id={}", participant_id);
                            self.push_test_video_frame(cx, participant_id);
                            self.update_ui(cx);
                        }
                        VoipAction::TestStartVideoStream { participant_id } => {
                            log!("VoipScreen: TestStartVideoStream - participant_id={}", participant_id);
                            self.start_test_video_stream(cx, participant_id);
                        }
                        VoipAction::TestStopVideoStream => {
                            log!("VoipScreen: TestStopVideoStream");
                            self.stop_test_video_stream(cx);
                        }
                        VoipAction::PipMicToggle { room_id } => {
                            if self.room_id.as_ref() == Some(room_id) {
                                log!("VoipScreen: PipMicToggle from PiP");
                                self.toggle_microphone();
                                self.update_ui(cx);
                            }
                        }
                        VoipAction::PipCameraToggle { room_id } => {
                            if self.room_id.as_ref() == Some(room_id) {
                                log!("VoipScreen: PipCameraToggle from PiP");
                                self.toggle_camera(cx);
                                self.update_ui(cx);
                            }
                        }
                        VoipAction::PipScreenShareToggle { room_id } => {
                            if self.room_id.as_ref() == Some(room_id) {
                                log!("VoipScreen: PipScreenShareToggle from PiP");
                                self.toggle_screenshare();
                                self.update_ui(cx);
                            }
                        }
                        VoipAction::PipHangup { room_id } => {
                            if self.room_id.as_ref() == Some(room_id) {
                                log!("VoipScreen: PipHangup from PiP");
                                self.hangup(cx);
                            }
                        }
                        VoipAction::ShowPip { room_id } => {
                            if self.room_id.as_ref() == Some(room_id) {
                                log!("VoipScreen: ShowPip - stopping local camera for PiP");
                                // Stop the call camera so PiP can use it
                                if !self.call.local_video_muted {
                                    CameraManager::stop_call_camera(&self.view, cx);
                                    self.camera_active = false;
                                }
                            }
                        }
                        VoipAction::HidePip => {
                            // When PiP is hidden (user returned to VoIP tab), restart camera
                            if !self.in_lobby && !self.call.local_video_muted {
                                log!("VoipScreen: HidePip - attempting to restart camera");

                                if let Some(choice) = self.camera_choice.clone() {
                                    let video = self.view.video(cx, &[live_id!(local_camera_video)]);
                                    let is_unprepared = video.is_unprepared();
                                    let is_cleaning_up = video.is_cleaning_up();
                                    log!("VoipScreen: local_camera_video state - unprepared={}, cleaning_up={}",
                                        is_unprepared, is_cleaning_up);

                                    if is_unprepared {
                                        // Camera is ready to start
                                        log!("VoipScreen: Starting camera immediately");
                                        if CameraManager::start_call_camera(&self.view, cx, &choice) {
                                            log!("VoipScreen: Call camera started successfully");
                                            self.camera_active = true;
                                        } else {
                                            log!("VoipScreen: Failed to start camera, will retry on release event");
                                            self.pending_call_camera_start = true;
                                            self.camera_active = false;
                                        }
                                    } else if is_cleaning_up {
                                        // Camera is still cleaning up, wait for release event
                                        log!("VoipScreen: Camera is cleaning up, will wait for release");
                                        self.pending_call_camera_start = true;
                                        self.camera_active = false;
                                    } else {
                                        // Video is in some other state - force stop and restart
                                        log!("VoipScreen: Camera in unexpected state, forcing stop and restart");
                                        CameraManager::stop_call_camera(&self.view, cx);
                                        self.pending_call_camera_start = true;
                                        self.camera_active = false;
                                    }
                                }
                            }
                        }
                        VoipAction::ReturnToVoipTab { room_id } => {
                            if self.room_id.as_ref() == Some(room_id) {
                                // When returning to VoIP tab from PiP, restart camera
                                if !self.in_lobby && !self.call.local_video_muted {
                                    log!("VoipScreen: ReturnToVoipTab - attempting to restart camera");

                                    if let Some(choice) = self.camera_choice.clone() {
                                        let video = self.view.video(cx, &[live_id!(local_camera_video)]);
                                        let is_unprepared = video.is_unprepared();
                                        let is_cleaning_up = video.is_cleaning_up();
                                        log!("VoipScreen: local_camera_video state - unprepared={}, cleaning_up={}",
                                            is_unprepared, is_cleaning_up);

                                        if is_unprepared {
                                            // Camera is ready to start
                                            log!("VoipScreen: Starting camera immediately");
                                            if CameraManager::start_call_camera(&self.view, cx, &choice) {
                                                log!("VoipScreen: Call camera started successfully");
                                                self.camera_active = true;
                                            } else {
                                                log!("VoipScreen: Failed to start camera, will retry on release event");
                                                self.pending_call_camera_start = true;
                                                self.camera_active = false;
                                            }
                                        } else if is_cleaning_up {
                                            // Camera is still cleaning up, wait for release event
                                            log!("VoipScreen: Camera is cleaning up, will wait for release");
                                            self.pending_call_camera_start = true;
                                            self.camera_active = false;
                                        } else {
                                            // Video is in some other state - force stop and restart
                                            log!("VoipScreen: Camera in unexpected state, forcing stop and restart");
                                            CameraManager::stop_call_camera(&self.view, cx);
                                            self.pending_call_camera_start = true;
                                            self.camera_active = false;
                                        }
                                    }
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
        self.view.draw_walk(cx, scope, walk)
    }
}

impl VoipScreen {
    /// Initialize the VoIP screen.
    ///
    /// `voice_only`: when `true`, opens in voice-call mode — no
    /// camera preview in the lobby, no local video publish, and
    /// remote video tracks are not subscribed. The lobby still
    /// shows the mic toggle so the user can choose to start muted.
    pub fn initialize(&mut self, cx: &mut Cx, room_id: OwnedRoomId, voice_only: bool) {
        log!("VoipScreen: Initializing for room {} (voice_only={})", room_id, voice_only);
        self.voice_only = voice_only;
        self.in_lobby = true;
        self.lobby_mic_enabled = true;
        // In voice-only mode the lobby camera preview is suppressed
        // and the call's local video track starts muted.
        self.lobby_camera_enabled = !voice_only;
        self.show_participants = true;
        self.call = Call::default();
        self.call.local_video_muted = voice_only;
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

        // Try to start camera if we already have permission and camera
        // choice — but only when this is a video call. Voice-only
        // calls deliberately leave the camera off to keep the lobby
        // and the in-call view avatar-only.
        if !self.voice_only {
            self.try_start_camera(cx);
        }

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

        // Get room name and user display name from client
        if let Some(client) = get_client() {
            if let Some(room) = client.get_room(&room_id) {
                let room_name = room.name().unwrap_or_else(|| room_id.to_string());
                self.view.label(cx, ids!(room_name)).set_text(cx, &room_name);
            }

            // Set local user's display name
            if let Some(session) = client.session_meta() {
                let user_id = session.user_id.to_string();
                // Extract display name from user_id (remove @ prefix and domain)
                let display_name = user_id
                    .strip_prefix('@')
                    .and_then(|s| s.split(':').next())
                    .unwrap_or(&user_id);
                // Set name on local user badge
                self.view.label(cx, ids!(local_name_label)).set_text(cx, display_name);

                // Also set avatar letter
                let letter = display_name.chars().next().unwrap_or('?').to_uppercase().to_string();
                self.view.label(cx, ids!(local_avatar_letter)).set_text(cx, &letter);
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
    fn poll_livekit_messages(&mut self, cx: &mut Cx) -> bool {
        let messages: Vec<LiveKitMessage> = if let Some(rx) = &mut self.livekit_rx {
            let mut msgs = Vec::new();
            while let Ok(msg) = rx.try_recv() {
                msgs.push(msg);
            }
            msgs
        } else {
            // Only log this once per second to avoid spam — the absence
            // of `livekit_rx` is the bug we want to surface.
            static LAST_LOG: std::sync::Mutex<Option<std::time::Instant>> =
                std::sync::Mutex::new(None);
            if let Ok(mut last) = LAST_LOG.lock() {
                let now = std::time::Instant::now();
                if last.is_none_or(|t| now.duration_since(t).as_secs() >= 1) {
                    log!("poll_livekit_messages: livekit_rx is None (LiveKit client not initialized?)");
                    *last = Some(now);
                }
            }
            Vec::new()
        };

        let mut needs_update = false;
        for msg in messages {
            match msg {
                LiveKitMessage::Connected => {
                    self.call.connection_state = ConnectionState::Connected;
                    self.in_lobby = false;
                    self.call_start_time = Some(Cx::time_now());
                    // Start call duration timer (updates every second)
                    self.call_duration_timer = cx.start_interval(1.0);
                    log!("LiveKit connected");
                    needs_update = true;
                }
                LiveKitMessage::Disconnected => {
                    self.call.connection_state = ConnectionState::Disconnected;
                    log!("LiveKit disconnected");
                    needs_update = true;
                }
                LiveKitMessage::ParticipantJoined(p) => {
                    log!("Participant joined: {}", p.user_id);

                    // Add to participants list UI
                    let name = if p.display_name.is_empty() {
                        p.user_id.clone()
                    } else {
                        p.display_name.clone()
                    };
                    let letter = name.chars().next().unwrap_or('?').to_uppercase().to_string();

                    let participant = Participant {
                        id: p.user_id.clone(),
                        name,
                        avatar_letter: letter,
                        is_muted: p.is_muted,
                        is_speaking: p.is_speaking,
                        is_video_on: p.is_video_on,
                    };

                    let list = self.view.participants_list(cx, ids!(participants_list));
                    list.add_participant(cx, participant);

                    self.call.participants.insert(p.user_id.clone(), p);
                    needs_update = true;
                }
                LiveKitMessage::ParticipantLeft(id) => {
                    log!("Participant left: {}", id);

                    // Remove from participants list UI
                    let list = self.view.participants_list(cx, ids!(participants_list));
                    list.remove_participant(cx, &id);

                    self.call.participants.remove(&id);
                    needs_update = true;
                }
                LiveKitMessage::Error(e) => {
                    log!("LiveKit error: {}", e);
                    self.call.connection_state = ConnectionState::Disconnected;
                    needs_update = true;
                }
                LiveKitMessage::VideoTrackSubscribed { participant_id } => {
                    log!("Video track subscribed for participant: {}", participant_id);

                    // Update participant's video state
                    let list = self.view.participants_list(cx, ids!(participants_list));
                    list.update_participant(cx, &participant_id, |p| {
                        p.is_video_on = true;
                    });

                    if let Some(p) = self.call.participants.get_mut(&participant_id) {
                        p.is_video_on = true;
                    }
                    needs_update = true;
                }
                LiveKitMessage::VideoTrackUnsubscribed { participant_id } => {
                    log!("Video track unsubscribed for participant: {}", participant_id);

                    // Update participant's video state
                    let list = self.view.participants_list(cx, ids!(participants_list));
                    list.update_participant(cx, &participant_id, |p| {
                        p.is_video_on = false;
                    });

                    if let Some(p) = self.call.participants.get_mut(&participant_id) {
                        p.is_video_on = false;
                    }
                    needs_update = true;
                }
                LiveKitMessage::ParticipantAudioMuteChanged { participant_id, is_muted } => {
                    log!("Audio mute changed for {}: muted={}", participant_id, is_muted);
                    let list = self.view.participants_list(cx, ids!(participants_list));
                    list.update_participant(cx, &participant_id, |p| {
                        p.is_muted = is_muted;
                    });
                    // Mirror to the internal call-state map so any
                    // downstream consumers (PiP, status) stay in sync.
                    if let Some(p) = self.call.participants.get_mut(&participant_id) {
                        p.is_muted = is_muted;
                    }
                    needs_update = true;
                }
                LiveKitMessage::RemoteVideoFrame { participant_id, y, u, v, width, height, pts_ms } => {
                    // Push the I420 frame to the participant's video session
                    // Only log periodically to avoid spam
                    if pts_ms % 1000 < 33 {
                        log!("Remote video frame from {}: {}x{} pts={}ms", participant_id, width, height, pts_ms);
                    }

                    // Get the participants list and push the frame
                    let participants_list = self.view.participants_list(cx, ids!(participants_list));
                    participants_list.push_video_frame(
                        cx,
                        &participant_id,
                        y,
                        u,
                        v,
                        width,
                        height,
                        pts_ms,
                    );
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
        // Stop call duration timer
        self.call_duration_timer = Timer::default();
        CameraManager::stop_lobby_camera(&self.view, cx);
        self.pending_call_camera_start = true;
        self.camera_active = false;

        // Clear global state for PiP
        VoipGlobalState::clear_active_call(cx);

        self.update_ui(cx);
    }

    /// Update call duration display
    fn update_call_duration(&mut self, cx: &mut Cx) {
        if let Some(start) = self.call_start_time {
            let elapsed = (Cx::time_now() - start) as u64;
            let mins = elapsed / 60;
            let secs = elapsed % 60;
            self.view.label(cx, ids!(call_duration))
                .set_text(cx, &format!("{:02}:{:02}", mins, secs));
            self.redraw(cx);
        }
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

        // NOTE: participant_count label is owned by
        // `update_participants_from_call_members` (Matrix call-member
        // state is authoritative for "who joined"). Writing it here
        // too caused the Matrix-driven count to be overwritten on the
        // next LiveKit poll tick.

        // Update call control icon button styles based on state
        let mut mic_btn = self.view.button(cx, ids!(mic_button));
        let mut cam_btn = self.view.button(cx, ids!(camera_button));
        let mut screen_btn = self.view.button(cx, ids!(screenshare_button));
        let mut users_btn = self.view.button(cx, ids!(participants_button));

        // Mic button - red when muted
        if self.call.local_audio_muted {
            script_apply_eval!(cx, mic_btn, {
                draw_bg +: { color: #e53935 }
            });
        } else {
            script_apply_eval!(cx, mic_btn, {
                draw_bg +: { color: #3a3a5a }
            });
        }

        // Camera button - red when off
        if self.call.local_video_muted {
            script_apply_eval!(cx, cam_btn, {
                draw_bg +: { color: #e53935 }
            });
        } else {
            script_apply_eval!(cx, cam_btn, {
                draw_bg +: { color: #3a3a5a }
            });
        }

        // Screen share button - green when sharing
        if self.call.is_screen_sharing {
            script_apply_eval!(cx, screen_btn, {
                draw_bg +: { color: #4CAF50 }
            });
        } else {
            script_apply_eval!(cx, screen_btn, {
                draw_bg +: { color: #3a3a5a }
            });
        }

        // Participants button - highlighted when panel is visible
        if self.show_participants {
            script_apply_eval!(cx, users_btn, {
                draw_bg +: { color: #4a4a6a }
            });
        } else {
            script_apply_eval!(cx, users_btn, {
                draw_bg +: { color: #3a3a5a }
            });
        }

        // Update local mute icon - red when muted, gray when unmuted
        let mut local_mute_btn = self.view.button(cx, ids!(local_mute_icon));
        if self.call.local_audio_muted {
            script_apply_eval!(cx, local_mute_btn, {
                draw_icon +: { color: #e53935 }
            });
        } else {
            script_apply_eval!(cx, local_mute_btn, {
                draw_icon +: { color: #aaa }
            });
        }

        // Toggle participants panel visibility
        self.view.view(cx, ids!(participants_panel)).set_visible(cx, self.show_participants);
        self.view.view(cx, ids!(debug_panel)).set_visible(cx, self.show_debug);

        // Update call duration display (timer handles continuous updates)
        self.update_call_duration(cx);

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
        //self.view.redraw(cx);

        // Sync state to global for PiP display
        self.sync_to_global_state(cx);
    }

    /// Sync current call state to global state for PiP overlay access
    fn sync_to_global_state(&mut self, cx: &mut Cx) {
        // Only sync if we have a room and are in a call (not lobby)
        if self.in_lobby {
            // Clear global state when in lobby
            VoipGlobalState::clear_active_call(cx);
            return;
        }

        let status_text = match self.call.connection_state {
            ConnectionState::Disconnected => "Not connected".to_string(),
            ConnectionState::Connecting => "Connecting...".to_string(),
            ConnectionState::Connected => "In call".to_string(),
            ConnectionState::Disconnecting => "Disconnecting...".to_string(),
        };

        // Get local participant info
        let local_participant = if let Some(client) = get_client() {
            if let Some(session) = client.session_meta() {
                let user_id = session.user_id.to_string();
                let display_name = user_id
                    .strip_prefix('@')
                    .and_then(|s| s.split(':').next())
                    .unwrap_or(&user_id)
                    .to_string();
                let avatar_letter = display_name.chars().next()
                    .unwrap_or('?')
                    .to_uppercase()
                    .to_string();
                ParticipantInfo {
                    user_id,
                    display_name,
                    avatar_letter,
                }
            } else {
                ParticipantInfo::default()
            }
        } else {
            ParticipantInfo::default()
        };

        let active_call = ActiveCallState {
            room_id: self.room_id.clone(),
            status_text,
            in_call: !self.in_lobby && self.call.connection_state != ConnectionState::Disconnected,
            mic_muted: self.call.local_audio_muted,
            camera_muted: self.call.local_video_muted,
            screen_sharing: self.call.is_screen_sharing,
            local_participant,
            participant_count: self.call.participants.len() + 1,
        };

        VoipGlobalState::update_active_call(cx, active_call);
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

    /// Handle video resources released (camera handoff from lobby/PiP to call)
    fn handle_video_resources_released(&mut self, cx: &mut Cx) {
        if self.pending_call_camera_start {
            log!("VoipScreen: Camera resources released, attempting to start call camera...");
            self.pending_call_camera_start = false;
            if let Some(choice) = self.camera_choice.clone() {
                let video = self.view.video(cx, &[live_id!(local_camera_video)]);
                let is_unprepared = video.is_unprepared();
                let is_cleaning_up = video.is_cleaning_up();
                log!("VoipScreen: local_camera_video state - unprepared={}, cleaning_up={}",
                    is_unprepared, is_cleaning_up);

                if is_unprepared {
                    // Camera is ready to start
                    if CameraManager::start_call_camera(&self.view, cx, &choice) {
                        log!("VoipScreen: Call camera started successfully");
                        self.camera_active = true;
                    } else {
                        log!("VoipScreen: Failed to start camera even though unprepared");
                        self.pending_call_camera_start = true;
                    }
                } else if is_cleaning_up {
                    // Still cleaning up, wait for next release event
                    log!("VoipScreen: Camera still cleaning up, will retry on next release");
                    self.pending_call_camera_start = true;
                } else {
                    // Unexpected state - force stop and retry
                    log!("VoipScreen: Camera in unexpected state, forcing stop");
                    CameraManager::stop_call_camera(&self.view, cx);
                    self.pending_call_camera_start = true;
                }
            }
        } else {
            log!("VoipScreen: Video resources released (no pending start)");
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
    /// Returns the participant ID for use with push_test_video_frame
    pub fn add_participant(&mut self, cx: &mut Cx, name: &str, is_video_on: bool) -> String {
        let letter = name.chars().next().unwrap_or('?').to_uppercase().to_string();
        // Use a predictable ID format: "test_<name>" for easy testing
        let participant_id = format!("test_{}", name.to_lowercase().replace(' ', "_"));
        let participant = Participant {
            id: participant_id.clone(),
            name: name.to_string(),
            avatar_letter: letter,
            is_muted: false,
            is_speaking: false,
            is_video_on,
        };
        log!("Adding participant: {} (id={}, video={})", name, participant_id, is_video_on);

        let list = self.view.participants_list(cx, ids!(participants_list));
        list.add_participant(cx, participant);
        participant_id
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

    /// Clear all participants and their video textures
    pub fn clear_participants(&mut self, cx: &mut Cx) {
        log!("Clearing all participants");
        let list = self.view.participants_list(cx, ids!(participants_list));
        list.clear_all(cx);  // Use clear_all to also remove video textures
    }

    /// Start continuous test video frames to a participant (~30fps)
    pub fn start_test_video_stream(&mut self, cx: &mut Cx, participant_id: &str) {
        log!("Starting test video stream for participant: {}", participant_id);
        self.test_video_participant_id = Some(participant_id.to_string());
        self.test_video_frame_timer = cx.start_interval(1.0 / 30.0);  // ~30fps

        // Also ensure the participant has video enabled
        let list = self.view.participants_list(cx, ids!(participants_list));
        list.update_participant(cx, participant_id, |p| {
            p.is_video_on = true;
        });
    }

    /// Stop continuous test video frames
    pub fn stop_test_video_stream(&mut self, cx: &mut Cx) {
        log!("Stopping test video stream");
        self.test_video_frame_timer = Timer::default();  // Stop the timer
        self.test_video_participant_id = None;
        self.redraw(cx);
    }

    /// Push a test video frame to a participant for debugging
    /// Generates a colored gradient pattern in I420 format
    pub fn push_test_video_frame(&mut self, cx: &mut Cx, participant_id: &str) {
        let width: u32 = 320;
        let height: u32 = 240;

        // Generate I420 test pattern (colored gradient)
        let y_size = (width * height) as usize;
        let uv_size = ((width / 2) * (height / 2)) as usize;

        let mut y_plane = vec![0u8; y_size];
        let mut u_plane = vec![128u8; uv_size];  // Neutral U
        let mut v_plane = vec![128u8; uv_size];  // Neutral V

        // Create a gradient pattern - Y varies horizontally, U/V create color
        // Use time-based offset to animate the pattern
        let time_offset = (Cx::time_now() * 100.0) as u32 % 256;

        for j in 0..height {
            for i in 0..width {
                let y_idx = (j * width + i) as usize;
                // Luminance gradient (bright in center, dark at edges)
                let cx_dist = ((i as i32 - width as i32 / 2).abs() as f32) / (width as f32 / 2.0);
                let cy_dist = ((j as i32 - height as i32 / 2).abs() as f32) / (height as f32 / 2.0);
                let dist = (cx_dist * cx_dist + cy_dist * cy_dist).sqrt().min(1.0);
                let luma = ((1.0 - dist * 0.5) * 200.0 + time_offset as f32) as u8;
                y_plane[y_idx] = luma.wrapping_add(((i + j) % 32) as u8);
            }
        }

        // Create color pattern in UV planes (blue-ish tint that shifts over time)
        for j in 0..(height / 2) {
            for i in 0..(width / 2) {
                let uv_idx = (j * (width / 2) + i) as usize;
                // U controls blue-yellow, V controls red-cyan
                u_plane[uv_idx] = (128u8).wrapping_add((time_offset / 2) as u8).wrapping_add((i * 2) as u8);
                v_plane[uv_idx] = (128u8).wrapping_sub((time_offset / 3) as u8).wrapping_add((j * 2) as u8);
            }
        }

        let pts_ms = (Cx::time_now() * 1000.0) as u64;

        log!("Pushing test video frame to participant {}: {}x{} pts={}ms",
            participant_id, width, height, pts_ms);

        let list = self.view.participants_list(cx, ids!(participants_list));
        list.push_video_frame(
            cx,
            participant_id,
            y_plane,
            u_plane,
            v_plane,
            width,
            height,
            pts_ms,
        );
    }

    /// Update participants list from Matrix call member state events
    fn update_participants_from_call_members(&mut self, cx: &mut Cx, members: &[CallMember]) {
        log!("update_participants_from_call_members: received {} members", members.len());

        // Get the participants list reference
        let list = self.view.participants_list(cx, ids!(participants_list));

        // Snapshot per-participant runtime state that is owned by
        // LiveKit (not by Matrix call-member events) before we wipe
        // the list. Without this snapshot, mute / speaking state would
        // get reset to `false` on every 5-second Matrix refresh,
        // overwriting whatever the LiveKit `TrackMuted` handler had
        // just set. Keyed by participant id — with prefix-match
        // fallback to handle LiveKit's `@user:server.tld:<session>`
        // identities vs. our stored bare `@user:server.tld` ids.
        let previous = list.get_participants();
        let lookup_prev = |id: &str| -> Option<&Participant> {
            previous.iter()
                .find(|p| p.id == id)
                .or_else(|| previous.iter().find(|p| id.starts_with(&p.id)))
                .or_else(|| previous.iter().find(|p| p.id.starts_with(id)))
        };

        // Clear existing participants but preserve video textures
        // Video textures are keyed by participant ID (user_id) and will be matched
        // when participants are re-added with the same IDs
        list.clear(cx);

        // Get current user to exclude self from participants list
        let current_user_id = get_client()
            .and_then(|c| c.session_meta().map(|m| m.user_id.to_string()));
        log!("Current user ID: {:?}", current_user_id);

        // Track added user_ids to avoid duplicates (multiple devices same
        // user). The size of this set is the count of "other" participants
        // — total participants displayed in the header is `len() + 1`
        // (the +1 is self), matching the formula in `update_ui`.
        let mut added_user_ids = std::collections::HashSet::new();

        for member in members {
            // Skip self
            if current_user_id.as_ref() == Some(&member.user_id) {
                continue;
            }

            // Skip if we already added this user (multiple devices)
            if added_user_ids.contains(&member.user_id) {
                log!("Skipping duplicate user: {} (device={})", member.user_id, member.device_id);
                continue;
            }
            added_user_ids.insert(member.user_id.clone());

            let name = member.display_name.clone()
                .unwrap_or_else(|| member.user_id.clone());
            let letter = name.chars().next().unwrap_or('?').to_uppercase().to_string();

            // Use just user_id as the participant ID to match LiveKit identity format
            // LiveKit identity is set from the JWT which uses the Matrix user_id
            let participant_id = member.user_id.clone();

            // Check if this participant already has video texture (from LiveKit video frames)
            let has_video = list.has_video_texture(&participant_id);

            // Restore LiveKit-derived runtime flags (mute, speaking) if
            // we had them before the wipe. Matrix call-member events
            // don't carry this state, so without the snapshot we'd
            // reset it to false on every refresh.
            let prev = lookup_prev(&participant_id);
            let is_muted = prev.map(|p| p.is_muted).unwrap_or(false);
            let is_speaking = prev.map(|p| p.is_speaking).unwrap_or(false);

            let participant = Participant {
                id: participant_id.clone(),
                name,
                avatar_letter: letter,
                is_muted,
                is_speaking,
                is_video_on: has_video,  // Preserve video state from LiveKit
            };

            log!("Adding call member: {} (id={}, video={}, muted={})",
                participant.name, participant_id, has_video, is_muted);
            list.add_participant(cx, participant);
        }

        // Update participant count display. `+ 1` accounts for self,
        // who was excluded from `added_user_ids` above.
        let count = added_user_ids.len() + 1;
        self.view.label(cx, ids!(participant_count))
            .set_text(cx, &format!("{} participant{}", count, if count == 1 { "" } else { "s" }));

        // Force redraw so the new label text actually paints. `update_ui`
        // (the LiveKit-driven sibling) already calls redraw at its end;
        // this function needs its own redraw because it's invoked from
        // a separate action arm and won't be followed by `update_ui` in
        // general.
        self.view.redraw(cx);

        log!("Updated participants panel: {} total ({} others)",
            count, added_user_ids.len());
    }

    /// Connect to LiveKit with the given URL and JWT token
    fn connect_livekit(&mut self, cx: &mut Cx, url: &str, jwt: &str) {
        log!("connect_livekit: url={}", url);
        log!("connect_livekit: jwt length={}, empty={}", jwt.len(), jwt.is_empty());
        if jwt.len() > 20 {
            log!("connect_livekit: jwt starts with: {}", &jwt[..20]);
        }

        if jwt.is_empty() {
            log!("ERROR: JWT token is empty, cannot connect to LiveKit");
            self.call.connection_state = ConnectionState::Disconnected;
            self.update_ui(cx);
            return;
        }

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
    /// Initialize the VoIP screen. `voice_only`: when true, opens in
    /// voice-call mode (no camera preview, avatar tiles, mic-only).
    pub fn initialize(&self, cx: &mut Cx, room_id: OwnedRoomId, voice_only: bool) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.initialize(cx, room_id, voice_only);
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

    /// Add a participant, returns the participant ID
    pub fn add_participant(&self, cx: &mut Cx, name: &str, is_video_on: bool) -> Option<String> {
        self.borrow_mut().map(|mut inner| inner.add_participant(cx, name, is_video_on))
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

    /// Push a test video frame to a participant for debugging
    pub fn push_test_video_frame(&self, cx: &mut Cx, participant_id: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.push_test_video_frame(cx, participant_id);
        }
    }

    /// Start continuous test video frames to a participant
    pub fn start_test_video_stream(&self, cx: &mut Cx, participant_id: &str) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.start_test_video_stream(cx, participant_id);
        }
    }

    /// Stop continuous test video frames
    pub fn stop_test_video_stream(&self, cx: &mut Cx) {
        if let Some(mut inner) = self.borrow_mut() {
            inner.stop_test_video_stream(cx);
        }
    }
}
