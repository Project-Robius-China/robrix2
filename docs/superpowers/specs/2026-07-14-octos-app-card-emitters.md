# Octos-side `org.octos.app` card emitters

> Status: **design note** (robrix renderers exist; octos emitters NOT built yet)
> Date: 2026-07-14 · Octos ref: HEAD `9ab0b17d`
> Companion: `2026-07-10-agent-message-response-cards.md` (robrix render side)

## Why this doc

Robrix renders a growing set of `org.octos.app` mini-app cards (weather,
mission_room, **tool_call**, **diff_view**, **task_tree**, and next **pipeline**,
**run_summary**). Today octos only ever *emits* two of them (weather,
mission_room), and only because the **LLM explicitly calls the `send_app_card`
tool**. The rest are currently **test-injected** by
`~/Projects/octos/.octos/robrix-test-card.sh` — no real conversation produces
them.

To make the cards appear in real chats, octos needs a second, **automatic**
producer that projects runtime events into cards without the model asking. This
note captures that design so future card work (pipeline, cost, ask-user, etc.)
can share one bridge instead of each reinventing it.

## Two producers of `org.octos.app` (keep them distinct)

| Producer | Trigger | Lives in | Emits today |
|---|---|---|---|
| **Agent tool** `send_app_card` | the LLM decides to call it | `crates/octos-agent/src/tools/send_app_card.rs` | `weather`, `mission_room` |
| **Runtime projection bridge** *(to build)* | a UI-protocol event fires | `crates/octos-bus/` (new module) | `tool_call`, `diff_view`, `task_tree`, `pipeline`, `run_summary` |

Both converge on the **same wire shape**: an `OutboundMessage` whose `metadata`
carries an `"org.octos.app"` object `{type, version, initial_state}`. The Matrix
channel (`crates/octos-bus/src/matrix_channel.rs`, const `CONTENT_APP =
"org.octos.app"`) copies that key onto the `m.room.message` content. So the
bridge does **not** touch Matrix directly — it just produces `OutboundMessage`s
with the right metadata, exactly like the tool does.

## Architecture: a projection bridge

Octos already broadcasts a stream of `UiNotification`s
(`crates/octos-core/src/ui_protocol.rs`, enum around line 6040) that feeds the
WebSocket UI protocol (octos-web / CLI). The bridge subscribes to that **same
stream** and, for an allowlisted subset of event kinds, synthesizes a card:

```
UiNotification stream ──► ProjectionBridge ──► OutboundMessage{ metadata: org.octos.app } ──► matrix_channel ──► palpo
   (tool/*, diff/*,          (this doc)            (existing OutboundMessage path)
    task/updated,
    token_cost_update, …)
```

Bridge responsibilities:
1. **Session → room routing.** Each `UiNotification` carries a `session_id`
   (`SessionKey`). The bridge maps session → Matrix room (the same mapping the
   inbound router already maintains in `matrix_channel`), and drops events for
   sessions with no bound room.
2. **Allowlist + config gate.** Only project a configured set of event kinds.
   Default OFF per-kind so we can roll cards out one at a time. Config lives
   next to the existing matrix channel settings in `.octos/config.json`
   (e.g. `gateway.channels[matrix].settings.app_card_projection = ["tool_call",
   "task_tree", ...]`).
3. **Update vs new message.** A tool card goes `running → completed`. Prefer
   **editing** the original event (Matrix `m.replace` / the
   `org.matrix.msc4357.live` streaming-edit key octos already uses) so the card
   mutates in place instead of spamming three messages. Fallback: post a fresh
   card and let robrix's timeline show the latest. **Decision needed** (see open
   questions) — robrix's renderer is idempotent either way.
4. **Throttling.** `tool/progress` and `token_cost_update` are chatty. Coalesce:
   at most one card edit per (session, card) per ~500ms; always flush the
   terminal event (`tool/completed`, `turn/completed`).

## Per-type projections

Source event → `initial_state`. Robrix parses these leniently (missing fields
degrade gracefully), so the emitter can start minimal and grow.

### `tool_call` ← `ToolStartedEvent` / `ToolProgressEvent` / `ToolCompletedEvent`
`ui_protocol.rs` @5003 / @5021 / @5035 · methods `tool/started|progress|completed`.
One card per `tool_call_id`, edited across the lifecycle.
```
initial_state = {
  tool_name,                 // ToolStartedEvent.tool_name
  status,                    // started→"running", completed(ok)→"completed", completed(err)→"error"
  arguments,                 // ToolStartedEvent args (string or object)
  output_preview,            // ToolCompletedEvent result summary (truncate emitter-side ~2KB)
  error,                     // ToolCompletedEvent error, when failed
  duration_ms,               // completed_at - started_at
}
```

### `diff_view` ← `DiffPreview`
`ui_protocol.rs` @2493 (result of `diff/preview/get`, or a diff-preview
notification if one is added). Shape is **already** what robrix expects — near
1:1 copy:
```
initial_state = {
  title,                     // DiffPreview.title
  files: [ { path, old_path, status,          // DiffPreviewFile
             hunks: [ { header,
                        lines: [ { kind, content } ] } ] } ]   // kind ∈ context|added|removed
}
```
Emitter should cap payload size (drop/hunk-limit huge diffs) — robrix also
budgets, but keep the wire small.

### `task_tree` ← `TaskListEntry` (`task/list`) + `TaskUpdatedEvent` (`task/updated`)
`ui_protocol.rs` @2390 / @5414. Octos's model is a **flat** list with parent/child
session keys; the emitter builds the tree:
```
initial_state = {
  title,                     // session goal title, if any
  tasks: [ {
     id,                     // TaskListEntry.id
     title | role,           // role ("reviewer"/"implementer"/…) is the friendly label
     state,                  // TaskRuntimeState: pending|running|completed|failed|cancelled
     summary,                // TaskListEntry.summary (bounded capsule)
     error,                  // TaskListEntry.error
     children: [ … ]         // built from parent_session_key / child_session_key
  } ]
}
```
Rebuild-and-replace one board card per session on each `task/updated` (throttled).

### `pipeline` ← workflow tasks (`workflow_kind` + `current_phase`)  *(B4)*
Octos has **no arbitrary DAG-with-edges** on the wire. Workflows are tasks
tagged `workflow_kind` (e.g. `"coding"`) with a `current_phase`
(`TaskListEntry.workflow_kind` @2444, `current_phase` @2446;
`TaskRestartFromNodeParams` @2329 confirms node identity). So project a
**staged (linearized) pipeline**, not a free graph:
```
initial_state = {
  title,                     // workflow label
  workflow_kind,             // "coding" | "review" | …
  current_phase,             // highlighted stage
  stages: [ { name, status } ]   // status ∈ pending|running|completed|failed|skipped
}
```
Emitter derives `stages` from the workflow template's known phase order, marking
`current_phase` running and prior phases completed. (A true DAG with `edges`
would need a new octos event — out of scope; revisit if a graph workflow lands.)

### `run_summary` ← `token_cost_update` + `turn/completed` + reasoning  *(B5)*
`UiTokenCostUpdate` @4701 (method `token_cost_update`), `EnvelopeTokenUsage`
@3554 on `TurnCompleted{token_usage}` @3756, `reasoning_content` @2669 /
`message/reasoning_delta` @1039. A compact per-turn footer card:
```
initial_state = {
  model,                     // UiTokenCostUpdate.model
  input_tokens, output_tokens, reasoning_tokens, total_tokens,
  response_cost,             // this turn ($)
  session_cost,              // cumulative ($)
  duration_ms,               // turn wall-clock
  reasoning,                 // optional short reasoning/thinking summary
}
```
Emit once on `turn/completed` (coalesce the mid-turn `token_cost_update`s into
the final figure). Reasoning text is optional and may be gated for privacy.

> ⚠️ **Matrix canonical JSON forbids floating-point numbers.** Palpo rejects an
> `m.room.message` whose content contains a float with
> `M_BAD_JSON: float cannot be serialized as canonical JSON`. So `response_cost`
> / `session_cost` **must not** be sent as raw floats — send them as a **decimal
> string** (`"0.0021"`) or a scaled integer (e.g. micro-USD). Token counts and
> `duration_ms` are integers and are fine. Robrix's `run_summary` renderer
> accepts a number *or* a string for the cost fields. This applies to **every**
> card type: keep all `initial_state` numbers integer-valued on the wire.

## Open questions / decisions

- **Edit-in-place vs append.** Does palpo + robrix handle `m.replace` on an
  appservice-sent `org.octos.app` message cleanly (robrix re-renders the card on
  edit)? If yes, prefer edits for `tool_call`/`task_tree`/`pipeline`. If not,
  append and dedup by a stable `card_id` in `initial_state`.
- **Where the bridge subscribes.** Reuse the WS server's broadcast receiver, or
  tap the event bus one level upstream? Prefer the same broadcast so cards match
  exactly what octos-web shows.
- **Privacy.** `arguments`, diff `content`, and `reasoning` can carry secrets.
  Gate per-field via config (mirror the WS protocol's existing PII omissions,
  ui_protocol.rs ~5253).
- **Ordering.** A card edit must not arrive before its create. Serialize
  per-(session, card_id).

## Robrix side (already done)

Renderers in `src/home/room_screen.rs` dispatch on `org.octos.app.type`
(`render_octos_app_card`): `weather`, `mission_room`, `tool_call`, `diff_view`,
`task_tree` built; `pipeline`, `run_summary` in progress. All render into the
shared `octos_app_card` `HtmlOrPlaintext` slot; colors come from `RBX_*` tokens
via `utils::vec4_to_hex` → `<font color>`. Unknown `type` falls back to body
text, so **the emitter can ship a type before robrix supports it** without
breaking anything.
