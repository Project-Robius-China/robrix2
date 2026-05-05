# Agent-to-App System Design

**Status:** Long-term architecture note. For the current Robrix2 v1 implementation
contract, use [`agent-to-app-simplified-design.md`](agent-to-app-simplified-design.md).
This document is intentionally broader than the shipped static-template path and
should not be used as the next implementation checklist.

**Last revised:** 2026-04-23
**Related:** `specs/task-agent-to-app-system.spec.md` (master contract),
`specs/task-agent-to-app-producer-routing.spec.md` (producer half),
`specs/task-agent-to-app-splash-host-evolution.spec.md` (Layer 2 host),
`specs/task-agent-to-app-template-runtime.spec.md` (host runtime contract),
`docs/roadmap/2026-04-17-agent-to-app-implementation-plan.md` (phased work).

---

## 1. Purpose

This document is the **architectural floor** for agent-to-app work in
Robrix + OctOS. Specs describe what individual tasks verify;
implementation plans describe when work lands. This document describes
**what the system is, layer by layer, and what it will never be** — so
every future spec, plan, and review has a shared reference.

It is not a spec (no BDD scenarios), not a plan (no dates), not a
tutorial. It is a contract between contributors and future contributors.

---

## 2. Problem statement

Traditional LLM agents reply with markdown / prose. Rich visual replies
(cards, timelines, interactive panels) require one of:

1. **HTML / WebView artifacts** (ChatGPT Canvas, Claude Artifacts):
   heavy, sandboxed, security-sensitive, breaks the "native feel" on
   Matrix/Telegram/mobile clients.
2. **Per-app hand-coded UI in the client**: forces every new capability
   through a coordinated client-side release; agent capabilities can
   never out-pace client releases.
3. **LLM directly emitting UI code**: unsafe, unpredictable, breaks on
   prompt or model changes.

None of those match our constraints:

- Matrix native rendering (no WebView tax)
- Same visual data flows cross-channel (Matrix → Telegram → CLI)
- Agent can add capabilities without client releases
- Every capability is testable, regressible, giveable a gated rollout

We need an architecture that lets the agent ship native-rich replies
without any of the failure modes above.

---

## 3. Architectural principle (the one sentence)

> **The system has exactly one Rust UI codebase (host + widget
> library); every app is a declarative four-part artifact — schema,
> state, template, actions — composed from that shared UI. LLMs
> make choices inside whitelists the capability declares; they never
> write code.**

Everything below is a corollary of this sentence.

---

## 4. The four-part app contract

Every app is exactly these four artifacts. Nothing more, nothing less.

### 4.1 Schema

A JSON Schema describing the shape of `state`. Pinned by
`(type, version)` from the `org.octos.app` envelope. Version bumps are
governed by the master spec's evolution rules (optional fields don't
bump; required / semantic changes do).

**Author:** capability writer (Rust `Capability` impl)
**Lifetime:** compile-time constant; changes only in capability version
bumps

### 4.2 State

A concrete instance of the schema for one user turn. Produced by
`Capability::build_state(slots, data) -> JsonValue`.

**Author:** capability runtime
**Lifetime:** one turn — not persisted in the capability (the Matrix
event that carries it is the authoritative record)
**Paired-output invariant:** `build_state` and `build_body` share the
same `data` from a single `fetch_data` call. No drift possible.
**Incremental update semantics:** future L2/L3 capabilities may update
subtrees of `state` with path-scoped replacement/remove semantics
without redefining template structure. This borrows the useful part of
A2UI's `updateDataModel` idea without adopting A2UI's full
component-graph protocol.

### 4.3 Template

A Splash composition that renders `state`. A template is always
**shape-identical at the wire**: widget tree bound to `state` via
declarative `$state.foo` paths, no imperative Rust. It may arrive at
the Splash Host through one of two authorship modes — these are
distinct in origin but interchangeable at render time:

#### 4.3a Static template

A `*.splash` file committed to the Robrix client repo under
`src/home/app_registry/templates/<capability_id>/`. Pinned by
`(capability_id, template_id)`. This is the default mode for every
capability. OctOS emits schema/state/body; the Splash template belongs
to the client-side host and widget library.

- **Author:** capability writer (human)
- **Lifetime:** compile-time artifact on disk, with dev-time hot-reload
- **Versioning:** tied to capability's `app_version`; template content
  changes that alter state contract bump the capability version

#### 4.3b Generated template (opt-in per capability)

A Splash snippet produced at runtime by a Template-Author LLM (Layer
5b, §5) within the capability's bounded mode. Must pass Splash Host
validation (widget whitelist W5, prop schema, no imperative code, no
unbounded recursion) before rendering. Caching by
`(capability_id, state_hash, focus, language)` is permitted; any
validation failure falls back to the capability's static template.

- **Author:** Template-Author LLM (5b)
- **Lifetime:** per-turn value (optionally cached); NOT a
  compile-time artifact
- **Fallback:** every capability that enables generated templates MUST
  also ship a static template as the safety fallback

Capabilities declare which `template_id`s are static vs generated in
`Capability::template_ids()`. The Resolver LLM (5a) only ever picks a
`template_id`; it never knows or cares whether that id resolves to a
static file or a generated snippet.

### 4.4 Actions

A declaration of which interactive actions the app accepts. Each action
has:

- `id` — stable string used in `org.octos.actions`
- `params_schema` — JSON Schema for the action payload
- `trust_level` — whether this action can be invoked without human
  confirmation

Handled by `Capability::on_action(action_id, payload)`.

**Author:** capability writer
**Lifetime:** compile-time constant per capability version

Actions may target either:

- **Remote agent events** — routed back to Layer 3 over transport
- **Host-owned local functions** — named primitives executed entirely by
  Layer 2 and governed by W7; templates and capabilities may reference
  them, but do not define arbitrary local code

---

## 5. Layer architecture

Five layers, each with one purpose and hard boundaries against the
others.

```
┌──────────────────────────────────────────────────────────────┐
│ LAYER 5 — LLM (two distinct roles; never conflated)          │
│                                                              │
│  5a. Resolver LLM (always-on, per user turn)                 │
│      Identifies intent; picks capability + slots + focus +   │
│      template_id. Output: strict JSON matching               │
│      resolver_output_json_schema. NEVER writes code; NEVER   │
│      names a widget; NEVER emits Splash.                     │
│                                                              │
│  5b. Template-Author LLM (opt-in per capability, §9)         │
│      When a capability chooses LLM-generative mode for a     │
│      template slot, this LLM produces a Splash snippet       │
│      constrained by widget manifest (W5) and schema. Its     │
│      output is parsed, validated, and rejected if it names   │
│      any widget outside the W5 `public` whitelist.           │
│                                                              │
│  The two roles may use different models, different prompts,  │
│  and different rollout gates. The Resolver LLM exists for    │
│  every turn; the Template-Author LLM only runs when the      │
│  capability's selected `template_id` points at a generative  │
│  slot (NOT a static file).                                   │
└──────────────────────────────────────────────────────────────┘
                                ↓
┌──────────────────────────────────────────────────────────────┐
│ LAYER 4 — Dispatcher (Rust, pure, no LLM)                    │
│ Validates LLM output against capability whitelist, slot     │
│ schema, focus enum, confidence threshold. Routes to          │
│ capability or falls through to legacy reply path. All        │
│ fallback reasons emit structured `tracing` events.           │
└──────────────────────────────────────────────────────────────┘
                                ↓
┌──────────────────────────────────────────────────────────────┐
│ LAYER 3 — Capability (Rust, thin)                            │
│ Declares: id, app_type, app_version, supported_focuses,     │
│ required_slots, min_confidence, template_ids, action_ids.    │
│ Runs: fetch_data → build_state / build_body → on_action.    │
│ Does NOT contain UI code.                                    │
└──────────────────────────────────────────────────────────────┘
                                ↓
┌──────────────────────────────────────────────────────────────┐
│ LAYER 2 — Splash Host (Rust, written once)                   │
│ Loads template files; validates widgets against trust       │
│ whitelist; binds state via declarative paths; applies        │
│ path-scoped state updates; routes action events back to      │
│ Layer 3; owns attribution chrome (display name / icon /      │
│ trust badge). Provides lifecycle (scroll-out teardown,       │
│ tick scheduling for L3). Shared by all capabilities —        │
│ never grows per-app.                                         │
└──────────────────────────────────────────────────────────────┘
                                ↓
┌──────────────────────────────────────────────────────────────┐
│ LAYER 1 — Widget library                                     │
│ Makepad built-ins + `script_mod!` project widgets +          │
│ third-party packages. Every widget has a `trust_level` tag   │
│ and a typed prop schema. New widgets extend this library     │
│ once and are then available to all capabilities.             │
└──────────────────────────────────────────────────────────────┘
```

**Boundary rules (enforced by code review + future lint):**

- Layer 3 (Capability) MUST NOT import Layer 1 (Widget) types directly.
  Widgets are only referenced by name inside Layer 4 template files.
- Layer 2 (Splash Host) is the single point where Layer 1 is resolved.
  No other Rust module instantiates widgets dynamically.
- Layer 5 (LLM) output flows only through Layer 4 (Dispatcher)
  validation. No direct bridge from LLM to Layer 2 or 1.
- Attribution (`capability_id`, display name, icon, trust badge) is
  owned by Layer 2 host chrome + envelope metadata. Templates render
  content regions only and MUST NOT override host-owned identity.

---

## 6. The seven whitelists (LLM freedom bounded by capability)

The LLM is *generative* within the capability's declared envelope; it is
never allowed to break out of that envelope. Seven whitelists enforce
this:

| # | Whitelist | Who declares | What it bounds | Example |
|---|-----------|--------------|----------------|---------|
| W1 | **Capability** | `CapabilityRegistry` at startup | Which `capability_id` strings the dispatcher accepts | Only `weather_guidance` exists today |
| W2 | **Slot schema** | `Capability::required_slots()` | Which slot keys the LLM may fill | `{location, time_scope, language}` |
| W3 | **Focus enum** | `Capability::supported_focuses()` | Which `focus` values are valid | `[overview, clothing, umbrella, outdoor]` |
| W4 | **Template whitelist** | `Capability::template_ids()` (future) | Which template files the capability can render | `[card_standard, card_compact]` |
| W5 | **Widget trust whitelist** | Layer 2 Splash Host + widget manifest | Which widgets templates may instantiate | Only `public` widgets are reachable from templates |
| W6 | **Action whitelist** | `Capability::action_ids()` | Which action ids the capability will accept on `on_action` | Unknown action ids are dropped before the capability sees them |
| W7 | **Local function whitelist** | Layer 2 host-owned function registry | Which local primitives templates may reference | `open_url`, validation checks, limited formatters |

When someone asks "can LLM go beyond this?", the answer is "no, one of
the seven whitelists blocks it." That's the shape of the safety
argument.

### 6.1 Host-owned attribution and identity

Every app reply has an identity envelope the template does not control:

- `capability_id`
- `display_name`
- `icon`
- optional trust or provenance badge

These values are bound by the dispatcher and host, not by the template.
If future sub-agent or third-party capability routing is added, the
orchestrator must verify and stamp these fields before rendering.
Templates may render app content, but they must not impersonate host
chrome or overwrite provenance.

For v1 built-in capabilities, the host derives these fields from the
Robrix-side static `CapabilityDescriptor` table. A transport-level
chrome channel is intentionally deferred until dynamic / third-party
capabilities exist.

---

## 7. Evolution state — where we are

### 7.1 Shipped / implemented in current branch

- `org.octos.app` envelope protocol + Robrix consumer registry (master
  spec baseline)
- Producer-routing: Resolver + Dispatcher + Capability trait +
  `weather_guidance` (Phase 1)
- Robustness gate: 80+ fixture phrasings, top-1 ≥ 90%, FPR = 0% on
  gpt-4o (Phase 2.3)
- Two-flag startup gating (`enable_capability_dispatcher` +
  `capability_dispatcher_gate_passed`) (Phase 2.4)
- Robrix Layer 2 `SplashHost` skeleton + static template loader seam
- `WidgetManifest` + W5 preflight enforcement for template-reachable
  widgets
- `LocalFunctionRegistry` + W7 preflight enforcement for host-owned
  local functions
- Render-time W7 formatter/check execution for `${format_number(...)}`,
  `${format_date(...)}`, `required(...)`, and `regex_match(...)`
- Host-owned `AttributionChrome` from Robrix-side static
  `CapabilityDescriptor` table
- Weather guidance static Splash template extraction on the Robrix
  consumer side (`templates/weather_guidance/card_standard.splash`)
- Template preflight harness: parse, W5, W7, attribution override, and
  `$state.*` binding-path schema checks
- `TemplateHandle` cache keyed by app/template/content/manifest/host
  compatibility fields
- Fallback/error shape primitives: `FallbackReason`,
  `ValidationError`, template fallback helper, and plain-text fallback
  boundary
- V1 path-scoped state updates on host-owned state: JSON Pointer
  `replace` / `remove`; `append` / `splice` remain deferred

### 7.2 Remaining gap — what still proves the target shape

The Robrix weather consumer no longer needs a Rust-heavy UI string
builder as its primary render path: it loads the static Splash template,
binds the weather view-model through `$state.*`, and falls back to the
legacy string builder only as a defensive local fallback.

This is still not the terminal proof. The full shape is only proven when
a second capability (planned: `news_guidance`) ships with schema + state
+ templates and no per-app Rust UI code, and when the OctOS-side weather
cleanup/regression gate is completed. Per user direction on 2026-04-23,
OctOS agent cleanup is deferred while Robrix host/runtime work proceeds.

### 7.3 Not yet shipped

- Capability-level template whitelist as a first-class W4 declaration
- Widget manifest generator (for LLM discoverability)
- Full incremental state update contract beyond v1 `replace` / `remove`
  (`append`, `splice`, array inserts, replay semantics)
- L2 interactive capabilities (actions + on_action)
- L3 stateful mini-app host runtime
- Layer 5b generated templates and repair loop
- `news_guidance` as the first zero-Rust-UI second capability
- Dynamic capability discovery / plugin loading
- Skill-with-UI packaging and template/version distribution

---

## 8. Target shape for a new capability

The target for any capability written after Phase B:

```
crates/octos-agent/src/capabilities/news_guidance.rs      (~80 lines)
  └─ impl Capability for NewsGuidance {
       fn id() → "news_guidance"
       fn supported_focuses() → &["headlines", "digest", "deep_dive"]
       fn required_slots() → SlotSchema(topic, time_range)
       fn template_ids() → &["headlines_card", "digest_card"]
       fn action_ids() → &["next", "open_source", "save"]
       async fn fetch_data(slots) → news items from API
       fn build_state(slots, data) → JSON conforming to news schema
       fn build_body(slots, data, language) → plain text summary
       fn on_action(action_id, payload) → capability-specific handler
     }

robrix2/src/home/app_registry/templates/news_guidance/
  ├─ headlines_card.splash
  └─ digest_card.splash
```

**No Rust UI code anywhere.** The templates compose widgets from the
shared Layer 1 library (plus any new project widgets added once for the
news domain — `NewsTile`, `SourceChip`, etc.).

Roughly **~80 lines of Rust + 2 Splash files**. Compare to the
historical `weather_guidance` baseline (~500 lines Rust including the
old 200-line `render_guidance_weather` path).

---

## 9. LLM-generative templates (bounded opt-in, not required)

This section is the operational detail of **§4.3b generated template**,
driven by the **Layer 5b Template-Author LLM**. Static template
(§4.3a, Layer 5a only) remains the default path.

A capability that declares a generated `template_id` must wire:

1. A **static fallback template** registered under the same capability
   with the same state-contract (hard requirement — never ship a
   generated-only template slot).
2. An **LLM invocation point** inside the capability's render path
   that calls the Template-Author LLM with:
   - Widget manifest filtered to `trust_level = public` (W5)
   - State schema (`(capability_id, app_version)`-pinned)
   - Focus hint and language
   - The static fallback's structural budget (max widget count,
     max nesting depth)
3. A **validation step** in the Splash Host:
   - Parse the emitted snippet
   - Every widget name must be on W5 public
   - Every prop value must match the widget's declared schema
   - Every local function name must be on W7
   - Reject imperative code (no script nodes, no eval)
   - Reject unbounded recursion (depth cap)
4. A **cache policy** keyed by `(capability_id, template_id,
   state_hash, focus, language)` — optional but recommended to keep
   latency bounded.
5. A **repair loop** — any of `(parse failure, widget not on
   whitelist, prop schema mismatch, unknown local function, budget
   exceeded)` emits a structured validation error
   `{code, path, message}` and MAY retry the Template-Author LLM once.
6. A **fallback trigger** — if the repair loop still fails, fall back
   to the static template and log a structured warning.

This preserves "LLM decides template shape" without granting unbounded
code-generation. The Template-Author LLM is a narrow tool under the
capability's supervision; it never touches the conversation with the
user, never sees the LLM agent loop, never runs outside the
capability's `build_state`/`render_template` code path.

**Generated-template mode is NOT required.** Every capability may ship
with static templates only (§4.3a) and still satisfy this design.
Generated mode is reserved for capabilities where visual variety is a
product requirement (e.g. news summary cards that need to adapt to
arbitrary story types).

---

## 10. Non-goals

Explicitly out of scope for this architecture:

- **Arbitrary LLM-authored Rust code.** Never. Rust is host + library;
  capabilities are Rust but declarative in spirit (no UI logic).
- **Unrestricted `script_mod!` loading from untrusted sources.** Widget
  packages are a future feature; when they arrive, they are *installed*
  (compile-time or plugin), not loaded at runtime from LLM output.
- **HTML / WebView fallback for rich rendering.** Splash + Makepad Live
  is the only rich render path. Non-capable clients get the
  `body` plain-text fallback, not an HTML render.
- **A2UI-style adjacency-list component graphs as the primary render
  format.** We may borrow A2UI control mechanisms (catalog-style
  whitelists, validation loops, local-vs-remote action split,
  path-scoped data updates), but Splash templates remain the primary
  view representation.
- **Dynamic capability discovery at runtime.** v1 and v2 capabilities
  are compiled into the binary. Plugin-style capability loading is
  deferred.
- **Multi-capability composition in one reply.** A resolver decision
  picks exactly one capability. "Weather + calendar in one card" is a
  new composite capability, not a runtime join.
- **Client-side template authoring tools.** Templates are authored in
  source (or at runtime by LLM within bounds). No in-app template
  editor.

---

## 11. Spec / doc lineage

This design doc is the root. Work under it is organized as follows:

```
agent-to-app-design.md  (this file — living)
  │
  ├── specs/task-agent-to-app-system.spec.md
  │     └─ Master contract: envelope protocol, layer contracts,
  │        lifecycle, security envelope. Invariants for ALL capabilities.
  │
  ├── specs/task-agent-to-app-producer-routing.spec.md
  │     └─ Producer half: resolver + dispatcher + capability trait +
  │        weather_guidance as first reference impl. SHIPPED.
  │
  ├── specs/task-agent-to-app-l1-weather-card.spec.md
  │     └─ Consumer half for weather: L1 static card, v2 schema
  │        addendum. SHIPPED.
  │
  ├── specs/task-agent-to-app-l1-weather-v2-doc-sync.spec.md
  │     └─ v2 doc-sync. SHIPPED.
  │
  ├── specs/task-agent-to-app-splash-host-evolution.spec.md
  │     └─ Template extraction, SplashHost trait, widget whitelist,
  │        news_guidance as first "zero-Rust-UI" capability.
  │
  ├── specs/task-agent-to-app-template-runtime.spec.md
  │     └─ Splash Host runtime contract: preflight, cache,
  │        compatibility, fallback hierarchy, error shape.
  │
  ├── specs/task-agent-to-app-l2-actions.spec.md  (TBD)
  │     └─ Interactive capabilities with buttons/forms.
  │
  └── specs/task-agent-to-app-l3-stateful.spec.md  (TBD)
        └─ Stateful mini-apps with tick, host runtime extensions.
```

The L2/L3/packaging specs remain the next writing work. The
splash-host-evolution and template-runtime specs consume this design
doc's §4–§6 as their binding contract.

---

## 12. Open questions

Listed here as a ledger — each becomes a spec decision or a research
ticket.

- **Template binding syntax** — do we extend Splash with `$state.foo`
  path resolution, or wrap it in a binding layer that pre-substitutes?
  Splash skill and Makepad DSL syntax need to be consulted.
- **Widget ABI versioning across app releases** — when a project
  widget's prop schema changes, how do older `org.octos.app` payloads
  still render? Pin template by `(widget_name, widget_version)` in the
  manifest?
- **Template hot-reload vs production bundling** — in dev, templates
  should reload on save. In production, should they be `include_str!`
  at compile time or read from disk? Both?
- **LLM-generated template cache invalidation** — if LLM generates a
  template, do we cache by `(capability, state_hash, focus)` for
  latency? What keys the cache?
- **Cross-language Splash for zh-CN / EN / others** — embed language
  branches in one template or maintain parallel templates? Performance
  and maintenance trade-offs.
- **W7 local function surface area** — which local primitives belong in
  the permanent whitelist, and which should stay out of templates?
- **Incremental state update contract beyond v1** — v1 has host-side
  `replace` / `remove`; L2/L3 still need `append`, `splice`, array
  inserts, and replay semantics mapped onto Matrix event history.

These are **not blockers**. Each is resolved as its corresponding spec
is written.

---

## 13. How this document evolves

This doc updates when the architecture itself changes — layer
boundaries, whitelist semantics, the four-part contract. **Phase
progress and work-item status do not belong here**; those go in
`docs/roadmap/`. Scenario-level testable contracts belong in `specs/`.

When a new spec (e.g. `splash-host-evolution`) lands a change that
affects this doc, update the relevant section and cite the spec by
filename. Keep §11 in sync.

If a contributor wants to propose a change that contradicts §3 (the
principle), §4 (the four-part contract), or §6 (the whitelists), they
write a new section here and bring it to review **before** writing a
spec or code. Those three sections are load-bearing; changing them
without review breaks the other layers.
