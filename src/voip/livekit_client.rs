//! LiveKit client integration for WebRTC
//!
//! This is currently a stub implementation. When the livekit crate is enabled,
//! it will provide actual WebRTC connectivity.

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

    /// Start the LiveKit client with channels for communication
    pub fn start(&mut self) -> mpsc::UnboundedReceiver<LiveKitMessage> {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (msg_tx, msg_rx) = mpsc::unbounded_channel();

        self.command_tx = Some(cmd_tx);

        let is_connected = self.is_connected.clone();

        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            rt.block_on(async {
                Self::run_event_loop(cmd_rx, msg_tx, is_connected).await;
            });
        });

        msg_rx
    }

    async fn run_event_loop(
        mut cmd_rx: mpsc::UnboundedReceiver<LiveKitCommand>,
        msg_tx: mpsc::UnboundedSender<LiveKitMessage>,
        is_connected: Arc<Mutex<bool>>,
    ) {
        // Stub implementation - simulates LiveKit behavior
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                LiveKitCommand::Connect { url, token: _ } => {
                    log!("LiveKit (stub): Connecting to {}", url);

                    // Simulate successful connection
                    if let Ok(mut connected) = is_connected.lock() {
                        *connected = true;
                    }
                    let _ = msg_tx.send(LiveKitMessage::Connected);
                    SignalToUI::set_ui_signal();

                    // Note: In a real implementation, we would:
                    // 1. Connect to LiveKit using Room::connect(&url, &token, RoomOptions::default())
                    // 2. Listen for RoomEvent::ParticipantConnected, TrackSubscribed, etc.
                    // 3. Extract I420 frames from video tracks using:
                    //    let i420 = frame.buffer.to_i420();
                    //    let (data_y, data_u, data_v) = i420.data();
                    log!("LiveKit (stub): Connection simulated. Enable 'livekit' crate for real WebRTC.");
                }
                LiveKitCommand::Disconnect => {
                    log!("LiveKit (stub): Disconnecting");
                    if let Ok(mut connected) = is_connected.lock() {
                        *connected = false;
                    }
                    let _ = msg_tx.send(LiveKitMessage::Disconnected);
                    SignalToUI::set_ui_signal();
                }
                LiveKitCommand::SetMicrophoneMuted(muted) => {
                    log!("LiveKit (stub): Set microphone muted: {}", muted);
                }
                LiveKitCommand::SetCameraMuted(muted) => {
                    log!("LiveKit (stub): Set camera muted: {}", muted);
                }
                LiveKitCommand::StartScreenShare => {
                    log!("LiveKit (stub): Starting screen share");
                }
                LiveKitCommand::StopScreenShare => {
                    log!("LiveKit (stub): Stopping screen share");
                }
                LiveKitCommand::PublishVideoFrame(frame) => {
                    log!("LiveKit (stub): Publishing video frame: {}x{}", frame.width, frame.height);
                }
            }
        }
    }

    pub fn send_command(&self, cmd: LiveKitCommand) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(cmd);
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
}

impl Default for LiveKitClient {
    fn default() -> Self {
        Self::new()
    }
}
