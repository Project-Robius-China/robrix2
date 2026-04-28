# TG Bot Phase 6 — UI/UX Roadmap

## Context

The TG-alignment core interaction model is now in place:

- Phase 3: mention/reply-first routing
- Phase 4a: BotFather menu button
- Phase 4b: `/command@bot`
- Phase 4c: action buttons / approval buttons
- Phase 5a: bot-aware command discovery v1
- Phase 5b: `/allbots`
- Phase 5c: `/schedule` / `/schedules` / `/unschedule`

At this point, the product has moved from "can talk to bots" to "can orchestrate bots".
The next roadmap should focus on UX quality, capability clarity, and better structured results,
not on piling on many new commands.

## Goal

Make bot orchestration in Robrix feel intentional and legible:

- users should understand which commands are valid for which bot
- approval and action flows should communicate state clearly
- orchestration results should look like structured bot outcomes, not plain chat dumps

## Non-Goals

Phase 6 does **not** aim to:

- redesign the routing model again
- replace command-based orchestration with a graphical workflow builder
- add free-form bot-to-bot autonomous conversation
- add aggregated execution engines or cron dashboards

## Recommended Direction

### 1. Dynamic Bot Capability Discovery

Current state:

- slash discovery is context-aware, but still backed by static catalogs
- child-bot rooms avoid obviously wrong commands, but do not expose true bot-specific capability data

Next step:

- let Octos expose bot capability / command catalogs
- let Robrix render slash/menu suggestions from live capability data
- distinguish:
  - BotFather management commands
  - orchestration commands
  - child-bot local commands
  - commands that require approval or additional parameters

Why first:

- this directly improves the current biggest UX gap:
  routing is correct, but command discovery still does not fully explain what is valid for the current target bot

### 2. Approval and Orchestration Status Cards

Current state:

- approval and action buttons work
- button click state is reflected locally
- failure recovers correctly

Next step:

- show `expires_at`
- show who can approve
- show final status:
  - `Approved`
  - `Denied`
  - `Expired`
- show whether an action is pending, completed, or failed

Why second:

- the protocol already exists; this is mostly UX clarity and state communication

### 3. Structured Result Cards

Current state:

- bot messages already have card rendering
- approval/action messages can attach button rows
- schedule and `/allbots` responses are still mostly plain text

Next step:

- make schedule creation / list / delete results more structured
- make `/allbots` broadcast results more explicit
- improve file/task completion messages so they read as outcomes, not generic replies

Examples:

- "`/allbots` sent to 3 bots"
- "Schedule created"
- "Schedule expired"
- "Approval denied by Alice"

Why third:

- this is high-value polish, but should follow command/capability clarity

### 4. Orchestration V2

Only after the above is stable should we consider:

- scheduled broadcast combinations
- richer multi-bot coordination summaries
- aggregated result presentation

This is intentionally **not** part of the immediate next step.

## Execution Order

### Phase 6a

Dynamic bot capability discovery:

- define Octos capability contract
- consume it in Robrix slash/menu UI
- filter commands by active bot context

### Phase 6b

Approval / orchestration state polish:

- status rendering
- approver visibility
- expiry visibility
- action lifecycle display

### Phase 6c

Structured result cards:

- `/allbots` result cards
- schedule result cards
- richer action outcome rendering

## Success Criteria

This roadmap is successful when:

- users no longer need to guess which slash commands are valid in the current room
- approval requests communicate state before and after action clearly
- orchestration commands return structured outcomes instead of ambiguous plain text

## Suggested Next Spec

The next concrete implementation spec should be:

- `Dynamic Bot Capability Discovery`

That is the highest-leverage follow-up to the current Phase 5 work and the best foundation
for later UI/UX polish.
