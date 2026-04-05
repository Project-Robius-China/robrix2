# Usage Guide: Robrix + Palpo + Octos

[中文版](03-using-robrix-with-palpo-and-octos-zh.md)

> **Goal:** After following this guide, you will know how to use Robrix to connect to your Palpo server, register an account, create rooms, invite AI bots, have conversations, and manage bots through the BotFather system — all demonstrated with step-by-step screenshots.

This guide walks you through using Robrix as a Matrix client connected to a Palpo homeserver with Octos AI bots. Every step includes what to click and what to type.

**Quick Reference**

| What you want to do | Go to |
|---|---|
| Connect to your server | [Section 2](#2-connecting-to-palpo) |
| Create an account | [Section 3](#3-registration) |
| Chat with the AI bot | [Section 5](#5-chatting-with-the-ai-bot) |
| Create specialized bots | [Section 6](#6-bot-management-advanced) |
| Bot commands and public/private bots | [Section 7](#7-octos-bot-commands-and-behavior) |

---

## 1. Before You Start

Make sure:

- **Palpo and Octos are running.** Follow the [Deployment Guide](01-deploying-palpo-and-octos.md) to set up all services.
- **Robrix is installed and ready.** See [Getting Started](../robrix/getting-started-with-robrix.md) for build or download instructions.

> **Note:** This guide assumes a local deployment with `server_name = 127.0.0.1:8128`. Replace this with your actual server name if you deployed remotely.

---

## 2. Connecting to Palpo

When you open Robrix, the login screen appears. By default, Robrix connects to `matrix.org`. You need to point it to your Palpo server instead.

1. Look at the **bottom** of the login screen for the **Homeserver URL** field.
2. Enter `http://127.0.0.1:8128` for a local deployment.
3. For a remote server, enter `https://your.server.name` or `http://server-ip:8128`.

<img src="../images/login-screen.png" width="600" alt="Robrix login screen — enter your Homeserver URL at the bottom">

> **Note:** If the Homeserver URL field is left empty, Robrix connects to `matrix.org` by default. You must fill it in to reach your own Palpo server.

---

## 3. Registration

To create a new account on your Palpo server:

1. Enter your desired **Username** (e.g., `alice`).
2. Enter a **Password**.
3. Enter the same password again in the **Confirm password** field.
4. Enter the **Homeserver URL**: `http://127.0.0.1:8128`.
5. Click **Sign up**.

<img src="../images/register-account.png" width="600" alt="Register account — enter username, password, and Homeserver URL">

> **Note:** Registration must be enabled on the server. Make sure `allow_registration = true` is set in your `palpo.toml`. See [Deployment Guide -- Configuration](01-deploying-palpo-and-octos.md) for details.

---

## 4. Login

If you already have an account:

1. Enter your **Username** and **Password**.
2. Enter the **Homeserver URL**: `http://127.0.0.1:8128`.
3. Click **Log in**.

After logging in, you will see the room list. For a new account, this list is empty.

<!-- screenshot: room-list-empty.png — Empty room list after first login -->

---

## 5. Chatting with the AI Bot

This is the main workflow: create a room, invite the bot, and start a conversation.

### 5.1 Create a New Room

1. Click the **create room** button (the "+" icon in the room list area).
2. Give the room a name, for example "AI Chat".
3. The room is created and you enter it automatically.

<!-- screenshot: create-room.png — Create room dialog with a room name entered -->

### 5.2 Invite the Bot

1. Click the **search icon** (**①** in the screenshot below) at the top of the room list.
2. In the search dialog, type the bot's full Matrix ID: `@octosbot:127.0.0.1:8128`.
3. Click the **People** tab (**②**) to filter results to users and bots (instead of Rooms or Spaces).
4. Select the bot from the search results to start a direct conversation or invite it to a room.
5. The bot joins automatically. This is handled by the Application Service mechanism -- no manual acceptance is needed on the bot side.

<img src="../images/search-invite-bot.png" width="600" alt="Search for the bot: click the search icon (1), type the bot ID, then click People (2) to find it">

> **How is this bot name determined?** The BotFather's Matrix ID is assembled from two config values:
>
> | Part | Value | Configured in |
> |------|-------|---------------|
> | Username (localpart) | `octosbot` | `sender_localpart` in `octos-registration.yaml` and `botfather.json` |
> | Server domain | `127.0.0.1:8128` | `server_name` in `palpo.toml` |
> | **Full Matrix ID** | **`@octosbot:127.0.0.1:8128`** | |
> | Display name (shown in rooms) | `BotFather` | `name` in `botfather.json` |
>
> Child bots created via `/createbot` follow a similar pattern. The `user_prefix` field in `botfather.json` (default: `octosbot_`) is automatically prepended to the username you specify:
>
> `/createbot weather Weather Bot` → Matrix ID: `@octosbot_weather:127.0.0.1:8128`
>
> If you change `server_name` in production, all bot IDs change accordingly. You must also update the namespace regex in `octos-registration.yaml` to match.

### 5.3 Start Chatting

1. Type a message in the input box at the bottom of the room.
2. Press **Enter** or click **Send**.
3. The bot processes your message through the configured LLM and replies.
4. You will see a streaming animation as the response arrives in real time.

<!-- screenshot: bot-conversation.png — A conversation showing a user message and the bot's AI-generated reply -->

> The bot's response time depends on the LLM provider and model. DeepSeek typically responds within a few seconds. Larger models may take longer.

**Example conversation:**

```
You:   What is the Matrix protocol?
Bot:   Matrix is an open standard for decentralized, real-time communication.
       It provides HTTP APIs for creating and managing chat rooms, sending
       messages, and synchronizing state across federated servers...
```

### 5.4 Alternative: Join an Existing Bot Room

If someone else has already created a room with the bot and invited you, or if a public room exists:

1. Click **Join Room**.
2. Enter the room alias (e.g., `#ai-chat:127.0.0.1:8128`) or the room ID.
3. You can start chatting with the bot right away.

<!-- screenshot: join-room.png — Join room dialog with a room alias entered -->

---

## 6. Bot Management (Advanced)

Octos supports a "BotFather" pattern: the main bot (`@octosbot`) can create **child bots**, each with its own personality and system prompt. This is useful for building specialized assistants.

For a deeper understanding of how this works, see the [Architecture Guide](02-how-robrix-palpo-octos-work-together.md).

### 6.1 Enable App Service Support in Robrix

Before managing bots, enable the feature in Robrix:

1. Open **Settings** in Robrix (gear icon).
2. Navigate to **Bot Settings**.
3. Toggle **Enable App Service** to on.
4. Enter the **BotFather User ID**: `@octosbot:127.0.0.1:8128`.
5. Click **Save**.

<!-- screenshot: bot-settings.png — Bot Settings panel in Robrix settings with Enable App Service toggled on and BotFather User ID filled in -->

### 6.2 Create Child Bots

With BotFather enabled, you can create specialized bots:

1. Open the **Create Bot** dialog from the bot management panel.
2. Fill in the following fields:
   - **Username** -- lowercase letters, digits, and underscores only (e.g., `translator_bot`).
   - **Display Name** -- a human-readable name shown in rooms (e.g., "Translator Bot").
   - **System Prompt** -- instructions that define the bot's behavior. Examples:
     - `"You are a translator. Translate all messages to English."`
     - `"You are a coding assistant. Help users write and debug code."`
     - `"You are a writing coach. Review text for clarity and grammar."`
3. Click **Create Bot**.

The child bot is registered as `@octosbot_<username>:127.0.0.1:8128`. For the example above, it would be `@octosbot_translator_bot:127.0.0.1:8128`.

<!-- screenshot: create-bot-dialog.png — Create Bot dialog with Username, Display Name, and System Prompt fields filled out -->

### 6.3 Using Child Bots

After creating a child bot, use it like the main bot:

1. Create a new room or use an existing one.
2. Invite the child bot by its full Matrix ID (e.g., `@octosbot_translator_bot:127.0.0.1:8128`).
3. Chat with it. The bot follows the system prompt you defined.

<!-- screenshot: child-bot-conversation.png — Conversation with a specialized child bot following its system prompt -->

---

## 7. Octos Bot Commands and Behavior

Octos bots support a small set of slash commands that you type directly in the chat room. In this guide, we focus on the main BotFather management commands and the public/private visibility model for child bots.

### 7.1 BotFather Management Commands

These commands only work when sent to the BotFather bot (`@octosbot`). Child bots do not respond to them.

| Command | Description | Example |
|---------|-------------|---------|
| `/createbot <username> <display_name> [flags]` | Create a new child bot. Flags: `--public` or `--private` (default), `--prompt "..."` for system prompt. | `/createbot weather Weather Bot --public --prompt "You are a weather assistant"` |
| `/deletebot <matrix_user_id>` | Delete a child bot. Only the bot's creator (or the operator) can delete it. | `/deletebot @octosbot_weather:127.0.0.1:8128` |
| `/listbots` | List all public bots plus your own private bots. | `/listbots` |
| `/bothelp` | Show help text for bot management commands. | `/bothelp` |

> **Note:** You can also create bots through Robrix's UI (Section 6.2), which provides a form-based alternative to these slash commands.

### 7.2 BotFather vs Child Bots

BotFather and child bots serve different roles:

| | BotFather (`@octosbot`) | Child Bot (`@octosbot_<name>`) |
|---|---|---|
| **Role** | Management gateway + general AI chat | Specialized AI assistant |
| **Bot management commands** | Yes (`/createbot`, `/deletebot`, `/listbots`) | No |
| **Custom system prompt** | Uses default prompt | Has its own dedicated prompt |
| **Can create other bots** | Yes | No |
| **Matrix user ID** | `@octosbot:server_name` | `@octosbot_<username>:server_name` |

**When to use which:**
- Use **BotFather** for general-purpose AI chat and for managing (creating/deleting) other bots.
- Use **child bots** when you need a dedicated assistant for a specific task (translation, coding help, writing review, etc.) with a fixed system prompt.

### 7.3 Public vs Private Bots

When creating a child bot, you can set its **visibility**:

- **Private (default):** Only the creator can invite and chat with this bot. Other users cannot discover it via `/listbots`, and if they try to invite it, the bot will join briefly, send a rejection message, then leave the room.
- **Public:** Any user on the server can discover the bot via `/listbots`, invite it to rooms, and chat with it.

**Creating a private bot (default):**
```
/createbot myhelper My Helper --prompt "You are my personal assistant"
```

**Creating a public bot:**
```
/createbot translator Translator Bot --public --prompt "Translate all messages to English"
```

**Who can delete a bot:**
- The **creator** (owner) of the bot can always delete it.
- The **operator** (anyone in `allowed_senders` in `botfather.json`) can delete any bot as an override.

> **Tip:** Start with private bots for personal use. Make a bot public only when you want other users on the server to use it.

---

## 8. Tips

- **Multiple bots in one room.** You can invite several bots into the same room. Each bot responds independently based on its own system prompt. This is useful for comparing outputs or building multi-agent workflows.

- **Private conversations.** Create a private room and invite only one bot for focused 1-on-1 chats without noise from other users or bots.

- **Change the LLM provider.** The LLM backend is configured in `botfather.json` (or via environment variables). You can switch between DeepSeek, OpenAI, Anthropic, and other providers. See the [Deployment Guide -- Configuration](01-deploying-palpo-and-octos.md) for details.

- **Bot not responding?** Common causes:
  - The Octos service is not running.
  - The LLM API key is missing or invalid.
  - The bot was not properly invited to the room.
  - Check the [Troubleshooting section](01-deploying-palpo-and-octos.md#5-troubleshooting) in the Deployment Guide.

- **Server name mismatch.** All Matrix IDs (users, bots, rooms) must use the same `server_name` that Palpo is configured with. If your bot ID does not match the server name, the invitation will fail.

---

## 9. Common Matrix IDs Reference

For a local deployment with `server_name = 127.0.0.1:8128`:

| Item | Matrix ID |
|---|---|
| Your user account | `@yourusername:127.0.0.1:8128` |
| Main AI bot (BotFather) | `@octosbot:127.0.0.1:8128` |
| A child bot (e.g., translator) | `@octosbot_translator_bot:127.0.0.1:8128` |
| A room alias | `#room-name:127.0.0.1:8128` |

For remote deployments, replace `127.0.0.1:8128` with your configured `server_name`.

---

## What's Next

- [Deployment Guide](01-deploying-palpo-and-octos.md) -- set up and configure services
- [Architecture Guide](02-how-robrix-palpo-octos-work-together.md) -- understand how the components work together
