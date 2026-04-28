# Feasibility Study: Airbnb-like Booking Flow on agent2app

**Date:** 2026-04-28
**Status:** Feasibility analysis — no implementation commitment
**Owner:** Robrix + OctOS
**Snapshot:** branch `feat/agent-to-app`, commit `ab53096a`

## Purpose

Answer the question: *can the current agent2app mini-app system support an
Airbnb-like multi-step booking flow?* This document is an analysis, not a
plan; it stratifies what is achievable today, what requires already-spec'd
but unbuilt layers, and what would need entirely new specs.

It is meant as a reference for prioritization discussions, especially
when evaluating which non-informational capabilities to introduce as
proving grounds for L2 / L3 work.

## TL;DR

- **Full native Airbnb UX (forms, in-card pickers, in-app payment): not feasible.**
- **Chat-driven booking (cards for display, dialog + enumerated buttons for input,
  external-link payment): feasible today as a new `booking` capability inside
  the existing L1 + L2a envelope, ~2-4 days of work per side.**
- The recommendation is to build the chat-driven version first as the first
  non-informational capability, and use what we learn to drive future L2 form-input
  / L3 host-state / Sensitive-trust spec work.

## Restated Question

Given the agent2app mini-app system as defined by `specs/task-agent-to-app-*.spec.md`
and implemented in `src/home/app_registry/` (currently L1 weather + news working,
L2a action-button protocol working, L2b gated on PoC, L3 unimplemented), can it
support a complete Airbnb-style booking flow:

1. search input (location + dates + guests)
2. listings browse (photo grid, price, rating)
3. property detail (gallery, map, amenities, reviews, host)
4. date range selection
5. guest count selection
6. order preview + price breakdown
7. payment
8. confirmation + booking status

Each step has a *display* half and an *input* half. Native Airbnb fidelity is
form-heavy; that is the load-bearing question.

## Section 1 — What an Airbnb Flow Demands

| Step | Display | Input |
|------|---------|-------|
| 1. Search | — | city + date range + guests |
| 2. Listings | image grid / cards / price / rating / pagination | tap a listing |
| 3. Detail | gallery / map / description / amenities / reviews | tap "book" |
| 4. Dates | availability calendar | range date picker |
| 5. Guests | breakdown | numeric stepper or dropdown |
| 6. Preview | itemized bill | confirm |
| 7. Payment | method selector | secure card / wallet input |
| 8. Confirm | booking summary | none (status pushed) |

Form input dominates steps 1, 4, 5, 7. Steps 2, 3, 6, 8 are mostly display.

## Section 2 — What agent2app Provides Today

References:
- `specs/task-agent-to-app-system.spec.md` lines 86-108 (L1/L2a/L2b/L3 layered contract)
- `src/home/app_registry/widget_manifest.rs` (widget allowlist)
- `src/home/app_registry/local_functions.rs` (local function allowlist)
- `specs/task-agent-to-app-composite-response.spec.md` (card + body twin surfaces)

### Render layer (L1, shipped)

- Bot sends Matrix event with `org.octos.app: { type, version, initial_state }`.
- Robrix's type registry is a **compile-time closed allowlist**; only `weather`
  and `news` are recognized today. Unknown `type` falls back to plain body text,
  no Splash eval.
- Templates live at `templates/<capability>/<template>.splash`, with `$state.path`
  bindings + host-injected attribution chrome.
- Widget palette (entire allowlist): `View`, `Label`, `Icon`, `Image`, `Button`,
  `RoundedView`. **No input controls.**
- Local functions: `open_url`, `format_date`, `format_number`, `required`,
  `regex_match`. **No form-handling helpers.**
- Five-stage template preflight + cache + fail-to-body fallback.

### Interaction layer (L2a, shipped)

- Same event carries `org.octos.actions` for an external button row.
- Click → Phase 4c path → `org.octos.action_response` back to original sender.
- **Each button conveys only an `action_id`; no form-field payload semantics.**

### State layer

- Host state keyed on `(room_id, event_id)`, owned by `RoomScreen` /
  `TimelineUiState`.
- `m.replace` immune: edits to envelope are ignored; updates require new events.
- Scroll-out triggers `teardown`; local state does not persist across eviction.
- v1 simplifications: no cross-restart persistence, no cross-device sync.
- **All multi-step state lives in the agent.** Robrix has no notion of a
  "booking session."

### Composite response (shipped)

- One event can carry both `org.octos.app` (card) and Matrix `body` (detail text)
  derived from one capability fetch.
- This is *not* multi-component composition — it is "card plus text bubble."

## Section 3 — What's Spec'd But Not Yet Built

| Layer | Capability | Status | Blocker |
|-------|------------|--------|---------|
| L2b | In-card buttons (Splash-DSL `Button` clicks) | spec'd, gated | PoC for `splash_ref.button(cx, ids!(<dynamic>))` resolving after `set_text` (master spec lines 96-98) |
| L3 | client-driven tick + local persistent state | spec'd, unimplemented | full lifecycle spec landing + host runtime trait + tick scheduler |
| Phase 5 | third-party capability distribution / marketplace | concept-only | no concrete spec |

## Section 4 — Hard Gaps (No Spec, Not on Roadmap)

1. **Form widgets**: `TextInput`, `DatePicker`, `RangeDatePicker`, `Dropdown`,
   `Checkbox`, `RadioGroup`, `Stepper`. Not in the manifest; any template that
   references them fails preflight and falls back to body text.
2. **Keyed action payloads**: `org.octos.action_response` only carries `action_id`,
   no "form fields" envelope.
3. **Client-side persistent booking session**: lost on room switch, restart, or
   scroll-out eviction.
4. **Sensitive trust tier + payment widgets**: not mentioned in any spec.
5. **Multi-component containers**: `View` can stack children, but `PortalList` /
   `LazyGrid` / `Carousel` are not exposed to Splash; long lists render eagerly.
6. **Map & rich media**: no Map widget, no Image carousel or gestures.

## Section 5 — Step-by-Step Feasibility Map

| Step | Today | Workaround | Native gap |
|------|-------|------------|------------|
| 1. Search input | ❌ no native input | user types in Matrix input → OctOS NLU extracts slots | DatePicker / NumericInput / form widget set |
| 2. Listings | ✅ L1 card with rows of `RoundedView` + `Image` + `Label` | minor visual fidelity loss only | Carousel / LazyGrid for performance |
| 3. Detail | ✅ dense L1 card, image + label composition | static gallery only, no swipe | Image carousel / Map widget |
| 4. Date selection | ❌ no DatePicker | L2a buttons "Today / Tomorrow / Weekend / Custom" → custom falls through to text + NLU | RangeDatePicker (L2 form input) |
| 5. Guest count | ❌ no NumericInput | L2a buttons 1/2/3/4+ | Stepper or NumericInput |
| 6. Price calc | ✅ display | every choice triggers a new agent event with recalculated card | L3 host state for local recalculation |
| 7. Payment | ⚠️ `open_url` jump to a Stripe Checkout URL | external redirect + agent listens for webhook → posts confirmation card | Sensitive trust tier + payment widgets (no spec) |
| 8. Confirmation | ✅ L1 card | status changes propagated as new agent events | none |

### UX comparison

- **Airbnb native**: screen-by-screen form flow, real-time client validation,
  reactive price changes within one view.
- **agent2app today**: hybrid of chat + cards + buttons; all reasoning lives in
  the agent; the client is a sophisticated renderer.
- Closest analogy: an upgraded "OpenTable Telegram bot," not an Airbnb iOS app.

## Section 6 — Three Path Options

### Option A — Maximize the current envelope (~2-4 days each side)

Implement a `booking` capability fully inside L1 + L2a, no protocol changes.

**OctOS side**
- New capability `booking` implementing the existing capability trait (mirror of `weather_guidance`).
- Resolver trained to recognize booking intent and extract slots
  `{location, date_range, guests, property_id, action}`.
- Dispatcher state machine manages booking sessions in Redis or SQLite, picks
  the appropriate card per step.
- Each step yields `(initial_state, body)` + an `org.octos.actions` button row.

**Robrix side**
- New `type = "booking"`, version 1.
- `src/home/app_registry/booking.rs`: `init` validates schema, `render` selects
  template by step.
- `src/home/app_registry/templates/booking/`: `search.splash`, `list.splash`,
  `detail.splash`, `confirm.splash`, `paid.splash`.
- Templates use only the existing widget palette + `${open_url}` for payment redirects.
- `widget_manifest.rs` / `local_functions.rs` unchanged.
- `mod.rs::registry()` adds one registration.
- `capability_descriptors.rs` adds `booking` descriptor + chrome metadata.

**Payment**
- Agent generates Stripe Checkout URL → `org.octos.actions` button id `pay` →
  client uses `open_url` to launch external browser.
- Webhook delivery posts a confirmation event back to the room.

**Hard constraint**: every "input" must be either free-form chat (handled by NLU)
or a preset button. No on-card date entry, no on-card numeric entry.

**Output**: all eight steps walkable. UX feels like a chat bot with rich
read-only cards, not a native booking app.

**Risks**: resolver accuracy under the producer-routing spec's ≥90% bar;
state-machine semantics must be legible enough that users do not get lost
between steps.

### Option B — Wait for L2b PoC (incremental ~1-2 weeks)

Once L2b lands, in-card radio-group buttons let the date / guest selection
step move *inside* the card instead of producing a fresh card per choice.
Still no free-form input. Prerequisite chain: PoC → L2b sub-spec → impl.

### Option C — Full native UX (multi-month, protocol-level work)

Requires landing, in series:
1. **L2 form-input sub-spec**: `DatePicker` / `RangeDatePicker` / `NumericInput`
   / `TextInput` / `Dropdown` — each needing a widget kind, preflight rules,
   value validation, and round-trip schema.
2. **`action_response` extension**: `action_id` → `action_id + form_payload`
   with schema validation.
3. **L3 host-state sub-spec implementation**: cross-step local state (selected
   dates, guest count, etc.).
4. **Sensitive trust tier + payment widget sub-spec**: brand-new threat-model
   discussion required.
5. **Gallery / map widgets**: image carousel + map widget (the latter likely
   requiring new shaders).

Each item is its own spec → review → PoC → implementation cycle.

## Section 7 — Recommendation

**Build Option A as the first non-informational agent2app capability.**

Rationale:
1. Validates whether agent2app can host "multi-step interaction with
   agent-side state machine" — the system's own thesis.
2. Requires zero protocol-level changes; all work fits inside the approved L1 + L2a contracts.
3. Generates concrete evidence to inform any future L2b / L3 / Sensitive-trust
   sub-specs (we'll know which corners hurt before we spec the corner).
4. Low cost of failure: the worst case is that we have measured the ceiling of
   the current stack and produced a reusable capability template.

**Do not start Option C** before Option A's lessons land. The roadmap explicitly
defers form widgets and payment to Phase 6+, and writing those specs without
real-usage signal is premature.

## Section 8 — Critical Files & Specs

If Option A is approved, these are the touch points:

- `specs/task-agent-to-app-system.spec.md` — read-only constraint reference.
- `specs/task-agent-to-app-composite-response.spec.md` — booking card + body
  twin surface follows this pattern.
- **`specs/task-agent-to-app-l2a-booking-capability.spec.md`** — drafted in
  this round; contains the schema, per-step templates, action button matrix,
  validation rules, and acceptance scenarios.
- **`fixtures/airbnb-mock/`** — 500 deterministic mock listings (real images
  from airbert-vln/bnb-dataset, synthesized structured fields).
  See `fixtures/airbnb-mock/README.md`.
- `src/home/app_registry/mod.rs` — add `register("booking", BookingFactory)`.
- `src/home/app_registry/booking.rs` — new file modeled on `weather.rs`.
- `src/home/app_registry/templates/booking/*.splash` — new template directory
  (`search` / `results` / `detail` / `confirm` / `booked`).
- `src/home/app_registry/widget_manifest.rs` — verify existing allowlist
  suffices (expected: no change).
- `src/home/app_registry/local_functions.rs` — verify `open_url` works as the
  payment redirect (expected: no change).
- `src/home/app_registry/capability_descriptors.rs` — add `booking` descriptor
  + chrome metadata.

OctOS (out-of-tree):
- new capability impl reading from `fixtures/airbnb-mock/listings.json`
- resolver retraining (≥50 EN+ZH phrases, ≥90% hit rate)
- booking session state store (in-memory `HashMap<session_id, BookingSession>`)

## Section 9 — How to Audit This Analysis

1. **L1/L2a/L2b/L3 layering**: `specs/task-agent-to-app-system.spec.md` lines 86-108.
2. **Widget allowlist excludes form widgets**: read `src/home/app_registry/widget_manifest.rs`.
3. **`open_url` available, no form helpers**: read `src/home/app_registry/local_functions.rs`.
4. **L2b gated on PoC**: master spec lines 96-98 plus the
   `test_l2b_is_gated_on_splash_button_micro_poc` acceptance scenario.
5. **State does not survive eviction**: master spec lines 119-120 ("v1 不恢复
   pre-eviction 的本地变化").
6. **No payment / Sensitive coverage**: `grep -irE "payment|sensitive|stripe"
   specs/task-agent-to-app-*.spec.md` should return nothing.

## Out of Scope

- Detailed UX wireframes for the chat-driven flow.
- OctOS-side resolver training data design (regression fixture details).
- Any commitment to L2b / L3 / Sensitive-trust timelines.

## Status — 2026-04-28 update

- `booking` capability sub-spec drafted:
  [`specs/task-agent-to-app-l2a-booking-capability.spec.md`](../../specs/task-agent-to-app-l2a-booking-capability.spec.md).
- 500-listing demo fixture committed: [`fixtures/airbnb-mock/`](../../fixtures/airbnb-mock/).
- Implementation has not started; awaiting user approval to proceed.
