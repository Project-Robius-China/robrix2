# 联邦功能：跨服务器通信

[English Version](04-federation-with-palpo.md)

> **目标：** 按照本指南操作后，你的 Palpo 将配置好 Matrix 联邦功能，你服务器上的用户可以与其他 Matrix 服务器（如 matrix.org）上的用户通信，远程用户也可以访问你的 Octos AI 机器人。

本指南介绍 Matrix **联邦**（Federation）功能 -- 将你的 Palpo 服务器与其他 Matrix 服务器连接，使不同服务器上的用户能够互相通信。

> **前提条件：** 你应该已经完成了本地部署。如果还没有，请先参阅 [01-deploying-palpo-and-octos-zh.md](01-deploying-palpo-and-octos-zh.md)。

---

## 目录

1. [什么是 Matrix 联邦？](#1-什么是-matrix-联邦)
2. [联邦的前提条件](#2-联邦的前提条件)
3. [Palpo 联邦配置](#3-palpo-联邦配置)
4. [生产环境部署](#4-生产环境部署)
5. [使用联邦功能](#5-使用联邦功能)
6. [验证与故障排除](#6-验证与故障排除)
7. [延伸阅读](#7-延伸阅读)

---

## 1. 什么是 Matrix 联邦？

Matrix 是一个**去中心化**的通信协议。每个组织都可以运行自己的服务器，联邦功能允许不同服务器上的用户无缝通信。

可以类比电子邮件：

- `@alice:server-a.com` 可以与 `@bob:server-b.com` 聊天
- 每个服务器存储自己用户的数据
- 消息在参与对话的所有服务器之间同步复制
- 没有单点控制 -- 如果一台服务器宕机，其他服务器继续正常运行

在本地部署指南中，所有服务运行在 `127.0.0.1:8128` 上 -- 这是一个完全隔离的服务器。联邦功能将你的服务器接入更广阔的 Matrix 网络。

```
  服务器 A                         服务器 B
┌──────────┐   Federation API    ┌──────────┐
│  Palpo   │ ◄────────────────►  │  Synapse  │
│  + Octos │   (端口 8448)        │  或其他   │
│  + Robrix│                     │  Matrix   │
└──────────┘                     └──────────┘
  @alice:server-a.com              @bob:server-b.com
       └─── 可以互相聊天 ───────────────┘
```

---

## 2. 联邦的前提条件

与本地部署不同，联邦功能有额外的基础设施要求：

| 需求 | 本地部署 | 联邦部署 |
|------|---------|---------|
| 域名 | 不需要（`127.0.0.1`） | 必需（如 `matrix.example.com`） |
| TLS 证书 | 不需要（HTTP） | 必需（HTTPS，推荐 Let's Encrypt） |
| 端口 443 | 不需要 | 开放（Client-Server API） |
| 端口 8448 | 不需要 | 开放（Server-Server 联邦 API） |
| 反向代理 | 不需要 | 推荐（Caddy 或 Nginx） |
| DNS 记录 | 不需要 | A 记录必需，SRV 记录可选 |

> **自签名证书不能用于联邦。** 其他 Matrix 服务器会拒绝连接。请使用 [Let's Encrypt](https://letsencrypt.org/) 获取免费的受信证书。

---

## 3. Palpo 联邦配置

### 3.1 基本设置（`palpo.toml`）

以下是支持联邦的 `palpo.toml` 配置。与本地部署的区别用注释标出：

```toml
# 修改：使用真实域名代替 127.0.0.1:8128
server_name = "matrix.example.com"

# 修改：生产环境关闭开放注册
# 先创建账号，再设为 false
allow_registration = false

enable_admin_room = true
appservice_registration_dir = "/var/palpo/appservices"

[[listeners]]
address = "0.0.0.0:8008"

[logger]
format = "json"    # 修改：生产环境使用 "json" 格式

[db]
url = "postgres://palpo:你的强密码@palpo_postgres:5432/palpo"
pool_size = 10

# 修改：使用真实域名进行服务发现
[well_known]
server = "matrix.example.com:443"
client = "https://matrix.example.com"

# --- 联邦设置（新增）---
[federation]
enable = true
allow_inbound_profile_lookup = true

# 可选：限制仅与特定服务器联邦
# allowed_servers = ["matrix.org", "*.trusted.com"]
# denied_servers = ["evil.com"]

[tls]
enable = true
cert = "/path/to/fullchain.pem"
key = "/path/to/privkey.pem"

[presence]
allow_local = true
allow_incoming = true
allow_outgoing = true

[typing]
allow_incoming = true
allow_outgoing = true
federation_timeout = 30000

trusted_servers = ["matrix.org"]
```

### 3.2 联邦设置参考

`[federation]` 部分控制你的服务器如何与其他 Matrix 服务器交互：

| 字段 | 类型 | 默认值 | 说明 |
|------|------|--------|------|
| `enable` | bool | `true` | 联邦总开关。设为 `false` 则运行完全隔离的服务器。 |
| `allow_loopback` | bool | `false` | 允许向自身发送联邦请求。仅用于开发。 |
| `allow_device_name` | bool | `false` | 向联邦用户暴露设备显示名称。出于隐私考虑，建议禁用。 |
| `allow_inbound_profile_lookup` | bool | `true` | 允许远程服务器查询本地用户资料。禁用后联邦用户将看不到显示名称。 |
| `allowed_servers` | list | 无 | 允许列表：仅这些服务器可以与你联邦。支持通配符（如 `*.trusted.com`）。未设置时允许所有服务器。 |
| `denied_servers` | list | `[]` | 拒绝列表：阻止特定服务器。**优先级高于** `allowed_servers`。支持通配符。 |

### 3.3 服务发现（`[well_known]`）

`[well_known]` 部分对联邦**至关重要**。它告诉其他服务器如何找到你的服务器。

Palpo 自动提供以下端点：

| 端点 | 响应内容 | 使用者 |
|------|---------|--------|
| `/.well-known/matrix/server` | `{"m.server": "matrix.example.com:443"}` | 其他服务器（联邦） |
| `/.well-known/matrix/client` | `{"m.homeserver": {"base_url": "https://matrix.example.com"}}` | Matrix 客户端（Robrix、Element） |

如果使用反向代理，需确保这些端点被正确转发到 Palpo。参见[第 4.2 节](#42-反向代理caddy-示例)的代理配置。

### 3.4 TLS 配置

```toml
[tls]
enable = true
cert = "/path/to/fullchain.pem"
key = "/path/to/privkey.pem"
dual_protocol = false    # 生产环境不要同时允许 HTTP 和 HTTPS
```

如果使用反向代理终止 TLS（推荐方式），可以在 Palpo 中禁用 `[tls]`，让代理处理证书。参见[第 4.2 节](#42-反向代理caddy-示例)。

### 3.5 在线状态与输入提示（联邦功能）

这些设置控制跨联邦服务器的实时状态指示器：

```toml
[presence]
allow_local = true       # 本地在线状态（仅你的服务器）
allow_incoming = true    # 接收远程服务器的在线状态更新
allow_outgoing = true    # 发送在线状态更新到远程服务器

[typing]
allow_incoming = true    # 接收远程用户的输入提示
allow_outgoing = true    # 发送输入提示到远程用户
federation_timeout = 30000   # 毫秒
```

> **注意：** `[presence]` 下的 `allow_outgoing` 需要 `allow_local` 为 `true` 才能生效。

### 3.6 受信服务器

```toml
trusted_servers = ["matrix.org"]
```

受信服务器充当**公证服务器** -- 帮助验证其他服务器的签名密钥。这是 [Perspectives 密钥验证](https://spec.matrix.org/latest/server-server-api/#querying-keys-through-another-server)机制的一部分。`matrix.org` 是最常用的选择。

### 3.7 DNS 配置

这些是 `palpo.toml` 中的顶层设置（不在任何 `[section]` 内）：

```toml
# 将这些添加到 palpo.toml 的顶层（与 server_name 等同级）
query_over_tcp_only = true       # 使用 TCP 进行 DNS 查询（在容器中更可靠）
query_all_nameservers = true     # 查询所有配置的域名服务器
ip_lookup_strategy = 5           # 5 = 先查 IPv4，再查 IPv6
```

> **提示：** 如果在 Docker 中运行，建议设置 `query_over_tcp_only = true` 以避免容器网络中的 UDP DNS 解析问题。

---

## 4. 生产环境部署

本节介绍从本地部署升级到联邦部署所需的基础设施变更。

### 4.1 域名和 DNS 配置

1. **注册域名**（如 `example.com`）

2. **创建 DNS A 记录**，指向你的服务器：
   ```
   matrix.example.com.  IN  A  203.0.113.10
   ```

3. **可选：创建 SRV 记录**（用于非标准端口的联邦）：
   ```
   _matrix-fed._tcp.example.com.  IN  SRV  10 0 8448 matrix.example.com.
   ```
   > 如果在 443 端口提供联邦服务且 `/.well-known/matrix/server` 响应正确，则不需要 SRV 记录。

### 4.2 反向代理（Caddy 示例）

生产环境推荐使用反向代理。Caddy 通过 Let's Encrypt 自动管理 TLS 证书。

```
matrix.example.com {
    # Well-known 端点（联邦发现）
    handle /.well-known/matrix/server {
        respond `{"m.server":"matrix.example.com:443"}`
    }

    handle /.well-known/matrix/client {
        header Access-Control-Allow-Origin "*"
        respond `{"m.homeserver":{"base_url":"https://matrix.example.com"}}`
    }

    # 其他请求代理到 Palpo
    reverse_proxy localhost:8008
}
```

使用 Caddy 处理 TLS 时，可以在 `palpo.toml` 中禁用 TLS，让 Palpo 在内部使用纯 HTTP：

```toml
[tls]
enable = false    # Caddy 终止 TLS
```

> **Nginx 替代方案：** 如果使用 Nginx，需要单独管理 Let's Encrypt 证书（如使用 certbot），并配置 `ssl_certificate` / `ssl_certificate_key` 指令。

### 4.3 更新 Docker Compose

与本地 `compose.yml` 相比的关键变更：

```yaml
services:
  palpo:
    # ...（构建部分与本地相同）...
    ports:
      - "8008:8008"    # 修改：Caddy 代理到此端口
      # 不再直接暴露 8128
    volumes:
      - ./palpo.toml:/var/palpo/palpo.toml:ro
      - ./appservices:/var/palpo/appservices:ro
      - ./data/media:/var/palpo/media
      # 新增：挂载 TLS 证书（仅当 Palpo 直接处理 TLS 时）
      # - /etc/letsencrypt/live/matrix.example.com:/certs:ro
    # ... 其余与本地相同 ...
```

整体 Docker Compose 结构与本地部署保持一致 -- PostgreSQL、Palpo、Octos。主要区别是：

- `palpo.toml` 中的 `server_name` 使用真实域名
- 端口映射变更（Caddy 在 443 代理到 Palpo 的 8008）
- TLS 由 Caddy 处理（或在 Palpo 直接处理 TLS 时挂载证书）
- `allow_registration = false`（先创建账号，然后锁定注册）

### 4.4 更新 Appservice 注册

从 `127.0.0.1:8128` 切换到真实域名时，需要更新以下文件：

**`appservices/octos-registration.yaml`** -- 更新正则表达式：

```yaml
namespaces:
  users:
    - exclusive: true
      regex: "@octosbot_.*:matrix\\.example\\.com"    # 已修改
    - exclusive: true
      regex: "@octosbot:matrix\\.example\\.com"       # 已修改
```

**`config/botfather.json`** -- 更新 `server_name`：

```json
{
  "config": {
    "channels": [{
      "type": "matrix",
      "homeserver": "http://palpo:8008",
      "server_name": "matrix.example.com",
      "sender_localpart": "octosbot",
      ...
    }]
  }
}
```

> **重要：** `botfather.json` 中的 `homeserver` URL 保持为 Docker 内部地址（`http://palpo:8008`）。只有 `server_name` 需要改为真实域名。

---

## 5. 使用联邦功能

联邦配置完成后，你可以与其他 Matrix 服务器上的用户和房间进行交互。

### 5.1 加入其他服务器上的房间

在 Robrix 中：

1. 点击 **Join Room**（加入房间）
2. 输入其他服务器上的房间别名，例如 `#general:matrix.org`
3. 你的服务器通过联邦与 `matrix.org` 连接并加入该房间
4. 来自所有参与服务器的用户消息实时显示

<!-- screenshot: federated-room.png -- Robrix 显示来自其他服务器的房间 -->

### 5.2 邀请其他服务器上的用户

1. 打开你服务器上的一个房间
2. 点击 **Invite**（邀请）
3. 输入其他服务器上的用户 ID：`@friend:other-server.com`
4. 邀请通过联邦传送到远程服务器
5. 远程用户接受邀请后加入你的房间

### 5.3 跨服务器 AI 机器人

启用联邦后，**其他服务器**上的用户也可以与你的 Octos 机器人交互：

1. `matrix.org` 上的用户邀请 `@octosbot:matrix.example.com` 到他们的房间
2. 邀请通过联邦传送到你的服务器
3. Octos 接受邀请并加入房间
4. 机器人响应消息 -- 即使房间在不同的服务器上

> **注意：** 要使此功能正常工作，`botfather.json` 中的 `allowed_senders` 必须为空数组 `[]`（允许所有用户）或明确包含远程用户的 Matrix ID（如 `@remoteuser:matrix.org`）。

---

## 6. 验证与故障排除

### 6.1 测试联邦

**检查 well-known 端点是否可访问：**

```bash
# 服务器发现（其他服务器使用）
curl https://matrix.example.com/.well-known/matrix/server

# 客户端发现（Robrix/Element 使用）
curl https://matrix.example.com/.well-known/matrix/client
```

**使用 Matrix Federation Tester：**

访问 [https://federationtester.matrix.org](https://federationtester.matrix.org) 并输入你的域名（如 `matrix.example.com`）。它会检查：

- DNS 解析
- TLS 证书有效性
- Well-known 端点响应
- Server-Server API 可达性
- 签名密钥验证

### 6.2 常见问题

| 症状 | 原因 | 解决方法 |
|------|------|---------|
| 无法加入其他服务器的房间 | 联邦未启用或端口被阻止 | 检查 `palpo.toml` 中 `[federation] enable = true`。确保防火墙开放端口 443 和 8448。 |
| "Unable to find signing key" | TLS 或 DNS 配置错误 | 验证 TLS 证书有效（非自签名）。检查 DNS 解析是否正确。运行 Federation Tester。 |
| Well-known 返回 404 | 反向代理未转发 | 检查 Caddy/Nginx 配置是否将 `/.well-known/matrix/*` 转发到 Palpo（或直接响应）。 |
| 远程用户看不到资料 | 资料查询被禁用 | 在 `[federation]` 中设置 `allow_inbound_profile_lookup = true`。 |
| 连接远程服务器超时 | 防火墙或 DNS 问题 | 检查出站连接。尝试设置 `query_over_tcp_only = true`。验证服务器能否通过 8448 端口访问其他 Matrix 服务器。 |
| 机器人不响应联邦用户 | `allowed_senders` 过滤 | 在 `botfather.json` 中将 `allowed_senders` 设为 `[]` 以允许所有用户，或添加远程用户的完整 Matrix ID。 |

### 6.3 调试命令

```bash
# 查看 Palpo 联邦相关日志
docker compose logs palpo | grep -i federation

# 检查 Palpo 是否能访问其他服务器
docker compose exec palpo curl -sf https://matrix.org/.well-known/matrix/server

# 从外部验证 well-known 端点
curl -sf https://matrix.example.com/.well-known/matrix/server
curl -sf https://matrix.example.com/.well-known/matrix/client

# 检查 TLS 证书
openssl s_client -connect matrix.example.com:443 -servername matrix.example.com < /dev/null 2>/dev/null | openssl x509 -noout -dates
```

---

## 7. 延伸阅读

- **Matrix 联邦规范：** [spec.matrix.org/latest/server-server-api](https://spec.matrix.org/latest/server-server-api/) -- 服务器间通信的协议规范。
- **Matrix Federation Tester：** [federationtester.matrix.org](https://federationtester.matrix.org/) -- 在线工具，验证你的联邦配置。
- **Palpo GitHub：** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) -- Palpo 服务器源码和文档。
- **Let's Encrypt：** [letsencrypt.org](https://letsencrypt.org/) -- 免费、自动化的 TLS 证书。
- **Caddy：** [caddyserver.com](https://caddyserver.com/) -- 自动 HTTPS 的反向代理。

---

*本指南覆盖 2026 年 4 月的联邦配置。获取最新更新，请查看各项目仓库。*
