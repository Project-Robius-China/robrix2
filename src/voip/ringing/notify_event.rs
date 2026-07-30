//! Thin facade over ruma's `m.call.notify` event (MSC4075).
//!
//! Ruma already provides a typed [`CallNotifyEventContent`] and a matching
//! synced-event alias (`OriginalSyncCallNotifyEvent`). This module re-exports
//! the type so 1:1 call code can refer to it via the `voip::ringing` path,
//! and adds a handful of small helpers (targeting check, freshness check,
//! 1:1 ring constructor) that the orchestrator and `Ringer` use.
//!
//! Note: `m.call.notify` was deprecated in ruma in favour of MSC4143
//! (`m.rtc.notification`). MSC4143 is not yet shipped by mainstream
//! homeservers, and Element X production traffic still uses
//! `m.call.notify`, so for compatibility this code path stays here.
//! Deprecation warnings are silenced locally with `#[allow(deprecated)]`.

#![allow(deprecated)]

use matrix_sdk::ruma::{MilliSecondsSinceUnixEpoch, UserId};
use matrix_sdk::ruma::events::call::notify::{ApplicationType, CallNotifyEventContent};
use matrix_sdk::ruma::events::Mentions;
use matrix_sdk::ruma::events::rtc::notification::NotificationType;

pub use matrix_sdk::ruma::events::call::notify::{
    CallNotifyEventContent as ContentType, OriginalSyncCallNotifyEvent,
};

/// Default freshness window for inbound rings (30 s, matches Element).
pub const RING_FRESHNESS_MS: u64 = 30_000;

/// Build an outgoing 1:1 voice-call ring targeting a single callee.
pub fn new_oneonone_ring(call_id: String, callee: matrix_sdk::ruma::OwnedUserId) -> ContentType {
    let mentions = Mentions::with_user_ids(vec![callee]);
    CallNotifyEventContent::new(
        call_id,
        ApplicationType::Call,
        NotificationType::Ring,
        mentions,
    )
}

/// True when this notify is a 1:1 voice ring (rather than a room-wide
/// group invite or a silent badge).
pub fn is_oneonone_ring(content: &ContentType) -> bool {
    matches!(content.application, ApplicationType::Call)
        && matches!(content.notify_type, NotificationType::Ring)
        && !content.mentions.room
}

/// True when the given user is a target of this notification.
pub fn is_targeted_at(content: &ContentType, user: &UserId) -> bool {
    content.mentions.user_ids.iter().any(|u| u.as_str() == user.as_str())
}

/// True when the event is fresh enough to ring on. The timestamp is taken
/// from the event envelope (`origin_server_ts`) rather than the content,
/// which makes the check resilient to client clock skew.
pub fn is_fresh(origin_server_ts: MilliSecondsSinceUnixEpoch, now_ms: u64, max_age_ms: u64) -> bool {
    let ts_ms: u64 = origin_server_ts.0.into();
    now_ms.saturating_sub(ts_ms) <= max_age_ms
}

#[cfg(test)]
mod tests {
    use super::*;
    use matrix_sdk::ruma::{user_id, UInt};

    #[test]
    fn ring_constructor_targets_callee() {
        let callee = user_id!("@bob:example.org").to_owned();
        let content = new_oneonone_ring("abc-123".to_string(), callee.clone());
        assert!(is_oneonone_ring(&content));
        assert!(is_targeted_at(&content, &callee));
        assert!(!is_targeted_at(&content, user_id!("@alice:example.org")));
    }

    #[test]
    fn freshness_window_drops_stale_events() {
        let ts = MilliSecondsSinceUnixEpoch(UInt::new(1_000).unwrap());
        assert!(is_fresh(ts, 5_000, 10_000));
        assert!(!is_fresh(ts, 20_000, 10_000));
    }
}
