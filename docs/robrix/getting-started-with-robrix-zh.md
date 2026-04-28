# Robrix 快速开始

[English Version](getting-started-with-robrix.md)

> **目标：** 按照本指南操作后，你将完成 Robrix 的安装和运行，连接到 Matrix 服务器，并可以开始聊天。

Robrix 是一个用 Rust 编写的跨平台 Matrix 聊天客户端，基于 [Makepad](https://github.com/makepad/makepad/) UI 框架。原生运行在 macOS、Linux、Windows、Android 和 iOS 上。

## 下载预编译版本（推荐）

从 [Robrix 发布页面](https://github.com/Project-Robius-China/robrix2/releases) 下载最新版本。支持 macOS、Linux 和 Windows。

## 从源码构建

### 前提条件

- [Rust](https://www.rust-lang.org/tools/install)（最新稳定版）
- Linux 上需要安装系统依赖：
  ```bash
  sudo apt-get install libssl-dev libsqlite3-dev pkg-config libxcursor-dev libx11-dev libasound2-dev libpulse-dev libwayland-dev libxkbcommon-dev
  ```

### 桌面端（macOS / Linux / Windows）

```bash
git clone https://github.com/Project-Robius-China/robrix2.git
cd robrix2
cargo run --release
```

### 移动端

Android 和 iOS 构建方法请参考 [Robrix README — 构建与运行](https://github.com/Project-Robius-China/robrix2#building--running-robrix-on-desktop)。

---

## 连接 Matrix 服务器

启动 Robrix 后，登录界面底部有一个 **Homeserver URL** 输入框。

<img src="../images/login-screen.png" width="600" alt="Robrix 登录界面">

- **留空** 默认连接 `matrix.org`（公共服务器）
- **输入自定义 URL** 连接其他 Matrix 兼容服务器：
  - 本地 Palpo 实例：`http://127.0.0.1:8128`
  - 远程服务器：`https://your.server.name`

> **注意：** Robrix 要求主服务器支持 [Sliding Sync](https://spec.matrix.org/latest/client-server-api/#sliding-sync)。Palpo 原生支持此功能；其他服务器请查阅其文档。

## 注册或登录

**新账号（服务器允许注册时）：**

1. 输入**用户名**和**密码**
2. 确认密码
3. 设置 **Homeserver URL**
4. 点击 **Sign up**

**已有账号：**

1. 输入**用户名**和**密码**
2. 设置 **Homeserver URL**
3. 点击 **Log in**

登录后你会看到房间列表。你可以加入房间、创建新房间并开始聊天。

---

## 下一步？

- **只是聊天？** 你已经准备好了——加入房间，和 Matrix 网络上的人交流。
- **想要 AI 机器人？** 查看 [Robrix + Palpo + Octos 部署指南](../robrix-with-palpo-and-octos/01-deploying-palpo-and-octos-zh.md)，搭建你自己的 AI 聊天系统。
