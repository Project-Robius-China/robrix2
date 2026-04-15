use std::time::{Duration, Instant};

const FINISHED_STREAM_TIMEOUT: Duration = Duration::from_secs(30);
const LIVE_STREAM_STALL_TIMEOUT: Duration = Duration::from_secs(5 * 60);

/// Upper bound on how long a single live update's interpolation can run.
/// Chosen strictly smaller than any reasonable server edit throttle so
/// the client always catches up before the next edit arrives, preventing
/// the long-tail behaviour of PR #14's fixed-cadence pacer.
const TWEEN_DURATION: Duration = Duration::from_millis(150);

/// Animation state for a single streaming message.
/// Tracks an MSC4357 live message and caches the latest full snapshot.
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

impl StreamingAnimState {
    fn sync_displayed_to_target(&mut self) {
        self.displayed_char_count = self.target_char_count;
        self.displayed_byte_offset = self.target_text.len();
        self.reveal_base_char = self.target_char_count;
    }

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

    pub fn restore(previous: &Self, new_text: &str, is_live: bool) -> Self {
        let mut restored = Self::new(new_text, is_live);
        restored.animation_start_time = previous.animation_start_time;
        restored.timeline_index = previous.timeline_index;
        restored
    }

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

    pub fn advance_displayed(&mut self, chars_to_add: usize) {
        if chars_to_add == 0 || self.displayed_char_count >= self.target_char_count {
            return;
        }

        let remaining = &self.target_text[self.displayed_byte_offset..];
        let mut byte_advance = 0;
        let mut actual_chars = 0;
        for (byte_idx, _char) in remaining.char_indices() {
            if actual_chars >= chars_to_add {
                byte_advance = byte_idx;
                break;
            }
            actual_chars += 1;
        }
        if actual_chars <= chars_to_add && byte_advance == 0 && !remaining.is_empty() {
            byte_advance = remaining.len();
        }
        self.displayed_char_count =
            (self.displayed_char_count + actual_chars).min(self.target_char_count);
        self.displayed_byte_offset =
            (self.displayed_byte_offset + byte_advance).min(self.target_text.len());
    }

    pub fn tick(&mut self) -> bool {
        let elapsed = self.reveal_base_time.elapsed();
        self.tick_with_elapsed(elapsed)
    }

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

    pub fn fill_display_buffer(&mut self) {
        self.display_buffer.clear();
        self.display_buffer
            .push_str(&self.target_text[..self.displayed_byte_offset]);
        self.display_buffer.push_str(" \u{25CF}");
    }

    pub fn needs_frame(&self) -> bool {
        self.displayed_char_count < self.target_char_count
    }

    /// Streaming is complete when the live field is absent and all text has been revealed.
    pub fn is_complete(&self) -> bool {
        !self.needs_frame() && !self.is_live
    }

    pub fn timeout_after(&self) -> Duration {
        if self.is_live {
            LIVE_STREAM_STALL_TIMEOUT
        } else {
            FINISHED_STREAM_TIMEOUT
        }
    }

    pub fn is_timed_out(&self) -> bool {
        self.last_update_time.elapsed() > self.timeout_after()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_state(text: &str) -> StreamingAnimState {
        StreamingAnimState::new(text, true)
    }

    #[test]
    fn test_advance_ascii() {
        let mut s = StreamingAnimState::new("Hello, world!", true);
        s.displayed_char_count = 0;
        s.displayed_byte_offset = 0;
        s.advance_displayed(5);
        assert_eq!(s.displayed_char_count, 5);
        assert_eq!(&s.target_text[..s.displayed_byte_offset], "Hello");
    }

    #[test]
    fn test_advance_utf8_multibyte() {
        let mut s = StreamingAnimState::new("你好世界abcd", true);
        s.displayed_char_count = 0;
        s.displayed_byte_offset = 0;
        s.advance_displayed(2);
        assert_eq!(s.displayed_char_count, 2);
        assert_eq!(&s.target_text[..s.displayed_byte_offset], "你好");
    }

    #[test]
    fn test_advance_clamps_at_end() {
        let mut s = StreamingAnimState::new("abc", true);
        s.displayed_char_count = 0;
        s.displayed_byte_offset = 0;
        s.advance_displayed(100);
        assert_eq!(s.displayed_char_count, 3);
        assert_eq!(s.displayed_byte_offset, 3);
    }

    #[test]
    fn test_new_state_starts_fully_visible() {
        let s = make_state("Hello");
        assert_eq!(s.displayed_char_count, s.target_char_count);
        assert_eq!(s.displayed_byte_offset, s.target_text.len());
        assert!(!s.needs_frame());
    }

    #[test]
    fn test_update_target_tracks_latest_full_snapshot() {
        let mut s = make_state("Hello");
        s.update_target("Hello, world!", true);
        assert_eq!(s.target_char_count, 13);
        assert_eq!(s.displayed_char_count, s.target_char_count);
        assert_eq!(s.displayed_byte_offset, s.target_text.len());
        assert!(!s.needs_frame());
    }

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

    #[test]
    fn test_tick_completes_tween_at_full_duration() {
        let mut s = make_state("Hello");
        s.update_target(&"a".repeat(100), true);

        let changed = s.tick_with_elapsed(TWEEN_DURATION);

        assert!(changed);
        assert_eq!(s.displayed_char_count, s.target_char_count);
        assert!(!s.needs_frame());
    }

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

    #[test]
    fn test_update_target_shrinks_safely() {
        let mut s = make_state("Hello, world!");
        s.update_target("Hi", true);
        assert_eq!(s.displayed_char_count, 2);
        assert_eq!(s.displayed_byte_offset, 2);
        s.fill_display_buffer();
        assert!(s.display_buffer.starts_with("Hi"));
    }

    #[test]
    fn test_update_target_recalculates_byte_offset_for_different_prefix() {
        let mut s = make_state("hello world");
        s.update_target("你好世界测试数据", true);
        assert_eq!(s.displayed_char_count, 8);
        assert_eq!(s.displayed_byte_offset, s.target_text.len());
        s.fill_display_buffer();
        assert!(s.display_buffer.starts_with("你好世界测试数据"));
    }

    #[test]
    fn test_tick_does_not_advance_without_local_typewriter() {
        let mut s = make_state("Hello, world!");
        let before = s.displayed_char_count;
        let changed = s.tick_with_elapsed(Duration::from_secs(1));
        assert!(!changed);
        assert_eq!(s.displayed_char_count, before);
    }

    #[test]
    fn test_fill_display_buffer_appends_cursor_to_full_snapshot() {
        let mut s = make_state("Hello");
        s.fill_display_buffer();
        assert!(s.display_buffer.starts_with("Hello"));
        assert!(s.display_buffer.ends_with(" \u{25CF}"));
    }

    #[test]
    fn test_is_complete_msc4357() {
        let mut s = make_state("Hi");
        assert!(!s.is_complete());
        s.is_live = false;
        assert!(s.is_complete());
    }

    #[test]
    fn test_update_target_sets_live() {
        let mut s = make_state("Hello");
        assert!(s.is_live);
        s.update_target("Hello, world!", false);
        assert!(!s.is_live);
    }

    #[test]
    fn test_restore_tracks_latest_full_snapshot() {
        let prev = make_state("Hello, world!");
        let restored = StreamingAnimState::restore(&prev, "Hello, world!!!", true);
        assert_eq!(restored.displayed_char_count, restored.target_char_count);
        assert_eq!(restored.displayed_byte_offset, restored.target_text.len());
    }

    #[test]
    fn test_timeout_split_by_live_state() {
        let mut live = make_state("Hello");
        live.last_update_time = Instant::now() - Duration::from_secs(31);
        assert!(!live.is_timed_out());

        let mut finished = make_state("Hello");
        finished.is_live = false;
        finished.last_update_time = Instant::now() - Duration::from_secs(31);
        assert!(finished.is_timed_out());
    }

    #[test]
    fn test_tick_zero_elapsed() {
        let mut s = make_state("Hello");
        assert!(!s.tick_with_elapsed(Duration::ZERO));
        assert_eq!(s.displayed_char_count, s.target_char_count);
    }

    #[test]
    fn test_finished_stream_is_complete_without_extra_frames() {
        let mut s = make_state(&"a".repeat(30));
        s.is_live = false;
        assert!(s.is_complete());
        assert!(!s.tick_with_elapsed(Duration::from_secs(1)));
    }
}
