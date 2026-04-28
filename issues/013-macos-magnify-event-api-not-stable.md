# Issue #013: macOS magnify event API is not stable enough for Robrix release builds

**Date:** 2026-04-28
**Severity:** Medium (pinch zoom integration blocked; scroll/button zoom still works)
**Status:** Open
**Affected component:** Makepad macOS platform events and Robrix diagram modal zoom handling

## Summary

Robrix attempted to handle a newly added Makepad platform event:

```rust
Event::Magnify(magnify_event)
```

This was intended to support macOS trackpad pinch-to-zoom in the Mermaid/diagram preview modal. The Makepad git checkout was locally patched with a `MagnifyEvent` and an `Event::Magnify` variant, and `cargo check` could see the event.

However, `cargo build --release` and `cargo run --release` failed when Robrix matched on `Event::Magnify`:

```text
error[E0599]: no variant or associated item named `Magnify` found for enum `makepad_widgets::Event` in the current scope
    --> src/home/room_screen.rs:1722:23
     |
1722 |         if let Event::Magnify(magnify_event) = event
     |                       ^^^^^^^ variant or associated item not found in `makepad_widgets::Event`
```

## Current Mitigation

Robrix no longer directly matches `Event::Magnify` in `src/home/room_screen.rs`.

The diagram modal still supports:

- scroll-wheel zoom,
- `-` / `Reset` / `+` header controls,
- drag/pan,
- static timeline preview + interactive modal rendering.

The helper for magnify zoom factor is kept test-only until the platform event API becomes stable.

## Root Cause Hypothesis

The magnify support exists only as a local Makepad checkout patch, not as a stable committed API in the pinned Makepad dependency. This creates an unreliable state:

- `cargo check --release` can validate against fresh metadata,
- `cargo build --release` / `cargo run --release` can still fail when compiling Robrix against the exported `makepad_widgets::Event`,
- the Robrix source becomes dependent on an API that may disappear when Cargo refreshes the git checkout.

The correct long-term fix is not to keep a Robrix-only assumption about `Event::Magnify`. The event must first become a stable Makepad platform API.

## Recommended Fix

Move the magnify event support into the Makepad source branch used by Robrix:

1. Add `MagnifyEvent` to Makepad platform event definitions.
2. Add `Event::Magnify(MagnifyEvent)` to the public event enum.
3. Re-export `MagnifyEvent` through Makepad platform/widget public APIs.
4. On macOS, translate `NSEventTypeMagnify` into `Event::Magnify`.
5. Use the current `NSEvent` location for the event position, not only `last_mouse_pos`.
6. Update Robrix to match `Event::Magnify` only after the pinned Makepad commit includes the API.

## Verification Plan

1. Run:

```bash
cargo check --release
cargo build --release
```

2. Confirm both succeed without local uncommitted Makepad checkout patches.
3. Run Robrix and open a Mermaid/diagram modal.
4. Confirm trackpad pinch zoom works in the modal.
5. Confirm scroll-wheel zoom and header zoom controls still work.
6. Confirm timeline scrolling after closing the modal does not reintroduce DrawList corruption.

## Related Files

- [src/home/room_screen.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/home/room_screen.rs)
- [issues/012-diagram-modal-scroll-zoom-hit-area-offset.md](/Users/zhangalex/Work/Projects/FW/robius/robrix2/issues/012-diagram-modal-scroll-zoom-hit-area-offset.md)
- Makepad platform event enum: `/Users/zhangalex/.cargo/git/checkouts/makepad-05aed5730390fa6f/17210f2/platform/src/event/event.rs`
- Makepad macOS event backend: `/Users/zhangalex/.cargo/git/checkouts/makepad-05aed5730390fa6f/17210f2/platform/src/os/apple/macos/macos_app.rs`
