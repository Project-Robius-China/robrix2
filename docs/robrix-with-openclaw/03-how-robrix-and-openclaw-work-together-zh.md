# 架构原理：Robrix 与 OpenClaw 如何协作

[English](03-how-robrix-and-openclaw-work-together.md)

> **目标：** 阅读本文档后，你将理解 OpenClaw 如何以普通客户端身份连接 Matrix，消息如何在 Robrix、Matrix 服务器和 OpenClaw AI 代理之间流转，以及这与 Octos 使用的 Application Service 模式有何本质区别。

本文档解释 Robrix + OpenClaw 系统背后的**机制**。如需部署，请参见 [部署指南](01-deploying-openclaw-with-matrix-zh.md)。如需使用，请参见 [使用指南](02-using-robrix-with-openclaw-zh.md)。

---

## 目录

1. [两个项目概览](#1-两个项目概览)
2. [OpenClaw 如何连接 Matrix](#2-openclaw-如何连接-matrix)
3. [消息生命周期](#3-消息生命周期)
4. [客户端模式 vs Application Service 模式](#4-客户端模式-vs-application-service-模式)
5. [端到端加密（E2EE）](#5-端到端加密e2ee)
6. [延伸阅读](#6-延伸阅读)

---

## 1. 两个项目概览

| 项目 | 角色 | 作用 |
|------|------|------|
| [**Robrix**](https://github.com/Project-Robius-China/robrix2) | Matrix 客户端 | 使用 Rust + [Makepad](https://github.com/makepad/makepad/) 构建的跨平台 Matrix 聊天客户端。这是用户界面——你在这里读写消息。 |
| [**OpenClaw**](https://github.com/openclaw/openclaw) | AI 代理框架 | 开源 AI 助手平台，通过 Matrix 频道插件以**普通客户端**身份登录 Matrix 服务器。接收用户消息，调用 LLM 生成回复，再发送回房间。 |

两个项目完全独立。OpenClaw 不是专为 Robrix 设计的，也不是专为 Matrix 设计的——它支持 Telegram、Discord、Slack 等多种频道。Matrix 只是其中之一。

---

## 2. OpenClaw 如何连接 Matrix

OpenClaw 通过 **Client-Server API** 连接 Matrix，方式和 Robrix 一样——它就是一个普通的 Matrix 客户端。

### 连接流程

```
1. OpenClaw 启动时，用 userId + password 调用 POST /_matrix/client/v3/login
2. 服务器返回 access_token，OpenClaw 缓存到 ~/.openclaw/credentials/
3. OpenClaw 开始 Sliding Sync 循环，持续拉取新事件
4. 收到 m.room.message 事件时，提取消息内容，调用 LLM
5. LLM 返回回复后，OpenClaw 通过 PUT /_matrix/client/v3/rooms/{roomId}/send/ 发送回复
```

### 关键特征

- **登录方式**：用户名 + 密码（和普通用户一样）
- **消息获取**：通过 Sync 主动拉取（不是服务器推送）
- **权限级别**：和普通用户完全一样（受速率限制、需要被邀请才能加入房间）
- **底层 SDK**：OpenClaw 的 Matrix 插件基于 [matrix-js-sdk](https://github.com/matrix-org/matrix-js-sdk)（官方 JavaScript SDK）

---

## 3. 消息生命周期

### 数据流图

```
用户在 Robrix 中输入 "你好"
         |
         v
+-----------------+
| 1. Robrix 发送  |  PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message
|    通过 CS API  |  -> Palpo (http://127.0.0.1:8128)
+--------+--------+
         |
         v
+-----------------+
| 2. Palpo 存储   |  事件写入 PostgreSQL
|    事件         |  房间状态更新
+--------+--------+
         |
         v
+-----------------+
| 3. OpenClaw     |  通过 Sliding Sync 拉取到新事件
|    收到消息     |  （OpenClaw 是普通客户端，主动同步）
+--------+--------+
         |
         v
+-----------------+
| 4. OpenClaw     |  POST https://api.deepseek.com/v1/chat/completions
|    调用 LLM     |  携带对话历史作为上下文
+--------+--------+
         |
         v
+-----------------+
| 5. OpenClaw     |  PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message
|    发送回复     |  -> Palpo (http://127.0.0.1:8128)
|    通过 CS API  |  认证：Bearer {access_token}
+--------+--------+
         |
         v
+-----------------+
| 6. Palpo 存储   |  Bot 的回复事件写入数据库
|    并推送       |  Sliding Sync 推送到 Robrix
+--------+--------+
         |
         v
用户在 Robrix 中看到 AI 回复
```

### 架构图

```
+----------+                        +----------+                       +----------+         +-----+
|  Robrix  |  Client-Server API     |  Palpo   |  Client-Server API    | OpenClaw |  HTTPS  | LLM |
| (客户端) | -------------------->  | (服务器) | <------------------> | (AI Bot) | ------> |     |
|          | <--------------------  |          |   Sliding Sync        |          | <------ |     |
+----------+   Sliding Sync        +----------+                       +----------+         +-----+
  你的电脑                         Docker :8128                        你的电脑              外部 API
```

**关键观察：**

- **Robrix 和 OpenClaw 对 Palpo 来说地位完全平等** —— 都是通过 Client-Server API 连接的普通客户端。
- **OpenClaw 不依赖服务器端配置** —— 不需要修改 Palpo 的任何配置文件（对比 Octos 需要注册 AppService YAML）。
- **只有 LLM 调用离开本机** —— Robrix ↔ Palpo ↔ OpenClaw 全部在本地网络，只有 DeepSeek API 调用走公网。

---

## 4. 客户端模式 vs Application Service 模式

Robrix 生态中有两种接入 AI Bot 的方式：OpenClaw 使用的**客户端模式**和 Octos 使用的 **Application Service 模式**。它们的核心区别如下：

### 4.1 连接机制对比

| | OpenClaw（客户端模式） | Octos（Application Service 模式） |
|---|---|---|
| **连接方式** | 和普通用户一样，用密码登录 | 通过 YAML 注册文件在服务器端注册 |
| **消息获取** | **Sync 拉取**——OpenClaw 主动轮询服务器获取新事件 | **服务器推送**——Palpo 主动将事件推送到 Octos 的 HTTP 端点 |
| **认证方式** | access_token（用户级别） | as_token / hs_token（服务级别，双向认证） |
| **服务器端配置** | **无需任何配置**——Bot 就是一个普通用户 | **需要注册**——在 Palpo 的 `appservice_registration_dir` 放置 YAML 文件 |
| **用户命名空间** | 只有一个用户 ID | 可以声明排他的用户命名空间，动态创建子 Bot |
| **速率限制** | 受限（和普通用户一样） | 不受限（`rate_limited: false`） |

### 4.2 能力对比

| 能力 | OpenClaw | Octos |
|------|----------|-------|
| 基本对话 | 支持 | 支持 |
| 多模型切换 | 支持（14+ LLM provider） | 支持 |
| E2EE 加密 | 支持（Rust crypto SDK） | 不需要（AppService 绕过加密） |
| 服务器端管理 | 不需要 | 需要管理员权限注册 AppService |
| 多频道（Telegram、Discord 等） | 支持 | 仅 Matrix |
| 对 homeserver 的要求 | 任何标准 Matrix 服务器 | 需要支持 AppService API |

### 4.3 消息延迟

| 环节 | OpenClaw | Octos |
|------|----------|-------|
| 消息到达 Bot | Sync 间隔（通常 1-5 秒） | 即时推送（< 100ms） |
| LLM 响应 | 取决于 LLM provider | 取决于 LLM provider |
| Bot 发送回复 | 即时 | 即时 |

> OpenClaw 使用 Sliding Sync 的长轮询模式，实际延迟通常在 1-2 秒，对于聊天场景几乎感觉不到。

### 4.4 部署复杂度

| | OpenClaw | Octos |
|---|---|---|
| 服务器端 | 无需配置 | 需要放置注册 YAML、配置 token |
| 客户端 | 安装 OpenClaw + 编辑一个 JSON 文件 | 需要 Docker Compose 编排三个服务 |
| Token 管理 | 密码自动登录，token 自动缓存 | 需要手动生成并同步 as_token / hs_token |
| 架构复杂度 | 简单（一个进程） | 复杂（Palpo + Octos + PostgreSQL） |

### 4.5 什么时候用哪个？

| 场景 | 推荐方案 |
|------|---------|
| 快速测试 AI 对话 | **OpenClaw** —— 5 分钟配好，不需要碰服务器 |
| 个人 AI 助手 | **OpenClaw** —— 简单、灵活、支持多频道 |
| 团队内多个专业 Bot | **Octos** —— BotFather 可以动态创建多个子 Bot |
| 需要服务器端管理 | **Octos** —— AppService 由管理员注册和控制 |
| 高并发消息 | **Octos** —— 服务器推送 + 无速率限制 |
| 跨平台 AI 代理（同时接入 Telegram/Discord） | **OpenClaw** —— 原生支持多频道 |

---

## 5. 端到端加密（E2EE）

### OpenClaw 如何处理加密

OpenClaw 的 Matrix 插件使用 matrix-js-sdk 的 **Rust crypto 路径**，实现了 Olm（一对一密钥交换）和 Megolm（群组加密）协议。

当 `"encryption": true` 配置启用后：

1. **首次登录**：OpenClaw 创建加密设备，生成 cross-signing identity
2. **自动引导**：执行 secret storage bootstrap，设备被标记为 "verified by its owner"
3. **接收消息**：OpenClaw 解密 Megolm 加密的消息
4. **发送回复**：回复自动加密发送

### 注意事项

- **历史消息不可解密** —— 在 OpenClaw 设备创建之前发送的消息，其 Megolm 会话密钥未分发给 OpenClaw，永远无法解密。
- **vs Octos** —— Octos 作为 AppService 接收的是**服务器端解密后的明文事件**，不需要处理 E2EE。OpenClaw 作为客户端必须自己处理加密。

---

## 6. 延伸阅读

- **OpenClaw 文档：** [docs.openclaw.ai](https://docs.openclaw.ai/) — OpenClaw 完整文档。
- **OpenClaw Matrix 插件：** [docs.openclaw.ai/channels/matrix](https://docs.openclaw.ai/channels/matrix) — 官方 Matrix 频道插件参考。
- **Matrix Client-Server API 规范：** [spec.matrix.org -- Client-Server API](https://spec.matrix.org/latest/client-server-api/) — OpenClaw 使用的协议。
- **Matrix Application Service API 规范：** [spec.matrix.org -- Application Service API](https://spec.matrix.org/latest/application-service-api/) — Octos 使用的协议。
- **Octos 架构原理：** [03-how-robrix-palpo-octos-work-together-zh.md](../robrix-with-palpo-and-octos/03-how-robrix-palpo-octos-work-together-zh.md) — Octos AppService 模式的完整解析。
- **部署指南：** [01-deploying-openclaw-with-matrix-zh.md](01-deploying-openclaw-with-matrix-zh.md) — 如何部署 OpenClaw + Matrix。
- **使用指南：** [02-using-robrix-with-openclaw-zh.md](02-using-robrix-with-openclaw-zh.md) — 如何使用 Robrix 与 OpenClaw 代理对话。

---

*本文档基于 2026 年 4 月的实测结果编写。最新更新请参见各项目仓库。*
