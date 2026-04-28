# Robrix2 Bot 功能 vs Telegram Bot 功能差距分析

> 分析日期: 2026-04-11
> 参考文档: https://core.telegram.org/bots/features

## 当前 Robrix2 已有的功能

| 类别 | 功能 | 状态 |
|------|------|------|
| Bot 生命周期 | 创建 Bot (`/createbot`) | :white_check_mark: |
| | 删除 Bot (`/deletebot`) | :white_check_mark: |
| | 列出 Bot (`/listbots`) | :white_check_mark: |
| | Bot 帮助 (`/bothelp`) | :white_check_mark: |
| 房间绑定 | 绑定/解绑 Bot 到房间 | :white_check_mark: |
| | 邀请/踢出 Bot (Matrix SDK) | :white_check_mark: |
| | 多 Bot 绑定同一房间 | :white_check_mark: |
| Bot 发现 | 自动检测房间内 Bot 成员 | :white_check_mark: |
| | 解析 `/listbots` 回复提取 Bot ID | :white_check_mark: |
| | 启发式 Bot 用户名识别 | :white_check_mark: |
| 设置 | App Service 开关 | :white_check_mark: |
| | BotFather 用户 ID 配置 | :white_check_mark: |
| | 每房间 Bot 备注 | :white_check_mark: |
| | 持久化存储 | :white_check_mark: |
| UI | AppServicePanel (操作面板) | :white_check_mark: |
| | 创建/删除/绑定 Bot 模态框 | :white_check_mark: |
| | 房间右键菜单"管理 Bot" | :white_check_mark: |
| | i18n (中英文) | :white_check_mark: |

---

## 与 Telegram 的差距

### P0 — 核心交互能力缺失

| Telegram 功能 | 描述 | Robrix2 现状 | 差距 |
|---------------|------|-------------|------|
| **Bot Commands** | 用户输入 `/` 弹出命令列表，点击即发送 | :x: 无命令发现机制 | Bot 无法向客户端声明自己支持哪些命令，用户只能手动输入 |
| **Inline Keyboards** | 消息下方显示可点击按钮（回调、URL、切换等） | :x: 无 | Bot 消息无法携带交互按钮，用户只能通过纯文本回复 |
| **Reply Keyboards** | Bot 替换用户键盘为预设选项 | :x: 无 | 无法引导用户从固定选项中选择 |
| **Callback Queries** | 用户点击 Inline 按钮后 Bot 收到回调并可更新消息 | :x: 无 | 缺少 Bot 与用户之间的结构化交互循环 |

### P1 — 重要体验功能缺失

| Telegram 功能 | 描述 | Robrix2 现状 | 差距 |
|---------------|------|-------------|------|
| **Inline Mode** | 在任意聊天中 `@bot_name query` 触发 Bot 返回结果 | :x: 无 | 只能在 Bot 所在房间交互，不能跨房间调用 |
| **Deep Linking** | 通过链接 `t.me/bot?start=param` 传参启动 Bot | :x: 无 | 无法通过链接分享 Bot 并传递上下文 |
| **Bot Profile** | Bot 的 About、Description、头像、描述图片 | :warning: 部分 | 依赖 Matrix profile，无 Bot 专属 profile 编辑 UI |
| **Menu Button** | 聊天窗口的 Bot 菜单按钮 | :x: 无 | 无专属入口快速调出 Bot 功能 |
| **Bot-to-Bot 通信** | Bot 之间可以互相交互 | :x: 无 | 无 Bot 间编排/链式调用能力 |
| **Privacy Mode** | 群组中 Bot 默认只收到 `/command` 和 reply | :warning: 依赖 Matrix | Matrix 无对等概念，Bot 默认收到所有消息 |

### P2 — 高级/商业化功能缺失

| Telegram 功能 | 描述 | Robrix2 现状 |
|---------------|------|-------------|
| **Payments / Stars** | 内置支付流程、数字货币 | :x: 无 |
| **Mini Apps (Web Apps)** | Bot 内嵌 JS Web 应用 | :x: 无 |
| **HTML5 Games** | 游戏平台，排行榜 | :x: 无 |
| **Stickers / Custom Emoji** | Bot 创建贴纸包 | :x: 无 |
| **Paid Media / Subscriptions** | 付费内容、订阅分层 | :x: 无 |
| **Ad Revenue Sharing** | 广告收入分成 | :x: 无 |
| **Web Login** | Bot 驱动的第三方网站认证 | :x: 无 |
| **Managed Bots** | 代管其他 Bot | :x: 无 |
| **Bots for Business** | 企业客服 Bot 模式 | :x: 无 |
| **Attachment Menu** | 附件菜单直接调用 Bot | :x: 无 |
| **i18n 自动适配** | Bot 根据用户语言自动切换 | :x: 无 |
| **Bot 健康监控** | 回复率、响应时间告警 | :x: 无 |

---

## 建议的优先路线图

### Phase 1 — 让 Bot 真正"可交互"

1. **Bot Commands 声明与发现** — Bot 注册命令列表，用户输入 `/` 时显示可用命令
2. **Inline Keyboards (按钮)** — 消息附带可点击按钮，支持回调
3. **Callback 机制** — 点击按钮后触发 Bot 回调并更新消息

### Phase 2 — 扩展 Bot 触达范围

4. **Inline Mode** — `@bot` 跨房间查询
5. **Deep Linking** — 链接分享 Bot 并传参
6. **Bot Menu Button** — 聊天窗口专属 Bot 菜单入口
7. **Bot Profile 编辑** — 专属 About/Description/Avatar 管理

### Phase 3 — 平台化

8. Mini Apps / Web Apps
9. 支付集成
10. Bot 间通信与编排

---

## 总结

Robrix2 目前的 Bot 功能集中在**管理层**（创建、删除、绑定、发现），相当于 Telegram 的 BotFather 管理部分。但在**用户交互层**（Commands、Keyboards、Inline Mode、Callback）几乎为零——这恰恰是 Telegram Bot 生态最核心的部分。

最大的差距不是缺少高级功能（支付、游戏等），而是**Bot 与用户之间缺乏结构化交互能力**。用户只能发纯文本给 Bot，Bot 也只能回纯文本，没有按钮、没有命令菜单、没有回调更新。这使得 Bot 的实用性大打折扣。
