# Robrix2 UI 视觉 Spec

状态：**Draft v0.2**（基于 8 张参考设计稿 + 当前代码审计重写）
维护：UI / 设计系统
本仓库**只交付两样东西**：
- `src/shared/design_tokens.rs` —— 语义 token 层（本 spec 第 3 节的代码落地，**唯一交付的代码**）
- 本文档（实现合同）

**不预置成品组件**：卡片 / 徽章 / 行等一律由 agent 按第 4 节的合同 / recipe 实现。
**agent 行为约束**：见第 0 节「硬约束」，并已写入 `CLAUDE.md` / `AGENTS.md` / `specs/project.spec.md`。

---

## 0. 这份文档怎么用（给 agent / 实现者）

这不是一份"看着好看就行"的设计稿描述，而是**实现合同**。落地任何一个界面前：

1. **先读第 3 节 token 表**，所有颜色 / 圆角 / 字号一律用 `RBX_*` token，禁止在页面里写裸 hex。
2. **再读第 4 节组件合同**，按合同 / recipe 实现卡片 / 徽章 / 行，不要重新发明样式。
3. **对照第 5 节对应界面**：每个界面给了「参考图 → 线框 → 目标结构 → 当前代码与差距 → 落地步骤」。直接按"当前代码位置 + 差距"动手。
4. **遵守第 8 节构建顺序**：先 token，后组件，先 Settings，后 Detail，最后 Timeline。不要从 Timeline 开干。
5. **写 Makepad DSL 前看第 10 节 gotchas**，这些坑会让 `cargo build` 过、但运行时白屏 / panic。

### 0.1 硬约束（agent MUST / MUST NOT）

> 这些是**强制规则**，与 `CLAUDE.md` / `AGENTS.md` / `specs/project.spec.md` 一致；违反视为不合格实现。

- **MUST** 颜色 / 圆角 / 字号用 `RBX_*`（DSL：`(RBX_TOKEN)`；Rust：`crate::shared::design_tokens::RBX_*`）或既有 `styles.rs` token。
- **MUST NOT** 在界面里写裸 hex（`#xRRGGBB` 字面量）—— 需要新颜色先进 `design_tokens.rs`，再引用。
- **MUST NOT** 重新发明卡片 / 徽章 / 行样式；先按第 4 节合同 / recipe 实现，复用沉淀到 `src/shared/`。
- **MUST** 改 token 只改 `design_tokens.rs` 一处；不在各页 copy 一套视觉常量。
- **MUST** 写 Makepad DSL 前遵守第 10 节；尤其**不要**依赖「向项目内派生模板追加子节点」（本 fork 运行时不可靠）。
- **MUST** 按第 8 节顺序推进；视觉重构与业务逻辑改动**拆成独立提交**。
- **MUST** 同一状态语义固定一种色（见 3.1）；不得同语义换色。
- 状态必须补齐 `loading / empty / disabled / pending / success / warning / danger / stale`（见第 7 节），不是只做 happy path。

> 一句话方向：Robrix2 是**「可协作的 AI 工作空间」**，不是「给 bot 加了入口的聊天工具」。复杂对象卡片化、普通对话保持轻量。

本轮范围（来自 8 张稿）：移动端 Login / Settings / Room Detail / Timeline，桌面端 Login / Workbench（三~四栏）。

---

## 1. 设计北极星与原则

| 关键词 | 含义 | 反例（要避免） |
|--------|------|----------------|
| Calm | 冷白底、低饱和、靠描边分层 | 荧光 cyan、大色块后台感 |
| Layered | 卡片 / 分组 / 状态标签建立秩序 | 控件平铺、信息无层级 |
| Operational | 状态语义稳定（success/warning/danger 永远一种色） | 同语义换色 |
| Human + Agent | 人类发言轻量，Agent 对象卡片化 | 每条消息都像审批工单 |
| Cross-platform | 移动 / 桌面共享同一套 token 与组件，只在布局适配 | 两端各画一套视觉 |

硬约束：
- **一张卡片只回答一个问题**；**一个页面只保留一个主 CTA**。
- `Manage / Edit / View all` 这类次级操作统一放卡片右上角或行尾。
- 必须提前定义全部状态：`loading / empty / disabled / pending / success / warning / danger / stale-offline`（见第 7 节）。

---

## 2. 视觉语言总览（从 8 张稿提炼）

- **品牌**：3D 立方 logo（紫 `#572DCC` + 青 `#05CDC7` + 蓝），字标「Robrix」深色 +「2」青色。紫色**只**做品牌记忆点，不做功能主色。
- **主色 Accent = 冷静的青蓝 teal `#119FB3`**：主 CTA（"Sign in securely"）、选中态、链接、聚焦。承接 Robrix cyan 气质但降荧光。
- **明亮为主**：内容区（Settings / Detail / Timeline / 桌面主区）一律浅色；**仅两处深色**——桌面左侧导航栏、移动端登录页。本轮不做全量 dark mode。
- **房间 / 空间身份色**：teal 圆角方块头像（`#14B8A6`）。
- **状态色**：绿=Connected/Healthy/Active/Synced；琥珀=Approval required/Pending；红=Reject/Failed；蓝=info/链接；灰=Idle/中性。
- **卡片**：白底、大圆角（12–16）、1px 浅描边、**几乎不用重阴影**。
- **Badge / Chip**：胶囊形，浅底 + 同色系深字。
- **Agent 消息**：头像带绿色在线点 + 名字后 `APP` 标；步骤 chips；左侧强调边的分析卡；琥珀色审批卡；深底代码块 + "Translated from Chinese / Show original"。
- **Composer**：单一圆角容器，左侧 attach/emoji/slash，右侧 teal 圆形发送，外加显眼的 `Run agent` 模式切换。

---

## 3. Design Tokens（**唯一真源** → `src/shared/design_tokens.rs`）

DSL 中用 `(RBX_TOKEN)` 引用（已 `use mod.widgets.*`）；Rust 侧用 `crate::shared::design_tokens::RBX_*` 的 `Vec4` 常量。下表即代码，改值改这一处。

### 3.1 颜色

| Token | Hex | 用途 |
|-------|-----|------|
| `RBX_BG_CANVAS` | `#F7F9FC` | 页面底 |
| `RBX_BG_SURFACE` | `#FFFFFF` | 卡片 / sheet |
| `RBX_BG_SURFACE_SUBTLE` | `#F4F7FB` | 次级面板 / 分组底 |
| `RBX_BG_SUNKEN` | `#EEF2F8` | 浅色代码 / 预览内嵌 |
| `RBX_BG_HOVER` | `#EFF4FB` | 行 / 列表 hover |
| `RBX_FG_PRIMARY` | `#16233B` | 主文字（非纯黑） |
| `RBX_FG_SECONDARY` | `#5A6B86` | 副文字 / meta |
| `RBX_FG_TERTIARY` | `#8A98AE` | 时间戳 / 极弱字 |
| `RBX_FG_ON_ACCENT` | `#FFFFFF` | accent/深底上的字 |
| `RBX_FG_DISABLED` | `#AEB7C6` | 禁用字 |
| `RBX_ACCENT` | `#119FB3` | 主 CTA / 选中 / 聚焦 |
| `RBX_ACCENT_HOVER` | `#0E8C9E` | accent hover |
| `RBX_ACCENT_PRESSED` | `#0B7484` | accent pressed |
| `RBX_ACCENT_SOFT` | `#E4F5F7` | 选中 chip 底 / 高亮行 |
| `RBX_LINK` | `#1887C9` | 链接 |
| `RBX_BRAND_PURPLE` | `#572DCC` | 仅品牌入口 |
| `RBX_BRAND_CYAN` | `#05CDC7` | 仅品牌 |
| `RBX_IDENTITY_TEAL` | `#14B8A6` | 房间 / 空间头像 |
| `RBX_STROKE_SOFT` | `#E6EBF2` | 卡片 / 控件默认描边 |
| `RBX_STROKE_STRONG` | `#D5DEEA` | 强调 / 聚焦描边 |
| `RBX_DIVIDER` | `#00000010` | 行间分隔线 |
| `RBX_SUCCESS_FG` / `_BG` | `#1B8A4B` / `#E8F6EE` | Connected/Healthy/Active |
| `RBX_WARNING_FG` / `_BG` | `#C6790B` / `#FBF1DD` | Approval required/Pending |
| `RBX_DANGER_FG` / `_BG` | `#C5392F` / `#FBE9E7` | Reject/Failed/Error |
| `RBX_INFO_FG` / `_BG` | `#1E6FBF` / `#E7F0FB` | capability/linked |
| `RBX_NEUTRAL_FG` / `_BG` | `#5A6B86` / `#EEF1F6` | Idle/中性 |
| `RBX_NAV_BG` | `#1A2336` | 桌面导航栏底 |
| `RBX_NAV_FG` / `_FG_ACTIVE` | `#AEBAD0` / `#FFFFFF` | 导航项字 |
| `RBX_NAV_ITEM_ACTIVE_BG` | `#2A3650` | 导航选中底 |
| `RBX_NAV_DIVIDER` | `#2C384F` | 导航分隔 |
| `RBX_LOGIN_BG` | `#0E1626` | 移动登录底 |
| `RBX_LOGIN_SURFACE` | `#16213A` | 移动登录卡 / 输入框 |

### 3.2 圆角

| Token | 值 | 用途 |
|-------|----|----|
| `RBX_RADIUS_XS` | 6 | 小控件 |
| `RBX_RADIUS_SM` | 8 | 输入框 / 内嵌块 |
| `RBX_RADIUS_MD` | 12 | **卡片默认** |
| `RBX_RADIUS_LG` | 16 | 大卡片 / sheet |
| `RBX_RADIUS_XL` | 20 | hero / modal |
| `RBX_RADIUS_PILL` | 100 | badge / chip |

### 3.3 间距

沿用 `styles.rs` 的 4px 网格 `SPACE_XS..SPACE_XXL`（4/8/12/16/20/24），新增 `RBX_SPACE_2XL=32`、`RBX_SPACE_3XL=40` 用于 section 级留白。规则：卡片内上下 padding ≥ 16；移动端可点行高 ≥ 52。

### 3.4 字号（TextStyle preset）

| Token | 字号/字重 | 用途 |
|-------|-----------|------|
| `RBX_TEXT_PAGE_TITLE` | 17 bold | 页面标题（Settings、room hero 名） |
| `RBX_TEXT_SECTION_TITLE` | 13 bold | 分组标题 |
| `RBX_TEXT_CARD_TITLE` | 12 bold | 卡片标题 |
| `RBX_TEXT_BODY` | 11 regular | 正文 / 行标题 |
| `RBX_TEXT_BODY_STRONG` | 11 bold | 选中值 / 关键数字 |
| `RBX_TEXT_META` | 9.5 regular | meta / caption |
| `RBX_TEXT_BADGE` | 9 bold | badge / chip |

> 注：Makepad 字号偏小（项目现有 8.5–18），上表已对齐。一张卡片内字号层级 ≤ 3，密度靠**颜色变浅**而非字号变小。

---

## 4. 组件合同（Component Contracts）

本仓库**不预置成品组件**——下面是 agent 实现这些组件时**必须遵守的合同**：解剖 + token + 状态机 + 落点。能用 recipe（纯 token + 内置 widget 组合）的优先 recipe；需要复用的沉淀到 `src/shared/`。

### 4.1 SectionCard（卡片）📐 recipe（非模板）
本 Makepad fork 运行时**不可靠支持**「向项目内派生模板追加子节点」（仓库零先例），所以卡片**不做成 `RbxXxx` 模板**，而是一段 token recipe：直接用 `RoundedView` + 下面的 `draw_bg`，和 Labs「App Service」卡同结构。
- 解剖：白底卡壳，`flow: Down`，内部塞 header / rows / content。
- recipe：
  ```
  RoundedView {
      width: Fill, height: Fit, flow: Down
      padding: Inset{left:(SPACE_LG), right:(SPACE_LG), top:(SPACE_MD), bottom:(SPACE_LG)}
      show_bg: true
      draw_bg +: {
          color: (RBX_BG_SURFACE)
          border_radius: (RBX_RADIUS_MD)
          border_size: 1.0
          border_color: (RBX_STROKE_SOFT)
      }
      // ...children...
  }
  ```

### 4.2 StatusBadge（状态徽章）⛏ 按合同实现
- 变体语义：success / warning / danger / info / accent / neutral（映射见 3.1，**严禁**同语义换色）。
- 解剖：胶囊（`RBX_RADIUS_PILL`），浅底 `RBX_<STATE>_BG` + 同色系深字 `RBX_<STATE>_FG`，字 `RBX_TEXT_BADGE`，padding ≈ 9/3。
- 推荐实现：派生 `RobrixNeutralIconButton`（继承 focus-off），把 `draw_bg` / `draw_text` 重塑为对应状态色 + pill 半径，`text:` 即标签——这样 `XxxBadge { text: "Connected" }` 一行即用。
- 或内联 recipe：`RoundedView`(pill `draw_bg`) + `Label`。

### 4.3 `SettingRow` ⛏ 待建（合同已在预览里示范）
- 解剖：`[左图标/小头像] [标题 + 副标题(Fill)] [右值 / StatusBadge / chevron]`，`flow: Right, align y:0.5`。
- token：标题 `RBX_TEXT_BODY_STRONG`，副标题 `RBX_TEXT_META/FG_SECONDARY`，行高 ≥ 48（移动 ≥ 52），底部 `RBX_DIVIDER` 1px。
- 状态：default / hover(`RBX_BG_HOVER`) / disabled(`RBX_FG_DISABLED`)。
- 落点：先在 `src/settings/` 内提炼，稳定后移到 `src/shared/setting_row.rs`。

### 4.4 `CapabilityChip` ⛏ 待建
- 与 badge 同形，但语义=能力/标签/角色（Admin / Mod / Agent / Vision / Tool calls）。用 info / accent 徽章变体（§4.2）起步即可。

### 4.5 `AgentMessageCard` ⛏ 待建（Timeline 核心，高风险）
- 解剖：`头像(带绿点) + [名字][APP] + 时间` → 步骤 chips 行 → 分析卡（左 accent 边）→ footer meta。
- token：卡 `RBX_BG_SURFACE` + `RBX_STROKE_SOFT`；分析卡左边 4px `RBX_ACCENT`；`Recommended action` 用 `RBX_TEXT_BODY_STRONG`。
- 落点：`src/home/room_screen.rs`，作为 `bot_message_card`（行 ~2044）的兄弟节点，插在 `username_view` 之后（见 5.4）。

### 4.6 `ApprovalCard` ⛏ 待建（Robrix2 标志性组件）
- 解剖：琥珀底（`RBX_WARNING_BG` + `RBX_WARNING_FG` 边）→ 标题 + `Pending` badge → 待决动作正文 → 请求者/时间 meta →`[Approve(success)] [Reject(danger)]`。
- 现状：`approval_request_view`（room_screen.rs ~2139）已存在，但用的是**浅蓝** `COLOR_BOT_STATUS_BG`，需改为琥珀；缺 `Pending` badge。
- 预览里 Card 2 的「mini approval card」即此合同的视觉样板。

### 4.7 `CodeOutputCard` ⛏ 待建
- 解剖：深底面板（深 navy）+ 语法高亮 + 底部「Translated from Chinese / Show original」。
- 现状：bot markdown 代码块已渲染，缺翻译 footer 与统一深底。

### 4.8 `Composer`（房间输入区）⛏ 改造
- 目标：单一圆角容器；左 cluster（attach/emoji/slash）；显眼 `Run agent` 模式切换（比普通图标按钮重）；右 teal 圆形发送。
- 现状与落点见 5.5。

---

## 5. 各界面 Spec（参考图 → 线框 → 目标 → 现状/差距 → 落地）

### 5.1 Settings（移动端，参考图 7）

```
┌─────────────────────────────┐
│ Settings                 🔍  │  Page title + search
│ ───────────────────────────  │
│ [Account] Security Agents …  │  Segmented tabs（选中=accent 实底/描边）
│ Homeserver & Account         │
│ ┌─────────────────────────┐  │  卡片 recipe(§4.1)
│ │🌐 Homeserver  matrix… [Healthy]│ SettingRow + Badge
│ │👤 User ID    @alice…    › │  │
│ │🔑 Password   ••••••     › │  │
│ │🔒 E2E encryption  [Enabled] │ │
│ └─────────────────────────┘  │
│ AI & Models                  │
│ ┌ Model Provider OpenAI[Connected]
│ │ Default Model  gpt-4o    › │
│ │ Connected Tools           │
│ │  [GitHub Connected][Jira…] +Add tool
│ │ Agent Permissions         │
│ │  File Access  Allowed      │
│ │  …  View all permissions › │
│ └─────────────────────────┘  │
│ Automation & Policies        │
│ ┌ Approval required   [◉ ]   │  toggle
│ │ Data sharing        [ ◯]   │
│ Sync & Preferences  …         │
│ [Reset to defaults][Save changes] ← sticky 底栏（次/主 CTA）
│ ───────────────────────────  │
│ ⌂Home ▢Rooms ⚇Agents 🔍 ⚙Set │ bottom tab bar
└─────────────────────────────┘
```

- **目标结构**：page title + search → segmented tabs → 多张卡片（recipe §4.1，每张含若干 `SettingRow`，右侧状态用 StatusBadge §4.2）→ sticky 底栏 `Reset to defaults`(次) + `Save changes`(主 accent)。分区：`Homeserver & Account / AI & Models / Automation & Policies / Sync & Preferences`。
- **现状**（`src/settings/settings_screen.rs`）：用 `PageFlip` + 5 个**横排按钮 tab**（Account/Preferences/Devices/Labs/Contribute）；卡是 `RoundedView` 底色 `#F8F8FA`；无 search、无 segmented tab、无 SettingRow 抽象、无 sticky 底栏、状态用彩色 label 而非 badge。
- **差距 → 落地**：
  1. 把横排按钮换成 segmented tabs 视觉（选中 `apply_primary_button_style` 已在，改成 accent token）。
  2. 卡壳 `RoundedView #F8F8FA` → 卡片 recipe §4.1（白底 + 描边）。
  3. 行抽象成 `SettingRow`（4.3），右侧状态用 StatusBadge（§4.2）。
  4. 加 sticky 底栏（`Save changes` = 主 accent 按钮）。
  5. 分区改名/重组到目标 4 组。
- **这是首个落地界面**（风险低、密度高，验证卡片+badge+tab 体系）。

### 5.2 Room / Mission Detail（移动端，参考图 8）

```
┌─────────────────────────────┐
│ ‹  # ai-ops-mission  ★   ⤴ ✕ │  hero header
│ Mission space · Coordinate…  │
│ 24 members •Encrypted •Synced│  meta + 状态
│ [Agent-enabled][Approval req]│  badges
│ ┌ About ───────────────────› │  object cards
│ ┌ Members 24  ◯◯◯+21  ────› │  [Admin][Mod][Agent] chips
│ ┌ Linked Agents ──────────› │  SRE•Active DataAnalyst•Waiting
│ ┌ Connected Tools  6   +1 › │
│ ┌ Knowledge Sources ──────› │
│ ┌ Pinned Goals 3 ─────────› │  • Reduce API latency 20%
│ ┌ Recent Automations ─────› │  ✓ Incident classification
│ ┌ Media & Files 38 ───────› │
│ ┌ Room Permissions [Edit] › │
└─────────────────────────────┘
```

- **目标结构**：hero（teal `#` 头像 + 名 + 人数 + Encrypted/Synced + `Agent-enabled`/`Approval required` badge）→ 一串「对象集合卡」，每张 = 预览 + 数量 + 进入箭头。模块：About / Members / Linked Agents / Connected Tools / Knowledge / Pinned Goals / Recent Automations / Media & Files / Room Permissions。
- **现状**：robrix2 现有 room info / space lobby 偏「房间设置」而非「对象总览」。无 Linked Agents / Knowledge / Goals / Automations 卡。
- **落地**：用卡片 recipe（§4.1）+ `SettingRow`(进入式，右 chevron) 拼总览；角色/状态用 StatusBadge / CapabilityChip（§4.2 / §4.4）。桌面端复用同卡，见 5.6 右栏。

### 5.3 Timeline（移动端，参考图 6）—— **最后做，风险最高**

```
‹ # ai-ops-mission           🔍 ⚙ ⋯
Room goal: Reduce API latency 20% (Q2)  [View details]   ← goal banner
[Chat] Tasks Artifacts Pinned Info                        ← 二级 tab
◯ Bob Martin 10:12   incident #421 summary ⌄              ← human（轻量）
◯•SRE Agent [APP] 10:14  Investigate API latency
   [Collecting][Committing][Analyzing][Proposing]         ← 步骤 chips
◯•SRE Agent [APP]
  ┃ Analysis summary                                       ← 左 accent 边卡
  ┃ • P95 latency up…  Recommended action: …
  ┃ [View progress] [Approve ▸]
  ┌ Alice Chen  Rollout patch v1.24.3   [Pending] ┐        ← 琥珀审批卡
  │ [Approve] [Reject] [View details]             │
◯ Data Assistant
  ▓ SELECT region, error_rate … (dark SQL panel) ▓        ← 代码卡
  ↺ Translated from Chinese · Show original
┌─────────────────────────────────────────────┐
│ Write a message or use /                      │
│ 📎 😀 /                    [Run agent]   ➤    │           ← composer
└─────────────────────────────────────────────┘
```

- **目标层次**：导航 → goal banner → 二级 tab → 消息流 → composer。消息类型：human（轻量）/ agent progress / `AgentMessageCard` / `ApprovalCard` / `CodeOutputCard`。
- **现状**（`src/home/room_screen.rs`）：`Message`/`CondensedMessage` 模板 + `bot_message_card`（~2044）+ `approval_request_view`（~2139，浅蓝非琥珀，缺 Pending）。无 goal banner、无二级 tab、无步骤 chips、无绿点头像、badge 是 `bot` 非 `APP`。
- **落地（按序）**：①先做 human vs agent 层级区分；②`ApprovalCard`（改色 + Pending badge）；③`CodeOutputCard`（深底 + 翻译 footer）；④步骤 chips + 绿点头像 + `APP` 标；⑤最后 composer。把消息卡拆成小渲染单元，建立稳定 render contract，别再在 room_screen 堆页面级 if/else。

### 5.4 AgentMessageCard 插入点（room_screen.rs）

`Message` 模板 content 区，`username_view` 之后、`bot_message_card` 之前插入 `agent_message_card`（默认 `visible: false`）。在 `populate_message_view()` 检测 agent sender，显示 `agent_message_card` 而非 `bot_message_card`。审批 / 代码卡作为 `action_buttons` 子节点。建议加 `agent_render_state` 协调三类卡，避免 visibility 互相打架。

### 5.5 Composer（`src/room/room_input_bar.rs`）

- **现状**：`RoundedView`(radius 5) 容器，上排工具条（attach/emoji/translate/bot-menu）+ 下排 `MentionableTextInput`，右侧 `RobrixPositiveIconButton`(矩形) 发送。无 `Run agent` 模式。
- **落地**：①发送键改 teal 圆形（`border_radius` 调大 + accent 色）；②`button_row` 的 `LeftActionButtons` 与 `Filler` 之间插 `run_agent_toggle`（带边/高亮，读作"模式切换器"）；③`RoomInputBar` 加 `run_agent_mode: bool`，`clicked()` 切换，发送时按 flag 派发不同请求；④不动 mention/slash 检测。

### 5.6 Login + Desktop Workbench（参考图 1/5/2/4）

- **Login**（`src/login/login_screen.rs`）：品牌卡（cube logo + `Robrix2` + `Agent-native collaboration client`）→ `User ID / Password(👁) / Homeserver URL` → teal **Sign in securely** → `Or continue with` → `SSO/Google/GitHub/Microsoft/More` → `Create an account` → 状态 footer（Secure session · Self-host ready · Matrix connected）。两变体：桌面浅色（`RBX_BG_CANVAS`）/ 移动深色（`RBX_LOGIN_BG` + `RBX_LOGIN_SURFACE`）。
- **Desktop Workbench**（`src/home/main_desktop_ui.rs` + `home_screen.rs`）：`[深色 NAV 栏 | 房间列表 | Timeline 主区 | 右侧 Info 面板]`。NAV 用 `RBX_NAV_*`（深色锚点）；右侧 Info 面板（Active agents / Pending approvals / Linked agents / Tools / Knowledge / Recent automations）目前**缺失**，需新建，并复用 5.2 的对象集合卡。

---

## 6. 响应式与平台策略

| | 移动端 | 桌面端 |
|--|--------|--------|
| 布局 | 单列、大卡、大点击面积 | 双/三/四栏（nav｜list｜main｜info） |
| 展开 | 多用 sheet | 多用面板 / dock，少全屏 modal |
| 导航 | 底部 tab：Home/Rooms/Agents/Search/Settings | 左侧深色 nav 栏 |
| 共通 | 同组件状态语义必须一致；同一圆角/描边/badge 体系；中英文 + 长用户名/长 room 名/长 badge 都要验不截断 | |

当前导航差距：移动端现为 Home/Add Room/Spaces/Profile（非 5 tab），缺 Rooms/Agents/Search 独立 tab；桌面缺右侧 Info 面板。

---

## 7. 状态矩阵（每个新组件都要补齐）

| 状态 | 视觉 |
|------|------|
| loading | 骨架 / `bouncing_dots`，不留白屏 |
| empty | 居中插画/说明 + 单一引导 CTA |
| disabled | `RBX_FG_DISABLED` + 降不透明，禁交互 |
| pending | StatusBadge·warning + 琥珀容器 |
| success | StatusBadge·success |
| warning | StatusBadge·warning |
| danger/error | StatusBadge·danger + 错误说明 + 重试 |
| stale/offline | 灰中性 + 离线提示，不伪装在线 |

---

## 8. 构建顺序（Roadmap，强约束）

| Phase | 内容 | 主要文件 | 状态 |
|-------|------|----------|------|
| 1 | **Design Tokens** | `src/shared/design_tokens.rs` | ✅ 完成 |
| 2 | **基础组件**：StatusBadge / SettingRow / CapabilityChip（按 §4 合同实现；SectionCard 用 §4.1 recipe） | `src/shared/`（新） | ⛏ |
| 3 | **重构 Settings**（5.1） | `src/settings/settings_screen.rs` 等 | ⛏ 下一步 |
| 4 | **Room Detail / Workbench 右栏**（5.2/5.6） | `src/home/*` | ⛏ |
| 5 | **Timeline**：ApprovalCard→CodeCard→AgentCard→Composer（5.3） | `src/home/room_screen.rs`、`src/room/room_input_bar.rs` | ⛏ 最后 |
| 6 | 视觉 QA：移动/桌面截图对比 + 全状态 + i18n 截断 | — | ⛏ |

每个 Phase 的视觉改动尽量与业务逻辑改动**拆成独立提交**。

---

## 9. 本地视觉验证（如何自查）

本仓库**不含演示页**（只交付 token + spec）。需要肉眼回归 token / 组件时：

- 临时在某个可达界面内联样板，推荐 **Settings ▸ Labs**（`settings_screen.rs` 的 `labs_settings_section`）。
- 用 §4.1 卡片 recipe + §4.2 徽章拼出色板 / 徽章 / 行样例，`cargo run` 看效果，**验完即删**，不要把演示合进生产代码。
- 注意用 `cargo run`（非 `--hot`）：新增 Rust 模块的注册不走 DSL 热更。

---

## 10. Makepad DSL 实现 gotchas（会让 build 过、运行炸）

1. `script_mod!` 内**只能用 `//` 注释**，`///` 会被当成 Rust `#[doc]` 编译失败。
2. token / 模板必须在引用方**之前**注册（`shared/mod.rs::script_mod` 里 `design_tokens` 紧跟在 `styles` 之后；新组件模块要排在它依赖的基底 widget 之后）。
3. 颜色含字母用 `#x` 前缀（如 `#x1E90FF`）；透明用 `#x00000000`。
4. `draw_bg +:` **合并**、`draw_bg:` **替换**（替换会丢边框/动画 uniform）。
5. 命名子节点用 `:=` 定义；**不要**指望 `child = { ... }` 在实例处覆盖已命名子节点（本仓库无先例，运行时不可靠）——把可变内容做成根属性（如 `Button` 的 `text:`）或直接内联组合。
6. badge 用 `Button` 基底是因为 `text:` 可在实例处参数化；并把 hover/down/focus 颜色钉死=静止态，否则会有点击涟漪。
7. 动态创建的 widget（`widget_ref_from_live_ptr`）上 `script_apply_eval!` 失效（`ScriptObject::ZERO`），改用 Animator + shader instance 变量。
8. `script_apply_eval!` 内不能用 DSL 常量（`Right/Fit/Align`），用 `#(expr)` 插值或烘进模板。

---

## 11. 非目标（本轮不做）

全量 dark mode / 全量品牌重做 / 消息协议重设计 / Matrix 数据模型重构 / 大范围动画重写。这些会让视觉重构失去边界。

---

## 12. 结论

8 张稿给的不是单页样式，而是一条明确路线：**从「聊天客户端」升级到「AI 协作工作台」**。落地节奏：**先 token（✅）→ 后组件（按 §4 合同实现）→ 先 Settings → 后 Detail → 最后 Timeline**。token 层已可用，下一步从 §5.1 Settings 开始，并严格按 §0.1 硬约束执行。
