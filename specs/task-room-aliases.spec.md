spec: task
name: "Room Aliases — 在房间设置里查看与管理房间别名"
inherits: project
tags: [matrix, room-settings, alias, ui, directory]
estimate: 2d
---

## Intent

为 Room Settings 模态（`src/home/room_settings_modal.rs`）新增一个「房间别名（Room
Aliases）」区块，让有权限的用户能够**查看并管理**当前房间的别名：

- 展示房间的 **canonical alias**（主别名 `#room:server`）与全部 **alt aliases**（备用别名）。
- **发布新别名**：把用户输入的别名（`#localpart:server`，或裸 `localpart` 按当前
  homeserver 解析）注册到房间目录（room directory），映射到本房间。
- **删除别名**：把某个别名从房间目录解绑。
- **设置 / 更换 canonical alias**：写 `m.room.canonical_alias` 状态事件（`alias` +
  `alt_aliases`），或清空主别名。
- **权限门控**：仅当用户具备发送 `m.room.canonical_alias` 状态事件的 power level 时，
  才显示编辑控件；否则该区块只读展示。

**读取侧已经存在**：`RoomsList`（`src/home/rooms_list.rs`）已从 matrix-sdk 的
`room.canonical_alias()` / `room.alt_aliases()` 填充 `canonical_alias:
Option<OwnedRoomAliasId>` 与 `alt_aliases: Vec<OwnedRoomAliasId>`。本任务只新增
**写入/管理**链路与对应 UI，不重写读取模型。

## Decisions

- **UI 落点**：别名区块放在 `room_settings_modal.rs` 现有设置列表内，作为一个新分组。
  当前 `show_settings(cx, room_id, room_name, "", alias_str)` 已把 canonical alias 以只读
  字符串传入；本任务把它升级为「canonical + alt aliases 列表 + 增删/设主控件」。
- **数据来源**：区块渲染的别名数据取自已 fetch 的房间设置（`RoomSettingsFetchedAction`，
  `sliding_sync.rs:702`）与 `RoomsList` 中的 `canonical_alias` / `alt_aliases`；不新增
  独立的别名缓存。请求侧沿用 `MatrixRequest::FetchRoomSettings` 触发的刷新。
- **新增 MatrixRequest 变体**（在 `sliding_sync.rs` 定义并处理，经
  `submit_async_request` 派发，**不得**裸开 tokio 任务）：
  - `MatrixRequest::PublishRoomAlias { room_id, alias }` — 目录注册（PUT
    `/directory/room/{alias}`，对应 ruma `create_room_alias` 端点）。
  - `MatrixRequest::RemoveRoomAlias { room_id, alias }` — 目录解绑（DELETE
    `/directory/room/{alias}`，对应 ruma `delete_room_alias` 端点）。
  - `MatrixRequest::SetRoomCanonicalAlias { room_id, alias: Option<OwnedRoomAliasId>,
    alt_aliases: Vec<OwnedRoomAliasId> }` — 发送 `m.room.canonical_alias` 状态事件。
  > 具体 matrix-sdk / ruma 调用符号由实现者对照本仓 pinned 版本确认；本 spec 只约束
  > 语义（目录端点 + canonical_alias 状态事件），不锁定 API 符号名。
- **输入规范化与校验（纯函数，可单测）**：新增
  `normalize_and_validate_alias(input: &str, homeserver: &ServerName) ->
  Result<OwnedRoomAliasId, AliasInputError>`：
  - `#localpart:server` → 直接按 `RoomAliasId` 解析。
  - 裸 `localpart`（不含 `#` 与 `:`）→ 补成 `#localpart:{当前 homeserver}` 再解析
    （与 `src/home/add_room.rs` 的 `parse_address` 一致的「localpart 落到当前
    homeserver」语义）。
  - 空串、缺 `#`、缺 `:server`、含空白、非法字符 → 返回 `AliasInputError` 的对应变体。
- **canonical 与 alt 的一致性（纯函数，可单测）**：新增
  `reconcile_canonical_alias(current_canonical, current_alts, op)` 计算写入
  `m.room.canonical_alias` 的目标 `(alias, alt_aliases)`：
  - 设某别名为 canonical 时，它必须已在「canonical ∪ alts」集合内（否则先要求发布）；
    旧的 canonical 降级进 alt_aliases。
  - 删除某别名时，从 alt_aliases 移除；若删的是当前 canonical，则 canonical 置空。
  - 结果里 canonical 不得同时出现在 alt_aliases（去重）。
- **乐观 UI + 结果回执**：发起写请求后本地乐观更新列表，成功/失败经现有 room settings
  刷新（`FetchRoomSettings` 回灌）与一条 toast 通知反馈；失败时回滚乐观项。
- **权限门控**：编辑控件的可见性由「能否发 `m.room.canonical_alias` 状态事件」决定
  （复用房间 power levels）。目录发布/解绑在服务端另有权限，客户端失败时以 toast 呈现
  服务端错误，不做本地静默吞掉。
- **i18n**：新增 key（`resources/i18n/` 下所有 locale 都要，中英必备）：
  `room_settings.aliases.section_title`、`room_settings.aliases.canonical_label`、
  `room_settings.aliases.alt_label`、`room_settings.aliases.add_placeholder`、
  `room_settings.aliases.add_button`、`room_settings.aliases.remove_button`、
  `room_settings.aliases.set_canonical_button`、`room_settings.aliases.invalid_format`、
  `room_settings.aliases.publish_failed`、`room_settings.aliases.readonly_hint`。
- **UI 规范**：颜色/圆角/字体一律走 `RBX_*` 设计 token 与现有 `room_settings_modal`
  的行/徽章样式，不硬编码 hex、不另起卡片风格（遵循 `docs/ui-visual-spec-zh.md`）。

## Boundaries

### Allowed Changes
- src/home/room_settings_modal.rs
- src/sliding_sync.rs
- src/app.rs
- src/i18n.rs
- resources/i18n/**
- specs/task-room-aliases.spec.md

### Forbidden
- 不要新增 cargo 依赖（matrix-sdk / ruma 已提供别名与目录端点）。
- 不要重写 `RoomsList` 的别名**读取**模型（`canonical_alias` / `alt_aliases` 已存在）。
- 不要为别名操作裸开 tokio 任务——必须走 `submit_async_request(MatrixRequest::*)`。
- 不要在 UI 屏里硬编码 hex 颜色；不要用 Makepad 1.x `live_design!` 语法。
- 不要用 `.unwrap()` 处理用户输入的别名（非法输入必须走 `AliasInputError`）。
- 不要在本任务里改动房间**加入**（join-by-alias，`add_room.rs`）的既有逻辑。

## Completion Criteria

场景: 合法的完整别名被接受
  测试: test_normalize_alias_accepts_full_alias
  假设 当前 homeserver 为 "example.org"
  当 规范化并校验输入 "#general:example.org"
  那么 返回合法的 RoomAliasId "#general:example.org"

场景: 裸 localpart 按当前 homeserver 补全
  测试: test_normalize_alias_completes_bare_localpart
  假设 当前 homeserver 为 "example.org"
  当 规范化并校验输入 "general"
  那么 返回合法的 RoomAliasId "#general:example.org"

场景: 非法别名输入被拒绝
  测试: test_normalize_alias_rejects_invalid
  假设 当前 homeserver 为 "example.org"
  当 分别规范化并校验输入 ""、"#:example.org"、"#has space:example.org" 和 "#general"
  那么 每个输入都返回 AliasInputError，而不是 panic

场景: 设置某个已存在的别名为 canonical，旧主别名降级为 alt
  测试: test_reconcile_promote_alias_to_canonical
  假设 当前 canonical 为 "#old:example.org"
  并且 当前 alt_aliases 包含 "#new:example.org"
  当 把 "#new:example.org" 设为 canonical
  那么 目标 canonical 为 "#new:example.org"
  并且 目标 alt_aliases 包含 "#old:example.org"
  但是 目标 alt_aliases 不包含 "#new:example.org"

场景: 把尚未发布的别名设为 canonical 被拒绝
  测试: test_reconcile_rejects_unpublished_canonical
  假设 当前 canonical 为 "#old:example.org"
  并且 当前 alt_aliases 为空
  当 把未在集合内的 "#ghost:example.org" 设为 canonical
  那么 返回错误，要求先发布该别名

场景: 删除当前 canonical 别名会清空主别名
  测试: test_reconcile_remove_canonical_clears_it
  假设 当前 canonical 为 "#main:example.org"
  并且 当前 alt_aliases 包含 "#alt:example.org"
  当 删除 "#main:example.org"
  那么 目标 canonical 为空 (None)
  并且 目标 alt_aliases 仍包含 "#alt:example.org"

场景: canonical 不会同时出现在 alt_aliases 里
  测试: test_reconcile_dedups_canonical_from_alts
  假设 当前 canonical 为 "#old:example.org"
  并且 当前 alt_aliases 同时包含 "#dup:example.org"
  当 把 "#dup:example.org" 设为 canonical
  那么 目标 alt_aliases 不包含 "#dup:example.org"

场景: 别名区块的 i18n key 在所有 locale 中都存在
  测试: test_room_aliases_i18n_keys_exist_in_all_locales
  假设 en 与 zh-CN 两套字典已加载
  当 用 tr_key 解析 "room_settings.aliases.section_title"、"room_settings.aliases.add_button" 和 "room_settings.aliases.invalid_format"
  那么 每个 key 在两种语言下都返回非 key 本身的翻译文案

## Out of Scope

- 房间**加入**流程（join-by-alias）的任何改动——已由 `add_room.rs` 覆盖。
- 别名的服务端权限管理 / 房间目录可见性（`m.room.history_visibility`、published-in-directory 开关）。
- Space 的 canonical alias 管理（`space_service_sync.rs` 另行处理）。
- 跨 homeserver 的别名迁移或批量导入。
- 别名冲突（already-in-use）的自动改名建议——仅以服务端错误 toast 呈现。
- 平台端到端验收（macOS / Android 手动测试）在实现 PR 中确认，本 spec 场景只绑定纯函数单测。
