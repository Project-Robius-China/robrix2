spec: task
name: "Month 2 — Video Message Rendering"
inherits: project
tags: [month-2, media, video]
---

## Intent

Replace the "Video playback not yet supported" HTML placeholder produced by
`populate_video_message_content` in `src/home/room_screen.rs:4575` with a
deterministic summary that exposes the video message's filename, mime,
duration, size, and `width x height` dimensions, and that asks the existing
`MediaCache` for a thumbnail so the timeline shows a still preview while the
user decides whether to open the file. Detection, summary, and HTML formatting
are split into pure helpers tested in the `tests_*` style already used in
`src/utils.rs`, so the existing test conventions are reinforced.

## Decisions

- Add a pure helper `summarize_video_message(&VideoMessageEventContent) -> VideoSummary`
  in `src/event_preview.rs`. `VideoSummary` is a `pub struct` with fields
  `filename: String`, `mime: Option<String>`, `duration_secs: Option<f64>`,
  `size_bytes: Option<u64>`, `dimensions: Option<(u64, u64)>`,
  `caption_html: Option<String>`.
- A second pure helper `video_summary_html(&VideoSummary) -> String` produces
  the HTML rendered by `populate_video_message_content`.
- Thumbnail fetch is dispatched through
  `MediaCache::try_get_media_or_fetch(&mxc, MediaFormat::Thumbnail(_))` whenever
  the message's `MediaSource` is `MediaSource::Plain` and `info.thumbnail_source`
  is `None` — reusing the existing thumbnail negotiation in `src/media_cache.rs`.
- Dimensions are formatted as `"{width}x{height}"` exactly as the current
  placeholder does (`src/home/room_screen.rs:4596`), so HTML diffs stay small.
- New tests live in `#[cfg(test)] mod tests_video_summary` inside
  `src/event_preview.rs`, using `test_<feature>_<case>` naming that mirrors
  `tests_human_readable_list` in `src/utils.rs`.

## Constraints

- Must NOT change the signature of `populate_video_message_content`; only its
  body changes.
- Must NOT block the UI thread: `summarize_video_message` is pure, and the
  thumbnail request goes through the existing async `MediaCache` path.
- Must NOT add a video-decoding dependency. Inline playback is out of scope —
  this task only renders metadata + thumbnail.
- Must NOT call `.unwrap()` on `info.width` / `info.height`; missing dimensions
  must produce `dimensions: None`, not panic.

## Boundaries

### Allowed Changes

- src/home/room_screen.rs
- src/event_preview.rs
- src/media_cache.rs
- src/shared/text_or_image.rs

### Forbidden

- Do not invent a new "video" template id; reuse `id!(Message)` and
  `id!(CondensedMessage)` exactly as today (see `src/home/room_screen.rs:4001`).
- Do not duplicate `summarize_audio_message` logic — `VideoSummary` is a
  separate struct with its own helper, sharing nothing but the
  `bytesize::ByteSize` and duration formatting conventions.

## Completion Criteria

Scenario: Video summary captures dimensions when width and height are both set
  Test:
    Package: robrix
    Filter: test_video_summary_full_info
  Given a `VideoMessageEventContent` whose `body` is `"clip.mp4"`
  And whose `info.mimetype` is `Some("video/mp4")`
  And whose `info.width` is `Some(1920_u64.into())`
  And whose `info.height` is `Some(1080_u64.into())`
  And whose `info.duration` is `Some(Duration::from_millis(7_250))`
  And whose `info.size` is `Some(2_048_000_u64.into())`
  When `summarize_video_message` is called
  Then `VideoSummary.dimensions` equals `Some((1920, 1080))`
  And `VideoSummary.duration_secs` equals `Some(7.25)`
  And `VideoSummary.size_bytes` equals `Some(2_048_000)`
  And `VideoSummary.mime` equals `Some("video/mp4".to_string())`

Scenario: Video summary drops dimensions when only one of width or height is set
  Test:
    Package: robrix
    Filter: test_video_summary_partial_dimensions
  Given a `VideoMessageEventContent` whose `info.width` is `Some(1280_u64.into())`
  And whose `info.height` is `None`
  When `summarize_video_message` is called
  Then `VideoSummary.dimensions` equals `None`

Scenario: Video summary handles a missing info block
  Test:
    Package: robrix
    Filter: test_video_summary_missing_info
  Given a `VideoMessageEventContent` whose `info` is `None`
  When `summarize_video_message` is called
  Then `VideoSummary.dimensions` equals `None`
  And `VideoSummary.duration_secs` equals `None`
  And `VideoSummary.size_bytes` equals `None`
  And `VideoSummary.mime` equals `None`

Scenario: Video summary HTML renders dimensions in the WIDTHxHEIGHT shape
  Test:
    Package: robrix
    Filter: test_video_summary_html_includes_dimensions
  Given a `VideoSummary` whose `dimensions` is `Some((640, 480))`
  When `video_summary_html` is called
  Then the returned string contains the substring `"640x480"`

Scenario: Video summary HTML omits the dimensions line when none are known
  Test:
    Package: robrix
    Filter: test_video_summary_html_omits_dimensions_when_none
  Given a `VideoSummary` whose `dimensions` is `None`
  When `video_summary_html` is called
  Then the returned string does NOT contain the character `'x'` between two digit runs
  And the returned string does NOT contain the substring `"None"`

Scenario: Video summary HTML escapes a hostile filename
  Test:
    Package: robrix
    Filter: test_video_summary_html_escapes_filename
  Given a `VideoSummary` whose `filename` is `"<img src=x onerror=1>.mp4"`
  When `video_summary_html` is called
  Then the returned string contains `"&lt;img"`
  And the returned string does NOT contain `"<img "`

## Out of Scope

- In-app video playback or transcoding.
- Subtitle rendering.
- Frame extraction or scrubbing previews.
- Any change to `MediaCache` semantics beyond calling its existing thumbnail
  fetch path.
