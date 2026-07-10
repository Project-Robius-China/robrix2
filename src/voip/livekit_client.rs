//! LiveKit client integration for WebRTC
//!
//! This module provides LiveKit WebRTC connectivity for VoIP calls.
//! On macOS and Linux, it uses the real LiveKit SDK.
//! On Android, iOS, and Windows, it provides a stub implementation (VoIP not supported).
//! Windows is excluded due to MSVC runtime library mismatch with webrtc-sys.

use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;
use makepad_widgets::{SignalToUI, log};

use super::call_state::CallParticipant;

/// Video frame data for publishing
#[derive(Clone)]
pub struct VideoFrame {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
    #[allow(dead_code)]
    pub format: VideoFrameFormat,
}

#[derive(Clone, Copy, Debug)]
#[allow(dead_code)]
pub enum VideoFrameFormat {
    Rgb24,
    Rgba32,
}

/// Messages sent from LiveKit client to UI
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum LiveKitMessage {
    Connected,
    Disconnected,
    Error(String),
    ParticipantJoined(CallParticipant),
    ParticipantLeft(String),
    /// I420 video frame received from remote participant
    RemoteVideoFrame {
        participant_id: String,
        /// Y plane data
        y: Vec<u8>,
        /// U plane data
        u: Vec<u8>,
        /// V plane data
        v: Vec<u8>,
        width: u32,
        height: u32,
        /// Presentation timestamp in milliseconds
        pts_ms: u64,
    },
    /// Remote participant's video track subscribed
    VideoTrackSubscribed { participant_id: String },
    /// Remote participant's video track unsubscribed
    VideoTrackUnsubscribed { participant_id: String },
    /// Remote participant changed their microphone (audio track) mute
    /// state. Sent in response to LiveKit `RoomEvent::TrackMuted` /
    /// `TrackUnmuted` events filtered to audio kind. Video mute is
    /// already covered by `VideoTrackSubscribed`/`Unsubscribed`.
    ParticipantAudioMuteChanged {
        participant_id: String,
        is_muted: bool,
    },
}

/// Commands sent from UI to LiveKit client
pub enum LiveKitCommand {
    Connect { url: String, token: String },
    Disconnect,
    SetMicrophoneMuted(bool),
    SetCameraMuted(bool),
    StartScreenShare,
    StopScreenShare,
    PublishVideoFrame(VideoFrame),
    PublishData { payload: Vec<u8>, reliable: bool },
}

/// LiveKit client state
pub struct LiveKitClient {
    command_tx: Option<mpsc::UnboundedSender<LiveKitCommand>>,
    is_connected: Arc<Mutex<bool>>,
}

impl LiveKitClient {
    pub fn new() -> Self {
        Self {
            command_tx: None,
            is_connected: Arc::new(Mutex::new(false)),
        }
    }

    /// Start the LiveKit client with channels for communication.
    ///
    /// The event loop runs on the project's existing Matrix tokio
    /// runtime (via [`crate::sliding_sync::spawn_async_task`]) — we do
    /// NOT spawn our own OS thread or create a second runtime.
    /// Tokio's signal driver and process-wide singletons would conflict
    /// if we did, which is what caused `Runtime::new().unwrap()` to
    /// panic silently before.
    pub fn start(&mut self) -> mpsc::UnboundedReceiver<LiveKitMessage> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();

        self.command_tx = Some(cmd_tx);

        let is_connected = self.is_connected.clone();

        log!("LiveKitClient::start - scheduling event loop on Matrix tokio runtime");
        crate::sliding_sync::spawn_async_task(async move {
            log!("LiveKitClient: event loop task started");
            Self::run_event_loop(cmd_rx, msg_tx, is_connected).await;
            log!("LiveKitClient: event loop task ended");
        });

        msg_rx
    }

    /// Stub implementation for Android/iOS/Windows where LiveKit is not supported.
    #[cfg(any(target_os = "android", target_os = "ios", target_os = "windows"))]
    async fn run_event_loop(
        mut cmd_rx: mpsc::UnboundedReceiver<LiveKitCommand>,
        msg_tx: mpsc::UnboundedSender<LiveKitMessage>,
        _is_connected: Arc<Mutex<bool>>,
    ) {
        // On Android/iOS/Windows, VoIP is not supported. Just drain commands and send error.
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                LiveKitCommand::Connect { .. } => {
                    log!("LiveKit: VoIP not supported on this platform");
                    let _ = msg_tx.send(LiveKitMessage::Error(
                        "VoIP calls are not supported on this platform".to_string()
                    ));
                    SignalToUI::set_ui_signal();
                }
                LiveKitCommand::Disconnect => {
                    let _ = msg_tx.send(LiveKitMessage::Disconnected);
                    SignalToUI::set_ui_signal();
                }
                _ => {
                    // Ignore other commands on unsupported platforms
                }
            }
        }
    }

    /// Full LiveKit implementation for desktop platforms (macOS, Linux).
    #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "windows")))]
    async fn run_event_loop(
        mut cmd_rx: mpsc::UnboundedReceiver<LiveKitCommand>,
        msg_tx: mpsc::UnboundedSender<LiveKitMessage>,
        is_connected: Arc<Mutex<bool>>,
    ) {
        use livekit::prelude::*;
        use livekit::RoomOptions;
        use livekit::webrtc::video_stream::native::NativeVideoStream;
        use livekit::webrtc::prelude::VideoBuffer;
        use futures_util::StreamExt;

        let mut room: Option<Arc<Room>> = None;

        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                LiveKitCommand::Connect { url, token } => {
                    log!("LiveKit: Connecting to {}", url);
                    log!("LiveKit: Token length: {}, starts with: {}",
                        token.len(),
                        if token.len() > 20 { &token[..20] } else { &token });

                    // Validate token is not empty and looks like a JWT (three dot-separated parts)
                    if token.is_empty() {
                        log!("LiveKit: ERROR - Token is empty!");
                        let _ = msg_tx.send(LiveKitMessage::Error("Token is empty".to_string()));
                        SignalToUI::set_ui_signal();
                        continue;
                    }

                    let jwt_parts: Vec<&str> = token.split('.').collect();
                    if jwt_parts.len() != 3 {
                        log!("LiveKit: WARNING - Token doesn't look like a JWT (expected 3 parts, got {})", jwt_parts.len());
                    } else {
                        log!("LiveKit: Token appears to be valid JWT format");
                    }

                    log!("LiveKit: Calling Room::connect with url={} and token length={}", url, token.len());
                    match Room::connect(&url, &token, RoomOptions::default()).await {
                        Ok((r, mut room_events)) => {
                            let room_name = r.name().to_string();
                            let room_sid = String::from(r.sid().await);

                            log!("LiveKit: Connected to room: {} - {}", room_name, room_sid);

                            let r = Arc::new(r);
                            room = Some(r.clone());

                            if let Ok(mut connected) = is_connected.lock() {
                                *connected = true;
                            }
                            let _ = msg_tx.send(LiveKitMessage::Connected);
                            SignalToUI::set_ui_signal();

                            // Spawn task to handle room events
                            let msg_tx_clone = msg_tx.clone();
                            tokio::spawn(async move {
                                while let Some(event) = room_events.recv().await {
                                    log!("LiveKit: Room event: {:?}", event);
                                    match event {
                                        RoomEvent::ParticipantConnected(participant) => {
                                            log!("LiveKit: Participant connected: {}", participant.identity());
                                            let name = participant.name();
                                            let display_name = if name.is_empty() {
                                                participant.identity().to_string()
                                            } else {
                                                name.to_string()
                                            };
                                            let participant_info = CallParticipant {
                                                user_id: participant.identity().to_string(),
                                                display_name,
                                                is_muted: false,
                                                is_video_on: false,
                                                is_speaking: false,
                                                is_screen_sharing: false,
                                            };
                                            // Capture the send Result so a closed
                                            // channel (UI side dropped its
                                            // receiver) becomes visible in the
                                            // log instead of silently dropping
                                            // the message.
                                            match msg_tx_clone.send(LiveKitMessage::ParticipantJoined(participant_info)) {
                                                Ok(_) => log!("LiveKit: Sent ParticipantJoined to UI channel"),
                                                Err(e) => log!("LiveKit: FAILED to send ParticipantJoined (UI receiver closed): {:?}", e),
                                            }
                                            SignalToUI::set_ui_signal();
                                        }
                                        RoomEvent::ParticipantDisconnected(participant) => {
                                            log!("LiveKit: Participant disconnected: {}", participant.identity());
                                            let _ = msg_tx_clone.send(LiveKitMessage::ParticipantLeft(
                                                participant.identity().to_string()
                                            ));
                                            SignalToUI::set_ui_signal();
                                        }
                                        RoomEvent::TrackSubscribed { track, publication: _, participant } => {
                                            let participant_id = participant.identity().to_string();
                                            log!("LiveKit: Track subscribed from {}: kind={:?}", participant_id, track.kind());

                                            if let RemoteTrack::Video(video_track) = track {
                                                log!("LiveKit: Video track subscribed from {}", participant_id);
                                                let _ = msg_tx_clone.send(LiveKitMessage::VideoTrackSubscribed {
                                                    participant_id: participant_id.clone(),
                                                });
                                                SignalToUI::set_ui_signal();

                                                // Start receiving video frames
                                                let msg_tx_video = msg_tx_clone.clone();
                                                let participant_id_clone = participant_id.clone();
                                                let rtc_track = video_track.rtc_track();

                                                tokio::spawn(async move {
                                                    let mut video_stream = NativeVideoStream::new(rtc_track);
                                                    let mut frame_count = 0u64;
                                                    let mut pts_counter = 0u64;

                                                    log!("LiveKit: Starting video frame reception for {}", participant_id_clone);

                                                    while let Some(frame) = video_stream.next().await {
                                                        frame_count += 1;

                                                        // Convert to I420 buffer
                                                        let buffer = frame.buffer.to_i420();
                                                        let width = buffer.width();
                                                        let height = buffer.height();

                                                        // Log first frame and then periodically
                                                        if frame_count == 1 {
                                                            log!("LiveKit: Received first video frame from {}: {}x{}", participant_id_clone, width, height);
                                                        } else if frame_count.is_multiple_of(60) {
                                                            log!("LiveKit: Video frame #{} from {}: {}x{}", frame_count, participant_id_clone, width, height);
                                                        }

                                                        // Get I420 plane data
                                                        let (data_y, data_u, data_v) = buffer.data();

                                                        // Calculate presentation timestamp (30fps assumed)
                                                        pts_counter += 33; // ~33ms per frame at 30fps

                                                        // Send I420 frame directly (let the UI handle conversion)
                                                        let _ = msg_tx_video.send(LiveKitMessage::RemoteVideoFrame {
                                                            participant_id: participant_id_clone.clone(),
                                                            y: data_y.to_vec(),
                                                            u: data_u.to_vec(),
                                                            v: data_v.to_vec(),
                                                            width,
                                                            height,
                                                            pts_ms: pts_counter,
                                                        });
                                                        SignalToUI::set_ui_signal();
                                                    }
                                                    log!("LiveKit: Video stream ended for {} after {} frames", participant_id_clone, frame_count);
                                                });
                                            }
                                        }
                                        RoomEvent::TrackUnsubscribed { track, publication: _, participant } => {
                                            let participant_id = participant.identity().to_string();
                                            log!("LiveKit: Track unsubscribed from {}: kind={:?}", participant_id, track.kind());

                                            if matches!(track, RemoteTrack::Video(_)) {
                                                let _ = msg_tx_clone.send(LiveKitMessage::VideoTrackUnsubscribed {
                                                    participant_id,
                                                });
                                                SignalToUI::set_ui_signal();
                                            }
                                        }
                                        RoomEvent::TrackMuted { participant, publication } => {
                                            let kind = publication.kind();
                                            let participant_id = participant.identity().to_string();
                                            log!(
                                                "LiveKit: TrackMuted fired by {} (kind={:?})",
                                                participant_id, kind
                                            );
                                            if kind == TrackKind::Audio {
                                                match msg_tx_clone.send(
                                                    LiveKitMessage::ParticipantAudioMuteChanged {
                                                        participant_id: participant_id.clone(),
                                                        is_muted: true,
                                                    },
                                                ) {
                                                    Ok(_) => log!("LiveKit: Sent ParticipantAudioMuteChanged(muted=true) for {}", participant_id),
                                                    Err(e) => log!("LiveKit: FAILED to send ParticipantAudioMuteChanged(muted=true) for {}: {:?}", participant_id, e),
                                                }
                                                SignalToUI::set_ui_signal();
                                            } else {
                                                log!("LiveKit: TrackMuted ignored (non-audio kind)");
                                            }
                                        }
                                        RoomEvent::TrackUnmuted { participant, publication } => {
                                            let kind = publication.kind();
                                            let participant_id = participant.identity().to_string();
                                            log!(
                                                "LiveKit: TrackUnmuted fired by {} (kind={:?})",
                                                participant_id, kind
                                            );
                                            if kind == TrackKind::Audio {
                                                match msg_tx_clone.send(
                                                    LiveKitMessage::ParticipantAudioMuteChanged {
                                                        participant_id: participant_id.clone(),
                                                        is_muted: false,
                                                    },
                                                ) {
                                                    Ok(_) => log!("LiveKit: Sent ParticipantAudioMuteChanged(muted=false) for {}", participant_id),
                                                    Err(e) => log!("LiveKit: FAILED to send ParticipantAudioMuteChanged(muted=false) for {}: {:?}", participant_id, e),
                                                }
                                                SignalToUI::set_ui_signal();
                                            } else {
                                                log!("LiveKit: TrackUnmuted ignored (non-audio kind)");
                                            }
                                        }
                                        RoomEvent::Disconnected { reason } => {
                                            log!("LiveKit: Room disconnected: {:?}", reason);
                                            let _ = msg_tx_clone.send(LiveKitMessage::Disconnected);
                                            SignalToUI::set_ui_signal();
                                            break;
                                        }
                                        _ => {}
                                    }
                                }
                            });
                        }
                        Err(e) => {
                            log!("LiveKit: Failed to connect: {}", e);
                            let _ = msg_tx.send(LiveKitMessage::Error(e.to_string()));
                            SignalToUI::set_ui_signal();
                        }
                    }
                }
                LiveKitCommand::Disconnect => {
                    log!("LiveKit: Disconnecting");
                    if let Some(r) = room.take() {
                        r.close().await.ok();
                    }
                    if let Ok(mut connected) = is_connected.lock() {
                        *connected = false;
                    }
                    let _ = msg_tx.send(LiveKitMessage::Disconnected);
                    SignalToUI::set_ui_signal();
                }
                LiveKitCommand::SetMicrophoneMuted(muted) => {
                    log!("LiveKit: Set microphone muted: {}", muted);
                    // Note: Muting requires publishing/unpublishing tracks or using LocalAudioTrack::set_enabled
                    // For now, just log the request. Full implementation requires track management.
                    if let Some(r) = &room {
                        let local = r.local_participant();
                        for (_, publication) in local.track_publications().iter() {
                            if matches!(publication.kind(), TrackKind::Audio) {
                                log!("LiveKit: Audio track found, muted state: {}", publication.is_muted());
                                // publication.mute() is async in newer versions
                            }
                        }
                    }
                }
                LiveKitCommand::SetCameraMuted(muted) => {
                    log!("LiveKit: Set camera muted: {}", muted);
                    // Note: Muting requires publishing/unpublishing tracks or using LocalVideoTrack::set_enabled
                    // For now, just log the request. Full implementation requires track management.
                    if let Some(r) = &room {
                        let local = r.local_participant();
                        for (_, publication) in local.track_publications().iter() {
                            if matches!(publication.kind(), TrackKind::Video) {
                                log!("LiveKit: Video track found, muted state: {}", publication.is_muted());
                                // publication.mute() is async in newer versions
                            }
                        }
                    }
                }
                LiveKitCommand::StartScreenShare => {
                    log!("LiveKit: Starting screen share");
                    // Screen sharing requires platform-specific implementation
                    // For now, just log the request
                }
                LiveKitCommand::StopScreenShare => {
                    log!("LiveKit: Stopping screen share");
                }
                LiveKitCommand::PublishVideoFrame(frame) => {
                    log!("LiveKit: Publishing video frame: {}x{}, format: {:?}, data_len: {}",
                        frame.width, frame.height, frame.format, frame.data.len());
                    // Video frame publishing requires creating a video track source
                    // For now, this is a placeholder for future implementation
                }
                LiveKitCommand::PublishData { payload, reliable } => {
                    if let Some(r) = &room {
                        let data_packet = livekit::DataPacket {
                            payload,
                            reliable,
                            ..Default::default()
                        };

                        match r.local_participant().publish_data(data_packet).await {
                            Ok(_) => {
                                log!("LiveKit: Published data packet");
                            }
                            Err(e) => {
                                log!("LiveKit: Failed to publish data: {}", e);
                            }
                        }
                    } else {
                        log!("LiveKit: Cannot publish data: not connected to room");
                    }
                }
            }
        }
    }

    pub fn send_command(&self, cmd: LiveKitCommand) {
        // Log the variant tag (without payload, for brevity) so we can
        // see which commands fire from the UI side and whether the
        // command channel is open.
        let tag = match &cmd {
            LiveKitCommand::Connect { .. } => "Connect",
            LiveKitCommand::Disconnect => "Disconnect",
            LiveKitCommand::SetMicrophoneMuted(_) => "SetMicrophoneMuted",
            LiveKitCommand::SetCameraMuted(_) => "SetCameraMuted",
            LiveKitCommand::StartScreenShare => "StartScreenShare",
            LiveKitCommand::StopScreenShare => "StopScreenShare",
            LiveKitCommand::PublishVideoFrame(_) => "PublishVideoFrame",
            LiveKitCommand::PublishData { .. } => "PublishData",
        };
        match &self.command_tx {
            None => log!("LiveKitClient::send_command({}) — command_tx is None; start() never ran", tag),
            Some(tx) => match tx.send(cmd) {
                Ok(_) => log!("LiveKitClient::send_command({}) — queued", tag),
                Err(e) => log!("LiveKitClient::send_command({}) — channel closed: {:?}", tag, e),
            },
        }
    }

    pub fn connect(&self, url: String, token: String) {
        self.send_command(LiveKitCommand::Connect { url, token });
    }

    pub fn disconnect(&self) {
        self.send_command(LiveKitCommand::Disconnect);
    }

    pub fn set_microphone_muted(&self, muted: bool) {
        self.send_command(LiveKitCommand::SetMicrophoneMuted(muted));
    }

    pub fn set_camera_muted(&self, muted: bool) {
        self.send_command(LiveKitCommand::SetCameraMuted(muted));
    }

    pub fn publish_video_frame(&self, frame: VideoFrame) {
        self.send_command(LiveKitCommand::PublishVideoFrame(frame));
    }

    pub fn publish_data(&self, payload: Vec<u8>, reliable: bool) {
        self.send_command(LiveKitCommand::PublishData { payload, reliable });
    }
}

impl Default for LiveKitClient {
    fn default() -> Self {
        Self::new()
    }
}
