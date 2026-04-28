# Issue #012: Diagram modal scroll zoom only works in part of the modal

**Date:** 2026-04-28
**Severity:** Medium (diagram preview usable, but zoom interaction feels unreliable)
**Status:** Open
**Affected component:** Diagram/Mermaid preview modal zoom handling and Makepad macOS scroll/magnify event coordinates

## Summary

The diagram preview modal supports scroll-wheel zoom for Mermaid/diagram inspection. During manual testing, zoom worked, but only when the pointer was in a specific region of the modal. If the modal is mentally divided into four quadrants, scroll zoom appears to work only in the second quadrant.

This makes zoom feel inconsistent: the same scroll gesture can zoom the diagram in one area but do nothing in another visible part of the modal.

## Symptoms

- The modal opens correctly.
- Mermaid/diagram content renders correctly.
- Scroll zoom is active and can work.
- Scroll zoom only triggers when the pointer is over a subset of the visible modal area.
- Moving the pointer to other visible parts of the diagram/modal and scrolling does not zoom.

Observed user-facing behavior:

```text
如果把 modal 界面分成四个象限，滚动只有鼠标在第二象限才生效
```

## Reproduction Context

1. Run Robrix with the OctOS bot Markdown renderer enabled.
2. Ask OctOS bot for Markdown containing Mermaid or diagram content.
3. Click the rendered diagram preview to open the modal.
4. Move the pointer across different areas of the modal.
5. Scroll to zoom.
6. Zoom only responds in one region instead of the whole visible diagram area.

## Current Assessment

This is likely not a Markdown, Mermaid, or diagram rendering problem. Rendering succeeds. The issue is in the event hit testing path for modal zoom.

The current modal zoom path checks whether the raw Makepad scroll/magnify event position is inside the diagram view's last rendered rect:

```rust
if self.last_rect.contains(scroll_event.abs) {
    self.zoom_at(cx, scroll_event.abs, factor);
}
```

If `scroll_event.abs` is stale or computed in a different coordinate space than `last_rect`, hit testing can be offset. The result is exactly this symptom: zoom works in a shifted region rather than over the actual visible diagram.

## Root Cause Hypothesis

The Makepad macOS backend may be using `MacosWindow.last_mouse_pos` for scroll and magnify events instead of the precise `NSEvent` location for the current scroll/magnify event.

That can become inaccurate when:

- the pointer has not recently emitted a mouse-move event,
- a modal changes the active draw area without updating the cached mouse position,
- DPI scaling or window-coordinate conversion differs between mouse and scroll events,
- nested modal layout offsets the visual diagram area from the cached pointer area.

The same class of coordinate mismatch can also affect the newly added macOS magnify gesture path if it shares the same cached-position mechanism.

## Impact

- Diagram zoom is discoverable but unreliable.
- Users may think zoom is broken unless the pointer happens to be in the working region.
- Pinch/scroll modal interaction cannot be considered polished until event coordinates are aligned.

## Short-Term Workaround

Keep the current behavior if it is usable enough for testing:

- Scroll zoom is still functional in the working region.
- Header buttons (`-`, `Reset`, `+`) provide deterministic zoom controls.
- Drag/pan and modal rendering remain independent from this hit-area issue.

## Recommended Fix

Fix the event coordinate source in the Makepad macOS platform layer:

1. For scroll events, derive the pointer position from the current `NSEvent` location instead of relying only on `MacosWindow.last_mouse_pos`.
2. For magnify events, do the same.
3. Reuse the same y-flip, DPI, and window-coordinate conversion path used by mouse move/down events.
4. After platform event coordinates are correct, keep Robrix hit testing simple:

```rust
if self.last_rect.contains(event.abs) {
    zoom_at(event.abs, factor);
}
```

Avoid broadening Robrix hit testing to the whole modal as a workaround unless necessary. That would make zoom fire outside the actual diagram and could interfere with modal controls.

## Verification Plan

1. Open a Mermaid/diagram preview modal.
2. Test scroll zoom in all four quadrants of the visible diagram area.
3. Confirm zoom is anchored at the pointer position rather than the modal center.
4. Confirm header controls are still clickable and do not trigger diagram zoom.
5. Confirm room timeline scrolling remains stable after closing the modal.

## Related Files

- [src/home/room_screen.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/home/room_screen.rs)
- [issues/011-timeline-mermaid-drawlist-corruption.md](/Users/zhangalex/Work/Projects/FW/robius/robrix2/issues/011-timeline-mermaid-drawlist-corruption.md)
- Makepad macOS event backend: `/Users/zhangalex/.cargo/git/checkouts/makepad-05aed5730390fa6f/17210f2/platform/src/os/apple/macos/macos_app.rs`
- Makepad macOS window state: `/Users/zhangalex/.cargo/git/checkouts/makepad-05aed5730390fa6f/17210f2/platform/src/os/apple/macos/macos_window.rs`
