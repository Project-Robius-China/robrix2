spec: task
name: "移动端登录界面视觉重构 —— 浅色响应式布局"
inherits: project
tags: [ui, login, mobile, design-tokens, redesign, makepad]
estimate: 1d
---

## Intent

在已合并的桌面登录重构基础上，为 `src/login/login_screen.rs` 增加移动端浅色登录布局，
对齐 `docs/ui-visual-spec-zh.md` §5.6 的登录信息结构。移动端应使用与桌面一致的浅色品牌体系，
但采用移动字号/间距、移动 SSO 图标栅格、卡外状态 footer 与版本号；
登录 / SSO / OIDC / proxy / add-account / capability-probe 的业务逻辑与功能 widget id 必须继续复用。

## Constraints

- 移动端浅色布局必须使用 `RBX_BG_CANVAS`、`RBX_BG_SURFACE`、`RBX_STROKE_SOFT`、`RBX_ACCENT` 等既有 token；登录界面内禁止新增裸 hex
- 在全局深色 / 浅色主题切换完成前，移动登录不得单独引入 `RBX_LOGIN_BG` / `RBX_LOGIN_SURFACE` 深色变体
- 桌面浅色变体必须保留：浅色背景、浅色卡片、`Agent-ready workspace` badge、494px 桌面卡片上限、422px 内容列上限
- 移动端不得新增 / 删除 SSO provider，不得把现有六家 provider 改成参考稿里的 Microsoft / More；只允许改布局与展示标签
- 必须保留现有功能 id：`user_id_input`、`password_input`、`confirm_password_input`、`homeserver_input`、`login_button`、`mode_toggle_button`、`cancel_button`、`sso_view`、六个 SSO 按钮、`login_status_modal`、`proxy_settings_modal`
- 不得改动 `LoginAction` 枚举、Matrix 登录请求、OIDC / SSO provider 映射、proxy 保存逻辑
- 移动端新增用户可见文案必须走 `resources/i18n/en.json` 与 `resources/i18n/zh-CN.json`

## Decisions

- 响应式策略：不复制登录业务子树；同一棵 `LoginScreen` widget 根据窗口宽度切换 desktop/mobile layout，避免功能 id 分叉
- 移动阈值：桌面窗口宽度 `<= 700px` 进入 mobile login layout，宽屏恢复 desktop layout；Android / iOS 目标直接进入 mobile login layout，避免高密度设备物理像素宽度被误判为桌面
- 宽度策略：桌面保留新版 `494px` 卡片 / `422px` 内容列视觉；移动端学习 `main` 分支的稳定布局思路，即外层 `Fill` 居中 + 内层窄列约束，避免桌面 422px 表单直接挤进手机宽度
- 移动视觉：页面背景用 `RBX_BG_CANVAS`，登录卡片/输入框用 `RBX_BG_SURFACE`，文本使用浅色体系的 foreground token；深色登录留待全局主题切换统一实现
- 移动表单：保留 `User ID` / `Password` / `Homeserver URL` 字段 label id，并使用完整、不裁切的 placeholder（例如 "Enter your user ID"）
- 移动 SSO：保持现有 Apple / Facebook / GitHub / GitLab / Google / X provider 集合，移动端展示为稳定的 3x2 图标栅格；桌面端仍保持紧凑图标行
- 移动 footer：状态 footer 与 `v2.0.0` 版本号放在登录卡片外；桌面端继续使用卡片内 footer

## Boundaries

### Allowed Changes
- src/login/login_screen.rs
- docs/ui-visual-spec-zh.md
- resources/i18n/en.json
- resources/i18n/zh-CN.json
- specs/task-login-mobile-redesign.spec.md

### Forbidden
- 不要修改 Matrix 登录、注册、SSO、OIDC、proxy 的业务控制流
- 不要删除 / 重命名现有功能 widget id
- 不要在登录界面写裸 hex 颜色字面量
- 不要新增 cargo 依赖
- 不要提交 `image.png`

## Out of Scope

- 改造 RegisterScreen
- 新增 Microsoft / More provider
- 全量 dark mode
- 登录背景复杂动效 / 新位图资产
- 改变桌面登录界面的业务行为

## Completion Criteria

场景: 移动端宽度和移动平台进入浅色登录布局
  测试: test_mobile_login_layout_target_detection
  假设 登录页收到窗口宽度
  当 宽度为 700px 或更窄
  那么 登录页判定为 mobile layout
  并且 宽度为 701px 时判定为 desktop layout
  并且 Android / iOS 目标即使物理像素宽度为 1272px 也判定为 mobile layout

场景: 移动登录使用浅色 token 且不单独引入深色登录 token
  测试: test_login_screen_source_contains_mobile_light_contract
  假设 登录界面同时支持 desktop 与 mobile
  当 读取 login_screen.rs 源码
  那么 源码包含 "RBX_BG_CANVAS"
  并且 源码仍包含 "RBX_BG_SURFACE"
  并且 源码不包含 "RBX_LOGIN_BG"
  并且 源码不包含 "RBX_LOGIN_SURFACE"
  并且 登录界面不包含裸 hex 颜色字面量

场景: 移动表单保留字段 label id 与移动 placeholder
  测试: test_mobile_login_form_labels_and_placeholders_are_i18n_bound
  假设 移动登录需要稳定输入提示
  当 读取 login_screen.rs 与 i18n 字典
  那么 源码包含 `user_id_field_label`
  并且 源码包含 `password_field_label`
  并且 源码包含 `homeserver_field_label`
  并且 英文与中文字典包含移动 placeholder 键

场景: 移动 SSO 使用 3x2 图标栅格但不改 provider 集合
  测试: test_mobile_sso_grid_preserves_provider_ids
  假设 移动端需要 3x2 SSO 图标栅格
  当 读取 login_screen.rs 源码
  那么 源码包含移动端 SSO 容器宽度约束
  并且 六个现有 SSO 按钮 id 仍存在
  并且 源码不新增 "microsoft_button"
  并且 源码不新增 "more_button"

场景: 移动 footer 在卡片外展示状态和版本
  测试: test_mobile_login_footer_contract
  假设 移动参考稿要求卡外状态 footer 与版本号
  当 读取 login_screen.rs 源码
  那么 源码包含 `mobile_status_footer`
  并且 源码包含 `mobile_version_label`
  并且 desktop footer 仍保留
