spec: task
name: "设置图标登出后状态重置"
inherits: project
tags: [bugfix, navigation, logout, login, state-management, ui]
estimate: 0.5d
---

## 意图

修复一个导航状态不一致问题：用户点击侧边栏设置图标进入设置页后执行退出登录，再次登录时主界面已显示为 Home，但设置图标仍保持选中态，且再次点击设置图标无效。任务目标是确保退出/重登后页面状态与导航图标状态一致，并保证设置图标可再次正常触发。

## 已定决策

- `selected_tab` 与 `NavigationTabBar` 视觉选中态必须保持同步
- 当登出流程触发 `ClearAppState` 并将 `selected_tab` 重置为 `Home` 时，必须显式同步导航按钮状态（通过统一 action 或等效机制）
- 仅做最小修复，不改变现有导航架构（`GoTo*` / `CloseSettings` / `TabSelected` 的职责边界保持不变）
- 修复后，点击已高亮图标不触发 action 的现象不能再阻塞“重进设置页”

## 边界

### 允许修改
- src/app.rs
- src/home/home_screen.rs
- src/home/navigation_tab_bar.rs
- src/logout/logout_state_machine.rs
- 与上述逻辑直接相关的少量调用点（如 action 分发/转发）

### 禁止做
- 不修改无关 UI 样式与布局
- 不引入新依赖
- 不重构整套导航系统
- 不修改登录/登出协议层行为（仅修复 UI 状态同步）

## 完成条件

场景: 设置页登出后重登，导航高亮恢复正确
  Level: manual-e2e
  层级: 手工-E2E
  测试: manual_test_settings_tab_state_after_relogin
  假设 用户已登录，且当前位于设置页（设置图标处于选中态）
  当 用户执行退出登录并再次成功登录
  那么 主内容页面显示 Home
  并且 Home 图标为选中态
  并且 Settings 图标为未选中态

场景: 重登后设置图标可以再次点击进入设置页
  Level: manual-e2e
  层级: 手工-E2E
  测试: manual_test_settings_tab_clickable_after_relogin
  假设 用户完成“设置页 -> 退出登录 -> 重新登录”流程
  当 用户点击一次设置图标
  那么 设置页成功打开
  并且不会出现“点击无效”现象

场景: selected_tab 与 NavigationTabBar 视觉选中态保持同步
  Level: manual-state-consistency
  层级: 手工-状态一致性验证
  测试: manual_test_tab_state_sync_on_clear_app_state
  假设 登出流程触发了 `LogoutAction::ClearAppState` 并把 `selected_tab` 设为 `Home`
  当 后续 UI 继续处理导航相关 action（包括但不限于 `CloseSettings`）
  那么 NavigationTabBar 的视觉选中态与 `selected_tab` 一致

场景: 非该缺陷路径行为保持不变
  Level: manual-regression-smoke
  层级: 手工-回归冒烟
  命中: NavigationTabBar, HomeScreen
  Targets: NavigationTabBar, HomeScreen
  Test Double: none
  测试: manual_test_navigation_regression_smoke
  假设 用户在正常登录态使用 Home、AddRoom、Settings 切换
  当 用户进行常规导航点击
  那么 各页面切换与图标选中行为与修复前一致（除本缺陷外）

## 排除范围

- 不在本任务内新增自动化 UI 测试框架
- 不处理“空间页（Space）选中态”的独立历史问题（若存在）
- 不处理账户切换流程中的其他潜在状态一致性问题（除本缺陷直接相关路径外）
