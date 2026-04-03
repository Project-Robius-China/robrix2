//! VoIP screen module for voice/video calls
//!
//! This module provides VoIP functionality including:
//! - Call state management
//! - Camera handling
//! - LiveKit WebRTC integration
//! - Speaking detection
//! - Participants list

use makepad_widgets::*;
use makepad_widgets::makepad_platform::video::VideoInputsEvent;
use makepad_widgets::makepad_platform::permission::PermissionStatus;
use matrix_sdk::ruma::OwnedRoomId;

pub mod call_state;
pub mod camera;
pub mod livekit_client;
pub mod speaking;
pub mod participants_list;
pub mod voip_screen;

pub use voip_screen::VoipScreenWidgetRefExt;
pub use participants_list::{Participant, ParticipantsListWidgetRefExt};
pub use camera::CameraChoice;

/// Actions emitted by VoIP screens
#[derive(Clone, Debug, Default)]
pub enum VoipAction {
    /// Close the VoIP screen and return to the room
    Close(OwnedRoomId),
    #[default]
    None,
}

/// Global VoIP state stored in Makepad's Cx context.
/// This allows camera permission and video inputs events to be captured
/// at app startup before VoipScreen is shown.
#[derive(Default)]
pub struct VoipGlobalState {
    /// Camera permission status (captured at app level)
    pub camera_permission: Option<PermissionStatus>,
    /// Selected camera choice from VideoInputsEvent
    pub camera_choice: Option<CameraChoice>,
    /// Whether video inputs have been requested
    pub video_inputs_requested: bool,
}

impl VoipGlobalState {
    /// Initialize global VoIP state and request permissions/video inputs.
    /// Call this in App::handle_startup.
    pub fn initialize(cx: &mut Cx) {
        // Set global state
        cx.set_global(VoipGlobalState::default());

        // Request camera permission
        log!("VoipGlobalState: Requesting camera permission...");
        cx.request_permission(makepad_widgets::makepad_platform::permission::Permission::Camera);

        // Request video inputs enumeration - this triggers VideoInputsEvent
        log!("VoipGlobalState: Requesting video inputs...");
        cx.video_input(0, |_buf| {});
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
}
