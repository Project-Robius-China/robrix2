//! 1:1 voice call orchestrator and UI surfaces.
//!
//! Owns the *flow* of a 1:1 voice call: placing a call, ringing,
//! answering, declining, joining the underlying MatrixRTC session,
//! hanging up, and surfacing the result in the room timeline.
//!
//! The orchestrator coordinates three subsystems:
//!
//! * [`super::ringing`] — `m.call.notify` events + ringtone playback.
//! * [`super::livekit_client`] — the actual media transport, reused
//!   from the existing group-call code in voice-only configuration.
//! * UI widgets — [`incoming_call_modal`] for the callee's accept /
//!   decline prompt, plus a reused [`super::voip_screen::VoipScreen`]
//!   for the in-call experience.
//!
//! Only one [`OneOnOneCall`] can be active at a time across the whole
//! app — see [`OneOnOneCallState::is_busy`].

use std::time::SystemTime;

use makepad_widgets::*;
use matrix_sdk::ruma::{OwnedRoomId, OwnedUserId};

pub mod call_flow;
pub mod incoming_call_modal;
pub mod timeline_event;

pub use call_flow::{
    CallOutcome, CallRole, OneOnOneAction, OneOnOneCall, OneOnOneCallState, OneOnOneEvent,
    RING_TIMEOUT,
};

/// UI-level actions emitted by the orchestrator and consumed by widgets
/// in `app.rs` (modal, VoipScreen). Kept distinct from the FSM's
/// internal [`OneOnOneAction`] so the FSM stays I/O-free and the UI
/// only sees the events that actually drive widget state.
#[derive(Clone, Debug)]
pub enum OneOnOneUiAction {
    ShowIncomingModal {
        call_id: String,
        room_id: OwnedRoomId,
        caller: OwnedUserId,
    },
    HideIncomingModal,
    /// The orchestrator has decided to join the SFU in voice-only mode.
    /// VoipScreen handles this by calling `enter_voice_only_call`.
    JoinSfu {
        room_id: OwnedRoomId,
        peer: OwnedUserId,
        role: CallRole,
    },
    /// The orchestrator has decided to tear down the SFU connection.
    LeaveSfu,
    /// A call ended; the app may want to surface a toast.
    CallEnded {
        room_id: OwnedRoomId,
        outcome: CallOutcome,
    },
}

/// Dispatch a single [`OneOnOneAction`] emitted by the FSM. Routes each
/// action to the appropriate subsystem.
///
/// Each match arm re-acquires the [`super::VoipGlobalState`] borrow
/// fresh, so we never hold it across calls into other globals.
pub fn dispatch_action(cx: &mut Cx, action: OneOnOneAction) {
    match action {
        OneOnOneAction::SendRing { room_id, call_id, callee } => {
            crate::sliding_sync::submit_async_request(
                crate::sliding_sync::MatrixRequest::SendCallNotify {
                    room_id,
                    call_id,
                    callee,
                },
            );
        }
        OneOnOneAction::StartDialTone => {
            with_ringtone(cx, |p| p.play_outgoing());
        }
        OneOnOneAction::StartRingtone => {
            with_ringtone(cx, |p| p.play_incoming());
        }
        OneOnOneAction::StopTone => {
            with_ringtone(cx, |p| p.stop());
        }
        OneOnOneAction::ShowIncomingModal { call_id, room_id, caller } => {
            Cx::post_action(OneOnOneUiAction::ShowIncomingModal {
                call_id,
                room_id,
                caller,
            });
        }
        OneOnOneAction::HideIncomingModal => {
            Cx::post_action(OneOnOneUiAction::HideIncomingModal);
        }
        OneOnOneAction::JoinSfu { room_id, peer, role } => {
            Cx::post_action(OneOnOneUiAction::JoinSfu { room_id, peer, role });
        }
        OneOnOneAction::LeaveSfu => {
            Cx::post_action(OneOnOneUiAction::LeaveSfu);
        }
        OneOnOneAction::StartRingTimer { deadline } => {
            // Convert the FSM's absolute deadline into a relative
            // Makepad timeout. If the deadline is already in the past
            // (clock weirdness or a long pause), fire essentially
            // immediately.
            let remaining = deadline
                .duration_since(SystemTime::now())
                .unwrap_or_default()
                .as_secs_f64();
            // Start the timeout first (mutable borrow of cx), then
            // re-borrow cx to stash the timer into the global. Doing
            // this in two phases avoids holding two mutable borrows
            // simultaneously.
            let timer = cx.start_timeout(remaining.max(0.05));
            if cx.has_global::<super::VoipGlobalState>() {
                let state = cx.get_global::<super::VoipGlobalState>();
                state.ring_timer = timer;
            }
        }
        OneOnOneAction::CancelRingTimer => {
            // Same two-phase dance: take the timer value out of the
            // global, then call cx.stop_timer once that borrow is
            // released.
            let timer = if cx.has_global::<super::VoipGlobalState>() {
                let state = cx.get_global::<super::VoipGlobalState>();
                std::mem::take(&mut state.ring_timer)
            } else {
                Timer::default()
            };
            if !timer.is_empty() {
                cx.stop_timer(timer);
            }
        }
        OneOnOneAction::WriteTimelineSummary { room_id, call_id, outcome, duration: _ } => {
            // For v1, the `m.call.notify` event itself is already
            // rendered in the timeline (via TimelineItemContent::
            // RtcNotification — see `populate_rtc_notification_event`
            // in `home/room_screen.rs`). A separate summary event with
            // explicit outcome/duration would require a custom event
            // type and is out of scope for this round; the ended-call
            // outcome is surfaced via the UI toast instead.
            log!(
                "1:1 call ended: room={} call_id={} outcome={:?}",
                room_id, call_id, outcome,
            );
            Cx::post_action(OneOnOneUiAction::CallEnded { room_id, outcome });
        }
    }
}

fn with_ringtone(cx: &mut Cx, f: impl FnOnce(&super::ringing::RingtonePlayer)) {
    if cx.has_global::<super::VoipGlobalState>() {
        let state = cx.get_global::<super::VoipGlobalState>();
        if let Some(player) = state.ringtone_player.as_ref() {
            f(player);
        }
    }
}

/// Returns whether the orchestrator currently holds an in-progress call
/// in the given room. Used by the in-call UI to decide whether to surface
/// the "Hangup" button or treat the room as call-free.
pub fn is_call_active_in_room(cx: &mut Cx, room_id: &matrix_sdk::ruma::RoomId) -> bool {
    if cx.has_global::<super::VoipGlobalState>() {
        let state = cx.get_global::<super::VoipGlobalState>();
        return state.one_on_one.state.room_id()
            .is_some_and(|rid| rid.as_str() == room_id.as_str());
    }
    false
}
