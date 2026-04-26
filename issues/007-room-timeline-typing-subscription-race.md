# Issue 007: Room timeline lifecycle race can drop typing-notice subscriptions and emit missing-state errors

**Date:** 2026-04-12
**Severity:** Major (room lifecycle inconsistency, lost typing subscriptions, noisy BUG logs)
**Status:** Open
**Affected components:** `src/home/room_screen.rs`, `src/sliding_sync.rs`

## Summary

Room open/close transitions can hit a lifecycle race where `RoomScreen` asks the worker to subscribe to typing notices for a room, but the worker cannot find that room in `ALL_JOINED_ROOMS`.

At the same time, `RoomScreen::save_state()` can be called when no `tl_state` is present, producing an additional error log.

Observed logs:

```text
[E] src/home/room_screen.rs:7089:13: Timeline::save_state(): skipping due to missing state, room Some(MainRoom { room_id: "!MA1lotGpuzwWlMuwOR:127.0.0.1:8128" }), Some(Calculated("octosbot"))
[I] src/sliding_sync.rs:2534:25: BUG: room info not found for subscribe to typing notices request, room !MA1lotGpuzwWlMuwOR:127.0.0.1:8128
```

## Symptoms

- Switching rooms or reopening a room can emit noisy `BUG:` logs.
- Typing notice subscription can silently fail for a room.
- `RoomScreen::save_state()` can be asked to persist state even though no timeline state is attached.
- The issue appears around room transitions involving loaded main-room timelines.

## Evidence

### 1. `RoomScreen` subscribes to typing notices whenever a loaded main room is shown

When `show_timeline()` runs for a loaded main room, it immediately asks the worker to subscribe:

- [src/home/room_screen.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/home/room_screen.rs:7008)

Relevant code path:

```rust
if matches!(tl_state.kind, TimelineKind::MainRoom { .. }) {
    submit_async_request(MatrixRequest::SubscribeToTypingNotices {
        room_id: room_id.clone(),
        subscribe: true,
    });
}
```

### 2. The worker assumes the room is already present in `ALL_JOINED_ROOMS`

The request handler immediately looks up room details:

- [src/sliding_sync.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/sliding_sync.rs:2530)

If the room entry is missing, it logs and drops the request:

```rust
let Some(jrd) = all_joined_rooms.get_mut(&room_id) else {
    log!("BUG: room info not found for subscribe to typing notices request, room {room_id}");
    continue;
};
```

### 3. `save_state()` can also run with no active timeline state

When hiding a timeline, `RoomScreen` calls `save_state()` before unsubscribing:

- [src/home/room_screen.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/home/room_screen.rs:7052)

But `save_state()` itself can find `self.tl_state` already missing:

- [src/home/room_screen.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/home/room_screen.rs:7087)

```rust
let Some(mut tl) = self.tl_state.take() else {
    error!("Timeline::save_state(): skipping due to missing state, room {:?}, {:?}", ...);
    return;
};
```

## Suspected Root Cause

This looks like a room/timeline lifecycle ordering bug:

- UI-side `RoomScreen` assumes the room is fully registered when it subscribes.
- Worker-side `ALL_JOINED_ROOMS` can lag behind or already have dropped the room entry.
- `hide_timeline()` / `show_timeline()` transitions can also be invoked when `tl_state` has already been detached, producing the paired `save_state()` error.

So the bug is likely not "typing notices are broken in general", but a race between:

- room UI lifecycle,
- worker room registry lifecycle,
- and `tl_state` attach/detach timing.

## Impact

- Typing notice updates may never arrive for the affected room.
- Logs are noisy and misleadingly labeled as `BUG`.
- Transition ordering around room screens is fragile, which may hide additional state bugs.

## Reproduction Notes

The issue has been observed during normal room navigation with loaded main-room timelines.

One concrete log sequence included:

```text
[I] src/sliding_sync.rs:1903:25: Got 2 members for MainRoom(!NZ8JGjxwqQSn4EWwuj:127.0.0.1:8128)
[E] src/home/room_screen.rs:7089:13: Timeline::save_state(): skipping due to missing state, room Some(MainRoom { room_id: "!MA1lotGpuzwWlMuwOR:127.0.0.1:8128" }), Some(Calculated("octosbot"))
[I] src/sliding_sync.rs:2534:25: BUG: room info not found for subscribe to typing notices request, room !MA1lotGpuzwWlMuwOR:127.0.0.1:8128
```

## Recommended Fix Direction

1. Make the typing-notice subscribe path tolerant to temporary room-registry lag:
   - either delay subscription until room details exist,
   - or queue/retry instead of dropping the request.
2. Tighten `RoomScreen` lifecycle invariants so `hide_timeline()` cannot call `save_state()` after `tl_state` has already been detached.
3. Downgrade or restructure the logging so normal navigation races do not emit `BUG:` unless invariants are truly violated.
4. Add a targeted regression test or scripted repro around room switch / reopen timing.

## Open Questions

- Is `ALL_JOINED_ROOMS` being pruned too early for a room that is still visible in UI?
- Is `show_timeline()` subscribing before the worker has finished registering the main room timeline entry?
- Is there a second caller that invokes `hide_timeline()` after a prior detach already consumed `tl_state`?

## Related Files

- [src/home/room_screen.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/home/room_screen.rs)
- [src/sliding_sync.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/sliding_sync.rs)
