# robrix2 × agent-chat × mempal:三层架构(修订版)

> 状态:**已实现,待真机 demo 测试**。承重事实已对**今日构建的 mempal 二进制**
> (`~/.cargo/bin/mempal`,2026-05-30)和**正在运行的 wf_* tmux 会话**一手核实;
> §5 的两处加法(`start-demo.sh` Step 4.5、`SKILL.md` reviewer peek+capture)已落地,
> `bash -n start-demo.sh` 通过。分支:`analyze/agent-chat-integration`。
> **未提交**——按项目规则等用户先测。
>
> 一句话:**人↔agent 走 agent-chat/Matrix(不动);agent↔agent 用 mempal cowork 加一条
> 精确上下文车道(peek 看对方活会话 + capture 沉淀裁决);跨会话/跨项目记忆走 mempal MCP
> (另一条轨,已在 `mempal-agentchat-memory-integration-zh.md` 设计,保留)。**
> **零改源码**(agent-chat 和 mempal 都不动),纯加法:config + skill 文本 + 脚本。

---

## 0. 这版相对上一版改了什么

上一版(memory-integration 文档)的判断是「cowork 明确不用,通信全归 agent-chat」。
本版**修订**该判断:cowork 的**通信总线**仍不替代 agent-chat,但 cowork 的**两个
精确特性**(`cowork-tmux-peek` 读对方活 pane、`cowork-capture` 沉淀裁决)是 agent-chat
给不了的,且**纯读 / 纯写 db,不碰 pane 写入,不与 push-relay 冲突**——这是真正的增量。

记忆轨(mempal MCP:`search`/`context`/`ingest`/`tunnels`)与本文**正交**,原样保留。

---

## 1. 三层架构图

```
┌──────────────────────── robrix2(人在看)───────────────────────────────┐
│                  Matrix room "demoboard"  (m.room.message)              │
└───────────▲───────────────────────────────────────────────▲────────────┘
   post(group)/@mention │                       人下命令(/create-issue…)│
┌───────────┴─────────────── ① 人↔agent:agent-chat / Matrix ────────────┴──┐
│   bridge-matrix.js ── backend-v2.js ── push-relay.js                       │
│   人的唯一入口:robrix2 聊天 → coordinator;结果经 post(group) 回到房间      │
│   push-relay 独占 [NOTIFICATION] 注入(tmux send-keys)→ 唤醒 agent pane     │
└────┬──────────────────────────┬───────────────────────────┬──────────────┘
 ┌───▼────────┐          ┌───────▼────┐              ┌───────▼────┐
 │wf_coordi…  │          │wf_impleme… │              │wf_reviewer │  每 agent=1 tmux pane
 │ MCP#1 agent│          │ MCP#1 agent│              │ MCP#1 agent│  ← ① 传输:whoami/
 │      -chat │          │      -chat │              │      -chat │     check_inbox/send/post
 │ MCP#2 memp…│          │ MCP#2 memp…│              │ MCP#2 memp…│  ← ③ 记忆:search/context/
 └───┬────────┘          └───┬────────┘              └───┬────────┘     ingest/tunnels
     │                       │                           │
     │   ② agent↔agent:mempal cowork(本版新增的精确车道)│
     │   ─ 派活/交接:cowork-send --from --to(可选,见 §3 唤醒缺口)
     │   ─ reviewer 判前看 implementer 活会话:cowork-tmux-peek ◄──读 pane(纯读)
     └───────────────────────┴─ 裁决沉淀:cowork-capture --execute ─┐
                                                                     ▼
                         ┌───────────────────────────────────────────────┐
              ② 运行态 → │ ~/.mempal/cowork-bus/<project_identity>/        │ 易失:registry +
                         │   registry.json / inbox/<id>.jsonl / events     │ 每-agent inbox
                         ├───────────────────────────────────────────────┤
              ③ 持久  →  │ ~/.mempal/palace.db (wing=<repo>)               │ 持久:跨会话/跨项目
                         │   search/context/ingest/tunnels + capture 落盘   │ 记忆(混合搜索)
                         └───────────────────────────────────────────────┘
              ③ 两层都经 stdio:`mempal serve --mcp`(MCP) / `mempal cowork-*`(CLP)
```

**三层各跑各的,载体不同:**

| 层 | 解决 | 载体(机制) | 工具 | 是否易失 |
|---|---|---|---|---|
| **① 人↔agent** | 人此刻下命令、看结果 | Matrix 消息 + push-relay→tmux | agent-chat MCP:`whoami`/`check_inbox`/`send_message`/`post` | 易失 |
| **② agent↔agent** | coordinator 派活、impl→reviewer 交接、reviewer 判前看活会话 | cowork-bus(per-agent inbox JSONL)+ tmux 纯读 capture-pane | CLI:`cowork-send`/`cowork-tmux-peek`/`cowork-capture` | 易失(inbox)/落盘(capture) |
| **③ 记忆** | 任何 agent 任何时候拿到充足历史/跨项目上下文 | `~/.mempal/palace.db` | mempal MCP:`mempal_search`/`context`/`ingest`/`tunnels` | 持久 |

**每层流什么:**
- **① 流命令与可见进度**:人 `@wf_coordinator /go 0001` → coordinator;每步 `post(group)` 让人看到 coord→impl→reviewer 全链。**push-relay 独占 `[NOTIFICATION]` 注入**(唯一写 agent pane 的)。
- **② 流精确交接上下文**:派活/交接的 message(可选,见唤醒缺口);**reviewer 判决前 `cowork-tmux-peek wf_implementer` 直接读 implementer 活 pane**——拿到 agent-chat 的 summary/full 给不了的「它此刻在终端里到底说了什么/跑了什么」;裁决出来 `cowork-capture --execute` 沉淀。
- **③ 流持久决策/跨项目**:接活前 `mempal_search` 捞背景,出决策后 `mempal_ingest` 写回。

> 关键区分:**②(cowork-bus)是「此刻的精确运行态」**(per-agent inbox + 活 pane 快照),
> **③(palace.db)是「跨时间的持久记忆」**。`cowork-capture --execute` 是②→③的桥
> (把一次裁决 handoff 落到 palace 抽屉,bus.rs:1032 `capture_handoff_to_memory`,
> 只有 `--execute` 才真写,核实)。

---

## 2. agent↔agent 怎么接(真实命令,全部对今日二进制核实)

### 2.0 关键事实(承重,先记)
- **per-agent inbox 是真路径**:`cowork-send --to <id>` 写
  `~/.mempal/cowork-bus/<project_identity>/inbox/<id>.jsonl`(实测 message_id 返回 `inbox=…/inbox/test_receiver.jsonl`)。
- **registry 按 `--cwd` 的 git-root identity 键**:`<project_identity>` = 从 `--cwd` 向上
  走到 `.git` 根再编码成 `-` 串。**三个 agent 必须用同一个 `--cwd`(= repo 根)**,否则互不可见。
- **⚠ 符号链接 workdir 不是 git 根(实测核实)**:demo agent 跑 `--project-mode symlink`,
  其 pane cwd = `~/.agentchat/agents/agent_<name>/workdir`,**该目录无 `.git`**
  (`git rev-parse --show-toplevel` 失败,真实 repo 在其 `projects/` 子目录下)。
  → **所有 cowork-* 调用都必须显式 `--cwd <DEMO_REPO 真实根>`**,不能靠 agent 自身 cwd 推断。
  这是最易出「registry 查不到 / 静默空」的点。

### 2.1 把每个 agent 注册进 cowork(带 tmux-target)
demo 的 tmux 会话名就是 agent 名(`agentchat up-v1 <name>` → tmux session `<name>`,
pane `<name>:0.0`,实测 `tmux list-panes -t wf_implementer` → `wf_implementer:0.0`)。
**启动循环之后**注册(pane 必须已存在):

```bash
REPO=/Users/zhangalex/Work/Projects/FW/robius/robrix2
for name in wf_coordinator wf_implementer wf_reviewer; do
  mempal cowork-register \
    --agent-id "$name" --tool claude \
    --cwd "$REPO" \
    --transport tmux --tmux-target "$name:0.0"
done
```
> 实测:`registered agent wf_implementer tool=claude transport=tmux tmux_target=wf_implementer:0.0`。
> `--transport tmux` 时 `--tmux-target` 必填(否则 `BusError::TmuxTargetRequired`,bus.rs:485)。

**注意一个张力(见 §3):** `transport=tmux` 让该 agent **可被 peek**(纯读 capture-pane),
但 `cowork-send --to <tmux-agent>` 会走 `send_tmux`→`send-keys` **写它的 pane**(bus.rs:1299),
**和 push-relay 抢同一个 pane**。所以「可 peek」与「可 cowork-send」在同一 agent-id 上**不能
同时干净成立**。本设计的解法:**tmux 注册只用于 peek;派活/交接的 message 走 inbox 车道
(见 §2.5)或干脆保留 agent-chat send_message(加法版,§6 推荐)。**

### 2.2 coordinator 派活给 implementer(message 车道)
若走 cowork message(非 agent-chat),用 **inbox transport**(不碰 pane):
```bash
mempal cowork-send \
  --from wf_coordinator --to wf_implementer \
  --cwd "$REPO" \
  --thread-id issue-0001 \
  --message "实现 issue 0001。spec: specs/task-0001-*.spec.md  plan: docs/plans/0001-*.md。干完交给 wf_reviewer。"
```
> `--thread-id` 是**可选元数据(不是队列)**,只用于分组/在 delivery 事件里显示
> (p90 spec,inbox.rs:48),不影响投递路由——所有消息都进收件人的同一个 inbox。

### 2.3 implementer 交接给 reviewer(message 车道)
```bash
mempal cowork-send \
  --from wf_implementer --to wf_reviewer \
  --cwd "$REPO" --thread-id issue-0001 \
  --message "issue 0001 实现完。changed: src/foo.rs。已 build,未跑集成测试。diff 待你审。"
```

### 2.4 reviewer 判前看 implementer 的活会话(★ 本设计核心增量)
**这是 agent-chat 给不了的精确上下文。** reviewer 在给裁决**之前**,直接读 implementer
pane 此刻的内容(它跑了什么、test 输出、它自己说「未验证什么」):
```bash
mempal cowork-tmux-peek \
  --agent-id wf_implementer \
  --cwd "$REPO" \
  --lines 120
```
> **实测真读到活 pane**:对正在运行的 `wf_implementer` 调用,返回了它终端里的真实文字
> (「Diff is handed to wf_reviewer. I'm idle again, standing by for the review outcome」+
> token 计数行)——正是 reviewer 判前要的「它此刻到底干到哪」。
> 机制:`tmux capture-pane -p`(**纯读,无 send-keys**,bus.rs:800),前提是 implementer
> 已 `--transport tmux --tmux-target` 注册(§2.1)。

reviewer 据此 + `git diff` + spec 做对抗性检查,得出 `approve|reject`。

### 2.5 把裁决沉淀进持久记忆(② → ③ 的桥)
reviewer 出裁决后,把这次 handoff 落到 palace.db(只有 `--execute` 才真写,实测核实):
```bash
mempal cowork-capture \
  --cwd "$REPO" \
  --wing "$(basename "$REPO")" \
  --room issue-0001 \
  --thread-id issue-0001 \
  --note "issue 0001 评审裁决:reject。根因:src/foo.rs:42 未处理空输入;spec 完成项 3/5。修复:加 guard + 单测。" \
  --execute
```
> 不带 `--execute` = dry-run(`CoworkCaptureReport.writes=false`,bus.rs:1072);带 `--execute`
> 才写抽屉并返回 `drawer_id`。`--wing` 默认 `cowork-capture`,**demo 应显式传 repo basename**
> 让裁决落到项目 wing,与记忆轨(③)的 `mempal_ingest` 同一个 wing,可被后续 `mempal_search` 检索。

---

## 3. tmux 冲突结论(CRITICAL)

### 结论一句话
**会冲突——但只在「cowork 用 tmux transport 发消息」这一种用法。** peek 是纯读、capture 是写 db,
**都不碰 pane 写入,与 push-relay 零冲突**。干净接法:**push-relay 永远独占 `[NOTIFICATION]`
注入;cowork 的 tmux 注册只用于 `cowork-tmux-peek`(读);agent↔agent 的消息要么走 cowork
inbox transport,要么保留 agent-chat send_message。**

### 冲突在哪(说清楚,别埋)
- **push-relay 写 agent pane**:`pushToTmux()` 对每条消息执行 **6 步 `tmux send-keys`**
  (payload + Tab + Enter×2 + C-m×2,带 300ms 间隔,push-relay-core.js:664-701)。
- **cowork tmux transport 也写 pane**:`cowork-send --to <tmux-agent>` → `send_tmux()` →
  **单条 `tmux send-keys -t <target> -- <envelope> Enter`**(bus.rs:1296)。
- **若两者同写一个 pane**:push-relay 的 6 步注入与 cowork 的单步注入**交错** →
  乱序投递、键击被覆盖、消息序列损坏。**这是真实的 dual-writer race,不是理论风险。**

### 不冲突的部分(实测确认安全)
- `cowork-tmux-peek` 走 `tmux capture-pane -p`(**只读快照,无 send-keys**)——实测对正在被
  push-relay 管的 `wf_implementer` 活 pane 读取成功,不干扰。
- `cowork-capture` 写 `palace.db`,**完全不碰 tmux**。
- cowork **inbox transport** 写 `inbox/<id>.jsonl` 文件(bus.rs:1252),**不碰 pane**。

### 最干净的非冲突布线
```
① 人→agent 唤醒/投递 : push-relay 独占(SSE → 6 步 send-keys)  ── 唯一 pane 写入者
② agent↔agent 消息   : cowork inbox transport(写 JSONL 文件)   ── 不碰 pane
② reviewer 看活会话  : cowork-tmux-peek(capture-pane 纯读)     ── 不碰 pane(只读)
② 裁决沉淀          : cowork-capture --execute(写 palace.db)   ── 不碰 pane
✗ 禁止              : cowork tmux transport 发消息(与 push-relay 抢 pane)
```

> **一个 agent-id 同时「可 peek」+「可 cowork-send」的张力**:peek 要求 `transport=tmux`,
> 但 `cowork-send` 到 `transport=tmux` 的 agent 就触发 send_tmux 冲突。若真要 cowork 双向发消息,
> 得给每个物理 agent **注册两个 id**(一个 `tmux` 供 peek、一个 `inbox` 供收消息)——徒增复杂。
> 加法版(§6 推荐)直接绕过:**tmux 注册只服务 peek,消息保留 agent-chat**,一个 id 就够。

---

## 4. SKILL.md 改动(delta)

**whoami 角色分支原样不动(agent-chat MCP 保留)。** ROLE-MODEL 块的核实结论:只要 agent-chat
还在,`whoami().me.name` 子串匹配(coordinator/implementer/reviewer,SKILL.md:12-17、65、140、154)
**完全不用改**。变的只是「交接动作」加了 peek/capture,**且加法版连 send_message 都不必换**。

### 4a. 加法版(推荐,§6 论证)— 改动极小
- **角色分支(12-17, 65, 140, 154)**:不动。
- **send_message 交接(118-127, 145, 162)**:不动(继续用 agent-chat 发交接)。
- **只新增两段:**
  - reviewer 角色(154 段)判决前加一步:
    ```
    在给出 approve|reject 之前,先读 implementer 此刻的活会话:
      mempal cowork-tmux-peek --agent-id wf_implementer --cwd <REPO 真实根> --lines 120
    （--cwd 必须是 repo 根,不是你的 workdir——workdir 不是 git 根。工具不可用就跳过,退回 git diff。）
    把它正在跑/已说「未验证」的内容纳入对抗性检查。
    ```
  - reviewer 出裁决后加一步:
    ```
    裁决后沉淀:
      mempal cowork-capture --cwd <REPO 真实根> --wing <repo basename> --room issue-NNN \
        --note "<裁决+根因+修复>" --execute
    ```
- **一次性 setup 提示**(放「Shared workspace」段):说明 agent 已被 `start-demo.sh` 注册进
  cowork(`transport=tmux`),peek/capture 直接可用;`--cwd` 永远传 repo 根。

### 4b. 全 cowork-handoff 版(替换 send_message)— 改动较大,且有缺口
若要把交接也搬到 cowork:
- **角色分支**:仍不动(whoami 还在)。
- **send_message → cowork-send**:118/125/145/162 的 `send_message(to=…)` 换成
  `mempal cowork-send --from <self> --to <peer> --cwd <REPO> --message …`。
- **新增 peer 发现**:cowork 无 whoami;peer 的 agent_id 用 `mempal cowork-agents --cwd <REPO>`
  列出再缓存(ROLE-MODEL forDesign),或直接硬编码三个固定名。
- **⚠ 缺口(见 §3 唤醒 + 下方诚实评估)**:cowork-send 到 inbox 后,**没有唤醒信号**——
  idle 的 implementer 不会主动 drain,要等它下一次 UserPromptSubmit 才触发 hook。而触发那次
  prompt 的恰恰是 push-relay 的 `[NOTIFICATION]`。**所以全 cowork-handoff 仍依赖 agent-chat
  来唤醒**,没真正脱钩。**不推荐用它替换 send_message。**

---

## 5. 插入点(精确编辑)

### 5.1 `start-demo.sh`(改 2 处,纯加法)

**(a) Step 4 之后新增 Step 4.5:注册 agents 进 cowork(带 tmux-target)。**
pane 此时已存在(Step 4 已 `up-v1`)。**如实现**(注意两处比早期草稿更稳:
`pwd -P` 取规范真实路径作 bus key;用 **`.agentchat-demo/cowork.json` 文件**把这个 cwd
交给 skill,**不用 `export`**——agents 是各自独立的 tmux 进程,父脚本的环境变量到不了它们,
文件才是正确的交接通道):
```bash
echo "== Step 4.5: register agents into mempal cowork (peek + capture layer) ==="
if command -v mempal >/dev/null 2>&1; then
  COWORK_CWD="$(cd "$DEMO_REPO" && pwd -P)"   # canonical real path = the bus key
  for name in wf_coordinator wf_implementer wf_reviewer; do
    mempal cowork-register --agent-id "$name" --tool claude \
      --cwd "$COWORK_CWD" --transport tmux --tmux-target "$name:0.0" >/dev/null 2>&1 \
      && echo "  registered $name → tmux $name:0.0" \
      || echo "  ⚠ register $name failed (peek disabled for it; flow still works)"
  done
  # Hand the skill the EXACT cwd string to pass to every cowork-* call (never pwd).
  mkdir -p "$DEMO_REPO/.agentchat-demo"
  printf '{ "cowork_cwd": "%s", "mempal_wing": "%s" }\n' \
    "$COWORK_CWD" "$(basename "$COWORK_CWD")" > "$DEMO_REPO/.agentchat-demo/cowork.json"
  echo "  cowork cwd → $COWORK_CWD  (written to .agentchat-demo/cowork.json)"
else
  echo "  mempal not on PATH — peek/capture disabled (transport + workflow unaffected)"
fi
```

> **不要装 `cowork-install-hooks`**:实测它生成的 hook 跑 `cowork-drain --target claude`
> (**tool-family** inbox,inbox::drain),而 `cowork-send --to <id>` 写的是 **per-agent**
> inbox(`cowork-agent-drain --agent-id`,bus::drain_agent)。**实测两者不互通**:
> probe 消息发到 per-agent inbox 后,`cowork-drain --target claude` 返回**空**,
> `cowork-agent-drain --agent-id test_receiver` 才取到。所以标准 hook **对 N-agent
> 派活无效**。加法版根本不发 cowork 消息(只 peek/capture),**不需要任何 drain hook**——
> 这也是加法版更干净的又一原因。(全 cowork-handoff 版才需要给每 agent 装**自定义**
> per-agent hook 跑 `cowork-agent-drain --agent-id <self> --cwd <REPO>`,且仍有唤醒缺口。)

**(b) 可选:Step 4.5 里加一行 cowork-capture 的 wing 对齐说明**(已在 §5.2 skill 覆盖,脚本可不动)。

### 5.2 `issue-workflow/SKILL.md`(加法版的 delta,见 §4a)— **已实现**
- 「Shared workspace & tools」段加了一段 **Memory / peek layer** 说明:三个 agent 已被
  Step 4.5 注册进 cowork,`.agentchat-demo/cowork.json` 给出 `cowork_cwd`/`mempal_wing`,
  任何 `cowork-*` 调用都用 `cowork_cwd`(**绝不用 pwd**,symlink → 另一条 bus)。
- reviewer 段加了 **One-time setup**(读 cowork.json 取 `<COWORK_CWD>`/`<WING>`)+ **step 2**
  judge 前 `cowork-tmux-peek` 看 implementer 活会话 + **step 4** reply *前* `cowork-capture
  --execute` 沉淀裁决(放在 reply 前,好让 reply 如实报告 `capture=ok`)+ **step 5** reply。
  两步都「mempal 缺失/报错 → 静默跳过、退回 `git diff`」。
- **可观测痕迹(按 advisor 建议)**:reply 与给人看的 `post` 都带一行
  `Context: <peek=ok(120 lines)|peek=unavailable> · <capture=ok|capture=skipped>`——把
  「静默 no-op」变成 Matrix transcript 里可见的事实,否则② 层失效与成功长得一模一样,
  正好掩盖了这层存在的唯一目的(更精准上下文)。
- **不动 whoami 分支、不动 send_message。** transport 仍全归 agent-chat。

### 5.3 `mempal.mcp.json`(③记忆轨,**沿用另一文档,不重复设计**)
`roadmap/agentchat-demo/mempal.mcp.json` 已在 memory-integration 文档定义:
```json
{ "mcpServers": { "mempal": { "command": "/Users/zhangalex/.cargo/bin/mempal", "args": ["serve","--mcp"] } } }
```
> 注意:repo 根的 `.mcp.json` 已是这份内容(实测)。memory-integration 文档用 `--extra-args
> "--mcp-config=$SCRIPT_DIR/mempal.mcp.json"` 在启动循环注入 ③。**本文(②层)不依赖 MCP**——
> peek/capture 全走 **CLI**(`mempal cowork-*`),只要 `mempal` 在 PATH 即可,**与 MCP 注入解耦**。
> 这意味着即使 ③ 的 MCP 注入没接上,② 的 peek/capture 仍能独立工作。

---

## 6. 诚实评估

### 这比把 agent↔agent 留在 agent-chat 更好吗?——**分两件事看**

**A. peek + capture:是真增量,推荐接(加法,可与 agent-chat 消息并存)。**
- `cowork-tmux-peek` 给 reviewer 的「读 implementer **活终端**」是 agent-chat **结构上给不了**的:
  agent-chat 只有 `send_message` 的 `summary`/`full`(implementer 自己愿意写的摘要),
  **拿不到它终端里此刻真实跑了什么 / test 真实输出 / 它没主动汇报的状态**。实测 peek 读到了
  implementer pane 的真实文字。**这就是用户要的「精确上下文」,确实是 cowork 独有的赢点。**
- `cowork-capture --execute` 把一次裁决 handoff **确定性地**落到 palace.db,带 wing/room/
  thread 元数据,可被记忆轨 `mempal_search` 检索——比让 agent 手搓 `mempal_ingest` 文本更结构化。
- **两者纯读 pane / 纯写 db,与 push-relay 零冲突(§3 实测),且不改 agent-chat 消息**——
  可以「**保留 agent-chat 全部消息 + 仅叠加 peek/capture**」,这是**最低风险、最高确定收益**的版本。

**B. 用 cowork-send 全面替换 agent↔agent 消息:不推荐(此版有两个真实缺口)。**
1. **唤醒缺口(承重)**:cowork-send 写 inbox 后**没有唤醒信号**(MSG-BUS forDesign 原文
   "No wakeup signal; agent must complete its prompt turn, which triggers the hook")。idle 的
   implementer 要等下一次 UserPromptSubmit 才 drain,而**戳它产生那次 prompt 的正是 push-relay
   的 `[NOTIFICATION]`**。→ 全 cowork-handoff **仍离不开 agent-chat 唤醒**,没真脱钩,只是把
   消息体从 Matrix 挪到 JSONL,徒增一套要维护的 drain hook。
2. **标准 hook 不对**(实测):`cowork-install-hooks` 生成的 hook 跑 `cowork-drain --target
   claude`(tool-family),**取不到** `cowork-send --to <id>` 写的 per-agent inbox(probe 实测为空);
   要 N-agent 派活得给每 agent 写**自定义** hook 跑 `cowork-agent-drain --agent-id <self>`——
   额外维护面,且不解决缺口 1。
3. **tmux transport 发消息会与 push-relay 抢 pane(§3)**;换 inbox transport 又回到缺口 1。

### 取舍表

| 维度 | 加法版(peek+capture,留 agent-chat 消息)★推荐 | 全 cowork-handoff(替换 send_message) |
|---|---|---|
| 改动量 | SKILL.md 加 2 段 + 脚本 1 个 Step | SKILL.md 改所有交接 + peer 发现 + 自定义 drain hook |
| 唤醒 | push-relay 原样(不动) | **仍依赖 push-relay 唤醒**(没脱钩) |
| tmux 冲突 | 无(peek 纯读) | inbox 无冲突但有唤醒缺口;tmux 直接冲突 |
| 精确上下文增量 | **全拿到**(peek+capture) | 同样拿到,但额外背缺口 1/2/3 |
| 风险 | 低(纯加,可降级) | 中(多套机制,缺口未闭合) |
| 用户「精确上下文」诉求 | **满足**(就是 peek+capture 带来的) | 满足但代价大 |

**结论**:用户要的「精确上下文」=**peek + capture**,而这恰恰是 cowork **不与 agent-chat
冲突、可纯加法叠加**的部分。**推荐加法版:保留 agent-chat 做 ① 人↔agent 和 agent↔agent
消息/唤醒,仅叠加 cowork 的 peek(reviewer 判前看活会话)+ capture(裁决沉淀)。** 全面把
消息搬到 cowork 不仅没消除对 push-relay 的依赖,还引入唤醒缺口和自定义 hook 维护面——
**收益(精确上下文)在加法版已全部到手,替换版只多代价不多收益。**

### 已解决(由本次实现兜住)
1. **`--cwd` identity 一致性** ✓:Step 4.5 用 `COWORK_CWD="$(cd "$DEMO_REPO" && pwd -P)"`
   注册,并把这个**规范真实路径**原样写进 `.agentchat-demo/cowork.json` 的 `cowork_cwd`;
   SKILL.md reviewer 段开头「One-time setup」明确要求**逐字读 `cowork_cwd`,绝不用 pwd**
   (pwd 是 symlink,hash 到另一条 bus → "unknown agent")。脚本注册侧与 skill 读取侧
   因此解析到同一 `project_identity`。
2. **重启幂等** ✓:`cowork-register` 是「register **or update**」,无 `cowork-unregister`;
   重跑 `start-demo.sh` 对同一 `(agent-id, cwd)` 直接覆盖,registry 不累积陈旧项,
   故 Step -1 无需为 cowork 加清理。

### 已对活 bus 烟测(2026-05-30,wf_* 会话在线时)
- Step 4.5 注册环跑通:三个 `wf_*` 全部 `cowork-register` 成功、`presence=online`,
  `.agentchat-demo/cowork.json` 写出 `cowork_cwd=/Users/zhangalex/Work/agentchat-demo-sandbox`、
  `mempal_wing=agentchat-demo-sandbox`;`cowork-agents --cwd <real>` 三个都在。
- 链路四步全验:`cowork-tmux-peek --agent-id wf_implementer` 读到 implementer 活 pane
  真实文字(它正说「diff 已交 wf_reviewer、idle」)→ `cowork-capture --execute` 返回
  `writes=true drawer_id=… wing=agentchat-demo-sandbox` → `mempal search "<note>"` **能召回**
  该抽屉(证实 wing=basename 可检索,非只可写)→ 探针抽屉已 `mempal delete` 清掉。

### TODO-VERIFY(剩余待真机确认,非阻断)
1. **首跑端到端(行为层)**:上面是手动逐条验命令;还需真机跑一次完整 demo,确认 reviewer
   **自己**在 review 时触发了 peek/capture,且 reply/`post` 里带出了 `Context: peek=… ·
   capture=…` 那行(advisor 的可观测痕迹——这行就是「它真的吃了② 层」的正信号;若显示
   `peek=unavailable` 也 OK,说明优雅降级生效、且**可见**)。
2. **pane target 漂移**:注册硬编码 `<name>:0.0`(实测当前布局即 `:0.0`)。若 tmux 窗口
   布局变了 target 会失配 → peek 失败但优雅降级到 `git diff`。要更稳可注册前
   `tmux list-panes -t <name>` 取真实 target(防御性,可选,未做)。
3. **peek 窗口 120 行**:若 implementer 的关键推理被 build 输出顶出 120 行外,peek 可能看不到——
   小调参,真机看效果再定(可选调大)。
4. **codex 不对称**(沿用另一文档):demo 全 claude;若某 agent 换 codex,cowork CLI 仍可用
   (peek/capture 不分 runtime),但其 ③ 记忆轨的 MCP 注入走全局 `~/.codex/config.toml`。

---

## 7. 相关文件 / 证据

- **改**:`roadmap/agentchat-demo/start-demo.sh`(Step 4.5 新增,§5.1)、
  `roadmap/agentchat-demo/issue-workflow/SKILL.md`(reviewer peek+capture 两段,§4a/§5.2)。
- **沿用**:`roadmap/agentchat-demo/mempal.mcp.json`(③记忆轨,memory-integration 文档已定义)。
- **不动**:agent-chat 源码、mempal 源码、push-relay。
- **cowork 证据(对今日二进制 `~/.cargo/bin/mempal` 2026-05-30 实测)**:
  - 注册带 tmux:`cowork-register --transport tmux --tmux-target wf_implementer:0.0` → 成功。
  - peek 读活 pane:`cowork-tmux-peek --agent-id wf_implementer --cwd <REPO>` → 读到真实终端文字。
  - per-agent vs tool-family drain 分叉:`cowork-send --to test_receiver` → `cowork-drain
    --target claude`(hook 默认)**空**;`cowork-agent-drain --agent-id test_receiver` 取到。
  - capture 落盘:`cowork-capture … --execute` 才写抽屉(无 execute = dry-run)。
  - symlink workdir 非 git 根:`agent_wf_implementer/workdir` 无 `.git`(实测)。
  - 源行:`bus.rs`(register 475-533 / send 1098-1238 / send_tmux 1296-1310 /
    tmux_peek 800-829 / capture 1032-1096 / agent_inbox_path 408-417);
    `inbox.rs`(drain 185-200);`push-relay-core.js:664-701`(6 步 send-keys);
    生成的 hook:`.claude/hooks/user-prompt-submit.sh` → `cowork-drain --target claude`。
