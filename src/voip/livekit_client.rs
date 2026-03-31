//! LiveKit client integration for WebRTC

use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

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
}

/// Commands sent from UI to LiveKit client
pub enum LiveKitCommand {
    Connect { url: String },
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
        while let Some(cmd) = cmd_rx.recv().await {
            match cmd {
                LiveKitCommand::Connect { url } => {
                    println!("Connecting to LiveKit: {}", url);
                    if let Ok(mut connected) = is_connected.lock() {
                        *connected = true;
                    }
                    let _ = msg_tx.send(LiveKitMessage::Connected);
                }
                LiveKitCommand::Disconnect => {
                    println!("Disconnecting from LiveKit");
                    if let Ok(mut connected) = is_connected.lock() {
                        *connected = false;
                    }
                    let _ = msg_tx.send(LiveKitMessage::Disconnected);
                }
                LiveKitCommand::SetMicrophoneMuted(muted) => {
                    println!("Set microphone muted: {}", muted);
                }
                LiveKitCommand::SetCameraMuted(muted) => {
                    println!("Set camera muted: {}", muted);
                }
                LiveKitCommand::StartScreenShare => {
                    println!("Starting screen share");
                }
                LiveKitCommand::StopScreenShare => {
                    println!("Stopping screen share");
                }
                LiveKitCommand::PublishVideoFrame(frame) => {
                    println!(
                        "Publishing video frame: {}x{} ({} bytes)",
                        frame.width, frame.height, frame.data.len()
                    );
                }
            }
        }
    }

    pub fn send_command(&self, cmd: LiveKitCommand) {
        if let Some(tx) = &self.command_tx {
            let _ = tx.send(cmd);
        }
    }

    pub fn connect(&self, url: String, _token: String) {
        self.send_command(LiveKitCommand::Connect { url });
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
