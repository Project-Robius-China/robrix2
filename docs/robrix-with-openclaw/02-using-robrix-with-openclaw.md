# Usage Guide: Robrix + OpenClaw

[中文版](02-using-robrix-with-openclaw-zh.md)

> **Goal:** After following this guide, you will know how to use Robrix to chat with OpenClaw AI agents -- including starting conversations, using DMs and rooms, and understanding how OpenClaw features appear in Robrix.

This guide assumes you have completed the [Deployment Guide](01-deploying-openclaw-with-matrix.md) and the OpenClaw gateway is running.

**Quick Reference**

| What you want to do | Go to |
|---|---|
| Start a DM with the bot | [Section 2](#2-starting-a-dm) |
| Invite the bot to a room | [Section 3](#3-using-the-bot-in-rooms) |
| Understand feature compatibility | [Section 4](#4-openclaw-features-in-robrix) |
| Compare with Octos workflow | [Section 5](#5-differences-from-octos-workflow) |

---

## 1. Before You Start

Confirm the following:

- [ ] OpenClaw gateway is running (`openclaw gateway status` shows `running`)
- [ ] Logs show `matrix: logged in as @bot-name:server`
- [ ] You have another Matrix account (your personal account) to chat with the bot
- [ ] Robrix is installed and can connect to the same Matrix server

---

## 2. Starting a DM

### 2.1 Search for the Bot

1. Open Robrix and log in with your **personal account**
2. Click the **search icon** at the top
3. Type the bot's full Matrix ID, e.g., `@chalice:127.0.0.1:8128`
4. Switch to the **People** tab (the bot is a regular user, so you must search under People)

<img src="../images/openclaw-search-bot.png" width="400" alt="Searching for OpenClaw bot in Robrix">

### 2.2 Send the First Message

1. Select the bot to enter the conversation
2. Type a message (e.g., "Hello"), press Enter
3. Wait for the bot to reply

> **Tip:** LLM responses take time to generate, especially for longer answers (a few seconds to tens of seconds). Please be patient and avoid sending duplicate messages.

<img src="../images/openclaw-bot-reply.png" width="600" alt="OpenClaw Bot (chalice) successfully replying">

> **Note:** If the bot was just deployed, messages you sent earlier may not be decryptable (because those messages' encryption keys were not distributed to the bot's device). This is normal Matrix E2EE behavior -- send a **new message** instead.

### 2.3 Multi-Turn Conversation

OpenClaw maintains conversation context. You can ask follow-up questions and the bot will remember previous messages. The context window size depends on the LLM configuration (DeepSeek Chat supports 164K tokens).

---

## 3. Using the Bot in Rooms

In addition to DMs, you can invite the bot to group chat rooms.

### 3.1 Create a Room and Invite the Bot

1. Create a new room in Robrix
2. Invite the bot (type the bot's Matrix ID)
3. The bot joins automatically (because `autoJoin: "always"` is configured)

<img src="../images/openclaw-bot-join-room.png" width="600" alt="OpenClaw Bot (chalice) accepts invitation and joins the room, just like a regular user">

> Notice that chalice accepts the invitation and joins the room as a regular user -- this is a key characteristic of the client mode. The bot is indistinguishable from any other user.

### 3.2 Chat in the Room

- **Default behavior:** The bot responds to all messages in the room
- **If `requireMention: true` is configured:** You need to @mention the bot to trigger a reply

<img src="../images/openclaw-bot-reply.png" width="600" alt="Chatting with OpenClaw Bot in a room">

---

## 4. OpenClaw Features in Robrix

| OpenClaw Feature | Robrix Support | Notes |
|------------------|---------------|-------|
| **Text messages** | Fully supported | Standard Matrix messages, no compatibility issues |
| **Multi-turn context** | Fully supported | OpenClaw automatically maintains conversation history |
| **E2EE encryption** | Fully supported | Messages are encrypted end-to-end |

---

## 5. Differences from Octos Workflow

If you have previously used Robrix + Palpo + Octos, here are the key differences:

| | OpenClaw | Octos |
|---|---|---|
| **Bot management** | No BotFather system. One OpenClaw instance = one bot. | BotFather can dynamically create multiple child bots |
| **Creating new bots** | Deploy a new OpenClaw instance | Type `/createbot` command in chat |
| **Bot discovery** | Need to know the bot's Matrix ID | Use `/listbots` to see all available bots |
| **Access control** | Via OpenClaw's `dm.policy` configuration | Via AppService namespaces and `allowed_senders` |
| **Server-side setup** | None required | Must register AppService YAML |
| **Robrix Bot Settings panel** | Not used | Used to configure BotFather and create child bots |

> Want to understand the technical differences in depth? See [Architecture Guide](03-how-robrix-and-openclaw-work-together.md).

---

## 6. Tips

- **DM vs Room**: DMs are better for personal assistant use cases -- the bot replies to all messages. Rooms are better for team collaboration; configure `requireMention` to prevent excessive replies.
- **Switching LLMs**: Edit the `models.providers` section in `~/.openclaw/openclaw.json`, then run `openclaw gateway restart`.
- **Bot not responding?** Common causes: expired LLM API key, unverified encryption device, `autoJoin` configuration issues. See [Deployment Guide - Troubleshooting](01-deploying-openclaw-with-matrix.md#7-troubleshooting).

---

## What's Next

- [Deployment Guide](01-deploying-openclaw-with-matrix.md) -- set up and configure OpenClaw with Matrix
- [Architecture Guide](03-how-robrix-and-openclaw-work-together.md) -- understand OpenClaw client mode vs Octos AppService mode

---

*This guide covers usage as of April 2026. For the latest updates, see the respective project repositories.*
