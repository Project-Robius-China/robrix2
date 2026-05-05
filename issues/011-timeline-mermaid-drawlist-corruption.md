# Issue #011: Timeline Mermaid/diagram interaction corrupts Makepad DrawList during room scrolling

**Date:** 2026-04-27
**Severity:** High (crashes Robrix while scrolling rich OctOS bot replies)
**Status:** Mitigated (timeline renderer made static); follow-up spec opened
**Affected component:** `src/home/room_screen.rs` bot Markdown/Mermaid rendering inside `PortalList`

## Summary

Robrix crashed while scrolling a room containing OctOS bot Markdown replies with Mermaid/diagram content. Static rendering worked, but the initial Mermaid integration copied the interactive `examples/aichat` renderer directly into timeline items. That renderer keeps hit-test areas, pan/zoom state, and continuous `NextFrame` animation for flow dots.

In Robrix, timeline messages live inside a `PortalList`. `PortalList` virtualizes and reuses row widgets while scrolling. Long-lived child widget areas or animation redraws inside those rows can outlive the row's current DrawList generation. When the stale area is used during scrolling, Makepad reports a DrawList generation mismatch and then panics in turtle alignment.

## Symptoms

The crash occurs after Markdown code/table/Mermaid rendering appears visually correct, then the user scrolls the room timeline:

```text
[E] .../platform/src/draw_list.rs:345:13: Drawlist id generation wrong 99 280 279
[E] .../platform/src/draw_list.rs:345:13: Drawlist id generation wrong 99 280 279

thread 'main' panicked at .../draw/src/turtle.rs:2346:37:
index out of bounds: the len is 0 but the index is 0
```

Earlier variants of the same failure showed:

```text
[E] .../platform/src/draw_list.rs:345:13: Drawlist id generation wrong 79 1 0
thread 'main' panicked at .../draw/src/turtle.rs:2340:37:
index out of bounds: the len is 2929 but the index is 2929
```

## Reproduction Context

1. Use Makepad dev branch from `https://github.com/ZhangHanDong/makepad.git`.
2. Render an OctOS bot reply containing Markdown with fenced code, table, and Mermaid/diagram blocks.
3. Let Robrix render the reply through `BotTimelineMarkdown`.
4. Scroll the room timeline containing that message.
5. Robrix panics with DrawList generation mismatch followed by `turtle.rs` index out of bounds.

The crash was observed after adding:

- `streaming-markdown-kit` with the `mermaid` feature
- `makepad-diagram-kit`
- `BotTimelineMermaidSvgView` based on Makepad `examples/aichat`
- `mermaid_block` and `diagram_block` templates inside `BotTimelineMarkdown`

## Root Cause

The `examples/aichat` Mermaid renderer is safe in a non-virtualized chat surface, but it is not safe as an interactive animated child inside Robrix's virtualized `PortalList` timeline.

The unsafe combination is:

1. `BotTimelineMermaidSvgView` stores a rendered `DrawSvg` area.
2. The widget handles mouse hit testing against that area for pan/zoom.
3. The widget schedules repeated `NextFrame` callbacks for flow-dot animation.
4. The containing `PortalList` row is recycled or shifted during timeline scrolling.
5. The old area points at a DrawList generation that has already been replaced.
6. Makepad logs `Drawlist id generation wrong`.
7. Later turtle alignment touches an invalid draw item/index and panics.

This is the same class of problem as Issue #001, but the trigger is different:

- Issue #001: `Dock.load_state()` destroyed DrawLists during state restore.
- Issue #011: an animated/interactive child inside a virtualized timeline row retained stale DrawList state while scrolling.

## Mitigation Applied

The timeline Mermaid renderer was made static:

- Removed timeline Mermaid hit testing and pan/zoom handling.
- Removed timeline Mermaid continuous `NextFrame` animation.
- Removed flow-dot rendering state from the timeline widget.
- Removed extra `new_batch: true` from Markdown dynamic block templates that were nested inside `PortalList` rows.

After this mitigation:

- Code highlighting works.
- Markdown table routing works.
- Mermaid/diagram content renders.
- Room scrolling no longer crashes in the tested flow.

## Product Gap

Static diagrams are stable but not ideal. Users still need a way to inspect complex diagrams with interaction:

- zoom in/out
- drag/pan
- reset view
- show animated flow dots for Mermaid edges
- inspect larger diagrams without the timeline row constraints

## Recommended Solution

Keep timeline rows as static previews and move interactivity into a top-level modal owned by `RoomScreen`.

Recommended architecture:

1. Timeline `mermaid_block` / `diagram_block` remains static and non-animated.
2. Clicking a diagram preview emits a widget action with:
   - diagram kind (`mermaid` or `diagram`)
   - original source text
3. `RoomScreen` catches the action and opens a top-level modal outside the `PortalList`.
4. The modal renders the same source using an interactive renderer:
   - Mermaid: SVG renderer with pan/zoom and optional flow-dot animation
   - Diagram: `DiagramView` with modal-safe sizing and future interaction hooks
5. Closing the modal drops the interactive renderer state before returning to timeline scrolling.

This preserves timeline stability while restoring a richer inspection experience.

## Open Spec

Follow-up implementation contract:

- `specs/task-bot-diagram-modal-renderer.spec.md`

## Related Files

- [src/home/room_screen.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/home/room_screen.rs)
- [issues/001-dock-load-state-drawlist-corruption.md](/Users/zhangalex/Work/Projects/FW/robius/robrix2/issues/001-dock-load-state-drawlist-corruption.md)
- Makepad example reference: `/Users/zhangalex/.cargo/git/checkouts/makepad-05aed5730390fa6f/17210f2/examples/aichat/src/main.rs`
