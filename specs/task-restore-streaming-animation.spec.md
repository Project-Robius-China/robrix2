spec: task
name: "Adopt Moly-Style Live Streaming Rendering"
inherits: project
tags: [bot, streaming, animation, msc4357, regression]
depends: [task-tg-bot-timeline-cards]
estimate: 0.5d
---

## 意图

Octos 的 MSC4357 流式回复不再使用本地匀速打字机节奏。当前实现为了追求“丝滑逐字”而在客户端维护 `StreamingAnimState` 的 reveal cadence，但这会把长回复拖成明显长尾，并在掉帧后出现“卡一会儿然后突然补很多字”与“为了避免突跳而尾巴过长”之间的两难。用户已经明确要求与 Moly 保持同类体验：流式期间直接展示服务端当前最新文本，只在尾部追加一个进行中标记，完成后再切回既有 rich markdown / bot card 分层渲染。

本任务将 Robrix 的 streaming path 改为 Moly 风格的实时流式展示：不再做本地二次 cadence 动画，不再按字符 backlog 追帧，也不再对长回复做本地 reveal 节奏控制。流式期间消息始终显示最新 `m.text` 快照加 trailing cursor；流式结束后走现有富文本渲染。

参考实现：Moly 在 `moly-kit/src/widgets/standard_message_content.rs` 中仅在 `metadata.is_writing()` 时把当前全文和 `|` 尾标直接送入 markdown widget；其数据层 `openclaw_client.rs` 在每次流式事件到来时合并最新文本并立即 `Yield(content.clone())`，没有本地 reveal 时钟。

## 决策

- 保留 `StreamingAnimState` 作为 MSC4357 live message 的生命周期跟踪容器。它不承担服务端 pace 控制，但承担**有界的每次 update 过渡**（bounded per-update tween），把服务端节流造成的阶跃过渡平滑到人眼感知阈值以下。
- 引入常量 `TWEEN_DURATION = Duration::from_millis(150)`，**严格小于**任意合理的服务端 edit throttle（参考 Octos 默认 1000ms / 测试环境 200ms），保证每次服务端 edit 触发的 tween 在下一次 edit 到达前完成，**永不累积 backlog**，不会产生 PR #14 式的长尾。
- `StreamingAnimState::new()` 和 `restore()` 在初始化时 sync displayed 到完整 target（首次出现的消息不 tween，避免初显空白）。
- `update_target(new_text, is_live=true)` 且 `target_char_count` 有增长时：记录 `reveal_base_char = 当前 displayed_char_count` 与 `reveal_base_time = Instant::now()`，**不**立刻 sync displayed 到 target。shrink 或无增长时 sync 到 target 并重置 `reveal_base_*`。
- `update_target(new_text, is_live=false)`（finish edit）立即 sync displayed 到 target，**不 tween**，保留 Moly 的"结束即完整"语义。
- `tick_with_elapsed()` 当 `displayed_char_count < target_char_count` 时，按 `elapsed_since_reveal_base / TWEEN_DURATION` 做线性插值推进 displayed；progress ≥ 1.0 时 clamp 到 target。返回值表示 displayed 是否有变化。
- `tick()` 复用 `tick_with_elapsed`（内部基于 `reveal_base_time` 测算 elapsed）。
- `StreamingAnimState::needs_frame()` 返回 `displayed_char_count < target_char_count`，让现有帧调度器在 tween 进行中持续 tick，完成后停止。
- Streaming render path inside `populate_bot_text_message_content()` continues to show plaintext inside `bot_card_body`, but the plaintext must be the latest full snapshot plus trailing `●`, not a locally delayed prefix.
- Streaming render path must also keep hiding `content.link_preview_view` so recycled timeline items cannot show stale previews during live updates.
- Final post-stream render path remains unchanged: after the live field clears and the streaming state is removed, the existing rich markdown / layered metadata path renders the finished message.
- `LinkPreviewRef::populate_below_message()` must continue to recompute collapsible-button state on every populate call.

## 边界

### Allowed Changes

- src/home/streaming_animation.rs
- src/home/room_screen.rs
- src/home/link_preview.rs
- specs/task-restore-streaming-animation.spec.md

### Forbidden

- Do NOT modify bot card DSL layout (`bot_message_card`, `bot_card_body`, `bot_card_markdown`, `bot_card_markdown_plain`, `bot_status_strip`, `bot_metadata_footer`).
- Do NOT modify `is_msc4357_live`, `content_has_msc4357_live_marker`, `streaming_scan_range`, `refresh_stream_indices`, `rebuild_streaming_messages_for_full_snapshot`, `next_stream_timeout`.
- Do NOT modify `src/sliding_sync.rs`, `src/home/mod.rs`, or any other files outside the listed allowed paths.
- Do NOT modify Octos / testenv / any non-robrix2 file.
- Do NOT add new cargo dependencies.
- Do NOT add `#[allow(dead_code)]` to suppress warnings.
- Do NOT run `cargo fmt`.

## 排除范围

- Bot timeline card layered metadata extraction (status / provider / footer) — already shipped by `task-tg-bot-timeline-cards`.
- Non-MSC4357 bots — those replies never enter `streaming_messages`, so behavior is unchanged.
- Streaming for non-bot senders.
- Any client-side fallback for missing stream-finalization markers.
- Octos upstream bug — if the final `finish_stream` signal is missing, Robrix still waits for the existing live-stall timeout path.

## 完成条件

场景: 新建 streaming state 时直接显示最新全文
  测试: test_new_state_starts_fully_visible
  假设 `StreamingAnimState::new("Hello, world!", true)` 被调用
  当 state 初始化完成
  那么 `displayed_char_count` 等于 `target_char_count`
  并且 `displayed_byte_offset` 等于 `target_text.len()`
  并且 `needs_frame()` 返回 false

场景: live update 增长目标时保留 tween base,不立即 sync
  测试: test_update_target_live_growth_defers_sync_via_tween
  假设 一个 `StreamingAnimState` 已 sync 到 initial target (例如 `"Hello"`)
  当 调用 `update_target("Hello, world!", true)` 扩展文本
  那么 `displayed_char_count` **不**等于新的 `target_char_count` (保持在旧 target 的 char count)
  并且 `reveal_base_char` 等于 update 之前的 `displayed_char_count`
  并且 `needs_frame()` 返回 true

场景: live=false 的 finish edit 立即 sync 到完整文本,不 tween
  测试: test_update_target_live_false_syncs_immediately
  假设 一个 `StreamingAnimState` 中 displayed 落后于 target
  当 调用 `update_target(final_text, false)`
  那么 `displayed_char_count` 等于新的 `target_char_count`
  并且 `displayed_byte_offset` 等于新的 `target_text.len()`
  并且 `needs_frame()` 返回 false

场景: tween 期间 tick 按线性插值推进 displayed
  测试: test_tick_interpolates_displayed_toward_target
  假设 一个 live `StreamingAnimState`:`reveal_base_char = 0`、`target_char_count = 100`、`reveal_base_time = 当前`
  当 调用 `tick_with_elapsed(TWEEN_DURATION / 2)`
  那么 `displayed_char_count` 约等于 50 (中途进度)
  并且 `displayed_char_count` 严格大于 0 且小于 `target_char_count`
  并且 `needs_frame()` 返回 true

场景: tween 完成后 displayed clamp 到 target
  测试: test_tick_completes_tween_at_full_duration
  假设 一个 live `StreamingAnimState`:`reveal_base_char = 0`、`target_char_count = 100`
  当 调用 `tick_with_elapsed(TWEEN_DURATION)` (或更长)
  那么 `displayed_char_count` 等于 `target_char_count`
  并且 `needs_frame()` 返回 false

场景: 已 catch-up 的 live state tick 不做事
  测试: test_tick_noop_when_displayed_already_caught_up
  假设 一个 live `StreamingAnimState` 中 `displayed_char_count == target_char_count`
  当 调用 `tick_with_elapsed(Duration::from_secs(1))`
  那么 返回值为 false
  并且 `displayed_char_count` 不变化

场景: Bot streaming reply renders latest plaintext snapshot with cursor inside the bot card body
  测试: manual_test_bot_streaming_live_snapshot
  Level: manual
  假设 一个 Octos bot reply 当前仍带有 MSC4357 `live` field
  当 populate path 运行且 `streaming_messages` 中存在该 event 的 state
  那么 `bot_card_body` 显示最新完整文本快照加 trailing `●`
  并且 `bot_card_markdown` 与 `bot_card_markdown_plain` 不可见
  并且 status strip / provider line / footer line 在 streaming 期间隐藏
  并且 `content.link_preview_view` 在 streaming 期间隐藏

场景: Completed bot reply renders rich markdown after the live field clears
  测试: manual_test_bot_stream_finalization
  Level: manual
  假设 一个 bot reply 的最终 edit 去掉了 `org.matrix.msc4357.live`
  并且 `StreamingAnimState` 已经从 `streaming_messages` 中移除
  当 populate path 再次运行
  那么 body 通过现有 layered bot card 富文本路径渲染
  并且 trailing `●` 消失

场景: Link preview collapsible buttons reset when link count shrinks
  测试: link_preview_collapsible_state_
  假设 一个此前显示过 expand/collapse controls 的 link preview
  当 后续 populate 结果只有零到两个 preview entry
  那么 collapsible controls 被隐藏
  并且 hidden-link count 重置为零

场景: cargo check remains green
  测试: cargo_check
  假设 当前 worktree 包含本任务改动
  当 运行 `cargo check`
  那么 命令退出状态为零

场景: streaming_animation unit tests pass
  测试: cargo_test_streaming_animation
  假设 当前 worktree 包含本任务改动
  当 运行 `cargo test --lib home::streaming_animation::tests::`
  那么 相关测试全部通过

场景: room_screen streaming regression tests pass
  测试: cargo_test_room_screen_streaming
  假设 当前 worktree 包含本任务改动
  当 运行 targeted room-screen streaming regression tests
  那么 `test_streaming_scan_range`、`test_refresh_stream_indices`、`test_timeout_picks_earliest`、`test_full_snapshot_rebuild_*` 与 `test_clear_cache_update_rebuild_*` 全部通过

场景: Manual test — long streaming reply keeps pace with upstream updates
  测试: manual_test_long_stream
  Level: manual
  假设 一个 Octos bot 流式输出 500+ chars 的长回复
  当 运营者观察 timeline
  那么 文本展示速度跟随上游流式更新
  并且 不存在客户端本地打字机尾巴
  并且 不会在掉帧恢复后一次性补出本地 backlog
