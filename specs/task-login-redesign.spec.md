spec: task
name: "登录界面视觉重构 —— 对齐 UI 视觉 Spec §5.6（桌面浅色 + 移动深色两变体）"
inherits: project
tags: [ui, login, design-tokens, redesign, adaptive, makepad]
estimate: 2d
---

## Intent

把 `src/login/login_screen.rs` 的登录界面重构为新版「AI 工作空间」视觉语言，对齐
`docs/ui-visual-spec-zh.md` §5.6 与参考稿 `docs/ui-reference/01-desktop-login.png`
（桌面浅色）、`docs/ui-reference/05-mobile-login.png`（移动深色）。目标结构：品牌卡
（cube logo + `Robrix2` 字标 + `Agent-native collaboration client` 副标题）→ `User ID /
Password(👁) / Homeserver URL` → teal 主 CTA **Sign in securely** → `Or continue with`
分隔 → SSO 行 → `Create an account` → 状态 footer（Secure session · Self-host ready ·
Matrix connected）。

这是**纯视觉重构**：换皮 + 重排容器 + 新增展示性文案，登录 / SSO / OIDC / proxy /
add-account / capability-probe 的业务逻辑与所有功能 widget id 必须原样保留。基于
`robrix2-new-ui` 分支（design tokens 与视觉 spec 已落地的分支），作为新版前瞻，测试通过
后 PR 回 `robrix2-new-ui`。

**本轮范围（已与负责人确认）**：只做**桌面浅色**变体（= `01-desktop-login.png` / 用户提供的
image.png），单层布局，移动端也能显示（浅色）。移动端**深色**变体（`RBX_LOGIN_BG` +
`RBX_LOGIN_SURFACE`，经 `AdaptiveView` 双子树）作为紧随的下一步，**不在本 PR 范围**——本轮
结构需保持「日后能平滑加 `AdaptiveView { Desktop / Mobile }` 包裹」的形态。

## Constraints

- 颜色 / 圆角 / 字号一律用 `src/shared/design_tokens.rs` 的 `RBX_*` token（DSL：`(RBX_TOKEN)`）或既有 `styles.rs` token；登录界面内**禁止写裸 hex**（`#xRRGGBB` 字面量），状态色 / 中性色用既有 `RBX_*` 语义对
- 主 CTA「Sign in securely」必须用 teal `RBX_ACCENT`（按下 `RBX_ACCENT_PRESSED`、hover `RBX_ACCENT_HOVER`），**不得**用旧主色蓝 `COLOR_ACTIVE_PRIMARY` / `RBX_LEGACY_BLUE`
- 登录输入框必须使用登录局部 `LoginTextInput` 样式，聚焦态只允许克制地使用 `RBX_ACCENT` 边框与光标；hover / down 仍使用中性描边，不得继承旧蓝色 focus 或制造输入框外的色块毛边
- 本轮桌面浅色：页面底用 `RBX_BG_CANVAS`，登录卡面用 `RBX_BG_SURFACE` + `RBX_STROKE_SOFT` 描边 + `RBX_RADIUS_LG` 圆角；不得依赖重阴影建立层次
- 必须保留所有功能 widget 的命名子节点 id（`handle_actions` 经 `ids!()` 寻址）：`user_id_input` `password_input` `confirm_password_input` `homeserver_input` `show_password_button` `hide_password_button` `show_confirm_password_button` `hide_confirm_password_button` `login_button` `mode_toggle_button` `cancel_button` `oidc_card` `oidc_info_title` `oidc_info_body` `oidc_continue_button` `oidc_status_label` `oidc_cancel_button` `title` `sso_prompt_label` `account_prompt_label` `homeserver_hint_label` `sso_view` 及六个 SSO 按钮 `apple_button` `facebook_button` `github_button` `gitlab_button` `google_button` `twitter_button`（各含子 `image`）、`login_status_modal` `login_status_modal_inner` `proxy_settings_modal` 及其内部全部 id、`proxy_settings_button`
- 不得改动 `LoginScreen` 的 Rust 字段、`handle_actions` / `Widget` / `ScriptHook` 业务逻辑与 `LoginAction` 枚举；只允许在 `set_app_language` 内为新增展示标签补 `set_text` 绑定
- 不得改动 SSO provider 集合与 `provider_brands` 映射（保持现有六家）；参考稿里的 `Microsoft / More` 仅为示意，本轮不增删 provider
- 新增用户可见文案必须走 i18n：键加进 `resources/i18n/en.json` 与 `resources/i18n/zh-CN.json` 两个字典，经 `tr_key` 读取，禁止在 DSL/Rust 里硬编码英文串
- `draw_bg` / `draw_text` 属性一律 `+:` 合并，命名子节点用 `:=`；遵守视觉 spec §10 DSL gotchas（`script_mod!` 内只用 `//` 注释、含字母颜色用 `#x` 前缀）
- 不新增任何 cargo 依赖

## Decisions

- 目标视觉源：`docs/ui-reference/01-desktop-login.png`（= 用户提供的 image.png，已确认同一张稿）+ 视觉 spec §5.6；移动深色源 `05-mobile-login.png`
- 本轮单层桌面浅色布局，结构保持日后可被 `AdaptiveView { Desktop := ... Mobile := ... }` 包裹（移动深色下一步做）；功能 widget id 不变以便复用
- 新增 i18n 键（en + zh）：`login.subtitle.tagline`（"Agent-native collaboration client"）、`login.button.sign_in_securely`（"Sign in securely"）、`login.divider.or_continue_with`（"Or continue with"）、`login.account_prompt.new_to_robrix`（"New to Robrix?"）、`login.mode_toggle.create_account`（"Create an account"）、`login.footer.secure_session`（"Secure session"）、`login.footer.self_host_ready`（"Self-host ready"）、`login.footer.matrix_connected`（"Matrix connected"）
- 品牌字标 `Robrix2` 作为静态 Label（品牌名，不本地化）
- 参考稿右上角 "Agent-ready workspace" 徽章：作为登录卡片内的顶部右对齐普通行实现，不使用会覆盖输入区域的卡片级 overlay；底用 `RBX_ACCENT_SOFT`、字用 `RBX_ACCENT`、pill 圆角；文案 i18n 键 `login.badge.agent_ready`
- 桌面浅色卡片采用稳定几何：登录卡 `494px` 宽，主内容列 / 输入框 / CTA / footer 对齐到 `422px` 宽，卡片圆角使用 `RBX_RADIUS_LG`
- 登录输入框采用局部 `LoginTextInput = RobrixTextInput`：背景保持 `RBX_BG_SURFACE`，默认描边 `RBX_STROKE_SOFT`，hover / down 描边 `RBX_STROKE_STRONG`，focus 描边与光标使用 `RBX_ACCENT`，并开启 `clip_x` / `clip_y` 避免 Makepad 输入框内部绘制溢出
- Homeserver 说明文字使用独立 `homeserver_hint_row`，不再把 label 夹在紧贴输入框的左右分隔线中，避免输入框下沿出现蓝色 protrusion / 毛边 artifact
- 主 CTA 文案从 `login.button.login`("Login") 改用 `login.button.sign_in_securely`；旧键保留（capability-probe 进行中仍把 `login_button` 文案临时设回 `login.button.login` 风格，沿用既有键即可）
- 密码可见切换沿用既有 `show_password_button` / `hide_password_button` 眼睛图标（`ICON_EYE_OPEN/CLOSED`），位置叠加在 password_input 右侧
- 验收里凡涉及像素 / 布局对照（卡片留白、teal 色准、聚焦状态观感）由人工在桌面 QA；本 spec 的机械验收覆盖：i18n 键齐全、关键 token 接线、桌面卡片几何、登录输入框 focus 样式、homeserver hint 布局、登录逻辑谓词回归不变
- 验证用 `cargo run`（非 `--hot`），新增 Rust 模块注册不走 DSL 热更（视觉 spec §9）

## Boundaries

### Allowed Changes
- src/login/login_screen.rs
- docs/ui-visual-spec-zh.md
- resources/i18n/en.json
- resources/i18n/zh-CN.json
- specs/task-login-redesign.spec.md

### Forbidden
- 不要修改 `handle_actions` / `Widget::handle_event` / `ScriptHook` 的控制流或 `LoginAction` 枚举
- 不要删除 / 重命名任何上方「Constraints」列出的功能 widget id
- 不要把主 CTA 改成旧蓝 `COLOR_ACTIVE_PRIMARY` / `RBX_LEGACY_BLUE`
- 不要在登录界面写裸 hex 颜色字面量
- 不要增删 SSO provider 或改 `provider_brands`
- 不要新增 cargo 依赖
- 不要改动 `src/shared/design_tokens.rs` 的 token 值

## Out of Scope

- 移动端深色登录变体（`AdaptiveView` Mobile 子树 + `RBX_LOGIN_*` 深色面）—— 紧随的下一个 PR
- 全量 dark mode（视觉 spec §11）
- Settings / Timeline / Workbench / Room Detail 等其它界面（视觉 spec §8 后续 Phase）
- 注册（RegisterScreen）界面重构
- 改变 SSO provider 集合、新增 Microsoft / 折叠 "More" 溢出菜单
- proxy settings modal 的功能变更（仅允许同步换肤，逻辑不动）

## Completion Criteria

场景: 新增登录文案在英文字典齐全
  测试: test_login_redesign_i18n_keys_exist_en
  假设 已为登录重构新增展示性文案键
  当 用英文语言读取下列键:
    | key                                |
    | login.subtitle.tagline             |
    | login.button.sign_in_securely      |
    | login.divider.or_continue_with     |
    | login.mode_toggle.create_account   |
    | login.footer.secure_session        |
    | login.footer.self_host_ready       |
    | login.footer.matrix_connected      |
  那么 每个键解析为非空字符串
  并且 解析结果不等于键名本身

场景: 主 CTA 用 teal accent 且功能 id 保留
  测试: test_login_screen_source_wires_accent_token
  假设 登录界面已重构为新视觉
  当 读取 login_screen.rs 源码
  那么 源码引用 "RBX_ACCENT"
  并且 源码不再把主 CTA 设为旧蓝 "COLOR_ACTIVE_PRIMARY"
  并且 源码包含命名子节点 "login_button"

场景: 桌面登录卡片使用稳定桌面布局
  测试: test_login_screen_source_uses_desktop_card_contract
  假设 登录界面已重构为桌面浅色目标布局
  当 读取 login_screen.rs 源码
  那么 登录卡片使用 "width: 494"
  并且 主内容列控件使用 "width: 422"
  并且 登录卡片圆角使用 "RBX_RADIUS_LG"
  并且 登录界面不包含裸 hex 颜色字面量

场景: 登录输入框使用克制 focus 且不继承旧蓝
  测试: test_login_inputs_do_not_inherit_legacy_blue_focus_border
  假设 登录页存在局部 LoginTextInput 样式
  当 读取 LoginTextInput 样式与 homeserver 输入框源码
  那么 hover 描边使用 "RBX_STROKE_STRONG"
  并且 focus 描边使用 "RBX_ACCENT"
  并且 down 描边使用 "RBX_STROKE_STRONG"
  并且 光标颜色使用 "RBX_ACCENT"
  并且 输入框开启 "clip_y: true"
  并且 homeserver_input 使用 "mod.widgets.LoginTextInput"
  并且 homeserver_input 不直接使用旧 "RobrixTextInput"

场景: Homeserver hint 不再紧贴输入框使用夹线布局
  测试: test_login_form_uses_non_overlay_layout_around_inputs
  假设 homeserver 输入框下方需要显示可本地化说明
  当 读取 login_screen.rs 源码
  那么 源码包含 "homeserver_hint_row"
  并且 "homeserver_hint_label" 位于独立行内
  并且 homeserver hint 不再使用紧贴输入框的左右 LineH 夹线结构

场景: 能力探测在未分类时仍要求探测
  测试: capability_probe_is_required_when_login_mode_is_unknown
  假设 homeserver 尚未分类且无 OIDC 流程在飞
  当 判定是否需要能力探测
  那么 结果为需要探测

场景: 中文字典缺键被发现
  测试: test_login_redesign_i18n_keys_exist_zh
  假设 新增文案键应在中文字典同样存在
  当 用中文语言读取同一批新增键
  那么 每个键解析为非空字符串
  并且 解析结果不等于键名本身

场景: 重复登录失败消息被抑制
  测试: duplicate_login_failure_message_is_suppressed
  假设 上一次已展示过失败消息 "boom"
  当 再次收到相同失败消息 "boom"
  那么 不再弹出失败 modal

场景: 全新失败消息仍会展示
  测试: fresh_login_failure_message_is_shown_when_not_suppressed
  假设 上一次展示的失败消息是 "old"
  当 收到新的失败消息 "boom" 且未被注册流程抑制
  那么 弹出失败 modal

场景: OIDC 流程在飞时不重复探测
  测试: capability_probe_is_not_required_while_oidc_login_is_in_flight
  假设 一个 OIDC 浏览器登录流程正在进行
  当 判定是否需要能力探测
  那么 结果为不探测
