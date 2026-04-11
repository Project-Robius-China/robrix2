# 部署指南：OpenClaw + Matrix

[English](01-deploying-openclaw-with-matrix.md)

> **目标：** 完成本指南后，你将拥有一个连接到 Matrix 服务器的 OpenClaw AI 代理。之后你可以使用 Robrix（或任何 Matrix 客户端）与 OpenClaw 驱动的 AI 代理对话。

## 什么是 OpenClaw？

[OpenClaw](https://github.com/openclaw/openclaw) 是一个开源的自托管 AI 助手平台（前身为 MoltBot，2026 年初更名）。它支持多种 LLM（DeepSeek、OpenAI、Anthropic 等），并通过频道插件接入 Matrix、Telegram、Discord 等聊天平台。在本指南中，OpenClaw 通过 **Matrix 频道插件**以**普通用户身份**登录服务器，不需要任何服务器端配置。与 Octos 的 Application Service 模式不同，OpenClaw 的接入方式更简单——详见[架构原理](03-how-robrix-and-openclaw-work-together-zh.md)。

本指南将逐步引导你完成 OpenClaw 与 Matrix 的部署：从创建 Matrix Bot 账号，到配置 OpenClaw Matrix 频道插件，再到端到端验证连接。

> **想快速体验？** 跳转到 [快速开始](#2-快速开始)。
>
> **想了解 OpenClaw 如何连接 Matrix？** 参见 [架构原理](03-how-robrix-and-openclaw-work-together-zh.md)。

> **关于 OpenClaw：** OpenClaw 目前仍在快速迭代中，CLI 和插件系统存在不少 bug（例如 `channels add` 向导可能崩溃）。本指南给出的是我们**实测验证过**的配置方式——直接编辑配置文件，跳过不稳定的 CLI 向导。如果你遇到本指南未覆盖的问题，请查阅 [OpenClaw 官方文档](https://docs.openclaw.ai/) 和 [GitHub Issues](https://github.com/openclaw/openclaw/issues)。

---

## 目录

1. [前置条件](#1-前置条件)
2. [快速开始](#2-快速开始)
3. [创建 Matrix Bot 账号](#3-创建-matrix-bot-账号)
4. [安装 OpenClaw 并初始化配置目录](#4-安装-openclaw-并初始化配置目录)
5. [编写配置文件](#5-编写配置文件)
6. [启动并验证](#6-启动并验证)
7. [故障排查](#7-故障排查)
8. [生产环境配置](#8-生产环境配置)
9. [延伸阅读](#9-延伸阅读)

---

## 1. 前置条件

| 条件                       | 说明                                                                                                                            |
| -------------------------- | ------------------------------------------------------------------------------------------------------------------------------- |
| **两个 Matrix 账号** | 一个作为你自己使用的账号，另一个作为 OpenClaw Bot 使用的账号                                                                    |
| **Node.js**          | v22.16+ 或 v24+（推荐）                                                                                                         |
| **LLM API Key**      | 例如[DeepSeek](https://platform.deepseek.com/)（有免费额度）、OpenAI、Anthropic 等                                                 |
| **Matrix 服务器**    | 本地 Palpo（推荐，参见[Palpo 部署指南](../robrix-with-palpo-and-octos/01-deploying-palpo-and-octos-zh.md)）或公共服务器 matrix.org |
| **Robrix**           | 参见[Robrix 快速开始](../robrix/getting-started-with-robrix-zh.md)                                                                 |

---

## 2. 快速开始

```
1. 注册一个 Matrix Bot 账号（记住用户名和密码）
2. 安装 OpenClaw → 运行 openclaw config → 编辑 ~/.openclaw/openclaw.json
3. 运行 openclaw gateway start
4. 在 Robrix 中用另一个账号给 Bot 发消息
```

详细步骤见下文。

---

## 3. 创建 Matrix Bot 账号

Bot 账号就是一个**普通的 Matrix 账号**。OpenClaw 会用它的用户名和密码自动登录，不需要你手动获取 Access Token。

| 服务器                 | 注册方式                                              | 说明                                           |
| ---------------------- | ----------------------------------------------------- | ---------------------------------------------- |
| **本地 Palpo**   | 在 Robrix 中注册                                      | 连接 `http://127.0.0.1:8128`，注册一个新账号 |
| **matrix.org**   | 在 Robrix 或[Element Web](https://app.element.io) 中注册 | 公共服务器，免费，注册即用                     |
| **自建 Synapse** | 通过 Admin API 或 Web 注册                            | 生产环境推荐                                   |

注册时记住：

- **用户名**（例如 `chalice`）
- **密码**

---

## 4. 安装 OpenClaw 并初始化配置目录

### 4.1 安装

```bash
npm install -g openclaw@latest
openclaw --version    # 验证安装
```

### 4.2 初始化配置目录

```bash
openclaw config
```

> **这个命令会报错——这是正常的，忽略即可。** 重要的是它已经在 `~/.openclaw/` 下创建了配置目录。后续所有配置都在这个目录中进行。

> **为什么不用 `openclaw channels add` 向导？** OpenClaw v2026.4.7 的 CLI 向导存在多个 bug（Telegram 插件路径错误导致向导崩溃、参数不完整等）。**直接编辑配置文件是唯一可靠的方式。**

---

## 5. 编写配置文件

编辑 `~/.openclaw/openclaw.json`。下面提供两种场景的完整配置。

### 5.1 连接本地 Palpo（推荐）

```json
{
  "commands": {
    "native": "auto",
    "nativeSkills": "auto"
  },
  "models": {
    "providers": {
      "deepseek": {
        "baseUrl": "https://api.deepseek.com/v1",
        "apiKey": "sk-你的DeepSeek密钥",
        "api": "openai-completions",
        "models": [
          {
            "id": "deepseek-chat",
            "name": "DeepSeek Chat",
            "contextWindow": 164000,
            "maxTokens": 8192
          }
        ]
      }
    }
  },
  "channels": {
    "matrix": {
      "enabled": true,
      "homeserver": "http://127.0.0.1:8128",
      "network": {
        "dangerouslyAllowPrivateNetwork": true
      },
      "userId": "@chalice:127.0.0.1:8128",
      "password": "你的密码",
      "deviceName": "OpenClaw Bot",
      "encryption": true,
      "autoJoin": "always",
      "dm": {
        "policy": "open"
      }
    }
  },
  "plugins": {
    "entries": {
      "matrix": {
        "enabled": true
      }
    }
  },
  "gateway": {
    "mode": "local"
  }
}
```

### 5.2 连接公共服务器 matrix.org

```json
{
  "commands": {
    "native": "auto",
    "nativeSkills": "auto"
  },
  "models": {
    "providers": {
      "deepseek": {
        "baseUrl": "https://api.deepseek.com/v1",
        "apiKey": "sk-你的DeepSeek密钥",
        "api": "openai-completions",
        "models": [
          {
            "id": "deepseek-chat",
            "name": "DeepSeek Chat",
            "contextWindow": 164000,
            "maxTokens": 8192
          }
        ]
      }
    }
  },
  "channels": {
    "matrix": {
      "enabled": true,
      "homeserver": "https://matrix.org",
      "userId": "@your-bot:matrix.org",
      "password": "你的密码",
      "deviceName": "OpenClaw Bot",
      "encryption": true,
      "autoJoin": "always",
      "dm": {
        "policy": "open"
      }
    }
  },
  "plugins": {
    "entries": {
      "matrix": {
        "enabled": true
      }
    }
  },
  "gateway": {
    "mode": "local"
  }
}
```

### 5.3 配置项详解

#### `gateway` 配置

| 字段     | 值          | 重点说明                                                                                                                                                                                                       |
| -------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mode` | `"local"` | **必填。** 没有这个字段 gateway 会拒绝启动，报 "missing gateway.mode" 错误。`"local"` 指的是 OpenClaw gateway 本身运行在本地（只监听 127.0.0.1），与 LLM 是否远程无关——DeepSeek API 调用仍然走公网。 |

#### `models.providers` 配置

| 字段              | 值                                | 重点说明                                                                                                                                                           |
| ----------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `baseUrl`       | `"https://api.deepseek.com/v1"` | **必须带 `/v1` 后缀。** DeepSeek 使用 OpenAI 兼容 API。                                                                                                    |
| `apiKey`        | `"sk-xxx"`                      | **直接写明文密钥。** 不要用 `${ENV_VAR}` 格式——macOS LaunchAgent 服务读不到终端的环境变量。写完后 `chmod 600 ~/.openclaw/openclaw.json` 保护文件权限。 |
| `api`           | `"openai-completions"`          | **不是 `type`。** 网上很多教程写 `"type"` 是错的，正确字段名是 `"api"`。                                                                               |
| `contextWindow` | `164000`                        | **必须设大。** OpenClaw 系统提示词占 16K+ token，默认 4096 会直接报错。DeepSeek Chat 支持 164K。                                                             |
| `maxTokens`     | `8192`                          | 单次回复最大 token 数。                                                                                                                                            |

> **注意 `providers` 的格式：** `providers` 是一个对象（provider 名称作为 key），不是数组。`models` 是数组。

#### `channels.matrix` 配置

| 字段                                       | 值                            | 重点说明                                                                                                                                                                                                                       |
| ------------------------------------------ | ----------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `enabled`                                | `true`                      | 启用 Matrix 频道。                                                                                                                                                                                                             |
| `homeserver`                             | `"http://127.0.0.1:8128"`   | **本地 Palpo 必须用 `http`**，不是 `https`（Palpo 默认没有 TLS）。matrix.org 用 `https`。                                                                                                                          |
| `network.dangerouslyAllowPrivateNetwork` | `true`                      | **仅本地/内网部署需要。** OpenClaw 默认阻止连接私有 IP（127.0.0.1、10.x、192.168.x），这是防 SSRF 的安全措施。连公共服务器（matrix.org）不需要此项。                                                                     |
| `userId`                                 | `"@chalice:127.0.0.1:8128"` | **必须是完整 Matrix ID 格式** `@用户名:服务器`。                                                                                                                                                                       |
| `password`                               | `"你的密码"`                | 密码认证——OpenClaw 自动登录并缓存 token 到 `~/.openclaw/credentials/matrix/`。也支持 Access Token 认证（将 `password` 替换为 `accessToken`），详见 [OpenClaw Matrix 插件文档](https://docs.openclaw.ai/channels/matrix)。 |
| `encryption`                             | `true`                      | **强烈建议开启。** Matrix DM 默认启用 E2EE。如果不开，Bot 收到加密消息无法解密，表现为"发了消息但没回复"。                                                                                                               |
| `autoJoin`                               | `"always"`                  | 测试阶段接受所有邀请。生产环境改为 `"allowlist"`。                                                                                                                                                                           |
| `dm.policy`                              | `"open"`                    | 测试阶段允许所有私聊。生产环境改为 `"allowlist"`。                                                                                                                                                                           |

#### `plugins` 配置

| 字段                               | 值       | 重点说明                 |
| ---------------------------------- | -------- | ------------------------ |
| `plugins.entries.matrix.enabled` | `true` | 确保 Matrix 插件已启用。 |

### 5.4 本地 Palpo vs 公共 matrix.org 的差异

| 配置项                                     | 本地 Palpo                 | 公共 matrix.org                             |
| ------------------------------------------ | -------------------------- | ------------------------------------------- |
| `homeserver`                             | `http://127.0.0.1:8128`  | `https://matrix.org`                      |
| `network.dangerouslyAllowPrivateNetwork` | **需要** `true`    | **不需要**（删除整个 `network` 块） |
| `userId` 格式                            | `@用户名:127.0.0.1:8128` | `@用户名:matrix.org`                      |
| TLS                                        | 无（`http`）             | 有（`https`）                             |
| 注册方式                                   | Robrix 连接 Palpo 注册     | Element Web 或 Robrix 注册                  |

> **从本地 Palpo 切换到 matrix.org：** 本指南以 Palpo 为例，但同样的配置可以直接用于 matrix.org 或任何标准 Matrix 服务器。只需修改 `openclaw.json` 中的 3 处：
>
> 1. `homeserver`：`http://127.0.0.1:8128` → `https://matrix.org`
> 2. `userId`：`@用户名:127.0.0.1:8128` → `@用户名:matrix.org`
> 3. 删除整个 `"network": { "dangerouslyAllowPrivateNetwork": true }` 块（公网服务器不需要）
>
> 其他配置（LLM、加密、autoJoin 等）**完全不变**。改完后 `openclaw gateway restart` 即可。
>
> 如果你使用其他自建 Matrix 服务器（如 Synapse、Dendrite），同样只需要修改这 3 处，将域名和协议替换为你的服务器地址即可。

---

## 6. 启动并验证

### 6.1 启动 Gateway

```bash
openclaw gateway start
```

### 6.2 检查日志

```bash
tail -20 ~/.openclaw/logs/gateway.log
```

确认看到以下关键日志：

```
[gateway] agent model: deepseek/deepseek-chat          ← LLM 配置正确
[gateway] ready (6 plugins, 0.3s)                       ← Gateway 就绪
[matrix] [default] starting provider (http://...)       ← Matrix 开始连接
matrix: logged in as @chalice:127.0.0.1:8128           ← 登录成功
matrix: device is verified by its owner and ready for encrypted rooms  ← 加密就绪
```

> **提示：** OpenClaw 启动后需要一些时间完成 Matrix 登录和加密设备初始化。如果日志中只看到 `starting provider` 而没有 `logged in`，请耐心等待几秒到十几秒，登录完成后才能接收消息。

### 6.3 在 Robrix 中测试

1. **启动 Robrix**，用你的**个人账号**登录
2. **搜索 Bot**：点搜索图标，输入 Bot 的 Matrix ID（如 `@chalice:127.0.0.1:8128`），切到 **People** 标签（Bot 是普通用户，必须在 People 中搜索）
3. **发起私聊**：选择 Bot，进入对话
4. **发送消息**，耐心等待回复（LLM 生成回复需要几秒到十几秒）

<img src="../images/openclaw-bot-reply.png" width="600" alt="OpenClaw Bot（chalice）在 Robrix 中成功回复消息">

> **重要：** 如果你在 OpenClaw 加密设备创建之前发送过消息，那些历史消息**永远无法解密**（这是 Matrix E2EE 的正常行为）。必须发送**新消息**才能触发回复。

---

## 7. 故障排查

| 现象                                                          | 原因                                                  | 解决方案                                                                                                     |
| ------------------------------------------------------------- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `channels add` 向导崩溃报 ENOENT                            | v2026.4.7 Telegram 插件路径 bug                       | 跳过向导，直接编辑 `~/.openclaw/openclaw.json`                                                             |
| Gateway 拒绝启动："missing gateway.mode"                      | 配置文件缺少 `gateway` 配置节                       | 添加 `"gateway": {"mode": "local"}`                                                                        |
| "Blocked hostname or private/internal/special-use IP address" | OpenClaw 默认阻止连接私有 IP                          | 添加 `"network": {"dangerouslyAllowPrivateNetwork": true}`                                                 |
| Matrix 连接失败，反复重试                                     | `homeserver` 使用了 `https` 但本地 Palpo 没有 TLS | 改为 `http://127.0.0.1:8128`                                                                               |
| 启动报 "Invalid input: expected record, received array"       | `providers` 格式写成了数组                          | `providers` 是对象（key-value），不是数组                                                                  |
| 启动报 "Unrecognized key: type"                               | 字段名写错                                            | 用 `"api"` 而不是 `"type"`                                                                               |
| "missing env var DEEPSEEK_API_KEY"                            | 环境变量对 LaunchAgent 不可见                         | API key 直接写进配置文件                                                                                     |
| 消息发出但 Bot 不回复（无错误）                               | DM 默认加密，但 OpenClaw 没开                         | 添加 `"encryption": true`                                                                                  |
| "encrypted event received without encryption enabled"         | 同上                                                  | 添加 `"encryption": true`                                                                                  |
| "This message was sent before this device logged in"          | 历史消息无法解密                                      | 正常现象。发送**新消息**即可                                                                           |
| Bot 回复为空或报错                                            | LLM API Key 无效或余额不足                            | 检查 DeepSeek API Key 和账户余额                                                                             |
| Robrix 搜索不到 Bot                                           | Bot 账号未注册成功                                    | 确认 Bot 账号存在（在 Element Web 中验证）                                                                   |
| 其他 OpenClaw 问题                                            | —                                                    | 查阅[OpenClaw 官方文档](https://docs.openclaw.ai/) 和 [GitHub Issues](https://github.com/openclaw/openclaw/issues) |

> **重要说明：** 本指南仅覆盖我们实测验证过的配置流程（OpenClaw v2026.4.7）。OpenClaw 本身仍在快速迭代中，其 CLI、插件系统、Gateway 行为可能在后续版本中发生变化。如果你遇到本指南中未列出的 OpenClaw 问题（如 CLI 报错、插件加载失败、Gateway 行为异常等），这些属于 OpenClaw 自身的问题，请参考以下资源：
>
> - [OpenClaw 官方文档](https://docs.openclaw.ai/) — 最新配置参考
> - [OpenClaw Matrix 频道插件文档](https://docs.openclaw.ai/channels/matrix) — Matrix 插件专项
> - [OpenClaw GitHub Issues](https://github.com/openclaw/openclaw/issues) — 已知问题和社区讨论
>
> Robrix 作为标准 Matrix 客户端，与 OpenClaw 之间通过 Matrix 协议通信，两者完全解耦。Robrix 侧无需任何特殊配置。

---

## 8. 生产环境配置

测试通过后，收紧权限。修改 `channels.matrix` 中的以下字段：

```json
{
  "autoJoin": "allowlist",
  "autoJoinAllowlist": ["!room-id:your-server"],
  "dm": {
    "policy": "allowlist",
    "allowFrom": ["@admin:your-server"],
    "sessionScope": "per-room"
  },
  "groupPolicy": "allowlist",
  "groupAllowFrom": ["@admin:your-server"],
  "groups": {
    "!room-id:your-server": {
      "requireMention": true
    }
  }
}
```

| 字段               | 测试值       | 生产值          | 说明                     |
| ------------------ | ------------ | --------------- | ------------------------ |
| `autoJoin`       | `"always"` | `"allowlist"` | 只加入白名单中的房间     |
| `dm.policy`      | `"open"`   | `"allowlist"` | 只接受白名单用户的私聊   |
| `groupPolicy`    | —           | `"allowlist"` | 群聊中限制谁可以触发 Bot |
| `requireMention` | —           | `true`        | 群聊中必须 @Bot 才响应   |

---

## 9. 延伸阅读

- **OpenClaw 文档：** [docs.openclaw.ai](https://docs.openclaw.ai/) — OpenClaw 完整文档。
- **OpenClaw Matrix 插件：** [docs.openclaw.ai/channels/matrix](https://docs.openclaw.ai/channels/matrix) — 官方 Matrix 频道插件参考。
- **OpenClaw GitHub：** [github.com/openclaw/openclaw](https://github.com/openclaw/openclaw) — 源码、Issues 和最新发布。
- **Palpo 部署指南：** [01-deploying-palpo-and-octos-zh.md](../robrix-with-palpo-and-octos/01-deploying-palpo-and-octos-zh.md) — 如何部署本地 Palpo 服务器。
- **架构原理：** [03-how-robrix-and-openclaw-work-together-zh.md](03-how-robrix-and-openclaw-work-together-zh.md) — OpenClaw 如何连接 Matrix，以及与 Octos AppService 模式的对比。
- **使用指南：** [02-using-robrix-with-openclaw-zh.md](02-using-robrix-with-openclaw-zh.md) — 如何使用 Robrix 与 OpenClaw 代理对话。

---

*本指南基于 2026 年 4 月的实测结果编写（OpenClaw v2026.4.7 + Palpo）。OpenClaw 正在快速迭代中，如遇到问题请以 [官方文档](https://docs.openclaw.ai/) 为准。*
