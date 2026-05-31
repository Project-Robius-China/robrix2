# Robrix2 × agent-chat Demo 集成设计(基于本地 Palpo,纯文本版)

> 分支:`analyze/agent-chat-integration`
> 目标:用 robrix2 客户端 + agent-chat 多 agent 系统,在**本地 Palpo** 上跑通
> `issue → spec → plan → 实现 → 对抗审查` 工作流。**demo 先不用富 UI**(不接 `org.octos.*` 审批卡片/按钮),全程标准 `m.room.message` 文本/Markdown。

---

## 0. 一句话结论

robrix2 与 agent-chat **没有任何代码级耦合**(双向 grep 均为 0),它们只在 **Matrix 协议层**相遇。
做这个 demo **不需要改 robrix2 的 Rust 代码**:robrix2 只当一个普通 Matrix 客户端用,
全部工作流逻辑放在 **agent-chat 的协调 agent(coordinator skill)** + 已安装的 `agent-spec` CLI 里。
唯一要写的「新代码」是一个**给协调 agent 用的 skill**(Markdown + 调用现有 CLI),外加几处**配置**。

---

## 1. 现状(集成事实,均经双代码库一手验证)

| 维度 | robrix2(Rust/Makepad) | agent-chat(Node) |
|---|---|---|
| 与对方的代码引用 | 0(`src/`+`Cargo.*` 无 `agent-chat`) | 0(repo 内无 `robrix`) |
| Matrix 角色 | 普通 client(matrix-sdk,sliding sync) | 普通 client(matrix-bot-sdk),**非 appservice** |
| 自定义事件 | 实现了 `org.octos.*`(审批/按钮),**为 OctOS appservice 设计** | **完全不发自定义事件**,只发 `m.text/m.image/m.file`(+`org.matrix.custom.html`) |
| 身份 | 登录人类账号 | bot 账号登录 + `@ac_<name>` agent 账号(注册令牌/开放注册建号) |
| 命令前缀 | `/`(`/createbot /listbots`…,纯文本发给 botfather) | `!`(`!mkgroup !dm !status`…,bridge 级命令) |
| 编排后端 | 无 | backend `:8090`(REST+SSE)、每 agent 一个 MCP server、tmux 跑 Claude Code/Codex |

> 重要推论:robrix2 的审批卡片/动作按钮是 **robrix2 ↔ OctOS(appservice)** 的私有协议,
> **agent-chat 不会触发它**。所以本 demo 走纯文本,完全绕开这块。

### agent-chat 的 agent 运行底座(纯文本即可驱动闭环)
- 每个 agent = 一个 tmux pane 里的 Claude Code/Codex 会话,带专属 MCP server(`lib/mcp-server-core.js`),工具:
  - `whoami` / `send_message(to, summary, full)` / `check_inbox(kinds?)`
- 新消息到达 → `push-relay` 往该 agent 的 tmux 注入 `[NOTIFICATION] FIRST ACTION: call check_inbox() now…`
  → agent 读收件箱 → 行动 → `send_message` 回复。
- **agent 之间互发走同一条路**(`send_message(to="reviewer", …)`)→ 天然支持「实现 agent 与审查 agent 对抗」。
- CLI:`agent-up-v1`(起 agent)、`agent-send`、`agent-ls`、`agent-project`、`register-agents`、`agentchat-sync-skills`(把 skill 同步进 agent)。

### 本地 Palpo 事实(`palpo-and-octos-deploy/`,已在 OrbStack 实测)
- compose 端口映射 `8128:8008`(host:container)→ **主机侧一律连 `http://127.0.0.1:8128`**;容器内 `:8008` 仅内部用。
- `server_name = "127.0.0.1:8128"`,`[well_known] client = "http://127.0.0.1:8128"` → 三处一致,**无端口/域割裂,不需要 well-known 折腾**(官方文档 02 也用此 URL 登录)。
- **实测**(OrbStack docker):`curl http://127.0.0.1:8128/_matrix/client/versions` 正常返回;well-known 返回 `{"m.homeserver":{"base_url":"http://127.0.0.1:8128"}}`。
- `allow_registration = true` 且开放注册标志已开 → agent-chat 建 `@ac_*` 账号**无需额外令牌**。
- Octos appservice(`@octosbot`,push-based,as_token/hs_token)与本 demo **是两套、无关**,可共存——本项目 `docs/robrix-with-palpo-and-octos/` 讲的就是 OctOS 那条路径。

---

## 2. Demo 目标链路

```
                         本地 Palpo homeserver
                    (CS-API http://127.0.0.1:8128,
                     server_name 127.0.0.1:8128)
                              ▲   ▲   ▲
        robrix2 (人类)────────┘   │   └──────── @agent-bridge (bot, 中继)
                                  │
                 @ac_coordinator / @ac_implementer / @ac_reviewer
                                  │
                  agent-chat backend :8090  ←SSE/REST→  bridge-matrix.js
                                  │
                push-relay → tmux panes (Claude Code/Codex 会话)
                                  │
              agent-spec CLI · file-issue skill · 仓库 issues//specs/
```

- **一个房间 = 一个项目看板**。robrix2 建房,邀请 `@agent-bridge`(中继必须在场)+ 三个 agent。
- 人类在房间里用斜杠命令(纯文本)发指令;协调 agent 解析并编排;所有进度以文本/Markdown 回帖。

---

## 3. 五步工作流 → 角色落点

| 步骤 | 谁做 | 用什么(全部已存在) | 产物 |
|---|---|---|---|
| 1. 建 issue | coordinator | `file-issue` skill / 直接写 `issues/NNN-*.md` | issue 文件 + 回帖确认 |
| 2. issue→spec | coordinator | `agent-spec`(已装 `~/.cargo/bin/agent-spec 0.2.7`):`agent-spec parse` + `agent-spec lint --min-score 0.7` | `specs/task-*.spec.md`,回帖摘要 + 「回复 `approve` 确认」(默认确认) |
| 3. spec→plan | planner(或 coordinator 兼) | `superpowers-writing-plans` 思路 + spec | 执行计划(贴在房间 / 写 `docs/plans/`) |
| 4. 实现 | implementer | `send_message` 派活;agent 在自己的 tmux/项目里改代码 | 代码改动 + 回帖 |
| 5. 对抗审查 | reviewer | coordinator `send_message(to="reviewer", …)` 让其独立审查实现;不通过则打回 implementer | 审查结论 + 通过/打回 |

> 最小 agent 集:**coordinator + implementer + reviewer** 三个即可演示「派发 + 对抗」。
> 想更省,可只起 1 个 coordinator,内部用 Claude 的子 agent 扮演 implementer/reviewer——但**两个独立 `@ac_*` 账号在房间里你来我往**的画面 demo 效果最好。

---

## 4. 命令设计(纯文本,人在 robrix2 里发)

约定(已核对 bridge 路由 + 端到端审计):
- **先建 group(关键!):** robrix2 随手建的房间**不是** agent-chat 的 group,agent 自己也建不了 group。
  在有 bot 的房里发 bridge 命令 **`!mkgroup demoboard coordinator implementer reviewer`** ——
  bridge 会建后端 group + **新建一个 Matrix 群房 `demoboard`** 并邀请你和 3 个 agent。进这个群房演示。
- **群里,只有被 `@提及` 的 agent 才进 inbox**;`@mention` 匹配 agent 的**短名**(`@coordinator`),不是 `@ac_coordinator` MXID。
- coordinator 用 **`post(group="demoboard", …)`** 把进度回帖到群 → 整条 coordinator↔implementer↔reviewer 流水线在 robrix2 里可见。
- (备选)与 coordinator 的 **1:1 DM 房**每条直达、无需 @,最稳;但 agent 间用 `send_message` 互发是走后端、**不经房间,人看不到**——所以要"可演示",用 group 模式。
- group 名 coordinator **运行时从触发消息的 `group` 字段学到**,不硬编码。

```
@coordinator /create-issue <标题> | <描述>     # 步骤1:建 issue
@coordinator /status                            # 查看当前工作流状态/各步进度
@coordinator /spec <issue-id>                   # 步骤2:为某 issue 生成 spec(可由 create-issue 自动串起)
approve                                          # 通过当前 spec 门禁(默认确认)
@coordinator /go <issue-id>                      # 步骤3-5:plan→实现→审查 一条龙
@coordinator /review <issue-id>                  # 单独触发对抗审查
```

实现要点:这些 `/xxx` 对 agent-chat 而言就是普通文本,**由 coordinator skill 的提示词解析**(不是 robrix2、也不是 bridge 的 `!` 命令)。所以加命令 = 改 skill 文本,零编译。

---

## 5. 落地清单(按改动归属分块)

### A. Palpo(基本就绪,确认即可)
- [ ] 确认 robrix2 与 agent-chat 都连 `http://127.0.0.1:8128`(compose 映射 `8128:8008`),MXID 域同为 `127.0.0.1:8128`(三处一致,无需 well-known)。
- [ ] 开放注册已开 → 不必配注册令牌(若改回 invite-only,则需 `MATRIX_REG_TOKEN`)。

### B. agent-chat 配置(`.env`,无代码改动)
```ini
API_TOKEN=<任意强口令>
MATRIX_BRIDGE_SECRET=<bridge↔backend 同一个值>   # 必填!`!mkgroup`(POST /api/groups)要它,否则建群被拒;.env.example 里没有,要自己加
MATRIX_HOMESERVER=http://127.0.0.1:8128   # 主机端口(compose 映射 8128:8008),与 server_name 一致
MATRIX_SERVER_NAME=127.0.0.1:8128
MATRIX_BOT_USERNAME=agent-bridge
MATRIX_BOT_PASSWORD=<bot 密码;bridge 首启会自动注册该账号>
MATRIX_AGENT_PREFIX=ac_
MATRIX_REG_TOKEN=               # 开放注册下可留空
MATRIX_AGENT_PASSWORD_SECRET=<长随机串,稳定派生 agent 账号密码>
MATRIX_TRUST_MODE=audit         # demo 用 audit:被邀请即自动 join;严格可用 enforce + 下行白名单
MATRIX_TRUSTED_INVITER_MXIDS=@<你的robrix用户>:127.0.0.1:8128
```
> 完整可用版见 `roadmap/agentchat-demo/agent-chat.env.demo`。**`MATRIX_BRIDGE_SECRET` 是审计抓到的漏项**:没有它 `!mkgroup` 建群会失败,coordinator 就 `post` 不进房间。
- [ ] **单独启动 bridge**(它不是 agent 的参数):`node bridge-matrix.js`(或装好的 `bridge-matrix.service`)。
      ⚠️ 校验发现:`agent-up-v1` **没有 `--with-bridge`**(传了会报错退出);`--with-bridge` 只是 `install-full.sh` 的**安装期**开关。
- [ ] 起 backend(`node backend-v2.js`)+ push-relay(`PUSH_RELAY_MODE=local node push-relay.js`)。
- [ ] 起 3 个 agent(**没有 role 参数,角色靠名字**):
      `agentchat up-v1 coordinator claude --project <repo> --project-mode symlink`
      (implementer / reviewer 同理,后两个加 `--allow-shared-workspace` 共享同一 workspace)。
- [ ] 同步 skill:`agentchat-sync-skills` **只会同步 `agent-chat` 这一个 skill**,
      所以新 skill 用 `roadmap/agentchat-demo/link-skill.sh` 手动 symlink 进 `~/.claude/skills` 和 `~/.codex/skills`。

### C. 共享 skill `issue-workflow`(**唯一要新写的东西**,纯 Markdown + 调 CLI)
> 校验发现:agent-chat **没有按 agent 注入 system prompt 的机制**;`agent-up-v1` 无 `--role`。
> 因此**不写 3 个 skill,而是 1 个共享 skill,内部按 `whoami` 名字分支**
> (coordinator / implementer / reviewer)。三个 agent 都加载它,行为按名字不同。

已写好:`roadmap/agentchat-demo/issue-workflow/SKILL.md`(用真实 MCP 工具名/参数 + 真实 `agent-spec` 调用)。职责概要:
1. **coordinator**:`check_inbox` 解析 `/create-issue` → 写 `issues/` + 起草 spec → `agent-spec parse` + `lint --min-score 0.7` → `post(group,…)` 回帖请求 `approve`(仅认 issue 发起人,默认确认可配)。
2. 通过后生成 `docs/plans/` 执行计划 → `send_message(to="implementer", …)` 派实现 → 完成后 `send_message(to="reviewer", …)` 派对抗审查 → 聚合结论 `post` 回房间;不过则打回 implementer(最多 3 轮)。
3. **implementer**:读 spec+plan 在共享 workspace 改码 → `send_message(to="coordinator", …)` 回 diff 摘要。
4. **reviewer**:对抗审查 implementer 的改动,默认存疑即 reject → `send_message(to="coordinator", verdict…)`。
5. 状态写 `.agentchat-demo/state.json`,响应 `/status`。

> MCP 工具(已核对 `lib/mcp-server-core.js`):`whoami()` / `send_message(to,summary,full,type?,priority?,reply_to?,attachments?,schema?)` / `check_inbox(kinds?)→{dm,group}` / `post(group,summary,full,…)` / `check_group(group,…)`。

### D. robrix2(**可选,纯 UX,不做也能跑**)
- 现在 `/create-issue` 等会被当普通文本发出——**功能已通**。
- 可选:在 `src/shared/mentionable_text_input.rs` 的 `SLASH_COMMANDS` 里加 `/create-issue`、`/status` 做输入自动补全提示(纯前端体验,不影响协议)。
- 建房+拉人已具备:`MatrixRequest::CreateRoom` + `InviteUser`(`src/sliding_sync.rs`)。

---

## 6. 一次完整时序(create-issue → 审查通过)

```
robrix2: 邀请 @agent-bridge 进任意房 → 发 "!mkgroup demoboard coordinator implementer reviewer"
  bridge: 建后端 group + 新建群房 "demoboard",邀请 你 + 3 个 agent(agent 30s 内 pollAgentInvites 自动 join)
robrix2: 进 demoboard,发 "@coordinator /create-issue 登录页崩溃 | 点击登录按钮闪退"
  bridge → backend /api/messages → coordinator.inbox(@提及才进)→ push-relay 往 tmux 注入 [NOTIFICATION]
coordinator: 见 [NOTIFICATION] → check_inbox → 从消息 group 字段学到 "demoboard" → 写 issues/010-*.md → agent-spec parse/lint
           → post(group="demoboard", "已生成 spec,得分0.82,回复 approve 确认")
robrix2: 发 "approve"
coordinator: 生成 plan → send_message(to="implementer", type="request", 实现任务+plan) + post(group) 告知
implementer: 改代码 → send_message(to="coordinator", type="reply", "实现完成,diff 摘要…")
coordinator: send_message(to="reviewer", type="request", "对抗审查这段实现:…") + post(group)
reviewer: 审查 → send_message(to="coordinator", type="reply", "发现2处问题:…" 或 "通过")
coordinator: 聚合 → post(group="demoboard", 结论)(robrix2 看到)
robrix2: 发 "@coordinator /status" → coordinator post(group) 各步状态
```

---

## 7. robrix2 改动量评估

| 路线 | robrix2 改动 | 说明 |
|---|---|---|
| **本 demo(纯文本)** | **≈0 行**(可选加几行斜杠补全) | 全部逻辑在 agent-chat 侧 skill + CLI |
| 将来要富 UI(审批卡片/按钮) | 中等:给 bridge 加 `org.octos.*` 透传 + 确认 `@ac_*` 走 bot 渲染 | 见 §8,本期不做 |

---

## 8. 风险 / 待确认

1. ~~Palpo 端口/域割裂~~ **已实测排除**:compose 映射 `8128:8008`,server_name/well-known 都是 `127.0.0.1:8128`,主机连 `:8128` 即可;OrbStack 下 `curl /_matrix/client/versions` 已通。仍建议 demo 前用 `roadmap/agentchat-demo/preflight.sh` 跑一遍确认登录往返。
2. **群聊里命令路由**:无 `@mention` 的群消息可能不进具体 agent inbox。约定命令必须 @coordinator,或改用「与 coordinator 的 DM 房」承载命令。
3. **trust gate**:`MATRIX_TRUST_MODE=enforce` 时需把你的 robrix 用户加进 `MATRIX_TRUSTED_INVITER_MXIDS`,否则 bot 拒绝加入。demo 用 `audit` 最省事。
4. **approve 门禁**:`approve` 是纯文本,coordinator 靠提示词识别;需在 skill 里写清「仅接受房主/特定人的 approve」,避免任意成员误触发。
5. **agent-spec 工作目录**:agent 在自己的 `--project` 工作区跑 `agent-spec`,确认该工作区就是目标仓库(robrix2 或别的 demo 仓)。

---

## 9. 最小可演示路径(MVP 顺序)

1. 起 Palpo(已部署)。`@agent-bridge` 由 bridge 首启自动注册,`@ac_*` 自动建。
2. 配 agent-chat `.env`(`roadmap/agentchat-demo/agent-chat.env.demo`),**务必填 `API_TOKEN` + `MATRIX_BRIDGE_SECRET` + `MATRIX_BOT_PASSWORD`**。
3. **分别**起 backend → (健康检查通过后) bridge + push-relay(`start-demo.sh` 已用 `/health` 轮询替代盲 sleep)。
4. `agentchat up-v1 coordinator/implementer/reviewer claude --project <repo> --project-mode symlink --allow-shared-workspace --fresh`(**三个都要** `--allow-shared-workspace`)。
5. link `issue-workflow` skill:`roadmap/agentchat-demo/link-skill.sh`(skill 已写好)。
6. robrix2 登录本地 Palpo → 邀请 `@agent-bridge` → 发 **`!mkgroup demoboard coordinator implementer reviewer`** 建群 → 进 `demoboard`。
7. 在 demoboard **`@coordinator /create-issue …`** 跑通全链路。

> 一键脚本:`roadmap/agentchat-demo/start-demo.sh` 已把 3-5 步串好(含 `/health` 轮询、三 agent 共享 workspace、`!mkgroup` 指引)。
> 真正的「开发量」只有 §5C 那个共享 skill,且**已写好并经对抗审计修正**;其余全是配置与现成 CLI。
