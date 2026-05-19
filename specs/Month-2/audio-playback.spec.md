spec: task
name: "Month 2 — Audio Message Rendering"
inherits: project
tags: [month-2, media, audio]
---

## Intent

Replace the static "Audio playback not yet supported" HTML placeholder produced
by `populate_audio_message_content` in `src/home/room_screen.rs:4577` with a
**standalone Makepad widget** — `AudioMessagePlayer` — that renders a circular
Play button, the audio filename as title, a `mm:ss (size)` subtitle, and a
draggable seek slider with an elapsed `mm:ss` label. The widget owns its
playback state and drives real audio output through a single global
playback controller modeled on `examples/media_player` in the makepad
repository (`/Users/alanpoon/Documents/rust/makepad/examples/media_player`).

The reference UI a finished widget must match:
`/Users/alanpoon/Downloads/Screenshot 2026-05-19 at 11.25.52 AM.png` —
a round play affordance on the left, filename ("sample.wav") above
"00:02 (344.61 KB)", a thin slider with a thumb dot, and the elapsed time
"00:01" trailing the slider.

Decoding logic is a thin port of `examples/media_player/src/decoder.rs`
(symphonia-based `decode_audio` returning `DecodedPcm`). Mixing logic is a
thin port of `examples/media_player/src/player.rs` (`PlayerState`,
`fill_audio_output`). The port is verbatim modulo namespace, so the upstream
example remains the canonical reference and bugs can be cross-checked against
it.

Detection, time formatting, and extension inference stay as pure helpers in
`src/event_preview.rs` in the `tests_*` style already used in `src/utils.rs`,
so the easy-to-test logic stays decoupled from the widget shell.

## Decisions

### Standalone widget

- New widget `AudioMessagePlayer` lives at `src/shared/audio_message_player.rs`,
  next to `text_or_image.rs`, `animated_image.rs`, and `image_viewer.rs`.
  Declared in `src/shared/mod.rs`. Exposes `AudioMessagePlayerRef` and
  `AudioMessagePlayerWidgetRefExt` following the existing widget conventions.
- `live_design!` block defines the layout as a horizontal row: circular
  `play_button: Button` on the left, then a `Down`-flow `View` containing
  `filename_label: Label`, `subtitle_label: Label` ("00:02 (344.61 KB)"),
  and a horizontal row with `slider: Slider` (Fill width) plus
  `elapsed_label: Label`. Background is a rounded rect matching the mockup's
  light grey card.
- The widget owns: `filename: String`, `total_duration_secs: Option<f64>`,
  `total_size_bytes: Option<u64>`, `media_source: Option<MediaSource>`,
  `decoded: Option<Arc<DecodedPcm>>`, `state: Arc<Mutex<PlayerState>>`,
  `decode_error: Option<String>`, `slider_drag: Option<SliderDragState>`,
  `self_uid: WidgetUid`.

### Decoder port

- New module `src/shared/audio_decoder.rs` ports
  `examples/media_player/src/decoder.rs` verbatim except for the module path.
  Exposes `pub fn decode_audio(bytes: &[u8], hint_ext: &str) -> Result<DecodedPcm, DecodeError>`
  and `pub struct DecodedPcm { sample_rate: u32, channels: usize, interleaved_samples: Vec<f32> }`.
  Stereo-interleaved `f32` output, matching the example.
- A file-header comment in `src/shared/audio_decoder.rs` cites the upstream
  source path so future readers know it is a port, not original code.
- Decoding runs off the UI thread: when bytes arrive from `MediaCache`, the
  widget spawns `std::thread::spawn(move || decode_audio(...))` and posts the
  result back as a `Cx` action (`DecodeReady { uid, decoded }` or
  `DecodeFailed { uid, error }`). The widget reads its own `uid` to filter.

### Single-active playback controller

- New module `src/shared/audio_playback_controller.rs` owns a process-global
  `OnceLock<Mutex<ActiveTrack>>` where
  `ActiveTrack = Option<(WidgetUid, Arc<DecodedPcm>, Arc<Mutex<PlayerState>>)>`.
- The first time any widget calls `controller::set_active(cx, uid, decoded, state)`,
  the controller registers exactly one `cx.audio_output(0, ...)` callback for
  the lifetime of the process. The callback locks the mutex and, if a track
  is present, calls a ported `fill_audio_output` (from
  `examples/media_player/src/player.rs`) on the active `(decoded, state)`.
- `set_active` posts `AudioPlaybackAction::ActiveTrackChanged { now_playing: WidgetUid }`
  via `Cx::post_action`. Every other `AudioMessagePlayer` observes this in
  `handle_event`, and if `now_playing != self.self_uid` it sets its own
  `state.playing = false` and resets its Play button label to "Play".
- The controller registers the audio output callback at most once per process.
  Re-registration is forbidden because Makepad keeps every registered callback
  alive and a second registration would double-mix.

### Pure helpers

- `summarize_audio_message(&AudioMessageEventContent) -> AudioSummary` in
  `src/event_preview.rs`. `AudioSummary` is a `pub struct` with fields
  `filename: String`, `mime: Option<String>`, `duration_secs: Option<f64>`,
  `size_bytes: Option<u64>`, `caption_html: Option<String>`. The helper is
  unchanged from the previous spec direction and continues to drive the
  widget's bound values; it no longer produces HTML.
- `format_mmss(secs: f64) -> String` in `src/event_preview.rs` returns a
  zero-padded `"mm:ss"` string. Negative, NaN, and infinite inputs return
  `"00:00"`. This is the format shown in the reference screenshot
  ("00:02", "00:01").
- `infer_audio_extension(filename: &str, mime: Option<&str>) -> &'static str`
  in `src/event_preview.rs` returns a stable hint string (`"mp3"`, `"wav"`,
  `"aiff"`, `"flac"`, `"m4a"`, or `""`) for symphonia's
  `Hint::with_extension`. Filename extension wins; mime is a fallback.
- `apply_slider_drag(state: &mut PlayerState, normalized_pos: f64, source_frames: usize, phase: DragPhase) -> ()`
  in `src/shared/audio_message_player.rs`. Pure function called from the
  widget's `handle_event` so the pause-on-drag / resume-on-release math is
  unit-testable without a Makepad event loop. `DragPhase = Start | Move | End`.

### `populate_audio_message_content` rewrite

- Signature changes to accept an `AudioMessagePlayerRef` (the new live_design
  slot in the per-message template) instead of an `HtmlOrPlaintextRef`. The
  `bool` return contract is preserved: returns `true` once the widget's
  scaffolding (filename, subtitle, disabled Play button) has been written,
  which always succeeds in one call — the async fetch/decode transitions are
  driven by the widget itself afterward.
- The per-message live_design template in `src/home/room_screen.rs` gains an
  `audio_player = <AudioMessagePlayer>` slot adjacent to the existing
  `message_content` slot. The slot is hidden by default and made visible only
  for `MessageType::Audio`.

### Slider behavior

- Pause-on-drag, resume-on-release. On `FingerDown` over the slider track or
  thumb, the widget snapshots `was_playing = state.playing`, sets
  `state.playing = false`, and enters `SliderDragState { was_playing }`. On
  `FingerMove`, it maps the cursor x-coordinate to a normalized `[0.0, 1.0]`
  position and updates `state.cursor_frames`. On `FingerUp`, it sets
  `state.playing = was_playing && state.cursor_frames < source_frames as f64`
  and clears the drag state.

### Decode failure UX

- If `decode_audio` returns `Err`, the widget enters a disabled visual state:
  Play button is greyed (disabled), slider is hidden, and an inline error
  label replaces the subtitle with a message like
  `"Unsupported audio format"`. Filename remains visible. The widget never
  panics on decode failure.

### Cargo

- `Cargo.toml` adds `symphonia = { version = "0.5", default-features = false,
  features = ["mp3", "wav", "aiff", "flac", "isomp4", "alac", "pcm"] }`,
  matching the upstream example's feature set so the same codecs are
  supported.

## Constraints

- Must NOT change the `bool` return contract of `populate_audio_message_content`
  (always `true` after the first paint, since the widget owns later
  transitions).
- Must NOT register more than one `cx.audio_output(0, ...)` callback per
  process lifetime. Re-registration would double-mix because Makepad keeps
  every registered callback alive.
- Must NOT block the UI thread on `decode_audio`. Decoding always runs on a
  spawned thread; the widget only consumes the result via a `Cx` action.
- Must NOT make `summarize_audio_message`, `format_mmss`,
  `infer_audio_extension`, or `apply_slider_drag` async — they are pure CPU
  functions over already-deserialized inputs so they can be tested without a
  runtime.
- Must NOT introduce per-widget audio sinks; the global controller is the
  only path to `cx.audio_output`. Simultaneous playback of multiple audio
  messages is forbidden.
- Must NOT panic on malformed `AudioMessageEventContent.info` — every nested
  `Option` field stays `Option`-typed in `AudioSummary`.
- Must NOT regress html-escape behavior for any caption text that is still
  rendered as HTML elsewhere — captions go through `htmlize::escape_text`.
- Must NOT modify any file under `/Users/alanpoon/Documents/rust/makepad/`;
  the example is the upstream reference, not a workspace member.

## Boundaries

### Allowed Changes

- Cargo.toml
- src/shared/mod.rs
- src/shared/audio_message_player.rs (new)
- src/shared/audio_decoder.rs (new, port of examples/media_player/src/decoder.rs)
- src/shared/audio_playback_controller.rs (new, ports fill_audio_output from examples/media_player/src/player.rs)
- src/event_preview.rs
- src/home/room_screen.rs
- src/media_cache.rs
- src/sliding_sync.rs

### Forbidden

- Do not move the widget files out of `src/shared/`; the chat-message media
  widgets all live there and the new widget must follow that pattern.
- Do not edit anything under `/Users/alanpoon/Documents/rust/makepad/`. The
  decoder and player code are *ported* into `src/shared/`, not depended on
  in place.
- Do not panic on malformed `AudioMessageEventContent.info` — every nested
  `Option` field must remain `Option`-typed in `AudioSummary`.
- Do not register `cx.audio_output` from inside `AudioMessagePlayer` directly;
  go through `audio_playback_controller::set_active`.
- Do not introduce waveform thumbnails, scrubbing previews, or amplitude
  visualization.
- Do not introduce streaming or chunked decode. The full file is decoded once
  after fetch.

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

Scenario: format_mmss zero-pads sub-minute durations
  Test:
    Package: robrix
    Filter: test_format_mmss_sub_minute
  Given a duration of `2.6` seconds
  When `format_mmss` is called
  Then the returned string equals `"00:02"`

Scenario: format_mmss rolls minutes correctly past 60 seconds
  Test:
    Package: robrix
    Filter: test_format_mmss_over_one_minute
  Given a duration of `65.0` seconds
  When `format_mmss` is called
  Then the returned string equals `"01:05"`

Scenario: format_mmss is robust to NaN, negative, and infinite input
  Test:
    Package: robrix
    Filter: test_format_mmss_handles_bad_input
  Given each of `f64::NAN`, `-1.0`, and `f64::INFINITY`
  When `format_mmss` is called
  Then the returned string equals `"00:00"` in every case

Scenario: infer_audio_extension prefers the filename extension over the mime type
  Test:
    Package: robrix
    Filter: test_infer_audio_extension_prefers_filename
  Given a filename `"call.wav"` and a mime `Some("audio/mpeg")`
  When `infer_audio_extension` is called
  Then the returned string equals `"wav"`

Scenario: infer_audio_extension falls back to mime when filename has no extension
  Test:
    Package: robrix
    Filter: test_infer_audio_extension_falls_back_to_mime
  Given a filename `"recording"` and a mime `Some("audio/mpeg")`
  When `infer_audio_extension` is called
  Then the returned string equals `"mp3"`

Scenario: infer_audio_extension returns empty string when neither source is informative
  Test:
    Package: robrix
    Filter: test_infer_audio_extension_returns_empty
  Given a filename `"recording"` and a mime `None`
  When `infer_audio_extension` is called
  Then the returned string equals `""`

Scenario: Decoder produces stereo interleaved f32 samples for a WAV input
  Test:
    Package: robrix
    Filter: test_decode_audio_returns_stereo_pcm_for_wav
  Given the bytes of a small embedded WAV test fixture in `tests/resources/sample.wav`
  When `decode_audio` is called with hint `"wav"`
  Then the returned `DecodedPcm.channels` equals `2`
  And `DecodedPcm.sample_rate` is greater than `0`
  And `DecodedPcm.interleaved_samples.len() % 2` equals `0`
  And `DecodedPcm.interleaved_samples` is non-empty

Scenario: Decoder reports an error for truncated input rather than panicking
  Test:
    Package: robrix
    Filter: test_decode_audio_returns_error_for_truncated_input
  Given the first 16 bytes of the WAV fixture
  When `decode_audio` is called with hint `"wav"`
  Then the result is `Err(DecodeError::Probe(_) | DecodeError::Decode(_) | DecodeError::Empty)`

Scenario: Slider drag start pauses an already-playing track
  Test:
    Package: robrix
    Filter: test_apply_slider_drag_start_pauses_playback
  Given a `PlayerState { playing: true, cursor_frames: 100.0 }`
  When `apply_slider_drag` is called with `phase = Start` and `normalized_pos = 0.5` and `source_frames = 1000`
  Then `state.playing` equals `false`
  And `state.cursor_frames` equals `500.0`

Scenario: Slider drag end resumes playback when the track was playing before drag
  Test:
    Package: robrix
    Filter: test_apply_slider_drag_end_resumes_when_was_playing
  Given a `PlayerState { playing: false, cursor_frames: 250.0 }` and `was_playing = true`
  When `apply_slider_drag` is called with `phase = End` and `normalized_pos = 0.25` and `source_frames = 1000`
  Then `state.playing` equals `true`
  And `state.cursor_frames` equals `250.0`

Scenario: Slider drag end does NOT resume when the user has scrubbed to the very end
  Test:
    Package: robrix
    Filter: test_apply_slider_drag_end_does_not_resume_at_track_end
  Given `was_playing = true`
  When `apply_slider_drag` is called with `phase = End` and `normalized_pos = 1.0` and `source_frames = 1000`
  Then `state.playing` equals `false`

Scenario: Playback controller broadcasts ActiveTrackChanged when a new widget claims the slot
  Test:
    Package: robrix
    Filter: test_playback_controller_broadcasts_on_set_active
  Given the controller has no active track
  When `set_active(uid_a, decoded, state)` is called
  Then exactly one `AudioPlaybackAction::ActiveTrackChanged { now_playing: uid_a }` is observed in the pending action queue

Scenario: Playback controller replaces previous active track on second set_active call
  Test:
    Package: robrix
    Filter: test_playback_controller_replaces_previous_active
  Given `set_active(uid_a, decoded_a, state_a)` has been called
  When `set_active(uid_b, decoded_b, state_b)` is called
  Then the controller's stored `ActiveTrack.0` equals `uid_b`
  And an `AudioPlaybackAction::ActiveTrackChanged { now_playing: uid_b }` is observed

Scenario: Playback controller registers cx.audio_output at most once
  Test:
    Package: robrix
    Filter: test_playback_controller_registers_audio_output_once
  Given `set_active` has been called twice with different widgets
  When inspecting the controller's `audio_output_registered` flag
  Then the flag equals `true`
  And the registration counter equals `1`

Scenario: PlayerState mixer zero-fills output when not playing
  Test:
    Package: robrix
    Filter: test_player_state_mixer_zero_fills_when_paused
  Given a `PlayerState { playing: false, cursor_frames: 0.0 }`
  And a non-empty `DecodedPcm` source
  When `fill_audio_output` is called
  Then every sample in the output buffer equals `0.0`
  And `state.cursor_frames` is unchanged

Scenario: PlayerState mixer stops and resets cursor at end of source
  Test:
    Package: robrix
    Filter: test_player_state_mixer_stops_at_end_of_source
  Given a `PlayerState { playing: true, cursor_frames: 95.0 }`
  And a `DecodedPcm` source of 100 frames at 44_100 Hz
  When `fill_audio_output` is called with a 32-frame output at 44_100 Hz
  Then `state.playing` equals `false`
  And `state.cursor_frames` equals `0.0`

## Out of Scope

- Voice-message-specific UI (MSC3245 `m.voice`).
- Waveform thumbnails, amplitude rendering, or scrubbing previews.
- Re-encoding or transcoding audio.
- Streaming or chunked decode — the full file is decoded once after fetch.
- Simultaneous playback of multiple audio messages.
- Background playback while the widget is scrolled offscreen.
- Auto-advance to the next audio message after the current one ends.
- Per-widget or global volume controls.
- Persisting playback position across app restarts.
