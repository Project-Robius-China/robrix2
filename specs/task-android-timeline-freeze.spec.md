spec: task
name: "Android Timeline Freeze — Bound UI Work And Stop Full Timeline Snapshots"
inherits: project
tags: [bugfix, android, timeline, performance, matrix]
estimate: 1.5d
---

## Intent

阻止 Android 端 Robrix timeline 页面在运行一段时间后卡死或接近 ANR。当前风险点在 Robrix 本体：后台 timeline 订阅通过无界通道向 UI 发送更新，`RoomScreen::process_timeline_updates` 在一次 `Event::Signal` 中无限 drain 队列，同时 timeline subscriber 每次增量 timeline update 都 clone 整条 timeline snapshot。目标是迁移 Element X 类似的 diff-driven timeline 思路，让 UI 线程只消费有界批次和局部变化，避免小更新被放大成全量拷贝、全量扫描和长时间主线程占用。

## Constraints

- 本任务限定修复 Robrix 本体的 timeline update 热路径；没有 ANR trace 或平台调用栈证据时，不修改 Makepad 平台层
- UI 线程每次 `Event::Signal` 消费 timeline updates 的工作量必须有硬上限，不能无界 `try_recv` 直到队列清空
- 后台 timeline subscriber 不得在每个 `TimelineUpdate::ItemsChanged` 中发送整条 `timeline_items.clone()` snapshot
- UI 侧 `TimelineUiState.items` 仍然是当前房间 timeline 的权威 UI 数据源，后台只发送 diff/metadata，UI 按 diff 更新本地 items
- 必须保持现有滚动位置语义：append 时在底部保持 tail，不在底部时显示 jump-to-bottom；前置插入/clear_cache 时尽量保持当前可见 item
- 不新增 cargo 依赖
- 不运行 `cargo fmt`

## Decisions

- Add `MAX_TIMELINE_UPDATES_PER_SIGNAL` in `src/home/room_screen.rs` and use `should_yield_timeline_update_batch(processed_updates)` to cap timeline updates per `Event::Signal`
- When `process_timeline_updates` reaches the per-signal update budget and the receiver still has pending updates, call `SignalToUI::set_ui_signal()` so remaining work is resumed in a later UI event
- Replace full-snapshot incremental updates with `TimelineUpdate::ItemsChanged { item_diffs, changed_indices, is_append, clear_cache }`
- Introduce a Robrix-owned diff enum for UI application, covering the Matrix SDK diff operations currently handled in `timeline_subscriber_handler`: Append, PushFront, PushBack, PopFront, PopBack, Insert, Set, Remove, Truncate, Reset, Clear
- Keep `TimelineUpdate::FirstUpdate { initial_items }` as the initial snapshot path; only incremental timeline updates after the first update must avoid full snapshot cloning
- Apply item diffs on the UI thread to `tl.items` before running scroll-position adjustment, cache invalidation, streaming detection, bot discovery, and redraw decisions
- Limit bot discovery for incremental updates to `changed_indices` unless `clear_cache == true`, using a helper such as `timeline_update_scan_range(clear_cache, changed_indices, new_len)`
- Preserve `SignalToUI::set_ui_signal()` calls from async background tasks after enqueueing timeline updates

## Boundaries

### Allowed Changes
- `src/home/room_screen.rs`
- `src/sliding_sync.rs`
- `docs/plans/2026-05-07-android-timeline-freeze.md`
- `specs/task-android-timeline-freeze.spec.md`

### Forbidden
- Do NOT modify Makepad platform crates or Android platform event-loop code in this task
- Do NOT change Matrix SDK upstream APIs or fork behavior
- Do NOT change message sending semantics, read receipt semantics, or pagination request semantics
- Do NOT rewrite the timeline UI widgets or message rendering design
- Do NOT add new crates or change `Cargo.toml`
- Do NOT run `cargo fmt`

## Acceptance Criteria

Scenario: Root cause is documented as Robrix UI-thread amplification, not assumed Makepad freeze
  Test: manual_review_android_timeline_freeze_contract
  Given this task contract is reviewed
  When the reviewer reads the Intent and Constraints
  Then the problem statement names `RoomScreen::process_timeline_updates` unbounded queue draining
  And the problem statement names `sliding_sync::timeline_subscriber_handler` full timeline snapshot cloning
  And the contract does not require Makepad platform changes

Scenario: timeline update batch helper yields at the configured budget
  Test: timeline_update_batch_yields_after_budget
  Given `MAX_TIMELINE_UPDATES_PER_SIGNAL` is configured
  When `should_yield_timeline_update_batch` is called with values below the limit
  Then it returns `false`
  When it is called with the limit or a value above the limit
  Then it returns `true`

Scenario: process_timeline_updates does not drain an unbounded backlog in one Signal
  Test: manual_test_android_timeline_update_backlog_is_batched
  Given a RoomScreen timeline update receiver has more than `MAX_TIMELINE_UPDATES_PER_SIGNAL` pending updates
  When one `Event::Signal` is handled
  Then at most `MAX_TIMELINE_UPDATES_PER_SIGNAL` timeline updates are applied in that call
  And `SignalToUI::set_ui_signal()` is requested for the remaining backlog
  And the UI thread returns to the Makepad event loop before processing the remaining updates

Scenario: incremental timeline updates no longer clone the entire timeline snapshot
  Test: manual_review_timeline_subscriber_sends_diffs_not_full_snapshot
  Level: static_review
  Verification Strength: source_inspection
  Targets: src/sliding_sync.rs, src/home/room_screen.rs
  Given `sliding_sync::timeline_subscriber_handler` receives SDK timeline diffs after the first update
  When it enqueues an incremental `TimelineUpdate`
  Then the update is `TimelineUpdate::ItemsChanged` and contains Robrix-owned item diffs and metadata
  And it does not contain `new_items: timeline_items.clone()`
  And `TimelineUpdate::FirstUpdate` remains the only full initial snapshot path

Scenario: UI applies append and set diffs to the existing TimelineUiState items
  Test: timeline_items_apply_append_and_set_diffs
  Given `tl.items` contains two timeline items
  When a diff update appends one item and sets the second item
  Then `tl.items` contains three items
  And the second item reflects the set item
  And no full timeline replacement is required

Scenario: UI applies removal and reset diffs without panicking
  Test: timeline_items_apply_remove_truncate_clear_reset_diffs
  Given `tl.items` contains multiple timeline items
  When diff updates remove, truncate, clear, and reset items in valid SDK order
  Then `tl.items` matches the expected item order after each operation
  And no panic occurs for valid indices emitted by the SDK

Scenario: bot discovery scans only changed indices for incremental updates
  Test: timeline_update_scan_range_limits_incremental_work
  Given `clear_cache == false`
  And `changed_indices == 10..12`
  And the new timeline length is 1000
  When the bot discovery scan range is computed
  Then the range is `10..12`
  When `clear_cache == true`
  Then the range is `0..1000`

Scenario: scroll position semantics are preserved after diff-based updates
  Test: manual_test_timeline_scroll_position_preserved_after_diff_updates
  Given the user is viewing a room timeline away from the bottom
  When older events are inserted before the current visible range
  Then the same visible event remains anchored after the update
  And the timeline does not jump to the bottom
  When a new message is appended while the user is at the bottom
  Then the timeline remains in tail mode

Scenario: Android large-room active timeline remains responsive during update bursts
  Test: manual_test_android_large_room_timeline_remains_responsive
  Given an Android device logged into an account with a large active room
  And the room receives a burst of timeline updates
  When the user scrolls the timeline or taps the input bar during the burst
  Then touch input is still processed
  And no Android ANR dialog appears
  And logcat does not show the main thread stuck inside `process_timeline_updates` for multiple seconds

Scenario: build and focused timeline tests pass
  Test: cargo_check_and_timeline_tests_green
  When the developer runs `cargo check`
  And the developer runs `cargo clippy`
  And the developer runs `cargo test timeline_update_batch_yields_after_budget`
  And the developer runs `cargo test timeline_update_scan_range_limits_incremental_work`
  And the developer runs `cargo test timeline_items_apply_append_and_set_diffs`
  And the developer runs `cargo test timeline_items_apply_remove_truncate_clear_reset_diffs`
  Then all commands complete with exit code `0`

## Out of Scope

- Fixing iOS IME candidate-bar / keyboard frame handling in Makepad platform code
- Changing Makepad `PortalList` internals
- Replacing Makepad with native Android `RecyclerView`, Compose `LazyColumn`, or iOS `UITableView`
- Full room-list startup pagination refactor for `entries_with_dynamic_adapters(usize::MAX)`
- Media cache redesign, avatar cache redesign, and message rendering visual redesign
