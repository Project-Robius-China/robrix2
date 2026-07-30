# 使用云端 Matrix：Meldry 或官方节点

> **定位**：本章把架构中的「通信底座」换成云端，本地只保留 agent-chat 与 Robrix2。前置依赖：第 4 章路线选择。

不想自己维护 homeserver？agent-chat 和 Robrix2 的部署方式不变（仍按[上一章](deploy-local.md)第 2、3 步进行），只是 Matrix 服务器地址指向云端。

## 方案 A：Meldry 托管 Matrix

[Meldry](https://tenant.meldry.com/) 提供基于 Palpo 的托管 Matrix 租户。服务能力、数据边界、限流和价格应以其当前官方说明为准；本书只依赖它提供一个可用的 Client-Server API 与可创建所需账号的前提。

1. 打开 <https://tenant.meldry.com/> 注册账号；
2. 创建自己的租户（Matrix 服务器），获得独立的服务器地址；
3. 准备三类账号：人类账号、bridge bot，以及每个 Agent 的 `@ac_*` 木偶账号。服务器必须允许 agent-chat 通过 registration token / 支持的注册流程创建后两类账号，或者由管理员预创建；
4. 修改 agent-chat `.env`：Matrix 服务器地址与桥机器人凭据指向你的 Meldry 租户；
5. Robrix2 登录时 Homeserver 填你的租户地址。

完成后仍按本地部署章的顺序：配置 secret → 启动 agent-chat → 人类 owner 邀请每个实际 Agent → 绑定项目房间。托管 homeserver 不会替你建立 owner。

## 方案 B：matrix.org 官方节点

也可以评估 [matrix.org](https://matrix.org) 等公共 homeserver。但“能注册一个人类账号”不等于“允许自动注册 bridge 和多个 `@ac_*` 木偶账号”；如果节点不给 registration token、也不提供兼容的开放注册流程，必须先由管理员预创建账号，否则不能按本书流程部署。

注意两点：

- 公共节点有注册与发消息的**速率限制**。桥机器人消息较频繁，可能触发限流（agent-chat 的桥内置速率控制，但体验仍不如专属服务器）；
- 当前项目作战室必须是非加密房，内容会存储在其 homeserver，并可能随联邦传播；审批房的正文使用 E2EE，但服务器仍能看到成员、时间、事件大小等元数据。

## 怎么选

| | 本地 Palpo | Meldry 租户 | matrix.org |
|---|---|---|---|
| 数据控制 | 自行托管 | 取决于服务条款 | 公共服务条款 |
| homeserver 运维 | 自己负责 | 服务商负责 | 服务商负责 |
| 跨设备/跨网络访问 | 需自行暴露 | ✅ 天然公网 | ✅ |
| 注册与限流 | 自己配置 | 先核对租户策略 | 先核对公共节点策略 |

选择标准不是品牌，而是四个可验证条件：允许所需账号 provisioning、Client-Server API 稳定、限流能承受 bridge 轮询、你接受非加密项目数据的存储位置。跨 homeserver 协作还要求双方正确配置公网 DNS、TLS 与 Matrix federation。

若把 dashboard 暴露到公网，应放在 HTTPS 反向代理后，设置 `AGENT_CHAT_WEB_URL`，并继续把 backend API 保持在 loopback，除非你明确部署了额外的访问控制。
