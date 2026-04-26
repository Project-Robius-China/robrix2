# Robrix2 Bot Features vs Telegram Bot Features — Gap Analysis

> Analysis date: 2026-04-11
> Reference: https://core.telegram.org/bots/features

## Current Robrix2 Bot Features

| Category | Feature | Status |
|----------|---------|--------|
| Bot Lifecycle | Create Bot (`/createbot`) | :white_check_mark: |
| | Delete Bot (`/deletebot`) | :white_check_mark: |
| | List Bots (`/listbots`) | :white_check_mark: |
| | Bot Help (`/bothelp`) | :white_check_mark: |
| Room Binding | Bind/unbind Bot to rooms | :white_check_mark: |
| | Invite/kick Bot (Matrix SDK) | :white_check_mark: |
| | Multiple Bots per room | :white_check_mark: |
| Bot Discovery | Auto-detect Bot members in rooms | :white_check_mark: |
| | Parse `/listbots` replies to extract Bot IDs | :white_check_mark: |
| | Heuristic Bot username recognition | :white_check_mark: |
| Settings | App Service toggle | :white_check_mark: |
| | BotFather user ID configuration | :white_check_mark: |
| | Per-room Bot remarks | :white_check_mark: |
| | Persistent storage | :white_check_mark: |
| UI | AppServicePanel (action panel) | :white_check_mark: |
| | Create/Delete/Bind Bot modals | :white_check_mark: |
| | "Manage Bots" in room context menu | :white_check_mark: |
| | i18n (English & Chinese) | :white_check_mark: |

---

## Gap Analysis with Telegram

### P0 — Missing Core Interaction Capabilities

| Telegram Feature | Description | Robrix2 Status | Gap |
|------------------|-------------|----------------|-----|
| **Bot Commands** | User types `/` to see a command list; tap to send | :x: No command discovery | Bots cannot declare supported commands to the client; users must type manually |
| **Inline Keyboards** | Clickable buttons below messages (callback, URL, switch, etc.) | :x: None | Bot messages cannot carry interactive buttons; users can only reply with plain text |
| **Reply Keyboards** | Bot replaces user keyboard with preset options | :x: None | Cannot guide users to choose from fixed options |
| **Callback Queries** | User taps an Inline button; Bot receives callback and can update the message | :x: None | No structured interaction loop between Bot and user |

### P1 — Missing Important UX Features

| Telegram Feature | Description | Robrix2 Status | Gap |
|------------------|-------------|----------------|-----|
| **Inline Mode** | `@bot_name query` in any chat triggers Bot results | :x: None | Can only interact in the Bot's room; no cross-room invocation |
| **Deep Linking** | Link `t.me/bot?start=param` passes parameters to start Bot | :x: None | Cannot share Bot via links with context |
| **Bot Profile** | Bot About, Description, avatar, description image | :warning: Partial | Relies on Matrix profile; no dedicated Bot profile editing UI |
| **Menu Button** | Bot menu button in chat window | :x: None | No dedicated entry point to quickly access Bot functions |
| **Bot-to-Bot Communication** | Bots can interact with each other | :x: None | No Bot orchestration or chaining capability |
| **Privacy Mode** | In groups, Bot only receives `/command` and replies by default | :warning: Matrix-dependent | Matrix has no equivalent concept; Bots receive all messages by default |

### P2 — Missing Advanced / Monetization Features

| Telegram Feature | Description | Robrix2 Status |
|------------------|-------------|----------------|
| **Payments / Stars** | Built-in payment flow, digital currency | :x: None |
| **Mini Apps (Web Apps)** | Bot-embedded JS web applications | :x: None |
| **HTML5 Games** | Gaming platform with leaderboards | :x: None |
| **Stickers / Custom Emoji** | Bot-created sticker packs | :x: None |
| **Paid Media / Subscriptions** | Paid content, tiered subscriptions | :x: None |
| **Ad Revenue Sharing** | Share ad revenue with bots | :x: None |
| **Web Login** | Bot-powered third-party website authentication | :x: None |
| **Managed Bots** | Manage other bots on behalf of owners | :x: None |
| **Bots for Business** | Enterprise customer service bot mode | :x: None |
| **Attachment Menu** | Invoke Bot directly from attachment menu | :x: None |
| **i18n Auto-adaptation** | Bot auto-switches language based on user locale | :x: None |
| **Bot Health Monitoring** | Reply rate and response time alerts | :x: None |

---

## Recommended Roadmap

### Phase 1 — Make Bots Truly Interactive

1. **Bot Command Declaration & Discovery** — Bots register command lists; show available commands when user types `/`
2. **Inline Keyboards (Buttons)** — Messages carry clickable buttons with callback support
3. **Callback Mechanism** — Button taps trigger Bot callbacks and message updates

### Phase 2 — Extend Bot Reach

4. **Inline Mode** — `@bot` cross-room queries
5. **Deep Linking** — Share Bots via links with parameters
6. **Bot Menu Button** — Dedicated Bot menu entry in chat window
7. **Bot Profile Editing** — Dedicated About/Description/Avatar management

### Phase 3 — Platform Features

8. Mini Apps / Web Apps
9. Payment Integration
10. Bot-to-Bot Communication & Orchestration

---

## Summary

Robrix2's current Bot functionality is concentrated at the **management layer** (create, delete, bind, discover) — equivalent to Telegram's BotFather management portion. However, at the **user interaction layer** (Commands, Keyboards, Inline Mode, Callbacks) it is nearly zero — which is precisely the core of Telegram's Bot ecosystem.

The biggest gap is not the absence of advanced features (payments, games, etc.), but the **lack of structured interaction between Bots and users**. Users can only send plain text to Bots, and Bots can only reply with plain text — no buttons, no command menus, no callback updates. This severely limits Bot utility.
