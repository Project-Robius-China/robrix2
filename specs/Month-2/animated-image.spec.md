spec: task
name: "Month 2 — Animated Image (GIF) Rendering"
inherits: project
tags: [month-2, media, image, gif]
---

## Intent

Make `MessageType::Image` events whose underlying file is animated (GIF, APNG,
animated WebP) load the full file via `MediaCache` instead of stopping at the
thumbnail, so the image animates in the timeline. Today
`populate_image_message_content` in `src/home/room_screen.rs:4362` always asks
for the thumbnail first via the existing `text_or_image` widget, which freezes
animations to a single static frame. The fix has two parts: (1) a pure
detection helper — `is_animated_image_mime` — that can be unit-tested in the
`tests_*` module style already used in `src/utils.rs` (`tests_ends_with_href`,
`tests_human_readable_list`), reinforcing the existing test infrastructure;
and (2) a new **standalone Makepad widget** `AnimatedImage` (in
`src/shared/animated_image.rs`) that owns the full-bytes fetch and animation
rendering, kept fully separate from `TextOrImage` so the static thumbnail path
is untouched.

## Decisions

- Add a pure helper `is_animated_image_mime(mime: &str) -> bool` in
  `src/utils.rs` next to `linkify` so it lives beside the other small pure
  utilities.
- The helper returns `true` when `mime.eq_ignore_ascii_case` matches any of
  `"image/gif"`, `"image/apng"`, `"image/webp"`, and `false` otherwise.
  WebP files that turn out to be static are still treated as animated for the
  purpose of fetch-strategy choice — over-fetching one static WebP is cheaper
  than freezing a real animation.
- A second pure helper `is_animated_image_filename(name: &str) -> bool` in
  `src/utils.rs` is used as a fallback only when `mime` is unknown. It checks
  the lowercased extension against `"gif"`, `"apng"`, and `"webp"`.
- `AnimatedImage` is a **standalone Makepad widget** defined in
  `src/shared/animated_image.rs`. It follows the same conventions as
  `src/shared/text_or_image.rs`: a `script_mod! { ... }` block that registers
  the widget on the `ScriptVm`, plus a Rust struct
  `pub struct AnimatedImage` deriving `Script, Widget, ScriptHook` and
  `#[deref] view: View`. It is registered alongside the other shared widgets
  by appending `pub mod animated_image;` and a
  `animated_image::script_mod(vm);` call to `src/shared/mod.rs`.
- `AnimatedImage` does **not** wrap, extend, or compose `TextOrImage`. It owns
  its own `Image` child plus a placeholder `Label` for the pre-load state,
  and it talks to `MediaCache::try_get_media_or_fetch` directly with
  `MediaFormat::File`. Keeping it standalone means the static-thumbnail path
  through `TextOrImage` keeps its current behaviour byte-for-byte.
- Inside `populate_image_message_content`, when `is_animated_image_mime`
  returns `true`, the function instantiates / queries an `AnimatedImage` ref
  (instead of the `text_or_image_ref`) and asks it for the full
  `MediaFormat::File` via `MediaCache::try_get_media_or_fetch`. The function
  returns `false` until the cache reports `MediaCacheEntry::Loaded`, so the
  timeline keeps painting the message as "not fully drawn" and re-runs once
  bytes arrive.
- New tests live in `#[cfg(test)] mod tests_animated_image` inside
  `src/utils.rs`, using `test_is_animated_image_mime_<case>` naming that
  mirrors `test_ends_with_href<n>` and `test_linkify<n>` already in that file.

## Constraints

- Must NOT change the rendering for non-animated images. Calls to
  `populate_image_message_content` for `image/jpeg` or `image/png` must still
  go through the thumbnail-first path that exists today.
- Must NOT add a new image decoder dependency. Animation is delegated to the
  Makepad `Image` widget that `AnimatedImage` already embeds, once the full
  bytes are available.
- Must NOT extend, subclass, or modify `TextOrImage` to render animated
  payloads — `AnimatedImage` is a separate, standalone widget. The static
  thumbnail path through `TextOrImage` stays exactly as it is today.
- Must NOT match by file extension when the MIME type is present and known —
  `is_animated_image_filename` is a strict fallback only.
- Must NOT introduce per-event timers or polling loops; the cache already
  re-emits a `TimelineUpdate::MediaFetched` signal when the bytes arrive.

## Boundaries

### Allowed Changes

- src/utils.rs
- src/home/room_screen.rs
- src/shared/animated_image.rs   # new standalone Makepad widget file
- src/shared/mod.rs               # register the new widget's `script_mod`
- src/shared/image_viewer.rs

### Forbidden

- Do not edit `src/media_cache.rs`. The animated-image task uses the existing
  `MediaCache::try_get_media_or_fetch` contract verbatim.
- Do not edit `src/shared/text_or_image.rs`. `AnimatedImage` is a standalone
  widget; `TextOrImage` must remain untouched so the static-image path is
  byte-for-byte unchanged.
- Do not branch on `mime` strings inside `populate_image_message_content`
  itself — call `is_animated_image_mime` so the behaviour is unit-testable.

## Completion Criteria

Scenario: GIF mime is classified as animated
  Test:
    Package: robrix
    Filter: test_is_animated_image_mime_accepts_gif
  Given the input string `"image/gif"`
  When `is_animated_image_mime` is called
  Then the function returns `true`

Scenario: APNG mime is classified as animated
  Test:
    Package: robrix
    Filter: test_is_animated_image_mime_accepts_apng
  Given the input string `"image/apng"`
  When `is_animated_image_mime` is called
  Then the function returns `true`

Scenario: WebP mime is classified as animated
  Test:
    Package: robrix
    Filter: test_is_animated_image_mime_accepts_webp
  Given the input string `"image/webp"`
  When `is_animated_image_mime` is called
  Then the function returns `true`

Scenario: Mime classification is case-insensitive
  Test:
    Package: robrix
    Filter: test_is_animated_image_mime_is_case_insensitive
  Given the input string `"IMAGE/GIF"`
  When `is_animated_image_mime` is called
  Then the function returns `true`

Scenario: Static image mime is rejected
  Test:
    Package: robrix
    Filter: test_is_animated_image_mime_rejects_static_image
  Given the input string `"image/jpeg"`
  When `is_animated_image_mime` is called
  Then the function returns `false`

Scenario: Empty mime is rejected
  Test:
    Package: robrix
    Filter: test_is_animated_image_mime_rejects_empty_string
  Given the input string `""`
  When `is_animated_image_mime` is called
  Then the function returns `false`

Scenario: Filename fallback detects gif extension
  Test:
    Package: robrix
    Filter: test_is_animated_image_filename_accepts_gif_extension
  Given the input filename `"reaction.GIF"`
  When `is_animated_image_filename` is called
  Then the function returns `true`

Scenario: Filename fallback rejects png extension
  Test:
    Package: robrix
    Filter: test_is_animated_image_filename_rejects_png
  Given the input filename `"chart.png"`
  When `is_animated_image_filename` is called
  Then the function returns `false`

Scenario: Filename without extension is rejected
  Test:
    Package: robrix
    Filter: test_is_animated_image_filename_rejects_no_extension
  Given the input filename `"justaname"`
  When `is_animated_image_filename` is called
  Then the function returns `false`

## Out of Scope

- Lottie / video-as-image stickers (covered by `MessageType::Sticker`, not
  `MessageType::Image`).
- Animation playback controls (pause, scrub).
- Server-side animated thumbnail negotiation; this task only changes the
  client's local fetch strategy.
- Memory limits for very large animated files — handled by the existing
  `MediaCache` size policies, unchanged here.
