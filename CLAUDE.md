# Robrix2 — Claude Code Instructions

## Required Reading

Before starting any task, read these documents:

1. **[DESIGN.md](DESIGN.md)** — Architecture overview, module organization, technology stack
2. **[specs/project.spec.md](specs/project.spec.md)** — Project-level constraints, decisions, and forbidden actions
3. **[AGENTS.md](AGENTS.md)** — Makepad 2.0 patterns and DSL syntax reference
4. **[MAKEPAD.md](MAKEPAD.md)** — Makepad 2.0 skill routing and design judgment entry point; **For ALL Makepad questions, FIRST load `makepad-2.0-design-judgment`.**

## Critical Rules

### Do NOT run `cargo fmt`
This project does not use rustfmt. Formatting changes create noisy diffs and break existing code style.

### Do NOT commit or create PRs without user testing
Always let the user test changes before committing. Present what's ready for testing, wait for confirmation.

### Makepad 2.0 Syntax (NOT 1.x)
- Use `script_mod!` (NOT `live_design!`)
- Use `#[derive(Script, ScriptHook, Widget)]` (NOT `Live, LiveHook`)
- Use `:=` for named children (NOT `=`)
- Use `+:` to merge properties (NOT `:` which replaces)
- Use `script_apply_eval!` for runtime updates (NOT `apply_over` + `live!`)

### Dynamic Widget State Changes
`script_apply_eval!` does NOT work on widgets created via `widget_ref_from_live_ptr()` (ScriptObject is ZERO). Use Animator + shader instance variables instead:

```rust
// In DSL template:
draw_bg +: { selected: instance(0.0) }
animator: Animator { highlight: { ... apply: { draw_bg: { selected: 1.0 } } } }

// In Rust:
view.animator_cut(cx, ids!(highlight.on));
```

### Async Matrix Operations
Always use `submit_async_request(MatrixRequest::*)`. Do NOT spawn raw tokio tasks for Matrix API calls.

## Makepad 2.0 Skills

When working on Makepad UI code, **always invoke the relevant Makepad 2.0 skill** before writing or debugging:

| Situation | Skill to Use |
|-----------|-------------|
| UI not rendering, widget invisible, click not working | `makepad-2.0-troubleshooting` (Pitfalls #1-#44) |
| DSL syntax questions, `script_mod!`, property system | `makepad-2.0-dsl` |
| Layout issues (width/height/flow/align) | `makepad-2.0-layout` |
| Hover effects, state transitions, animation | `makepad-2.0-animation` |
| Shader code, `draw_bg`, `Sdf2d`, pixel functions | `makepad-2.0-shaders` |
| Event handling, `handle_event`, actions, `script_apply_eval!` | `makepad-2.0-events` |
| Widget catalog (View, Button, Label, etc.) | `makepad-2.0-widgets` |
| Migrating from Makepad 1.x to 2.0 | `makepad-2.0-migration` |
| App structure, `app_main!`, `MatchEvent` | `makepad-2.0-app-structure` |
| Theme system, colors, fonts | `makepad-2.0-theme` |

**Key pitfalls from this project** (in `makepad-2.0-troubleshooting`):
- **#40**: `script_apply_eval!` fails on dynamic widgets — use Animator instead
- **#41**: DSL constants (`Right`, `Fit`, `Align`) unavailable at runtime in `script_apply_eval!`
- **#42**: `Dock.load_state()` corrupts DrawList references
- **#43**: Named children: `=` vs `:=` in `script_mod!`
- **#44**: `draw_bg:` replaces vs `draw_bg +:` merges

## Build & Test

```bash
# Build
cargo build

# Run
cargo run

# Run with hot reload
cargo run -- --hot

# Tests (limited — mostly manual testing)
cargo test
```

## Project Structure

See [DESIGN.md](DESIGN.md) for full module organization.

Key entry points:
- `src/app.rs` — Root app, global state
- `src/sliding_sync.rs` — Matrix client, sync
- `src/home/room_screen.rs` — Timeline rendering
- `src/shared/mentionable_text_input.rs` — @mention system

## Specs

Task specs live in `specs/` and inherit from `specs/project.spec.md`:
- `specs/task-mention-user.spec.md` — @mention autocomplete feature

Use `agent-spec parse` and `agent-spec lint --min-score 0.7` to validate specs.
