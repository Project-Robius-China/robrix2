# 部署指南：Robrix + Palpo + Octos

[English Version](01-deploying-palpo-and-octos.md)

> **目标：** 按照本指南操作后，你的机器上将通过 Docker Compose 运行 Palpo（Matrix 主服务器）、Octos（AI 机器人）和 PostgreSQL。Robrix 将能连接到你的 Palpo 服务器，你可以与 Octos AI 机器人对话。

本指南带你一步步部署后端服务：从克隆源码，到配置各组件，再到验证一切正常运行。

> **只想快速试试？** 跳到 [快速开始](#2-快速开始) — 5 步即可运行。
>
> **想了解每个配置背后的原理？** 参阅 [架构原理](02-how-robrix-palpo-octos-work-together-zh.md) 了解完整解释。

---

## 目录

1. [前提条件](#1-前提条件)
2. [快速开始](#2-快速开始)
3. [配置详解](#3-配置详解)
4. [端到端验证](#4-端到端验证)
5. [故障排除](#5-故障排除)
6. [延伸阅读](#6-延伸阅读)

---

## 1. 前提条件

开始之前，请确保你已具备以下条件：

| 需求 | 版本 | 备注 |
|------|------|------|
| **Docker** + **Docker Compose** | v2+ | 运行 `docker compose version` 检查。Docker Desktop 自带 Compose v2。 |
| **Git** | 任意 | 用于克隆源码仓库。 |
| **一个 LLM API Key** | -- | 如 [DeepSeek](https://platform.deepseek.com/)（有免费额度）、OpenAI、Anthropic 等。 |
| **Robrix** | 最新版 | 参阅 [Robrix 快速开始](../robrix/getting-started-with-robrix-zh.md) 了解下载或构建方式。 |

> **注意：** Palpo 和 Octos 都在 Docker 内从源码构建。你不需要在宿主机上安装 Rust 或任何其他工具链。

---

## 2. 快速开始

5 步在本地跑通所有服务。

### 步骤 1：克隆仓库

```bash
git clone https://github.com/Project-Robius-China/robrix2.git
cd robrix2/palpo-and-octos-deploy
```

### 步骤 2：运行初始化脚本

```bash
./setup.sh
```

此脚本会：
- 将 Palpo 源码仓库克隆到 `repos/palpo/`（从 GitHub 浅克隆）
- 将 Octos 源码仓库克隆到 `repos/octos/`（从 GitHub 浅克隆）
- 从 `.env.example` 创建 `.env` 文件

两个服务均在 Docker 内从源码构建，以支持所有架构（x86_64、ARM64/Apple Silicon 等）。

> **文件存放位置说明：** 运行 `setup.sh` 和 `docker compose up` 后，`palpo-and-octos-deploy/` 目录会包含：
> - `repos/` — Palpo 和 Octos 的源码（Docker 用来构建镜像）
> - `data/` — 运行时数据（PostgreSQL 数据库、Octos 会话、媒体文件）
> - `.env` — 你的环境变量（API Key 等）
>
> 这些目录已在 `.gitignore` 中列出，**不会**被提交到版本库。

### 步骤 3：设置 API Key

编辑 `.env`，将 `your-api-key-here` 替换为你的实际 API Key：

```
DEEPSEEK_API_KEY=sk-xxxxxxxxxxxxxxxxxxxxxxxx
```

### 步骤 4：启动服务

```bash
docker compose up -d
```

> **重要：** 首次运行会从源码编译 Palpo 和 Octos，这可能需要 **10--30 分钟**（取决于你的机器性能和网络速度）。Palpo 需要编译其 Rust 代码；Octos 还额外需要下载 Node.js、Chromium 等技能插件的运行时工具。后续启动会使用缓存镜像，几秒内完成。

检查运行状态：

```bash
docker compose ps
```

你应该看到三个服务（`palpo_postgres`、`palpo`、`octos`）都处于 `running` 状态。

### 步骤 5：用 Robrix 连接

1. **打开 Robrix**（还没有？参阅 [Robrix 快速开始](../robrix/getting-started-with-robrix-zh.md)）

2. **设置服务器地址**：在登录界面，在 **Homeserver URL** 输入框中输入 `http://127.0.0.1:8128`

3. **注册新账号**：输入用户名和密码，点击 **Sign up**

4. **与 AI 机器人对话**：登录后，创建一个房间并邀请机器人：
   - 点击房间中的邀请按钮
   - 输入 `@octosbot:127.0.0.1:8128`
   - 等待机器人加入房间（你应该能看到加入事件）
   - 发送一条消息——AI 机器人应该会回复！

**完成！** 你现在拥有了一个可工作的 Robrix + Palpo + Octos 系统。继续阅读了解配置详情，或跳到 [故障排除](#5-故障排除) 解决问题。

---

## 3. 配置详解

本节解释 `palpo-and-octos-deploy/` 目录中的每个配置文件。快速开始已经让你跑起来了——当你需要自定义时再来这里查阅。

> **注意：** 想了解架构以及每个组件为何如此配置，请参阅 [架构原理](02-how-robrix-palpo-octos-work-together-zh.md)。

### 3.1 目录结构

```
palpo-and-octos-deploy/
├── compose.yml                         # Docker Compose — 编排所有服务
├── setup.sh                            # 一次性初始化脚本
├── .env.example                        # 环境变量模板
├── palpo.toml                          # Palpo 主服务器配置
├── palpo.Dockerfile                    # Palpo Docker 构建（多阶段，release 模式）
├── appservices/
│   └── octos-registration.yaml         # 应用服务注册文件（连接 Palpo <-> Octos）
├── config/
│   ├── botfather.json                  # Octos 机器人配置（LLM + Matrix 通道）
│   └── octos.json                      # Octos 全局设置
├── repos/                              # 源码（由 setup.sh 创建，已 gitignore）
│   ├── palpo/                          # Palpo 主服务器源码
│   └── octos/                          # Octos 机器人源码
├── data/                               # 持久化数据（运行时自动创建，已 gitignore）
│   ├── pgsql/                          # PostgreSQL 数据库文件
│   ├── octos/                          # Octos 运行时数据
│   └── media/                          # Palpo 媒体存储
```

### 3.2 令牌生成

应用服务注册文件和 Octos 机器人配置共享两个密钥令牌，用于双向认证。示例文件中已预填开发用令牌，但**在生产环境中你必须重新生成**：

```bash
openssl rand -hex 32   # → 用作 as_token
openssl rand -hex 32   # → 用作 hs_token
```

这两个值必须在 `palpo-and-octos-deploy/appservices/octos-registration.yaml` 和 `palpo-and-octos-deploy/config/botfather.json` 中完全一致。如果不匹配，机器人将无法工作。详见 [3.8 令牌匹配检查清单](#38-令牌匹配检查清单)。

### 3.3 应用服务注册文件（`appservices/octos-registration.yaml`）

此文件告诉 Palpo 关于 Octos 的信息——Octos 管理哪些用户命名空间，以及将事件发送到哪里。

```yaml
id: octos-matrix-appservice
url: "http://octos:8009"

as_token: "d1f46062a08e4833b18286d95c5e09a5f3e4a1b2c3d4e5f6a7b8c9d0e1f2a3b4"
hs_token: "e2a57173b19f5944c29397ea6d6f1ab6a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9"

sender_localpart: octosbot
rate_limited: false

namespaces:
  users:
    - exclusive: true
      regex: "@octosbot_.*:127\\.0\\.0\\.1:8128"
    - exclusive: true
      regex: "@octosbot:127\\.0\\.0\\.1:8128"
  aliases: []
  rooms: []
```

| 字段 | 说明 |
|------|------|
| `id` | 此应用服务注册的唯一标识符。 |
| `url` | Palpo 发送事件的目标地址。使用 Docker 服务名 `octos`（不是 `localhost`），因为两个容器在同一个 Docker 网络中。 |
| `as_token` | Octos 调用 Palpo API 时使用的令牌。**必须**与 `botfather.json` 匹配。 |
| `hs_token` | Palpo 向 Octos 推送事件时使用的令牌。**必须**与 `botfather.json` 匹配。 |
| `sender_localpart` | 机器人的 Matrix 本地用户名。最终变为 `@octosbot:127.0.0.1:8128`。 |
| `rate_limited` | 设为 `false`，让机器人回复不受速率限制。 |
| `namespaces.users` | 此应用服务管理的用户 ID 正则匹配模式。包含机器人本身（`@octosbot:...`）和动态创建的子机器人（`@octosbot_*:...`）。 |

### 3.4 Palpo 配置（`palpo.toml`）

```toml
server_name = "127.0.0.1:8128"

allow_registration = true
yes_i_am_very_very_sure_i_want_an_open_registration_server_prone_to_abuse = true
enable_admin_room = true

appservice_registration_dir = "/var/palpo/appservices"

# HTTP 监听器（Client-Server API）
[[listeners]]
address = "0.0.0.0:8008"

[logger]
format = "pretty"

[db]
url = "postgres://palpo:palpo_dev_password@palpo_postgres:5432/palpo"
pool_size = 10

[well_known]
server = "127.0.0.1:8128"
client = "http://127.0.0.1:8128"
```

| 字段 | 说明 |
|------|------|
| `server_name` | 所有 Matrix ID 的域名部分（如 `@user:127.0.0.1:8128`）。 |
| `allow_registration` | 是否允许新用户注册。设为 `true` 以便 Robrix 用户创建账号。 |
| `yes_i_am_very_very_sure_...` | 当 `allow_registration = true` 时必填的安全确认。 |
| `enable_admin_room` | 启用服务器管理员房间。 |
| `appservice_registration_dir` | Palpo 启动时自动加载此目录下所有 `.yaml` 文件。Octos 就是通过这种方式被发现的。 |
| `[[listeners]]` | 网络监听器。每个条目定义一个 Palpo 监听的地址。 |
| `[logger]` | 日志格式。`"pretty"` 用于开发，`"json"` 用于生产。 |
| `[db]` | PostgreSQL 连接配置。`palpo_postgres` 是 Docker 服务名。密码必须与 `compose.yml` 中的 `POSTGRES_PASSWORD` 匹配。 |
| `[well_known]` | 用于客户端发现服务器。必须与外部可访问的地址匹配。 |

> **注意：** `server_name` 值 `"127.0.0.1:8128"` 仅用于本地开发。生产环境部署时，请替换为你的实际域名（如 `"chat.example.com"`）。更改 `server_name` 时，你还需要同步更新 `octos-registration.yaml`（正则表达式部分）和 `botfather.json`（`server_name` 字段）。

> **重要：** 在这个本地 Docker 示例里，Matrix 身份统一使用 `127.0.0.1:8128`。因此 `server_name`、应用服务正则和机器人用户 ID 都必须写成 `127.0.0.1:8128`。只有容器之间通信时才使用 `palpo:8008`、`octos:8009` 这类 Docker 服务名。

### 3.5 Octos 机器人配置（`config/botfather.json`）

此文件定义机器人的身份、LLM 提供商和 Matrix 通道配置。

```json
{
  "id": "botfather",
  "name": "BotFather",
  "enabled": true,
  "config": {
    "provider": "deepseek",
    "model": "deepseek-chat",
    "api_key_env": "DEEPSEEK_API_KEY",
    "channels": [
      {
        "type": "matrix",
        "homeserver": "http://palpo:8008",
        "as_token": "d1f46062a08e4833b18286d95c5e09a5f3e4a1b2c3d4e5f6a7b8c9d0e1f2a3b4",
        "hs_token": "e2a57173b19f5944c29397ea6d6f1ab6a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9",
        "server_name": "127.0.0.1:8128",
        "sender_localpart": "octosbot",
        "user_prefix": "octosbot_",
        "port": 8009,
        "allowed_senders": []
      }
    ],
    "gateway": {
      "max_history": 50,
      "queue_mode": "followup"
    }
  },
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z"
}
```

> **重要：** `created_at` 和 `updated_at` 字段是 Octos **必需的**。如果缺少这两个字段，Octos 会跳过该 profile，机器人将无法启动。

**LLM 提供商设置：**

| 字段 | 说明 |
|------|------|
| `provider` | LLM 提供商名称。Octos 支持 `deepseek`、`openai`、`anthropic` 等[多种提供商](https://octos-org.github.io/octos/)。 |
| `model` | 模型标识符（如 `deepseek-chat`、`gpt-4o`、`claude-sonnet-4-20250514`）。 |
| `api_key_env` | 存放 API Key 的环境变量名称。 |

**Matrix 通道设置：**

| 字段 | 说明 |
|------|------|
| `type` | 必须为 `"matrix"`。 |
| `homeserver` | Palpo 的内部 URL。使用 Docker 服务名 `palpo`，不是 `localhost`。 |
| `as_token` / `hs_token` | 必须与应用服务注册 YAML 文件匹配。 |
| `server_name` | Matrix 域名。必须与 `palpo.toml` 中的 `server_name` 一致。 |
| `sender_localpart` | 机器人用户名。必须与注册文件一致。 |
| `user_prefix` | 动态创建的子机器人用户 ID 前缀（如 `octosbot_translator`）。 |
| `port` | Octos 监听 Palpo 应用服务事件的端口。 |
| `allowed_senders` | 允许与机器人对话的 Matrix 用户 ID。空数组 `[]` = 所有人都可以对话。 |

> **重要：** `homeserver` 是 Octos 访问 Palpo 时使用的 Docker 内部 URL；`server_name` 是写进 Matrix 用户 ID 的域名部分。两者相关但不能混用。详见 [架构原理](02-how-robrix-palpo-octos-work-together-zh.md)。

**Gateway 设置：**

| 字段 | 说明 |
|------|------|
| `max_history` | 作为 LLM 上下文发送的最大历史消息数量。 |
| `queue_mode` | Octos 处理传入消息的方式。`followup` 将新消息排队并顺序处理。 |

**切换 LLM 提供商（以 OpenAI 替代 DeepSeek 为例）：**

1. 在 `botfather.json` 中修改：`"provider": "openai"`、`"model": "gpt-4o"`、`"api_key_env": "OPENAI_API_KEY"`
2. 在 `.env` 中修改：`OPENAI_API_KEY=sk-xxxxxxxx`
3. 在 `compose.yml` 的 `octos` 服务 `environment` 中添加：`OPENAI_API_KEY: ${OPENAI_API_KEY}`

Octos 支持 14+ 种提供商——完整列表见 [Octos Book](https://octos-org.github.io/octos/)。

### 3.6 Octos 全局设置（`config/octos.json`）

此文件配置 Octos 的核心运行路径和日志级别。

```json
{
  "profiles_dir": "/root/.octos/profiles",
  "data_dir": "/root/.octos",
  "log_level": "debug"
}
```

| 字段 | 说明 |
|------|------|
| `profiles_dir` | Octos 加载机器人配置文件（如 `botfather.json`）的目录。通过 Docker 卷映射自 `./config/`。 |
| `data_dir` | Octos 运行时数据（会话、记忆）的根目录。映射自 `./data/octos/`。 |
| `log_level` | Octos 日志详细程度。开发环境用 `debug`，生产环境用 `info`。 |

> **注意：** 这些是容器内部路径。`compose.yml` 中的 Docker 卷映射会将它们连接到宿主机目录。

### 3.7 Docker Compose（`compose.yml`）

提供的 `compose.yml` 启动三个服务：

| 服务 | 镜像 | 暴露端口 | 用途 |
|------|------|----------|------|
| `palpo_postgres` | `postgres:17` | *（无，仅内部）* | Palpo 的数据库 |
| `palpo` | 从源码构建 | `8128:8008` | Matrix 主服务器 |
| `octos` | 从源码构建 | `8009:8009`、`8010:8080` | AI 机器人应用服务 |

**端口映射说明：**

- `8128` — Robrix 连接此端口（Client-Server API）
- `8009` — Palpo 向 Octos 推送事件（Appservice API，同时暴露到宿主机供调试）
- `8010` — Octos 管理控制面板（可选，用于监控）

**持久化卷：**

| 卷 | 用途 |
|----|------|
| `./data/pgsql` | PostgreSQL 数据。`docker compose down` 后保留。 |
| `./data/octos` | Octos 运行时数据（会话、记忆）。 |
| `./data/media` | 通过 Matrix 上传的媒体文件（图片、文件）。 |

**环境变量（`.env`）：**

| 变量 | 必填 | 默认值 | 说明 |
|------|------|--------|------|
| `DEEPSEEK_API_KEY` | **是** | -- | 你的 LLM API Key |
| `DB_PASSWORD` | 否 | `palpo_dev_password` | PostgreSQL 密码 |
| `RUST_LOG` | 否 | `octos=debug,info` | 日志详细程度 |

### 3.8 令牌匹配检查清单

最常见的配置错误是令牌不匹配。以下值在两个文件中**必须完全一致**：

| 值 | 在 `octos-registration.yaml` 中 | 在 `botfather.json` 中 |
|----|----------------------------------|------------------------|
| `as_token` | `as_token: "d1f4..."` | `"as_token": "d1f4..."` |
| `hs_token` | `hs_token: "e2a5..."` | `"hs_token": "e2a5..."` |
| `sender_localpart` | `sender_localpart: octosbot` | `"sender_localpart": "octosbot"` |
| `server_name` | regex: `@octosbot:127\\.0\\.0\\.1:8128` | `"server_name": "127.0.0.1:8128"` |

如果有任何不匹配，机器人将不会响应消息。提交 bug 报告前请先检查！

---

## 4. 端到端验证

部署完成后，按照以下检查清单确认一切正常。

### 服务健康检查

```bash
# 检查所有容器是否运行
docker compose ps

# 检查 Palpo 日志是否有启动错误
docker compose logs palpo | tail -20

# 检查 Octos 日志——寻找 "appservice listening" 或类似信息
docker compose logs octos | tail -20

# 验证 Palpo 是否响应 Matrix API
curl -s http://127.0.0.1:8128/_matrix/client/versions | head -5
```

### 客户端连接检查清单

- [ ] Robrix 能连接到 `http://127.0.0.1:8128`
- [ ] 能注册新账号
- [ ] 登录后房间列表能加载（新账号可能为空）
- [ ] 能创建新房间

### 机器人交互检查清单

- [ ] 能邀请 `@octosbot:127.0.0.1:8128` 到房间
- [ ] 机器人加入房间（如果没有，检查 `docker compose logs octos`）
- [ ] 发送消息后机器人回复
- [ ] 回复内容合理（确认 LLM 连接正常）

### 日志检查顺序（跟随数据流）

如果某步失败，按照数据在系统中流动的顺序检查日志：

```bash
# 1. Palpo 是否收到了 Robrix 的消息？
docker compose logs palpo --since 1m

# 2. Palpo 是否将事件转发给了 Octos？
docker compose logs palpo --since 1m | grep -i appservice

# 3. Octos 是否收到并处理了事件？
docker compose logs octos --since 1m

# 4. Octos 是否成功调用了 LLM？
docker compose logs octos --since 1m | grep -i -E "deepseek|llm|provider"
```

---

## 5. 故障排除

### 5.1 服务启动问题

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| `palpo_postgres` 无法启动 | 端口 5432 已被占用，或数据损坏 | 检查 `docker compose logs palpo_postgres`。删除 `data/pgsql/` 重新开始。 |
| `palpo` 构建失败 | 网络问题或源码获取失败 | 确保 Docker 能访问 `github.com`。检查 `docker compose logs palpo` 查看构建错误。 |
| `palpo` 启动时崩溃 | `palpo.toml` 语法错误或数据库连接失败 | 检查日志。确保 `palpo_postgres` 先正常运行。验证数据库密码一致。 |
| `octos` 构建失败 | 缺少 Dockerfile 或网络问题 | 确保 Docker 能访问 `github.com`。运行 `./setup.sh` 确认仓库已克隆。 |
| `octos` 启动但日志有错误 | `botfather.json` 无效或缺少 API Key | 检查 JSON 语法。验证 `.env` 中已设置 `DEEPSEEK_API_KEY`。 |

### 5.2 Robrix 连接问题

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| "无法连接到服务器" | Homeserver URL 错误或 Palpo 未运行 | 确认 Palpo 正在运行（`docker compose ps`）。确认 URL 为 `http://127.0.0.1:8128`。 |
| 登录成功但没有房间 | 新账号的正常现象 | 创建一个新房间。加入或创建后房间会出现在列表中。 |
| 注册失败 | `palpo.toml` 中 `allow_registration = false` | 检查 `palpo.toml`。确保 `allow_registration = true`。 |
| "Homeserver 不支持 Sliding Sync" | Palpo 版本过旧 | 重新构建 Palpo：`docker compose build --no-cache palpo`。 |
| 连接超时 | 防火墙阻止了端口 8128 | 检查防火墙规则。macOS 上在系统设置中允许传入连接。 |

### 5.3 机器人问题

| 症状 | 原因 | 解决方法 |
|------|------|----------|
| 机器人不响应消息 | 注册文件和配置文件之间令牌不匹配 | 验证 [令牌匹配检查清单](#38-令牌匹配检查清单)。 |
| Palpo 日志中出现 `Connection refused` | Octos 未运行，或注册 YAML 中 `url` 错误 | 确保 Octos 正在运行。`url` 必须使用 Docker 服务名（`http://octos:8009`），不能用 `localhost`。 |
| `User ID not in namespace` | `sender_localpart` 与 `namespaces.users` 正则不匹配 | 更新 `octos-registration.yaml` 中的正则表达式，包含机器人的完整用户 ID 模式。 |
| 机器人加入房间但回复空消息 | LLM API Key 无效或额度不足 | 检查 `docker compose logs octos` 中的 API 错误。验证 API Key 和账户余额。 |
| 部分用户的消息被忽略 | `botfather.json` 中的 `allowed_senders` 过滤 | 设 `allowed_senders` 为 `[]` 允许所有人，或添加用户的 Matrix ID。 |
| 机器人配置未加载 | `botfather.json` 缺少 `created_at` / `updated_at` | 这两个字段是必需的。按 [3.5 节](#35-octos-机器人配置configbotfatherjson) 示例添加。 |

### 5.4 常用调试命令

```bash
# 实时查看所有服务日志
docker compose logs -f

# 查看特定服务的日志
docker compose logs -f palpo
docker compose logs -f octos

# 重启单个服务（如修改 botfather.json 后）
docker compose restart octos

# 重新构建单个服务（如更新源码后）
docker compose build --no-cache palpo
docker compose up -d palpo

# 检查 Palpo 的 Client-Server API
curl http://127.0.0.1:8128/_matrix/client/versions

# 完全重置（警告：删除所有数据，包括账号和消息）
docker compose down -v
rm -rf data/
docker compose up -d
```

---

## 6. 延伸阅读

- **Octos 完整文档：** [octos-org.github.io/octos](https://octos-org.github.io/octos/) — 覆盖所有 LLM 提供商、通道、技能、记忆系统和高级配置。
- **Octos Matrix Appservice 指南：** [octos-org/octos#171](https://github.com/octos-org/octos/pull/171) — 本文档参考的原始 Palpo + Octos 集成指南。
- **Palpo：** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) — Palpo 主服务器文档。
- **Robrix：** [Project-Robius-China/robrix2](https://github.com/Project-Robius-China/robrix2) — Robrix 客户端、构建说明和功能追踪。
- **Matrix Appservice 规范：** [spec.matrix.org — Application Service API](https://spec.matrix.org/latest/application-service-api/) — 应用服务的 Matrix 协议规范。
- **架构原理：** [02-how-robrix-palpo-octos-work-together-zh.md](02-how-robrix-palpo-octos-work-together-zh.md) — 应用服务机制如何运作、消息生命周期和 BotFather 系统。

---

*本指南内容截至 2026 年 4 月。最新更新请查看各项目的仓库。*
