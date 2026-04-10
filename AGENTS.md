# Robrix2 — Agent Instructions

Keep this file short. Use it for project rules and working guidance. Use the codebase, `CLAUDE.md`, and Makepad 2.0 skills as the detailed reference.

## Required Reading

Before starting work, read these documents:

1. [DESIGN.md](DESIGN.md) — architecture overview, module organization, technology stack
2. [specs/project.spec.md](specs/project.spec.md) — project constraints, decisions, forbidden actions
3. [CLAUDE.md](CLAUDE.md) — project workflow rules and Makepad 2.0 guidance
4. [MAKEPAD.md](MAKEPAD.md) — Makepad 2.0 skill routing and design judgment entry point

## Critical Rules

### Do NOT run `cargo fmt` or `rustfmt`

This project does not use automatic Rust formatting. Do not run `cargo fmt`, `rustfmt`, or formatter wrappers. Formatting churn creates noisy diffs and breaks the repo's hand-maintained style.

### Do NOT commit or create PRs without user testing

Present changes for testing first. Wait for user confirmation before committing or opening a PR.

### Makepad 2.0 only

- Use `script_mod!`, not `live_design!`
- Use `#[derive(Script, ScriptHook, Widget)]`, not `Live` / `LiveHook`
- Use `:=` for named children, not `=`
- Use `+:` to merge properties; bare `:` replaces
- Use `script_apply_eval!` for runtime updates, not `apply_over` + `live!`

### Converting syntax

- Search the new crates first: `widgets`, `code_editor`, `studio`
- Prefer copying an existing Makepad 2.0 pattern over guessing syntax
- Always use `Name: value`, never `Name = value`
- Named widget instances use `name := Type{...}`

### Dynamic widget state changes

`script_apply_eval!` does not work on widgets created via `widget_ref_from_live_ptr()` because the backing `ScriptObject` is `ZERO`. For dynamic popup and list items, use Animator state plus shader instance variables instead.

### Async Matrix operations

Always use `submit_async_request(MatrixRequest::*)`. Do not spawn raw tokio tasks for Matrix API calls from UI code.

## Quick Makepad Notes

- `draw_bg +:` merges with the parent shader config; `draw_bg:` replaces it
- In `script_apply_eval!`, Rust expressions use `#(expr)` interpolation
- Runtime `script_apply_eval!` cannot rely on DSL constants like `Right`, `Fit`, or `Align`
- `Dock.load_state()` can corrupt DrawList references in this project

## Build & Test

```bash
cargo build
cargo run
cargo test
```

## Key Entry Points

- `src/app.rs` — root app and global state
- `src/sliding_sync.rs` — Matrix sync pipeline
- `src/home/room_screen.rs` — room timeline and input integration
- `src/shared/mentionable_text_input.rs` — `@mention` system

## Specs

Task specs live in `specs/` and inherit from [specs/project.spec.md](specs/project.spec.md).

- `specs/task-mention-user.spec.md` — `@mention` autocomplete feature

Use `agent-spec parse` and `agent-spec lint --min-score 0.7` when working on specs.

## Working Philosophy

You are an engineering collaborator on this project, not a standby assistant. Work in a direct, execution-first style:

- Finish concrete work before reporting back
- Report what you changed, why you changed it, and what tradeoffs you made
- Prefer complete, reviewable units over tentative partial steps
- Keep mid-work chatter low; use delivery reports for important context

## What You Submit To

In priority order:

1. The task's completion criteria
2. The project's existing style and patterns
3. The user's explicit, unambiguous instructions

Correctness outranks performative deference. Do the engineering work instead of offloading routine implementation choices back to the user.

## On Stopping to Ask

Stop and ask only when genuine ambiguity would likely produce output contrary to the user's intent.

Do not stop just to ask about:

- Reversible implementation details
- Obvious next steps that are already part of the task
- Style choices you can resolve by reading the codebase
- Post-hoc "should I also do X" follow-ups when X is already implied by the task
