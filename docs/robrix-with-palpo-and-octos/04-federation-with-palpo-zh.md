# 联邦功能（本地双节点测试）

[English Version](04-federation-with-palpo.md)

> **目标：** 按照本指南操作后，你将在本机运行两个相互联邦的 Palpo 节点：节点 1 上部署 Octos AI 机器人，节点 2 上注册普通用户，然后在 Robrix 中以节点 2 的用户身份**跨服务器**与节点 1 上的机器人聊天 -- 完全不需要公网域名或真实证书。

---

## 🚀 快速开始（5 条命令跑起来）

本仓库已经提供了一套**开箱即用的联邦配置**，在 `palpo-and-octos-deploy/federation/` 目录下。你不需要自己写任何配置文件。

### 前提

- 已安装 Docker + Docker Compose
- 已按 [01-deploying-palpo-and-octos-zh.md](01-deploying-palpo-and-octos-zh.md) clone 过 `repos/palpo` 和 `repos/octos`
- 如果单节点部署在跑，先停掉它：`cd palpo-and-octos-deploy && docker compose down`

### 运行

```bash
cd palpo-and-octos-deploy/federation

# 1. 生成两节点自签证书（一次性）
./gen-certs.sh

# 2. 设置 API key
cp .env.example .env
$EDITOR .env         # 填 DEEPSEEK_API_KEY

# 3. 构建并启动全部 5 个服务
docker compose up -d --build

# 4. 观察状态（palpo-1 / palpo-2 要变 healthy）
docker compose ps
```

### 服务端点对照（后续步骤会用到）

两个 palpo 容器各自对外暴露**两组端口**：客户端 API（给 Robrix 用的 HTTP）和联邦 API（两节点之间握手用的 TLS）。下面的表把 URL 和 Matrix 身份字符串一起列出来——两者**不是一回事**，新用户最容易混淆这一点。

| 服务 | 客户端 API URL（Robrix 里 Homeserver 填这个） | server_name（MXID 里用这个） | 联邦 API（节点之间握手） | 用途 |
|------|------------------------------------------|--------------------------|---------------------|------|
| **palpo-1** | `http://localhost:6001` | `palpo-1:8448` | `https://localhost:6401` | 跑 Octos bot，bot 账号 `@bot:palpo-1:8448` |
| **palpo-2** | `http://localhost:6002` | `palpo-2:8448` | `https://localhost:6402` | 跑普通用户，稍后注册 `@alice:palpo-2:8448` |

> **记住这条规则**：Robrix 登录 / curl 打 HTTP 用 **左边的 URL**；MXID 里出现的 `palpo-X:8448` 是**身份标识**，不是 URL，不要混着填。

### 注册 palpo-2 上的用户（必需）

全新环境下 palpo-2 上还没有任何用户。在 Robrix 登录界面点 **Sign up（注册）**，用下面的值注册 alice：

| 字段 | 值 |
|------|-----|
| Username | `alice` |
| Password | `test1234` |
| **Homeserver** | `http://localhost:6002` |

完整截图和步骤见 [02-using-robrix-with-palpo-and-octos-zh.md 第 3 节 注册账号](02-using-robrix-with-palpo-and-octos-zh.md#3-注册账号)。

### 验证联邦通路（可选，用 curl）

想在打开 Robrix 之前先确认联邦握手没问题，可以用 alice 跨服务器查一次 bot 的 profile（这会触发 palpo-2 → palpo-1 的联邦调用）：

```bash
# alice 登录拿 token
TOKEN=$(curl -s -X POST http://localhost:6002/_matrix/client/v3/login \
  -H "Content-Type: application/json" \
  -d '{"type":"m.login.password","identifier":{"type":"m.id.user","user":"alice"},"password":"test1234"}' \
  | jq -r .access_token)

# 用 alice 查 palpo-1 上 bot 的 profile（会触发联邦调用）
curl -s "http://localhost:6002/_matrix/client/v3/profile/@bot:palpo-1:8448" \
  -H "Authorization: Bearer $TOKEN"
# 返回非 404 就说明联邦通路 OK
```

### 在 Robrix 里跨联邦和 bot 聊天

用上一步注册的 alice 账号登录 Robrix：

| 字段 | 值 |
|------|-----|
| Username | `@alice:palpo-2:8448` |
| Password | `test1234` |
| **Homeserver** | `http://localhost:6002` |

登录后，点击左侧导航栏的 **＋** 按钮打开 **Add/Explore Rooms and Spaces** 页面。**Matrix 的用户目录搜索只覆盖本服务器**，所以你搜不到 palpo-1 上的 bot -- 要走中间的 **Add a friend** 这条路：

![Robrix Add a friend 面板](../images/robrix-add-friend.png)

1. 在 **Add a friend** 输入框里填入 bot 的完整 MXID：`@bot:palpo-1:8448`
2. 点击 **Add friend** 按钮 -- Robrix 会通过 palpo-2 → palpo-1 的联邦通道创建 DM 房间
3. 进入新创建的房间，发送 `hello`，等 bot 回复

> **为什么必须从 "Add a friend" 走，而不是搜索？**
>
> Matrix 的用户目录搜索（`/user_directory/search`）**只索引本服务器已知的用户**。palpo-2 刚起来只有 alice 自己，**不认识任何 palpo-1 上的用户**，所以不管在搜索框里怎么搜都找不到 `@bot`。
>
> "Add a friend" 直接调用 `/createRoom` + `invite`：palpo-2 收到邀请请求后会**主动发起联邦请求**去 palpo-1 验证这个 MXID 存在、是否可邀请 -- 这是跨联邦建立 DM 的**唯一正确入口**。
>
> 这个限制不是 Robrix 特有的 -- Element、SchildiChat 等所有 Matrix 客户端跨联邦聊天都得这样：输入完整 MXID + 发起邀请，而不是搜索。

**如果 bot 回复了消息，说明：联邦握手、AppService 转发、Octos bot 回复这三条链路全部打通了。**

---

## 📚 本文档的后续内容 / 进阶阅读

上面的快速开始足以完成测试。下面是这套配置**为什么能工作**的详细解释，遇到问题时查阅。

| 你想做什么 | 看哪里 |
|------|------|
| 只想跑起来，遇到报错时查问题 | [第 8 节 故障排查](#8-故障排查) |
| 理解为什么要这么配（架构原理） | [第 2 节 架构](#2-本地双节点架构) + [第 7 节 消息流](#7-消息流详解) |
| 想改配置（不同端口、不同 bot 名字等） | [第 4 节 配置文件说明](#4-配置文件说明) |
| 要部署到真实服务器 | [05-federation-production-deployment-zh.md](05-federation-production-deployment-zh.md)（高级内容） |
| 单节点部署（无联邦） | [01-deploying-palpo-and-octos-zh.md](01-deploying-palpo-and-octos-zh.md) |
| 在 Robrix 里使用 Palpo + Octos | [02-using-robrix-with-palpo-and-octos-zh.md](02-using-robrix-with-palpo-and-octos-zh.md) |

---

## 目录（进阶内容）

1. [什么是 Matrix 联邦？](#1-什么是-matrix-联邦)
2. [本地双节点架构](#2-本地双节点架构)
3. [文件结构](#3-文件结构)
4. [配置文件说明](#4-配置文件说明)
5. [启动细节](#5-启动细节)
6. [备选方案：API 层测试（CI / 无头脚本）](#6-备选方案api-层测试ci--无头脚本)
7. [消息流详解](#7-消息流详解)
8. [故障排查](#8-故障排查)
9. [下一步](#9-下一步)

---

## 1. 什么是 Matrix 联邦？

Matrix 是一个**去中心化**的通信协议。每个组织都可以运行自己的服务器，联邦机制允许不同服务器上的用户无缝通信，类似电子邮件：

- `@alice:server-a.com` 可以和 `@bob:server-b.com` 直接聊天
- 每个服务器独立存储自己用户的数据
- 消息在参与对话的所有服务器之间复制同步
- 任意一台服务器宕机不影响其他服务器

Matrix 客户端连接的 API 分两类：

| API | 端口（默认） | 用途 |
|-----|-------------|------|
| **Client-Server API (C-S)** | 443（或 8008） | 客户端（Robrix、Element）与自己的 homeserver 通信 |
| **Server-Server API (联邦)** | 8448 | 两个 homeserver 之间互相通信 |

本地部署指南里只用了 C-S API，服务器是隔离的。**联邦的关键是多开一个端口 8448，并且用 TLS 加密。**

---

## 2. 本地双节点架构

本文档使用 `palpo-and-octos-deploy/federation/` 目录下的双节点 Docker 环境。两个 Palpo 节点通过 Docker 内部网络（`palpo-federation`）互相发现，不需要公网域名。

```
┌──── Docker 网络 "palpo-federation" ────────────────────┐
│                                                         │
│   ┌──────────────────────┐     ┌──────────────────────┐│
│   │  palpo-1             │     │  palpo-2             ││
│   │  server_name:        │     │  server_name:        ││
│   │    palpo-1:8448      │◄───►│    palpo-2:8448      ││
│   │                      │联邦 │                      ││
│   │  8008 → host:6001    │8448 │  8008 → host:6002    ││
│   │  8448 → host:6401    │     │  8448 → host:6402    ││
│   │  (TLS self-signed)   │     │  (TLS self-signed)   ││
│   └──────────┬───────────┘     └──────────────────────┘│
│              │                                          │
│              │ AppService (HTTP transaction)           │
│              ▼                                          │
│   ┌──────────────────────┐                             │
│   │  octos               │                             │
│   │  bot MXID:           │                             │
│   │    @bot:palpo-1:8448 │                             │
│   │  监听 8009           │                             │
│   └──────────────────────┘                             │
│                                                         │
│   (postgres 数据库略去)                                 │
└─────────────────────────────────────────────────────────┘

                   Robrix (host 上)
                   ↓ 连接到 localhost:6002
                   登录 @alice:palpo-2:8448
                   ↓ 发 DM 给 @bot:palpo-1:8448
                   （通过联邦跨服务器送达）
```

### 端口分配

| 服务 | 容器端口 | Host 暴露端口 | 用途 |
|------|---------|-------------|------|
| palpo-1 | 8008 | 6001 | Client-Server API（Robrix / curl 直连） |
| palpo-1 | 8448 | 6401 | 联邦 API（外部调试观察） |
| palpo-2 | 8008 | 6002 | Client-Server API |
| palpo-2 | 8448 | 6402 | 联邦 API |
| octos | 8009 | 8009 | AppService transaction 接收 |

容器之间通过 Docker 网络别名（`palpo-1`、`palpo-2`、`octos`）直接通信，不走 host 暴露端口。

---

## 3. 文件结构

完整部署目录（git 里的 + 运行时生成的）：

```
palpo-and-octos-deploy/federation/
├── docker-compose.yml              # 5 个服务：2 palpo + 2 postgres + octos
├── palpo.Dockerfile                # Alpine 版 Palpo 构建+运行时镜像
├── gen-certs.sh                    # 生成自签 TLS 证书到 certs/
├── .env.example                    # 模板：DEEPSEEK_API_KEY、RUST_LOG
├── .gitignore                      # 排除 certs/ data/ nodes/*/media/
├── README.md                       # 目录内速查说明
├── config/
│   ├── botfather.json              # Octos bot 配置文件（渠道 + LLM + admin_mode）
│   └── octos.json                  # Octos 运行时：profiles_dir、data_dir、log_level
├── nodes/
│   ├── node1/
│   │   ├── palpo.toml              # server_name = "palpo-1:8448"
│   │   ├── appservices/
│   │   │   └── octos.yaml          # AppService 注册（octos 的 namespace）
│   │   └── media/                  # [运行时] 上传的媒体，持久化
│   └── node2/
│       ├── palpo.toml              # server_name = "palpo-2:8448"
│       └── media/                  # [运行时]
├── certs/                          # [运行时] gen-certs.sh 生成
│   ├── node1.crt / node1.key
│   └── node2.crt / node2.key
└── data/                           # [运行时] postgres + octos 持久化
    ├── pg-1/                       # palpo-1 的 postgres 数据
    ├── pg-2/                       # palpo-2 的 postgres 数据
    └── octos/                      # Octos 状态（挂载为 /root/.octos）
```

> `[运行时]` 标记的目录被 `.gitignore` 排除 — 跑 `./gen-certs.sh` 和 `docker compose up` 之后才会出现。

---

## 4. 配置文件说明

> **本节目的：** 快速开始用的配置文件已经在 `palpo-and-octos-deploy/federation/` 里写好。下面的内容是**解释每个文件里的关键字段在做什么**，方便你需要改配置时知道该改哪里，以及出错时明白哪里容易坑。

### 4.1 生成自签证书（`./gen-certs.sh` 做了什么）

`gen-certs.sh` 脚本等价于下面这段 openssl 命令：

```bash
# 为 palpo-1 生成证书，CN 必须匹配 server_name 的主机部分
openssl req -x509 -nodes -newkey rsa:2048 -days 365 \
  -keyout certs/node1.key -out certs/node1.crt \
  -subj "/CN=palpo-1" \
  -addext "subjectAltName=DNS:palpo-1"

# 为 palpo-2 生成同样的证书
openssl req -x509 -nodes -newkey rsa:2048 -days 365 \
  -keyout certs/node2.key -out certs/node2.crt \
  -subj "/CN=palpo-2" \
  -addext "subjectAltName=DNS:palpo-2"
```

关键点：**CN 和 subjectAltName 必须匹配 `palpo.toml` 里 `server_name` 的主机部分**（这里是 `palpo-1` / `palpo-2`），否则 TLS 握手会因 hostname 不匹配失败。

### 4.2 `docker-compose.yml`

> 📁 **实际文件：** [`palpo-and-octos-deploy/federation/docker-compose.yml`](../../palpo-and-octos-deploy/federation/docker-compose.yml) -- 下面展示的是关键结构，完整内容请直接看文件。

```yaml
services:
  # ── Node 1：含 Octos AppService ──────────────────────
  palpo-1:
    build:
      context: ..                   # 使用 palpo-and-octos-deploy/repos/palpo
      dockerfile: federation/palpo.Dockerfile
    image: palpo-federation:local-dev
    container_name: palpo-1
    depends_on:
      palpo-pg-1: { condition: service_healthy }
    volumes:
      - ./nodes/node1/palpo.toml:/var/palpo/palpo.toml:ro
      - ./nodes/node1/media:/var/palpo/media
      - ./nodes/node1/appservices:/var/palpo/appservices:ro
      - ./certs/node1.crt:/var/palpo/certs/node1.crt:ro
      - ./certs/node1.key:/var/palpo/certs/node1.key:ro
    environment:
      PALPO_CONFIG: /var/palpo/palpo.toml
      RUST_LOG: palpo=debug,palpo_core=info
    ports:
      - "6001:8008"               # C-S API
      - "6401:8448"               # 联邦 API
    networks:
      federation: { aliases: [palpo-1] }

  palpo-pg-1:
    image: postgres:16-alpine
    container_name: palpo-pg-1
    environment:
      POSTGRES_DB: palpo_node_1
      POSTGRES_USER: palpo
      POSTGRES_PASSWORD: palpo
    volumes: [pg-1-data:/var/lib/postgresql/data]
    networks: [federation]
    healthcheck:
      test: [CMD-SHELL, pg_isready -U palpo]
      interval: 5s
      retries: 10

  # ── Node 2：普通用户（alice 注册到这里）────────────
  palpo-2:
    build:                            # 构建规格和 palpo-1 一样；Docker 层缓存
      context: ..                     # 让第二次构建基本是 no-op
      dockerfile: federation/palpo.Dockerfile
    image: palpo-federation:local-dev # 和 palpo-1 共用同一个 tag
    container_name: palpo-2
    depends_on:
      palpo-pg-2: { condition: service_healthy }
    volumes:
      - ./nodes/node2/palpo.toml:/var/palpo/palpo.toml:ro
      - ./nodes/node2/media:/var/palpo/media
      - ./certs/node2.crt:/var/palpo/certs/node2.crt:ro
      - ./certs/node2.key:/var/palpo/certs/node2.key:ro
    environment:
      PALPO_CONFIG: /var/palpo/palpo.toml
      RUST_LOG: palpo=debug,palpo_core=info
    ports:
      - "6002:8008"
      - "6402:8448"
    networks:
      federation: { aliases: [palpo-2] }

  palpo-pg-2:
    image: postgres:16-alpine
    container_name: palpo-pg-2
    environment:
      POSTGRES_DB: palpo_node_2
      POSTGRES_USER: palpo
      POSTGRES_PASSWORD: palpo
    volumes: [pg-2-data:/var/lib/postgresql/data]
    networks: [federation]
    healthcheck:
      test: [CMD-SHELL, pg_isready -U palpo]
      interval: 5s
      retries: 10

  # ── Octos AppService（只对接 palpo-1）─────────────────
  octos:
    build:
      context: ../repos/octos     # 本地 Octos 源码（由父目录 ./setup.sh 克隆）
      dockerfile: Dockerfile
    image: octos-federation:local-dev
    container_name: octos
    depends_on: [palpo-1]
    volumes:
      - ./data/octos:/root/.octos                                         # 持久化状态
      - ./config/botfather.json:/root/.octos/profiles/botfather.json:ro    # Octos 启动时加载的 bot profile
      - ./config/octos.json:/config/octos.json:ro                          # 运行时配置（profiles_dir 等）
    environment:
      DEEPSEEK_API_KEY: ${DEEPSEEK_API_KEY}
      RUST_LOG: ${RUST_LOG:-octos=debug,info}
    command: ["serve", "--host", "0.0.0.0", "--port", "8080", "--config", "/config/octos.json"]
    ports:
      - "8009:8009"               # AppService transaction 接收
      - "8010:8080"               # Octos 面板 / admin API
    networks:
      federation: { aliases: [octos] }

networks:
  federation:
    name: palpo-federation

volumes:
  pg-1-data:
  pg-2-data:
```

> **关于 Octos 的位置：** 和单节点部署（`palpo-and-octos-deploy/`）一样，本方案把 Octos 也放在 docker 网络里，AppService URL 使用服务名 `http://octos:8009`。这比"Octos 跑在 host 上 + `host.docker.internal`"更简单，也更接近生产部署模式。

### 4.3 `nodes/node1/palpo.toml`

> 📁 **实际文件：** [`palpo-and-octos-deploy/federation/nodes/node1/palpo.toml`](../../palpo-and-octos-deploy/federation/nodes/node1/palpo.toml)

```toml
# ── palpo-1: 用 Docker 网络别名当 server_name ──
server_name = "palpo-1:8448"

allow_registration = true
yes_i_am_very_very_sure_i_want_an_open_registration_server_prone_to_abuse = true
enable_admin_room = true

# ── 本地测试关键：允许自签证书 ──
allow_invalid_tls_certificates = true

appservice_registration_dir = "/var/palpo/appservices"

# Client-Server API（明文 HTTP，给 Robrix / curl 用）
[[listeners]]
address = "0.0.0.0:8008"

# 联邦 API（TLS，给 palpo-2 用）
[[listeners]]
address = "0.0.0.0:8448"
[listeners.tls]
cert = "/var/palpo/certs/node1.crt"
key = "/var/palpo/certs/node1.key"

[logger]
format = "pretty"

[db]
url = "postgres://palpo:palpo@palpo-pg-1:5432/palpo_node_1"
pool_size = 10

# ── 开启联邦 ──
[federation]
enable = true

# well-known：供 host 上的客户端发现（Robrix 用 C-S 连接时）
[well_known]
server = "localhost:6401"
client = "http://localhost:6001"
```

### 4.4 `nodes/node2/palpo.toml`

> 📁 **实际文件：** [`palpo-and-octos-deploy/federation/nodes/node2/palpo.toml`](../../palpo-and-octos-deploy/federation/nodes/node2/palpo.toml)

和 node1 几乎一样，只需要改 server_name、端口、数据库、证书路径：

```toml
server_name = "palpo-2:8448"

allow_registration = true
yes_i_am_very_very_sure_i_want_an_open_registration_server_prone_to_abuse = true
enable_admin_room = true
allow_invalid_tls_certificates = true

[[listeners]]
address = "0.0.0.0:8008"

[[listeners]]
address = "0.0.0.0:8448"
[listeners.tls]
cert = "/var/palpo/certs/node2.crt"
key = "/var/palpo/certs/node2.key"

[logger]
format = "pretty"

[db]
url = "postgres://palpo:palpo@palpo-pg-2:5432/palpo_node_2"
pool_size = 10

[federation]
enable = true

[well_known]
server = "localhost:6402"
client = "http://localhost:6002"
```

> **注意：** node2 上**没有** `appservice_registration_dir`，因为本地测试里 Octos 只注册在 node1。

### 4.5 `nodes/node1/appservices/octos.yaml`

> 📁 **实际文件：** [`palpo-and-octos-deploy/federation/nodes/node1/appservices/octos.yaml`](../../palpo-and-octos-deploy/federation/nodes/node1/appservices/octos.yaml)

这是 Palpo-1 侧的 AppService 注册文件，告诉 Palpo："凡是匹配 `@bot_*:palpo-1:8448` 或 `@bot:palpo-1:8448` 的消息都转发给 Octos"。

```yaml
id: octos-matrix-appservice
url: "http://octos:8009"          # Docker 网络里 octos 服务名
as_token: "436682e5f10a0113775779eb8fedf702a095254a95e229c7d20f085b9082903b"
hs_token: "ef642609a1a5b2eda1486a6bada6411f4e861691a7500b10ff26b5b2e16573fd"
sender_localpart: bot
rate_limited: false
namespaces:
  users:
    - exclusive: true
      regex: "@bot:palpo-1:8448"
    - exclusive: true
      regex: "@bot_.*:palpo-1:8448"
  aliases: []
  rooms: []
```

> **生成自己的 token：** 上面的 `as_token` / `hs_token` 仅用于演示。生产环境用 `openssl rand -hex 32` 为每个 token 生成独立的随机值。本地测试可以直接复用上面的示例值。

### 4.6 `config/botfather.json` 和 `config/octos.json`

> 📁 **实际文件：**
> - [`palpo-and-octos-deploy/federation/config/botfather.json`](../../palpo-and-octos-deploy/federation/config/botfather.json) -- Octos bot profile（LLM 设置 + 把 bot 绑到 palpo-1 的 Matrix 渠道）
> - [`palpo-and-octos-deploy/federation/config/octos.json`](../../palpo-and-octos-deploy/federation/config/octos.json) -- Octos 运行时配置（profiles / data 目录等）

`botfather.json` 是一个 **Octos bot profile**。Octos 启动时从 `profiles_dir` 加载它，用来把 bot 连到某个 LLM 后端 + 某个 Matrix homeserver：

```json
{
  "id": "botfather",
  "name": "BotFather",
  "enabled": true,
  "config": {
    "provider": "deepseek",
    "model": "deepseek-chat",
    "api_key_env": "DEEPSEEK_API_KEY",
    "admin_mode": true,
    "channels": [
      {
        "type": "matrix",
        "homeserver": "http://palpo-1:8008",
        "as_token": "436682e5f10a0113775779eb8fedf702a095254a95e229c7d20f085b9082903b",
        "hs_token": "ef642609a1a5b2eda1486a6bada6411f4e861691a7500b10ff26b5b2e16573fd",
        "server_name": "palpo-1:8448",
        "sender_localpart": "bot",
        "user_prefix": "bot_",
        "port": 8009,
        "allowed_senders": []
      }
    ],
    "gateway": {
      "max_history": 50,
      "queue_mode": "followup"
    }
  }
}
```

关键字段：

- **`provider` / `model` / `api_key_env`** — LLM 后端；换成其他 Octos 支持的 provider 时同时改 `.env` 里的 API key 环境变量
- **`admin_mode: true`** — 开启 Octos 的 admin 指令（和 #86 一致）
- **`channels[0].homeserver` vs `channels[0].server_name`** — 两个不同概念：
  - `homeserver = "http://palpo-1:8008"` — Octos 调 Client-Server API 的 HTTP URL
  - `server_name = "palpo-1:8448"` — Matrix 身份；必须和 palpo-1 的 `palpo.toml` 一致
- **`as_token` / `hs_token`** — 必须和 `nodes/node1/appservices/octos.yaml` **完全一致**，否则 palpo-1 拒绝 AppService 连接
- **`allowed_senders: []`** — 空数组表示所有人（包括联邦用户）都能 DM bot
- **`gateway.queue_mode: "followup"`** — Octos 并发对话的处理策略（`followup` 按房间维度串行回复）

`octos.json` 要简单得多 — 只告诉 Octos 到哪里找 profiles 和 data：

```json
{
  "profiles_dir": "/root/.octos/profiles",
  "data_dir": "/root/.octos",
  "log_level": "debug"
}
```

docker-compose 把 `config/botfather.json` 挂载进 `/root/.octos/profiles/`，Octos 启动时就自动发现它了。

---

## 5. 启动细节

> 快速开始已经覆盖基本启动流程。本节补充一些观察点和常见现象。

### 5.1 期望的启动顺序

```
1. palpo-pg-1 / palpo-pg-2   启动并通过 pg_isready 健康检查
2. palpo-1 / palpo-2         连接 postgres，监听 8008 + 8448
3. palpo-1                   加载 /var/palpo/appservices/octos.yaml
4. octos                     向 palpo-1 登录为 @bot:palpo-1:8448
```

### 5.2 健康检查和日志

```bash
# 5 个容器状态
docker compose ps
# palpo-pg-1 / palpo-pg-2  → healthy
# palpo-1 / palpo-2         → healthy
# octos                     → running

# palpo-2 成功联系 palpo-1（应该看到联邦握手）
docker compose logs palpo-2 | grep -i "palpo-1"

# Octos 登录成功
docker compose logs octos | grep -i "bot\|logged in"
```

### 5.3 首次构建时间

palpo 和 octos 都从源码编译，首次 `up -d --build` 可能花 5-10 分钟。之后重启只用 1-2 秒（除非改了源码）。构建产物走 Docker BuildKit 缓存，不会重复编译 crate。

### 5.4 磁盘占用与清理

联邦模式会跑 **两份** palpo 镜像、**两份** postgres 和 **一份** octos,占用比单机大:

| 阶段 | 大小 |
|---|---|
| 镜像(稳态,两份 palpo 共享图层) | ~3 GB |
| Build cache(首次构建峰值) | ~5 GB(可回收) |
| 运行数据(`data/node1` + `data/node2`) | 每节点 ~50-100 MB |

`docker system df` 显示可回收缓存太多时清理:

```bash
docker builder prune -af            # 清构建缓存(安全)
docker compose down -v              # 停容器 + 清数据卷
docker system prune -af --volumes   # 核选项:所有 Docker 相关内容
```

完整说明(为什么缓存会涨、应该选哪条命令)见 [01 §5.5 清理 Docker 缓存](01-deploying-palpo-and-octos-zh.md#55-清理-docker-缓存)。

---

## 6. 备选方案：API 层测试（CI / 无头脚本）

> **本节等价于 Quick Start，但用纯 HTTP 调用替代 Robrix UI。** 适合 CI 流水线、命令行复现问题、或想看清 Robrix 在底层到底发了哪些请求。如果只是交互式跑一遍 demo，[Quick Start](#-快速开始5-条命令跑起来) 已经够了，本节可以跳过。

### 6.1 用 curl 在 palpo-2 上注册 alice

等价于在 Robrix 里点 **Sign up** — 走的是 `palpo.toml` 里 `allow_registration = true` 开启的无需鉴权的 `m.login.dummy` 注册流：

```bash
curl -X POST http://localhost:6002/_matrix/client/v3/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "test1234",
    "auth": {"type": "m.login.dummy"}
  }'
```

预期返回：

```json
{
  "user_id": "@alice:palpo-2:8448",
  "access_token": "...",
  "home_server": "palpo-2:8448",
  ...
}
```

### 6.2 用 curl 验证 palpo-2 和 palpo-1 之间的联邦

从 palpo-2 查 palpo-1 上 bot 的 profile — 这会触发一次 server-to-server 握手，在连 UI 之前就能证明联邦通道通畅：

```bash
# 1) 用 alice 登录拿 token
TOKEN=$(curl -s -X POST http://localhost:6002/_matrix/client/v3/login \
  -H "Content-Type: application/json" \
  -d '{
    "type":"m.login.password",
    "identifier":{"type":"m.id.user","user":"alice"},
    "password":"test1234"
  }' | jq -r .access_token)

# 2) 通过 palpo-2 查 palpo-1 上 bot 的 profile（触发联邦）
curl -s "http://localhost:6002/_matrix/client/v3/profile/@bot:palpo-1:8448" \
  -H "Authorization: Bearer $TOKEN"
```

**预期结果：** 返回 `{"displayname": "...", "avatar_url": "..."}` 或空对象 `{}`。返回 `404` 说明联邦没通 — 看[第 8 节](#8-故障排查)。

### 6.3 和 bot 聊天

只要 6.1 注册好了 alice，6.2 证实联邦可用，后面的 DM 创建 + 聊天流程和 Quick Start 完全一致。跟着 [在 Robrix 里跨联邦和 bot 聊天](#在-robrix-里跨联邦和-bot-聊天) 走 UI 版就行（Add a friend → `@bot:palpo-1:8448` → 发 `hello`）。

如果 CI 里需要连 DM 创建也自动化，用 Matrix 的 [`POST /_matrix/client/v3/createRoom`](https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3createroom)，带上 6.1 里拿到的 token，body 里填 `invite: ["@bot:palpo-1:8448"]` + `is_direct: true` — 这会触发跟 Robrix "Add a friend" 按钮一样的跨联邦 MXID 查找。

---

## 7. 消息流详解

当 alice 给 bot 发 `hello` 时，消息经历如下路径：

```
┌─────────────────┐
│ Robrix (host)   │
│ @alice:palpo-2  │
└────────┬────────┘
         │ PUT /_matrix/client/v3/rooms/{id}/send/m.room.message
         │ 目标 http://localhost:6002
         ▼
┌─────────────────────────────────────────────────────┐
│ palpo-2 容器                                        │
│ 看到消息事件里有 @bot:palpo-1:8448                 │
│ server_name 部分是 "palpo-1:8448"                   │
│ 通过 Docker DNS 解析 palpo-1 → 容器 IP              │
└────────┬────────────────────────────────────────────┘
         │ PUT https://palpo-1:8448/_matrix/federation/v1/send/{txn}
         │ TLS（自签证书，allow_invalid=true 跳过验证）
         ▼
┌─────────────────────────────────────────────────────┐
│ palpo-1 容器（8448 TLS listener）                   │
│ 接收联邦事件                                        │
│ 检查 MXID 匹配 AppService namespace                 │
│   @bot:palpo-1:8448 匹配 octos.yaml 正则            │
└────────┬────────────────────────────────────────────┘
         │ PUT http://octos:8009/_matrix/app/v1/transactions/{txn}
         │ Authorization: Bearer <hs_token>
         ▼
┌─────────────────────────────────────────────────────┐
│ octos 容器                                          │
│ 解析事件，识别 "hello"                              │
│ 调用 DeepSeek API 生成回复                          │
└────────┬────────────────────────────────────────────┘
         │ PUT http://palpo-1:8008/_matrix/client/v3/rooms/{id}/send/...
         │ Authorization: Bearer <as_token>（bot 身份）
         ▼
┌─────────────────────────────────────────────────────┐
│ palpo-1 → 联邦回 palpo-2 → alice 的 Robrix 收到回复 │
└─────────────────────────────────────────────────────┘
```

**关键观察：**

1. Robrix 只认识 `localhost:6002`，它**感知不到**联邦的存在 -- 联邦是 palpo-2 内部完成的
2. 消息在 `palpo-2 → palpo-1` 之间走 TLS 联邦端口 8448，这是 Matrix 规范要求的
3. `palpo-1 → octos` 是 AppService HTTP，没有联邦概念 -- 对 palpo-1 来说 octos 就是本地的事件处理器
4. Octos 回复走的是 palpo-1 的 C-S API（它有 `as_token` 伪装成 bot 的身份发消息），不走联邦

---

## 8. 故障排查

### 8.1 诊断清单

| 症状 | 可能原因 | 查什么 |
|------|---------|--------|
| `docker compose up` 起不来 | 端口被占用 | `lsof -i :6001 :6002 :6401 :6402 :8009` |
| **首次 up 成功之后，Robrix 注册报 "Account Creation Failed" / 请求 hang** | 对应的 palpo 容器没在服务（`Restarting (127)` 或 `Exited`） | `docker compose ps` — palpo 如果不是 `healthy`，跑 `docker compose logs palpo-2 \| tail -30`。常见根因：运行时镜像里缺 `libgcc`（Rust 的 stack unwinding 需要 `libgcc_s.so.1`；本仓库 `palpo.Dockerfile` 已经 apk install 了 `libgcc`，千万别删）、`palpo.toml` 里证书路径写错、postgres 还没 healthy 时 palpo 就启动了 |
| Step 6.2 profile 查询返回 404 | 联邦未通 | `docker compose logs palpo-2 \| grep -i "fed\|palpo-1"` |
| 机器人能收消息但不回 | Octos → palpo-1 连接异常 | `docker compose logs octos \| tail -50` |
| Robrix 登录报 "invalid homeserver" | Homeserver URL 填错 | 必须是 `http://localhost:6002`，不是 `palpo-2:8448` |
| 创建 DM 时提示 "user not found" | 联邦 profile lookup 失败 | 查 palpo-2 日志看 TLS 握手和证书验证 |
| 消息发出去但没到 | 联邦异步队列堵塞 | `docker compose logs palpo-2 \| grep -i "send_txn\|backoff"` |
| 和 bot 建 DM 时弹出 "Failed to join: it has already been joined" | `/createRoom` 和 sliding sync 的竞态 | 无害 — DM 其实已经建好了，关掉弹窗即可。已被 PR #83（开着）修复 |

### 8.2 常用调试命令

```bash
# 查看全部服务日志（滚动）
docker compose logs -f

# 只看联邦相关日志
docker compose logs palpo-1 palpo-2 | grep -i "federation"

# 从容器内部测试 palpo-1 能否联系 palpo-2
docker compose exec palpo-1 curl -k https://palpo-2:8448/_matrix/federation/v1/version

# 查看 palpo-1 上的 AppService 注册状态
docker compose exec palpo-1 ls -la /var/palpo/appservices/

# 重启某个服务（不重启数据库）
docker compose restart palpo-1 octos

# 完全清掉重来（会删除所有用户和房间数据！）
docker compose down -v
```

### 8.3 验证 Octos 注册成功

```bash
# palpo-1 应该在启动日志里报告 AppService 已加载
docker compose logs palpo-1 | grep -i "appservice\|octos"

# Octos 启动后应该能用 bot token 访问 palpo-1
docker compose exec octos \
  curl -s -H "Authorization: Bearer 436682e5f10a0113775779eb8fedf702a095254a95e229c7d20f085b9082903b" \
  http://palpo-1:8008/_matrix/client/v3/account/whoami
# 期望：{"user_id":"@bot:palpo-1:8448",...}
```

---

## 9. 下一步

- **切到生产环境：** 本文档使用 Docker DNS 别名 + 自签证书，仅限本机测试。正式部署需要真实域名、Let's Encrypt 证书、反向代理等，参见 → [05-federation-production-deployment-zh.md](05-federation-production-deployment-zh.md)
- **和公共 Matrix 网络联邦：** 生产环境配置完成后，你可以和 `matrix.org` 等公共服务器互通。在公共房间里邀请你的 bot，或者让 `matrix.org` 用户主动来访。
- **扩展 Octos 能力：** bot 支持多种 LLM 后端、自定义指令、知识库等，参见 Octos 项目文档。

---

## 延伸阅读

- **Matrix 联邦规范：** [spec.matrix.org/latest/server-server-api](https://spec.matrix.org/latest/server-server-api/) -- Server-Server API 协议细节
- **AppService 规范：** [spec.matrix.org/latest/application-service-api](https://spec.matrix.org/latest/application-service-api/) -- AppService 通信协议
- **Palpo GitHub：** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) -- Palpo 源码和配置参考
- **Matrix Federation Tester：** [federationtester.matrix.org](https://federationtester.matrix.org/) -- 在线联邦配置检查工具（仅对公网域名有效）

---

*本指南基于 2026 年 4 月的 Palpo 和 Octos 版本。配置文件可能随上游更新而变化，以各项目仓库为准。*
