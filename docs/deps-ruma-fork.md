# 依赖决策:ruma fork 分支 `tsp-lax-syncv5-deser`

> 状态:实验验证中(PR #297)· 建立日期:2026-08-14 · 负责分支:`fix/ruma-lax-syncv5-deser`

## TL;DR

robrix2 的 `[patch."https://github.com/ruma/ruma"]` 从 `project-robius/ruma@tsp` 切换到
**`Project-Robius-China/ruma@tsp-lax-syncv5-deser`**,并在 ruma 依赖上启用
**`unstable-compat-lax-syncv5-deser`** feature。目的:修复"robrix2 运行期间收到的
邀请 / 服务端新建房间永远不显示"的 sliding-sync 房间物化 bug,同时**完全不动**
matrix-sdk fork。

## 分支构成

```
Project-Robius-China/ruma @ tsp-lax-syncv5-deser (HEAD = 20c2c60f4)
│
├─ 20c2c60f4  cherry-pick of ruma#2510 (官方 e8b924f2)
│             "client-api: Handle non-compliant/corrupt fields in
│              simplified sliding sync responses"
│             唯一冲突是 CHANGELOG.md,代码零冲突
│
└─ 98196b1db  ← 基底 = project-robius/ruma@tsp 在 robrix2 Cargo.lock
              里**原本就锁定**的那个 rev(ruma 0.14.1,含全部 TSP 补丁)
```

关键性质:**基底一个字节都没变**——就是切换前 lock 里锁的那个 commit,
所以 matrix-sdk fork 看到的 ruma API 与之前完全一致,唯一的增量是那一个
cherry-pick。ruma 版本保持 0.14.1,无任何 API churn。

## 依赖链全景

```
robrix2
├── matrix-sdk{,-base,-ui}  ← Project-Robius-China/matrix-rust-sdk
│        │                    @ space_room_suggested-event-cache-no-panic(不变)
│        └── ruma(声明指向 ruma/ruma@a0acf4187)
│              │
│              └─[patch."https://github.com/ruma/ruma"]─►
│                   Project-Robius-China/ruma @ tsp-lax-syncv5-deser   ← 本决策
└── ruma(直接依赖,同样被 patch 重定向;features 见下)
```

robrix2 直接依赖上启用的 ruma features:`compat-optional`、`compat-unset-avatar`、
**`unstable-compat-lax-syncv5-deser`(新增)**。Cargo feature 统一化保证
matrix-sdk-base 内部使用的同一份 ruma 实例也获得该行为。

## 为什么这么做(决策链)

1. **要修的 bug**(详见 memory `project_robrix_invite_sliding_sync_bug`):
   robrix2 开着时到达的邀请/新房间,在 sliding sync v5 增量响应中被引用,
   但 per-room 对象反序列化失败被静默丢弃 → matrix-sdk-base 不建 Room →
   `must exist` 报错、UI 永不显示,且 pos 越过后重启也救不回。
2. **上游 robrix 的修法**(提交 `6363b5f8`):整体切回官方 matrix-rust-sdk +
   新 ruma,并启用 `unstable-compat-lax-syncv5-deser`(ruma#2510:sync v5
   响应中 room/hero 的 name/avatar 等字段损坏时忽略该字段,而不是让整个
   room 对象反序列化失败)。
3. **我们不能整体跟进 SDK**(2026-08-14 实验确认,两个硬冲突):
   - 官方 SDK 已移除基于 `ring` 的 rustls provider,只剩 `rustls-aws-lc-rs`;
     与 robrix2 为绕过 Android 上 `rustls-platform-verifier`/aws-lc панic 而
     精心固定的 `ring` provider 方案直接冲突(见 Cargo.toml rustls 注释)。
   - `matrix-sdk-sqlite` 新版要求 `libsqlite3-sys ^0.38`,与 TSP 链
     (`tsp_sdk → aries-askar → sqlx` 补丁分支)的旧版本要求撞 native `links`,
     需要再 fork 一轮 sqlx,维护成本高。
4. **因此取最小增量**:只把 ruma#2510 这一个修复 cherry-pick 到我们已锁定的
   ruma rev 上。风险面 = 一个上游已合并的补丁。
5. **为什么放 Project-Robius-China 而不是 project-robius**:下游实验分支应放
   自己可控的 org(matrix-sdk fork 也在这里),不污染上游仓库、不怕被清理。

## 维护规则

- **不要 rebase / force-push 该分支**:robrix2 的 Cargo.lock 以
  branch+commit 锁定,force-push 会让全体开发者 `cargo update -p ruma` 后
  构建结果漂移。要加补丁就在分支顶端继续 cherry-pick(追加式)。
- **不要让该分支自动跟踪上游 tsp**:基底钉死在 98196b1d 是本决策的核心
  性质(保证 matrix-sdk fork 兼容)。上游 tsp 已升到 ruma 0.16,直接合并
  会破坏 matrix-sdk 0.14 时代的 API 兼容。
- **退役条件**:未来 matrix-sdk fork 升级/切官方(需先解决上面第 3 点的两个
  硬冲突)时,官方 ruma 自带 ruma#2510,本分支与本 patch 应一并删除,
  feature 改在新 ruma 上直接启用。
- 更新 lock:`cargo update -p ruma`(仅在分支追加补丁后需要)。
- 网络注意:该仓库首次 HTTPS 拉取在部分网络环境下会被掐断,已知 workaround
  是 git 全局配置 `url.git@github.com:Project-Robius-China/ruma.git.insteadOf
  https://github.com/Project-Robius-China/ruma.git` 走 SSH。

## 验证方法(人工)

1. robrix2 保持运行,用另一账号邀请当前用户 / 由服务端 bot 建房并拉人。
2. 期望:房间**无需重启**即出现在 Invites/房间列表(修复前:永不出现,
   日志报 `The room must exist since it has been joined`)。
3. 回归面:正常收发消息、房间列表增量更新、启动全量同步(`share_pos(false)`
   路径)不受影响——该 feature 只在"字段损坏"时改变行为,健康响应零差异。

关联:`specs/task-ruma-lax-syncv5-deser.spec.md`(任务合约)、PR #297、
上游参考 `robrix@6363b5f8`、`ruma/ruma#2510`。
