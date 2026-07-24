# Appendix: Projects and Resources

> **Scope**: An index of the project repositories, cloud services, and related technologies covered in this book, plus how the book itself is built.

## Core Projects

| Project | URL | Description |
|------|------|------|
| Robrix2 | <https://github.com/Project-Robius-China/robrix2> | Matrix client in Rust + Makepad |
| agent-chat | <https://github.com/ZhangHanDong/agent-chat> | Local-first agent coordination system and Matrix bridge |
| Palpo | <https://github.com/palpo-im/palpo> | Matrix homeserver written in Rust |

## Cloud Services

- **Meldry** (hosted Palpo tenants — create your own Matrix server): <https://tenant.meldry.com/>
- **The official Matrix server**: <https://matrix.org>

## Related Technologies

- Matrix protocol specification: <https://spec.matrix.org/>
- matrix-rust-sdk: <https://github.com/matrix-org/matrix-rust-sdk>
- Makepad (Rust UI framework): <https://github.com/makepad/makepad>
- Robius (Rust cross-platform app ecosystem): <https://github.com/project-robius>
- Claude Code: <https://claude.com/claude-code>
- Codex CLI: <https://developers.openai.com/codex>
- AtomGit OpenAPI: <https://docs.openatom.tech/en/category/api/>

## Building This Book

This book is written with [mdBook](https://rust-lang.github.io/mdBook/); the bilingual sources live in the `book/` directory of the robrix2 repository:

```bash
mdbook serve book/zh -p 8300   # Chinese edition at http://localhost:8300
mdbook serve book/en -p 8301   # English edition at http://localhost:8301
mdbook build book/zh
mdbook build book/en
```

Flow diagrams are rendered by [mdbook-mermaid](https://github.com/badboy/mdbook-mermaid) (`cargo install mdbook-mermaid`).
