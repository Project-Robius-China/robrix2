spec: project
name: "Month 2 — Media & Multimedia"
tags: [month-2, media, multimedia, matrix, ui]
---

## Intent

Land the README roadmap item "Display multimedia (audio/video/gif) message events"
(GitHub issue #120) in the `robrix` crate during Month 2. Today every non-image
multimedia `MessageType` (Audio, Video, animated GIF inside Image) renders as a
plain-text placeholder ending in "playback not yet supported" — see
`src/home/room_screen.rs:4534` (audio), `src/home/room_screen.rs:4575` (video),
and `src/home/room_screen.rs:4362` (image). Month 2 turns those placeholders into
real, observable rendering that is exercised by deterministic unit tests in the
same `#[cfg(test)] mod tests_*` style already used in `src/utils.rs`
(`tests_linkify`, `tests_human_readable_list`, `tests_ends_with_href`,
`tests_room_name`). Existing tests must keep passing — Month 2 reinforces them
rather than replacing them.

## Decisions

- Target crate: single binary crate `robrix` (see `Cargo.toml:2`).
- Matrix message types come from `matrix_sdk::ruma::events::room::message::MessageType`
  variants `Audio`, `Video`, and `Image`; do not introduce a new abstraction layer.
- Media fetching reuses the existing `MediaCache` in `src/media_cache.rs` —
  no new cache, no new background fetch task.
- Verification is `cargo test -p robrix` only; tests must be pure-Rust unit tests
  inside `#[cfg(test)] mod tests_*` blocks colocated with the code under test,
  matching the structural and naming style of `tests_human_readable_list` and
  `tests_linkify` in `src/utils.rs`.
- No live-homeserver integration tests, no `tokio::test`, no network. Inputs are
  constructed directly from `ruma` types (e.g. `AudioMessageEventContent::plain`).
- Size formatting continues to use `bytesize::ByteSize::b`, duration continues to
  use `format!("{:.2} sec", d.as_secs_f64())`, matching the existing
  `populate_audio_message_content` / `populate_video_message_content`
  conventions so reviewers can diff intent, not formatting churn.
- Every new public helper added under Month 2 ships with a `tests_*` module that
  has at least one happy-path test and at least one exception-path test, so the
  exception-vs-happy ratio in `src/` does not regress.

## Constraints

- Must NOT delete or rename any existing `#[test] fn test_*` in `src/utils.rs`,
  `src/event_preview.rs`, or anywhere else under `src/`. Existing assertions are
  the reinforcement baseline for Month 2.
- Must NOT change the public signature of `MediaCache::try_get_media_or_fetch`,
  `MediaCache::remove_cache_entry`, or `MediaCacheEntry`. Audio/Video/GIF work
  consumes the cache through its current API.
- Must NOT introduce a new heavyweight runtime dependency (no full media
  decoder, no ffmpeg binding) at the project layer. Per-task specs may add a
  narrowly scoped dependency if their own `Decisions` block names it.
- Must NOT regress the existing image rendering path
  (`populate_image_message_content`); image messages that are not animated must
  keep producing the same `text_or_image` output.

## Boundaries

### Allowed Changes

- specs/Month-2/**
- src/home/room_screen.rs
- src/event_preview.rs
- src/media_cache.rs
- src/shared/text_or_image.rs
- src/shared/image_viewer.rs
- src/utils.rs
- src/lib.rs

### Forbidden

- Do not modify `src/sliding_sync.rs` request types beyond what an existing
  `MatrixRequest::FetchMedia` already supports.
- Do not add new top-level crates or split `robrix` into a workspace — Month 2
  stays inside the single-crate layout described in `Cargo.toml`.
- Do not introduce `unwrap()` on user-supplied `MediaSource` values; reuse the
  existing `MediaSource::Plain` / `MediaSource::Encrypted` match arms in
  `src/home/room_screen.rs` (see `src/home/room_screen.rs:4471`).
- Do not delete the placeholder strings in a single commit that has no
  replacement rendering — each task spec must replace them atomically.

## Completion Criteria

Scenario: Existing utility tests still pass after Month 2 edits
  Test:
    Package: robrix
    Filter: tests_human_readable_list
  Given the Month 2 code changes are applied to `src/`
  When `cargo test -p robrix tests_human_readable_list` runs
  Then every test in the `tests_human_readable_list` module passes
  And the count of tests in that module is unchanged from the pre-Month-2 baseline

Scenario: Existing linkify tests still pass after Month 2 edits
  Test:
    Package: robrix
    Filter: tests_linkify
  Given the Month 2 code changes are applied to `src/`
  When `cargo test -p robrix tests_linkify` runs
  Then every test in the `tests_linkify` module passes

Scenario: Each Month-2 task ships at least one exception-path test
  Test:
    Package: robrix
    Filter: tests_audio_summary
  Given the audio, video, and animated-image task specs are stamped
  When `cargo test -p robrix tests_audio_summary` runs
  Then the run includes at least one test whose name contains the substring "missing"
  And the run includes at least one test whose name contains the substring "caption"

Scenario: Image rendering for non-animated images is unchanged
  Test:
    Package: robrix
    Filter: test_is_animated_image_mime_rejects_static_image
  Given a `MessageType::Image` whose `info.mimetype` is `"image/jpeg"`
  When the animated-image detector classifies it
  Then the detector returns `false`
  And the existing thumbnail-first rendering path is selected

## Out of Scope

- File download for `MessageType::File` (the "File download not yet supported"
  TODO at `src/home/room_screen.rs:4504`) — tracked separately, not Month 2.
- Voice messages (MSC3245 `m.voice`) — Month 2 covers `m.audio` only.
- Editing of existing audio/video messages — `populate_*` rendering only.
- Location messages (`MessageType::Location`) — already render with map links.
- Encrypted media beyond what the existing `MediaSource::Encrypted` arm in
  `src/home/room_screen.rs:4471` already handles.
