spec: task
name: "Space usability: persistence, deep links, removal feedback, directory toggle"
inherits: project
tags: [feature, space, navigation, i18n]
estimate: 2d
depends: [task-space-tree-hardening]
---

## Intent

补齐 2026-08-17 核实确认的四项 Space 功能缺失:选中空间重启后不恢复(回 Home)、时间线里 space 的 matrix.to 链接被当普通房间处理、被 ban/远程离开空间时零反馈、公共目录页无法切换到 Spaces。设计依据 docs/superpowers/specs/2026-08-18-space-completion-design.md。

## Decisions

- `AppState` 增 `selected_space_id: Option<OwnedRoomId>`,`#[serde(default)]` 兼容旧状态文件;恢复在空间列表异步到达后应用,目标不可用时安全回退 Home;不改动 `saved_dock_state_per_space` 既有机制
- 深链接:`MatrixId::Room` 分支经 `client.get_room()` 判断 room type;已加入 space 走 `NavigationBarAction::GoToSpace`;本地可知是 space 但未加入时 join modal 置 `is_space: true`;本地未知房间维持现有普通房间流程(不发预览请求探测,决策 A)
- banned/left 反馈用现有 `enqueue_popup_notification`(PopupKind 机制),不新建 UI 面;knocked 维持忽略并以注释说明(决策 B:重新加入/取消 knock 入口留下轮)
- 目录页:`directory_screen.rs` 的 `DirectoryRoomKind` 参数化,页头加 Rooms/Spaces 切换,复用既有 toggle 样式与 RBX token
- 新增用户可见文案一律同步 `resources/i18n/en.json` 与 `resources/i18n/zh-CN.json`
- Matrix 写操作一律走 `submit_async_request(MatrixRequest::*)`

## Boundaries

### Allowed Changes
- src/app.rs
- src/home/room_screen/interactions.rs
- src/space_service_sync.rs
- src/home/directory_screen.rs
- src/home/spaces_bar.rs
- src/home/rooms_list.rs
- src/persistence/**
- resources/i18n/en.json
- resources/i18n/zh-CN.json
- specs/task-space-usability.spec.md

### Forbidden
- 不新增 cargo 依赖
- 不硬编码 hex 颜色(用 RBX_* token)
- 不在 UI 直接 spawn tokio 任务
- 不运行 cargo fmt

## Out of Scope

- restricted join rule 配置器、拖拽排序、父链显示
- "已离开空间"管理列表、重新加入/取消 knock 操作入口
- 未知房间的 room type 预览探测
- ProjectRef / HAFleet 绑定

## Completion Criteria

### Rule: space-persistence — 选中空间跨重启恢复

场景: 重启恢复选中空间(critical)
  标签: critical
  测试: manual_test_selected_space_restored_after_restart
  层级: manual
  审核: human
  假设 用户选中某空间并打开其中房间后正常退出应用
  当 重新启动应用
  那么 启动后回到该空间视图且其 dock 布局恢复

场景: 已退出空间安全回退
  测试: manual_test_stale_selected_space_falls_back_home
  层级: manual
  审核: human
  假设 持久化的选中空间在启动前已被该账号退出
  当 重新启动应用
  那么 应用回到 Home 且无报错、无空白视图

场景: 旧状态文件兼容
  测试: app_state_without_selected_space_deserializes
  假设 一份不含 selected_space_id 字段的旧 AppState JSON
  当 反序列化该状态
  那么 成功且 selected_space_id 为 None

### Rule: space-deep-link — 时间线 space 链接正确路由

场景: 已加入空间链接进 Lobby(critical)
  标签: critical
  测试: manual_test_joined_space_link_opens_lobby
  层级: manual
  审核: human
  假设 时间线消息含指向用户已加入 space 的 matrix.to 链接
  当 用户点击该链接
  那么 导航到该 space 的 Space Lobby 而非普通房间时间线

场景: 未加入空间链接标记为空间
  测试: manual_test_unjoined_space_link_join_modal_is_space
  层级: manual
  审核: human
  假设 时间线链接指向本地可知为 space 但未加入的房间
  当 用户点击该链接
  那么 加入确认弹窗以空间语义展示(is_space 为 true)

场景: 未知房间维持原行为
  测试: manual_test_unknown_room_link_unchanged
  层级: manual
  审核: human
  假设 链接指向本地完全未知的房间
  当 用户点击该链接
  那么 行为与本变更前一致(普通房间加入流程)

### Rule: space-removal-feedback — 失去空间成员资格有反馈

场景: 被移出空间弹通知(critical)
  标签: critical
  测试: manual_test_banned_from_space_shows_popup
  层级: manual
  审核: human
  假设 robrix2 正在运行且用户是某空间成员
  当 用户被管理员从该空间 ban 或 kick
  那么 弹出包含空间名称的通知且空间从侧边栏消失
  但是 应用不崩溃、不残留空白视图

场景: 正浏览的空间被移除
  测试: manual_test_removed_while_viewing_space
  层级: manual
  审核: human
  假设 用户正在浏览某空间的 Lobby
  当 用户被移出该空间
  那么 视图安全离开该空间且收到通知

### Rule: directory-spaces — 目录页可浏览公共空间

场景: 目录页切换到 Spaces(critical)
  标签: critical
  测试: manual_test_directory_spaces_toggle
  层级: manual
  审核: human
  假设 打开公共目录页
  当 切换到 Spaces 标签并搜索
  那么 结果只含 space 类型条目且可发起加入

场景: Spaces 空结果有区分文案
  测试: manual_test_directory_spaces_empty_state
  层级: manual
  审核: human
  假设 Spaces 标签下搜索无结果
  当 结果返回空
  那么 显示 space 语义的空态文案且中英文案均存在

场景: 中英文案完整
  测试: manual_test_directory_i18n_complete
  层级: manual
  审核: human
  假设 应用语言分别切换为中文与英文
  当 浏览目录页 Spaces 标签全部状态
  那么 无缺失翻译键或英文回退

<!-- lint-ack: error-path — 已退出空间回退、未知房间回退、被移除场景、空结果均为失败/异常路径,数量不少于 happy path -->
