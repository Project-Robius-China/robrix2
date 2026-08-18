# Matrix Space 补齐设计(切片 B 收尾)

> 状态:已批准(2026-08-18)· 范围:纯 Matrix Space 能力,不含 HAFleet Phase 4 绑定
> 依据:matrix-project-space-roadmap.md + 2026-08-17 三组并行代码核实(全部判定有 file:line 证据)

## 背景与目标

roadmap(基线 2026-08-06)大半 P0 已被 PR #292 补齐。核实后的真实剩余:3 个代码缺陷、
4 个功能缺失、space 核心逻辑零测试。本设计以两个 PR 交付:

- **PR-1 `fix/space-tree-hardening`**:缺陷修复 + 首批单测(先行,低风险)
- **PR-2 `feat/space-usability`**:四项功能补齐

非目标:restricted join rule 配置器(有意设计,留 Phase 4 语境)、拖拽排序、父链显示、
"已离开空间"完整管理页、ProjectRef 绑定。

## PR-1:修复 + 测试

### 1. 树递归环保护(崩溃级)
`space_lobby.rs` 的 `build_tree_for_space` / `build_filtered_tree` / `subtree_has_match` /
`build_tree_for_space_ignoring_expansion` 与 `rooms_list.rs::is_room_indirectly_in_space`
均为纯递归,`m.space.child` 成环协议合法 → 栈溢出。

- 统一加 `visited: HashSet<OwnedRoomId>`。选 visited 而非深度上限:深度上限误伤合法深树,
  visited 精确且 O(n)。
- 树构建核心重构为接收显式输入(children 映射、展开集、过滤词)的自由函数,
  脱离 Makepad Widget 可单测。service 层任务级防护与未读聚合防环已存在,不动。

### 2. 侧边栏顺序稳定
`spaces_bar.rs` 增量路径顺序稳定,但 `update_displayed_spaces` 两处(搜索清空、过滤回退)
从 `HashMap.keys()` 重建 `displayed_spaces`,顺序漂移。

- `all_joined_spaces: HashMap` → `IndexMap`(insertion order;`indexmap` 已是项目依赖,
  不新增依赖)。重建路径顺序 = 插入序 = 增量路径顺序。

### 3. 挂载部分失败语义
`attach_room_to_space`:`m.space.child` 成功 + `m.space.parent` 失败 → 整体报"失败"且
UI 不刷新,与服务端状态不一致。

- 照搬 `detach_room_from_space` 已验证的 best-effort 模式:主写入(`m.space.child`)成功
  即成功,反向写入失败降级 `warning!`;
- `SpaceChildAction::Added` 无论成败都触发 `refresh_space_children`,UI 收敛到服务端状态。

### 测试(随 PR-1)
环(A→B→A)、自环、重复边、深树的树构建;未读聚合环/共享房间(补既有逻辑的缺失覆盖);
IndexMap 重建顺序 = 插入序。

## PR-2:四项功能

### 4. selected Space 持久化
`AppState` 增 `selected_space_id: Option<OwnedRoomId>`(`serde(default)` 兼容老状态文件)。
空间列表异步到达后应用恢复;目标空间已退出/不可见 → 安全回退 Home(沿用 room 恢复的
待恢复暂存模式)。恢复后 `saved_dock_state_per_space` 既有 dock 布局自然生效。

### 5. Space 深链接
`interactions.rs` 的 `MatrixId::Room` 分支先查 `client.get_room()` room type:
已加入 space → `NavigationBarAction::GoToSpace` 进 Space Lobby;本地可知是 space 但未加入
→ join modal 带 `is_space: true`;**本地完全未知的房间维持现有普通流程**(决策 A:
不发预览请求探测,保守渐进)。

### 6. banned/left/knocked 反馈
用现有 `enqueue_popup_notification` 做最小可感知反馈:被 ban / 远程离开时弹通知
"你已不在空间 X";knocked 维持忽略但注释写明原因。**本轮不做重新加入/取消 knock 入口**
(决策 B,放下轮)。

### 7. 目录页 Spaces 切换
`directory_screen.rs` 的 `kind` 参数化;页头加 Rooms/Spaces 切换(现有 toggle 样式 +
RBX token);空态文案区分;`en.json`/`zh-CN.json` 同步。

## 横切约束

- Matrix 写操作一律 `submit_async_request(MatrixRequest::*)`
- Makepad 2.0 语法(script_mod! / := / +:);禁 `cargo fmt`;UI 色值走 RBX token
- 两份 task spec 继承 `specs/project.spec.md`,`agent-spec lint --min-score 0.7`
- PR 推送后等用户实测,不自动合并

## 验收

单测随 PR-1;人工验收覆盖桌面/移动端、双账号权限、与 Element 交叉验证层级。
详见 `specs/task-space-tree-hardening.spec.md` 与 `specs/task-space-usability.spec.md`。
