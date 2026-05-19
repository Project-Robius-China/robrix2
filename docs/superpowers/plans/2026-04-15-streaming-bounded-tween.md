# Streaming Bounded Tween Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a bounded per-update linear-interpolation tween to `StreamingAnimState` so that streaming bot replies look smooth even when the server emits coarse-grained edits (Matrix edit events are throttled every 200-1000ms upstream).

**Architecture:** Each live `update_target` call with text growth captures `(reveal_base_char, reveal_base_time)`. `tick_with_elapsed` linearly interpolates `displayed_char_count` from `reveal_base_char` toward `target_char_count` over a fixed `TWEEN_DURATION = 150ms`. On `is_live=false` (finish edit) or on `new/restore` (first arrival), state syncs immediately without tween. `TWEEN_DURATION` is strictly smaller than any reasonable server throttle so the client always catches up before the next edit, guaranteeing no accumulated backlog and no PR #14-style long tail.

**Tech Stack:** Rust, Makepad 2.0. Changes isolated to `src/home/streaming_animation.rs`. No changes to `src/home/room_screen.rs` (its frame handler already polls `needs_frame()` and schedules the next frame).

**Spec:** [specs/task-restore-streaming-animation.spec.md](../../../specs/task-restore-streaming-animation.spec.md) (updated with tween decisions and completion criteria in this same branch).

---

## File Structure

- **Modify:** `src/home/streaming_animation.rs` — add `TWEEN_DURATION` const, add `reveal_base_char` and `reveal_base_time` fields, rewrite `update_target` / `tick_with_elapsed` / `needs_frame`, update `new` / `restore` to initialise reveal base at sync point. All tween logic confined to this file.

- **Unchanged:** `src/home/room_screen.rs` — frame handler already calls `state.tick()` when `needs_frame()` returns true and invalidates `content_drawn_since_last_update` to force re-render. The new `needs_frame()` now returns true during tween, which cleanly re-activates the existing scheduling without any call-site changes.

- **Unchanged:** `src/home/link_preview.rs` — orthogonal to tween.

- **No new files.**

---

## Task 1: Add failing tests for tween behaviour (TDD Red)

**Files:**

- Modify: `src/home/streaming_animation.rs` (add tests in `mod tests` at the bottom of the file)

All tests added to the existing `mod tests` block. The tests assert behaviour that does not yet exist and should therefore FAIL until Task 2 implements it.

- [ ] **Step 1.1: Add `test_update_target_live_growth_defers_sync_via_tween`**

Insert after the existing `test_update_target_tracks_latest_full_snapshot` test (roughly after line 207 of the current file). Place before `test_update_target_shrinks_safely`:

```rust
    #[test]
    fn test_update_target_live_growth_defers_sync_via_tween() {
        let mut s = make_state("Hello");
        // Sanity: new() already synced displayed to the first target.
        assert_eq!(s.displayed_char_count, s.target_char_count);

        let displayed_before = s.displayed_char_count;
        s.update_target("Hello, world!", true);

        // Growth path must NOT sync displayed; it should stay at the previous
        // target so tick_with_elapsed can interpolate toward the new target.
        assert_eq!(s.displayed_char_count, displayed_before);
        assert_eq!(s.reveal_base_char, displayed_before);
        assert!(s.displayed_char_count < s.target_char_count);
        assert!(s.needs_frame());
    }
```

- [ ] **Step 1.2: Add `test_update_target_live_false_syncs_immediately`**

Insert directly below `test_update_target_live_growth_defers_sync_via_tween`:

```rust
    #[test]
    fn test_update_target_live_false_syncs_immediately() {
        let mut s = make_state("Hello");
        // Force a mid-tween state to prove the sync still happens.
        s.displayed_char_count = 1;
        s.displayed_byte_offset = 1;

        s.update_target("Hello, world!", false);

        assert_eq!(s.displayed_char_count, s.target_char_count);
        assert_eq!(s.displayed_byte_offset, s.target_text.len());
        assert!(!s.needs_frame());
    }
```

- [ ] **Step 1.3: Add `test_tick_interpolates_displayed_toward_target`**

Insert directly below the previous test:

```rust
    #[test]
    fn test_tick_interpolates_displayed_toward_target() {
        let mut s = make_state("Hello");
        s.update_target(&"a".repeat(100), true);
        // Sanity: growth path sets up the tween.
        assert_eq!(s.reveal_base_char, 5);
        assert_eq!(s.target_char_count, 100);

        let changed = s.tick_with_elapsed(TWEEN_DURATION / 2);

        assert!(changed);
        // Halfway through TWEEN_DURATION should reveal ~half of the 95-char
        // delta (5 base + ~47 revealed = ~52). Give a ±2-char tolerance to
        // absorb rounding across platforms.
        assert!(s.displayed_char_count >= 50);
        assert!(s.displayed_char_count <= 54);
        assert!(s.displayed_char_count < s.target_char_count);
        assert!(s.needs_frame());
    }
```

- [ ] **Step 1.4: Add `test_tick_completes_tween_at_full_duration`**

Insert directly below the previous test:

```rust
    #[test]
    fn test_tick_completes_tween_at_full_duration() {
        let mut s = make_state("Hello");
        s.update_target(&"a".repeat(100), true);

        let changed = s.tick_with_elapsed(TWEEN_DURATION);

        assert!(changed);
        assert_eq!(s.displayed_char_count, s.target_char_count);
        assert!(!s.needs_frame());
    }
```

- [ ] **Step 1.5: Add `test_tick_noop_when_displayed_already_caught_up`**

Insert directly below the previous test:

```rust
    #[test]
    fn test_tick_noop_when_displayed_already_caught_up() {
        let mut s = make_state("Hello");
        // new() already synced displayed to target.
        assert_eq!(s.displayed_char_count, s.target_char_count);

        let before = s.displayed_char_count;
        let changed = s.tick_with_elapsed(Duration::from_secs(1));

        assert!(!changed);
        assert_eq!(s.displayed_char_count, before);
        assert!(!s.needs_frame());
    }
```

- [ ] **Step 1.6: Run tests to verify they all fail**

Run:

```
cargo test --lib home::streaming_animation -- \
    test_update_target_live_growth_defers_sync_via_tween \
    test_update_target_live_false_syncs_immediately \
    test_tick_interpolates_displayed_toward_target \
    test_tick_completes_tween_at_full_duration \
    test_tick_noop_when_displayed_already_caught_up
```

Expected: All 5 tests **FAIL** because `reveal_base_char` / `TWEEN_DURATION` do not exist and the existing `update_target` / `tick_with_elapsed` always sync / no-op.

- [ ] **Step 1.7: Commit red tests**

```
git add src/home/streaming_animation.rs
git commit -m "test: add failing tests for bounded streaming tween"
```

---

## Task 2: Implement TWEEN_DURATION constant, reveal_base fields, and tween-aware state transitions (TDD Green)

**Files:**

- Modify: `src/home/streaming_animation.rs` (constants block at top, `StreamingAnimState` struct, `new`, `restore`, `sync_displayed_to_target`, `update_target`, `tick`, `tick_with_elapsed`, `needs_frame`)

This task turns the failing tests green. All changes live in this single file.

- [ ] **Step 2.1: Add `TWEEN_DURATION` constant**

In the constants block near the top of the file (after `LIVE_STREAM_STALL_TIMEOUT`), add:

```rust
/// Upper bound on how long a single live update's interpolation can run.
/// Chosen strictly smaller than any reasonable server edit throttle so
/// the client always catches up before the next edit arrives, preventing
/// the long-tail behaviour of PR #14's fixed-cadence pacer.
const TWEEN_DURATION: Duration = Duration::from_millis(150);
```

- [ ] **Step 2.2: Add `reveal_base_char` and `reveal_base_time` fields to `StreamingAnimState`**

Modify the struct block (currently around lines 9-24):

```rust
pub struct StreamingAnimState {
    pub target_text: String,
    pub target_char_count: usize,
    pub displayed_char_count: usize,
    pub displayed_byte_offset: usize,
    pub last_update_time: Instant,
    pub animation_start_time: Instant,
    pub display_buffer: String,
    /// Whether the message currently carries the MSC4357 `live` field.
    pub is_live: bool,
    pub timeline_index: Option<usize>,
    /// Starting displayed_char_count for the current tween window.
    /// Equals target_char_count when no tween is in progress.
    pub reveal_base_char: usize,
    /// Reference time for the current tween window.
    pub reveal_base_time: Instant,
}
```

- [ ] **Step 2.3: Update `sync_displayed_to_target` to also reset `reveal_base_char`**

Replace the existing `sync_displayed_to_target` method. When we sync, the tween window collapses: `reveal_base_char` must match the new `displayed_char_count` so subsequent `tick_with_elapsed` calls correctly see "no backlog" until the next growth update.

```rust
fn sync_displayed_to_target(&mut self) {
    self.displayed_char_count = self.target_char_count;
    self.displayed_byte_offset = self.target_text.len();
    self.reveal_base_char = self.target_char_count;
}
```

- [ ] **Step 2.4: Update `new` to initialise `reveal_base_*` fields**

Modify the struct literal in `new` so the new fields are populated. The call to `sync_displayed_to_target()` at the end of `new` then aligns `reveal_base_char` to `target_char_count`.

```rust
pub fn new(initial_text: &str, is_live: bool) -> Self {
    let char_count = initial_text.chars().count();
    let now = Instant::now();
    let mut state = Self {
        target_text: initial_text.to_string(),
        target_char_count: char_count,
        displayed_char_count: 0,
        displayed_byte_offset: 0,
        last_update_time: now,
        animation_start_time: now,
        display_buffer: String::with_capacity(initial_text.len() + 4),
        is_live,
        timeline_index: None,
        reveal_base_char: 0,
        reveal_base_time: now,
    };
    state.sync_displayed_to_target();
    state
}
```

`restore` already delegates to `new` then copies `animation_start_time` / `timeline_index`, so no change is required there.

- [ ] **Step 2.5: Rewrite `update_target` to branch on live/growth**

Replace the current `update_target` body. The contract:

- `is_live = false` → finish edit → sync immediately (Moly semantics).
- `is_live = true` **and** new target is strictly larger than previous `target_char_count` → capture `reveal_base_char` and `reveal_base_time`, do NOT sync.
- `is_live = true` but no growth (equal or shrink) → sync immediately, no tween starts.

```rust
pub fn update_target(&mut self, new_text: &str, is_live: bool) {
    let prev_target_char_count = self.target_char_count;
    let previous_displayed = self.displayed_char_count;

    self.target_text.clear();
    self.target_text.push_str(new_text);
    self.target_char_count = new_text.chars().count();
    self.is_live = is_live;

    let now = Instant::now();
    self.last_update_time = now;

    let needed = new_text.len() + 4;
    if self.display_buffer.capacity() < needed {
        self.display_buffer.reserve(needed - self.display_buffer.len());
    }

    let is_growth = is_live && self.target_char_count > prev_target_char_count;
    if is_growth {
        // Keep displayed at its current position and open a fresh tween
        // window. displayed_char_count may already trail target_char_count
        // from an earlier tween; preserve it as the new reveal base.
        self.reveal_base_char = previous_displayed.min(self.target_char_count);
        self.reveal_base_time = now;
        // displayed_byte_offset is left trailing; the next tick will advance
        // it when displayed_char_count moves forward. Clamp it here to stay
        // within the new target text so advance_displayed's slicing stays
        // safe even before the first tick.
        self.displayed_char_count = self.reveal_base_char;
        self.displayed_byte_offset = self.target_text
            .char_indices()
            .nth(self.reveal_base_char)
            .map_or(self.target_text.len(), |(byte_idx, _)| byte_idx);
    } else {
        // Finish edit or non-growing live update: sync immediately.
        self.sync_displayed_to_target();
        self.reveal_base_time = now;
    }
}
```

- [ ] **Step 2.6: Rewrite `tick_with_elapsed` to perform bounded linear interpolation**

Replace the current no-op body:

```rust
pub fn tick_with_elapsed(&mut self, elapsed_since_reveal: Duration) -> bool {
    if self.displayed_char_count >= self.target_char_count {
        return false;
    }

    let progress = (elapsed_since_reveal.as_secs_f64()
        / TWEEN_DURATION.as_secs_f64())
        .clamp(0.0, 1.0);

    let delta = self.target_char_count.saturating_sub(self.reveal_base_char);
    let target_displayed = self.reveal_base_char
        + ((delta as f64) * progress).round() as usize;
    let target_displayed = target_displayed.min(self.target_char_count);

    if target_displayed <= self.displayed_char_count {
        return false;
    }

    let advance = target_displayed - self.displayed_char_count;
    self.advance_displayed(advance);
    true
}
```

- [ ] **Step 2.7: Update `tick` to derive elapsed from `reveal_base_time`**

Replace the current `tick` body:

```rust
pub fn tick(&mut self) -> bool {
    let elapsed = self.reveal_base_time.elapsed();
    self.tick_with_elapsed(elapsed)
}
```

- [ ] **Step 2.8: Update `needs_frame` to reflect tween progress**

Replace the current `needs_frame` body:

```rust
pub fn needs_frame(&self) -> bool {
    self.displayed_char_count < self.target_char_count
}
```

- [ ] **Step 2.9: Run the 5 new tests, expect PASS**

Run:

```
cargo test --lib home::streaming_animation -- \
    test_update_target_live_growth_defers_sync_via_tween \
    test_update_target_live_false_syncs_immediately \
    test_tick_interpolates_displayed_toward_target \
    test_tick_completes_tween_at_full_duration \
    test_tick_noop_when_displayed_already_caught_up
```

Expected: All 5 tests PASS.

- [ ] **Step 2.10: Commit the green implementation**

```
git add src/home/streaming_animation.rs
git commit -m "feat: bounded per-update tween for streaming animation"
```

---

## Task 3: Reconcile pre-existing tests that assumed immediate sync

**Files:**

- Modify: `src/home/streaming_animation.rs` (`mod tests` section)

Two existing tests were written for the pure-Moly design where `update_target` always synced. Under bounded tween, the growth branch no longer syncs, so the assertions need adjustment. Each requires a direct fix, not deletion.

- [ ] **Step 3.1: Locate `test_update_target_tracks_latest_full_snapshot`**

Current body (references exact lines; verify before editing):

```rust
    #[test]
    fn test_update_target_tracks_latest_full_snapshot() {
        let mut s = make_state("Hello");
        s.update_target("Hello, world!", true);
        assert_eq!(s.target_char_count, 13);
        assert_eq!(s.displayed_char_count, s.target_char_count);
        assert_eq!(s.displayed_byte_offset, s.target_text.len());
        assert!(!s.needs_frame());
    }
```

This asserts `displayed == target` immediately after a live growth, which now contradicts the tween contract.

- [ ] **Step 3.2: Update this test to cover the **non-live** sync path**

The point of this test was that `target_text` / `target_char_count` are correctly refreshed. Shift the scenario to the non-growth live update, which still syncs, so the invariant the test was trying to protect is still covered:

```rust
    #[test]
    fn test_update_target_tracks_latest_full_snapshot() {
        let mut s = make_state("Hello, world!");
        // Non-growing live update must still sync immediately because there
        // is no backlog to interpolate across.
        s.update_target("Greetings!", true);
        assert_eq!(s.target_char_count, 10);
        assert_eq!(s.displayed_char_count, s.target_char_count);
        assert_eq!(s.displayed_byte_offset, s.target_text.len());
        assert!(!s.needs_frame());
    }
```

- [ ] **Step 3.3: Locate `test_update_target_recalculates_byte_offset_for_different_prefix`**

Current body uses `update_target("你好世界测试数据", true)` on a 5-char ASCII state. Under tween, the growth branch now sets `displayed_char_count = reveal_base_char = 5` but our new text only has 8 CJK chars. `displayed_byte_offset` is computed via `char_indices().nth(5)` on the new text, which is the sixth CJK char boundary (valid). The test's current assertion `displayed_char_count == 8` (full sync) no longer holds during tween.

- [ ] **Step 3.4: Update this test to exercise a tick through tween**

Re-scope the test to assert byte-offset safety after the tween completes. This still proves the original concern (no mid-char byte slicing on multi-byte growth) while matching the tween contract:

```rust
    #[test]
    fn test_update_target_recalculates_byte_offset_for_different_prefix() {
        let mut s = make_state("hello world");
        s.update_target("你好世界测试数据", true);
        // Finish the tween in one step by ticking past TWEEN_DURATION.
        let _ = s.tick_with_elapsed(TWEEN_DURATION);
        assert_eq!(s.displayed_char_count, 8);
        assert_eq!(s.displayed_byte_offset, s.target_text.len());
        s.fill_display_buffer();
        assert!(s.display_buffer.starts_with("你好世界测试数据"));
    }
```

- [ ] **Step 3.5: Run the full `streaming_animation` suite**

Run:

```
cargo test --lib home::streaming_animation
```

Expected: every test in the module passes (previously 15 + 5 new = 20, but two existing ones were rewritten, so the count is still 20).

- [ ] **Step 3.6: Commit the test reconciliation**

```
git add src/home/streaming_animation.rs
git commit -m "test: align streaming tests with bounded tween contract"
```

---

## Task 4: Full verification (cargo check + room_screen + link_preview tests)

**Files:** (verification only)

- No code changes.

- [ ] **Step 4.1: `cargo check --lib`**

Run:

```
cargo check --lib
```

Expected: `Finished` with zero warnings about the `streaming_animation` module. Any new warning here means Task 2 introduced an unused import or dead branch.

- [ ] **Step 4.2: `cargo test --lib home::streaming_animation`**

Run:

```
cargo test --lib home::streaming_animation
```

Expected: all tests in the module pass.

- [ ] **Step 4.3: `cargo test --lib home::link_preview`**

Run:

```
cargo test --lib home::link_preview
```

Expected: 3/3 link_preview tests pass (unchanged behaviour).

- [ ] **Step 4.4: `cargo test --lib` (full library suite)**

Run:

```
cargo test --lib
```

Expected: same pass/fail count as before the tween work. Four pre-existing failures listed in PR #99's test plan (`test_parse_bot_timeline_layers_invalid_metadata_does_not_panic`, three `room_input_bar` tests) may still fail — confirm they are the **same** failures as before and **no new failures** were introduced.

If any new test fails, stop and diagnose before continuing to Task 5.

---

## Task 5: Append commit to PR #99

**Files:** (no code changes; git operations only)

- [ ] **Step 5.1: Verify the working tree contains exactly the three tween commits on top of the existing fix commit**

Run:

```
git log origin/main..HEAD --oneline
```

Expected, in order (oldest first):

```
<sha> fix: restore MSC4357 streaming animation for bot replies
<sha> test: add failing tests for bounded streaming tween
<sha> feat: bounded per-update tween for streaming animation
<sha> test: align streaming tests with bounded tween contract
```

- [ ] **Step 5.2: Push the branch**

Run:

```
git push origin fix/streaming-animation-regression
```

Expected: fast-forward update. `gh pr view 99 --repo Project-Robius-China/robrix2` should show the new commits appended to the existing PR #99.

- [ ] **Step 5.3: Leave a short PR comment flagging the tween addition**

Run:

```
gh pr comment 99 --repo Project-Robius-China/robrix2 --body "Added bounded per-update tween (TWEEN_DURATION = 150ms, strictly < any reasonable server edit throttle). Spec and tests updated in the same branch. Plan: docs/superpowers/plans/2026-04-15-streaming-bounded-tween.md. cargo check clean; streaming_animation tests 20/20."
```

Expected: new comment URL returned.

---

## Self-Review

**Spec coverage (spec at `specs/task-restore-streaming-animation.spec.md`):**

- Bounded per-update tween decision → Task 2 (Steps 2.1, 2.5, 2.6, 2.8) covers `TWEEN_DURATION`, growth branch, interpolation, `needs_frame` flip.
- `new()` / `restore()` sync-on-init decision → Task 2 (Steps 2.3, 2.4) covers.
- `update_target(live=false)` immediate sync → Task 1 (Step 1.2 test) and Task 2 (Step 2.5 else branch).
- `tick_with_elapsed` linear interpolation → Task 1 (Steps 1.3, 1.4) and Task 2 (Step 2.6).
- `needs_frame` reflects tween progress → Task 1 tests assert it, Task 2.8 implements it.
- `populate_bot_text_message_content` / `link_preview_view` hiding / layered metadata final render / `LinkPreviewRef::populate_below_message` behaviour → all unchanged by this plan; covered by the existing PR #99 code and existing tests (Task 4 verifies no regression).
- Completion criteria `test_new_state_starts_fully_visible` / `test_update_target_live_growth_defers_sync_via_tween` / `test_update_target_live_false_syncs_immediately` / `test_tick_interpolates_displayed_toward_target` / `test_tick_completes_tween_at_full_duration` / `test_tick_noop_when_displayed_already_caught_up` → Task 1 implements all.
- Existing `cargo_check` / `cargo_test_streaming_animation` / `cargo_test_room_screen_streaming` completion criteria → Task 4 verifies.

**Placeholder scan:** none of the steps contain "TBD", "similar to", "add validation", or missing code.

**Type consistency:** `reveal_base_char: usize` and `reveal_base_time: Instant` are the only new field names; both referenced consistently across steps 2.2, 2.3, 2.4, 2.5, 2.6. `TWEEN_DURATION: Duration` is defined in Step 2.1 and referenced in Steps 1.3, 1.4, 2.1, 2.6, 3.4. `advance_displayed` is an existing method unchanged by this plan; Step 2.6 calls it identically to the existing Moly-style code.

Nothing to fix inline.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-04-15-streaming-bounded-tween.md`.

Two execution options:

1. **Subagent-Driven (recommended)** — dispatch a fresh subagent per task, review between tasks, fast iteration.
2. **Inline Execution** — execute tasks in this session using executing-plans, batch execution with checkpoints.

Which approach?
