# Issue #014: Bot Markdown streaming renders partial prefixes instead of full snapshots

**Date:** 2026-05-05
**Severity:** Medium (visible rendering degradation for streaming bot replies)
**Status:** Open
**Affected component:** `src/home/room_screen.rs` bot Markdown streaming path and `src/home/streaming_animation.rs`

## Summary

Bot replies that contain rich Markdown currently stream with poor visual quality. During generation, Robrix can feed incomplete text prefixes into the Markdown renderer. This causes visible instability while the message is still live:

- headings/lists/tables reflow as syntax becomes complete,
- code fences render as plain or broken blocks until later chunks arrive,
- cursor/sanitized Markdown output can appear inside partially parsed structures,
- rich bot cards redraw more than necessary.

The intended design already exists in the code: rich Markdown bot replies should render the latest full Matrix snapshot directly, while plain text replies keep the local typewriter reveal.

## Current Evidence

The code has a `render_full_target` mode in `StreamingAnimState`:

```rust
/// Whether this message should render the full current snapshot directly
/// instead of the local typewriter prefix. Useful for markdown-rich bot replies
/// where partial prefixes degrade rendering quality and cost.
pub render_full_target: bool,
```

However, the room timeline update path currently forces this mode off:

```rust
streaming_update_requires_content_invalidation(state, &new_text, live, false);
state.set_render_full_target(false);
```

The draw path also computes `render_full_snapshot`, then immediately resets the state to `false` and still slices the target text by `displayed_byte_offset`:

```rust
let render_full_snapshot = should_render_streaming_full_snapshot(body, formatted.as_ref(), sender_is_bot);
state.set_render_full_target(false);

let visible_end = state.displayed_byte_offset.min(state.target_text.len());
bot_streaming_markdown_display(&state.target_text[..visible_end], ...);
```

This contradicts the nearby comment:

```rust
// - markdown-rich bot replies render the latest full snapshot directly
// - plain text keeps the local typewriter prefix with cursor
```

## Likely Root Cause

The rich Markdown full-snapshot mode was introduced but not wired through both phases of the streaming lifecycle:

1. Timeline update detection does not classify the updated event as bot/rich Markdown before updating `StreamingAnimState`.
2. `streaming_update_requires_content_invalidation()` receives `false`, so content is not invalidated when the same text switches render mode.
3. `state.set_render_full_target(false)` disables the state mode even when `should_render_streaming_full_snapshot()` returns true.
4. The Markdown streaming branch still renders `target_text[..displayed_byte_offset]`, so it behaves like the plain text typewriter path.

## Recommended Fix

Keep the two streaming modes explicit:

1. In the timeline update loop, derive `render_full_target` from:
   - whether the sender is a bot (`is_timeline_sender_bot()`), and
   - whether the message body has rich Markdown syntax or HTML formatted body.
2. Pass that value to `streaming_update_requires_content_invalidation()`.
3. Store it with `state.set_render_full_target(render_full_target)`.
4. When starting a new streaming state, set the same mode immediately.
5. In the draw path:
   - rich Markdown bot streaming should call `bot_streaming_markdown_display(&state.target_text, state.is_live)`,
   - plain text streaming should continue using `state.fill_display_buffer()`.

This preserves the typewriter effect for simple text while avoiding invalid partial Markdown parsing for rich bot replies.

## Verification Plan

1. Add or update unit tests around the state-mode transition:
   - rich Markdown bot update sets `render_full_target = true`,
   - plain text streaming remains `render_full_target = false`,
   - content invalidation fires when render mode changes.
2. Run targeted tests for `room_screen` / streaming helpers.
3. Manually test an OctOS bot reply containing:
   - `##` heading,
   - fenced code block,
   - list or table,
   - final non-live update.
4. Confirm Markdown appears as the latest full snapshot during streaming and the final reply has no streaming cursor.

## Related Files

- [src/home/room_screen.rs](/Users/zhangalex/Work/Projects/fw/robrix2/src/home/room_screen.rs)
- [src/home/streaming_animation.rs](/Users/zhangalex/Work/Projects/fw/robrix2/src/home/streaming_animation.rs)
