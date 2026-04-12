# 联邦功能（生产环境部署）

> ### ⚠️ 高级内容 / 不需要本地跑就能看这篇
>
> 本文档是**进阶内容**。如果你只想在本机跑起来一个可用的联邦环境（两个 Palpo + 一个 Octos bot），请看 [04-federation-with-palpo-zh.md](04-federation-with-palpo-zh.md) -- 那篇文档是**完全自足**的，不依赖本文档任何配置。
>
> 本文档的目的是让你在**真实服务器**上对外提供服务时，知道哪些东西必须从"本地测试模式"换成"生产模式"，以及为什么。

> **目标：** 按照本指南操作后，你的 Palpo 将部署在真实域名上，支持 Let's Encrypt TLS 证书和反向代理，可以和 `matrix.org` 等公共 Matrix 服务器互通。

本文档介绍**生产环境**的 Matrix 联邦部署 -- 真实域名、受信任 TLS 证书、反向代理、DNS 配置、安全加固。

---

## 🔀 本地测试 vs 生产部署：完整差异速查表

下面这张表是本文档的**核心价值** -- 把 04 篇里能跑的本地环境，逐项对照出生产部署需要做什么变化。

| 方面 | 本地测试（04 篇） | 生产部署（本篇） | 为什么变 |
|------|-----------------|-----------------|---------|
| **域名 / 主机名** | | | |
| `server_name` | `palpo-1:8448` / `palpo-2:8448` | `matrix.example.com` | 公网服务器要能被 DNS 解析 |
| 服务器如何互相找到 | Docker 网络 DNS 别名 | 真实 DNS A 记录 + (可选) SRV 记录 | Docker 别名只在容器内生效 |
| **TLS 证书** | | | |
| 证书类型 | 自签 `openssl req -x509` | 受信 CA（Let's Encrypt 等） | 远程服务器会拒绝自签证书 |
| 证书验证 | `allow_invalid_tls_certificates = true` | **不设** 或明确 `false` | 生产必须做证书验证 |
| 证书管理 | 手工生成（一次性） | Caddy 自动 / certbot 定期续签 | Let's Encrypt 证书 90 天过期 |
| **网络端口** | | | |
| C-S API 对外 | `localhost:6001` / `localhost:6002` | `https://matrix.example.com` (443) | 公网需标准端口 + HTTPS |
| 联邦 API 对外 | `localhost:6401` / `localhost:6402` | `matrix.example.com:8448` | 其他服务器通过 8448 或 443 + well-known 联系 |
| 反向代理 | 不用 | Caddy / Nginx（强烈推荐） | 集中管理 TLS、well-known、限流 |
| TLS 终止位置 | Palpo 自己（`[[listeners]] [listeners.tls]`） | Caddy / Nginx 终止，Palpo 只跑 HTTP 8008 | 反向代理统一处理证书 |
| **well-known 配置** | | | |
| `[well_known].server` | `localhost:6401`（方便 host 客户端调试） | `matrix.example.com:443` | 生产环境公告真实端点 |
| `[well_known].client` | `http://localhost:6001` | `https://matrix.example.com` | 生产强制 HTTPS |
| well-known 服务方 | Palpo 内置 | Caddy 直接响应（或 Palpo 后端） | Caddy 更灵活，不受 Palpo 重启影响 |
| **安全相关** | | | |
| `allow_registration` | `true`（方便测试） | `false`（先建账号再锁） | 防止随机注册刷账号 |
| `yes_i_am_very_very_sure...` | `true` | 移除或 `false` | 生产不应使用无条件注册 |
| 数据库密码 | `palpo:palpo`（固定弱密码） | `.env` 里的强随机密码 | 防止数据库暴露后被打爆 |
| API key（如 DeepSeek） | 直接写在 `compose.yml` / `config.json` | `.env` 环境变量，加 `.gitignore` | 避免误入 git 仓库 |
| 防火墙 | 无所谓（本机） | 只开 443 / 8448 | 内部端口不对外暴露 |
| **日志与运维** | | | |
| 日志格式 | `pretty`（给人看） | `json`（给日志系统收集） | 结构化日志方便告警 |
| `RUST_LOG` | `debug` | `info` 或 `warn` | 生产减少 I/O 开销 |
| 数据持久化 | Docker volume 足够 | 定期备份 Postgres + media | 生产数据丢失不可恢复 |
| **联邦访问控制** | | | |
| `[federation].enable` | `true` | `true` | 一致 |
| `[federation].allowed_servers` | 不设（全开） | 可设白名单限制联邦对象 | 内部服务器可能只允许特定伙伴 |
| `[federation].denied_servers` | 不设 | 可加黑名单屏蔽恶意服务器 | 用于防垃圾 / 封禁 |
| `trusted_servers` | 不设 | `["matrix.org"]` | 生产环境需要公证服务器帮验证远程 key |
| **Bot (Octos) 配置** | | | |
| `botfather.json` 的 `server_name` | `palpo-1:8448` | `matrix.example.com` | 生产用真实域名 |
| `botfather.json` 的 `homeserver` | `http://palpo-1:8008` | `http://palpo:8008`（仍是 Docker 内部名） | bot 通过 Docker 网络连 Palpo，不走公网 |
| AppService namespace regex | `@bot:palpo-1:8448` | `@octosbot:matrix\\.example\\.com` | 匹配真实 MXID 格式 |
| `allowed_senders` | `[]`（全开） | `[]` 或显式白名单 | 生产可限制谁能用 bot |

> **重要原则：** 上面表格里，**只有 `server_name` 和几个安全相关的字段必须改**。像 `homeserver` 指向 Docker 内部名（`http://palpo:8008`）这种配置，在本地和生产**都一样** -- 因为 Octos 总是通过 Docker 网络连 Palpo，不需要走公网。

---

## 📚 本文档的范围

| 场景 | 本文档 | 其他文档 |
|------|--------|---------|
| **生产环境联邦** | ✅ 本文档 | -- |
| **本地测试联邦**（Docker DNS、自签证书） | ❌ | [04-federation-with-palpo-zh.md](04-federation-with-palpo-zh.md) |
| 单节点本地部署 | ❌ | [01-deploying-palpo-and-octos-zh.md](01-deploying-palpo-and-octos-zh.md) |

> **前提条件：** 建议先在本地完成[第 04 篇](04-federation-with-palpo-zh.md)的双节点联邦测试，理解 `server_name`、`well-known`、联邦端口等概念之后再来部署生产环境。本文档假设你已经有能访问真实域名 DNS 的服务器和管理员权限。

---

## 目录

1. [生产部署的前提条件](#1-生产部署的前提条件)
2. [整体架构](#2-整体架构)
3. [域名与 DNS 配置](#3-域名与-dns-配置)
4. [反向代理（Caddy 示例）](#4-反向代理caddy-示例)
5. [`palpo.toml` 生产配置](#5-palpotoml-生产配置)
6. [Docker Compose 变更](#6-docker-compose-变更)
7. [AppService 注册更新](#7-appservice-注册更新)
8. [启动与验证](#8-启动与验证)
9. [使用联邦功能](#9-使用联邦功能)
10. [故障排查](#10-故障排查)
11. [延伸阅读](#11-延伸阅读)

---

## 1. 生产部署的前提条件

与本地测试部署不同，生产环境联邦对基础设施有严格要求：

| 需求 | 本地测试（04 篇） | 生产部署（本篇） |
|------|-----------------|-----------------|
| 域名 | 不需要（Docker DNS 别名） | **必需**（如 `matrix.example.com`） |
| TLS 证书 | 自签（`allow_invalid_tls_certificates = true`） | **必需**（Let's Encrypt 等受信 CA） |
| 端口 443 | 不需要 | **开放**（Client-Server API） |
| 端口 8448 | 仅 Docker 内部 | **开放**（Server-Server 联邦 API） |
| 反向代理 | 不需要 | **推荐**（Caddy / Nginx） |
| DNS 记录 | 不需要 | A 记录必需，SRV 记录可选 |
| 公网 IP | 不需要 | **必需** |

> **⚠️ 自签名证书不能用于生产联邦。** 其他 Matrix 服务器会拒绝 TLS 连接，联邦消息无法投递。请使用 [Let's Encrypt](https://letsencrypt.org/) 等受信 CA 获取证书。

---

## 2. 整体架构

典型的生产拓扑：

```
                  Internet
                      │
                      │ 443 / 8448
                      ▼
              ┌───────────────┐
              │  Caddy        │   ← 反向代理 + 自动 TLS
              │  (host)       │
              └───────┬───────┘
                      │ localhost:8008
                      ▼
┌─────────── Docker 网络 ────────────┐
│   ┌──────────────┐                 │
│   │ Palpo        │                 │
│   │ server_name: │                 │
│   │ matrix.      │                 │
│   │ example.com  │                 │
│   └──────┬───────┘                 │
│          │                         │
│          ▼ AppService              │
│   ┌──────────────┐    ┌─────────┐  │
│   │ Octos        │    │ Postgres│  │
│   └──────────────┘    └─────────┘  │
└─────────────────────────────────────┘
```

**关键设计：**

1. Caddy 监听公网 443，负责 TLS 终止和 Let's Encrypt 证书自动续签
2. Palpo 在 docker 内部只开 HTTP 8008，由 Caddy 代理
3. 客户端（Robrix/Element）通过 HTTPS 连 `matrix.example.com`
4. 其他联邦服务器通过 HTTPS 连 `matrix.example.com:8448`（或 443 + well-known 委托）

---

## 3. 域名与 DNS 配置

### 3.1 注册域名

注册一个域名（如 `example.com`），用子域名分配给 Matrix 服务器，例如 `matrix.example.com`。

### 3.2 创建 DNS A 记录

将子域名指向服务器的公网 IP：

```
matrix.example.com.   IN  A   203.0.113.10
```

### 3.3 （可选）创建 SRV 记录

如果联邦 API 用的不是 8448 的默认端口，或者要让 `example.com` 的联邦跳转到 `matrix.example.com`，需要 SRV 记录：

```
_matrix-fed._tcp.example.com.   IN  SRV   10 0 8448 matrix.example.com.
```

> **什么时候不需要 SRV 记录？** 如果你在 `matrix.example.com:443` 上提供了 `/.well-known/matrix/server`，Matrix 客户端会通过 well-known 委托发现联邦端点，不依赖 SRV。大多数生产部署采用这种方式。

### 3.4 生产 DNS 参数（palpo.toml）

在 Docker 网络中运行时，DNS 解析建议用 TCP：

```toml
query_over_tcp_only = true       # 容器网络中 UDP DNS 有时不稳定
query_all_nameservers = true     # 查所有配置的 DNS，避免单点失败
ip_lookup_strategy = 5           # 5 = 先 IPv4 再 IPv6
```

---

## 4. 反向代理（Caddy 示例）

生产环境**强烈推荐**使用反向代理，原因：

1. **自动 TLS** -- Caddy 内置 Let's Encrypt，自动申请和续签证书
2. **well-known 端点管理** -- 直接用 Caddy 响应，不依赖 Palpo
3. **流量控制** -- 限流、日志、WAF 等都好挂载

### 4.1 Caddyfile

```caddyfile
matrix.example.com {
    # Matrix 客户端发现端点
    handle /.well-known/matrix/client {
        header Access-Control-Allow-Origin "*"
        respond `{"m.homeserver":{"base_url":"https://matrix.example.com"}}`
    }

    # Matrix 联邦发现端点
    handle /.well-known/matrix/server {
        respond `{"m.server":"matrix.example.com:443"}`
    }

    # 其他请求代理给 Palpo
    reverse_proxy localhost:8008
}

# 如果走非 443 端口的联邦，额外监听：
# matrix.example.com:8448 {
#     reverse_proxy localhost:8008
# }
```

### 4.2 禁用 Palpo 自带 TLS

让 Caddy 独家处理 TLS，Palpo 内部只跑明文 HTTP：

```toml
# palpo.toml
[tls]
enable = false
```

### 4.3 Nginx 替代方案

如果必须用 Nginx，需要单独用 `certbot` 管理证书：

```bash
# 申请证书
sudo certbot --nginx -d matrix.example.com

# 证书路径
# /etc/letsencrypt/live/matrix.example.com/fullchain.pem
# /etc/letsencrypt/live/matrix.example.com/privkey.pem
```

然后在 Nginx 配置里写 `ssl_certificate` / `ssl_certificate_key` 指令，并代理 `/` 到 `http://127.0.0.1:8008`，以及显式响应 `/.well-known/matrix/server` 和 `/.well-known/matrix/client`。

---

## 5. `palpo.toml` 生产配置

```toml
# ── 核心配置 ──────────────────────────────────
# 修改：真实域名
server_name = "matrix.example.com"

# 修改：生产环境关闭开放注册
# 建议先创建管理员账号，再设为 false
allow_registration = false

enable_admin_room = true
appservice_registration_dir = "/var/palpo/appservices"

# 生产环境不要开自签证书豁免！
# allow_invalid_tls_certificates 保持默认 false

# ── 监听器 ─────────────────────────────────────
# Caddy 在 443 代理到这里
[[listeners]]
address = "0.0.0.0:8008"

# ── 日志 ───────────────────────────────────────
[logger]
format = "json"         # 修改：生产用 JSON 格式，方便收集
level = "info"

# ── 数据库 ─────────────────────────────────────
[db]
url = "postgres://palpo:<强密码>@palpo_postgres:5432/palpo"
pool_size = 10

# ── well-known（Palpo 自己响应；如果 Caddy 代管可省略）─
[well_known]
server = "matrix.example.com:443"
client = "https://matrix.example.com"

# ── 联邦设置 ───────────────────────────────────
[federation]
enable = true
allow_inbound_profile_lookup = true    # 允许远程查本地用户 profile
# 可选访问控制：
# allowed_servers = ["matrix.org", "*.trusted.com"]
# denied_servers = ["evil.com"]

# ── TLS（推荐关闭，让 Caddy 处理）───────────────
[tls]
enable = false
# 如果不用反向代理：
# enable = true
# cert = "/path/to/fullchain.pem"
# key = "/path/to/privkey.pem"
# dual_protocol = false

# ── 在线状态与输入提示（跨联邦实时指示）─────────
[presence]
allow_local = true
allow_incoming = true
allow_outgoing = true

[typing]
allow_incoming = true
allow_outgoing = true
federation_timeout = 30000

# ── 受信服务器（key 验证公证人）─────────────────
trusted_servers = ["matrix.org"]

# ── DNS 优化（容器内推荐）─────────────────────
query_over_tcp_only = true
query_all_nameservers = true
ip_lookup_strategy = 5
```

### 5.1 `[federation]` 字段参考

| 字段 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `enable` | bool | `true` | 联邦总开关 |
| `allow_loopback` | bool | `false` | 允许向自身发联邦请求，仅开发用 |
| `allow_device_name` | bool | `false` | 向联邦暴露设备名，隐私考虑建议关 |
| `allow_inbound_profile_lookup` | bool | `true` | 允许远程查本地用户 profile |
| `allowed_servers` | list | 无 | 允许列表，支持通配符 `*.example.com`，未设置则全部允许 |
| `denied_servers` | list | `[]` | 拒绝列表，**优先级高于** `allowed_servers` |

### 5.2 受信服务器（Perspectives Key 验证）

`trusted_servers` 充当**公证服务器**，帮助验证其他服务器的签名密钥。这是 [Perspectives 密钥验证](https://spec.matrix.org/latest/server-server-api/#querying-keys-through-another-server)机制。

```toml
trusted_servers = ["matrix.org"]
```

最常用的选择是 `matrix.org`，因为它是公共联邦的中心节点。

---

## 6. Docker Compose 变更

> **基线说明：** 本节的对比是针对**单节点部署**（`palpo-and-octos-deploy/compose.yml`，`server_name = "127.0.0.1:8128"`），不是 04 号文档的双节点联邦。因为生产环境拓扑几乎总是**单 homeserver + 对外联邦**，结构上更接近单节点而非 04 号的本地双节点模拟。如果你是从 04 号过来的，端口/服务名的本地列略过就好，重点关注**右列**（生产值）— 生产值本身是通用的。

相比本地 `compose.yml` 的关键变更：

```yaml
services:
  palpo:
    # 镜像与构建部分和本地相同
    ports:
      - "8008:8008"          # Caddy 代理到此端口（不再暴露 8128）
    volumes:
      - ./palpo.toml:/var/palpo/palpo.toml:ro
      - ./appservices:/var/palpo/appservices:ro
      - ./data/media:/var/palpo/media
      # 若 Palpo 直接处理 TLS（不推荐），挂载证书：
      # - /etc/letsencrypt/live/matrix.example.com:/certs:ro
    restart: unless-stopped
    # ... 其余配置和本地一致 ...

  palpo_postgres:
    environment:
      POSTGRES_PASSWORD: ${DB_PASSWORD}   # 改：从 .env 读强密码
      POSTGRES_USER: palpo
      POSTGRES_DB: palpo
    # ...

  octos:
    environment:
      DEEPSEEK_API_KEY: ${DEEPSEEK_API_KEY}
      RUST_LOG: octos=info                # 改：生产用 info 而非 debug
    # ...
```

整体架构和本地部署一致 -- Postgres、Palpo、Octos -- 主要差异：

| 本地 | 生产 |
|------|------|
| `server_name = "127.0.0.1:8128"` | `server_name = "matrix.example.com"` |
| 暴露端口 `8128:8008` | 暴露端口 `8008:8008`（由 Caddy 代理） |
| Palpo 自管 TLS（或不用 TLS） | Caddy 终止 TLS |
| `allow_registration = true` | `allow_registration = false` |
| 日志 `pretty` | 日志 `json` |
| API key 可直接写 yml | 从 `.env` 环境变量读 |

---

## 7. AppService 注册更新

从 `127.0.0.1:8128` 切到真实域名时，需要更新以下文件。

### 7.1 AppService namespace 文件

不同的本地部署用不同的文件名：

| 起点 | 路径 |
|------|------|
| 单节点（01 号文档） | `palpo-and-octos-deploy/appservices/octos-registration.yaml` |
| 双节点联邦（04 号文档） | `palpo-and-octos-deploy/federation/nodes/node1/appservices/octos.yaml` |

不管用哪个，都要把 namespace regex 改成你的真实域名：

```yaml
namespaces:
  users:
    - exclusive: true
      regex: "@octosbot_.*:matrix\\.example\\.com"    # 改：真实域名
    - exclusive: true
      regex: "@octosbot:matrix\\.example\\.com"       # 改：真实域名
```

### 7.2 `config/botfather.json`

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

> **重要：** `homeserver` URL **保持** Docker 内部地址（`http://palpo:8008`），因为 Octos 通过 Docker 网络连接 Palpo -- 不需要走公网。只有 `server_name` 需要改为真实域名，因为那是对外的 Matrix 身份。

---

## 8. 启动与验证

### 8.1 启动服务

```bash
cd palpo-and-octos-deploy   # 或你的生产部署目录

# 设置环境变量
cp .env.example .env
vim .env    # 填 DEEPSEEK_API_KEY、DB_PASSWORD 等

# 启动
docker compose up -d

# 查看状态
docker compose ps
docker compose logs -f
```

### 8.2 测试 well-known 端点

```bash
# 服务器发现（其他联邦服务器用）
curl https://matrix.example.com/.well-known/matrix/server
# 期望：{"m.server":"matrix.example.com:443"}

# 客户端发现（Robrix/Element 用）
curl https://matrix.example.com/.well-known/matrix/client
# 期望：{"m.homeserver":{"base_url":"https://matrix.example.com"}}
```

### 8.3 Matrix Federation Tester

访问 [https://federationtester.matrix.org](https://federationtester.matrix.org) 输入你的域名（`matrix.example.com`）。会检查：

- DNS 解析是否正确
- TLS 证书是否受信任
- well-known 端点是否响应
- Server-Server API 是否可达
- 签名密钥验证是否通过

所有检查通过才说明联邦配置完整。

---

## 9. 使用联邦功能

联邦生效后，你可以：

### 9.1 加入其他服务器的房间

Robrix 里：

1. 点左侧导航栏的 **＋** 按钮打开 **Add/Explore Rooms and Spaces** 页面
2. 在最底下的 **Join an existing room or space** 区域，输入目标房间别名（`#general:matrix.org`）、ID（`!...:matrix.org`）或 `matrix:` 链接，点 **Go**
3. 你的服务器通过联邦联系 `matrix.org` 并加入
4. 来自所有参与服务器的消息实时同步

### 9.2 邀请其他服务器的用户

1. 打开你的房间，通过房间里的 invite 入口（右键房间或打开房间 info 面板，具体位置取决于 Robrix 版本）
2. 输入远程用户 MXID：`@friend:other-server.com`
3. 邀请通过联邦送到远程服务器
4. 对方接受后加入你的房间

### 9.3 跨联邦 AI 机器人

联邦启用后，**其他服务器**的用户也能和你的 Octos 机器人交互：

1. `matrix.org` 上的用户邀请 `@octosbot:matrix.example.com` 到他们的房间
2. 邀请通过联邦送达你的服务器
3. Octos 接受邀请并加入房间
4. 机器人响应消息 -- 即使房间在远程服务器

> **注意：** 要让这个功能工作，`botfather.json` 里的 `allowed_senders` 必须是空数组 `[]`（允许所有人），或显式包含远程用户的 MXID（如 `@remoteuser:matrix.org`）。

---

## 10. 故障排查

### 10.1 常见问题

| 症状 | 原因 | 解决方法 |
|------|------|---------|
| 无法加入其他服务器房间 | 联邦未启用或端口被墙 | 检查 `[federation] enable = true`；确保防火墙开 443 和 8448 |
| "Unable to find signing key" | TLS 或 DNS 问题 | 证书必须是受信任 CA 签的（不能自签）；查 DNS 解析 |
| well-known 返回 404 | 反向代理没转发 | 检查 Caddy/Nginx 配置里 `/.well-known/matrix/*` 的 handle |
| 远程用户看不到本地用户 profile | profile lookup 被禁 | 设 `allow_inbound_profile_lookup = true` |
| 连接远程服务器超时 | 出站防火墙或 DNS 问题 | 试 `query_over_tcp_only = true`；验证能否访问 8448 |
| 机器人不回复联邦用户 | `allowed_senders` 过滤 | 设为 `[]` 或加远程用户 MXID |
| Federation Tester 报 TLS 错 | 证书链不完整或过期 | 检查 fullchain.pem 包含中间证书；查证书有效期 |

### 10.2 调试命令

```bash
# Palpo 联邦日志
docker compose logs palpo | grep -i federation

# Palpo 能否访问其他服务器
docker compose exec palpo curl -sf https://matrix.org/.well-known/matrix/server

# 从外部验证 well-known
curl -sf https://matrix.example.com/.well-known/matrix/server
curl -sf https://matrix.example.com/.well-known/matrix/client

# 证书有效期检查
openssl s_client -connect matrix.example.com:443 \
  -servername matrix.example.com < /dev/null 2>/dev/null \
  | openssl x509 -noout -dates

# 测试 Server-Server API 版本
curl -sf https://matrix.example.com:8448/_matrix/federation/v1/version
```

### 10.3 安全自检

生产环境启动前的清单：

- [ ] `allow_registration = false`（或设置注册 token）
- [ ] `yes_i_am_very_very_sure...` 已移除或设为 false
- [ ] 数据库密码不是默认值
- [ ] `.env` 文件不在 git 仓库里（加到 `.gitignore`）
- [ ] `allow_invalid_tls_certificates` 未设置或为 false
- [ ] TLS 证书来自受信 CA（非自签）
- [ ] Caddy/Nginx 的 HTTPS 重定向已启用
- [ ] 防火墙只开 443 / 8448（不暴露 8008）
- [ ] 日志格式为 `json`，方便收集和告警
- [ ] 数据卷配置了定期备份（尤其是 Postgres）

---

## 11. 延伸阅读

- **Matrix 联邦规范：** [spec.matrix.org/latest/server-server-api](https://spec.matrix.org/latest/server-server-api/) -- 服务器间通信协议规范
- **Matrix Federation Tester：** [federationtester.matrix.org](https://federationtester.matrix.org/) -- 联邦配置在线验证
- **Palpo GitHub：** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) -- Palpo 服务器源码
- **Let's Encrypt：** [letsencrypt.org](https://letsencrypt.org/) -- 免费自动化 TLS 证书
- **Caddy：** [caddyserver.com](https://caddyserver.com/) -- 内置自动 HTTPS 的反向代理
- **Certbot：** [certbot.eff.org](https://certbot.eff.org/) -- Nginx + Let's Encrypt 工具

---

*本指南覆盖 2026 年 4 月的生产部署。具体配置项可能随上游更新变化，以各项目仓库为准。*
