spec: project
name: "Robrix2 — Matrix Chat Client on Makepad 2.0"
tags: [makepad, matrix, rust, gui]
---

## Intent

Robrix is a multi-platform Matrix chat client built with Makepad 2.0 and matrix-sdk. This project spec defines the shared constraints, coding standards, and technical decisions that all task specs inherit.

## Constraints

- All code must compile with `cargo build` on the `feature/mention-user-migration` branch (or `main` after merge)
- All UI widgets must use Makepad 2.0 `script_mod!` DSL syntax — do NOT use Makepad 1.x `live_design!` syntax
- Named widget children must use `:=` operator, NOT `=`
- Property overrides on inherited widgets must use `+:` merge operator to preserve parent properties
- Do NOT use `cargo fmt` — the project does not enforce rustfmt and formatting changes create noisy diffs
- Do NOT add new cargo dependencies without explicit approval in the task spec
- Do NOT use `.unwrap()` on user-facing code paths — use proper error handling with `anyhow` or pattern matching
- Async Matrix operations must go through `submit_async_request(MatrixRequest::*)` — do NOT spawn raw tokio tasks for Matrix API calls
- Widget state changes on dynamically-created widgets (via `widget_ref_from_live_ptr()`) must use Animator + shader instance variables, NOT `script_apply_eval!` (which silently fails due to `ScriptObject::ZERO`)
- `script_apply_eval!` must NOT use DSL constants (`Right`, `Down`, `Fit`, `Fill`, `Align`, `Inset`, `MouseCursor`) — these are not available at runtime scope
- All `draw_bg` property modifications must use `+:` merge syntax, NOT `:` replace syntax, to avoid losing shader/border/animation properties

## Decisions

- UI Framework: Makepad 2.0 with `script_mod!` DSL (fork: `kevinaboos/makepad`, branch: `stack_nav_improvements`)
- Matrix SDK: `matrix-sdk` with sliding sync, E2E encryption, SQLite storage
- Async runtime: Tokio
- State persistence: JSON serialization via serde to `~/.local/share/org.robius.robrix/`
- Widget template instantiation: `crate::widget_ref_from_live_ptr(cx, Some(ptr))` for creating widgets from `#[live] Option<LivePtr>` fields
- Derive macros: `#[derive(Script, ScriptHook, Widget)]` for widget structs (NOT `Live`/`LiveHook`)
- DSL property syntax: whitespace-separated (no commas), `Inset{...}` for margins/padding, `Align{...}` for alignment
- Hex colors with letter 'e': use `#x` prefix (e.g., `#x1E90FF`)
- Background CPU work: `cpu_worker::spawn_cpu_job(cx, CpuJob::*)` via `cx.spawn_thread()`
- Dock state restoration: programmatic tab recreation via `close_all_tabs()` + `focus_or_create_tab()`, NOT `Dock.load_state()` (which corrupts DrawList references)

## Boundaries

### Forbidden
- Do NOT run `cargo fmt` on any files
- Do NOT modify `Cargo.toml` dependencies without task-level approval
- Do NOT use `live_design!` macro (Makepad 1.x syntax)
- Do NOT use `apply_over(cx, live!{...})` — use `script_apply_eval!(cx, widget, {...})` for runtime updates
- Do NOT call `Dock.load_state()` during event handling (causes DrawList corruption)
- Do NOT commit code that doesn't pass `cargo build`
- Do NOT create PRs without user testing and approval first
