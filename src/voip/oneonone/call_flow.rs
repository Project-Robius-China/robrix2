//! State machine for a 1:1 voice call.
//!
//! See the design doc for the full diagram. In short:
//!
//! ```text
//!  Idle ──► Outgoing ──► Connecting ──► InCall ──► Ended
//!  Idle ──► Incoming ──► Connecting ──► InCall ──► Ended
//!                ▲                                   ▲
//!                └─────── declined / timed-out ──────┘
//! ```
//!
//! The state machine is deliberately a plain data type that takes
//! [`OneOnOneEvent`]s as input and produces a new state plus optional
//! [`OneOnOneAction`]s as output. It owns no I/O. Side effects
//! (sending notify events, starting the ringtone, connecting LiveKit,
//! writing timeline summaries) are performed by the caller in response
//! to emitted actions.
//!
//! Keeping the state machine I/O-free makes it cheap to unit-test and
//! easy to reason about — the call lifecycle is a small finite-state
//! machine, but it interacts with three asynchronous subsystems
//! (Matrix, LiveKit, audio output), and conflating the two is where
//! bugs hide.

use std::time::{Duration, SystemTime};

use matrix_sdk::ruma::{OwnedRoomId, OwnedUserId};

/// How long to ring before giving up on an unanswered call.
/// Matches the value used by Element clients.
pub const RING_TIMEOUT: Duration = Duration::from_secs(45);

/// Whether the local user is the caller or the callee.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallRole {
    Caller,
    Callee,
}

/// Outcome of a 1:1 voice call, recorded in the room timeline.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallOutcome {
    /// Both sides joined the SFU and a hangup ended the call cleanly.
    Completed,
    /// Callee actively declined the incoming ring.
    Declined,
    /// Ring timed out before the callee answered.
    Missed,
    /// Caller hung up before the callee answered.
    Cancelled,
    /// LiveKit / SFU failure after accept; never reached `InCall`.
    FailedToConnect,
    /// Mid-call disconnect — peer connection dropped.
    ConnectionLost,
}

/// The state of the (single) 1:1 voice call across the whole app.
#[derive(Clone, Debug, Default)]
pub enum OneOnOneCallState {
    /// No call in progress.
    #[default]
    Idle,
    /// We are the caller; ring sent; waiting for the callee to join
    /// the MatrixRTC session in this room.
    Outgoing {
        call_id: String,
        room_id: OwnedRoomId,
        peer: OwnedUserId,
        started_at: SystemTime,
        ring_deadline: SystemTime,
    },
    /// We are the callee; an incoming-call modal is on screen and the
    /// ringtone is playing.
    Incoming {
        call_id: String,
        room_id: OwnedRoomId,
        caller: OwnedUserId,
        received_at: SystemTime,
        ring_deadline: SystemTime,
    },
    /// The other party answered (or we accepted) and we are joining
    /// the SFU. Brief; transitions to `InCall` on `LiveKitConnected`
    /// or back to `Ended { FailedToConnect }` on error.
    Connecting {
        call_id: String,
        room_id: OwnedRoomId,
        peer: OwnedUserId,
        role: CallRole,
        connect_started_at: SystemTime,
    },
    /// Active two-way audio. The duration shown in the UI is
    /// `now - connected_at`.
    InCall {
        call_id: String,
        room_id: OwnedRoomId,
        peer: OwnedUserId,
        connected_at: SystemTime,
    },
    /// Terminal-ish: the orchestrator emits the timeline summary
    /// action and immediately resets to `Idle`. This state exists so
    /// the outcome is observable in the FSM output.
    Ended {
        call_id: String,
        outcome: CallOutcome,
        duration: Option<Duration>,
    },
}

impl OneOnOneCallState {
    /// True if a new outbound or inbound call should be rejected
    /// because we're already busy with another one.
    pub fn is_busy(&self) -> bool {
        !matches!(self, Self::Idle | Self::Ended { .. })
    }

    /// Room currently associated with the call, if any.
    pub fn room_id(&self) -> Option<&OwnedRoomId> {
        match self {
            Self::Idle | Self::Ended { .. } => None,
            Self::Outgoing { room_id, .. }
            | Self::Incoming { room_id, .. }
            | Self::Connecting { room_id, .. }
            | Self::InCall { room_id, .. } => Some(room_id),
        }
    }

    /// Stable id for the call (if there is one). Used to correlate
    /// async events (e.g. a peer join arriving after a state change).
    pub fn call_id(&self) -> Option<&str> {
        match self {
            Self::Idle => None,
            Self::Outgoing { call_id, .. }
            | Self::Incoming { call_id, .. }
            | Self::Connecting { call_id, .. }
            | Self::InCall { call_id, .. }
            | Self::Ended { call_id, .. } => Some(call_id.as_str()),
        }
    }
}

/// Inputs to the state machine.
#[derive(Clone, Debug)]
pub enum OneOnOneEvent {
    /// User clicked "Voice call" in a DM room.
    UserPlaceCall { room_id: OwnedRoomId, peer: OwnedUserId },
    /// User clicked Accept on the incoming-call modal.
    UserAccept,
    /// User clicked Decline on the incoming-call modal.
    UserDecline,
    /// User clicked End / Hangup (either in-call or while still ringing
    /// as the caller).
    UserHangup,

    /// An `m.call.notify` ring targeted at us arrived from the ringing
    /// layer.
    IncomingRing { call_id: String, room_id: OwnedRoomId, caller: OwnedUserId },

    /// MatrixRTC session: the peer joined (callee accepted on caller
    /// side; or — symmetrically — caller's join confirms on the callee
    /// side after we sent ours).
    PeerJoinedSession { call_id: String },
    /// MatrixRTC session: the peer left (peer-side hangup or drop).
    PeerLeftSession { call_id: String },

    /// LiveKit: our local connection finished establishing.
    LiveKitConnected,
    /// LiveKit: we disconnected, expected or otherwise.
    LiveKitDisconnected,
    /// LiveKit: a fatal error occurred (auth, SFU unreachable, etc.).
    LiveKitError { reason: String },

    /// Ring timer fired (45 s elapsed without an answer).
    RingTimeout,
}

/// Outputs of the state machine. The orchestrator translates each of
/// these into the appropriate Matrix / LiveKit / audio call.
#[derive(Clone, Debug)]
pub enum OneOnOneAction {
    /// Send an `m.call.notify` ring event to the room.
    SendRing { room_id: OwnedRoomId, call_id: String, callee: OwnedUserId },
    /// Start the outgoing dial tone.
    StartDialTone,
    /// Start the incoming ringtone.
    StartRingtone,
    /// Stop any tone currently playing.
    StopTone,
    /// Show the incoming-call modal.
    ShowIncomingModal { call_id: String, room_id: OwnedRoomId, caller: OwnedUserId },
    /// Hide the incoming-call modal.
    HideIncomingModal,
    /// Start joining the MatrixRTC session in voice-only configuration.
    /// The orchestrator invokes `VoipScreen::enter_voice_only_call`.
    JoinSfu { room_id: OwnedRoomId, peer: OwnedUserId, role: CallRole },
    /// Leave the SFU and tear down the LiveKit connection.
    LeaveSfu,
    /// Start a one-shot timer; on fire it delivers `RingTimeout` back
    /// to the orchestrator.
    StartRingTimer { deadline: SystemTime },
    /// Cancel a previously-started ring timer.
    CancelRingTimer,
    /// Record the call outcome in the room timeline. The orchestrator
    /// constructs the appropriate visual representation in the
    /// timeline (round 6).
    WriteTimelineSummary {
        room_id: OwnedRoomId,
        call_id: String,
        outcome: CallOutcome,
        duration: Option<Duration>,
    },
}

/// The orchestrator state + transition logic.
#[derive(Default)]
pub struct OneOnOneCall {
    pub state: OneOnOneCallState,
}

impl OneOnOneCall {
    /// Apply an event. Returns the set of actions the caller should
    /// dispatch in order. The state machine itself never performs I/O.
    pub fn apply(&mut self, event: OneOnOneEvent) -> Vec<OneOnOneAction> {
        let mut actions = Vec::new();
        let mut next = std::mem::take(&mut self.state);
        next = self.transition(next, event, &mut actions);
        self.state = next;
        actions
    }

    fn transition(
        &self,
        state: OneOnOneCallState,
        event: OneOnOneEvent,
        actions: &mut Vec<OneOnOneAction>,
    ) -> OneOnOneCallState {
        use OneOnOneCallState::*;
        use OneOnOneEvent as E;

        match (state, event) {
            // --- Placing a call (caller) ------------------------------
            (Idle, E::UserPlaceCall { room_id, peer }) => {
                let call_id = new_call_id();
                let now = SystemTime::now();
                let deadline = now + RING_TIMEOUT;
                actions.push(OneOnOneAction::SendRing {
                    room_id: room_id.clone(),
                    call_id: call_id.clone(),
                    callee: peer.clone(),
                });
                actions.push(OneOnOneAction::StartDialTone);
                actions.push(OneOnOneAction::StartRingTimer { deadline });
                Outgoing {
                    call_id,
                    room_id,
                    peer,
                    started_at: now,
                    ring_deadline: deadline,
                }
            }

            // --- Receiving a ring (callee) ----------------------------
            (Idle, E::IncomingRing { call_id, room_id, caller }) => {
                let now = SystemTime::now();
                let deadline = now + RING_TIMEOUT;
                actions.push(OneOnOneAction::ShowIncomingModal {
                    call_id: call_id.clone(),
                    room_id: room_id.clone(),
                    caller: caller.clone(),
                });
                actions.push(OneOnOneAction::StartRingtone);
                actions.push(OneOnOneAction::StartRingTimer { deadline });
                Incoming {
                    call_id,
                    room_id,
                    caller,
                    received_at: now,
                    ring_deadline: deadline,
                }
            }

            // Busy: any new call attempt while we're already in one
            // gets dropped. The orchestrator may emit a synthetic
            // "missed (busy)" timeline entry separately if desired.
            (
                state @ (Outgoing { .. } | Incoming { .. } | Connecting { .. } | InCall { .. }),
                E::IncomingRing { .. } | E::UserPlaceCall { .. },
            ) => state,

            // --- Caller side: callee answers --------------------------
            (Outgoing { call_id, room_id, peer, .. }, E::PeerJoinedSession { call_id: peer_id })
                if peer_id == call_id =>
            {
                actions.push(OneOnOneAction::StopTone);
                actions.push(OneOnOneAction::CancelRingTimer);
                actions.push(OneOnOneAction::JoinSfu {
                    room_id: room_id.clone(),
                    peer: peer.clone(),
                    role: CallRole::Caller,
                });
                Connecting {
                    call_id,
                    room_id,
                    peer,
                    role: CallRole::Caller,
                    connect_started_at: SystemTime::now(),
                }
            }

            // --- Callee side: user accepts ---------------------------
            (Incoming { call_id, room_id, caller, .. }, E::UserAccept) => {
                actions.push(OneOnOneAction::StopTone);
                actions.push(OneOnOneAction::HideIncomingModal);
                actions.push(OneOnOneAction::CancelRingTimer);
                actions.push(OneOnOneAction::JoinSfu {
                    room_id: room_id.clone(),
                    peer: caller.clone(),
                    role: CallRole::Callee,
                });
                Connecting {
                    call_id,
                    room_id,
                    peer: caller,
                    role: CallRole::Callee,
                    connect_started_at: SystemTime::now(),
                }
            }

            // --- Callee side: user declines --------------------------
            (Incoming { call_id, room_id, .. }, E::UserDecline) => {
                actions.push(OneOnOneAction::StopTone);
                actions.push(OneOnOneAction::HideIncomingModal);
                actions.push(OneOnOneAction::CancelRingTimer);
                actions.push(OneOnOneAction::WriteTimelineSummary {
                    room_id,
                    call_id: call_id.clone(),
                    outcome: CallOutcome::Declined,
                    duration: None,
                });
                Ended { call_id, outcome: CallOutcome::Declined, duration: None }
            }

            // --- Ring timeouts ---------------------------------------
            (Outgoing { call_id, room_id, .. }, E::RingTimeout) => {
                actions.push(OneOnOneAction::StopTone);
                actions.push(OneOnOneAction::WriteTimelineSummary {
                    room_id,
                    call_id: call_id.clone(),
                    outcome: CallOutcome::Missed,
                    duration: None,
                });
                Ended { call_id, outcome: CallOutcome::Missed, duration: None }
            }
            (Incoming { call_id, room_id, .. }, E::RingTimeout) => {
                actions.push(OneOnOneAction::StopTone);
                actions.push(OneOnOneAction::HideIncomingModal);
                actions.push(OneOnOneAction::WriteTimelineSummary {
                    room_id,
                    call_id: call_id.clone(),
                    outcome: CallOutcome::Missed,
                    duration: None,
                });
                Ended { call_id, outcome: CallOutcome::Missed, duration: None }
            }

            // --- Connecting -> InCall --------------------------------
            (Connecting { call_id, room_id, peer, .. }, E::LiveKitConnected) => InCall {
                call_id,
                room_id,
                peer,
                connected_at: SystemTime::now(),
            },
            (Connecting { call_id, room_id, .. }, E::LiveKitError { reason: _ }) => {
                actions.push(OneOnOneAction::LeaveSfu);
                actions.push(OneOnOneAction::WriteTimelineSummary {
                    room_id,
                    call_id: call_id.clone(),
                    outcome: CallOutcome::FailedToConnect,
                    duration: None,
                });
                Ended { call_id, outcome: CallOutcome::FailedToConnect, duration: None }
            }

            // --- Hangup paths ----------------------------------------
            (Outgoing { call_id, room_id, .. }, E::UserHangup) => {
                actions.push(OneOnOneAction::StopTone);
                actions.push(OneOnOneAction::CancelRingTimer);
                actions.push(OneOnOneAction::WriteTimelineSummary {
                    room_id,
                    call_id: call_id.clone(),
                    outcome: CallOutcome::Cancelled,
                    duration: None,
                });
                Ended { call_id, outcome: CallOutcome::Cancelled, duration: None }
            }
            (InCall { call_id, room_id, connected_at, .. }, E::UserHangup) => {
                let duration = SystemTime::now().duration_since(connected_at).ok();
                actions.push(OneOnOneAction::LeaveSfu);
                actions.push(OneOnOneAction::WriteTimelineSummary {
                    room_id,
                    call_id: call_id.clone(),
                    outcome: CallOutcome::Completed,
                    duration,
                });
                Ended { call_id, outcome: CallOutcome::Completed, duration }
            }
            (InCall { call_id, room_id, connected_at, .. }, E::PeerLeftSession { call_id: peer_id })
                if peer_id == call_id =>
            {
                let duration = SystemTime::now().duration_since(connected_at).ok();
                actions.push(OneOnOneAction::LeaveSfu);
                actions.push(OneOnOneAction::WriteTimelineSummary {
                    room_id,
                    call_id: call_id.clone(),
                    outcome: CallOutcome::Completed,
                    duration,
                });
                Ended { call_id, outcome: CallOutcome::Completed, duration }
            }
            (InCall { call_id, room_id, connected_at, .. }, E::LiveKitDisconnected) => {
                let duration = SystemTime::now().duration_since(connected_at).ok();
                actions.push(OneOnOneAction::LeaveSfu);
                actions.push(OneOnOneAction::WriteTimelineSummary {
                    room_id,
                    call_id: call_id.clone(),
                    outcome: CallOutcome::ConnectionLost,
                    duration,
                });
                Ended { call_id, outcome: CallOutcome::ConnectionLost, duration }
            }

            // --- Ignore stale / unexpected events --------------------
            // Any combination not matched above is a no-op. This is
            // intentional: lifecycle events from LiveKit / MatrixRTC
            // can race with user actions, and the right answer for
            // most "doesn't apply right now" cases is to drop them.
            (state, _) => state,
        }
    }
}

fn new_call_id() -> String {
    // Lightweight UUID v4 without pulling a new crate: 16 random bytes,
    // formatted as the canonical 8-4-4-4-12 hex string. The randomness
    // source is `rand` (already in Cargo.toml).
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    // Set version (4) and variant (RFC 4122) bits.
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5],
        bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15],
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::{room_id, user_id};

    fn dummy_peer() -> OwnedUserId { user_id!("@bob:example.org").to_owned() }
    fn dummy_room() -> OwnedRoomId { room_id!("!abc:example.org").to_owned() }

    #[test]
    fn place_call_emits_ring_and_dial_tone() {
        let mut call = OneOnOneCall::default();
        let actions = call.apply(OneOnOneEvent::UserPlaceCall {
            room_id: dummy_room(),
            peer: dummy_peer(),
        });
        assert!(matches!(call.state, OneOnOneCallState::Outgoing { .. }));
        assert!(matches!(actions[0], OneOnOneAction::SendRing { .. }));
        assert!(matches!(actions[1], OneOnOneAction::StartDialTone));
        assert!(matches!(actions[2], OneOnOneAction::StartRingTimer { .. }));
    }

    #[test]
    fn incoming_ring_shows_modal_and_plays_tone() {
        let mut call = OneOnOneCall::default();
        let actions = call.apply(OneOnOneEvent::IncomingRing {
            call_id: "id-1".to_string(),
            room_id: dummy_room(),
            caller: dummy_peer(),
        });
        assert!(matches!(call.state, OneOnOneCallState::Incoming { .. }));
        assert!(matches!(actions[0], OneOnOneAction::ShowIncomingModal { .. }));
        assert!(matches!(actions[1], OneOnOneAction::StartRingtone));
        assert!(matches!(actions[2], OneOnOneAction::StartRingTimer { .. }));
    }

    #[test]
    fn decline_records_declined_outcome_and_ends() {
        let mut call = OneOnOneCall::default();
        call.apply(OneOnOneEvent::IncomingRing {
            call_id: "id-1".to_string(),
            room_id: dummy_room(),
            caller: dummy_peer(),
        });
        let actions = call.apply(OneOnOneEvent::UserDecline);
        assert!(matches!(
            call.state,
            OneOnOneCallState::Ended { outcome: CallOutcome::Declined, .. }
        ));
        assert!(actions.iter().any(|a| matches!(
            a,
            OneOnOneAction::WriteTimelineSummary { outcome: CallOutcome::Declined, .. }
        )));
    }

    #[test]
    fn busy_drops_concurrent_incoming_ring() {
        let mut call = OneOnOneCall::default();
        call.apply(OneOnOneEvent::UserPlaceCall {
            room_id: dummy_room(),
            peer: dummy_peer(),
        });
        let before = format!("{:?}", call.state);
        let actions = call.apply(OneOnOneEvent::IncomingRing {
            call_id: "id-2".to_string(),
            room_id: dummy_room(),
            caller: dummy_peer(),
        });
        assert!(actions.is_empty(), "busy ring should be silently dropped");
        assert_eq!(format!("{:?}", call.state), before);
    }

    #[test]
    fn caller_observes_peer_join_then_livekit_connects() {
        let mut call = OneOnOneCall::default();
        call.apply(OneOnOneEvent::UserPlaceCall {
            room_id: dummy_room(),
            peer: dummy_peer(),
        });
        let call_id = call.state.call_id().unwrap().to_string();
        let actions = call.apply(OneOnOneEvent::PeerJoinedSession {
            call_id: call_id.clone(),
        });
        assert!(matches!(call.state, OneOnOneCallState::Connecting { .. }));
        assert!(actions.iter().any(|a| matches!(a, OneOnOneAction::JoinSfu { .. })));

        call.apply(OneOnOneEvent::LiveKitConnected);
        assert!(matches!(call.state, OneOnOneCallState::InCall { .. }));
    }

    #[test]
    fn hangup_in_call_records_completed_with_duration() {
        let mut call = OneOnOneCall::default();
        call.apply(OneOnOneEvent::UserPlaceCall {
            room_id: dummy_room(),
            peer: dummy_peer(),
        });
        let call_id = call.state.call_id().unwrap().to_string();
        call.apply(OneOnOneEvent::PeerJoinedSession { call_id });
        call.apply(OneOnOneEvent::LiveKitConnected);

        let actions = call.apply(OneOnOneEvent::UserHangup);
        assert!(matches!(
            call.state,
            OneOnOneCallState::Ended { outcome: CallOutcome::Completed, .. }
        ));
        assert!(actions.iter().any(|a| matches!(
            a,
            OneOnOneAction::WriteTimelineSummary { outcome: CallOutcome::Completed, .. }
        )));
    }
}
