# mempal × agent-chat:给多 Agent 工作流加「跨会话/跨项目共享记忆」

> 状态:**设计**(已一手核实所有承重事实,未落地到 demo)。
> 分支:`analyze/agent-chat-integration`
>
> 一句话:**agent-chat 是「现在说话」的实时传输层,mempal 是「跨会话/跨项目记住」的
> 记忆层。两者正交组合**——给每个 agent 挂第二个 MCP(mempal),再加一段 skill 读写
> 约定。**零改源码**(mempal 和 agent-chat 都不动),只加 MCP 配置 + skill 文本。
> mempal 自带的 cowork 总线**明确不用**(Matrix 已独占传输)。

---

## 0. 背景 / 动机

我们已经跑通的 demo:robrix2 聊天 → agent-chat 把命令路由给 `wf_coordinator/
implementer/reviewer` 三个 tmux 里的 Claude Code agent,跑 `issue→spec→plan→
implement→review` 工作流。

**它缺什么**:agent-chat 只有**即时消息**(`send_message`/`check_inbox`/`post`)——
会话一结束、agent 一重启,上下文就蒸发。它没有持久的、可跨项目检索的「项目记忆」。

**mempal 正好补这块**:Rust 单文件项目记忆系统(`~/.mempal/palace.db`),混合搜索
(BM25+向量)+ 知识图谱 + **跨项目 tunnel**,通过 `mempal serve --mcp`(stdio MCP)
暴露给任何 agent。本设计把它接进工作流,让每个 agent(无论 claude 还是 codex)接活时
都能 `mempal_search` 捞背景、产出决策后 `mempal_ingest` 写回——**跨会话、跨项目、
跨 agent 共享上下文**。

---

## 1. 承重决策:记忆与通信是两个层面,各归其位

**核心原则(用户定调):记忆是记忆,通信是通信,两者不冲突、不重叠,是不同层面的事。**

- **通信层**解决"此刻怎么把任务从 A 实时送到 B + 唤醒对方"——点对点、易失、此刻。
- **记忆层**解决"任何 agent 任何时候接手时,如何拿到充足的历史/跨项目上下文"——
  持久、可检索、跨时间跨 agent。

二者**正交**:一个 agent 完全可以同时用——通过通信层收到"去实现 issue 0001",再
通过记忆层 `search` 查"这个项目以前的相关决策",干完 `ingest` 写回。谁也不替代谁。

设计的脊柱因此是「**各用每个系统最本质的那一层**」:

| | agent-chat | mempal |
|---|---|---|
| **产品本质** | 通信 / 编排(Matrix 房间 + bridge + dashboard) | 项目记忆(混合搜索 + KG + 跨项目 tunnel) |
| **本设计用它的** | 通信层:`send_message`/`check_inbox`/`post` + push-relay→tmux | 记忆层:`ingest`/`search`/`context`/`tunnels` |
| **载体** | Matrix 消息(易失) | `~/.mempal/palace.db`(持久) |

> **关于 mempal 的 `cowork-*`(为什么不接)**:mempal 确实也带了一套实时通信附件
> (`cowork-register/send/broadcast/channel-send`、`tmux send-keys` 投递、presence/
> heartbeat)。读 `cowork-runbook` 确认:它机制上**就是**一套点对点/广播消息总线,
> 和 agent-chat 的 send_message/push-relay→tmux 是**同一类东西**。
>
> 不接它**不是因为它差或要竞争淘汰**,而是因为"各归其位":**通信不该由记忆系统来扛**。
> agent-chat 的本职就是通信(且更完整),mempal 的本职是记忆。让记忆系统去做通信、
> 或让通信系统去做记忆,都是越界。所以:通信全部走 agent-chat,记忆全部走 mempal 的
> `ingest/search/context/tunnels`,`cowork-*` 这条"记忆系统顺手长出的通信附件"我们
> 不动——保持每层只做自己最强的事。

---

## 2. 分工图

```
┌─────────────────────────── robrix2 (人在看) ───────────────────────────┐
│                       Matrix room "demoboard" (m.room.message)          │
└───────────────▲─────────────────────────────────────────▲──────────────┘
                │ post(group) / @mention                   │ 人下命令
        ┌───────┴───────────── agent-chat 传输层 ──────────┴────────┐
        │  bridge-matrix.js ── backend-v2.js ── push-relay.js       │
        └───────┼──────────────────┼──────────────────┼────────────┘
         ┌──────▼─────┐    ┌───────▼────┐    ┌────────▼───┐
         │wf_coordi…  │    │wf_impleme… │    │wf_reviewer │   每个 agent = 1 个 tmux pane
         │ ┌────────┐ │    │ ┌────────┐ │    │ ┌────────┐ │   + 两个 MCP server:
   MCP#1 │ │agent-  │ │    │ │agent-  │ │    │ │agent-  │ │  ← 传输: whoami/check_inbox/
         │ │chat    │ │    │ │chat    │ │    │ │chat    │ │     send_message/post
   MCP#2 │ │mempal  │ │    │ │mempal  │ │    │ │mempal  │ │  ← 记忆: mempal_search/context/
         │ └───┬────┘ │    │ └───┬────┘ │    │ └───┬────┘ │     ingest/tunnels
         └─────┼──────┘    └─────┼──────┘    └─────┼──────┘
               └─────────────────┼─────────────────┘
                                 ▼ stdio: `mempal serve --mcp`
                     ┌───────────────────────────┐
                     │  ~/.mempal/palace.db       │  ← 跨会话/跨项目持久记忆
                     │  wing=<repo> / room=…      │     (混合搜索 + tunnels)
                     └───────────────────────────┘
```

**交界**:agent 拿到 task(传输层 inbox)后,先去记忆层 `mempal_search` 捞背景;
产出决策后(review 裁决/状态流转),把**决策文本**写回记忆层 `mempal_ingest`。
两条管道各跑各的,互不抢传输、互不改源码。

---

## 3. 最小可行集成:第二个 MCP 挂在哪

**确认「真的只是第 2 个 MCP + skill 约定吗?」→ 概念上是的,纯加法。** 但关键不在
"加什么",在**"何时/何处注入"**:Claude Code **只在启动时读一次 MCP 配置、不热重载**,
所以 mempal 必须在 agent 进程**启动前**就在配置里。启动后再 patch `.mcp.json` 无效。

### 已验证的注入机制(claude,demo 真实路径)

agent-chat 的 `agent-up` 启动 claude 的命令行(`bin/agent-up:1753`):
```
claude --session-id <id> $CLAUDE_FLAGS -- "<init prompt>"
```
- `$CLAUDE_FLAGS` 已含 per-agent 的 `--mcp-config=$AGENT_PATH/.mcp.json`(agent-up:1733)
- `--extra-args "<v>"` 会**追加进 `$CLAUDE_FLAGS`**(agent-up:1738-1739),位置在 `--`
  分隔符之前(变长 flag 该在的区域,agent-up:1749 注释自证)
- `up-v1` 把 `--extra-args "<v>"` 成对透传给 `agent-up`(agent-up-v1:101-103)
- 前置条件已核实:`/etc/claude-code/managed-mcp.json` 不存在 → per-agent `--mcp-config`
  生效;claude CLI 接受**重复** `--mcp-config`(多份合并)

### 推荐:静态 `mempal.mcp.json` + `--extra-args`(最干净)

新增静态文件 `roadmap/agentchat-demo/mempal.mcp.json`(不写进 agent home,存活 `--fresh`):
```json
{
  "mcpServers": {
    "mempal": { "command": "/Users/zhangalex/.cargo/bin/mempal", "args": ["serve", "--mcp"] }
  }
}
```
> 已核实:`mempal serve --mcp` 走 stdio(mempal/src/main.rs:5960-5962);二进制
> `~/.cargo/bin/mempal`;DB `~/.mempal/palace.db` 存在。

启动循环里给每个 agent 加 `--extra-args "--mcp-config=$MEMPAL_MCP"` → claude 合并两份
配置,`agent-chat` + `mempal` 两个 server 同时上线。

### codex 的不对称(诚实记一笔)

demo 三个 agent 都是 claude。codex **没有** per-agent 文件——agent-up 用启动时
`-c mcp_servers.agent-chat.*` 内联 TOML(agent-up:1788-1805),不改 agent-up 无法扩展。
所以 codex 的加法路径是**全局** `~/.codex/config.toml` 的 `[mcp_servers.mempal]`,非
per-agent。(与上一轮"跨 runtime"分析一致。)

---

## 4. 读写约定(skill 文本,真实工具签名)

追加进 `issue-workflow/SKILL.md`,对所有角色生效。**签名全部来自核实过的 struct**
(`SearchRequest` tools.rs:34、`ContextRequest` tools.rs:83、`IngestRequest` tools.rs:1341、
`TunnelsRequest` tools.rs:1543)。

### WHEN 读(拿到 task,动手之前)
- 触发:`[NOTIFICATION]`→`check_inbox()` 拿到带 issue 的消息后,**先查记忆再干活**。
- 查事实 `mempal_search(query, wing?, room?, top_k=5, memory_kind?, domain?)`:
  `wing` **必须精确或省略**(不自动路由;猜错=0 结果)。不确定就省略做全局搜。
- 要决策支持 `mempal_context(query, max_items=8, include_evidence=false)`:返回
  dao_tian→dao_ren→shu→qi 有序条目 + trigger_hints,比 search 更适合"该怎么办"。
- reviewer 专用:评审前 `mempal_search(query=<spec 约束>, wing)` 捞历史同类 bug/决策,
  作对抗性检查的弹药。

### WHEN 写(达成决策/给出裁决后 → `mempal_ingest`)
mempal 唯一写工具是 `mempal_ingest`,`content` 是 **TEXT 字符串**(已核实,
IngestRequest.content: String),`wing` **必填且精确**:
```
mempal_ingest(content="<决策文本:结论 + 理由,不是聊天记录>",
              wing="<DEMO 的 wing>", room?="…", importance=3)   # importance 0-5
```
**写的时机绑定真实工作流闸门**(不写 brainstorm/中间探索):
- **coordinator**:收到 `approve` 做状态流转后 → ingest「issue NNN spec 已批准 + 决策要点」
- **implementer**:产出 diff 后 → ingest「issue NNN 实现选择 + 理由 + 未验证项」(决策性,非 diff 本身)
- **reviewer**:给出 `approve|reject` 后 → ingest「issue NNN 评审结论 + 根因/修复」

> 进阶(可选,非 demo 必须):反复出现的根因可 `mempal_knowledge_distill` 造候选知识卡片,
> 但 **agent 永不自动 promote**(mempal protocol rule 17 禁止)——promote/demote 是
> gate-enforced 的人工动作。demo 只到 `mempal_ingest` 这层即可。

---

## 5. 跨项目映射:sandbox → wing + tunnels 捞别项目

**模型**(已核实):`wing` = 项目(目录 basename),`room` = 项目内子区(init 时按目录推断)。
同名 `room` 出现在 ≥2 个 wing 时,**passive tunnel** 自动发现
(db.rs:1512 `GROUP BY room HAVING COUNT(DISTINCT wing)>1`)。

### sandbox → wing(一次性 setup)
```bash
mempal init   "$DEMO_REPO"                                  # wing 自动取 basename
mempal ingest "$DEMO_REPO" --wing "$(basename "$DEMO_REPO")"  # --wing 必填,接目录
```
> CLI `mempal ingest <dir>` 吃**目录**(setup 灌整个 repo);MCP `mempal_ingest(content=…)`
> 吃**文本**(agent 运行时灌单条决策)。两者职责不同,别混。

### agent 从别项目捞记忆(tunnels)
```
mempal_tunnels(action="discover")                                   # 本项目 room 在哪些别 wing 也出现(passive)
mempal_tunnels(action="follow", from={wing,room}, max_hops=2)       # 顺链走(最多 2 跳)
mempal_tunnels(action="add", left={wing,room}, right={wing,room}, label="…")  # 手建语义链
```
典型用法:implementer 做某子系统时 `discover` 看同名 room 在别项目有没有现成决策,
避免重复踩坑——这就是"跨项目充足上下文"。

---

## 6. 插入点(对 demo 的精确改动,全部加法)

**新增 1 文件 + 改 2 处,零改 agent-chat/mempal 源码。**

| 改动 | 文件 | 内容 |
|---|---|---|
| **新增** | `roadmap/agentchat-demo/mempal.mcp.json` | §3 那段 JSON |
| **改** | `start-demo.sh` Step 0.5(新增) | `mempal init` + `mempal ingest --wing` 把 sandbox 注册为 wing;mempal 缺失则跳过(记忆层降级,传输照常) |
| **改** | `start-demo.sh` Step 4(启动循环) | 每个 agent 加 `--extra-args "--mcp-config=$SCRIPT_DIR/mempal.mcp.json"`;并 `export MEMPAL_WING` 或写入 `.agentchat-demo/state.json` |
| **改** | `issue-workflow/SKILL.md` | 新增"## 记忆层(mempal)"段 = §4 读写约定;注明 wing 来源、工具不可用就跳过 |

codex 备选(仅当某 agent 改 codex):不动 demo 脚本,全局 `~/.codex/config.toml` 加
`[mcp_servers.mempal]`。

---

## 7. 诚实风险 / 待你机器上确认

**已核实确定(非 TODO)**:
- ✅ `mempal_ingest.content` 是文本字符串,不是目录(tools.rs:1341);CLI `ingest <dir>` 才吃目录
- ✅ `mempal serve --mcp` 走 stdio;二进制/DB 路径已确认
- ✅ `--mcp-config` 注入链端到端(managed-mcp 不存在 / extra-args 透传 / `--` 之前)
- ✅ `mempal init <DIR>` / `mempal ingest --wing <WING> <DIR>` 签名(live 跑过 `--help`)

**真实风险**:
1. **启动时机硬约束**:mempal 必须在 claude 启动**前**进配置(`--extra-args` 满足)。
   **不要**启动后 patch `.mcp.json`(不热重载,无效)——本设计最易踩的坑。
2. **claude 是否合并重复 `--mcp-config`**:本设计依赖此行为。起 agent 后让它试调
   `mempal_search` 确认 mempal 上线;若 CLI 只认最后一份,退回**预置方案**(在 `up-v1`
   前往 `~/.agentchat/agents/agent_<name>/workdir/.mcp.json` 预写 mempal 条目,靠
   agent-up preserve-merge 叠加,agent-up:1410)。
3. **codex 不对称**:demo 全 claude;引入 codex 走全局 config,非 per-agent。
4. **wing 精确性**:`ingest.wing` 必填、不自动路由;`search.wing` 猜错=0 结果。必须把
   `MEMPAL_WING` 显式传到 skill——最易出静默空结果的点。
5. **mempal 缺失降级**:脚本与 skill 都做"工具不可用就跳过记忆、传输照常"——记忆层是
   增强,不是 demo 跑通的硬依赖。

---

## 8. 相关文件 / 证据

- 改:`roadmap/agentchat-demo/start-demo.sh`、`roadmap/agentchat-demo/issue-workflow/SKILL.md`
- 新增:`roadmap/agentchat-demo/mempal.mcp.json`
- MCP 注入证据:`agent-chat/bin/agent-up`(1733/1738/1749/1753)、`bin/agent-up-v1`(101-103)
- mempal:`~/.cargo/bin/mempal`(`serve --mcp` → main.rs:5960);DB `~/.mempal/palace.db`;
  工具签名 `mempal/src/mcp/tools.rs`(34/83/1341/1543)
- 弃用的 cowork:`mempal cowork-runbook`、`specs/p85-mcp-multi-agent-cowork-bus.spec.md`
