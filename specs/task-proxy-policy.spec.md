spec: task
name: "Proxy Policy Unified Source Of Truth"
inherits: project
tags: [proxy, network, matrix, persistence]
---

## Intent

统一 Robrix2 的网络代理策略，消除 GUI 保存配置与进程继承环境变量同时生效导致的请求路径不一致。GUI 保存的 `proxy_state.json` 是唯一来源：关闭代理必须强制无代理，开启代理必须通过同一套策略影响 Robrix 发起的 HTTP/Matrix 请求，并且本地 homeserver 不能被外部代理环境劫持。

## Decisions

- Source of truth: `proxy_state.json` / GUI 保存值优先于 shell 或系统继承的 `http_proxy`、`https_proxy`、`all_proxy`、`NO_PROXY`
- `proxy_url = null` 表示强制无代理，启动和保存时都必须清理进程代理环境
- `proxy_url = Some(...)` 表示启用代理，必须设置 `http_proxy`、`https_proxy`、`all_proxy` 与 `NO_PROXY`/`no_proxy`
- Proxy bypass baseline: 只包含通用 loopback: `localhost`、`127.0.0.1`、`::1`
- 本地 homeserver 地址不在代码中硬编码，也不从 homeserver URL 隐式追加 bypass；如未来需要配置非 loopback bypass，应作为 GUI 配置项进入 `proxy_state.json`
- Matrix SDK client、homeserver discovery reqwest client、直接下载 reqwest client 必须复用同一 proxy policy helper
- 显式 reqwest proxy 必须带相同 no-proxy bypass 规则；无 GUI proxy 时显式禁用 reqwest system proxy
- 不新增 Cargo 依赖，不运行 `cargo fmt`

## Boundaries

### Allowed Changes
- src/proxy_config.rs
- src/sliding_sync.rs
- src/persistence/matrix_state.rs
- src/login/login_screen.rs
- src/settings/settings_screen.rs
- src/tsp/mod.rs
- src/updater.rs
- specs/task-proxy-policy.spec.md

### Forbidden
- 不要修改 Matrix session 删除策略或 token 失效判断
- 不要重构登录、设置页的 UI 布局
- 不要添加新的 Cargo 依赖
- 不要运行 `cargo fmt` 或 `rustfmt`

## Completion Criteria

Scenario: 启动时无保存代理会清理继承环境
  Test: proxy_state_none_clears_inherited_env_proxy_vars
  Level: unit
  Test Double: serialized process env snapshot
  Targets: src/proxy_config.rs
  Given 进程环境里存在旧的 `http_proxy`、`https_proxy`、`all_proxy` 和 `NO_PROXY`
  And `proxy_state.json` 不存在或 `proxy_url` 为 null
  When Robrix 启动并应用保存的代理配置
  Then 旧代理环境变量被清理
  And 后续 HTTP client 不会自动继承 system proxy

Scenario: 保存关闭代理会立即强制无代理
  Test: save_proxy_url_none_clears_env_proxy_vars
  Level: unit
  Test Double: temp proxy_state file and serialized process env snapshot
  Targets: src/proxy_config.rs
  Given 进程环境里存在旧的代理变量
  When GUI 保存 `proxy_url = null`
  Then `proxy_state.json` 保存 null
  And 进程代理环境变量被清理

Scenario: 保存开启代理会设置统一环境和 bypass
  Test: save_proxy_url_some_sets_proxy_env_and_bypass_rules
  Level: unit
  Test Double: temp proxy_state file and serialized process env snapshot
  Targets: src/proxy_config.rs
  Given GUI 输入代理 "http://127.0.0.1:7890"
  When 保存代理配置
  Then `http_proxy`、`https_proxy`、`all_proxy` 都等于该代理
  And `NO_PROXY` 或 `no_proxy` 包含 localhost 和 loopback bypass 规则

Scenario: 显式 reqwest client 遵循无代理策略
  Test: build_policy_reqwest_client_disables_system_proxy_when_proxy_is_none
  Level: unit
  Test Double: serialized process env snapshot
  Targets: src/proxy_config.rs, src/sliding_sync.rs
  Given shell 环境中存在 `http_proxy`
  And GUI proxy policy 为 None
  When Robrix 构建 homeserver discovery 或下载使用的 reqwest client
  Then client builder 显式禁用 system proxy
  And 本地 homeserver 请求不会被旧环境代理污染

Scenario: 显式 reqwest proxy 只包含最小 loopback bypass
  Test: build_policy_reqwest_client_attaches_no_proxy_bypass_for_local_addresses
  Level: unit
  Test Double: reqwest proxy debug representation
  Targets: src/proxy_config.rs, src/sliding_sync.rs
  Given GUI proxy policy 为 "http://127.0.0.1:7890"
  When Robrix 构建 homeserver discovery 或下载使用的 reqwest client
  Then 显式 proxy 使用相同代理 URL
  And no-proxy bypass 包含 localhost、127.0.0.1、::1
  And 不包含硬编码私有网段或具体局域网 homeserver IP

Scenario: 无效代理 URL 被拒绝
  Test: discovery_http_client_rejects_invalid_proxy_override
  Level: unit
  Test Double: in-memory client builder construction
  Targets: src/proxy_config.rs, src/sliding_sync.rs
  Given GUI 或登录页输入代理 "ftp://proxy.invalid"
  When Robrix 构建 discovery HTTP client
  Then 构建失败并报告不支持的 proxy scheme

Scenario: cargo build passes
  Test: cargo_build
  Level: integration
  Targets: cargo build
  Given proxy policy 统一化改动完成
  When 运行 `cargo build`
  Then 构建通过

## Out of Scope

- Palpo 服务端配置或部署修改
- auto-login PR 的 session 删除策略修改
- 代理认证 UI 的视觉调整
- 支持 PAC、WPAD 或平台系统代理发现
