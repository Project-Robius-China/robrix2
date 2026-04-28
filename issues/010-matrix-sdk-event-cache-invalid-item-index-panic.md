# Issue #010: Matrix SDK event cache panics with InvalidItemIndex after timeline update

**Date:** 2026-04-24
**Severity:** High (can crash Robrix during normal message send / timeline update)
**Status:** Open
**Affected component:** Matrix Rust SDK event cache (`matrix-sdk/src/event_cache/caches/room/state.rs`) and Robrix timeline usage

## Summary

Robrix can panic in a Tokio runtime worker after sending a message or after receiving a bot reply. The panic comes from the Matrix Rust SDK event cache while removing an event from a linked chunk:

```text
ERROR matrix_sdk::event_cache::caches::event_linked_chunk: error=The item index is invalid: `94`

thread 'tokio-runtime-worker' panicked at .../matrix-sdk/src/event_cache/caches/room/state.rs:718:14:
failed to remove an event: InvalidItemIndex { index: 94 }
```

The same failure was observed with nearby indexes (`90`, `92`, `94`) during the agent-to-app E2E testing session.

## Symptoms

- User sends a message in Robrix to `@octosbot:127.0.0.1:8128`
- Robrix logs that the message was sent successfully
- Timeline/read-receipt/unread processing continues briefly
- Matrix SDK event cache logs `InvalidItemIndex`
- A Tokio runtime worker panics

Observed send path:

```text
Sending message to MainRoom(!hQ9r6oQFxxUX8VBi4i:127.0.0.1:8128) ...
Sent explicit-room message to MainRoom(!hQ9r6oQFxxUX8VBi4i:127.0.0.1:8128).
```

Observed crash:

```text
2026-04-24T07:22:02.555648Z ERROR matrix_sdk::event_cache::caches::event_linked_chunk: error=The item index is invalid: `94`

thread 'tokio-runtime-worker' panicked at .../matrix-sdk/src/event_cache/caches/room/state.rs:718:14:
failed to remove an event: InvalidItemIndex { index: 94 }
```

## Reproduction Context

This occurred while testing local OctOS agent-to-app news-card routing:

1. Run local Palpo homeserver on `127.0.0.1:8128`
2. Run local OctOS Matrix appservice gateway for `@octosbot:127.0.0.1:8128`
3. Start Robrix and restore account `@alex:127.0.0.1:8128`
4. Open room `!hQ9r6oQFxxUX8VBi4i:127.0.0.1:8128` (`octos-public`)
5. Send a mention message such as:

```text
@octosbot 今天有什么科技新闻
```

6. Robrix successfully sends the Matrix message
7. Timeline/event-cache processing panics with `InvalidItemIndex`

The failure also appeared after automatic pagination/read receipt/unread state updates:

```text
Automatically paginating timeline to fill viewport ...
Completed backwards pagination request ..., hit start of timeline? yes
Updating room ..., marked unread false --> false, unread messages 1 --> 0
```

## Current Assessment

This is separate from the agent-to-app application-card logic.

During the same session OctOS also returned a deterministic resolver error notice (`服务暂不可用`) while the resolver/provider path was being debugged. That text reply can trigger a timeline update, but the Robrix crash itself is in the Matrix SDK event cache while removing an event by index.

## Root Cause Hypothesis

The Matrix SDK event linked chunk state appears to keep or receive a stale item index during event removal. When timeline updates, pagination, read receipts, or redactions reorder or mutate the cache, `state.rs` attempts to remove an event at an index that is no longer valid for the current linked chunk.

Needs confirmation against the upstream Matrix Rust SDK version currently pinned in this workspace:

```text
/Users/zhangalex/.cargo/git/checkouts/matrix-rust-sdk-51f00540bf6ffb2d/627563b
```

## Impact

- Normal chat send / receive can panic the runtime worker
- Agent-to-app E2E testing is noisy because Robrix may crash independently of whether OctOS sent a valid `org.octos.app` envelope
- This can mask the actual producer/renderer result

## Short-Term Workaround

- Restart Robrix to clear the in-memory event cache
- Avoid repeated pagination + sends in the same busy room while debugging agent-to-app
- Prefer verifying OctOS envelope production from gateway logs before relying on Robrix UI

## Recommended Investigation

1. Search upstream Matrix Rust SDK issues / commits for `InvalidItemIndex`, `event_linked_chunk`, and `failed to remove an event`.
2. Check whether the pinned commit `627563b` has a newer fix.
3. If no upstream fix exists, capture a `RUST_BACKTRACE=1` crash and report upstream.
4. In Robrix, consider whether local timeline pagination/read-receipt handling can avoid triggering the problematic remove path, but do not paper over an SDK invariant bug without evidence.

## Related Files

- [src/sliding_sync.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/sliding_sync.rs:2827)
- [src/home/room_screen.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/home/room_screen.rs:5528)
- Matrix SDK checkout: `/Users/zhangalex/.cargo/git/checkouts/matrix-rust-sdk-51f00540bf6ffb2d/627563b/crates/matrix-sdk/src/event_cache/caches/room/state.rs`
