//! Pure decision helpers for the thread-timeline lifecycle
//! (spec `task-thread-timeline-lifecycle`, Rules th-2 / th-3).
//!
//! A thread's backend timeline (`sliding_sync::ThreadTimelineTable`) must be
//! closed exactly when the *last* drawn UI view of that thread goes away —
//! never earlier (mobile deep navigation can hold the same thread twice) and
//! never for non-thread rooms. These functions take the closed/popped
//! `SelectedRoom` and whatever views remain, and answer "which timeline kind,
//! if any, should be closed now". They hold no `Cx` and touch no globals, so
//! they are unit- and property-tested; the callers in `MainDesktopUI` /
//! `App` only feed them the right sets.

use crate::{app::SelectedRoom, sliding_sync::TimelineKind};

/// The thread timeline kind of `room`, or `None` for anything that is not a thread.
pub fn thread_kind_of(room: &SelectedRoom) -> Option<TimelineKind> {
    match room {
        SelectedRoom::Thread { room_name_id, thread_root_event_id } => Some(TimelineKind::Thread {
            room_id: room_name_id.room_id().clone(),
            thread_root_event_id: thread_root_event_id.clone(),
        }),
        _ => None,
    }
}

/// `true` if any of `remaining` still shows the thread `kind`.
pub fn still_referenced<'a>(
    kind: &TimelineKind,
    remaining: impl IntoIterator<Item = &'a SelectedRoom>,
) -> bool {
    remaining
        .into_iter()
        .any(|room| thread_kind_of(room).as_ref() == Some(kind))
}

/// Given the view that just closed and the views that remain, returns the
/// thread timeline kind that should be closed now, if any:
/// `Some(kind)` iff `closed` is a thread and no remaining view shows it.
pub fn close_if_unreferenced<'a>(
    closed: &SelectedRoom,
    remaining: impl IntoIterator<Item = &'a SelectedRoom>,
) -> Option<TimelineKind> {
    let kind = thread_kind_of(closed)?;
    if still_referenced(&kind, remaining) {
        None
    } else {
        Some(kind)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::RoomNameId;
    use matrix_sdk::{RoomDisplayName, ruma::{OwnedEventId, OwnedRoomId}};

    fn room(n: u8) -> RoomNameId {
        RoomNameId::new(
            RoomDisplayName::Named(format!("room{n}")),
            OwnedRoomId::try_from(format!("!room{n}:example.org")).unwrap(),
        )
    }
    fn ev(n: u8) -> OwnedEventId {
        OwnedEventId::try_from(format!("$thread{n}")).unwrap()
    }
    fn joined(n: u8) -> SelectedRoom {
        SelectedRoom::JoinedRoom { room_name_id: room(n) }
    }
    fn thread(r: u8, t: u8) -> SelectedRoom {
        SelectedRoom::Thread { room_name_id: room(r), thread_root_event_id: ev(t) }
    }
    fn kind(r: u8, t: u8) -> TimelineKind {
        TimelineKind::Thread { room_id: room(r).room_id().clone(), thread_root_event_id: ev(t) }
    }

    #[test]
    fn close_if_unreferenced_closes_when_no_remaining_view() {
        let remaining = [joined(1), thread(1, 2)];
        assert_eq!(close_if_unreferenced(&thread(1, 1), remaining.iter()), Some(kind(1, 1)));
        // Same thread root in a *different* room is a different thread.
        let remaining = [thread(2, 1)];
        assert_eq!(close_if_unreferenced(&thread(1, 1), remaining.iter()), Some(kind(1, 1)));
        // Nothing remaining at all.
        assert_eq!(close_if_unreferenced(&thread(1, 1), [].iter()), Some(kind(1, 1)));
    }

    #[test]
    fn close_if_unreferenced_ignores_non_threads() {
        let remaining = [thread(1, 1)];
        assert_eq!(close_if_unreferenced(&joined(1), remaining.iter()), None);
        assert_eq!(close_if_unreferenced(&joined(1), [].iter()), None);
        let invited = SelectedRoom::InvitedRoom { room_name_id: room(3) };
        assert_eq!(close_if_unreferenced(&invited, [].iter()), None);
        let space = SelectedRoom::Space { space_name_id: room(4) };
        assert_eq!(close_if_unreferenced(&space, [].iter()), None);
        assert_eq!(thread_kind_of(&joined(1)), None);
    }

    #[test]
    fn close_if_unreferenced_keeps_thread_still_referenced() {
        // Mobile deep navigation: room → thread → room → thread (same thread twice).
        let remaining = [joined(1), thread(1, 1), joined(1)];
        assert_eq!(close_if_unreferenced(&thread(1, 1), remaining.iter()), None);
        assert!(still_referenced(&kind(1, 1), remaining.iter()));
        // Once the other reference is gone too, it closes.
        let remaining = [joined(1), joined(1)];
        assert_eq!(close_if_unreferenced(&thread(1, 1), remaining.iter()), Some(kind(1, 1)));
    }

    mod props {
        use super::*;
        use proptest::prelude::*;

        #[derive(Clone, Debug)]
        enum Op { Push(u8, Option<u8>), Pop }

        fn arb_op() -> impl Strategy<Value = Op> {
            prop_oneof![
                3 => (0u8..3, prop::option::of(0u8..3)).prop_map(|(r, t)| Op::Push(r, t)),
                2 => Just(Op::Pop),
            ]
        }
        fn item(r: u8, t: Option<u8>) -> SelectedRoom {
            match t { Some(t) => thread(r, t), None => joined(r) }
        }

        proptest! {
            /// Rule th-3: a close fires iff the popped view is a thread that no
            /// longer appears anywhere in the remaining stack; never for non-threads.
            #[test]
            fn prop_close_fires_iff_last_reference_removed(ops in prop::collection::vec(arb_op(), 1..40)) {
                let mut stack: Vec<SelectedRoom> = Vec::new();
                for op in ops {
                    match op {
                        Op::Push(r, t) => stack.push(item(r, t)),
                        Op::Pop => {
                            let Some(popped) = stack.pop() else { continue };
                            let decision = close_if_unreferenced(&popped, stack.iter());
                            let expected = match thread_kind_of(&popped) {
                                None => None,
                                Some(k) => {
                                    let refs = stack.iter().filter(|s| thread_kind_of(s).as_ref() == Some(&k)).count();
                                    if refs == 0 { Some(k) } else { None }
                                }
                            };
                            prop_assert_eq!(decision, expected);
                        }
                    }
                }
            }
        }
    }
}
