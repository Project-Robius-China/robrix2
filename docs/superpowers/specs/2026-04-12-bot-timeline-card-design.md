# Bot Timeline Card Design

Date: 2026-04-12
Status: Draft for review
Scope: Robrix2 bot conversation timeline styling only

## Goal

Make bot replies in the room timeline feel closer to Telegram bot chats without changing the current mention-first / reply-first interaction model.

This design only covers the visual treatment of bot messages already rendered in the timeline. It does not change routing, slash commands, reply semantics, or bot action menus.

## Context

Current bot replies in Robrix have the correct functional behavior, but the reading experience is still flat:

- bot replies look too similar to ordinary Matrix text messages
- bot status text, provider text, and token/latency stats compete with the actual reply
- the username row does too much work to communicate bot identity
- there is no strong visual center for the bot response itself

The Telegram references the user supplied and the official bot features page point to a different hierarchy:

- the reply itself is the focal surface
- secondary metadata is visually quiet
- bot-specific affordances live with the message, not as a permanent input-state control

Reference:
- Telegram Bot Features: <https://core.telegram.org/bots/features>

## Design Direction

Adopt a Telegram-style card treatment for bot replies inside the existing Robrix timeline.

The timeline should read in this order:

1. sender identity
2. bot reply card
3. quiet metadata footer

The key shift is that the bot response body becomes a clear card surface, while generation status and model/provider details become secondary layers.

## Chosen Approach

Use a **carded bot reply layout** inside the existing `Message` timeline widget.

Why this approach:

- it improves readability without reopening the routing model
- it fits Makepad's existing `room_screen.rs` ownership model
- it stays local to timeline rendering and avoids input-bar churn
- it matches the Telegram references more closely than a light typography-only cleanup

## Message Anatomy

### 1. Identity Row

Keep the current username row, including the `bot` badge, but reduce its visual weight relative to the reply card.

Rules:

- username remains on top of the message content
- `bot` badge stays compact and close to the username
- identity row should not become the primary visual anchor

### 2. Reply Card

The actual bot response body becomes the main card.

Rules:

- use a soft rounded background behind the bot reply body
- keep padding noticeably larger than ordinary text messages
- preserve Markdown / HTML rendering through `HtmlOrPlaintext`
- avoid making the card look like a desktop form panel or debug container

Visual intent:

- lighter than Telegram's mobile bubbles, but clearly a separate response surface
- warmer and more intentional than the current plain timeline flow

### 3. Status Strip

Streaming or generation status should not appear as if it were the first line of the assistant's reply.

Rules:

- move transient status text such as "thinking", "generating", or tool phase summaries into a small status strip above the main reply body
- status strip should read as operational context, not content
- status strip can share the card family but should be visually lighter than the main body

### 4. Metadata Footer

Provider, model, token usage, duration, and related diagnostic text become a subdued footer.

Rules:

- provider/model line should sit below the body, not above it
- token/latency stats should be the weakest text layer in the message
- footer should remain selectable/readable, but never compete with the response

## Visual Principles

### Hierarchy

- Primary: bot reply body
- Secondary: sender name and reply preview
- Tertiary: provider/model line
- Quaternary: token/latency stats and edited markers

### Shape

- rounded corners should be consistent across the bot card family
- avoid mixing one radius for the card and another unrelated radius for adjacent bot-specific surfaces

### Spacing

- bot replies should breathe more than plain user text
- footer spacing must be tighter than body spacing
- reply preview to bot card spacing should feel intentional rather than inherited from generic message layout

### Color

- use restrained surfaces, not loud brand blocks
- bot responses should feel distinct without reading like warnings, notices, or selected rows
- keep contrast strong enough for long-form reading

## Component Boundaries

### Keep As-Is

- message routing and target selection
- input bar behavior
- mention parsing
- `HtmlOrPlaintext` content rendering model

### Change

- bot-specific visual treatment in `src/home/room_screen.rs`
- bot message sub-structure around message body and metadata
- optional small supporting style hooks in shared widgets only if required by the card layout

## File Impact

Primary:

- `src/home/room_screen.rs`

Possible supporting changes if needed:

- `src/shared/html_or_plaintext.rs`
- `src/home/edited_indicator.rs`

No planned changes:

- `src/room/room_input_bar.rs`
- Matrix routing code
- Octos backend

## Out of Scope

- inline keyboard / action button UI
- menu button near the input field
- slash command redesign
- bot profile pages
- non-bot message restyling across the entire timeline
- protocol or backend metadata changes

## Implementation Notes

- Prefer adapting existing `Message` / `CondensedMessage` structure rather than inventing a second message system.
- Bot card ownership should remain in `room_screen.rs`, which already decides sender identity, badge visibility, and message-body population.
- If dynamic styling is needed for bot-only surfaces, follow the project's Makepad rule: avoid relying on `script_apply_eval!` for dynamic widgets created from live pointers.

## Validation Plan

Manual validation should check:

- bot replies are distinguishable at a glance from user replies
- the main answer is visually stronger than provider/model/stats
- long bot responses remain comfortable to read
- streamed replies still look coherent while updating
- condensed messages and reply previews still align correctly

## Success Criteria

The redesign succeeds if:

- a user can scan a mixed room and immediately spot bot responses
- the assistant's actual answer becomes the visual center of each bot message
- operational metadata stays available without dominating the timeline
- the UI feels closer to Telegram bot chat references while still fitting Robrix's Matrix timeline
