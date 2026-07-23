# Agent Message Response Cards — Design

Date: 2026-07-10
Status: Draft for review
Scope: Robrix2 room-timeline **bot/agent** message rendering only
Related: [2026-04-12-bot-timeline-card-design.md](2026-04-12-bot-timeline-card-design.md), visual spec `docs/ui-visual-spec-zh.md` §4.5 / §4.6 / §4.7 / §4.10 / §5.3 / §5.4 / §7

## Goal

Design four new response-message UI styles for Octos/agent replies in the room
timeline, and land them without touching routing, the input bar, or the Octos
backend protocol:

1. **Streaming / generating indicator** — replace the trailing `●` glyph with a
   real animated affordance.
2. **AgentMessageCard** (§4.5) — the signature agent answer surface: identity +
   step chips + analysis card + quiet footer.
3. **ApprovalCard** (§4.6) — redesign the current light-blue approval view into
   the amber, `Pending`-badged signature component.
4. **CodeOutputCard** (§4.7) — dark syntax-highlighted panel for fenced code in
   bot replies.

## Current State (ground truth)

- Dispatch: `populate_message_view` (`src/home/room_screen.rs:11149`). Bot/agent
  output has **no separate template** — it rides inside `Message` /
  `CondensedMessage` and is toggled by `populate_bot_text_message_content`
  (`:12008`).
- Two orthogonal richness channels:
  1. **Text-shape heuristic 3-layer card** (status strip / body card / metadata
     footer) parsed from plaintext by `parse_bot_timeline_layers` (`:796`).
     No schema — the bot must emit exact line shapes (`施法中` /
     `via X (model)` / `_… · N in · N out · Zs_`).
  2. **Structured `org.octos.*` JSON**: `splash_card`, `actions` (buttons ≤6),
     `approval_request`. Action buttons + approval request/response are
     implemented (native Makepad widgets).
- Streaming: typewriter reveal (`StreamingAnimState`, `streaming_animation.rs`)
  + a trailing `●` (U+25CF) appended client-side (`:167`).
- Debt: bot card hardcodes `COLOR_BOT_*` (`room_screen.rs:1598`) bypassing the
  `RBX_*` token layer; bot badge uses deprecated `RBX_LEGACY_BLUE` (#0F88FE);
  the bot-card DSL block is **duplicated verbatim** in `Message` (`:2071`) and
  `CondensedMessage` (`:2260`).
- Radius: trust code — `RBX_RADIUS_MD = RBX_RADIUS_SM = 6` (not the stale spec
  §3.2 prose that says 8).

## Design Tokens (authoritative values)

| Token | Value | Used by |
|-------|-------|---------|
| `RBX_BG_SURFACE` / `RBX_STROKE_SOFT` | #FFFFFF / #E6EBF2 | card fill / border |
| `RBX_ACCENT` / `RBX_ACCENT_SOFT` | #119FB3 / #E4F5F7 | APP badge, active step, analysis left-edge |
| `RBX_WARNING_FG` / `_BG` | #C6790B / #FBF1DD | ApprovalCard amber container / Pending badge |
| `RBX_SUCCESS_FG` / `_BG` | #1B8A4B / #E8F6EE | done step, Approve, online dot |
| `RBX_DANGER_FG` / `_BG` | #C5392F / #FBE9E7 | failed step, Reject, Critical badge |
| `RBX_NEUTRAL_FG` / `_BG` | #5A6B86 / #EEF1F6 | expired / idle |
| `RBX_FG_PRIMARY/SECONDARY/TERTIARY` | #16233B / #5A6B86 / #8A98AE | body / meta / footer |
| `RBX_CODE_BG/FG/KEYWORD/STRING/COMMENT` | #1B2433 / #D7DEE8 / #7CC4FF / #8FD19A / #7F8B9B | CodeOutputCard |
| `RBX_RADIUS_SM` / `_MD` / `_PILL` | 6 / 6 / 100 | inner block / card / chip |
| `RBX_AVATAR_MD` | 40 | agent avatar |
| `RBX_TEXT_CARD_TITLE/BODY/META/BADGE` | 12b / 11 / 9.5 / 9b | title / body / meta / badge |

New dark code-syntax tokens added this cycle (DSL-only, `design_tokens.rs` §8):
`RBX_CODE_BORDER` #2C3A4E, `RBX_CODE_NUMBER` #E5C07B, `RBX_CODE_FUNCTION`
#61AFEF, `RBX_CODE_TYPE` #56B6C2, `RBX_CODE_ERROR` #E06C75, `RBX_CODE_WARNING`
#E5C07B, `RBX_CODE_PUNCT` #ABB2BF.

## Shared Foundation (step 0)

1. **Token migration** `COLOR_BOT_* → RBX_*` (spec §0.1: reference tokens, don't
   hardcode). Done per-surface as each card is built, to avoid a risky
   mid-file big-bang: the code colors migrate with ④; status/body/footer colors
   migrate with ③ / ②.
2. **`agent_render_state` coordinator** (§5.4): one classifier decides which card
   a message renders as, so visibility flags never fight. Scaffolded now
   (`AgentCardKind` { `PlainMessage`, `BotTextCard`, `AgentCard` } +
   `compute_agent_render_state`), behavior-preserving; the `AgentCard` branch is
   wired in ②.
3. **De-duplicate** the bot-card DSL across `Message` / `CondensedMessage` — new
   card fragments must stay in sync across both.
4. Dynamic widgets (created via `widget_ref_from_live_ptr`) → **Animator + shader
   instance vars**, never `script_apply_eval!` (Pitfall #40).

## Card Contracts

### ① Streaming / generating indicator — IMPLEMENTED

- Anatomy: an accent-soft pill (`bot_streaming_indicator`, first child of
  `bot_message_card`) holding a small teal spinner. Body still reveals via
  typewriter (plain) or full snapshot (markdown).
- Decision (revised from the mock's bouncing dots): reuse the makepad built-in
  self-animating `LoadingSpinner` (teal, 16px) instead of a custom dots shader —
  it animates off the shader's `draw_pass.time` with no per-tick instance-var
  plumbing (which would hit the dynamic-widget `script_apply_eval` limitation).
  Tokens: `RBX_ACCENT` spinner on `RBX_ACCENT_SOFT` pill, `RBX_RADIUS_PILL`.
- Visibility: `populate_bot_text_message_content` resets it hidden; the streaming
  render path sets it visible = `state.is_live`; hidden again on completion.
- Animation in both modes: the streaming frame loop now keeps scheduling frames +
  re-drawing (not re-populating) the item while any stream `is_live`, so the
  spinner spins smoothly even in full-snapshot markdown mode where the typewriter
  is not ticking (bounded by the 5-min live-stall timeout).
- Removed the trailing `●` cursor from `fill_display_buffer` (the spinner now
  signals "generating").
- Follow-up (not v1): optional localized "Generating… / 生成中" label beside the
  spinner (needs an i18n key); a bouncing-dots variant if preferred.

### ② AgentMessageCard (§4.5) — v1 IMPLEMENTED (folded into the bot card)

- Decision: rather than a separate `agent_message_card` gated on a signal Octos
  can't emit (it would never show / be untestable), v1 upgrades the existing bot
  text card in place into the agent surface — every bot reply reads as an agent
  card. (The `agent_render_state` scaffold stays for a future hard agent/bot split
  once a structured signal exists.)
- Delivered (all `RBX_*` tokens; both `Message` + `CondensedMessage`):
  - **StepChip (active)**: the parsed agent status pill is restyled to the §4.10
    active StepChip look — `RBX_ACCENT_SOFT` bg + `RBX_ACCENT` text +
    `RBX_RADIUS_PILL`. (Shows when Octos sends a status line; single chip only.)
  - **Answer card**: `bot_body_card` migrated to `RBX_BG_SURFACE` +
    `RBX_STROKE_SOFT`, corners tightened to `RBX_RADIUS_XS` (4). The 4px accent
    left spine was tried (outer-accent + inner-inset technique after a
    `Fill`-in-`Fit` bar failed) but **removed per user feedback** — the plain
    framed card reads cleaner; the accent lives on the `Bot` badge + StepChip.
  - **APP badge**: identity `bot_badge` migrated off deprecated legacy blue →
    `RBX_ACCENT`, text `bot` → `APP`.
  - **Footer** provider/usage text → `RBX_FG_SECONDARY` / `RBX_FG_TERTIARY`.
- **Data honesty**: Octos emits no structured steps, so the StepChip is a single
  active chip from the parsed status; a real multi-step chain (to-do/done/failed)
  needs backend `org.octos.steps` (see Backend Asks).
- **Deferred to ②.2**: the 6×6 `RBX_SUCCESS` online dot on the avatar (touches the
  profile column); a multi-chip row once backend steps exist.
- **Verify at runtime**: the 4px accent edge uses `height: Fill` inside a `Fit`
  row — confirm the bar renders full-height in the running app.

### ③ ApprovalCard (§4.6)

- **Decision (revised during impl):** re-skin the existing `approval_request_view`
  **recipe in place** (§4.1) rather than build a new `src/shared/approval_card.rs`.
  Rationale: this fork can't reliably append children to derived templates, so a
  self-contained new widget adds risk with no payoff — the Approve/Reject buttons
  already live in the sibling `action_button_row`, so the "card" is only the
  header/summary/meta. Editing the template source (not appending at a use site)
  is safe. The recipe is duplicated in `Message` and `CondensedMessage`; both
  edited together.
- Anatomy: amber container (`RBX_WARNING_BG` + 1px `RBX_WARNING_FG` +
  `RBX_RADIUS_SM`) → `[title RBX_TEXT_CARD_TITLE warning fg][Critical? danger
  badge][Pending badge]` → summary (`RBX_TEXT_BODY`) → tool / expiry meta
  (`RBX_TEXT_META`) → `[Approve success][Reject danger]` (from `action_button_row`).
- Data: `org.octos.approval_request` already parsed (request_id, tool_name,
  title, summary, risk_level, expires_at, authorized_approvers).
  `ApprovalCardRenderState` extended with `risk_critical` + `meta`; the Critical
  badge shows for `risk_level == critical`; the meta line shows tool + expiry, or
  "Only X can approve" when the local user is not authorized (buttons already
  disabled via `local_user_can_approve`). Reuse `MessageActionPrimary/DangerButton`.
- Follow-up (not in v1): approved / rejected / expired badge transitions need the
  clicked-decision / expiry state plumbed into the badge (today the buttons
  collapse to `✓ {label}` but the badge stays "Pending").

### ④ CodeOutputCard (§4.7)

- Anatomy: dark panel `RBX_CODE_BG` + `RBX_RADIUS_SM` + dark syntax token colors;
  optional footer "↺ translated · show original" (`RBX_LINK`) below a 1px
  `RBX_DIVIDER`.
- Change: `BotTimelineMarkdown` `code_block` (`room_screen.rs:1738`) `draw_bg`
  → `RBX_CODE_BG`/`RBX_CODE_BORDER`; `CodeView` `draw_text` color → `RBX_CODE_FG`;
  `token_colors` remapped GitHub-light → dark.
- Scope note: only the **fenced-block widget** (highlighted path) goes dark.
  Inline `` `code` `` and the CJK-in-fence *plain* fallback
  (`use_code_block_widget:false`, `draw_block.code_color`) stay light for v1 —
  darkening them needs a light fixed-text color that the shared Markdown body
  color can't provide per-span. Documented, revisit if needed.
- Translation footer deferred to the realtime-translation feature
  (`specs/task-realtime-translation.spec.md`); omitted when not translated.

## Build Order

0. Token/code additions + `agent_render_state` scaffold (this cycle).
1. **④ CodeOutputCard** — most self-contained, lowest risk (this cycle).
2. **③ ApprovalCard** — new shared component, data already present.
3. **① Streaming indicator** — Animator-driven, independent.
4. **② AgentMessageCard** — largest; needs `agent_render_state`; ship single-step
   v1 first, multi-step after the backend field lands.

Keep visual and logic changes in separate commits.

## Backend Asks (front-end vs needs Octos)

Front-end can ship today: ④ fully, ③ fully, ① fully, ② single-step.
Needs Octos changes (out of this repo's scope, tracked for later):
- `org.octos.steps` structured progress → real multi-step StepChip chains.
- Migrating the text-shape heuristic layers to structured `org.octos.*` fields.
- Approval-lifecycle notices (timeout / duplicate / unauthorized) as typed
  events instead of plain `m.notice`.

## Out of Scope

- Input bar / composer redesign, mention/slash routing, `bot_menu_button`.
- Non-bot user message restyling.
- Octos backend protocol/output changes (only *consumed*, not changed).
- Poll cards, location map thumbnails, image download progress (separate gaps).

## Validation Plan

- ④: a bot reply containing a fenced code block renders on the dark panel with
  readable syntax colors; inline code and CJK-fence fallback still readable.
- ③: an `org.octos.approval_request` renders amber with a `Pending` badge;
  Approve/Reject send the existing responses; unauthorized users see disabled
  buttons.
- ①: streaming replies show animated dots, no stray `●`; done state hides them.
- ②: agent replies show identity + step chip + analysis card; ordinary user and
  plain bot messages are unaffected.
- Cross-cutting: `cargo build` green; condensed grouping + reply previews still
  align; light-theme surfaces only (no white-flash, spec §7).
