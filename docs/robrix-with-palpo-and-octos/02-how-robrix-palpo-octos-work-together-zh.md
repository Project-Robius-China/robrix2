# 架构原理：Robrix + Palpo + Octos 如何协同工作

[English Version](02-how-robrix-palpo-octos-work-together.md)

> **目标：** 阅读本指南后，你将理解 Matrix Application Service（应用服务）机制如何运作，Octos 如何作为 App Service 注册到 Palpo 以接收和回复消息，以及消息从 Robrix 经过 Palpo 到达 AI 机器人再返回的完整生命周期。

本文档解释 Robrix + Palpo + Octos 系统背后的**工作机制**。如需部署请参阅 [01-deploying-palpo-and-octos-zh.md](01-deploying-palpo-and-octos-zh.md)。如需使用指南请参阅 [03-using-robrix-with-palpo-and-octos-zh.md](03-using-robrix-with-palpo-and-octos-zh.md)。

---

## 目录

1. [三个项目概览](#1-三个项目概览)
2. [Matrix 协议基础](#2-matrix-协议基础)
3. [Application Service 机制](#3-application-service-机制)
4. [消息生命周期](#4-消息生命周期)
5. [端口与协议](#5-端口与协议)
6. [BotFather 系统](#6-botfather-系统)
7. [延伸阅读](#7-延伸阅读)

---

## 1. 三个项目概览

| 项目 | 角色 | 功能说明 |
|------|------|----------|
| [**Robrix**](https://github.com/Project-Robius-China/robrix2) | Matrix 客户端 | 使用 Rust 和 [Makepad](https://github.com/makepad/makepad/) 编写的跨平台 Matrix 聊天客户端。支持 macOS、Linux、Windows、Android 和 iOS 原生运行。这是用户直接交互的应用程序——在这里阅读和发送消息。 |
| [**Palpo**](https://github.com/palpo-im/palpo) | Matrix 服务器 | Rust 原生的 Matrix 主服务器（homeserver）。使用 PostgreSQL 存储用户账号、房间和消息。负责在客户端（Robrix）和应用服务（Octos）之间路由事件。可以把它理解为系统的"中央邮局"。 |
| [**Octos**](https://github.com/octos-org/octos) | AI 机器人（应用服务） | Rust 原生的 AI 代理平台，以 [Matrix Application Service](https://spec.matrix.org/latest/application-service-api/) 的形式运行。从 Palpo 接收消息，将其转发给 LLM（DeepSeek、OpenAI、Anthropic 等），然后将 AI 的回复发布到房间中。 |

三个项目各自独立且完全开源。组合在一起，它们构成一个完整的 AI 聊天系统：用户通过原生聊天界面与 AI 机器人交互，所有通信都通过符合标准的 Matrix 服务器进行路由。

---

## 2. Matrix 协议基础

在深入架构之前，先了解理解本系统所需的 Matrix 协议核心概念。

### 主服务器（Homeserver）

主服务器是 Matrix 的骨干。它存储用户账号、房间状态和消息历史。每个用户恰好属于一个主服务器——例如，`@alice:example.com` 属于 `example.com` 上的主服务器。在我们的系统中，Palpo 就是主服务器。

### 房间（Room）

房间是一个共享的对话空间。当你发送消息时，消息是发送到房间的，而不是直接发给另一个用户。房间中的所有参与者都能看到消息。房间中可以包含任意组合的真实用户和机器人。

### 事件（Event）

Matrix 中的一切都是**事件**。一条消息是事件（`m.room.message`）。加入房间是事件（`m.room.member`）。修改房间名称也是事件。事件是最基本的数据单元——它们是不可变的、有序的，构成了房间的完整历史记录。

### 客户端-服务器 API（Client-Server API）

这是客户端（如 Robrix）与其主服务器（Palpo）之间的通信方式。Client-Server API 用于：

- 登录和注册账号
- 发送消息（`PUT /_matrix/client/v3/rooms/{roomId}/send/...`）
- 同步房间状态和消息历史
- 管理房间（创建、加入、邀请）

Robrix 完全通过此 API 与 Palpo 通信。Octos 在发送机器人回复时也使用此 API。

### 服务器间 API（Federation）

这是主服务器之间相互通信的方式。如果 `@alice:server-a.com` 在一个包含 `@bob:server-b.com` 的房间中发送消息，两个主服务器会通过联邦协议（Federation）通信来传递事件。这正是 Matrix 成为去中心化协议的关键所在。详见 [04-federation-with-palpo-zh.md](04-federation-with-palpo-zh.md)。

### 滑动同步（Sliding Sync）

传统的 Matrix 同步会在启动时下载完整的房间状态，在移动设备或受限设备上可能很慢。**滑动同步（Sliding Sync）** 是 Matrix 规范中定义的一种优化同步机制，只发送客户端当前需要的数据——就像在房间列表上滑动一个窗口。Robrix 要求主服务器支持 Sliding Sync。Palpo 原生支持这一特性。

---

## 3. Application Service 机制

本节是架构文档的核心。理解 Application Service（应用服务）机制是理解 Octos 如何接入 Palpo 的关键。

### 3.1 什么是 Matrix Application Service？

Matrix Application Service 是一种在主服务器上拥有**特殊权限**的程序。与使用用户名和密码登录的普通客户端不同，应用服务：

- **通过 YAML 注册文件向主服务器注册**（而不是通过 Client-Server API）
- **声明独占的用户命名空间** -- 拥有一系列用户 ID，并可以代表其中任何一个用户行事
- **从主服务器接收推送的事件** -- 无需轮询或同步
- **不受速率限制** -- 可以按需要的任何速度发送消息
- **可以动态创建虚拟用户**，无需经过常规注册流程

这是为桥接（将 Matrix 连接到 Telegram、Slack 等）和机器人设计的机制。Octos 使用它来运行 AI 机器人。

> Matrix 规范参考：[Application Service API](https://spec.matrix.org/latest/application-service-api/)

### 3.2 注册文件：Palpo 如何发现 Octos

启动时，Palpo 从 `palpo.toml` 中 `appservice_registration_dir` 指定的目录读取所有 `.yaml` 文件。每个文件代表一个已注册的应用服务。

注册文件（`appservices/octos-registration.yaml`）包含：

```yaml
id: octos-matrix-appservice
url: "http://octos:8009"

as_token: "<your-as-token>"
hs_token: "<your-hs-token>"

sender_localpart: octosbot
rate_limited: false

namespaces:
  users:
    - exclusive: true
      regex: "@octosbot_.*:127\\.0\\.0\\.1:8128"
    - exclusive: true
      regex: "@octosbot:127\\.0\\.0\\.1:8128"
```

各字段说明：

| 字段 | 用途 |
|------|------|
| `id` | 此应用服务的唯一名称。Palpo 用它来跟踪事件投递状态。 |
| `url` | Palpo 发送事件的 HTTP 端点。这是 Octos 在 Docker 网络内的地址。 |
| `as_token` | Octos 调用 Palpo API 时出示的令牌。证明"我是已注册的应用服务"。 |
| `hs_token` | Palpo 向 Octos 推送事件时出示的令牌。证明"我是你注册时对应的主服务器"。 |
| `sender_localpart` | 主机器人的用户名。与 `server_name` 组合后变成 `@octosbot:127.0.0.1:8128`。 |
| `namespaces.users` | 此应用服务独占的用户 ID 正则表达式模式。 |

这是一种**双向信任关系**：Octos 用 `as_token` 向 Palpo 认证，Palpo 用 `hs_token` 向 Octos 认证。双方必须持有相同的令牌对，分别配置在两个文件中：注册 YAML 文件（给 Palpo 读取）和 `botfather.json`（给 Octos 读取）。如果不匹配，系统将无法工作。请参阅部署指南中的[令牌匹配检查清单](01-deploying-palpo-and-octos-zh.md#38-令牌匹配检查清单)。

### 3.3 用户命名空间：机器人身份

`namespaces.users` 部分告诉 Palpo 哪些用户 ID 属于 Octos。正则表达式模式声明了特定的范围：

- **`@octosbot:127.0.0.1:8128`** -- 主机器人，也称为 **BotFather**。这是用户的入口点。
- **`@octosbot_.*:127.0.0.1:8128`** -- 动态创建的子机器人（例如 `@octosbot_translator:127.0.0.1:8128`）。`.*` 通配符意味着 Octos 可以创建任何带 `octosbot_` 前缀的用户 ID。

设置 `exclusive: true` 意味着**没有其他实体可以创建或声明这些用户 ID**。如果普通用户尝试注册为 `@octosbot:127.0.0.1:8128`，Palpo 会拒绝该请求。

命名空间机制也是 Palpo 决定是否通知 Octos 的依据。当有人邀请 `@octosbot:127.0.0.1:8128` 加入房间时，Palpo 检查其已注册的应用服务，发现此用户 ID 匹配 Octos 的命名空间，于是将邀请事件推送给 Octos。

### 3.4 事件推送流程

应用服务协议是**推送式**的，不是拉取式的。应用服务不需要同步或轮询——主服务器主动向它发送事件。

当一条消息到达一个包含应用服务用户的房间时：

1. **Palpo 检查其应用服务注册表。** 它查看哪些应用服务用户是该房间的成员。如果 `@octosbot:127.0.0.1:8128` 在房间中，Palpo 就知道需要通知 Octos。

2. **Palpo 向 Octos 发送 HTTP PUT 请求。** 请求发送到 `{url}/transactions/{txnId}`——在我们的场景中是 `http://octos:8009/transactions/{txnId}`。请求体包含事件数据（发送者、房间 ID、消息内容等），Palpo 附带 `hs_token` 进行认证。

3. **Octos 处理事件。** 它接收事件，识别房间和发送者，并决定如何响应。对于 AI 机器人来说，这意味着调用配置的 LLM。

4. **Octos 通过 Palpo 的 Client-Server API 发送回复。** Octos 没有直接连接到 Robrix 的通道。它以机器人用户的身份通过 Palpo 发送消息，就像其他任何客户端一样，使用 `as_token` 进行认证。

这种推送模型非常高效：Octos 不会浪费资源进行轮询，事件以最小延迟传递。

---

## 4. 消息生命周期

以下是一条消息在系统中的完整旅程，从你在 Robrix 中输入到 AI 机器人的回复出现在屏幕上。

### 逐步数据流

```
用户在 Robrix 中输入 "Hello"
         |
         v
+-----------------+
| 1. Robrix 发送  |  PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message
|    (CS API)     |  -> http://127.0.0.1:8128 (Palpo)
+--------+--------+
         |
         v
+-----------------+
| 2. Palpo 存储   |  事件保存到 PostgreSQL
|    事件         |  房间状态更新
+--------+--------+
         |
         v
+-----------------+
| 3. Palpo 推送   |  PUT /transactions/{txnId} -> http://octos:8009
|    到 Octos     |  (Appservice API，Docker 内部网络)
+--------+--------+
         |
         v
+-----------------+
| 4. Octos 调用   |  POST /v1/chat/completions -> DeepSeek API
|    LLM          |  (或其他配置的提供商)
+--------+--------+
         |
         v
+-----------------+
| 5. Octos 发送   |  PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message
|    回复         |  -> http://palpo:8008 (Docker 内部网络)
|    (CS API)     |  Auth: Bearer {as_token}
+--------+--------+
         |
         v
+-----------------+
| 6. Palpo 存储   |  机器人回复事件已保存
|    并投递       |  Sliding Sync 推送到 Robrix
+--------+--------+
         |
         v
用户在 Robrix 中看到 AI 回复
```

### 每一步发生了什么

**步骤 1 -- Robrix 发送消息。** 当你点击发送时，Robrix 向 Palpo 的 Client-Server API 发起 HTTP PUT 请求。请求包含房间 ID、事件类型（`m.room.message`）和消息内容。Robrix 连接到 `http://127.0.0.1:8128`，即 Palpo 暴露在主机上的端口。

**步骤 2 -- Palpo 存储事件。** Palpo 接收消息，分配一个事件 ID，并将其持久化到 PostgreSQL。房间状态更新以反映新消息。

**步骤 3 -- Palpo 将事件推送给 Octos。** Palpo 检查其应用服务注册表，发现 `@octosbot:127.0.0.1:8128` 是该房间的成员。它通过 HTTP PUT 请求将事件发送到 Octos 的应用服务端点（`http://octos:8009`），路径为 `/transactions/{txnId}`。这使用 Docker 内部网络——流量不会离开主机。

**步骤 4 -- Octos 调用 LLM。** Octos 接收事件，提取消息内容，并调用配置的 LLM 提供商（例如 DeepSeek 的 `/v1/chat/completions` 端点）。它会包含对话历史作为上下文。

**步骤 5 -- Octos 发送回复。** LLM 响应后，Octos 以机器人用户（`@octosbot:127.0.0.1:8128`）的身份，通过 Palpo 的 Client-Server API 发送回复。它使用 `as_token` 进行认证。注意 Octos 连接的是 `http://palpo:8008`（Docker 内部），而不是 `127.0.0.1:8128`（主机）。

**步骤 6 -- Palpo 将回复投递给 Robrix。** Palpo 存储机器人的回复事件，并将其包含在 Robrix 的下一次 Sliding Sync 响应中。Robrix 接收事件并在对话中显示 AI 机器人的消息。

### 架构图

```
+----------+                        +----------+                       +----------+         +-----+
|  Robrix  |  Client-Server API     |  Palpo   |  Appservice API       |  Octos   |  HTTPS  | LLM |
| (客户端) | -------------------->  | (服务器) | --------------------> | (机器人) | ------> |     |
|          | <--------------------  |          | <-------------------  |          | <------ |     |
+----------+   Sliding Sync        +----------+  Client-Server API    +----------+         +-----+
  你的机器                         Docker :8128      Docker :8009                          外部服务
```

关键观察：

- **Robrix 从不直接与 Octos 通信。** 所有通信都通过 Palpo 中转。Robrix 甚至不知道 Octos 的存在——它只看到房间中的机器人用户。
- **两条不同的路径，同一个 API。** Robrix 和 Octos 都使用 Client-Server API 与 Palpo 通信，但 Octos 使用 `as_token`（应用服务凭证）而非普通用户会话进行认证。
- **内部流量与外部流量。** Robrix 通过主机端口（8128）连接。Palpo 和 Octos 在 Docker 内部网络上通信（使用服务名 `palpo:8008` 和 `octos:8009`）。只有 LLM API 调用会访问互联网。

---

## 5. 端口与协议

| 连接 | 协议 | 默认端口 | 方向 | 说明 |
|------|------|----------|------|------|
| Robrix -> Palpo | Client-Server API (Sliding Sync) | 8128 (主机) -> 8008 (容器) | 双向 | Robrix 唯一需要的端口。暴露在主机上。 |
| Palpo -> Octos | Appservice API | 8009 (主机) -> 8009 (容器) | Palpo 推送事件 | 同时暴露到主机用于调试。内部使用 Docker 服务名 `octos`。 |
| Octos -> Palpo | Client-Server API | 8008 (Docker 内部网络) | Octos 发送回复 | 使用 Docker 服务名 `palpo`。通过 `as_token` 认证。 |
| Octos 管理面板 | HTTP | 8010 (主机) -> 8080 (容器) | 入站 | 可选的管理 UI，用于监控 Octos。 |
| Octos -> LLM | HTTPS | 443 (出站) | 出站 | 对 LLM 提供商的外部 API 调用。 |

**为什么 Palpo 有两个不同的端口（8008 vs. 8128）？** 在 Docker 网络内部，Palpo 监听 8008 端口（容器端口）。Docker 将主机端口 8128 映射到容器端口 8008。Octos 运行在同一 Docker 网络中，直接连接 `palpo:8008`。Robrix 运行在主机上，连接 `127.0.0.1:8128`。

---

## 6. BotFather 系统

Octos 实现了 **BotFather** 模式，通过单个应用服务管理多个 AI 机器人。

### 父机器人与子机器人

**BotFather** 是主机器人（`@octosbot:server_name`）。它是入口点——用户邀请 BotFather 加入房间即可开始交互。但 BotFather 还可以创建**子机器人**，每个子机器人拥有不同的个性和用途。

```
BotFather (@octosbot:127.0.0.1:8128)
    |
    +-- 翻译机器人 (@octosbot_translator:127.0.0.1:8128)
    |       系统提示词: "你是一个翻译。将所有消息翻译成中文。"
    |
    +-- 代码审查员 (@octosbot_reviewer:127.0.0.1:8128)
    |       系统提示词: "你是一个代码审查员。检查代码中的错误和风格问题。"
    |
    +-- 写作助手 (@octosbot_writer:127.0.0.1:8128)
            系统提示词: "你是一个写作助手。帮助改善文字的清晰度和语气。"
```

### 子机器人的工作原理

每个子机器人都有自己的：

- **显示名称** -- 在聊天中显示的可读名称（例如："翻译机器人"）
- **系统提示词（System Prompt）** -- 定义机器人个性和行为的指令
- **用户 ID** -- 使用 `octosbot_` 前缀生成（例如：`@octosbot_translator:127.0.0.1:8128`）

子机器人在运行时动态创建。它们不需要单独的注册文件或独立的进程。所有子机器人都在同一个 Octos 应用服务实例中运行。

### 为什么这能工作：命名空间的关联

还记得注册文件中的命名空间正则表达式吗？

```yaml
namespaces:
  users:
    - exclusive: true
      regex: "@octosbot_.*:127\\.0\\.0\\.1:8128"
```

这个通配符模式正是使动态创建子机器人成为可能的原因。当 Octos 创建新的子机器人（如 `@octosbot_translator:127.0.0.1:8128`）时，Palpo 检查已注册的命名空间，确认该用户 ID 在 Octos 的独占范围内，然后允许创建。无需额外配置。

### 从 Robrix 管理机器人

Robrix 内置了通过 BotFather 系统创建和管理子机器人的 UI。在 Robrix 的**机器人设置**面板中，你可以：

1. 启用应用服务支持并配置 BotFather 用户 ID
2. 创建新的子机器人，自定义用户名、显示名称和系统提示词
3. 查看和管理现有机器人

详细的操作步骤请参阅使用指南中的[机器人管理](03-using-robrix-with-palpo-and-octos-zh.md)部分。

---

## 7. 延伸阅读

- **Matrix Application Service 规范：** [spec.matrix.org -- Application Service API](https://spec.matrix.org/latest/application-service-api/) -- 应用服务的官方协议规范。
- **Octos 文档：** [octos-org.github.io/octos](https://octos-org.github.io/octos/) -- Octos 的完整文档，包括全部 14 种 LLM 提供商、频道、技能和记忆系统。
- **Palpo GitHub：** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) -- Palpo 主服务器文档和源代码。
- **Robrix GitHub：** [Project-Robius-China/robrix2](https://github.com/Project-Robius-China/robrix2) -- Robrix 客户端源代码和功能跟踪。
- **Matrix 规范 (Client-Server API)：** [spec.matrix.org -- Client-Server API](https://spec.matrix.org/latest/client-server-api/) -- 完整的 Client-Server API 规范，包括 Sliding Sync。
- **部署指南：** [01-deploying-palpo-and-octos-zh.md](01-deploying-palpo-and-octos-zh.md) -- 如何部署和配置系统。
- **使用指南：** [03-using-robrix-with-palpo-and-octos-zh.md](03-using-robrix-with-palpo-and-octos-zh.md) -- 如何使用 Robrix 与 AI 机器人交互的分步指南。

---

*本文档描述的是截至 2026 年 4 月的架构。如需最新更新，请参阅各项目的代码仓库。*
