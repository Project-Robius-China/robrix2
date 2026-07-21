# 四项目手工测试单(大白话版,2026-07-20)

> 每项 = 你做什么 → 应该看到什么。不符就把现象丢给 Claude。
> 建议顺序:一(零依赖)→ 三A → 二 → 完成 supervisor 切换 → 三B → 四。

## 一、agent-spec(纯命令行,零依赖,先测它)

- [ ] 1.1 `agent-spec --version` → 1.1.0
- [ ] 1.2 拿一份现成 spec 跑 `agent-spec parse` + `lint --min-score 0.7`
      → parse 出场景数、lint 100%(老功能没坏)
- [ ] 1.3 需求管线迷你走一遍(找个空目录):
      1) 写一小段 PRD 文本 → `agent-spec requirements import`(进 knowledge/requirements/)
      2) `requirements validate` + `graph --gate` → 通过;故意写个坏引用再跑 → 报错指得出位置
      3) `export --dialect v1 --out req.yaml` → 需求列表 yaml
      4) `export --dialect arc-native --out arc.yaml` → **单根树**(ROOT/FOLDER/ATOMIC 形状,就是 ARC 吃的格式)
      5) 加 `--check` 再跑一遍 → 无漂移,退出码 0
      6) 加 `--provenance prov.json` → 出编译来源清单(这就是"需求版本有据可查")
      7) `test-obligations` → JSON 义务清单
- [ ] 1.4 感受点:这条链 = "文字需求 → 机器可查的需求图 → 喂给任何执行后端的输入"

## 二、ARC(合作项目;注意:真编译要花 LLM 钱)

前置:Python 3.11 环境 + 它的模型 key。

- [ ] 2.1 不花钱冒烟:`--help` 里有 `--serve`;打开 `example/ticketbooking-demo/requirements.yaml`
      看树形结构(和 1.3-4 的输出同族)
- [ ] 2.2(可选,花钱)拿玩具需求树跑一次 CLI 编译 → 出接口 + 测试 + 代码 + 执行轨迹
- [ ] 2.3 serve 模式(现在的旧 backend 就能配合):
      1) `AGENT_CHAT_URL=http://127.0.0.1:8090` + token,`--serve` 起动
      2) 看板/花名册出现 **arc-compiler 在线**(heartbeat 自动注册,不用手工建)
      3) 给它发一条 task_request DM(缺 `requirement_dir` 的)→ 收到 `ok:false` 的
         task_result 报错回执 —— **错误路径通了就算过**
      4) 再发一条带真 requirement_dir 的(可选,花钱)→ ok:true 回执
- [ ] 2.4 两端对接:把 1.3-4 的 arc.yaml 放进一个 requirement_dir 喂 ARC → 能吃不报格式错

## 三、agent-chat

### A. 不用切换就能测(不碰现场)

- [ ] 3.1 `node scripts/provision-team.mjs --team demo --project <随便一个git仓> --dry-run`
      → 只打印计划,什么都不创建;去掉 --dry-run 真跑 → worktree + 4 个 agent home
      + 打印 4 步后续清单(测完可删)
- [ ] 3.2 配置工具:在 agent-chat 仓库运行下面三行(只改临时文件):
      `cp .env.example /tmp/agent-chat-manual-test.env`
      `node services/configure-standalone-env.mjs --env /tmp/agent-chat-manual-test.env --generate-bridge-secret --agent-token-mode hard`
      `stat -f '%Lp' /tmp/agent-chat-manual-test.env`
      → 只打印 `MATRIX_BRIDGE_SECRET`、`AGENTCHAT_AGENT_TOKEN_MODE` 两个键名,
      **不打印值**,最后权限是 600。测完删掉临时文件。
- [ ] 3.3(在 robrix2 当前 main)`node --test roadmap/agentchat-demo/palpo/tests/*.test.mjs`
      → 28 过 0 挂;5 个 Docker real-e2e 默认跳过,全程不起容器

### B. 完成 standalone supervisor 切换后再测

- [ ] 3.4 在 agent-chat 仓库跑 `node services/standalone-doctor.mjs` → 全绿;
      `ps` 里 **push-relay.js 真的在跑**(历史首次)
- [ ] 3.5 老毛病三连验:
      · 重启全栈 → 房间零重复回复
      · inboxGate:filtered 读不清 gate、full 读清 gate、agent 能回话
      · 盯 15 分钟日志 → 429 零新增
- [ ] 3.6 共享房三件套:
      · `MATRIX_DEFAULT_WAKE=off` 时,不带 @ 闲聊 → **零响应**(设回 auto → coordinator 响应,对照)
      · `!bindroom <组>` → 把现有房间绑上,回 "bound";`!bindroom 不存在的组` → 报 not found
- [ ] 3.7 gh 约定(skill 已恢复新版):`/create-issue 测试 | ...` → GitHub 出 `[wf-NNN]` issue;
      `/go` 走完 → PR 开着**没被自动合并**;`gh auth logout` 再来一单 → 本地照常,GH 记 none

## 四、robrix2

前置:#258 / #259 已合入 main;先检出并更新当前 main。

- [ ] 4.1 邀请 `wf_coordinator` 进房 → 桥 bot 自动跟进
      (#258 后的正确样子:先试邀 `agent-bridge-wf` 失败无害留日志,再邀 `agent-bridge` 成功)
- [ ] 4.2 #253 那批(已在 main):`/invitebot` 斜杠命令、房间绑定 bot 是 UI 偏好不是隐式收件人

> cockpit 当前没有可复现的公共分支或 PR,不放进这份通用清单,也不要在
> 通用测试流程里执行 `git stash pop`。等它有独立 PR 后另加对应手测项。

## 收尾

- [ ] 全部勾完后记录失败现象、日志位置和实际版本;cockpit 等独立 PR 后单独验收。
