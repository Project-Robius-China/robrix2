spec: task
name: "ruma lax sliding-sync-v5 deserialization"
inherits: project
tags: [dependency, sliding-sync, bugfix, ruma]
estimate: 0.5d
---

## Intent

修复 robrix2 运行期间收到的邀请/服务端新建房间永不显示的 sliding-sync 房间物化 bug:sync v5 增量响应中 per-room 对象因个别字段损坏而整体反序列化失败,matrix-sdk-base 静默丢弃该房间。采用最小增量方案——把 ruma#2510 的容错反序列化 cherry-pick 到当前已锁定的 ruma rev 上并启用对应 feature,完全不改动 matrix-sdk fork。决策背景与依赖链详见 docs/deps-ruma-fork.md。

## Decisions

- ruma patch 源:`Project-Robius-China/ruma` 分支 `tsp-lax-syncv5-deser`(基底 = 原锁定 rev `98196b1d`,ruma 0.14.1 + 唯一增量 commit `20c2c60f4` = ruma#2510 cherry-pick)
- 启用 feature:`unstable-compat-lax-syncv5-deser`(加在 robrix2 直接 ruma 依赖上,经 Cargo feature 统一化传导给 matrix-sdk-base 内的同一 ruma 实例)
- matrix-sdk 三 crate 依赖保持 `space_room_suggested-event-cache-no-panic` 分支不变
- 不整体升级 matrix-sdk:官方已移除 rustls `ring` provider(与 Android workaround 冲突)且 `libsqlite3-sys` 与 TSP/sqlx 链版本互斥,rebase 路线明确搁置
- ruma fork 分支只允许追加 cherry-pick,禁止 rebase/force-push;matrix-sdk 未来升级官方版时本分支与 patch 一并退役

## Boundaries

### Allowed Changes
- Cargo.toml
- Cargo.lock
- docs/deps-ruma-fork.md
- specs/task-ruma-lax-syncv5-deser.spec.md

### Forbidden
- 不修改任何 src/ 下的 Rust 源码(本任务是纯依赖层变更)
- 不改动 matrix-sdk / matrix-sdk-base / matrix-sdk-ui 的分支或 rev
- 不升级 ruma 主版本(保持 0.14.1,不引入 tsp 分支 HEAD 的 0.16 演进)

## Out of Scope

- 整体 rebase matrix-sdk fork 到官方(受阻原因见 docs/deps-ruma-fork.md)
- Palpo 服务端在增量响应中重发带外变更房间的修复
- robrix2 侧对 must-exist 的 phantom 房间检测/强制 resync 兜底

## Completion Criteria

### Rule: dependency-pinning — 依赖锁定正确

场景: 依赖解析指向补丁分支(critical)
  标签: critical
  测试: manual_test_cargo_tree_ruma_resolves_to_patch_branch
  层级: manual
  审核: human
  假设 工作区为 fix/ruma-lax-syncv5-deser 分支
  当 执行 `cargo tree -p ruma --depth 0`
  那么 输出的 ruma 来源为 `Project-Robius-China/ruma.git?branch=tsp-lax-syncv5-deser#20c2c60f4`
  并且 版本为 "0.14.1"

场景: matrix-sdk 依赖源保持不变
  测试: manual_test_matrix_sdk_source_unchanged
  层级: manual
  审核: human
  当 对比 Cargo.lock 中 matrix-sdk、matrix-sdk-base、matrix-sdk-ui 的 source 与 main 分支
  那么 三者均仍为 `space_room_suggested-event-cache-no-panic` 分支的同一 rev

场景: 全量编译通过(critical)
  标签: critical
  测试: manual_test_cargo_check_passes
  层级: manual
  审核: human
  当 执行 `cargo check`
  那么 退出码为 0

### Rule: room-materialization — 损坏字段不再丢房间

场景: 运行期邀请即时显示(critical)
  标签: critical
  测试: manual_test_live_invite_materializes_without_restart
  层级: manual
  审核: human
  假设 robrix2 正在运行且已完成初始同步
  当 另一账号向当前用户发送房间邀请
  那么 该邀请无需重启即出现在 Invites 列表
  但是 日志不出现 "The room must exist since it has been joined"

场景: 真实失败 payload 在两版 ruma 上重放归因(critical)
  标签: critical
  测试: manual_test_replay_captured_payload_on_both_ruma_revs
  层级: manual
  审核: human
  假设 已捕获一份真实失败的 sync v5 per-room 响应 JSON(通过开启 SDK HTTP 日志或代理抓包,在 main 构建复现房间不显示时截取)
  当 将完全相同的 payload 分别输入旧 ruma rev(98196b1d)与本分支 ruma rev(20c2c60f4)的 sync v5 响应反序列化(可仿照 ruma#2510 自带单测构造)
  那么 旧 rev 对该 per-room 对象反序列化失败,新 rev 保留房间对象且仅将非法字段置空
  并且 归因前提是 payload 确实含四个受宽松处理字段之一的非法值:room name、room avatar、hero displayname、hero avatar_url(ruma#2510 仅覆盖这四个;若捕获的 payload 不含其中任何非法值,则本修复不是该次失败的原因)

场景: 运行期 live A-B 对照(辅助证据)
  测试: manual_test_ab_comparison_against_main
  层级: manual
  审核: human
  假设 同一账号与同一 homeserver,分别运行 main 分支与本分支构建
  当 在两个构建下重复执行 运行期邀请与服务端建房场景
  那么 main 构建复现房间不显示,本分支构建房间即时显示
  但是 两次服务端响应可能不同,本场景仅作辅助证据,严格归因以 payload 重放场景为准

场景: 运行期服务端建房即时显示
  测试: manual_test_server_created_room_materializes_without_restart
  层级: manual
  审核: human
  假设 robrix2 正在运行且已完成初始同步
  当 服务端 bot 创建新房间并将当前用户 force-join
  那么 该房间无需重启即出现在房间列表

场景: 含损坏字段的房间对象仅丢字段不丢房间
  测试: manual_test_corrupt_field_drops_field_not_room
  层级: manual
  审核: human
  假设 homeserver 在某房间的 sync v5 响应中发送非法类型的 room name、room avatar、hero displayname 或 hero avatar_url 字段
  当 robrix2 处理该增量响应
  那么 该房间仍被物化并显示(对应字段回退为空)
  但是 整个 per-room 对象不因单字段损坏而被丢弃

### Rule: no-regression — 健康路径零回归

场景: 健康响应行为不回归
  测试: manual_test_healthy_sync_behavior_unchanged
  层级: manual
  审核: human
  假设 robrix2 与账号内既有房间均正常
  当 执行 启动全量同步、收发消息、房间列表增量更新
  那么 行为与变更前一致(该 feature 仅在字段损坏时生效)

<!-- lint-ack: error-path — 核心场景"含损坏字段的房间对象仅丢字段不丢房间"即异常路径:输入是非法/损坏字段;本任务无用户输入面,无其他可枚举失败路径 -->
<!-- lint-ack: decision-coverage — "不整体升级 matrix-sdk"是路线排除决策(不做什么),其可验证面即 matrix-sdk 依赖源保持不变,已由 manual_test_matrix_sdk_source_unchanged 覆盖 -->
<!-- lint-ack: observable-decision-coverage — "matrix-sdk 依赖源保持不变"由 manual_test_matrix_sdk_source_unchanged 场景经 Cargo.lock 对比覆盖,无 stdout/文件级可观察行为 -->
