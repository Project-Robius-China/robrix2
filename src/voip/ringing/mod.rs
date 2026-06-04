//! Ringing layer for VoIP calls.
//!
//! Protocol-level "ringing" primitive: send and receive `m.call.notify`
//! events (MSC4075) and play the ringtone. Knows nothing about LiveKit,
//! MatrixRTC session state, or the in-call UI — those live in
//! [`super::oneonone`] (1:1) and [`super::voip_screen`] (group).
//!
//! ## Wiring at a glance
//!
//! * Outbound: the 1:1 orchestrator emits
//!   `MatrixRequest::SendCallNotify { ... }`. The handler in
//!   `sliding_sync.rs` constructs a [`notify_event::ContentType`] and
//!   sends it via `room.send(content)`.
//! * Inbound: [`register_inbound_handler`] is called once at login,
//!   alongside the verification handler. It registers a typed event
//!   handler on the `Client` that funnels every `m.call.notify` through
//!   the singleton [`Ringer`], which in turn posts a
//!   [`RingerAction::IncomingRing`] for the 1:1 orchestrator to consume.

#![allow(deprecated)]

pub mod notify_event;
pub mod ringer;
pub mod ringtone;

pub use notify_event::{
    is_fresh, is_oneonone_ring, is_targeted_at, new_oneonone_ring, ContentType,
    OriginalSyncCallNotifyEvent, RING_FRESHNESS_MS,
};
pub use ringer::{IncomingRing, Ringer, RingerAction};
pub use ringtone::{RingtoneCmd, RingtonePlayer};

use makepad_widgets::log;
use matrix_sdk::{Client, Room};

/// Register the client-side handler that watches for inbound
/// `m.call.notify` events. Idempotent at the application level: the
/// caller is responsible for not calling this twice for the same
/// client (verification handler registration uses the same discipline
/// — see `sliding_sync.rs` for the call site).
pub fn register_inbound_handler(client: Client) {
    client.add_event_handler(
        |ev: OriginalSyncCallNotifyEvent, room: Room, client: Client| async move {
            let Some(local_user) = client.user_id() else {
                // Not logged in (shouldn't happen at this point, but be
                // defensive — the event handler can fire during the brief
                // window where the client is shutting down).
                return;
            };
            let room_id = room.room_id().to_owned();
            let sender = ev.sender.clone();
            let origin_server_ts = ev.origin_server_ts;

            // Lock the global ringer just long enough to filter + dedup.
            // The lock is never held across an .await, so there's no
            // contention risk with other handlers.
            let outcome = {
                let mut ringer = ringer::global().lock().unwrap_or_else(|p| p.into_inner());
                ringer.on_inbound_event(
                    &ev.content,
                    room_id.clone(),
                    sender.clone(),
                    origin_server_ts,
                    local_user,
                )
            };
            if let Some(ring) = outcome {
                log!(
                    "Ringer: surfaced incoming 1:1 ring from {} in room {} (call_id={})",
                    ring.caller, ring.room_id, ring.call_id
                );
            }
        },
    );
}
