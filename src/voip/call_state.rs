//! Call state management

use std::collections::HashMap;

/// Connection state for a call
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConnectionState {
    #[default]
    Disconnected,
    Connecting,
    Connected,
    Disconnecting,
}

/// Type of call (voice or video)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CallType {
    Voice,
    #[default]
    Video,
}

/// Represents a participant in the call
#[derive(Debug, Clone)]
pub struct CallParticipant {
    pub user_id: String,
    pub display_name: String,
    pub is_muted: bool,
    pub is_video_on: bool,
    pub is_speaking: bool,
    pub is_screen_sharing: bool,
}

/// Main call state structure
#[derive(Debug, Clone, Default)]
pub struct Call {
    pub call_type: CallType,
    pub connection_state: ConnectionState,
    pub participants: HashMap<String, CallParticipant>,
    pub local_audio_muted: bool,
    pub local_video_muted: bool,
    pub is_screen_sharing: bool,
    pub livekit_url: Option<String>,
    pub livekit_token: Option<String>,
}
