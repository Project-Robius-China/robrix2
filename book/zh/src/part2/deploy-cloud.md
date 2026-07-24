# 使用云端 Matrix：Meldry 或官方节点

> **定位**：本章把架构中的「通信底座」换成云端，本地只保留 agent-chat 与 Robrix2。前置依赖：第 4 章路线选择。

不想自己维护 homeserver？agent-chat 和 Robrix2 的部署方式不变（仍按[上一章](deploy-local.md)第 2、3 步进行），只是 Matrix 服务器地址指向云端。

## 方案 A：Meldry —— 一键创建自己的 Matrix 服务器

[Meldry](https://tenant.meldry.com/) 是基于 Palpo 的托管 Matrix 服务：注册后即可**创建一个属于自己的 Matrix 服务器（租户）**，拥有独立的服务器域名，无需任何运维。

1. 打开 <https://tenant.meldry.com/> 注册账号；
2. 创建自己的租户（Matrix 服务器），获得独立的服务器地址；
3. 在该服务器上准备两类账号：你的人类账号（用 Robrix2 注册界面创建）、agent-chat 桥机器人账号（用户名由 `.env` 的 `MATRIX_BOT_USERNAME` 决定，默认 `agent-bridge`，可自定义加后缀；开放注册的服务器上桥会自动注册，否则需预注册或配置注册 token）；
4. 修改 agent-chat `.env`：Matrix 服务器地址与桥机器人凭据指向你的 Meldry 租户；
5. Robrix2 登录时 Homeserver 填你的租户地址。

这条路线兼顾「自己的服务器」（独立域名、独立数据、独立管理）与「零运维」。

## 方案 B：matrix.org 官方节点

也可以直接使用 [matrix.org](https://matrix.org) 等公共 homeserver：注册人类账号与桥机器人账号，agent-chat `.env` 与 Robrix2 登录都指向 `https://matrix.org`。

注意两点：

- 公共节点有注册与发消息的**速率限制**。桥机器人消息较频繁，可能触发限流（agent-chat 的桥内置速率控制，但体验仍不如专属服务器）；
- 非加密房间的协作数据存储在公共服务器上；审批私聊始终端到端加密，服务器无法读取内容 —— 这条保证与服务器归属无关。

## 怎么选

| | 本地 Palpo | Meldry 租户 | matrix.org |
|---|---|---|---|
| 数据自持 | ✅ 完全 | ◐ 独立租户 | ✗ |
| 运维成本 | 需要 Docker | 零 | 零 |
| 跨设备/跨网络访问 | 需自行暴露 | ✅ 天然公网 | ✅ |
| 速率限制 | 自己说了算 | 宽松 | 严格 |

个人开发用本地 Palpo 最省心；要让异地队友（人类，或另一台机器上别人的 Agent 团队）加入同一空间时，Meldry 租户是最平衡的选择 —— 第 5.2 章截图里两支异地 Agent 团队同房协作，靠的就是一个公网可达的 Matrix 服务器。
