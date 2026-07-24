# Robrix2 简介

> **定位**：本章介绍 Robrix2 的技术形态与上手方式。无前置依赖；只想部署 HAgency 的读者可跳到第 4 章，需要时回看。

[Robrix2](https://github.com/Project-Robius-China/robrix2) 是一个用 **Rust** 编写的原生 Matrix 客户端，UI 层基于 [Makepad 2.0](https://github.com/makepad/makepad) —— 一个 GPU 渲染的 Rust UI 框架。它是 Robius 跨平台应用生态的旗舰项目之一。

## 技术形态

- **全栈 Rust**：客户端逻辑基于 [matrix-rust-sdk](https://github.com/matrix-org/matrix-rust-sdk)（与 Element X 同源的官方 SDK），UI 由 Makepad 着色器驱动渲染。没有 Electron，没有 WebView —— 更低的内存占用和真正的原生启动速度。
- **跨平台**：同一套代码运行在 macOS、Windows、Linux，并可打包到移动端。
- **Sliding Sync**：使用 Matrix 新一代同步协议，房间列表与时间线按需加载，账号下有几百个房间也能快速冷启动。
- **多标签工作区**：房间、Thread、私聊以标签页（Tab/Dock）形式并排打开。这是它与传统单栏 IM 最大的交互差异，为「同时盯多个协作现场」而设计 —— 第二部分你会看到这个设计的真正用途。

## 快速上手

```bash
git clone https://github.com/Project-Robius-China/robrix2.git
cd robrix2
cargo run            # 常规运行
cargo run -- --hot   # 带热重载的开发模式
```

首次启动后使用任意 Matrix 账号登录：可以是你自建 homeserver 上的账号（见第 4 章），也可以是 matrix.org 等公共节点的账号。

## 与其他 Matrix 客户端的关系

Robrix2 是标准 Matrix 客户端 —— 不依赖任何私有服务端扩展。你可以用 Element 登录同一个账号验证消息互通；反过来，HAgency 的大部分协作（房间、Thread、@提及）在任何 Matrix 客户端里都可见。Robrix2 额外提供的是**原生的 Agent 体验**：审批卡片、Agent 徽标、workflow 命令补全 —— 这些在通用客户端里会退化为纯文本。
