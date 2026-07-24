# 本地部署：Palpo + agent-chat + Robrix2

> **定位**：本章把三个组件部署在本地并验证单 Agent 链路。前置依赖：第 4 章路线选择。Linux 是正式安装路径；macOS 当前是开发运行路径。

部署完成后，你机器上的进程拓扑是：

```mermaid
flowchart LR
    subgraph docker["Docker（palpo-and-octos-deploy）"]
        PG[("PostgreSQL")]
        PALPO["Palpo :8128"]
        PALPO --- PG
    end
    subgraph local["本机进程"]
        ROBRIX["Robrix2（cargo run）"]
        BE["agent-chat backend :8090"]
        DASH["dashboard :8084"]
        BRIDGE["bridge-matrix.js"]
        TMUX["tmux: Claude Code / Codex"]
    end
    ROBRIX -->|登录 http://127.0.0.1:8128| PALPO
    BRIDGE <--> PALPO
    BRIDGE <--> BE
    BE --- DASH
    BE <-->|MCP + 通知| TMUX
```

## 1. 启动 Palpo（Matrix homeserver）

Robrix2 仓库自带一套开箱即用的 Docker Compose 部署（`palpo-and-octos-deploy/`），包含 PostgreSQL + 从源码构建的 Palpo（支持 x86_64 / ARM64）：

```bash
cd robrix2/palpo-and-octos-deploy

./setup.sh
docker compose up -d palpo_postgres palpo
docker compose logs -f palpo
```

默认配置（`palpo.toml`）：

- Client-Server API 监听 `http://127.0.0.1:8128`（Robrix2 连这里）；
- `server_name` 默认 `127.0.0.1:8128`，正式使用建议改成你的域名；
- **开放注册**，方便本地测试时创建人类、桥机器人和 Agent 木偶账号。

> 这里显式只启动 `palpo_postgres` 和 `palpo`，避免把 Octos 的模型 API 配置混进 HAgency 冒烟测试。开放注册只适合环回地址或受信开发网络；不要把这份测试配置直接暴露到公网。

**验证**：`curl http://127.0.0.1:8128/_matrix/client/versions` 返回版本列表即为就绪。

也可以不用 Docker，按 [Palpo 仓库](https://github.com/palpo-im/palpo) 的说明用 `cargo` 构建运行；agent-chat 只要求「一个可用的 Matrix 服务器」。

## 2. 配置并启动 agent-chat

前置要求：**Node.js 22+**、**tmux**，以及至少一个编码运行时（Claude Code 或 Codex CLI）。

```bash
git clone https://github.com/ZhangHanDong/agent-chat.git
cd agent-chat
npm install
cp .env.example .env
```

先生成三个独立 secret：

```bash
openssl rand -hex 32  # 填 API_TOKEN
openssl rand -hex 32  # 填 MATRIX_BRIDGE_SECRET
openssl rand -hex 32  # 也建议为 MATRIX_AGENT_PASSWORD_SECRET 生成独立值
```

`.env` 至少要显式设置以下项目。backend 与 bridge 必须读取**同一份** `.env` 和同一个 `MATRIX_BRIDGE_SECRET`：

```dotenv
API_TOKEN=<随机长 token>
MATRIX_HOMESERVER=http://127.0.0.1:8128
MATRIX_SERVER_NAME=127.0.0.1:8128
MATRIX_BOT_USERNAME=agent-bridge-alexlocal
MATRIX_BOT_PASSWORD=<桥账号密码>
MATRIX_AGENT_PASSWORD_SECRET=<另一个随机长 secret>
MATRIX_BRIDGE_SECRET=<backend 与 bridge 共享 secret>

MATRIX_TRUST_MODE=enforce
MATRIX_TRUSTED_INVITER_MXIDS=@alex:127.0.0.1:8128
MATRIX_OPERATOR_MXIDS=@alex:127.0.0.1:8128
MATRIX_DEFAULT_WAKE=off
```

不要只写 display name 或 `alex`；安全边界使用完整 MXID。`MATRIX_OPERATOR_MXIDS` / `MATRIX_ADMIN_MXIDS` 不能都留空，因为管理命令 ACL 为兼容旧部署存在 `no_acl` 回退。关闭 homeserver 注册时，还要预创建桥账号和每个 `@ac_*` 账号，或配置服务器支持的 registration token。

### Linux：正式安装路径

```bash
./install-full.sh --with-bridge
systemctl --user status agent-chat-v2 agent-chat agent-chat-push-relay bridge-matrix 2>/dev/null \
  || systemctl status agent-chat-v2 agent-chat agent-chat-push-relay bridge-matrix
```

安装器当前是 Linux/systemd 路径。是否使用 user unit 取决于你的安装选项，以安装器输出为准。

### macOS：开发运行路径

macOS 目前没有等价的完整安装器。先清除所有 `<...>` 占位符，并对含 shell 特殊字符的值正确加引号；然后在四个终端中从同一环境分别运行：

```bash
set -a; source .env; set +a
node backend-v2.js
node server.js
PUSH_RELAY_MODE=local node push-relay.js
node bridge-matrix.js
```

上面四条分别是 backend `:8090`、dashboard `:8084`、本地通知 relay 和 Matrix bridge；每条都必须继承同一份环境。已有本地 supervisor/LaunchAgent 配置时可以使用 `bin/agentchat service restart --profile local`，但 clone + `npm install` 本身不会在 macOS 创建这些服务。

**验证基础服务**：

```bash
curl --noproxy '*' http://127.0.0.1:8090/health   # backend 健康检查
open http://127.0.0.1:8084                        # 本地监控面板
```

### 给 Agent 绑定本地项目并启动

推荐先声明项目边界，再启动运行时：

```bash
bin/agentchat project add wf_coordinator /absolute/path/to/my-project --mode symlink
bin/agentchat project list wf_coordinator
bin/agentchat up wf_coordinator /absolute/path/to/my-project claude
bin/agentchat ls
```

`symlink` 会让 Agent 直接写源仓库；`copy` 是隔离副本，不会把修改直接写回源目录。只把需要的项目路径加入 Agent，避免把整个工作目录或 home 暴露进去。模型可在启动时用 `--model <model>` 选择，变更模型需要重启该运行时；Robrix2 里的自然语言按任务选模型目前还没有接通。

受管启动会注册 `@ac_<name>` 木偶账号、接入 MCP，并固定运行时权限策略：Claude Code 使用 `--permission-mode auto` 与审批 channel，Codex 使用 `workspace-write` + `on-request` hook。Codex 第一次 `up` 必须在本地 TTY 输入一次 `TRUST`；不要手工改 trust state，也不要在 tmux 里直接重开一个未带受管参数的 CLI。

## 3. 启动 Robrix2

workflow 命令面板等 agent-chat 集成功能由 `agent_chat` Cargo feature 提供（默认不编译），所以带上 feature 构建：

```bash
cd robrix2
cargo run --features agent_chat
```

登录界面中：**Homeserver** 填 `http://127.0.0.1:8128`，注册 / 登录你的人类账号（例如 `@alex:127.0.0.1:8128`）。

登录后还需打开一次运行时开关：**Settings → Preferences → Enable agent-chat support**。编译期 feature + 运行时开关是有意的双重门控 —— 不需要 Agent 功能的用户拿到的是一个纯粹的 IM。

## 4. 创建项目房间并建立 owner

1. **创建 backend group**：

   ```bash
   bin/agentchat cli create-group robrix2-board wf_coordinator
   ```

   **当前 bootstrap 限制**：bridge 观察到新 group 后会自动创建同名 Matrix 房并让 Agent 加入；目前没有 “backend group only / no room” 开关。自动房里的 Agent 邀请者是 bridge，不会建立人类 owner。正式发布应先补这个模式或受校验的 owner-claim 流程。

2. **选择非加密项目房间**：

   - 加入同事已有项目房：在那个房间邀请自己的 bridge，然后发送 `!bindroom robrix2-board`；新 group 自动创建的同名房只是多余房间，不要拿它做审批验收；
   - 全新单人测试：也可以先在 bridge DM 发送 `!mkgroup robrix2-board wf_coordinator` 并接受自动房邀请，然后执行下面的“移除再由人邀请”步骤。

   当前版本不要开启项目房 E2EE。绑定已有房时，以 operator 身份发送：

   ```text
   !bindroom robrix2-board
   ```

   `!bindroom` 只建立 `room → group`，**不会邀请 Agent，也不会建立 owner**。

3. **由 owner 亲自邀请实际 Agent**：

   - 在已有项目房中，直接用同一个人类账号邀请准确的 `@ac_wf_coordinator:<server_name>`；
   - 在 `!mkgroup` 自动房中，Agent 已由 bridge 加入。先在房内执行 `!rmember wf_coordinator`，确认木偶离开，再由人类账号重新邀请它；Matrix→backend reconcile 会把它加回 group。

   bridge 从这条人类发出的 membership invite 的真实 `event.sender` 建立：

   ```text
   (project_room_id, wf_coordinator) → @alex:127.0.0.1:8128
   ```

   **谁邀请这个 Agent，谁就是它在这个房间里的 owner。** 这是审批授权来源，不是 UI 约定。邀请轮询默认可能约 60 秒；Agent 加入后会邀请自己的 companion bridge。

4. **接受审批房邀请**：bridge 会创建或复用 `Approval: wf_coordinator` E2EE 房并邀请 owner。接受它；未接受时审批状态会是 `owner_invite_pending`。

5. **冒烟测试**：在作战室里显式 `@wf_coordinator`。默认 `MATRIX_DEFAULT_WAKE=off` 下，未 @ 的房间消息只被记录，不唤醒 Agent。收到木偶回复后，再触发一条需要审批的受保护命令，确认项目房只显示脱敏等待状态、详细卡片只在审批房出现。

## 常见问题定位

| 症状 | 先查哪里 |
|------|---------|
| Robrix2 登录失败 | Palpo 容器日志；homeserver 地址是否带对端口 |
| bridge 无法启动 | `API_TOKEN`、`MATRIX_BRIDGE_SECRET`、bot 密码与 Agent 密码 secret 是否非空；backend/bridge 是否读取同一 `.env` |
| **桥对房间消息完全无反应** | 确认 trust mode 为 enforce、trusted inviter 是邀请桥的完整 MXID；看 bridge trust 日志 |
| `!bindroom` 回复 Group not found | 先 `agentchat cli create-group` 创建 group |
| `!bindroom` 没有权限 | 发送者不在 `MATRIX_OPERATOR_MXIDS` 里 |
| @Agent 没反应 | Agent 是否真的在房间；`agentchat ls` 是否在线；bridge 是否收到了 explicit mention；push relay 是否健康 |
| `/` 面板里没有 workflow 命令 | 是否 `--features agent_chat` 构建 + 打开了 Preferences 开关；房间里是否有 `*_coordinator` |
| 审批立即拒绝 / 卡片不出现 | 是否存在唯一 owner binding；审批房邀请是否接受；运行时是否受管启动；backend 是否创建 pending record |

完整分层定位见[运行验收与故障排查](operations.md)。下一步：[团队协作实战](collab-overview.md)。
