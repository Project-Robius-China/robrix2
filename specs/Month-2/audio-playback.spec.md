spec: task
name: "Month 2 — Audio Message Rendering"
inherits: project
tags: [month-2, media, audio]
---

## Intent

Replace the static "Audio playback not yet supported" HTML placeholder produced
by `populate_audio_message_content` in `src/home/room_screen.rs:4534` with a
deterministic summary that surfaces the audio message's filename, mime type,
duration, and size, then triggers a media fetch via the existing `MediaCache`
when the user explicitly requests playback. Detection and formatting are split
into a pure helper that can be unit-tested in the `tests_*` style already used
in `src/utils.rs`, so review cost stays low and the existing test suite is
reinforced rather than rewritten.

## Decisions

- Add a pure helper `summarize_audio_message(&AudioMessageEventContent) -> AudioSummary`
  in `src/event_preview.rs`. `AudioSummary` is a `pub struct` with fields
  `filename: String`, `mime: Option<String>`, `duration_secs: Option<f64>`,
  `size_bytes: Option<u64>`, `caption_html: Option<String>`.
- The HTML rendered by `populate_audio_message_content` is produced from
  `AudioSummary` via a second pure helper `audio_summary_html(&AudioSummary) -> String`.
  Both helpers live in `src/event_preview.rs` next to the existing
  `text_preview_of_message` so the audio preview path and the room timeline
  path share one source of truth.
- Duration formatting is `format!("{:.2} sec", d)` matching the current
  `populate_audio_message_content` output; size formatting is
  `bytesize::ByteSize::b(bytes).to_string()`.
- A user-visible "Play" button is appended at the end of the rendered HTML
  whenever `AudioSummary.mime` starts with `"audio/"`. Clicking it dispatches
  `MatrixRequest::FetchMedia` for the message's `MediaSource` via the existing
  `MediaCache::try_get_media_or_fetch` API — no new request variant.
- New tests live in `#[cfg(test)] mod tests_audio_summary` inside
  `src/event_preview.rs`, using the same module-per-feature, `test_<feature>_<case>`
  naming style as `tests_human_readable_list` in `src/utils.rs`.

## Constraints

- Must NOT change the signature of `populate_audio_message_content`; only its
  body changes.
- Must NOT make `summarize_audio_message` async — it is a pure CPU function over
  already-deserialized ruma types so it can be tested without a runtime.
- Must NOT introduce a new audio-decoding crate. Playback dispatch only fetches
  the bytes via `MediaCache`; actual audio output is out of scope here.
- Must NOT regress the html-escape behavior — filenames and captions go through
  `htmlize::escape_text` exactly as today.

## Boundaries

### Allowed Changes

- src/home/room_screen.rs
- src/event_preview.rs
- src/media_cache.rs
- src/sliding_sync.rs

### Forbidden

- Do not move the placeholder rendering into a brand-new module file; keep the
  helpers in `src/event_preview.rs` so the existing module layout is preserved.
- Do not panic on malformed `AudioMessageEventContent.info` — every nested
  `Option` field must remain `Option`-typed in `AudioSummary`.

## Completion Criteria

Scenario: Audio summary captures filename and metadata when info is fully populated
  Test:
    Package: robrix
    Filter: test_audio_summary_full_info
  Given an `AudioMessageEventContent` whose `body` is `"call-recording.ogg"`
  And whose `info.mimetype` is `Some("audio/ogg")`
  And whose `info.duration` is `Some(Duration::from_millis(12_500))`
  And whose `info.size` is `Some(98_304_u64.into())`
  When `summarize_audio_message` is called
  Then the returned `AudioSummary.filename` equals `"call-recording.ogg"`
  And `AudioSummary.mime` equals `Some("audio/ogg".to_string())`
  And `AudioSummary.duration_secs` equals `Some(12.5)`
  And `AudioSummary.size_bytes` equals `Some(98_304)`

Scenario: Audio summary handles missing info block
  Test:
    Package: robrix
    Filter: test_audio_summary_missing_info
  Given an `AudioMessageEventContent` whose `info` is `None`
  When `summarize_audio_message` is called
  Then `AudioSummary.mime` equals `None`
  And `AudioSummary.duration_secs` equals `None`
  And `AudioSummary.size_bytes` equals `None`
  And `AudioSummary.filename` equals the message body

Scenario: Audio summary preserves the formatted caption when present
  Test:
    Package: robrix
    Filter: test_audio_summary_with_formatted_caption
  Given an `AudioMessageEventContent` carrying a `formatted_caption` whose body
       is `"<i>Voice memo</i>"`
  When `summarize_audio_message` is called
  Then `AudioSummary.caption_html` equals `Some("<i>Voice memo</i>".to_string())`

Scenario: Audio summary HTML escapes a hostile filename
  Test:
    Package: robrix
    Filter: test_audio_summary_html_escapes_filename
  Given an `AudioSummary` whose `filename` is `"<script>alert(1)</script>.mp3"`
  When `audio_summary_html` is called
  Then the returned string contains `"&lt;script&gt;"`
  And the returned string does NOT contain `"<script>"`

Scenario: Audio summary HTML omits the play button when mime is not audio
  Test:
    Package: robrix
    Filter: test_audio_summary_html_skips_play_for_non_audio_mime
  Given an `AudioSummary` whose `mime` is `Some("application/octet-stream")`
  When `audio_summary_html` is called
  Then the returned string does NOT contain the substring `"Play"`

Scenario: Audio summary HTML emits one Play affordance when mime starts with audio/
  Test:
    Package: robrix
    Filter: test_audio_summary_html_emits_play_for_audio_mime
  Given an `AudioSummary` whose `mime` is `Some("audio/mpeg")`
  When `audio_summary_html` is called
  Then the returned string contains exactly one occurrence of the substring `"Play"`

Scenario: Audio summary error path — empty filename does not produce empty HTML
  Test:
    Package: robrix
    Filter: test_audio_summary_html_handles_empty_filename
  Given an `AudioSummary` whose `filename` is the empty string
  And whose `mime` is `None`
  When `audio_summary_html` is called
  Then the returned string is NOT empty
  And the returned string contains a fallback label such as `"Audio"` or `"unknown"`

Scenario: Audio summary error path — hostile mime string is escaped
  Test:
    Package: robrix
    Filter: test_audio_summary_html_escapes_hostile_mime
  Given an `AudioSummary` whose `mime` is `Some("audio/<script>".to_string())`
  When `audio_summary_html` is called
  Then the returned string does NOT contain the substring `"<script>"`
  And the returned string contains the escaped substring `"&lt;script&gt;"`

## Out of Scope

- Decoding or playing the actual audio bytes after they are fetched.
- Voice-message-specific UI (MSC3245 `m.voice`).
- Waveform thumbnails or scrubbing.
- Re-encoding or transcoding audio.
