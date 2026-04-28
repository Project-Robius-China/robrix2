spec: task
name: "Mobile Room Header Layout & Info Button Alignment"
inherits: project
tags: [bugfix, mobile, room-screen, header, navigation]
estimate: 0.5d
---

## Intent

修复移动端房间页面顶部导航栏样式异常的问题，并确保房间 `info` 图标位于右侧，与返回 icon 水平对齐。
当前问题主要表现为：顶部区域视觉高度/留白不正确，且房间信息入口图标位置不符合预期。

## Constraints

- 只修复移动端 `StackNavigation` 下的房间页顶部导航栏
- 保持现有导航行为不变（返回按钮、房间切换栈行为）
- 保持现有 Room Info 打开逻辑不变（点击 `info` 仍触发 `MessageAction::ShowRoomInfoPane`）
- `Invite` 与 `SpaceLobby` 页面不显示顶部 `info` 图标
- 不修改桌面端布局和样式

## Decisions

- 调整移动端公共内容容器 `RobrixContentView` 的 header 布局参数，使顶部栏高度与内容区域一致，避免异常顶部留白/错位
- 使用继承合并方式覆盖 header 右侧按钮，确保 `info` 图标在 header 右侧
- 左右按钮使用同一垂直对齐基准，确保 `info` 图标与返回 icon 水平对齐
- 保留现有条件显示逻辑：
  - `JoinedRoom` / `Thread` 显示 `info`
  - `InvitedRoom` / `Space` 隐藏 `info`

## Boundaries

### Allowed Changes
- `src/home/home_screen.rs` — 移动端 `RobrixContentView` header 样式与右侧按钮布局
- `src/app.rs` — 若需要，微调移动端 header 右侧 `info` 按钮显隐与状态同步

### Forbidden
- 不修改桌面端 `MainDesktopUI` 布局
- 不改动 Room Info pane 内部交互逻辑
- 不改动消息列表、输入框、时间线业务逻辑
- 不新增依赖
- 不运行 `cargo fmt` 或进行无关格式化

## Acceptance Criteria

Scenario: Mobile room header no longer shows incorrect top style
  Test: manual_test_mobile_room_header_style_fixed
  Given 用户在移动端进入 JoinedRoom
  When 房间页面显示顶部导航栏
  Then 顶部不再出现异常留白或错位样式
  And 顶部导航栏高度与正文起始位置视觉一致

Scenario: Info icon is displayed on the right for JoinedRoom and Thread
  Test: manual_test_mobile_room_header_info_visible_on_chat_views
  Given 用户在移动端进入 JoinedRoom 或 Thread
  When 页面顶部导航栏渲染完成
  Then `info` 图标可见
  And 图标位于顶部导航栏右侧

Scenario: Info icon aligns horizontally with back icon
  Test: manual_test_mobile_room_header_info_back_horizontal_alignment
  Given 用户在移动端进入 JoinedRoom 或 Thread
  When 页面顶部导航栏渲染完成
  Then 左侧返回 icon 与右侧 `info` icon 在同一水平线上

Scenario: Info icon remains hidden on Invite and SpaceLobby
  Test: manual_test_mobile_room_header_info_hidden_on_invite_and_space
  Given 用户在移动端进入 Invite 页面或 SpaceLobby 页面
  When 页面顶部导航栏渲染完成
  Then 顶部右侧不显示 `info` 图标

Scenario: Clicking header info still opens room info pane
  Test: manual_test_mobile_room_header_info_click_opens_room_info
  Given 用户在移动端 JoinedRoom 页面且顶部 `info` 图标可见
  When 用户点击 `info` 图标
  Then 触发 `MessageAction::ShowRoomInfoPane`
  And Room Info pane 正常打开

## Out of Scope

- 重新设计移动端整体导航视觉风格
- 调整 status bar/系统安全区域策略以外的全局窗口策略
- 修改 Room Info pane 的内容和交互细节
