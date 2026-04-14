# TG Bot Phase 5 Orchestration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the next orchestration layer on top of the shipped TG-style bot UX: bot-aware command discovery, `/allbots` broadcast to all bound child bots, and natural-language scheduling commands backed by Octos cron.

**Architecture:** Keep Robrix focused on command discovery, input UX, and chat rendering. Keep Octos/BotFather as the source of truth for management command semantics, broadcast fan-out, cron parsing, execution, loop protection, and auditing. Deliver the work in three independent increments: Phase 5a command discovery, Phase 5b `/allbots`, then Phase 5c natural-language scheduling.

**Tech Stack:** Rust, Makepad 2.0 (`script_mod!`, `Widget`, `script_apply_eval!`), Matrix SDK/ruma event content in `robrix2`, Octos gateway/session runtime, Octos Matrix bus, existing cron service/tooling.

---

## File Structure

### Robrix2 command discovery/UI

- Modify: `src/shared/mentionable_text_input.rs`
  - Expand the local slash-command catalog model from a single global table to context-aware catalogs.
  - Keep `/command@bot` parsing independent from popup catalog filtering.
  - Add orchestration and schedule commands to the correct BotFather contexts only.
- Modify: `src/room/room_input_bar.rs`
  - Feed the correct command-discovery context into the popup/menu button.
  - Keep BotFather-only menu visibility aligned with the current management-room gating.
- Modify: `resources/i18n/en.json`
- Modify: `resources/i18n/zh-CN.json`
  - Add command descriptions for `/allbots`, `/schedule`, `/schedules`, `/unschedule`.

### Octos BotFather orchestration

- Modify: `../../octos/crates/octos-bus/src/matrix_channel.rs`
  - Extend BotFather slash-command interception to include `/allbots`, `/schedule`, `/schedules`, `/unschedule`.
  - Preserve existing explicit target and approval metadata behavior.
- Modify: `../../octos/crates/octos-cli/src/session_actor.rs`
  - Add helpers for internal BotFather fan-out and natural-language schedule command handling.
  - Reuse existing approval, outbound message, and cron context plumbing.
- Modify: `../../octos/crates/octos-cli/src/cron_tool.rs`
  - Reuse existing add/list/remove primitives; avoid parallel scheduler logic.
- Modify: `../../octos/crates/octos-cli/src/commands/gateway/gateway_runtime.rs`
  - Only if needed for shared config/runtime plumbing already used by cron and approval.
- Modify: `../../octos/book/src/advanced.md`
- Modify: `../../octos/book/src/channels.md`
- Modify: `../../octos/book/src/cli-reference.md`
  - Document BotFather orchestration commands, not just raw cron CLI.

### Specs and validation

- Modify: `specs/task-tg-bot-bot-aware-command-discovery.spec.md`
- Modify: `specs/task-tg-bot-allbots-broadcast.spec.md`
- Modify: `specs/task-tg-bot-natural-language-schedule.spec.md`
  - Only if implementation reveals a real contract gap.

---

## Execution Order

1. Phase 5a: Bot-aware command discovery in `robrix2`
2. Phase 5b: `/allbots` broadcast in `octos`, then expose it in discovery
3. Phase 5c: Natural-language schedule commands in `octos`, then expose them in discovery
4. End-to-end Matrix validation across `robrix2` and `octos`

This order is intentional:
- `/allbots` and `/schedule` need a clean command discovery model.
- `/schedule` is simpler once BotFather command surfaces are already split from child-bot command surfaces.
- Robrix should never advertise commands that Octos cannot yet execute.

---

### Task 1: Refactor slash-command discovery into bot-aware catalogs

**Files:**
- Modify: `src/shared/mentionable_text_input.rs`
- Modify: `src/room/room_input_bar.rs`
- Modify: `resources/i18n/en.json`
- Modify: `resources/i18n/zh-CN.json`
- Test: `src/shared/mentionable_text_input.rs`
- Test: `src/room/room_input_bar.rs`

- [ ] **Step 1: Write failing tests for context-aware command catalogs**

Add unit tests that cover:
- `ManagementDm` shows `/createbot`, `/deletebot`, `/listbots`, `/bothelp`, `/schedule`, `/schedules`, `/unschedule`, but not `/allbots`
- `ManagementRoom` shows the same plus `/allbots`
- `ChildBotRoom` shows only child session commands (`/new`, `/s`, `/sessions`, `/back`, `/delete`, `/soul`, `/status`, `/adaptive`, `/reset`, `/help`)
- no bot context returns an empty command list

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run:

```bash
cargo test slash_command --quiet
cargo test management_bot_room --quiet
```

Expected: failures or missing cases for bot-aware catalogs and new commands.

- [ ] **Step 3: Replace the single global slash-command table with context-aware catalogs**

Implement a small catalog layer in `mentionable_text_input.rs`, for example:

```rust
enum SlashCommandContext {
    ManagementDm,
    ManagementRoom,
    ChildBotRoom,
    None,
}
```

Keep `/command@bot` parsing independent from popup contents. The popup should filter *within* the active catalog, not change the explicit send-time parser.

- [ ] **Step 4: Wire `RoomInputBar` to compute the command-discovery context**

Use the existing management-room gating for BotFather contexts and add an explicit child-bot room context for bound child bots. Do not infer from raw bot membership alone; only use the same trusted room/binding signals already used by Phase 4a.

- [ ] **Step 5: Add i18n descriptions for new commands**

Add localized labels/descriptions for:
- `/allbots`
- `/schedule`
- `/schedules`
- `/unschedule`

- [ ] **Step 6: Re-run focused discovery tests**

Run:

```bash
cargo test slash_command --quiet
cargo test management_bot_room --quiet
```

Expected: PASS for the new catalog behavior and no regressions in existing popup/menu tests.

- [ ] **Step 7: Manual UI smoke test**

Verify in `cargo run`:
- BotFather DM shows schedule commands but not `/allbots`
- BotFather-bound room shows `/allbots`
- child-bot room does not show BotFather management commands

---

### Task 2: Add `/allbots` as a BotFather broadcast command in Octos

**Files:**
- Modify: `../../octos/crates/octos-bus/src/matrix_channel.rs`
- Modify: `../../octos/crates/octos-cli/src/session_actor.rs`
- Modify: `../../octos/book/src/advanced.md`
- Modify: `../../octos/book/src/channels.md`
- Test: `../../octos/crates/octos-bus/src/matrix_channel.rs`
- Test: `../../octos/crates/octos-cli/src/session_actor.rs`

- [ ] **Step 1: Write failing tests for `/allbots` interception and fan-out**

Add tests that cover:
- `/allbots <message>` is intercepted only in management rooms
- empty `/allbots` body is rejected with a friendly usage reply
- no child bindings yields a friendly refusal
- BotFather fans out to all bound child bots except itself
- bot-originated `/allbots` is rejected
- max target count is enforced

- [ ] **Step 2: Run the focused Octos tests and confirm failure**

Run:

```bash
cd ../../octos
cargo test -p octos-bus --features matrix listbots --quiet
cargo test -p octos-bus --features matrix allbots --quiet
```

Expected: new `/allbots` coverage fails because the command does not exist yet.

- [ ] **Step 3: Extend BotFather slash-command interception**

In `matrix_channel.rs`, add `/allbots` to the BotFather command dispatch layer next to `/createbot`, `/deletebot`, `/listbots`, `/bothelp`.

Keep these semantics:
- BotFather-only
- human-only
- no Matrix bot-to-bot message dependency

- [ ] **Step 4: Implement internal fan-out in `session_actor.rs`**

Add a helper that:
- snapshots the room's bound child bots
- excludes the management bot itself
- emits one internal dispatch per child bot
- preserves original human requester identity and room context
- records a request id for audit/log correlation

Do not implement aggregation. Each child bot should reply normally through existing message flow.

- [ ] **Step 5: Add loop/storm guards**

Implement v1 protections:
- reject bot-originated `/allbots`
- cap fan-out target count (default 8)
- mark the broadcast as one fan-out layer only

- [ ] **Step 6: Re-run Octos tests**

Run:

```bash
cd ../../octos
cargo test -p octos-bus --features matrix --quiet
cargo test -p octos-cli session_actor --quiet
cargo build
```

Expected: PASS, including the new `/allbots` coverage.

- [ ] **Step 7: Update docs**

Document:
- `/allbots <message>`
- management-room-only behavior
- no aggregation in v1
- human-only and target-cap limits

---

### Task 3: Expose `/allbots` in Robrix command discovery

**Files:**
- Modify: `src/shared/mentionable_text_input.rs`
- Modify: `resources/i18n/en.json`
- Modify: `resources/i18n/zh-CN.json`
- Test: `src/shared/mentionable_text_input.rs`

- [ ] **Step 1: Add or update tests for `/allbots` room-only discovery**

Add/adjust tests to verify:
- `/allbots` appears in `ManagementRoom`
- `/allbots` is absent in `ManagementDm`
- `/allbots` is absent in `ChildBotRoom`

- [ ] **Step 2: Adjust the management-room catalog**

Keep the command visible only in `ManagementRoom`.

- [ ] **Step 3: Re-run focused discovery tests**

Run:

```bash
cargo test slash_command --quiet
```

Expected: PASS.

- [ ] **Step 4: Manual smoke test**

In `cargo run`, confirm:
- BotFather room menu shows `/allbots`
- BotFather DM menu does not
- selecting `/allbots` inserts parameterized command text, not immediate send

---

### Task 4: Add natural-language schedule commands in Octos

**Files:**
- Modify: `../../octos/crates/octos-bus/src/matrix_channel.rs`
- Modify: `../../octos/crates/octos-cli/src/session_actor.rs`
- Modify: `../../octos/crates/octos-cli/src/cron_tool.rs`
- Modify: `../../octos/book/src/advanced.md`
- Modify: `../../octos/book/src/cli-reference.md`
- Modify: `../../octos/book/src/channels.md`
- Test: `../../octos/crates/octos-bus/src/matrix_channel.rs`
- Test: `../../octos/crates/octos-cli/src/session_actor.rs`

- [ ] **Step 1: Write failing tests for `/schedule`, `/schedules`, `/unschedule`**

Cover:
- `/schedule <task>` routes to BotFather and preserves user-visible natural-language body
- `/schedule` without body is rejected
- `/schedules` lists only jobs for current room/chat context
- `/unschedule <job-id>` removes only jobs in current room/chat context
- ambiguous time produces clarification and no cron job

- [ ] **Step 2: Run the focused scheduling tests and confirm failure**

Run:

```bash
cd ../../octos
cargo test -p octos-bus --features matrix schedule --quiet
cargo test -p octos-cli cron --quiet
```

Expected: failures because BotFather natural-language commands do not exist yet.

- [ ] **Step 3: Extend BotFather command dispatch for schedule commands**

Add handling for:
- `/schedule`
- `/schedules`
- `/unschedule`

Do not expose raw `cron` syntax to users.

- [ ] **Step 4: Reuse existing cron service/tooling instead of adding a second scheduler**

Implement command handlers in `session_actor.rs` that:
- parse NL task/time intent
- translate to cron-tool inputs or direct cron-service calls
- bind created jobs to current Matrix room/chat context
- list only current-context jobs
- remove only current-context jobs

For v1, keep parsing conservative:
- if time is ambiguous, ask for clarification
- do not create a cron job on uncertain parses

- [ ] **Step 5: Ensure schedule commands remain BotFather-only**

Do not allow child bots to claim these commands. Keep them as management/orchestration capabilities.

- [ ] **Step 6: Re-run scheduling tests**

Run:

```bash
cd ../../octos
cargo test -p octos-bus --features matrix --quiet
cargo test -p octos-cli cron --quiet
cargo test -p octos-cli session_actor --quiet
cargo build
```

Expected: PASS for command interception and cron-context behavior.

- [ ] **Step 7: Update Octos docs**

Document the user-facing BotFather commands and clearly state that they are backed by the existing cron runtime.

---

### Task 5: Expose schedule commands in Robrix discovery

**Files:**
- Modify: `src/shared/mentionable_text_input.rs`
- Modify: `resources/i18n/en.json`
- Modify: `resources/i18n/zh-CN.json`
- Test: `src/shared/mentionable_text_input.rs`

- [ ] **Step 1: Add or update tests for schedule command visibility**

Cover:
- `ManagementDm` shows `/schedule`, `/schedules`, `/unschedule`
- `ManagementRoom` shows the same
- `ChildBotRoom` does not show them
- `/schedules` remains pure in discovery
- `/schedule` and `/unschedule` remain parameterized

- [ ] **Step 2: Update the BotFather catalogs**

Add the schedule commands to the BotFather discovery catalogs only.

- [ ] **Step 3: Re-run focused discovery tests**

Run:

```bash
cargo test slash_command --quiet
```

Expected: PASS.

- [ ] **Step 4: Manual UI smoke test**

Verify:
- BotFather DM menu shows scheduling commands
- BotFather room menu shows scheduling commands plus `/allbots`
- child bot room does not show scheduling commands

---

### Task 6: End-to-end Matrix validation across both repos

**Files:**
- Modify only if needed based on real integration findings

- [ ] **Step 1: Start local services**

Run:

```bash
cd ../../octos
cargo run --bin octos --features "api matrix" -- serve --host 0.0.0.0 --port 8010 --auth-token 89285046d16c938aff19ecc1a94b11b2 --data-dir /Users/zhangalex/Work/Projects/FW/robius/robrix2/palpo-and-octos-deploy/data/octos
```

And in `robrix2`:

```bash
cargo run
```

- [ ] **Step 2: Validate bot-aware discovery**

Manual checks:
- BotFather DM shows management + schedule commands, not `/allbots`
- BotFather room shows `/allbots`
- child bot room shows child commands only

- [ ] **Step 3: Validate `/allbots`**

Manual checks:
- In a room bound to multiple child bots, send `/allbots 总结今天的状态`
- Confirm each bound child bot replies independently
- Confirm BotFather itself is not one of the execution targets
- Confirm plain rooms without management binding do not expose `/allbots`

- [ ] **Step 4: Validate natural-language scheduling**

Manual checks:
- `/schedule 每天早上 9 点提醒我看天气`
- `/schedules`
- `/unschedule <job-id>`
- ambiguous input such as `/schedule 明天提醒我` should ask for clarification instead of creating a job

- [ ] **Step 5: Run final repo tests**

Run in `robrix2`:

```bash
cargo test slash_command --quiet
cargo test management_bot_room --quiet
cargo build
```

Run in `octos`:

```bash
cd ../../octos
cargo test -p octos-bus --features matrix --quiet
cargo test -p octos-cli session_actor --quiet
cargo test -p octos-cli cron --quiet
cargo build
```

- [ ] **Step 6: Prepare user validation checklist before any commit**

Checklist:
- BotFather DM/room command menus are context-correct
- `/allbots` fans out only to explicitly bound child bots
- natural-language scheduling creates/list/removes jobs in the current room context
- no regression in existing `/command@bot`, approval buttons, or mention/reply-first behavior

---

## Notes

- Do not overload `@room`; it remains Matrix-native human notification semantics.
- Do not implement `/allbots@subset`, tag-based filtering, aggregation, or scheduled broadcast in this phase.
- Do not expose raw `cron` expressions in Robrix UI for v1.
- Keep all new orchestration commands BotFather-owned; child bots should not claim them unless a later spec explicitly adds that capability.
