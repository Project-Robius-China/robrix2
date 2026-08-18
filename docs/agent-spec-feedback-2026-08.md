# agent-spec 改进清单（源自 robrix2 实践，2026-08-18/19）

在 robrix2 上用 agent-spec 1.4.0 走完 #306（PR #320）、防回归基建（PR #321/#324）、#311（PR #325/#326）、#308（PR #328）四个任务后整理。每项给出：**现象 → 建议 → 优先级 → robrix2 中的证据**。

---

## A. 验证语义

| # | 现象 | 建议 | 优先级 |
|---|---|---|---|
| A1 | **`guard` 把 `skip` 视为失败**。robrix2 几乎每个 spec 都有 UI/homeserver 场景绑到 `manual_test_*`，`guard` 一跑 35/45 个 spec"失败"，全部是 skip 而非 fail | 引入一等公民的**人工场景**：`Test: manual` 或 `Verification: manual`（而不是靠命名约定），verdict 独立为 `manual-pending`；`guard/lifecycle` 增加 `--fail-on fail\|fail+skip`（默认 fail+skip，manual 场景不算 skip） | P0 |
| A2 | **人工验收无法记录**。只能在 commit message 里写"verified by user 2026-08-18"，`stamp` 永远 `Spec-Passing: false` | 依赖 A1（现有 answered-envelope 链路只结算 `uncertain`/`pending_review`，覆盖不了无测试的 skip）。`agent-spec attest <spec> --scenario <name>` 写入 run log，记录 verdict / reasoning / evidence digest / spec fingerprint / **外部审批引用**（PR review、commit trailer）；**不带身份字段**（与 agent-spec ADR-001 一致，审批人由外部系统按 digest 绑定）；spec 改动后失效 | P0（随 A1） |
| A3 | **边界层把整个变更集套在每一个 spec 上**。任何 PR 都会让不相关的 spec"越界失败"，repo-wide guard 无法用 | 边界只对**本次改动的 spec**（或 Allowed Changes 与变更集有交集的 spec）生效；其它 spec 只跑测试回归。提供 `--boundary-scope changed-specs\|all` | P0 |
| A4 | `guard --change-scope` 只有 `staged\|worktree`，CI 里对 PR base 做 diff 需要自己 `git diff --name-only base...HEAD` 再逐个 `--change` | 增加 `--change-scope base:<ref>`（或 `--base <ref>`） | P1 |
| A5 | lifecycle 汇总把边界检查算作一个"场景"（10 个场景显示 `passed: 11`） | 汇总里分开 `scenarios` 与 `layers`（lint/boundary/test 各自 pass/fail） | P2 |
| A6 | 测试判定**只按测试名**；spec 步骤文字与 API 漂移（#328 审查 P2：步骤仍写 `finish_create(a, value)`，实现已是 `&token`）lifecycle 发现不了 | ① `identifier-drift` lint：场景步骤里反引号标识符应出现在绑定测试源码/被测函数签名中；② `matrix --with-source` 显示绑定测试的 doc comment 与首行断言，便于 Contract Acceptance 对照 | P1 |
| A7 | **属性测试没有一等地位**，靠命名 `prop_*` 和标题"Property — …"约定 | 场景加 `Kind: property`（或 `Evidence: property`）；verify 识别 proptest/quickcheck 宏并在 matrix 标注；新增 lint `invariant-coverage`：带不变量的 Rule 至少绑 1 个 property 场景 | P1 |
| A8 | **不变量没有正式位置**，只能写在 `## Acceptance Criteria` 顶部的 HTML 注释里 | 支持 `Invariant: <id> — <math>` 行（Rule 之下或独立 `## Invariants` 段）；`explain` 打印；`promote` 随 Rule 搬；`trace` 里 ADR ↔ Invariant ↔ Scenario 可视 | P1 |
| A9 | 变异检验（把修复改回去、确认测试变红）全靠手工，但它是证明"测试真的在守不变量"的关键步骤 | `agent-spec mutate <spec> --patch <diff>`（或 `verify --expect-fail --scenario X`）：应用补丁 → 期望绑定测试失败 → 回滚；结果进 run log/explain | P2 |

## B. 解析与 lint

| # | 现象 | 建议 | 优先级 |
|---|---|---|---|
| B1 | Allowed Changes 里根目录文件必须写 `./Cargo.toml`；`Cargo.toml`、`` `Cargo.toml` `` 都匹配不到 | 路径匹配前去反引号、把裸文件名视为相对仓库根 | P0 |
| B2 | Allowed Changes 条目带括号说明（`` - `Cargo.toml` (dev-dep only) ``）被当整串路径，静默不匹配 | 允许 ` — 说明` 或 `# 说明` 尾注；对"不像路径"的条目 lint 告警 | P1 |
| B3 | `spec: project` 被按任务标准 lint（0%，error 级），guard 直接红 | level-aware lint：project/org 级不评分场景/覆盖率，只查结构 | P1 |
| B4 | `precedence-fallback-coverage` **条件性误报**：触发条件是文本含 `->`（`linters.rs:1491`）。robrix2 复现样例：Constraint "Keep the signature of `` `BotSettingsState::should_create_encrypted_dm(&self, &UserId, Option<&UserId>) -> bool` `` unchanged" 与 Decision "Add `` `AppState::should_create_encrypted_dm(&self, target, current) -> bool` ``"（`specs/task-dm-encryption-default.spec.md` 第 27、38 行）——都是行内代码里的 Rust 返回类型箭头 | 先把上述两条加为回归样例；启发式改为忽略行内代码（反引号）内的 `->`，或要求箭头两侧是自然语言词而非签名 | P2 |
| B5 | `explain --format markdown` 对多行 bullet 的 Decision 只输出第一行 | 渲染时合并续行 | P2 |
| B6 | `bdd-implementation-detail-step` 对"clicks Open chat"这类必须描述 UI 机制的步骤告警 | 对 manual 场景放宽 | P3 |

## C. 命令能力

| # | 现象 | 建议 | 优先级 |
|---|---|---|---|
| C1 | **`promote` 只复制了 `### Rule:` 标题，没带场景**（含 `Tags:` 的场景块），能力 spec 只能手写 | 修 bug；promote 应搬运 Rule 下全部场景（含 Tags/Test 结构化选择器）与不变量 | P0 |
| C2 | `Package:` 选择器对非 workspace 成员失败，报错"cargo exited before any test ran (build/toolchain failure)"无法定位 | 报错说明 "package not in workspace"；支持 `Manifest: tools/x/Cargo.toml` / `Dir:` 选择器在子目录跑 cargo | P1 |
| C3 | `check-structure` 只接受单个 `--in`/`--forbid`，只能靠策略文件 + 脚本循环 | `--in`/`--forbid` 可重复；或 `check-structure --config <file>`（每行 `forbid \| glob`） | P1 |
| C4 | ~~`trace --gate` 依赖 `.agent-spec/runs`~~ **误诊，已由 agent-spec 侧核实**：`trace` 现场调用 `verify_spec_rollup`（`spec_knowledge/trace.rs:60`），不读 run log；代价是每次重跑 cargo test，`spec-guard.sh` 第 3 步 lifecycle + trace 因此双跑 | 文档强调 trace 会现场执行；可选 `trace --from-run-log` 复用最近一次 lifecycle 结果以省时（opt-in） | P3 |
| C5 | `init --workspace` 把 `config.yaml` 放进通常被 gitignore 的 `.agent-spec/` | 生成 `.gitignore` 片段或把 config 放仓库根 `agent-spec.yaml` | P2 |
| C6 | 没有"每个 PR 只跑增量、其余回归"的现成命令，只能自写脚本 | 把 A1/A3/A4 落成 `agent-spec ci [--base ref]`：lint 改动 spec → 结构守卫 → capability + trace → 改动 spec 全量 / 其它回归 | P1 |

## D. 防漂移 / 治理

| # | 现象 | 建议 | 优先级 |
|---|---|---|---|
| D1 | 没有机制阻止"改 spec 让门禁变绿"，只能靠 CODEOWNERS + CLAUDE.md 口头规则 | `explain --history` 标注"spec content changed between failing run and passing run"；`stamp` trailer 加 `Spec-Fingerprint`；guard 可选 `--spec-frozen <ref>` 拒绝改动带 `critical` 的场景 | P1 |
| D2 | 棘轮式阈值（今天的债务 ≤ N，只能降）在 agent-spec 里没有对应物；ux-harness 自己做了 `gate.json` | lint/guard 支持 `--baseline <json>`：记录各 lint 规则/skip 数基线，只允许下降 | P2 |
| D3 | 叠放 PR 时 base 分支合并后内容滞留（#321、#325 都发生），工具无感知 | `stamp` 可写 `Spec-Base: <ref>`；文档提示 stacked PR 风险 | P3 |

---

## robrix2 中可参考的文档与代码

**方法论 / 模板**
- `CLAUDE.md` → "Invariant-driven spec template" 与 "Spec regression gate" 两节
- `specs/project.spec.md` Decisions 里 "Spec invariants" 一条（proptest 批准记录）

**spec 范例**
- `specs/task-dm-encryption-default.spec.md` — 4 条 Rule、不变量注释、属性场景、结构守卫场景、`./Cargo.toml` 写法（A7/A8/B1）
- `specs/task-thread-timeline-lifecycle.spec.md` — 生命周期不变量（守恒/ABA）、token 语义（A7/A8）
- `specs/task-ci-test-and-ux-gate.spec.md` — CI 策略型 spec，`Package: ux-harness` 结构化选择器（C2）
- `specs/capabilities/dm-encryption.spec.md` — 手写的能力 spec（C1 的对照）
- `knowledge/decisions/00001-dm-encryption-default.md` — ADR-001，`satisfies` 与 `trace --gate` 样例（C4）

**门禁与守卫**
- `scripts/spec-guard.sh` — A1/A3/A4/C6 的完整 workaround（skip 容忍、changed-spec 边界、`--base`）
- `specs/structure-guards.txt` — C3 的策略文件形态
- `.github/workflows/main.yml` `spec_gate` job、`.githooks/pre-commit`、`.github/CODEOWNERS`（D1）
- `tools/ux-harness/gate.json` + `tools/ux-harness/src/gate.rs` — 棘轮阈值策略实现（D2）
- `tests/ci_policy.rs` — "门禁自身被测试钉住"的写法

**测试范例（属性测试 + 独立 oracle + 变异检验）**
- `src/app.rs` `mod tests::dm_encryption_props`（`E(t) ⇔ ¬B(t)`，独立 oracle）
- `src/sliding_sync.rs` `mod thread_timeline_table_tests`（generation 感知的参考模型、重放历史 token）
- `src/home/room_screen/thread_lifecycle.rs` `mod tests::props`（push/pop 序列，"最后引用消失才关闭"）
- `src/home/room_screen/state.rs` `TimelineStateCache` 测试（按 kind 失效）
- `src/app.rs` `dm_entry_points_do_not_hardcode_plaintext`（源码扫描型守卫测试）

**PR 描述（含验证表与教训）**
- #320（Verification 表、manual 场景标注）、#324（"为什么 stock guard 不能用"）、#326（`Package:`/workspace 决策）、#328（不变量块 + ABA 修复与变异检验）
