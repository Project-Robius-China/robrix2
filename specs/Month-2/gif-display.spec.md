spec: task
name: "Month 2 — GIF: Display AnimatedImage when GIF Bytes are Available"
inherits: project
tags: [month-2, media, image, gif, display]
---

## Intent

When a Matrix `MessageType::Image` event whose mime is `image/gif` has its
full bytes available in `MediaCache` (i.e. `MediaCacheEntry::Loaded`), the
timeline must render via the **standalone `AnimatedImage` Makepad widget**
(`src/shared/animated_image.rs`) so the GIF actually animates. The
`AnimatedImage` widget is the only path that calls
`Image::load_image_from_data_async` for the full file — the existing
`TextOrImage` widget only carries the static-thumbnail path and must not be
used to display loaded GIF bytes. While the bytes are still being fetched
(`MediaCacheEntry::Requested`), `AnimatedImage` shows a "Loading animated
image..." placeholder; on `MediaCacheEntry::Failed`, it shows an inline error
referencing the source `mxc://` URI. The on-disk cache key derived for the
loaded bytes carries a `.gif` suffix when the message body filename ends in
`.gif`, so artifacts in the cache directory are debuggable. This spec is the
sibling of `animated-image.spec.md`, which covers detection (`is_animated_
image_mime`) and the widget's existence; this spec covers the GIF-specific
display path once bytes arrive.

## Decisions

- `populate_image_message_content` (in `src/home/room_screen.rs:4378`) routes
  to `AnimatedImageRef::populate_from_media_source` when
  `is_animated_image_mime(mimetype)` returns `true` AND an `AnimatedImageRef`
  was supplied by the caller. When `mimetype` is `Some("image/gif")` and
  `animated_image_ref` is `Some(_)`, the GIF render path runs — neither
  `text_or_image_ref.show_image` nor `text_or_image_ref.show_html` is called
  for that event.
- `AnimatedImageRef::populate_from_media_source` is the single public entry
  point used by `populate_image_message_content`. It dispatches
  `MediaSource::Plain(mxc)` to `AnimatedImage::populate_from_mxc` and
  `MediaSource::Encrypted` to a placeholder text message (encryption is out
  of scope for this task).
- `AnimatedImage::populate_from_mxc` calls
  `media_cache.try_get_media_or_fetch(&mxc_uri, MediaFormat::File)` and
  matches the returned `MediaCacheEntry`:
  - `Loaded(data)` with `MediaFormat::File` → call `show_image_data` and
    return `true`.
  - `Requested` or `Loaded(_)` with a non-`File` format → call
    `show_text("Loading animated image...")` and return `false`.
  - `Failed(_)` → call `show_text(format!("{body}\n\nFailed to fetch animated
    image from {mxc_uri:?}"))` and return `true` (terminal state).
- `animated_image_cache_key(mxc_uri, body)` is a free function in
  `src/shared/animated_image.rs`. It returns a `PathBuf` of the form
  `robrix_animated_image_<sanitized_mxc>.<ext>` where `<sanitized_mxc>` is
  the mxc URI with every non-ASCII-alphanumeric character replaced by `_`,
  and `<ext>` is one of `gif`, `apng`, `webp` (case-insensitively matched
  from the body's final extension) or the literal fallback `img` when no
  recognised animated extension is present.
- Tests for `animated_image_cache_key` live in `#[cfg(test)] mod
  tests_animated_image_cache_key` inside `src/shared/animated_image.rs`,
  mirroring the `tests_animated_image` module pattern already used in
  `src/utils.rs` for the detection helpers.

## Constraints

- Must NOT call `text_or_image_ref.show_image(...)` when the mime is
  `image/gif` AND `animated_image_ref` is `Some(_)`. The static-thumbnail
  path through `TextOrImage` would freeze the GIF to a single frame.
- Must NOT call `text_or_image_ref.set_visible(cx, true)` while
  `animated_image_ref` is rendering a GIF for the same message; the two
  widgets are mutually exclusive per `populate_image_message_content`'s
  visibility toggle.
- Must NOT alter the `populate_from_mxc` return-value contract: it returns
  `true` exactly when the render reaches a terminal state (`Loaded` decoded
  and displayed, or `Failed`); `Requested` returns `false` so the timeline
  re-runs once bytes arrive.
- Must NOT introduce a new image-format dispatch beyond what
  `makepad_widgets::Image::load_image_from_data_async` already supports —
  decoding GIF bytes is delegated to Makepad's `Image` widget.
- Must NOT modify `MediaCache` or its `MediaCacheEntry` variants; this task
  consumes the existing contract verbatim.

## Boundaries

### Allowed Changes

- src/shared/animated_image.rs   # `animated_image_cache_key` tests, widget body
- src/home/room_screen.rs        # `populate_image_message_content` routing

### Forbidden

- Do not edit `src/media_cache.rs`. The GIF display path uses the existing
  `MediaCache::try_get_media_or_fetch` contract verbatim.
- Do not edit `src/shared/text_or_image.rs`. `AnimatedImage` is a standalone
  widget; `TextOrImage` must remain untouched so the static-image path is
  byte-for-byte unchanged.
- Do not branch on `mime` strings inside `populate_image_message_content` 
  itself — call `is_animated_image_mime` so the behaviour is unit-testable
  (this constraint is shared with `animated-image.spec.md`).

## Completion Criteria

Scenario: Cache key keeps `.gif` extension from lowercase body
  Test:
    Package: robrix
    Filter: test_animated_image_cache_key_keeps_gif_extension
  Level: unit
  Given the body filename `"reaction.gif"`
  And the mxc URI `"mxc://example.org/abc123"`
  When `animated_image_cache_key` is called
  Then the returned path ends with `".gif"`

Scenario: Cache key extension is case-insensitive
  Test:
    Package: robrix
    Filter: test_animated_image_cache_key_lowercases_gif_extension
  Level: unit
  Given the body filename `"REACTION.GIF"`
  And the mxc URI `"mxc://example.org/abc123"`
  When `animated_image_cache_key` is called
  Then the returned path ends with `".gif"`

Scenario: Cache key falls back to `.img` for unknown extension
  Test:
    Package: robrix
    Filter: test_animated_image_cache_key_falls_back_to_img_for_png
  Level: unit
  Given the body filename `"chart.png"`
  And the mxc URI `"mxc://example.org/abc123"`
  When `animated_image_cache_key` is called
  Then the returned path ends with `".img"`

Scenario: Cache key falls back to `.img` when body has no extension
  Test:
    Package: robrix
    Filter: test_animated_image_cache_key_falls_back_to_img_when_no_extension
  Level: unit
  Given the body filename `"justaname"`
  And the mxc URI `"mxc://example.org/abc123"`
  When `animated_image_cache_key` is called
  Then the returned path ends with `".img"`

Scenario: Cache key falls back to `.img` for an empty body
  Test:
    Package: robrix
    Filter: test_animated_image_cache_key_falls_back_to_img_for_empty_body
  Level: unit
  Given the body filename `""`
  And the mxc URI `"mxc://example.org/abc123"`
  When `animated_image_cache_key` is called
  Then the returned path ends with `".img"`

Scenario: Cache key sanitises mxc URI to alphanumerics and underscores
  Test:
    Package: robrix
    Filter: test_animated_image_cache_key_sanitises_mxc_uri
  Level: unit
  Given the body filename `"reaction.gif"`
  And the mxc URI `"mxc://example.org/abc-123_XYZ"`
  When `animated_image_cache_key` is called
  Then the returned path's filename contains only ASCII alphanumerics, underscores, and a single `.`

Scenario: Cache key prefix is the literal `robrix_animated_image_`
  Test:
    Package: robrix
    Filter: test_animated_image_cache_key_prefix_is_stable
  Level: unit
  Given the body filename `"reaction.gif"`
  And the mxc URI `"mxc://example.org/abc123"`
  When `animated_image_cache_key` is called
  Then the returned path's filename starts with `"robrix_animated_image_"`

Scenario: GIF mime routes through is_animated_image_mime
  Test:
    Package: robrix
    Filter: test_is_animated_image_mime_accepts_gif
  Level: unit
  Given the input string `"image/gif"`
  When `is_animated_image_mime` is called
  Then the function returns `true`

## Out of Scope

- APNG and animated-WebP display paths — covered by sibling
  `animated-image.spec.md`. This spec is GIF-only.
- Encrypted media decryption. `MediaSource::Encrypted` currently shows a
  TODO text message via `AnimatedImage::populate_from_media_source`; that
  branch is untouched here.
- Lottie / video-as-image stickers (covered by `MessageType::Sticker`).
- Animation playback controls (pause, scrub, frame stepping).
- Server-side animated thumbnail negotiation; this spec only governs the
  client's local fetch-and-display path for GIFs.
- Memory limits for very large animated files — handled by the existing
  `MediaCache` size policies, unchanged here.
