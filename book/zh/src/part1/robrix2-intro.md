# Robrix2 简介

> **定位**：本章介绍 Robrix2 的技术形态与上手方式。无前置依赖；只想部署 HAgency 的读者可跳到第 4 章，需要时回看。

[Robrix2](https://github.com/Project-Robius-China/robrix2) 是一个用 **Rust** 编写的原生 Matrix 客户端，UI 层基于 [Makepad 2.0](https://github.com/makepad/makepad) —— 一个 GPU 渲染的 Rust UI 框架。它是 Robius 跨平台应用生态的旗舰项目之一。

这里的“原生”描述的是技术栈与渲染路径，不是一条未经测试的性能广告。Robrix2 的重点是让 Matrix 房间、Thread、设备加密和多标签工作区在同一桌面应用中协同工作，并在标准事件之上增加 Agent 交互视图。

## 技术形态

- **全栈 Rust**：客户端逻辑基于 [matrix-rust-sdk](https://github.com/matrix-org/matrix-rust-sdk)，UI 由 Makepad 着色器驱动渲染，不依赖 Electron 或 WebView。具体内存与启动性能应以目标平台实测为准。
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

Robrix2 是标准 Matrix 客户端。你可以用 Element 登录同一个账号验证普通消息互通；HAgency 的房间、Thread 与 @提及在兼容客户端里可见。Robrix2 额外提供原生审批卡片、Agent 徽标与 workflow 文本补全。通用客户端可能只显示 custom event 的 fallback/raw 内容，**不能用普通文字完成 owner approval**；审批需要能发送结构化 verdict 的 Robrix2 或兼容 UI。

这也给互操作性划出边界：公开协作应尽量使用标准 `m.room.message` 与 Matrix relation；授权等安全动作可以使用扩展事件，但必须由服务端校验，而不能依赖某个客户端画出的按钮。
