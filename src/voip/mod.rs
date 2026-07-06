//! VoIP screen module for voice/video calls
//!
//! This module provides VoIP functionality including:
//! - Call state management
//! - Camera handling
//! - LiveKit WebRTC integration
//! - Speaking detection
//! - Participants list
//! - Token caching for OpenID and LiveKit JWT

use makepad_widgets::*;
use makepad_widgets::makepad_platform::video::VideoInputsEvent;
use makepad_widgets::makepad_platform::permission::PermissionStatus;
use matrix_sdk::ruma::OwnedRoomId;

pub mod call_state;
pub mod camera;
pub mod livekit_client;
pub mod pip_overlay;
pub mod remote_video_session;
pub mod speaking;
pub mod participants_list;
pub mod token_cache;
pub mod voip_screen;

// 1:1 voice call submodules. `ringing` is a protocol-level layer (m.call.notify
// per MSC4075 + ringtone playback) that is intentionally independent of the
// LiveKit/MatrixRTC media stack so it can later be reused for group-call ringing.
// `oneonone` owns the 1:1-specific call-flow state machine and UI surfaces.
pub mod ringing;
pub mod oneonone;

pub use voip_screen::VoipScreenWidgetRefExt;
pub use participants_list::{Participant, ParticipantsListWidgetRefExt};
pub use camera::CameraChoice;
pub use token_cache::{CachedOpenIdToken, CachedLiveKitJwt, VoipTokenState};
pub use pip_overlay::PipVoipOverlayWidgetRefExt;

/// Represents a call member from Matrix state events
#[derive(Clone, Debug)]
pub struct CallMember {
    pub user_id: String,
    pub device_id: String,
    pub display_name: Option<String>,
}

/// Information about a participant in an active call (for PiP display)
#[derive(Clone, Debug, Default)]
pub struct ParticipantInfo {
    pub user_id: String,
    pub display_name: String,
    pub avatar_letter: String,
}

/// State of an active VoIP call (stored in VoipGlobalState for PiP access)
#[derive(Clone, Debug, Default)]
pub struct ActiveCallState {
    /// The room ID where the call is happening
    pub room_id: Option<OwnedRoomId>,
    /// Connection state description
    pub status_text: String,
    /// Whether we are in lobby or in an active call
    pub in_call: bool,
    /// Whether the local microphone is muted
    pub mic_muted: bool,
    /// Whether the local camera is off
    pub camera_muted: bool,
    /// Whether screen sharing is active
    pub screen_sharing: bool,
    /// Information about the local participant
    pub local_participant: ParticipantInfo,
    /// Number of remote participants
    pub participant_count: usize,
}

/// Actions emitted by VoIP screens
#[derive(Clone, Debug, Default)]
pub enum VoipAction {
    /// Close the VoIP screen and return to the room
    Close(OwnedRoomId),
    /// Join the call (triggers the join_call_button click)
    JoinCall,
    /// Notification that call member state was sent (or failed)
    CallMemberStateSent {
        room_id: OwnedRoomId,
        success: bool,
    },
    /// Call members list updated from Matrix state events
    CallMembersUpdated {
        room_id: OwnedRoomId,
        members: Vec<CallMember>,
    },
    /// OpenID token fetched from Matrix
    OpenIdTokenFetched {
        room_id: OwnedRoomId,
        access_token: String,
        token_type: String,
        matrix_server_name: String,
        expires_in: u64,
    },
    /// LiveKit JWT fetched
    LiveKitJwtFetched {
        room_id: OwnedRoomId,
        url: String,
        jwt: String,
    },
    /// LiveKit connection failed
    LiveKitConnectionFailed {
        room_id: OwnedRoomId,
        error: String,
    },
    /// Test action: Add a participant
    TestAddParticipant {
        name: String,
        is_video_on: bool,
    },
    /// Test action: Toggle participant video
    TestToggleParticipantVideo {
        id: String,
    },
    /// Test action: Remove a participant
    TestRemoveParticipant {
        id: String,
    },
    /// Test action: Clear all participants
    TestClearParticipants,
    /// Test action: Toggle participants sidebar
    TestToggleParticipantsSidebar,
    /// Test action: Push a test video frame to a participant
    TestPushVideoFrame {
        participant_id: String,
    },
    /// Test action: Start continuous test video frames to a participant
    TestStartVideoStream {
        participant_id: String,
    },
    /// Test action: Stop continuous test video frames
    TestStopVideoStream,
    /// Show the PiP overlay for an active call
    ShowPip { room_id: OwnedRoomId },
    /// Hide the PiP overlay
    HidePip,
    /// Toggle microphone from PiP
    PipMicToggle { room_id: OwnedRoomId },
    /// Toggle camera from PiP
    PipCameraToggle { room_id: OwnedRoomId },
    /// Toggle screen share from PiP
    PipScreenShareToggle { room_id: OwnedRoomId },
    /// Hangup from PiP
    PipHangup { room_id: OwnedRoomId },
    /// Return to the VoIP tab from clicking on PiP
    ReturnToVoipTab { room_id: OwnedRoomId },
    #[default]
    None,
}

/// Global VoIP state stored in Makepad's Cx context.
/// This allows camera permission and video inputs events to be captured
/// at app startup before VoipScreen is shown.
/// Also stores cached tokens for OpenID and LiveKit JWT.
#[derive(Default)]
pub struct VoipGlobalState {
    /// Camera permission status (captured at app level)
    pub camera_permission: Option<PermissionStatus>,
    /// Selected camera choice from VideoInputsEvent
    pub camera_choice: Option<CameraChoice>,
    /// Whether video inputs have been requested
    pub video_inputs_requested: bool,
    /// Cached OpenID token (valid for any room, tied to user session)
    pub cached_openid_token: Option<CachedOpenIdToken>,
    /// Cached LiveKit JWTs (per-room, since JWTs are room-specific)
    pub cached_livekit_jwts: Vec<CachedLiveKitJwt>,
    /// Active call state for PiP overlay display
    pub active_call: Option<ActiveCallState>,
    /// 1:1 voice-call orchestrator state machine. Drives ring → answer →
    /// in-call → hangup. Distinct from `active_call` above (which is the
    /// MatrixRTC group-call concept) — the 1:1 flow uses this FSM as the
    /// single source of truth, and updates `active_call` once it reaches
    /// the InCall state so the PiP overlay can render.
    pub one_on_one: oneonone::OneOnOneCall,
    /// Ringtone player handle (rodio-backed on desktop). Initialized on
    /// app startup. `None` until [`VoipGlobalState::initialize`] runs.
    pub ringtone_player: Option<ringing::RingtonePlayer>,
    /// Outgoing/incoming ring timeout timer. Active only in the
    /// `Outgoing` and `Incoming` states. Held here (not on a widget)
    /// because the FSM lives at the app level and survives screen
    /// navigation.
    pub ring_timer: Timer,
}

impl VoipGlobalState {
    /// Initialize global VoIP state and request permissions/video inputs.
    /// Call this in App::handle_startup.
    pub fn initialize(cx: &mut Cx) {
        // Set global state with a ringtone player. The audio thread spins
        // up lazily on the first play, so this is cheap if no call ever
        // happens.
        let state = VoipGlobalState {
            ringtone_player: Some(ringing::RingtonePlayer::spawn()),
            ..Default::default()
        };
        cx.set_global(state);

        // Request camera permission
        log!("VoipGlobalState: Requesting camera permission...");
        cx.request_permission(makepad_widgets::makepad_platform::permission::Permission::Camera);

        // Request video inputs enumeration - this triggers VideoInputsEvent
        log!("VoipGlobalState: Requesting video inputs...");
        cx.video_input(0, |_buf| {});
    }

    /// Apply a 1:1 call event through the orchestrator and dispatch the
    /// resulting actions. This is the single entry point for the 1:1
    /// flow — the voice-call button, the incoming-call modal, the
    /// ring-timeout timer, and the inbound `m.call.notify` handler all
    /// funnel through here.
    pub fn apply_call_event(cx: &mut Cx, event: oneonone::OneOnOneEvent) {
        // Step 1: run the FSM in a short borrow.
        let actions = if cx.has_global::<VoipGlobalState>() {
            cx.get_global::<VoipGlobalState>().one_on_one.apply(event)
        } else {
            return;
        };
        // Step 2: dispatch each action. Each arm re-acquires the borrow
        // as needed so we never hold it across an await or a recursive
        // call into another global.
        for action in actions {
            oneonone::dispatch_action(cx, action);
        }
    }

    /// Handle camera permission result. Call from App's event handler.
    pub fn handle_permission_result(cx: &mut Cx, status: PermissionStatus) {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            log!("VoipGlobalState: Camera permission result: {:?}", status);
            state.camera_permission = Some(status);
        }
    }

    /// Handle video inputs event. Call from App's event handler.
    pub fn handle_video_inputs(cx: &mut Cx, ev: &VideoInputsEvent) {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            log!("VoipGlobalState: VideoInputs event with {} cameras", ev.descs.len());
            state.video_inputs_requested = true;

            if ev.descs.is_empty() {
                log!("VoipGlobalState: No cameras found");
                state.camera_choice = None;
            } else {
                state.camera_choice = camera::CameraManager::pick_camera_choice(ev);
                if let Some(ref choice) = state.camera_choice {
                    log!("VoipGlobalState: Selected camera: {} ({}x{} {:?})",
                        choice.name, choice.width, choice.height, choice.pixel_format);
                } else {
                    log!("VoipGlobalState: No suitable camera format found");
                }
            }
        }
    }

    /// Get camera permission from global state
    pub fn get_camera_permission(cx: &mut Cx) -> Option<PermissionStatus> {
        if cx.has_global::<VoipGlobalState>() {
            cx.get_global::<VoipGlobalState>().camera_permission
        } else {
            None
        }
    }

    /// Get camera choice from global state
    pub fn get_camera_choice(cx: &mut Cx) -> Option<CameraChoice> {
        if cx.has_global::<VoipGlobalState>() {
            cx.get_global::<VoipGlobalState>().camera_choice.clone()
        } else {
            None
        }
    }

    /// Store a cached OpenID token in global state
    pub fn store_openid_token(cx: &mut Cx, token: CachedOpenIdToken) {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            log!("VoipGlobalState: Storing OpenID token (expires in {} seconds)", token.remaining_seconds());
            state.cached_openid_token = Some(token);
        }
    }

    /// Get a valid cached OpenID token from global state
    pub fn get_valid_openid_token(cx: &mut Cx) -> Option<CachedOpenIdToken> {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            if let Some(ref token) = state.cached_openid_token {
                if token.is_valid() {
                    log!("VoipGlobalState: Using cached OpenID token ({} seconds remaining)", token.remaining_seconds());
                    return Some(token.clone());
                } else {
                    log!("VoipGlobalState: Cached OpenID token expired, clearing");
                    state.cached_openid_token = None;
                }
            }
        }
        None
    }

    /// Store a cached LiveKit JWT in global state
    pub fn store_livekit_jwt(cx: &mut Cx, jwt: CachedLiveKitJwt) {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            log!("VoipGlobalState: Storing LiveKit JWT for room {} (expires in {} seconds)",
                jwt.room_id, jwt.remaining_seconds());
            // Remove any existing JWT for this room
            state.cached_livekit_jwts.retain(|j| j.room_id != jwt.room_id);
            // Add the new JWT
            state.cached_livekit_jwts.push(jwt);
            // Clean up expired JWTs
            state.cached_livekit_jwts.retain(|j| j.is_valid());
        }
    }

    /// Get a valid cached LiveKit JWT for the given room from global state
    pub fn get_valid_livekit_jwt(cx: &mut Cx, room_id: &OwnedRoomId) -> Option<CachedLiveKitJwt> {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            // Clean up expired JWTs first
            state.cached_livekit_jwts.retain(|j| j.is_valid());
            // Find a valid JWT for this room
            if let Some(jwt) = state.cached_livekit_jwts.iter().find(|j| j.is_valid_for_room(room_id)) {
                log!("VoipGlobalState: Using cached LiveKit JWT for room {} ({} seconds remaining)",
                    room_id, jwt.remaining_seconds());
                return Some(jwt.clone());
            }
        }
        None
    }

    /// Get the token state for persistence (to be called when saving app state)
    pub fn get_token_state(cx: &mut Cx) -> VoipTokenState {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            VoipTokenState {
                cached_openid_token: state.cached_openid_token.clone(),
                cached_livekit_jwts: state.cached_livekit_jwts.clone(),
            }
        } else {
            VoipTokenState::default()
        }
    }

    /// Restore token state from persistence (to be called when loading app state)
    pub fn restore_token_state(cx: &mut Cx, token_state: VoipTokenState) {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            // Only restore valid tokens
            if let Some(ref token) = token_state.cached_openid_token {
                if token.is_valid() {
                    log!("VoipGlobalState: Restoring cached OpenID token ({} seconds remaining)", token.remaining_seconds());
                    state.cached_openid_token = Some(token.clone());
                }
            }
            // Restore valid JWTs
            for jwt in token_state.cached_livekit_jwts {
                if jwt.is_valid() {
                    log!("VoipGlobalState: Restoring cached LiveKit JWT for room {} ({} seconds remaining)",
                        jwt.room_id, jwt.remaining_seconds());
                    state.cached_livekit_jwts.push(jwt);
                }
            }
        }
    }

    /// Check if there is an active call for the given room
    pub fn is_call_active(cx: &mut Cx, room_id: &OwnedRoomId) -> bool {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            if let Some(ref active) = state.active_call {
                return active.in_call && active.room_id.as_ref() == Some(room_id);
            }
        }
        false
    }

    /// Update the active call state
    pub fn update_active_call(cx: &mut Cx, call: ActiveCallState) {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            state.active_call = Some(call);
        }
    }

    /// Clear the active call state
    pub fn clear_active_call(cx: &mut Cx) {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            log!("VoipGlobalState: Clearing active call state");
            state.active_call = None;
        }
    }

    /// Get the active call state
    pub fn get_active_call(cx: &mut Cx) -> Option<ActiveCallState> {
        if cx.has_global::<VoipGlobalState>() {
            let state = cx.get_global::<VoipGlobalState>();
            return state.active_call.clone();
        }
        None
    }
}
