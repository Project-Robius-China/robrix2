# 部署指南

> **定位**：本章帮你在两条部署路线之间做选择，并给出动手前的准备清单。前置依赖：第 3 章（知道三层架构各是什么）。

一套完整的 HAgency 由三个组件构成，其中 **Matrix 服务器**可以自建也可以用云端：

| 组件 | 必需 | 部署位置 |
|------|------|---------|
| Matrix homeserver（Palpo） | ✅ | 本地 Docker，**或**云端（Meldry / matrix.org） |
| agent-chat | ✅ | 跑 Agent 的那台机器（backend、dashboard、push relay、Matrix bridge、受管运行时） |
| Robrix2 | ✅ | 你的桌面 |

## 两条路线

- **[本地部署](deploy-local.md)** —— Palpo + agent-chat + Robrix2 全部跑在自己的机器上。数据完全自持，适合开发、内网团队和隐私敏感场景。
- **[云端 Matrix](deploy-cloud.md)** —— Matrix 服务器用 [Meldry](https://tenant.meldry.com/) 上一键创建的托管 Palpo 租户（或 matrix.org 官方节点），本地只跑 agent-chat 和 Robrix2。省去自建 homeserver 的运维成本，天然支持异地成员加入。

> 无论哪条路线，agent-chat 都跑在**你自己**的机器上 —— 编码 Agent 需要访问你的代码仓库和 tmux，这正是「本地优先」的含义。变的只是 Matrix 服务器在哪。

## 动手前的准备清单

| 准备项 | 用途 | 备注 |
|--------|------|------|
| Docker（仅本地路线） | 跑 Palpo + PostgreSQL | 云端路线不需要 |
| Node.js 22+ 与 tmux | 跑 agent-chat 及其管理的运行时 | `node -v` 确认版本；Linux 有正式安装器，macOS 目前按开发方式运行 |
| Rust 工具链 | 构建 Robrix2 | `rustup` 安装即可 |
| Claude Code 或 Codex CLI | 至少一个编码运行时 | 两个都装可体验第 5.5 章的异构终审 |
| 一个要协作的代码仓库 | Agent 的工作对象 | 任意本地 Git 仓库 |

## 部署完成后你会得到什么

按下一章（或再下一章）走完后，你将拥有：一个可登录的 Matrix 服务器；一个在 tmux 里运行、拥有 Matrix 木偶身份的编码 Agent；一个绑定了 Agent group 的**非加密**项目作战室；以及 owner 接受邀请后可用的加密审批房。四角色 workflow 和 Project Board 还需要各自章节列出的额外准备，基础部署不会自动生成它们。

如果中途卡住，先看[运行验收与故障排查](operations.md)。最常见的断点是：必填 secret 缺失、桥的 trust 门禁没配自己的完整 MXID、没有由 owner 亲自邀请实际 Agent、运行时不是受管启动，或审批房邀请尚未接受。
