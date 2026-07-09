# 接入你自己的 Octos(Direct 模式)到 Robrix

[English](01-connecting-your-own-octos-to-robrix.md)

> **目标：** 完成本指南后，你将拥有一个跑在**自己机器上**的个人 Octos agent，以普通 Matrix 账号的身份登录你的服务器，并在 Robrix 里作为一个「Octos (Direct)」好友被添加、私聊或邀请进房间 —— 全程**无需**homeserver 管理员登记、无需公网 IP。

本指南配套一个**可直接运行的示例包**（[`example/`](example/)）：复制两个文件、填 4 个值、跑一条命令就能起来。

**快速索引**

| 你想做什么 | 跳转到 |
|---|---|
| 先搞懂 Direct 和 AppService 的区别 | [第 1 节](#1-direct-模式是什么) |
| 准备前置条件 | [第 2 节](#2-前置条件) |
| **直接用示例包跑起来** | [第 3 节](#3-可运行示例包) |
| 逐字段看懂每个配置项 | [第 4 节](#4-配置字段详解) |
| 在 Robrix 里添加它 | [第 5 节](#5-在-robrix-中接入) |
| Bot 不回复怎么办 | [第 6 节](#6-故障排查) |

---

## 1. Direct 模式是什么

Octos 有两种接入 Matrix 的方式，区别只在 channel 配置里的一个 `mode` 字段：

| | **Octos (Direct)** — 本指南 | Octos AppService |
|---|---|---|
| Matrix 身份 | 一个**普通用户账号**（`user_id` + 密码） | 服务器登记的 AppService（bridge 身份） |
| 服务器要求 | **无** —— 有个能登录的账号即可 | 需要 homeserver 管理员登记 AppService YAML |
| 网络方向 | **出站** `/sync`（笔记本连得出去就行，不需公网 IP） | 通常需要 homeserver 能入站访问到 bridge |
| 部署位置 | 任意机器（对 Robrix 不可见） | 通常与 homeserver 同侧 |
| BotFather / 子 bot | 不支持（单账号 = 单 bot） | 支持动态创建子 bot |
| 在 Robrix 里如何加 | 「添加 agent」→ 选 **Octos (Direct)** → 填 Matrix ID → **Add friend & bind** | 「添加 agent」→ 选 **Octos** → 填 AppService URL |

> **一句话选型：** 想在**自己电脑**上跑一个私人 agent、又没有 homeserver 管理权限 → 用 **Direct**。想在服务器侧提供多 bot 平台、有 BotFather 需求 → 用 AppService（见 [Robrix + Palpo + Octos 部署指南](../robrix-with-palpo-and-octos/01-deploying-palpo-and-octos-zh.md)）。

Direct 模式在 Robrix 侧走的是和 **Hermes / OpenClaw 完全相同的 direct-friend 绑定路径** —— 它就是一个 Matrix 好友，Robrix 通过 homeserver 与它交互，**不关心它跑在哪台机器**。

---

## 2. 前置条件

开始前确认：

- [ ] **一个 Octos 可执行文件**（`octos` 二进制，版本需包含 Matrix user-account channel，即 octos PR #1475 之后）
- [ ] **一个给 bot 专用的 Matrix 账号** —— 在你的 homeserver 上注册好，拿到 `user_id` 和密码（**不要**用你本人的账号）
- [ ] **一个 LLM 的 API Key** —— 本指南以 DeepSeek 为例（`DEEPSEEK_API_KEY`）
- [ ] **Robrix 已安装**，并能连接到**同一个** Matrix 服务器
- [ ] bot 账号和你打算聊天的账号，**都不在加密房里**（见 [第 6 节](#6-故障排查) 关于 E2EE 的说明）

> **提示：** 「给 bot 专用的账号」是关键。Direct 模式本质是「用一个 Matrix 账号登录并代跑 agent」，所以这个账号会以 bot 身份收发消息 —— 用独立账号，别和你自己的身份混用。

---

## 3. 可运行示例包

配套目录 [`example/`](example/) 里是一套最小可跑的文件：

| 文件 | 是什么 |
|---|---|
| [`myagent.example.json`](example/myagent.example.json) | Octos gateway **profile**（`--profile` 加载的档案） |
| [`.env.example`](example/.env.example) | 存放 `DEEPSEEK_API_KEY` |
| [`start.sh`](example/start.sh) | 加载 `.env`、设好代理排除、跑 `octos gateway` |

### 3.1 三步跑起来

```bash
cd example

# 1. 生成你的 profile,编辑其中 4 个标注的值
cp myagent.example.json myagent.json
#    改:homeserver、server_name、user_id、password

# 2. 生成 env 文件,填入你的 LLM key
cp .env.example .env
#    改:DEEPSEEK_API_KEY

# 3. 启动(前台运行,Ctrl-C 停止)
./start.sh
```

登录成功的标志是输出里出现这一行：

```
INFO Matrix user channel authenticated user_id=@myagent:example.org
```

> **需要改 5 个值**（都在 `myagent.json` 的 channel 里）：`homeserver`、`server_name`、`user_id`、`password`，以及 **`allowed_senders`（填你自己的 MXID —— 谁能驱使这个 agent，见 [§4](#4-配置字段详解) 的安全说明）**。其余字段（`llm`、`created_at`/`updated_at`、以及为「个人助手」预设好的 `auto_join`/`group_policy`/`require_mention`）保持原样即可 —— 下一节解释它们为什么这么设。

---

## 4. 配置字段详解

真正让它变成「Direct 模式」的是 `channels` 数组里那个对象。**下面每个字段名都对应 Octos 用户通道的真实设置项，取值和默认值以 Octos 源码为准**（不是随手写的）：

| 字段 | 作用 | 取值 / **默认** |
|---|---|---|
| `type` | 通道类型 | 固定 `"matrix"` |
| `mode` | **决定 Direct vs AppService** | `"user"` = Direct（本指南）；**省略/`"appservice"`** = AppService（会要求 `as_token`/`hs_token`，否则报错） |
| `homeserver` | CS-API 地址（带 scheme 和端口） | 如 `https://matrix.example.org`；**默认 `http://localhost:6167`** |
| `server_name` | MXID 里冒号后的域 | 如 `example.org`（自建常是 `192.168.1.58:8128`） |
| **认证（二选一）** | 缺失会报 *requires access_token or user_id + password* | `access_token`；**或** `user_id` + `password` |
| `device_name` | 登录设备名（日志/会话里可见） | 任意，如 `octos-personal` |
| `auto_join` | 收到邀请是否自动加入 | `always`/`on`/`true` 全接受；`allowlist`/`allowed` 仅白名单；**默认 `off`（不自动加入）** |
| `auto_join_allowlist` | 配合 `allowlist` 的白名单 | 数组或逗号串，可选 |
| `group_policy` | 群聊授权策略 | `open`/`all` 全响应；`disabled`/`off`/`false` 关闭；**默认 `allowlist`** |
| `require_mention` | 群里是否必须 @ 才回复 | **默认 `true`**（群里需 @）；个人助手设 `false` |
| `allowed_senders` | **允许驱使 bot 的发送者白名单** | 你自己的 MXID 数组，如 `["@you:example.org"]`；**留空 `[]` = 所在房间的任何人都能用** |

> **⚠️ 默认值对个人助手是「静默不工作」的。** Octos 的三个默认（`auto_join=off`、`group_policy=allowlist`、`require_mention=true`）合起来的效果是：不自动进房、群里不在白名单不响应、必须 @ 才回。所以示例里显式把它们翻成 `always` / `open` / `false` —— 这正是「个人助手」该有的行为。想收紧权限时再逐个改回。

> **🔒 `allowed_senders` 是你最重要的安全闸门 —— 务必填你自己的 MXID。** 上面三个字段一放开（`always`/`open`/`false`），这个 agent 就会自动进房、有问必答。此时**唯一决定「谁能驱使它、花你 LLM API 额度」的就是 `allowed_senders`**：
>
> - 三层访问控制是正交的：`auto_join` 管**接受谁的邀请**、`group_policy` 管**在哪些房活跃**、`allowed_senders` 管**允许谁让它回复**。前两个开得越大，`allowed_senders` 越是唯一的防线。
> - `["@you:example.org"]`（示例默认）= **只有你**能用它；给信任的同事就追加 MXID：`["@you:example.org", "@teammate:example.org"]`。
> - `[]`（留空）= 所在房间的**任何人**都能用它。在**联邦或公共服务器**上，这意味着任何知道 bot MXID 的陌生人都能把它拉进房、烧你的 API 额度 —— 只在**私有可信**服务器上才用 `[]`。

其余字段：

- `enabled` — **必须为 `true`**（默认 `false` = 禁用该 profile）
- `created_at` / `updated_at` — profile 元数据，**加载时必填**，保持示例里的值即可（任意合法 RFC 3339 时间都行）
- `llm.primary.{family_id, model_id, route.api_key_env}` — 模型路由；注意 `start.sh` 里的 CLI `--provider` / `--model` 会覆盖它
- `gateway.max_history` / `queue_mode` / `max_output_tokens` — 会话行为调优，可选

> **安全提示：** `myagent.json` 里的 `password` 和 `.env` 里的 key 都是明文 —— **别提交进版本库**，只提交 `*.example` 模板。若你的 Octos 版本支持，用 `access_token` 替代 `password` 更安全。

---

## 5. 在 Robrix 中接入

octos gateway 跑起来、bot 已登录后，到 Robrix 里把它登记为一个 agent：

### 5.1 添加 agent（Agent Lab 两步向导）

1. 打开 Robrix，用**你自己的账号**登录
2. 进入 **Settings → Labs → Agent Access**
3. 点 **「添加 agent」**
4. **Step 1 of 2 · Choose a framework** —— 选 **「Octos (Direct)」** 那张卡片（角标 `OD`，标签「直接 Agent」）
5. **Step 2** —— 在 **Agent Matrix ID** 里填你 bot 的完整 MXID（如 `@myagent:example.org`），点 **「Add friend & bind」**

<img src="images/add-agent-octos-direct.png" width="360" alt="Robrix「添加 agent」弹窗 Step 1:四张框架卡片中选中「Octos (Direct)」(DIRECT AGENT,Octos, added as a Matrix friend.)">

Robrix 会向该 MXID 发一个好友请求。因为配置了 `auto_join: "always"`，octos bot 会**自动接受**，随后它就作为一个已识别的 **Octos (Direct)** agent 出现在列表里。

> **提示：** 这一步是**客户端本地动作**（在 AgentRegistry 里登记 + 建私聊），不是发给 bot 的聊天命令。它和 Hermes / OpenClaw 的绑定流程完全一样。

### 5.2 私聊或邀请进房

登记后，它就是一个普通 Matrix 好友：

- **私聊：** 直接进和它的会话，发消息即可（`require_mention: false` 时私聊里它回复所有消息）
- **邀请进房：** 在任意房间邀请它的 MXID，它会自动加入。群里是否需要 @ 取决于 `require_mention`

Robrix 会在房间里给它显示 **Octos (Direct)** 的框架标识，让你一眼认出这是个 agent 而非真人。

---

## 6. 故障排查

| 症状 | 常见原因 | 处理 |
|---|---|---|
| 日志里 `/sync` 报 **502 Bad Gateway** | shell 代理把发往 homeserver 的请求也代理了 | 用 `start.sh` 里的 `NO_PROXY` 排除 homeserver 地址 |
| bot 登录失败，报 *requires access_token or user_id + password* | 认证字段没填全 | `access_token`，或 `user_id` + `password` 二选一填全 |
| profile 加载失败 | 少了 `created_at`/`updated_at`，或 `enabled` 没设 `true` | 照示例补齐这三个字段 |
| 私聊有反应、**群里不回** | `require_mention: true`（默认） | 群里 @ 它，或把 `require_mention` 设为 `false` |
| **谁发都不回** | 你的 MXID 不在 `allowed_senders` 里；或 `group_policy` 是默认 `allowlist`；或没自动进房 | 把你自己的 MXID 加进 `allowed_senders`；设 `group_policy: "open"`、`auto_join: "always"` |
| 加密房里读不到历史消息 | E2EE 密钥没分发给 bot 新设备 | 发**新消息**即可；本流程建议一律用**非加密房** |
| Robrix 里加不上 | Step 1 选成了 **Octos**（AppService）而非 **Octos (Direct)** | 回退重选 Direct 卡片，Step 2 才会是「Agent Matrix ID + Add friend & bind」 |

> **关于代理陷阱：** 如果 octos 跑在设了全局代理（Clash 等）的机器上，它连**本地或 LAN 上的 homeserver** 时会把 `/sync` 也走代理 → 报 502 → 收不到消息。`start.sh` 用 `NO_PROXY` **只排除 homeserver 地址**，发往外网 LLM 的请求仍走代理。把脚本里的 `matrix.example.org` 换成你 homeserver 的 host/IP。

> **关于加密房：** Direct 模式的 bot 是独立 Matrix 账号，读不了它加入前的加密历史，某些自建服务器的解密扩展也不覆盖它。**建议 Direct agent 一律在非加密房使用。**

---

## 接下来

- [示例包 `example/`](example/) —— 复制即用的 `myagent.example.json` + `start.sh` + `.env.example`
- [Robrix + Palpo + Octos 部署指南](../robrix-with-palpo-and-octos/01-deploying-palpo-and-octos-zh.md) —— 如果你想要的是**服务器侧 AppService + BotFather** 多 bot 平台
- [Robrix + OpenClaw](../robrix-with-openclaw/02-using-robrix-with-openclaw-zh.md) —— 另一个 direct-friend agent，接入方式与本指南一致

---

*本指南基于 2026 年 7 月的 Octos user-account channel（PR #1475）与 Robrix 的 Octos (Direct) 接入方式编写。配置字段以各项目仓库为准。*
