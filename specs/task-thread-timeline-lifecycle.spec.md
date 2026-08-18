spec: task
name: "Thread Timeline Lifecycle — Close The Backend Timeline And Subscriber Task When The Last Thread View Closes"
inherits: project
tags: [bugfix, lifecycle, thread, sliding-sync, memory]
estimate: 1d
---

## Intent

Fix GitHub issue #308: every opened thread creates a focused `Timeline`, a
`timeline_subscriber_handler` task and two channels in
`JoinedRoomDetails::thread_timelines`, but closing the thread's tab (desktop) or
popping its view (mobile) never removes them. In a long session, opening and
closing many distinct threads retains timelines, cached events and tasks until
the whole room/account is torn down.

The desired behavior is:

- when the **last** UI view of a thread closes, its backend entry is removed
  and its subscriber task terminates;
- a thread still shown by another view (mobile deep navigation) is not closed;
- reopening a closed thread creates a fresh, working timeline;
- a close that races with an in-flight creation never leaves an orphaned entry;
- existing whole-room cleanup (leave / kick / ban / account switch / logout)
  still clears every thread timeline.

## Constraints

- Keep `MatrixRequest::CreateThreadTimeline` unchanged; add `CloseThreadTimeline { room_id, thread_root_event_id }`
- Do not change `timeline_subscriber_handler`'s protocol (`TimelineRequest`, `is_timeline_open`) or matrix-sdk usage
- Do not introduce new dependencies (`proptest` is already a dev-dependency)
- Do not change the per-room `TIMELINE_ROOM_GENERATIONS` mechanism; per-thread invalidation is additive
- Do not change Makepad DSL files
- Every UI-side "should this thread be closed now" decision is a pure function with no `Cx`, so it is unit- and property-testable

## Decisions

- Worker bookkeeping moves into a pure generic table
  `ThreadTimelineTable<T>` (`src/sliding_sync.rs`) replacing the two fields
  `thread_timelines: HashMap<_, PerTimelineDetails>` and
  `pending_thread_timelines: HashSet<_>` on `JoinedRoomDetails`; API:
  `begin_create(id) -> Option<CreateToken>` (None if live or pending; the token
  carries the id and a monotonically increasing creation generation),
  `finish_create(&token, T) -> Result<(), T>` (Err(T) hands the value back when
  the token is not the *current* pending attempt for its id — closed while
  building, or superseded by a newer `begin_create`), `fail_create(&token) -> bool`
  (only clears the current attempt), `close(id) -> Option<T>` (removes pending
  of any generation and live), `get/get_mut/live_len/pending_len`
- Invariant kept by the table: `live ∩ pending = ∅`, `close(id)` ⇒ `¬live(id) ∧ ¬pending(id)`,
  `finish_create`/`fail_create` act iff their token's generation is the current
  pending generation for that id (so `begin(A) → close(A) → begin(A) → finish(old A)`
  is rejected and the new attempt is untouched — no ABA hijack)
- `MatrixRequest::CloseThreadTimeline` handler calls `table.close(id)`; dropping
  the returned `PerTimelineDetails` aborts its subscriber task (existing `Drop`)
- The create task calls `finish_create`; on `Err(details)` it drops the built
  timeline and logs that the thread was closed while building
- UI-side decision helpers (pure, in `src/home/room_screen/thread_lifecycle.rs`):
  `thread_kind_of(&SelectedRoom) -> Option<TimelineKind>` and
  `still_referenced(kind, remaining: impl Iterator<Item = &SelectedRoom>) -> bool`;
  `close_if_unreferenced(kind, remaining) -> Option<TimelineKind>` returns the kind to close
- Desktop: `MainDesktopUI::close_tab` and `close_all_tabs` compute
  `close_if_unreferenced` over the tabs that remain open and, for each returned
  kind, call `close_thread_timeline(cx, kind)`
- Mobile: `App` on `StackNavigationAction::Pop` computes it over
  `selected_room ∪ mobile_room_nav_stack` after the pop; on desktop the hidden
  mobile stack is not a consumer (its RoomScreens are not drawn), so desktop
  tab closes ignore it
- `close_thread_timeline(cx, kind)` (in `state.rs`) = `invalidate_timeline_state(kind)`
  + `submit_async_request(MatrixRequest::CloseThreadTimeline {..})`
- Per-thread invalidation: `state::invalidate_timeline_state(kind)` removes the
  cached `TimelineUiState` and records `kind` in an `INVALIDATED_TIMELINES` set;
  `store_timeline_state` drops (instead of storing) a state whose kind is in the
  set and clears the entry; `take_timeline_state(kind)` (used by `show_timeline`)
  removes any stale entry so a fresh show is never dropped on its next hide
- Observability: `pub fn thread_timeline_counts(room_id) -> Option<(usize, usize)>`
  (live, pending) in `sliding_sync.rs`, plus `log!` lines on close and on
  "closed while building"

## Boundaries

### Allowed Changes
- `src/sliding_sync.rs`
- `src/home/room_screen/state.rs`
- `src/home/room_screen/mod.rs`
- `src/home/room_screen/thread_lifecycle.rs`
- `src/home/main_desktop_ui.rs`
- `src/app.rs`
- `specs/task-thread-timeline-lifecycle.spec.md`

### Forbidden
- Do not modify `timeline_subscriber_handler`'s message protocol
- Do not modify matrix-sdk `TimelineBuilder` usage beyond what exists
- Do not touch `TIMELINE_ROOM_GENERATIONS` semantics
- Do not change Makepad DSL files
- Do not run `cargo fmt`

## Acceptance Criteria

<!--
V(t) = number of drawn UI views of thread t; L(t) = t ∈ live ∨ t ∈ pending (backend)
  th-1  live ∩ pending = ∅ ; close(t) ⇒ ¬L(t)                          (table invariants)
  th-2  V(t): 1 → 0 ⇒ CloseThreadTimeline sent ⇒ ¬L(t) ∧ task aborted   (last view closes backend)
  th-3  V(t) > 1 ∧ one view closes ⇒ no CloseThreadTimeline for t        (shared views protected)
  th-4  ¬L(t) then open ⇒ fresh create succeeds                          (reopen)
  th-5  close(t) while pending ⇒ finish_create(old token) is rejected; a newer begin(t)
        is never hijacked or cancelled by an older attempt (ABA)          (race)
  th-6  room removal / logout still clears all                           (existing, unchanged)
-->

### Rule: th-1 — The thread table never holds a closed or double-tracked thread

Scenario: Table lifecycle — begin, finish, close
  Tags: critical
  Test: thread_table_begin_finish_close_lifecycle
  Given an empty `ThreadTimelineTable<u32>`
  When `begin_create(a)` returns `Some(token)`, then `finish_create(&token, 1)`, then `close(a)`
  Then `finish_create` returns Ok and `close` returns Some(1)
  And afterwards `contains(a)` is false and both lengths are 0

Scenario: Table rejects duplicate creation while live or pending
  Test: thread_table_rejects_duplicate_begin
  Given a table where `begin_create(a)` returned `Some(token)` and `a` is pending
  When `begin_create(a)` is called again
  Then it returns None
  And after `finish_create(&token, 1)` a further `begin_create(a)` also returns None

Scenario: Property — table invariants hold under random operation sequences
  Tags: critical
  Test: prop_thread_table_invariants
  Given a random sequence of `begin_create` / `finish_create` / `fail_create` / `close` over a small id alphabet
  And finish/fail may use any token ever issued for that id (stale generations included)
  When the operations are applied to a `ThreadTimelineTable<u32>` and to a generation-tracking reference model
  Then after every step `live ∩ pending = ∅`, `close(id)` leaves id neither live nor pending
  And `finish_create` inserts iff its token is the current pending generation at that moment
  And `fail_create` clears pending iff its token is the current pending generation
  And the table's live/pending sets equal the reference model's

### Rule: th-5 — A close that races a creation never leaves an orphan

Scenario: finish_create after close hands the value back
  Tags: critical
  Test: thread_table_finish_after_close_is_rejected
  Given `begin_create(a)` returned `Some(token)` and then `close(a)` before creation finished
  When `finish_create(&token, 7)` is called
  Then it returns Err(7)
  And `contains(a)` is false
  And `fail_create(&token)` returns false

Scenario: A stale creation cannot hijack or cancel a reopened thread
  Tags: critical
  Test: thread_table_stale_generation_cannot_hijack_reopened_thread
  Given `begin_create(a)` yielded token `old`, then `close(a)`, then `begin_create(a)` yielded token `new`
  When `finish_create(old, 10)` and `fail_create(old)` are called
  Then both are rejected and `a` is still pending for `new`
  And `finish_create(new, 20)` succeeds and `get(a)` is 20
  And a later `finish_create(old, 30)` is rejected leaving 20 live

Scenario: Worker drops a timeline that finished building after its close
  Test: manual_test_close_during_thread_build_logs_and_drops
  Given a slow network and a thread tab opened then immediately closed
  When the thread timeline build completes
  Then the log contains "closed while building" for that thread
  And `thread_timeline_counts(room)` reports (0, 0) for it

### Rule: th-2 — The last view closes the backend

Scenario: Closing the only desktop tab of a thread yields a close
  Tags: critical
  Test: close_if_unreferenced_closes_when_no_remaining_view
  Given a closed `SelectedRoom::Thread { room R, thread T }`
  And the remaining open rooms are `[JoinedRoom R, Thread { R, U }]`
  When `close_if_unreferenced` is evaluated
  Then it returns Some(TimelineKind::Thread { R, T })

Scenario: Closing a non-thread never yields a close
  Test: close_if_unreferenced_ignores_non_threads
  Given a closed `SelectedRoom::JoinedRoom { R }` (or Invited / Space)
  When `close_if_unreferenced` is evaluated with any remaining set
  Then it returns None

Scenario: Closed thread tab removes the backend entry
  Test: manual_test_desktop_thread_tab_close_removes_timeline
  Given a joined room with `thread_timeline_counts(room) == (0, 0)`
  When the user opens thread T in a tab, waits for it to load, then closes the tab
  Then the log contains "Closed thread timeline" for T
  And `thread_timeline_counts(room)` returns to (0, 0)

### Rule: th-3 — Shared views are protected

Scenario: A thread still in the mobile stack is not closed
  Tags: critical
  Test: close_if_unreferenced_keeps_thread_still_referenced
  Given a popped `Thread { R, T }`
  And the remaining stack still contains `Thread { R, T }` (deep navigation)
  When `close_if_unreferenced` is evaluated
  Then it returns None

Scenario: Property — a close fires exactly when the last reference disappears
  Tags: critical
  Test: prop_close_fires_iff_last_reference_removed
  Given a random sequence of push/pop over a small set of rooms and threads
  When after each pop `close_if_unreferenced(popped, remaining)` is evaluated
  Then it returns Some iff the popped item is a thread that no longer appears in the remaining sequence
  And it never returns Some for a non-thread

Scenario: Mobile deep navigation keeps the shared thread alive
  Test: manual_test_mobile_deep_nav_shared_thread_not_closed
  Given mobile layout, navigation room R → thread T → room R → thread T
  When the user goes back once
  Then thread T's timeline is still live (no "Closed thread timeline" log yet)
  And after going back to the first T and back again the log shows exactly one close

### Rule: th-4 — Reopen after close is fresh

Scenario: Invalidated cached state is dropped, fresh state is kept
  Tags: critical
  Test: invalidated_timeline_state_is_dropped_and_fresh_state_is_stored
  Given a thread kind K with a stored `TimelineUiState`
  When `invalidate_timeline_state(K)` is called
  Then the stored state is gone
  And a state for K stored afterwards without an intervening `take_timeline_state(K)` is dropped
  And after `take_timeline_state(K)` a new state for K is stored normally

Scenario: Reopening a closed thread works
  Test: manual_test_reopen_closed_thread_creates_fresh_timeline
  Given thread T was opened and closed (counts back to (0, 0))
  When the user opens T again
  Then the timeline loads and shows messages
  And `thread_timeline_counts(room)` is (1, 0)

### Rule: th-6 — Whole-room cleanup still clears everything

Scenario: Leaving a room with open threads clears its thread table
  Test: manual_test_leave_room_clears_thread_timelines
  Given a room with two thread tabs open
  When the user leaves the room
  Then all its tabs close and no thread timeline log lines or counts remain for that room

Scenario: Stress — many opens and closes keep counts bounded
  Test: manual_test_thread_open_close_stress_bounded
  Given a room with at least 20 threads
  When the user opens and closes each thread once
  Then `thread_timeline_counts(room)` ends at (0, 0)
  And the number of "Closed thread timeline" log lines equals the number of opened threads

## Out Of Scope

- Reference-counting the hidden mobile stack while in desktop layout (its RoomScreens are not drawn; a later breakpoint switch re-creates the timeline on show)
- Closing main-room timelines when their tabs close
- Bounding the size of `TIMELINE_STATES` for main rooms
