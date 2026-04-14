# Roadmap: Agent-to-App — Mini-Apps in Robrix

> **Date:** 2026-04-13 (revised 2026-04-14 after Codex review + Makepad source audit)
> **Status:** Analysis hardened against primary sources. Ready to back the
> master spec `specs/task-agent-to-app-system.spec.md`.
> **Authors:** Claude + Codex (cross-reviewed)
> **Related:** `docs/superpowers/plans/2026-04-12-tg-bot-timeline-cards-plan.md`,
> `specs/task-tg-bot-action-buttons.spec.md` (Phase 4c),
> `specs/task-tg-bot-approval-request.spec.md` (Phase 5 approval flow)

## Motivation

Today bots in Robrix can send plain text, files, and (via the Phase 3
Splash-card prototype) a single `org.octos.splash_card` string that is evaluated
as raw Splash DSL. This gives us a pretty weather card but nothing that
*behaves* like an app — no interactivity, no refresh, no local state.

The next step is **agent-to-app**: let a bot deliver a small embedded
application — a weather card, a news reader, a pomodoro timer — directly into
the chat timeline, rendered natively via Splash, controllable by the user
without round-tripping every tick through Matrix.

"Agent-to-app" here means *bot delivers a mini-app that renders inside Robrix*,
not *bot calls out to a native desktop app*.

## Taxonomy: three complexity layers (L2 split into L2a / L2b)

| App | State | Tick | User interaction | Layer |
|---|---|---|---|---|
| Weather card (no buttons) | stateless snapshot | — | — | **L1 Static** |
| Weather card with refresh / news reader next/open | weak state (cursor) owned by agent | none | buttons **outside** the card | **L2a External action row** |
| Zoomable image / in-card "mark read" button | none-to-weak | none | buttons **inside** the Splash card body | **L2b In-card control** |
| Pomodoro timer, live countdown | strong state (mode, elapsed) | 1 Hz continuous | start / pause / reset | **L3 Stateful host** |

These four rows increase in implementation cost. They are also the right
order to ship:

- **L1 and L2a have no technical unknowns** — the click-to-Rust path on
  Phase 4c external action buttons is already implemented in production
  (see §Technical Feasibility).
- **L2b depends on a micro-PoC** — the bridge from a Splash-eval-produced
  button to the outer Rust host is almost certainly functional per the
  source evidence below, but needs a ~30-minute runtime check to confirm
  `ids!()` path resolution through the dynamic tree.
- **L3 depends on the same micro-PoC plus a real lifecycle design** — the
  hard part is not the `MiniApp` trait, it's integrating the host with
  `RoomScreen` / `PortalList` / `TimelineUiState`.

## What Splash is good at, and what it is not

Splash is a **view-layer DSL**: `set_text(splash_code)` evaluates a string and
produces a real Makepad `View` widget tree. It is naturally suited for:

- Pure rendering (L1 is a straight fit)
- Button clicks inside an evaluated tree — a `Button` emitted from the Splash
  eval produces a standard `ButtonAction::Clicked`, and the Makepad action
  queue plus dynamic-path `WidgetRef` lookup give the outer Rust host a way
  to catch it (see §Technical Feasibility)

It is **not** designed for:

- High-frequency ticks on very large trees — re-evaluating and rebuilding a
  50-node Splash tree every frame drops GPU caches. 1 Hz is fine; 60 Hz is
  not without widget reuse.
- Durable state across Matrix events — each `set_text` call builds a fresh
  widget subtree; any variable living inside the Splash body is thrown away.
- Complex local state management — Animator instance variables are for
  animation, not for modelling an app's data.

**Conclusion:** Splash is the *rendering engine* for mini-apps, not the
*runtime*. Anything beyond L1 needs a thin Rust-side host that owns the state
and uses Splash to render.

## Proposed architecture

```
┌──────────────────────────────────────────────────────┐
│ L3: Mini-App Host (Rust, stored per (room_id,        │
│     event_id))                                        │
│   - state:   Box<dyn MiniAppState>                    │
│   - on_tick(cx, now) → Option<splash_code>            │
│   - on_action(action_id) → ActionResult               │
│   - render() → splash_code                            │
└──────────────────────────────────────────────────────┘
         ↓ produces splash code
┌──────────────────────────────────────────────────────┐
│ L1/L2b rendering: Splash widget (existing)            │
│   splash_widget.set_text(cx, code)                    │
└──────────────────────────────────────────────────────┘
         ↕ click events (L2b: in-card;
         │                L2a: external action row)
┌──────────────────────────────────────────────────────┐
│ Matrix protocol (composed, NOT absorbed):             │
│                                                       │
│   org.octos.app              →  app data + lifecycle  │
│   org.octos.actions          →  interactive buttons   │
│   org.octos.action_response  →  click round-trip      │
│                                                       │
│   org.octos.app does NOT contain `actions`.           │
│   The two fields coexist in the same event.          │
└──────────────────────────────────────────────────────┘
```

Splash stays a pure renderer. State and ticks live one layer up in the host.
The protocol is explicitly **composed** across three custom fields, not
merged into one.

## Protocol draft (composed envelope)

A Matrix event delivering a mini-app with interactive buttons looks like this:

```json
{
  "msgtype": "m.text",
  "body": "⏱ Pomodoro 25:00",

  "org.octos.app": {
    "type": "pomodoro",
    "version": 1,
    "app_semantic_id": "pom_abc123",
    "initial_state": {
      "mode": "work",
      "duration_seconds": 1500,
      "started_at": "2026-04-14T02:30:00Z"
    },
    "client_tick": true
  },

  "org.octos.actions": [
    {"id": "pause", "label": "⏸", "style": "secondary"},
    {"id": "reset", "label": "🔄", "style": "secondary"}
  ]
}
```

Field semantics:

- `type` — key into the client-side registry (whitelist).
- `version` — per-type schema evolution.
- `app_semantic_id` — **renamed from the earlier `instance_id`. This is
  advisory metadata the app itself can use** (e.g., to correlate two
  pomodoros in the same room), but **it is NOT the host storage key**.
  Robrix must not trust it for identity.
- `initial_state` — what the agent sends; client may mutate during the
  lifetime of the app instance without writing back to Matrix.
- `client_tick` — opt-in for continuous local ticking (L3).
- `org.octos.actions` — the existing Phase 4c button protocol; reused
  verbatim, not re-invented.

**Host storage key:** `(room_id, event_id)`. Both come straight from the
Matrix event and are homeserver-signed and immutable. Any app instance owned
by the client is keyed on this pair.

## Immutability: `m.replace` is ignored for app envelopes

Any `m.replace` edit targeting a message that carries `org.octos.app`, or
`org.octos.actions`, or `org.octos.approval_request`, must not be allowed to
mutate those fields in the client's view. Robrix always reads app metadata
from the **original** event content, never from `m.new_content`.

Reasons:

- Consistency with Phase 5 approval requests, which already enforce this to
  prevent an edit from silently changing `authorized_approvers`.
- Consistency with Phase 4c action buttons, which are also treated as
  client-side immutable — bots update state by sending a new message.
- Host state is stored per `(room_id, event_id)`. Allowing the envelope to
  change under that key would break state invariants in subtle ways.

If a bot wants to update a running app (new weather data, new pomodoro
state, new action list), it sends a **new event**, not an edit. The old
event's app instance is torn down when the new one is rendered.

## Lifecycle integration with RoomScreen / PortalList / TimelineUiState

This is the single hardest design point for L3 and must live in the master
spec, not be deferred to the host sub-spec.

### The problem

- `PortalList` recycles timeline item widgets. A `Message` widget that
  scrolls offscreen is **reused** for a different event when it comes back.
- `TimelineUiState` holds per-room scroll position, editing state, etc.
- `#[rust]` fields on widget structs are therefore **not** a safe place to
  store per-message host state — they will be shared across events during
  recycling.

### The decision

- **Host state storage lives on `RoomScreen` (or wherever
  `TimelineUiState` lives)**, keyed on `(room_id, event_id)`.
- The `Message` widget receives a non-owning handle at draw time. It does
  not own the host and does not keep host state in its own fields.
- When a `Message` widget is recycled for a different event, it looks up
  the new `(room_id, event_id)` and retrieves the matching host (or
  nothing, for non-app messages).
- When a timeline scrolls far enough that the event is evicted from the
  in-memory window, the host is **torn down** via `teardown()` and its
  state is lost (v1: no persistence).
- When the event re-enters the window, the host is re-initialised from
  `initial_state` (v1) — state from before eviction is not restored.
- Room switch: all hosts owned by the leaving room are torn down.

### v1 simplifications

- No persistence across restarts.
- No restoration across eviction (scroll out far, scroll back: pomodoro
  resets to `initial_state`).
- No cross-device sync.
- These can be revisited after the first real user need.

## Mapping the example apps

### Weather card (L1 static)
- `render(data) -> splash_code` as a pure function.
- **No host needed.** The weather type is registered in the new `type`
  registry and routed via `org.octos.app` — **not** by extending
  `org.octos.splash_card`. The registry is a small registry module (exact
  file name and code organisation are deferred to the L1 sub-spec per
  master spec §Out of Scope) whose entries implement only `init + render`
  for L1 apps. Raw `splash_card` stays as a development-only backdoor and
  is disabled in production builds (see master spec §安全与校验).
- Refresh button (if present) is an external `org.octos.actions` row — the
  card itself stays stateless.
- **Ship target:** immediately after master spec lands. No technical
  unknowns.

### News reader (L2a external action row)
- Agent owns the cursor; each card is one article plus a next / open row.
- Click on "next" → existing Phase 4c `org.octos.action_response` → agent
  looks up the next article and sends a replacement card.
- No client tick, no persistent client state.
- **Not blocked on the micro-PoC** — the external action row path is
  already proven by Robrix's existing Phase 4c implementation (see
  §Technical Feasibility).

### Zoom-on-tap or in-card control (L2b in-card control)
- The button lives **inside** the Splash-rendered body, not in an
  external row below the card.
- Requires a 30-minute micro-PoC to confirm that `splash_ref.button(cx,
  ids!(<dynamic_child>))` resolves through a `set_text()`-created tree
  and that `ButtonAction::Clicked` reaches the outer `RoomScreen` action
  handler.
- Source evidence makes success the strong prior; the PoC is confirmation,
  not a genuine spike.

### Pomodoro timer (L3 stateful host)
- Strong local state + 1 Hz client tick. Agent-side ticking is
  unacceptable — one Matrix event per second would be a bandwidth disaster.
- Needs a real `MiniApp` trait:
  ```rust
  trait MiniApp {
      fn init(initial_state: Value) -> Box<dyn MiniApp>;
      fn on_tick(&mut self, now: Instant) -> Option<String>;  // Some(splash) if rerender
      fn on_action(&mut self, action_id: &str) -> ActionResult;
      fn render(&self) -> String;
      fn teardown(&mut self) {}
  }
  ```
- Robrix schedules `NextFrame` during draw of the message, calls `on_tick`
  once per second, re-calls `splash_widget.set_text` when `on_tick` returns
  `Some`.
- Pause / reset mutate local state, no Matrix round-trip.
- The *real* work is the §Lifecycle integration above, not the trait
  signature.

## Technical Feasibility (with source refs)

Evidence gathered from the Makepad checkout currently locked in `Cargo.lock`
(`kevinaboos/makepad @ cargo_makepad_ndk_fix`, revision `5e6d7b3`, cached at
`~/.cargo/git/checkouts/makepad-69d78fae78fc8901/5e6d7b3/`) and from the
Robrix codebase.

### Splash is a thin wrapper around View (Claude)

`widgets/src/splash.rs` is 94 lines end to end:

- `Splash` has `#[deref] pub view: View` and a `body: ArcStringMut`.
- `eval_body` (lines 32–59) prefixes the body with
  `"use mod.prelude.widgets.*View{height:Fit, "`, calls
  `vm.eval_with_append_source(...)`, and **assigns the result to
  `self.view`** via `View::script_from_value(vm, value)`. That is a real
  Makepad `View`, not a virtual tree.
- `handle_event` (lines 62–65) is literally
  `self.view.handle_event(cx, event, scope);` — no event filtering, no
  action interception. Everything that a normal `View` routes, Splash
  routes.
- `set_text` (lines 79–85) re-runs `eval_body` and `redraw`.

**Implication:** the children of a Splash subtree behave exactly like the
children of a statically-declared `View`.

### Buttons emit actions with widget_uid + optional action_data (Claude)

`widgets/src/button.rs` lines 553–585 show `Button::handle_event` calling:

```rust
cx.widget_action_with_data(
    &self.action_data,
    uid,
    ButtonAction::Clicked(fe.modifiers),
);
```

The `action_data` field is declared `#[action_data] #[rust]` (line ~460),
and `widget_action_with_data` stores `action_data.clone_data()` into
`WidgetAction::data` (widget.rs lines 1651–1663). This means outer Rust
can read the data in addition to the `widget_uid`.

Caveat: because the field is `#[rust]`, Splash DSL cannot set
`action_data` directly — Rust must walk the eval tree and call
`set_action_data(...)` after `set_text()`, OR store a
`HashMap<WidgetUid, ActionContext>` keyed on the button's UID (the latter
is already the pattern Robrix uses for Phase 4c action buttons —
`octos_action_button_contexts` in `src/home/room_screen.rs:6877–6882`).

### Dynamic child lookup works (Codex)

- `widgets/src/widget.rs:919` — `WidgetRef::widget()` refreshes its
  internal reference to track the current dynamic subtree before resolving
  a path.
- `widgets/src/widget_tree.rs:3740` — upstream test
  `test_widget_ref_helper_tracks_dynamic_nested_child_like_child_by_path()`
  explicitly verifies that nested child widgets can be found by path
  even after the child tree has been replaced.

This is the strongest piece of evidence: **Makepad upstream has a test
covering exactly the scenario we need**. Dynamic-subtree path lookup is a
first-class, tested feature, not an accidental property.

### Robrix already catches button clicks from rendered trees (Claude)

The Phase 4c action-button flow in `src/home/room_screen.rs`:

- Lines 1969–2030: the action row is rendered, each button's
  `widget_uid()` is stored in `octos_action_button_contexts`.
- Lines 6877–6920: in `handle_message_actions`, the code iterates the map,
  calls `actions.find_widget_action(widget_uid)`, downcasts the action to
  `ButtonAction::Clicked`, and dispatches.

This is the exact HashMap<WidgetUid, Context> pattern that would work for
Splash-embedded L2b buttons. The only difference is that the UIDs would
come from walking the dynamic Splash subtree after `set_text()`, rather
than from the static action row.

### Net conclusion

- **L1 and L2a:** no technical unknowns. Ship freely.
- **L2b:** the click bridge is strongly expected to work. The unknown has
  shrunk from "does the bridge exist at all" to "does `ids!()` path
  resolution + `UID` capture work across a `set_text()` boundary in our
  specific embedding". A 30-minute micro-PoC settles it.
- **L3:** same click bridge as L2b, plus the lifecycle integration design
  above. The click bridge is no longer the main risk; the lifecycle work
  is.

## Remaining hard questions

| Question | Status | Mitigation |
|---|---|---|
| Does `splash_ref.button(cx, ids!(dynamic_child))` resolve after `set_text()`? | **Almost certainly yes** per upstream test, but unverified in our embedding | 30-minute micro-PoC |
| What's the re-eval cost at 1 Hz for a ~50-node Splash tree? | Unmeasured | Measure once L3 has a real app running; fall back to widget reuse if needed |
| How does host state survive scroll-out and room switch? | Decided: eviction tears down, `initial_state` re-inits on return. See §Lifecycle | v1 simplification; revisit when users complain |
| How do we stop a malicious bot from sending bogus types / params? | Decided: registry is a whitelist, each type's `init` validates inputs | Will be enforced in master spec |
| Do we need persistence across restarts? | v1: no | Revisit per user need |
| How does this interact with Phase 5 approval requests? | Decided: approval + app envelopes both stay immutable under `m.replace` | Documented above |

## Recommended implementation order

1. **Write and lint master spec** (`specs/task-agent-to-app-system.spec.md`).
   Locks the envelope, host key, immutability, layering, and lifecycle
   decisions above.
2. **L1 weather card** (own sub-spec). New registry module dispatching on
   `org.octos.app.type`, plus the `org.octos.app` parsing path in
   `room_screen.rs`. Exact module name and code organisation are
   deferred to the L1 sub-spec (master spec §Out of Scope). Does NOT
   extend `org.octos.splash_card` — that path remains a development-only
   backdoor per the master spec. No dependency on anything else.
3. **L2a news reader** (own sub-spec). Reuses Phase 4c external action
   buttons. No dependency on the micro-PoC.
4. **L2b micro-PoC** (one scratch commit, not a sub-spec). 30-minute runtime
   check: create a Splash with a named Button, click it, confirm the outer
   `RoomScreen` receives `ButtonAction::Clicked`. Keep or delete the
   commit based on outcome.
5. **L2b in-card controls** (own sub-spec, gated on step 4 passing).
6. **L3 mini-app host + pomodoro** (own sub-spec). The biggest piece. Uses
   the lifecycle design above as its contract.

Steps 2 and 3 can run in parallel with step 4. Step 5 and step 6 can run in
parallel once step 4 passes.

## Non-goals (for v1)

- Cross-device state sync (only ephemeral local state).
- Arbitrary Splash code from unknown types (registry is a whitelist; raw
  Splash path stays for development but is locked down before GA).
- Full app store / dynamic type registration at runtime (every type is
  compiled into Robrix).
- Mini-apps outside the timeline (no standalone tabs, no notification
  pop-ups).
- **Note:** "bot updates via `m.replace`" is no longer just a non-goal —
  it is actively forbidden by the immutability rule above.

## Next actions

- [x] 2026-04-14: Incorporate Codex's 5 findings and both teams' Makepad
      source audits into this roadmap.
- [x] 2026-04-14: Write `specs/task-agent-to-app-system.spec.md` backed by
      this revised roadmap.
- [x] 2026-04-14: `agent-spec lint --min-score 0.7` the master spec to
      Quality 100%.
- [x] 2026-04-14: Fix Codex post-review nits (roadmap L1 section referenced
      old `splash_card` path; Next actions status stale).
- [ ] After master spec passes final Codex review, derive L1 sub-spec
      (`specs/task-agent-to-app-l1-weather-card.spec.md`).
- [ ] In parallel, run the L2b micro-PoC and record the result.
- [ ] Derive L2a news reader sub-spec.
- [ ] Derive L3 host runtime sub-spec (the biggest piece; uses the
      lifecycle section of master spec as its contract).
