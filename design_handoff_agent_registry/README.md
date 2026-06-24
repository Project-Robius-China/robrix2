# Handoff: Robrix Agent Registry (mobile)

## Overview
A mobile screen for Robrix that lets a user register AI agents to Matrix accounts. Three
agent frameworks are supported:

- **Hermes** — direct agent, registered by adding it as a Matrix friend.
- **OpenClaw** — direct agent, registered by adding it as a Matrix friend.
- **Octos** — runs as a Matrix **AppService**. Registered the same way (friend request)
  **plus** a local AppService binding that must be reachable before registration completes.

The core interaction is an **"Add an agent" bottom sheet** with two steps: (1) choose the
framework, (2) enter the agent's Matrix ID and add it. For Octos, step 2 additionally
exposes the AppService binding controls (BotFather ID, local service URL, a "Check now"
health probe, and "Open local binding").

## About the Design Files
The file in this bundle (`Agent Registry.html`) is a **design reference created in HTML** —
a working prototype that shows the intended look and behavior. It is **not production code
to copy directly**. The task is to **recreate this design in Robrix's existing codebase**
using its established framework, component library, state patterns, and Matrix client APIs.
If no UI environment exists yet, pick the most appropriate framework for the project and
implement there. The prototype uses React + inline styles purely so it runs standalone in a
browser; do not treat that as the prescribed stack.

## Fidelity
**High-fidelity.** Final colors, typography, spacing, radii, and interaction states are all
intended values — recreate the UI to match, using the codebase's existing primitives where
they exist (buttons, inputs, sheets, badges).

---

## Screens / Views

### 1. Agent Access (main screen)
**Purpose:** Overview of registered agents and entry point to add a new one.

**Layout:** Single vertical column, full-width, scrollable. Page padding 20px horizontal.
Fixed bottom navigation bar. Order top→bottom:
1. Back link `‹ Agents` (teal, 14px/600).
2. Title `Agent Access` (27px/800, navy, letter-spacing −.02em).
3. Subtitle paragraph (13.5px/1.5, muted).
4. Primary **"+ Add an agent"** button (full width, teal fill, white text, radius 13px,
   padding 14px, soft teal shadow).
5. **Octos AppService** summary card (light-blue bg `--blue-bg`, radius 14px, padding
   14×16). Header row: title (blue, 14px/700) + health pill `N/N online` with a green dot.
   Body: explanatory text (12.5px/1.5).
6. **Registered agents** section. Header row: `Registered agents` (16px/800 navy) on the
   left, `AgentRegistry` (12px/700 green) on the right. Below: a vertical list (gap 10px) of
   agent rows; empty-state is a dashed-border box if none.

**Agent row component** (card: white, 1px border `--border`, radius 14px, padding 12×13):
- Left: 44×44 rounded-square **framework tile** (radius 12px) — soft framework-bg fill,
  framework-color 2-letter monogram (`He`, `Cl`, `Oc`), 15px/700.
- Middle: agent display name (15px/700, ellipsis) with — for Octos only — a health dot
  beside it; second line is the Matrix ID (12.5px muted, ellipsis).
- Right: **framework badge** (pill: framework-bg fill, framework-color text, 11.5px/600, a
  6px color dot + name).
- Footer (separated by 1px top line, margin-top 11px): equal-width action buttons —
  `Open chat` (neutral), `Re-check` (teal, **Octos only**, shows "Checking…" while probing),
  `Unbind` (red). Each: radius 9px, padding 7px, 12.5px/600.

### 2. Add-agent bottom sheet
A modal bottom sheet over a 42%-opacity scrim. Sheet: white, top corners radius 22px,
shadow `0 -12px 40px -12px rgba(16,24,40,.4)`, max-height 88%, slides up with
`transform: translateY(102% → 0)` over **.34s cubic-bezier(.22,1,.36,1)**; scrim fades over
.28s. Structure: grip handle → header → scrollable body → sticky footer.

**Header:** back chevron `‹` (step 2 only) · title · close `×`. Title is `Add an agent`
(step 1) or `Connect <Framework>` (step 2), 18px/800 navy. Subtitle line below (12.5px
muted): `Step 1 of 2 · Choose a framework`, or `Step 2 of 2 · Find the Matrix friend`
(Hermes/OpenClaw) / `Step 2 of 2 · Friend + AppService binding` (Octos).

**Footer (sticky, 1px top line):** full-width primary button.
- Step 1: `Continue` — disabled until a framework is selected.
- Step 2: `Finish & register` — disabled until requirements met; disabled label is
  `Add the agent above to continue` (no friend yet) or `Service must be online` (Octos,
  service not online).

#### Step 1 — framework picker
Intro line (13.5px muted), then three selectable cards (vertical, gap 11px). Each card
(white, 1.5px border, radius 15px, padding 13×14):
- 50×50 framework tile (radius 14px, 17px monogram).
- Name (16px/700) + an uppercase tag pill: `DIRECT AGENT` (Hermes/OpenClaw) or `APPSERVICE`
  (Octos) in framework color on framework-bg.
- One-line blurb (12.5px muted).
- Right: a 21px radio circle; when selected → filled framework color with a white ✓, card
  border becomes framework color with a 3px framework-bg ring.

#### Step 2 — add the agent (simplified)
- Section heading: small 22px tile + `New <Framework> agent`.
- Field label `Agent Matrix ID` (12.5px/600 muted).
- Text input with a leading `@` adornment; placeholder `agent:server`; bg `--bg`, radius
  12px, 1px border. Input becomes **disabled (greyed)** once a friend request is sent.
- Helper text: "Robrix sends a friend request to this account and records that it runs on
  `<Framework>`."
- **Add friend & bind** button (full width, outline in framework color, framework-color
  text): disabled until the field is non-empty.
  - On click → **pending**: outline button shows a spinner + "Sending friend request…"
    (~1.3s).
  - On success → **added**: replaced by a green confirmation strip (bg `#eaf7ef`, border
    `#c6e9d3`, green ✓ badge) reading "Friend request sent — `@id:server` bound to
    `<Framework>`."

#### Step 2 — Octos AppService section (Octos only)
Rendered below the add-friend control, separated by a 1px top line. **Dimmed to 0.45 opacity
and non-interactive until the friend has been added**, then fades to full opacity (.25s).
- Heading `AppService binding` (14px/700 navy) + explanatory paragraph.
- `BotFather user ID` field — default value `octosbot`.
- `Local Octos service` field — default value `http://127.0.0.1:8010`.
- Row: **Check now** button (octos-bg fill, octos text) + a **status pill**.
  - Status states: `Unknown` (faint/grey), `Checking` (amber, shows spinner on the button),
    `Online` (green), `Offline` (red). Probe takes ~1.2s in the prototype.
  - **Offline** also shows a red error note: "No response from `<url>`. Start the local Octos
    service, then re-check."
- **Open local binding** button (full width, outline) — enabled only when status is
  `Online`. (In the real app: opens/focuses the locally bound Octos service.)

### 3. Bottom navigation
Fixed bar, top 1px border, bg `#f7f8fa`, ~4 evenly spaced items: Home (active = teal),
center **+** (also opens the add-agent sheet), grid/spaces, and an avatar with a red `!`
notification badge. Icons are simple stroked SVGs.

### 4. Toast
On successful registration, a dark navy pill toast slides up near the bottom (above the nav)
with a green ✓ and `<Name> registered`, auto-dismissing after ~2.6s.

---

## Interactions & Behavior
- **Open add sheet:** tap "Add an agent" (main) or the **+** in bottom nav → sheet slides up.
- **Close:** tap scrim or `×` → slides down; state resets on next open.
- **Step nav:** select framework → `Continue` enables → step 2. Back chevron returns to
  step 1. Selecting a framework auto-focuses the ID input on entering step 2 (~320ms).
- **Add friend:** idle → pending (spinner, ~1.3s) → added (green confirmation). Input locks
  after sending.
- **Octos gating:** AppService section interactive only after friend added. "Finish &
  register" requires `friend === added` AND (for Octos) `health === online`.
- **Service check:** unknown → checking (spinner, ~1.2s) → online | offline. Offline shows
  the error note and keeps Finish disabled.
- **Re-check (main screen, Octos rows):** sets that row's health to checking (~1.1s) →
  online/offline.
- **Finish & register:** prepends the new agent to the registered list, closes the sheet,
  fires the success toast.

> The timed transitions (`setTimeout`) and randomized online/offline outcomes are prototype
> stand-ins. In Robrix, wire these to the real Matrix friend-request flow and the Octos
> AppService health endpoint; reflect actual async results instead of timers.

## State Management
**App level**
- `agents: Agent[]` — registered agents. Seeded with two examples.
- `sheetOpen: boolean` — add-sheet visibility.
- `toast: string | null` — transient success message.

**Add-sheet level (reset each time the sheet opens)**
- `step: 1 | 2`
- `type: 'hermes' | 'openclaw' | 'octos' | null`
- `query: string` — the typed Matrix ID.
- `friend: 'idle' | 'pending' | 'added'`
- Octos: `botfather: string`, `url: string`, `health: 'unknown' | 'checking' | 'online' | 'offline'`

**Agent shape**
```
{ id, type, name, uid, status:'added',
  // Octos only:
  health, url, botfather }
```
Display name is derived from the entered ID (segment before `:`, leading `@` stripped,
capitalized); `@` is auto-prefixed to the stored uid if omitted.

**Data fetching (real app)**
- Matrix: send friend request / invite to the entered user ID; track its acceptance state.
- Octos: GET the local service URL (e.g. `http://127.0.0.1:8010`) for a health/status
  response to drive the status pill.

---

## Design Tokens

**Colors**
| Token | Hex | Use |
|---|---|---|
| `--teal` | `#0f95a3` | primary actions |
| `--teal-dark` | `#0b7d89` | — |
| `--teal-soft` | `#e4f4f5` | re-check button bg |
| `--navy` | `#16233f` | headings, toast |
| `--ink` | `#1f2b45` | body text |
| `--muted` | `#6c7787` | secondary text |
| `--faint` | `#9aa3b0` | placeholders, disabled |
| `--border` | `#e4e8ed` | card/input borders |
| `--line` | `#eef1f4` | dividers |
| `--bg` | `#f4f6f8` | input/field fills |
| `--card` | `#ffffff` | cards |
| `--blue` | `#2f6db3` | AppService summary title |
| `--blue-bg` | `#eaf2fb` | AppService summary card |
| `--green` | `#1f9d57` | online/success |
| `--amber` | `#c47d1e` | checking |
| `--red` | `#d94c4c` | unbind/offline/badge |
| Hermes | `#c47d1e` / bg `#fbf1e3` | framework accent |
| OpenClaw | `#6a52c4` / bg `#eee9fb` | framework accent |
| Octos | `#1488b5` / bg `#e6f2f9` | framework accent |
| Success strip | text `#1c6e42`, bg `#eaf7ef`, border `#c6e9d3` | friend-added |
| Error note | text `#9a3b3b`, bg `#fbecec`, border `#f3d2d2` | offline |

**Typography:** system stack
(`-apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif`).
Scale used: 27/800 (page title), 18/800 (sheet title), 16/800 & 16/700 (section/card name),
15.5/700 (primary button), 15/700, 14.5/14/13.5 (body & controls), 12.5/12 (meta), 11.5/11
(badges/pills). Title letter-spacing −.02em.

**Spacing:** page padding 20px; sheet padding 18px; card padding ~12–14px; list gaps 10–11px.

**Radius:** tiles 12–14px · buttons/inputs 11–13px · cards 14–15px · sheet top 22px ·
pills/badges 20px · circles full.

**Shadows:** primary button `0 6px 16px -7px rgba(15,149,163,.6)` · sheet
`0 -12px 40px -12px rgba(16,24,40,.4)` · phone frame `0 30px 80px -24px rgba(16,24,40,.5)` ·
toast `0 8px 24px -8px rgba(0,0,0,.5)`.

**Motion:** sheet slide `.34s cubic-bezier(.22,1,.36,1)` · scrim fade `.28s` ·
opacity/section reveals `.2–.25s` · spinner `spin .7s linear infinite`.

## Assets
No raster assets. Framework "icons" are 2-letter monograms in colored rounded squares —
substitute real framework logos in the codebase if available. Nav icons are inline stroked
SVGs (home, +, 2×2 grid) — use the codebase's existing icon set. Avatars are colored
initials circles.

## Files
- `Agent Registry.html` — the full standalone prototype (React + inline styles). All data
  lives in `TYPES` and `INITIAL_AGENTS` near the top; component structure: `MainScreen`,
  `AddSheet` → `Step1` / `Step2`, `BottomNav`, plus `Tile`, `FrameworkBadge`, `HealthDot`,
  `StatusPill`, `Spinner`, `RowAction`, `Field`.

## Screenshots
Reference captures of each key state are in `screenshots/`:
- `01-main-agent-access.png` — main screen (registered agents + AppService summary).
- `02-step1-framework-picker.png` — sheet step 1, framework selection.
- `03-step2-enter-id.png` — step 2, Matrix ID entered (Hermes), before add.
- `04-step2-friend-added.png` — step 2, friend-request-sent confirmation (Hermes).
- `05-octos-appservice-dimmed.png` — Octos step 2 before friend added (AppService section dimmed).
- `06-octos-appservice-online.png` — Octos step 2 after friend added + service Online; Finish enabled.
