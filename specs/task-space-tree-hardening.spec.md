spec: task
name: "Space tree hardening: cycle guards, stable order, attach semantics"
inherits: project
tags: [bugfix, space, hierarchy, testing]
estimate: 1d
---

## Intent

修复 2026-08-17 核实确认的三个 Space 缺陷并建立首批 space 核心逻辑单测:space_lobby 树递归无环保护(`m.space.child` 成环协议合法,现状会栈溢出)、spaces_bar 在搜索清空/过滤回退路径从 HashMap 重建导致顺序漂移、attach_room_to_space 部分失败被误报为完全失败且 UI 不刷新。设计依据 docs/superpowers/specs/2026-08-18-space-completion-design.md。

## Decisions

- 环保护用 `visited: HashSet<OwnedRoomId>`,不用深度上限(深度上限误伤合法深树);覆盖 space_lobby.rs 四个递归函数(`build_tree_for_space`、`build_filtered_tree`、`subtree_has_match`、`build_tree_for_space_ignoring_expansion`)与 rooms_list.rs 的 `is_room_indirectly_in_space`
- 树构建核心重构为接收显式输入(children 映射、展开集、过滤词)的自由函数,脱离 Makepad Widget 可单测
- spaces_bar 的 `all_joined_spaces` 由 `HashMap` 改为 `indexmap::IndexMap`(insertion order;`indexmap` 已是项目依赖,不新增依赖),重建路径顺序与增量路径一致
- `attach_room_to_space` 采用与 `detach_room_from_space` 相同的 best-effort 模式:`m.space.child` 写成功即主操作成功,`m.space.parent` 失败降级 `warning!` 日志
- `SpaceChildAction::Added` 无论成败都触发 `refresh_space_children`,UI 收敛到服务端状态
- service 层任务级防护(space_room_list_tasks 去重)与未读聚合防环(accumulate_space_unread)已实现且正确,不改动

## Boundaries

### Allowed Changes
- src/home/space_lobby.rs
- src/home/spaces_bar.rs
- src/home/rooms_list.rs
- src/sliding_sync.rs
- specs/task-space-tree-hardening.spec.md
- docs/superpowers/specs/2026-08-18-space-completion-design.md

### Forbidden
- 不新增 cargo 依赖(IndexMap 用既有 indexmap crate)
- 不改动 space_service_sync.rs 的既有循环防护与订阅逻辑
- 不修改 UI 视觉样式与 DSL 模板(本任务纯逻辑层)
- 不运行 cargo fmt

## Out of Scope

- selected Space 持久化、深链接、banned/left/knocked 反馈、目录页切换(task-space-usability)
- restricted join rule 配置器、拖拽排序、父链显示
- detach_room_from_space(已是正确的 best-effort 实现)

## Completion Criteria

### Rule: cycle-safety — 层级成环不崩溃

场景: 双节点环不栈溢出(critical)
  标签: critical
  测试: space_tree_cycle_two_nodes_terminates
  假设 children 映射中 space A 含子 B 且 space B 含子 A,两者均在展开集中
  当 构建 space A 的树
  那么 构建在有限步内终止且 B 只作为 A 的子节点出现一次

场景: 自环被忽略
  测试: space_tree_self_loop_ignored
  假设 children 映射中 space A 含子 A 自身
  当 构建 space A 的树
  那么 构建终止且 A 的子节点中不含 A 自身

场景: 重复边去重
  测试: space_tree_duplicate_edges_deduplicated
  假设 children 映射中 space A 含两条指向同一子房间 R 的边
  当 构建 space A 的树
  那么 R 在树中仅出现一次

场景: 深层合法树不被误伤
  测试: space_tree_deep_chain_fully_built
  假设 children 映射为 50 层无环链式嵌套且全部展开
  当 构建根 space 的树
  那么 全部 50 层节点均出现在树中

场景: 过滤匹配在环上终止
  测试: space_tree_filter_terminates_on_cycle
  假设 children 映射含 A↔B 环
  当 以任意过滤词执行子树匹配检查
  那么 检查在有限步内返回

### Rule: stable-order — 侧边栏顺序稳定

场景: 重建顺序等于插入序(critical)
  标签: critical
  测试: spaces_bar_rebuild_preserves_insertion_order
  假设 依次加入 space C、A、B
  当 从全量集合重建显示列表(模拟搜索清空路径)
  那么 显示顺序仍为 C、A、B

场景: 搜索后清空不打乱顺序
  测试: manual_test_spaces_bar_order_survives_search_clear
  层级: manual
  审核: human
  假设 侧边栏有 5 个以上空间
  当 在搜索框输入过滤词后再清空
  那么 空间顺序与搜索前一致

### Rule: attach-semantics — 挂载失败语义与服务端一致

场景: 反向写入失败不否定主写入
  测试: manual_test_attach_partial_failure_reports_success
  层级: manual
  审核: human
  假设 用户对父 space 有 m.space.child 写权限但对子房间无 m.space.parent 写权限
  当 把该房间挂载进 space
  那么 UI 显示挂载成功且该房间出现在 space 层级中
  但是 日志含 m.space.parent 写入失败的 warning

场景: 挂载后 UI 收敛服务端状态
  测试: manual_test_attach_always_refreshes_children
  层级: manual
  审核: human
  假设 任一挂载操作完成(成功或失败)
  当 SpaceChildAction::Added 到达 UI
  那么 space 子项列表触发 refresh_space_children 重新读取服务端状态

场景: 挂载重试幂等
  测试: manual_test_attach_retry_idempotent
  层级: manual
  审核: human
  假设 某房间已成功挂载进 space
  当 对同一房间再次执行挂载
  那么 不产生重复子项且不报错

<!-- lint-ack: error-path — cycle-safety 全组即异常输入路径(环/自环/重复边为非法或极端层级数据);attach 组含权限不足失败路径 -->
