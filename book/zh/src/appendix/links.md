# 附录：项目与资源

> **定位**：全书涉及的项目地址、云服务与相关技术索引，以及本书自身的构建方式。

## 核心项目

| 项目 | 地址 | 说明 |
|------|------|------|
| Robrix2 | <https://github.com/Project-Robius-China/robrix2> | Rust + Makepad 的 Matrix 客户端 |
| agent-chat | <https://github.com/ZhangHanDong/agent-chat> | 本地优先的 Agent 协调系统与 Matrix 桥 |
| Palpo | <https://github.com/palpo-im/palpo> | Rust 编写的 Matrix homeserver |

## 云服务

- **Meldry**（托管 Palpo 租户，可创建自己的 Matrix 服务器）：<https://tenant.meldry.com/>
- **Matrix 官方节点**：<https://matrix.org>

## 相关技术

- Matrix 协议规范：<https://spec.matrix.org/>
- matrix-rust-sdk：<https://github.com/matrix-org/matrix-rust-sdk>
- Makepad（Rust UI 框架）：<https://github.com/makepad/makepad>
- Robius（Rust 跨平台应用生态）：<https://github.com/project-robius>
- Claude Code：<https://claude.com/claude-code>
- Codex CLI：<https://developers.openai.com/codex>
- AtomGit OpenAPI：<https://docs.openatom.tech/en/category/api/>

## 本书构建

本书使用 [mdBook](https://rust-lang.github.io/mdBook/) 编写，双语源文件位于 robrix2 仓库的 `book/` 目录：

```bash
mdbook serve book/zh -p 8300   # 中文版 http://localhost:8300
mdbook serve book/en -p 8301   # 英文版 http://localhost:8301
mdbook build book/zh
mdbook build book/en
```

流程图由 [mdbook-mermaid](https://github.com/badboy/mdbook-mermaid) 渲染（`cargo install mdbook-mermaid`）。
