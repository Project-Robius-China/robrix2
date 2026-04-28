# Codex TG Bot 架构方案审查

> **⚠️ SUPERSEDED (2026-04-13).** This architecture review shaped the Phase 2
> explicit-target-chip design. The product direction has since moved to
> **mention/reply-first** (Phase 3) — see
> [`docs/superpowers/plans/2026-04-12-tg-bot-mention-reply-first-plan.md`](./2026-04-12-tg-bot-mention-reply-first-plan.md)
> and [`specs/task-tg-bot-mention-reply-first.spec.md`](../../../specs/task-tg-bot-mention-reply-first.spec.md).
> The type model (`ExplicitOverride`/`ResolvedTarget`), `is_known_or_likely_bot()`,
> and the reply-to-human fix survived the pivot; the persistent target chip
> UI and its switching menu did not. Kept for historical context.

## Context

Codex 提出将 Robrix2 的 bot 交互模型从"implicit room-bound bot routing"转向"explicit bot targeting"，对标 Telegram。本文档审查该方案的技术可行性、架构合理性，以及 Codex 未覆盖的设计问题。

**架构原则：** Telegram-style bot UX on top of Matrix semantics。Robrix2 是 Matrix IM 客户端，不是 Telegram 客户端。底层保留 Matrix 的 room/user/message 模型，只在客户端 UX 层补上 Telegram 风格的显式、低摩擦 bot 交互体验。不应把 Telegram 的协议级 bot 概念（原生命令列表、BotFather、menu button）直接映射为 Robrix 的产品默认。

**修正说明：** Bot ID 不一致（`"bot"` vs `"octosbot"`）确实存在。实际部署配置 `palpo-and-octos-deploy/config/botfather.json:16` 用的是 `"octosbot"`，与 `src/app.rs:2069` 的默认值 `"bot"` 不匹配。Codex 在这点上是对的。但 `DEFAULT_BOTFATHER_LOCALPART` 是通用默认值，不应绑定到特定 appservice 实现。

---

## 一、核心论断审查

**Codex 论断：** Robrix2 是"implicit room-bound bot routing"，Telegram 是"explicit bot targeting"。要对齐，应把 binding 降为内部实现，把显式 target 升为用户可见模型。

**审查结论：论断正确，方向正确。**

代码证据：
- `resolve_target_user_id()` (`room_input_bar.rs:861-876`) — 三级优先级 explicit > reply > fallback，但两个 send path 都传 `None` 给 explicit，从未使用
- `active_target_user_id` (`room_input_bar.rs:605`) — 状态字段存在但：(a) 从不被 UI 显示，(b) 发送后不清零，(c) 没有用户主动设置的入口
- `bound_bot_user_id` 在 `RoomScreenProps` 中作为 fallback 默默路由消息

**一句话：路由管道已经预留了显式 target 的参数位，但状态模型、bot 判定、UI 层都还没接入——不止是 UI 接线。**

---

## 二、P0/P1/P2 分层评估

### P0：基础修正 — 认可

| 项目 | 评估 | 备注 |
|------|------|------|
| bot/octosbot 配置对齐 | **应做但不改全局默认** | `botfather_user_id` 已是用户可配置项（`app.rs:2218`），应通过文档/preset 引导 Palpo+OctOS 用户配置，不应把 `DEFAULT_BOTFATHER_LOCALPART` 硬改为 `"octosbot"` |
| 改善绑定错误提示 | **应做** | 当前 "Bind BotFather..." 文案对用户不友好 |
| 改善自动检测 | **低优先** | `is_likely_bot_user_id()` 已有合理的启发式规则 |

### P1：核心模型对齐 — 认可，但有设计缺口

**Codex 方案：** 输入框加 target chip（`To room` / `To @configured bot`）

**技术可行性：非常高。** 代码库已有所有基础设施：

| 已有基础 | 位置 | 复用方式 |
|----------|------|----------|
| 状态字段 `active_target_user_id` | `room_input_bar.rs:605` | 直接用 |
| 三级路由函数 `resolve_target_user_id()` | `room_input_bar.rs:861` | 已支持 explicit target 参数 |
| 上下文指示器 UI 模式 | `reply_preview.rs:77-123` `ReplyingPreview` | 完全照搬：Label + 取消按钮，位于输入框上方 |
| Bot 绑定状态 | `RoomScreenProps.bound_bot_user_id` | 决定默认 target |
| 状态持久化 | `RoomInputBarState` (`room_input_bar.rs:1596`) | 已保存 `active_target_user_id` |

**Codex 未覆盖的 4 个设计问题：**

1. **⚠️ Reply 不区分 bot 和人（Matrix 语义风险）**
   - 当前 `reply_target_user_id` 直接取被回复消息的 sender（`room_input_bar.rs:1112-1115`），不检查是否为 bot
   - 这个值传入 `resolve_target_user_id()` 后作为 `target_user_id`，最终在 `sliding_sync.rs:2726` 触发 `ensure_target_user_joined_room()`
   - **问题：** 回复普通用户的消息，也会被当作 bot 定向处理。在 Matrix 语义下，reply 首先是原生回复关系，不应默认变成"把消息定向给被回复的人"
   - **方案：** 必须在 resolver 中增加 bot 判定——只有 reply-to-bot 才参与 bot targeting，reply-to-human 只作为普通 Matrix reply
   - **⚠️ Bot 判定不能只用 `is_likely_bot_user_id()`。** 该函数只覆盖 localpart 启发式和父 bot 精确匹配（`room_screen.rs:432`），不查 `known_bot_user_ids()`。代码中更完整的 bot 识别逻辑在 `detected_bot_binding_for_members()`（`room_screen.rs:360`），它先查 `resolved_bot_user_id()`，再查 `known_bot_user_ids()`，最后做启发式。但注意：该函数是"房间级绑定检测"（接受 `&[RoomMember]`），不是通用的"单个 user_id 是否是 bot"判定器
   - **正确方案：** 需要新建一个独立的 `is_known_or_likely_bot(user_id: &UserId, bot_settings: &BotSettingsState, current_user_id: Option<&UserId>) -> bool` 函数，合并 `known_bot_user_ids()` 精确查询、`resolved_bot_user_id(current_user_id)` 匹配和 `is_likely_bot_user_id()` 启发式。注意：`resolved_bot_user_id()` 需要 `current_user_id` 参数来将 localpart-only 的配置值解析为完整 MXID（`app.rs:2218`）。这是一个新函数，不是从 `detected_bot_binding_for_members()` 抽取——后者的职责是房间级绑定发现，不应被当作单 user_id 判定的前身
   - **⚠️ 依赖传递问题：** 该函数需要 `BotSettingsState` 和 `current_user_id`，但 `reply_target_user_id` 的决策点在 `room_input_bar` 的发送路径中（`room_input_bar.rs:1112`），而当前 `RoomScreenProps` 只带了 `bound_bot_user_id` 等少量 bot 上下文（`room_screen.rs:6480`）。P1 实现需要二选一：
     - **方案 A：** 扩展 `RoomScreenProps`，把 `&BotSettingsState` 引用或 resolved parent bot user_id 传入 `room_input_bar`，让 bot 判定在发送路径中本地完成
     - **方案 B：** 将 reply-target 的 bot 判定上移到持有 `AppState` / `current_user_id` 的 `room_screen` 层，在传入 `room_input_bar` 之前就完成过滤
     - 推荐 **方案 A**（改动面更小，`RoomScreenProps` 已经是 bot 上下文的传递通道）

2. **"To room" vs "无 target" 语义不同**
   - 当前 `active_target_user_id = None` + `bound_bot_user_id = Some` → 消息发给 bot（fallback）
   - 用户点击 "To room" 意味着**显式不发给 bot**
   - 需要区分 "未设置 target"（用 fallback）和 "显式选择 room"（跳过 fallback）
   - **方案：** 使用 `TargetSource` enum（见第 4 点），`ExplicitRoom` 表示用户主动选择发给房间，resolver 遇到此状态时跳过 fallback

2. **Target 何时清零？**
   - Codex 没说。当前 `active_target_user_id` 发送后不清零（sticky）
   - Telegram 官方文档只明确了群里可以通过 reply 或 `/command@OtherBot` 与 bot 通信（[Bot Features](https://core.telegram.org/bots/features#bot-to-bot-communication)），未规定"后续 target 是否自动清零"
   - **这是产品决策，不是 Telegram parity 事实。** 推荐行为：reply target 发送后清零，显式 bot target 保持 sticky — 但这需要作为明确的设计选择记录，而非伪装成对标 Telegram 的既有行为

3. **首次进入 bot room 的默认状态 + target 来源模型**
   - 绑定了 bot 的房间，首次进入时 target chip 应该显示什么？
   - "To room" 但 fallback 实际发给 bot → **矛盾**
   - "To @configured bot" 但用户没有主动选择 → **可能困惑**
   - 当前 resolver 只区分"有某个 user_id"或"没有"，丢失了 target 来源信息
   - **方案：** 拆分为"持久化的用户意图"和"运行时计算的 resolved target"两层：

     **持久化层（存入 `RoomInputBarState`）：**
     ```
     ExplicitOverride {
         None,                       // 用户没有主动覆盖，使用房间默认行为
         Bot(bot_user_id),           // 用户主动选择的 bot
         Room,                       // 用户主动选择发给房间（跳过默认 bot）
     }
     ```

     **运行时计算层（resolve 时根据上下文推导）：**
     ```
     ResolvedTarget {
         NoTarget,                   // 普通 Matrix 房间，没有任何 bot target
         RoomDefault(bot_user_id),   // 来自 bound_bot_user_id + ExplicitOverride::None
         ExplicitBot(bot_user_id),   // 来自 ExplicitOverride::Bot
         ExplicitRoom,               // 来自 ExplicitOverride::Room
         ReplyBot(bot_user_id),      // 来自当前 replying_to + bot 判定
     }
     ```

   - **持久化什么：** 只持久化 `ExplicitOverride`（用户的显式意图）。`RoomDefault` 和 `ReplyBot` 是派生状态，在 resolve 时从 `bound_bot_user_id` 和 `replying_to` 实时计算。这避免了房间绑定变化、取消回复时的陈旧值问题
   - **运行时 resolve 逻辑：**
     1. 如果有 `replying_to` 且被回复者是 bot → `ReplyBot(bot_user_id)`
     2. 否则看 `ExplicitOverride`：`Bot(id)` → `ExplicitBot(id)`，`Room` → `ExplicitRoom`
     3. 否则（`ExplicitOverride::None`）：如果有 `bound_bot_user_id` → `RoomDefault(bot)`，否则 → `NoTarget`
   - **混合场景决策（ExplicitOverride::Bot + reply-to-human）：**
     - 用户已设置 `ExplicitOverride::Bot(octosbot)`，然后 reply 一个普通人的消息
     - reply-to-human 不触发 `ReplyBot`（步骤 1 的 bot 判定不通过），继续走步骤 2
     - resolve 结果：`ExplicitBot(octosbot)`，同时挂上对普通人消息的 Matrix reply 关系
     - **产品语义：** 消息定向发给 bot，同时在 Matrix 协议层是对那条人类消息的 reply。这是合理的——用户可能想让 bot 看到被引用的上下文
     - **UI 展示：** target chip 显示 "To @bot"，reply preview 正常显示被引用的消息，两者独立
   - **UI 展示：**
     - `NoTarget`：chip 不显示
     - `RoomDefault`：淡色 "Default: @bot"
     - `ExplicitBot`：正常色 "To @bot"
     - `ExplicitRoom`：chip 显示 "To room"
     - `ReplyBot`：chip 显示 "Reply → @bot"（临时，取消 reply 即消失）
   - **chip × 行为：** 清除 `ExplicitOverride` 回到 `None`，resolve 自动回退到 `RoomDefault`（有绑定 bot 时）或 `NoTarget`（无 bot 时）
   - **产品决策：** 首次进入任何房间时 `ExplicitOverride` 为 `None`，resolve 根据是否有 `bound_bot_user_id` 决定显示

### P2：Telegram 化交互 — 方向对，优先级合理

| 项目 | 评估 |
|------|------|
| Menu button 替代 `/bot` | 可做，但 `/bot` 可保留给 power user |
| 命令分类（纯命令 send-on-select / 参数命令 insert） | 方向合理，但当前硬编码命令表（`mentionable_text_input.rs:188-195`）就是设计本身——spec 明确将"动态命令注册"列为 out of scope（`task-tg-bot-ui-alignment.spec.md:48`）。OctOS 的 slash 命令本质上也是"在聊天里输入的文本命令"，不是客户端可发现的协议能力。不应在静态 `SlashCommand` 上堆字段固化，但也不应把"动态注册"默认为自然的下一步——那需要一个新的 Matrix-side 元数据/协议设计，属于独立的未来方向 |
| `/command@bot` 显式寻址 | 长期目标，需解析语法 + 多 bot room 支持 |

---

## 三、架构判断

**Codex 的核心设计决策是正确的：**

> "底层继续复用现在的 target_user_id 机制，不推翻现有发送链路"

这是最合理的路径。`resolve_target_user_id()` 已经设计了三级优先级，需要：
1. 让 UI 能显示当前 resolved target
2. 让用户能主动设置 explicit target（当前两个 send path 都传 `None`）
3. 让用户能清除 target（切回 "To room"）

**⚠️ 不止是 UI 接线。** 当前 `resolve_target_user_id()` 的签名只接受 `Option<OwnedUserId>`，能表达"某个 bot"或"没有显式 bot"，但表达不了"显式发给 room、禁止 fallback"这个第三种状态（`room_input_bar.rs:861`）。需要两层模型（见第二节第 4 点）：
- **持久化层：** 将 `active_target_user_id: Option<OwnedUserId>` 改为 `ExplicitOverride { None, Bot(UserId), Room }`，只存用户显式意图
- **运行时层：** `resolve_target()` 从 `ExplicitOverride` + `bound_bot_user_id` + `replying_to` 实时推导 `ResolvedTarget`
- 抽取 `is_known_or_likely_bot(user_id, bot_settings, current_user_id)`（见第二节第 1 点），供 resolve 时判断 reply target 是否为 bot

---

## 四、Gap 总结

| Gap | 严重程度 | 需要决策 |
|-----|---------|---------|
| Reply 不区分 bot 和人 | **高** — 回复普通用户也触发 bot targeting | 需要统一 bot 判定函数（`known_bot_user_ids()` + `is_likely_bot_user_id()`），在 resolver 中检查 |
| "To room" vs "未设置 target" 的语义区分 | **高** — 不解决会导致 target chip 说谎 | 需要两层模型：持久化 `ExplicitOverride` + 运行时 `ResolvedTarget`，拆分用户意图和派生状态 |
| Target 清零时机 | **中** — 影响用户心智模型 | 这是产品决策，不是 TG parity 事实 |
| Target 跨导航持久化 | **已定义** | 持久化 `ExplicitOverride`（用户意图）。`ReplyBot` 不独立持久化，但因 `replying_to` 已持久化（`room_input_bar.rs:1595`）而间接恢复 |
| Reply 与 Target 所有权 | **高** — 必须定义 | 取消 reply → 清掉 ReplyBot；清掉 target → 保留 Matrix reply。ReplyBot 从 replying_to + bot 判定实时推导，不独立持久化 |
| 首次进入 bot room 的默认 target | **中** — 影响首次体验 | 建议用 `RoomDefault(bot)` 而非直接显示 "To @bot" |
| 静态命令表的定位 | **低** — spec 已明确 out of scope | 硬编码命令表就是当前设计，不应默认为"过渡层"；动态注册需新的 Matrix-side 协议设计，属独立未来方向 |
| 文档中 `@octosbot` 硬编码 | **低** — 通用架构不应绑定特定 appservice | 改为 "configured bot" / "default bot"，只在 OctOS 章节举例 |
| 用户如何主动切换 target | **高** — P1 必答题 | 无切换入口则 target chip 只是展示，不是交互模型。至少需定下一种：点 chip 弹出切换菜单 / reply bot 自动切换 / slash qualifier 选 target |
| 多 bot room 场景 | **低** — 当前不是主要场景 | P2 解决 |

---

## 五、推荐的实施路径

认可 Codex 的 P0 → P1 → P2 分层，补充设计细节后可以写 spec：

**P0（先做）：** 改善绑定错误文案 + 为 Palpo+OctOS 部署提供 migration/preset 示例值。注意：`DEFAULT_BOTFATHER_LOCALPART` 是通用默认值，`botfather_user_id` 本身已是用户可配置项（`src/app.rs:2218`），UI 文案也定义为通用输入（`resources/i18n/en.json:274`）。不应把全局默认硬改为 `"octosbot"`，而应通过文档/示例引导 Palpo+OctOS 用户配置正确的值

**P1（核心）：** Target 状态模型 + chip + 切换入口
- 将 `active_target_user_id: Option<OwnedUserId>` 重构为 `ExplicitOverride` enum（只持久化用户意图），运行时 resolve 为 `ResolvedTarget`
- 抽取 `is_known_or_likely_bot(user_id, bot_settings, current_user_id)` 统一 bot 判定（合并 `known_bot_user_ids()` + `resolved_bot_user_id()` + `is_likely_bot_user_id()`）
- 修复 reply-to-human 误触发 bot targeting（在 resolver 中加 bot 判定）
- 新增 `TargetIndicator` widget（参考 `ReplyingPreview` 的 UI 模式），区分来源显示
- **⚠️ TargetIndicator 与 ReplyingPreview 的所有权关系：** reply 预览已有独立的显示/取消/恢复状态机（`show_replying_to()` at `room_input_bar.rs:1214`、`clear_replying_to()` at `:1267`、`on_editing_pane_hidden()` at `:1307`）。必须先定义：
  - 取消 reply 是否同时清掉 `ReplyBot` target → **应该是**，因为 `ReplyBot` 的真相来源就是 `replying_to`
  - 清掉 target chip 是否保留 Matrix reply → **应该是**，reply 是 Matrix 原生关系，target 是 Robrix UX 层
  - `ReplyBot` 不应作为独立状态持久化，而应在 resolve 时从 `replying_to` + bot 判定实时推导（见上方持久化层设计）
- **定义切换入口（P1 必答题）：** 必须包含"主动选择 bot"的入口（否则 `ExplicitOverride::Bot` 永远不会被设置）。最小完整集：
  - 点 chip 弹出切换菜单 → 可选择 bot 或 room（产生 `ExplicitOverride::Bot` / `ExplicitOverride::Room`）
  - reply bot 消息 → 自动 resolve 为 `ReplyBot`（临时，取消 reply 即消失）
  - chip 上的 × → 清除 `ExplicitOverride` 回到 `None`（默认行为）
- 在 send path 中接入 explicit target（当前传 `None` 的地方）

**P2（增量）：** 命令分类 + menu button + `/command@bot`

## 六、验证方式

- 运行 `cargo run`，进入有 bot 的房间
- 确认 target chip 正确显示当前消息目标，且区分来源（默认 bot vs 显式选择）
- 测试切换 target（To room ↔ To configured bot）后发送消息，验证路由正确性
- 测试 reply-to-bot 时 target 自动切换为 `ReplyBot`
- **测试 reply-to-human 时不触发 bot targeting**（当前是 bug）
- 验证跨导航时 `ExplicitOverride` 被恢复；`ReplyBot` 会随 `replying_to` 一起恢复（`replying_to` 已持久化在 `RoomInputBarState`，`ReplyBot` 从中实时推导——不独立持久化，但因 `replying_to` 恢复而间接恢复）
