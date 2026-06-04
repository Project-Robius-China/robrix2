//! Inbound ring tracker.
//!
//! The [`Ringer`] is the single sink for inbound `m.call.notify` events.
//! Sliding sync registers a client event handler that decodes each event
//! and calls into [`Ringer::on_inbound_event`]. The ringer validates,
//! dedupes by `call_id`, filters out stale and self-sent events, and on
//! accept emits a [`RingerAction::IncomingRing`] via
//! [`makepad_widgets::Cx::post_action`].
//!
//! The ringer state (the dedup set) is kept behind a process-wide mutex
//! so the matrix-sdk handler (which runs on a tokio task) can hand events
//! to it directly without needing a channel.

use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use makepad_widgets::Cx;
use matrix_sdk::ruma::{MilliSecondsSinceUnixEpoch, OwnedRoomId, OwnedUserId, UserId};

use super::notify_event::{
    is_fresh, is_oneonone_ring, is_targeted_at, ContentType, RING_FRESHNESS_MS,
};

/// How long a `call_id` is remembered for dedup purposes.
const DEDUP_RETENTION: Duration = Duration::from_secs(120);

/// A validated, deduped, fresh incoming ring ready for the 1:1
/// orchestrator (or future group-call code) to act on.
#[derive(Clone, Debug)]
pub struct IncomingRing {
    pub call_id: String,
    pub room_id: OwnedRoomId,
    pub caller: OwnedUserId,
    /// Event envelope timestamp (ms since Unix epoch).
    pub origin_server_ts: u64,
}

/// Action emitted by the [`Ringer`] for the rest of the app to observe.
#[derive(Clone, Debug)]
pub enum RingerAction {
    IncomingRing(IncomingRing),
}

/// Tracks which inbound rings we've already surfaced.
pub struct Ringer {
    seen_calls: HashMap<String, Instant>,
}

impl Default for Ringer {
    fn default() -> Self {
        Self::new()
    }
}

impl Ringer {
    pub fn new() -> Self {
        Self { seen_calls: HashMap::new() }
    }

    /// Process an inbound `m.call.notify` event. Returns `Some` if the
    /// event passes all filters and the caller has already been notified
    /// via [`Cx::post_action`]. Returns `None` (silently) if it's
    /// stale, a duplicate, not targeted at us, or sent by us.
    pub fn on_inbound_event(
        &mut self,
        content: &ContentType,
        room_id: OwnedRoomId,
        sender: OwnedUserId,
        origin_server_ts: MilliSecondsSinceUnixEpoch,
        local_user: &UserId,
    ) -> Option<IncomingRing> {
        // Filter 1: only 1:1 voice rings for now. Silent notifications
        // and group-call invites flow through different code paths.
        if !is_oneonone_ring(content) {
            return None;
        }

        // Filter 2: ignore events we sent ourselves (sliding sync
        // re-delivers our own events on startup).
        if sender.as_str() == local_user.as_str() {
            return None;
        }

        // Filter 3: must be targeting us specifically.
        if !is_targeted_at(content, local_user) {
            return None;
        }

        // Filter 4: must be fresh. Stale events are typically historical
        // events being replayed during a sync catch-up — ringing for
        // those would be very confusing.
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        if !is_fresh(origin_server_ts, now_ms, RING_FRESHNESS_MS) {
            return None;
        }

        // Filter 5: dedupe by call_id. Periodically gc stale entries so
        // the map can't grow unbounded.
        self.gc_seen_calls();
        if self.seen_calls.contains_key(&content.call_id) {
            return None;
        }
        self.seen_calls.insert(content.call_id.clone(), Instant::now());

        let envelope_ts: u64 = origin_server_ts.0.into();
        let ring = IncomingRing {
            call_id: content.call_id.clone(),
            room_id,
            caller: sender,
            origin_server_ts: envelope_ts,
        };
        Cx::post_action(RingerAction::IncomingRing(ring.clone()));
        Some(ring)
    }

    fn gc_seen_calls(&mut self) {
        let cutoff = Instant::now() - DEDUP_RETENTION;
        self.seen_calls.retain(|_, &mut seen_at| seen_at >= cutoff);
    }
}

/// Process-wide singleton ringer. Lives behind a [`Mutex`] so the
/// matrix-sdk event handler (running on tokio) can dispatch directly
/// without round-tripping through a channel.
pub fn global() -> &'static Mutex<Ringer> {
    static RINGER: OnceLock<Mutex<Ringer>> = OnceLock::new();
    RINGER.get_or_init(|| Mutex::new(Ringer::new()))
}
